extern crate amber;
extern crate crossbeam_channel;
#[macro_use]
extern crate lazy_static;
extern crate num_cpus;
extern crate rustc_serialize;
#[macro_use]
extern crate structopt;

use amber::console::{Console, ConsoleTextKind};
use amber::matcher::{QuickSearchMatcher, RegexMatcher, TbmMatcher};
use amber::pipeline::{Pipeline, PipelineFork, PipelineInfo, PipelineJoin};
use amber::pipeline_finder::PipelineFinder;
use amber::pipeline_matcher::PipelineMatcher;
use amber::pipeline_replacer::PipelineReplacer;
use amber::pipeline_sorter::PipelineSorter;
use amber::util::{as_secsf64, decode_error, exit, read_from_file};
use crossbeam_channel::unbounded;
use std::cmp;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use structopt::{clap, StructOpt};

// ---------------------------------------------------------------------------------------------------------------------
// Opt
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(name = "ambr")]
#[structopt(raw(long_version = "option_env!(\"LONG_VERSION\").unwrap_or(env!(\"CARGO_PKG_VERSION\"))"))]
#[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
#[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
pub struct Opt {
    /// Keyword for search
    #[structopt(name = "KEYWORD", required_unless = "key_file")]
    pub keyword: Option<String>,

    /// Keyword for replace
    #[structopt(name = "REPLACEMENT", required_unless = "rep_file")]
    pub replacement: Option<String>,

    /// Use file contents as KEYWORD
    #[structopt(long = "key-file", value_name = "FILE")]
    pub key_file: Option<String>,

    /// Use file contents as REPLACEMENT
    #[structopt(long = "rep-file", value_name = "FILE")]
    pub rep_file: Option<String>,

    /// Search paths
    #[structopt(name = "PATHS")]
    pub paths: Vec<String>,

    /// Number of max threads
    #[structopt(long = "max-threads", raw(default_value = "&MAX_THREADS"), value_name = "NUM")]
    pub max_threads: usize,

    /// File size per one thread
    #[structopt(long = "size-per-thread", default_value = "1048576", value_name = "BYTES")]
    pub size_per_thread: usize,

    /// Read size for checking binary
    #[structopt(long = "bin-check-bytes", default_value = "256", value_name = "BYTES")]
    pub bin_check_bytes: usize,

    /// [Experimental] Minimum size for using mmap
    #[structopt(long = "mmap-bytes", default_value = "1048576", value_name = "BYTES")]
    pub mmap_bytes: u64,

    /// Enable regular expression search
    #[structopt(long = "regex")]
    pub regex: bool,

    /// Enable column output
    #[structopt(long = "column")]
    pub column: bool,

    /// Enable row output
    #[structopt(long = "row")]
    pub row: bool,

    /// Enable binary file search
    #[structopt(long = "binary")]
    pub binary: bool,

    /// Enable statistics output
    #[structopt(long = "statistics")]
    pub statistics: bool,

    /// Enable skipped file output
    #[structopt(long = "skipped")]
    pub skipped: bool,

    /// Disable interactive replace
    #[structopt(long = "no-interactive")]
    pub no_interactive: bool,

    /// Disable recursive directory search
    #[structopt(long = "no-recursive")]
    pub no_recursive: bool,

    /// Disable symbolic link follow
    #[structopt(long = "no-symlink")]
    pub no_symlink: bool,

    /// Disable colored output
    #[structopt(long = "no-color")]
    pub no_color: bool,

    /// Disable filename output
    #[structopt(long = "no-file")]
    pub no_file: bool,

    /// Disable vcs directory ( .hg/.git/.svn ) skip
    #[structopt(long = "no-skip-vcs")]
    pub no_skip_vcs: bool,

    /// Disable .gitignore skip
    #[structopt(long = "no-skip-gitignore")]
    pub no_skip_gitignore: bool,

    /// Disable output order guarantee
    #[structopt(long = "no-fixed-order")]
    pub no_fixed_order: bool,

    /// Disable .*ignore file search at parent directories
    #[structopt(long = "no-parent-ignore")]
    pub no_parent_ignore: bool,

    /// [Experimental] Enable TBM matcher
    #[structopt(long = "tbm")]
    pub tbm: bool,

    /// [Experimental] Enable SSE 4.2
    #[structopt(long = "sse")]
    pub sse: bool,
}

lazy_static! {
    static ref MAX_THREADS: String = format!("{}", cmp::min(4, num_cpus::get()));
}

// ---------------------------------------------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------------------------------------------

fn main() {
    // ---------------------------------------------------------------------------------------------
    // Parse Arguments
    // ---------------------------------------------------------------------------------------------

    // - Create opt ------------------------------------------------------------

    let opt = Opt::from_args();

    let mut console = Console::new();
    console.is_color = !opt.no_color;

    // - Set base path, keyword and replacement --------------------------------
    let mut base_paths: Vec<PathBuf> = Vec::new();
    if opt.paths.is_empty() {
        base_paths.push(PathBuf::from("./"));
    } else {
        for p in &opt.paths {
            base_paths.push(PathBuf::from(p));
        }
    }

    let keyword = match opt.key_file {
        Some(f) => match read_from_file(&f) {
            Ok(x) => {
                if x.len() != 0 {
                    x
                } else {
                    console.write(ConsoleTextKind::Error, &format!("Error: file is empty @ {:?}\n", f));
                    exit(1, &mut console);
                }
            }
            Err(e) => {
                console.write(
                    ConsoleTextKind::Error,
                    &format!("Error: {} @ {:?}\n", decode_error(e.kind()), f),
                );
                exit(1, &mut console);
            }
        },
        None => opt.keyword.unwrap().clone().into_bytes(),
    };

    let replacement = match opt.rep_file {
        Some(f) => match read_from_file(&f) {
            Ok(x) => x,
            Err(e) => {
                console.write(
                    ConsoleTextKind::Error,
                    &format!("Error: {} @ {:?}\n", decode_error(e.kind()), f),
                );
                exit(1, &mut console);
            }
        },
        None => opt.replacement.unwrap().clone().into_bytes(),
    };

    // ---------------------------------------------------------------------------------------------
    // Pipeline Construct
    // ---------------------------------------------------------------------------------------------

    let id_finder = 0;
    let id_sorter = 1;
    let id_replacer = 2;
    let id_matcher = 3;

    let matcher_num = opt.max_threads;

    let (tx_finder, rx_finder) = unbounded();
    let (tx_replacer, rx_replacer) = unbounded();
    let (tx_main, rx_main) = unbounded();

    let mut tx_matcher = Vec::new();
    let mut rx_sorter = Vec::new();

    let mut finder = PipelineFinder::new();
    let mut sorter = PipelineSorter::new(matcher_num);
    let mut replacer = PipelineReplacer::new(&keyword, &replacement, opt.regex);

    finder.is_recursive = !opt.no_recursive;
    finder.follow_symlink = !opt.no_symlink;
    finder.skip_vcs = !opt.no_skip_vcs;
    finder.skip_gitignore = !opt.no_skip_gitignore;
    finder.print_skipped = opt.skipped;
    finder.find_parent_ignore = !opt.no_parent_ignore;
    sorter.through = opt.no_fixed_order;
    replacer.is_color = !opt.no_color;
    replacer.is_interactive = !opt.no_interactive;
    replacer.print_file = !opt.no_file;
    replacer.print_column = opt.column;
    replacer.print_row = opt.row;

    let use_regex = opt.regex;
    let use_tbm = opt.tbm;
    let skip_binary = !opt.binary;
    let print_skipped = opt.skipped;
    let binary_check_bytes = opt.bin_check_bytes;
    let mmap_bytes = opt.mmap_bytes;
    let max_threads = opt.max_threads;
    let size_per_thread = opt.size_per_thread;

    for i in 0..matcher_num {
        let keyword = keyword.clone();
        let (tx_in, rx_in) = unbounded();
        let (tx_out, rx_out) = unbounded();
        tx_matcher.push(tx_in);
        rx_sorter.push(rx_out);

        let _ = thread::Builder::new().name("matcher".to_string()).spawn(move || {
            if use_regex {
                let m = RegexMatcher::new();
                let mut matcher = PipelineMatcher::new(m, &keyword);
                matcher.skip_binary = skip_binary;
                matcher.print_skipped = print_skipped;
                matcher.binary_check_bytes = binary_check_bytes;
                matcher.mmap_bytes = mmap_bytes;
                matcher.setup(id_matcher + i, rx_in, tx_out);
            } else if use_tbm {
                let mut m = TbmMatcher::new();
                m.max_threads = max_threads;
                m.size_per_thread = size_per_thread;
                let mut matcher = PipelineMatcher::new(m, &keyword);
                matcher.skip_binary = skip_binary;
                matcher.print_skipped = print_skipped;
                matcher.binary_check_bytes = binary_check_bytes;
                matcher.mmap_bytes = mmap_bytes;
                matcher.setup(id_matcher + i, rx_in, tx_out);
            } else {
                let mut m = QuickSearchMatcher::new();
                m.max_threads = max_threads;
                m.size_per_thread = size_per_thread;
                let mut matcher = PipelineMatcher::new(m, &keyword);
                matcher.skip_binary = skip_binary;
                matcher.print_skipped = print_skipped;
                matcher.binary_check_bytes = binary_check_bytes;
                matcher.mmap_bytes = mmap_bytes;
                matcher.setup(id_matcher + i, rx_in, tx_out);
            };
        });
    }

    let _ = thread::Builder::new().name("finder".to_string()).spawn(move || {
        finder.setup(id_finder, rx_finder, tx_matcher);
    });

    let _ = thread::Builder::new().name("sorter".to_string()).spawn(move || {
        sorter.setup(id_sorter, rx_sorter, tx_replacer);
    });

    let _ = thread::Builder::new().name("replacer".to_string()).spawn(move || {
        replacer.setup(id_replacer, rx_replacer, tx_main);
    });

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let mut seq_no = 0;
    let _ = tx_finder.send(PipelineInfo::SeqBeg(seq_no));
    for p in base_paths {
        let _ = tx_finder.send(PipelineInfo::SeqDat(seq_no, p));
        seq_no += 1;
    }
    let _ = tx_finder.send(PipelineInfo::SeqEnd(seq_no));

    let mut time_finder_bsy = Duration::new(0, 0);
    let mut time_finder_all = Duration::new(0, 0);
    let mut time_sorter_bsy = Duration::new(0, 0);
    let mut time_sorter_all = Duration::new(0, 0);
    let mut time_replacer_bsy = Duration::new(0, 0);
    let mut time_replacer_all = Duration::new(0, 0);

    let mut time_matcher_bsy = Vec::new();
    let mut time_matcher_all = Vec::new();
    for _ in 0..matcher_num {
        time_matcher_bsy.push(Duration::new(0, 0));
        time_matcher_all.push(Duration::new(0, 0));
    }

    loop {
        match rx_main.try_recv() {
            Ok(PipelineInfo::SeqEnd(_)) => break,
            Ok(PipelineInfo::MsgTime(id, t0, t1)) if id == id_finder => {
                time_finder_bsy = t0;
                time_finder_all = t1;
            }
            Ok(PipelineInfo::MsgTime(id, t0, t1)) if id == id_sorter => {
                time_sorter_bsy = t0;
                time_sorter_all = t1;
            }
            Ok(PipelineInfo::MsgTime(id, t0, t1)) if id == id_replacer => {
                time_replacer_bsy = t0;
                time_replacer_all = t1;
            }
            Ok(PipelineInfo::MsgTime(id, t0, t1)) => {
                time_matcher_bsy[id - id_matcher] = t0;
                time_matcher_all[id - id_matcher] = t1;
            }
            Ok(PipelineInfo::MsgInfo(_id, s)) => console.write(ConsoleTextKind::Info, &format!("{}\n", s)),
            Ok(PipelineInfo::MsgErr(_id, s)) => console.write(ConsoleTextKind::Error, &format!("{}\n", s)),
            Ok(_) => (),
            Err(_) => (),
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let sec_finder_bsy = as_secsf64(time_finder_bsy);
    let sec_finder_all = as_secsf64(time_finder_all);
    let sec_sorter_bsy = as_secsf64(time_sorter_bsy);
    let sec_sorter_all = as_secsf64(time_sorter_all);
    let sec_replacer_bsy = as_secsf64(time_replacer_bsy);
    let sec_replacer_all = as_secsf64(time_replacer_all);

    let mut sec_matcher_bsy = Vec::new();
    let mut sec_matcher_all = Vec::new();
    for i in 0..matcher_num {
        sec_matcher_bsy.push(as_secsf64(time_matcher_bsy[i]));
        sec_matcher_all.push(as_secsf64(time_matcher_all[i]));
    }

    if opt.statistics {
        console.write(ConsoleTextKind::Info, &format!("\nStatistics\n"));
        console.write(
            ConsoleTextKind::Info,
            &format!("  Max threads: {}\n\n", opt.max_threads),
        );
        console.write(ConsoleTextKind::Info, &format!("  Consumed time ( busy / total )\n"));
        console.write(
            ConsoleTextKind::Info,
            &format!("    Find     : {}s / {}s\n", sec_finder_bsy, sec_finder_all),
        );
        for i in 0..matcher_num {
            console.write(
                ConsoleTextKind::Info,
                &format!(
                    "    Match{:02}  : {}s / {}s\n",
                    i, sec_matcher_bsy[i], sec_matcher_all[i]
                ),
            );
        }
        console.write(
            ConsoleTextKind::Info,
            &format!("    Sort     : {}s / {}s\n", sec_sorter_bsy, sec_sorter_all),
        );
        console.write(
            ConsoleTextKind::Info,
            &format!("    Replace  : {}s / {}s\n\n", sec_replacer_bsy, sec_replacer_all),
        );
    }

    exit(0, &mut console);
}
