use amber::console::{Console, ConsoleTextKind};
use amber::matcher::{QuickSearchMatcher, RegexMatcher, TbmMatcher};
use amber::pipeline::{Pipeline, PipelineFork, PipelineInfo, PipelineJoin};
use amber::pipeline_finder::PipelineFinder;
use amber::pipeline_matcher::PipelineMatcher;
use amber::pipeline_replacer::PipelineReplacer;
use amber::pipeline_sorter::PipelineSorter;
use amber::util::{as_secsf64, decode_error, exit, read_from_file};
use crossbeam::channel::unbounded;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::cmp;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use structopt::{clap, StructOpt};

// ---------------------------------------------------------------------------------------------------------------------
// Opt
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(name = "ambr")]
#[structopt(long_version(option_env!("LONG_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))))]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
#[structopt(setting(clap::AppSettings::DeriveDisplayOrder))]
pub struct Opt {
    /// Keyword for search
    #[structopt(name = "KEYWORD")]
    pub keyword: String,

    /// Keyword for replace
    #[structopt(name = "REPLACEMENT")]
    pub replacement: String,

    /// Use file contents of KEYWORD as keyword for search
    #[structopt(long = "key-from-file")]
    pub key_from_file: bool,

    /// Use file contents of REPLACEMENT as keyword for replacement
    #[structopt(long = "rep-from-file")]
    pub rep_from_file: bool,

    /// Search paths
    #[structopt(name = "PATHS")]
    pub paths: Vec<String>,

    /// Number of max threads
    #[structopt(long = "max-threads", default_value = &MAX_THREADS, value_name = "NUM")]
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

    /// Verbose message
    #[structopt(long = "verbose")]
    pub verbose: bool,

    /// Enable regular expression search
    #[structopt(short = "r", long = "regex", hidden = DEFAULT_FLAGS.regex)]
    pub regex: bool,

    /// Enable column output
    #[structopt(long = "column", hidden = DEFAULT_FLAGS.column)]
    pub column: bool,

    /// Enable row output
    #[structopt(long = "row", hidden = DEFAULT_FLAGS.row)]
    pub row: bool,

    /// Enable binary file search
    #[structopt(long = "binary", hidden = DEFAULT_FLAGS.binary)]
    pub binary: bool,

    /// Enable statistics output
    #[structopt(long = "statistics", hidden = DEFAULT_FLAGS.statistics)]
    pub statistics: bool,

    /// Enable skipped file output
    #[structopt(long = "skipped", hidden = DEFAULT_FLAGS.skipped)]
    pub skipped: bool,

    /// Enable interactive replace
    #[structopt(long = "interactive", hidden = DEFAULT_FLAGS.interactive)]
    pub interactive: bool,

    /// Enable recursive directory search
    #[structopt(long = "recursive", hidden = DEFAULT_FLAGS.recursive)]
    pub recursive: bool,

    /// Enable symbolic link follow
    #[structopt(long = "symlink", hidden = DEFAULT_FLAGS.symlink)]
    pub symlink: bool,

    /// Enable colored output
    #[structopt(long = "color", hidden = DEFAULT_FLAGS.color)]
    pub color: bool,

    /// Enable filename output
    #[structopt(long = "file", hidden = DEFAULT_FLAGS.file)]
    pub file: bool,

    /// Enable vcs directory ( .hg/.git/.svn ) skip
    #[structopt(long = "skip-vcs", hidden = DEFAULT_FLAGS.skip_vcs)]
    pub skip_vcs: bool,

    /// Enable .gitignore skip
    #[structopt(long = "skip-gitignore", hidden = DEFAULT_FLAGS.skip_gitignore)]
    pub skip_gitignore: bool,

    /// Enable output order guarantee
    #[structopt(long = "fixed-order", hidden = DEFAULT_FLAGS.fixed_order)]
    pub fixed_order: bool,

    /// Enable .*ignore file search at parent directories
    #[structopt(long = "parent-ignore", hidden = DEFAULT_FLAGS.parent_ignore)]
    pub parent_ignore: bool,

    /// Enable timestamp preserve
    #[structopt(long = "preserve-time", hidden = DEFAULT_FLAGS.preserve_time)]
    pub preserve_time: bool,

    /// Disable regular expression search
    #[structopt(long = "no-regex", hidden = !DEFAULT_FLAGS.regex)]
    pub no_regex: bool,

    /// Disable column output
    #[structopt(long = "no-column", hidden = !DEFAULT_FLAGS.column)]
    pub no_column: bool,

    /// Disable row output
    #[structopt(long = "no-row", hidden = !DEFAULT_FLAGS.row)]
    pub no_row: bool,

    /// Disable binary file search
    #[structopt(long = "no-binary", hidden = !DEFAULT_FLAGS.binary)]
    pub no_binary: bool,

    /// Disable statistics output
    #[structopt(long = "no-statistics", hidden = !DEFAULT_FLAGS.statistics)]
    pub no_statistics: bool,

    /// Disable skipped file output
    #[structopt(long = "no-skipped", hidden = !DEFAULT_FLAGS.skipped)]
    pub no_skipped: bool,

    /// Disable interactive replace
    #[structopt(long = "no-interactive", hidden = !DEFAULT_FLAGS.interactive)]
    pub no_interactive: bool,

    /// Disable recursive directory search
    #[structopt(long = "no-recursive", hidden = !DEFAULT_FLAGS.recursive)]
    pub no_recursive: bool,

    /// Disable symbolic link follow
    #[structopt(long = "no-symlink", hidden = !DEFAULT_FLAGS.symlink)]
    pub no_symlink: bool,

    /// Disable colored output
    #[structopt(long = "no-color", hidden = !DEFAULT_FLAGS.color)]
    pub no_color: bool,

    /// Disable filename output
    #[structopt(long = "no-file", hidden = !DEFAULT_FLAGS.file)]
    pub no_file: bool,

    /// Disable vcs directory ( .hg/.git/.svn ) skip
    #[structopt(long = "no-skip-vcs", hidden = !DEFAULT_FLAGS.skip_vcs)]
    pub no_skip_vcs: bool,

    /// Disable .gitignore skip
    #[structopt(long = "no-skip-gitignore", hidden = !DEFAULT_FLAGS.skip_gitignore)]
    pub no_skip_gitignore: bool,

    /// Disable output order guarantee
    #[structopt(long = "no-fixed-order", hidden = !DEFAULT_FLAGS.fixed_order)]
    pub no_fixed_order: bool,

    /// Disable .*ignore file search at parent directories
    #[structopt(long = "no-parent-ignore", hidden = !DEFAULT_FLAGS.parent_ignore)]
    pub no_parent_ignore: bool,

    /// Disable timestamp preserve
    #[structopt(long = "no-preserve-time", hidden = !DEFAULT_FLAGS.preserve_time)]
    pub no_preserve_time: bool,

    /// [Experimental] Enable TBM matcher
    #[structopt(long = "tbm")]
    pub tbm: bool,

    /// [Experimental] Enable SSE 4.2
    #[structopt(long = "sse")]
    pub sse: bool,
}

#[derive(Debug, Deserialize)]
struct DefaultFlags {
    #[serde(default = "flag_false")]
    regex: bool,
    #[serde(default = "flag_false")]
    column: bool,
    #[serde(default = "flag_false")]
    row: bool,
    #[serde(default = "flag_false")]
    binary: bool,
    #[serde(default = "flag_false")]
    statistics: bool,
    #[serde(default = "flag_false")]
    skipped: bool,
    #[serde(default = "flag_true")]
    interactive: bool,
    #[serde(default = "flag_true")]
    recursive: bool,
    #[serde(default = "flag_true")]
    symlink: bool,
    #[serde(default = "flag_true")]
    color: bool,
    #[serde(default = "flag_true")]
    file: bool,
    #[serde(default = "flag_true")]
    skip_vcs: bool,
    #[serde(default = "flag_true")]
    skip_gitignore: bool,
    #[serde(default = "flag_true")]
    fixed_order: bool,
    #[serde(default = "flag_true")]
    parent_ignore: bool,
    #[serde(default = "flag_false")]
    preserve_time: bool,
}

impl DefaultFlags {
    fn new() -> DefaultFlags {
        toml::from_str("").unwrap()
    }

    fn load() -> DefaultFlags {
        match dirs::home_dir() {
            Some(mut path) => {
                path.push(".ambr.toml");
                if path.exists() {
                    match fs::File::open(&path) {
                        Ok(mut f) => {
                            let mut s = String::new();
                            let _ = f.read_to_string(&mut s);
                            match toml::from_str(&s) {
                                Ok(x) => x,
                                Err(_) => DefaultFlags::new(),
                            }
                        }
                        Err(_) => DefaultFlags::new(),
                    }
                } else {
                    DefaultFlags::new()
                }
            }
            None => DefaultFlags::new(),
        }
    }

    fn merge(&self, mut opt: Opt) -> Opt {
        opt.regex = if self.regex { !opt.no_regex } else { opt.regex };
        opt.column = if self.column { !opt.no_column } else { opt.column };
        opt.row = if self.row { !opt.no_row } else { opt.row };
        opt.binary = if self.binary { !opt.no_binary } else { opt.binary };
        opt.statistics = if self.statistics {
            !opt.no_statistics
        } else {
            opt.statistics
        };
        opt.skipped = if self.skipped { !opt.no_skipped } else { opt.skipped };
        opt.interactive = if self.interactive {
            !opt.no_interactive
        } else {
            opt.interactive
        };
        opt.recursive = if self.recursive {
            !opt.no_recursive
        } else {
            opt.recursive
        };
        opt.symlink = if self.symlink { !opt.no_symlink } else { opt.symlink };
        opt.color = if self.color { !opt.no_color } else { opt.color };
        opt.file = if self.file { !opt.no_file } else { opt.file };
        opt.skip_vcs = if self.skip_vcs { !opt.no_skip_vcs } else { opt.skip_vcs };
        opt.skip_gitignore = if self.skip_gitignore {
            !opt.no_skip_gitignore
        } else {
            opt.skip_gitignore
        };
        opt.fixed_order = if self.fixed_order {
            !opt.no_fixed_order
        } else {
            opt.fixed_order
        };
        opt.parent_ignore = if self.parent_ignore {
            !opt.no_parent_ignore
        } else {
            opt.parent_ignore
        };
        opt.preserve_time = if self.preserve_time {
            !opt.no_preserve_time
        } else {
            opt.preserve_time
        };
        opt
    }
}

fn flag_true() -> bool {
    true
}
fn flag_false() -> bool {
    false
}

lazy_static! {
    static ref MAX_THREADS: String = format!("{}", num_cpus::get());
    static ref DEFAULT_FLAGS: DefaultFlags = DefaultFlags::load();
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
    let opt = DEFAULT_FLAGS.merge(opt);

    let mut console = Console::new();
    console.is_color = opt.color;

    // - Set base path, keyword and replacement --------------------------------
    let mut base_paths: Vec<PathBuf> = Vec::new();
    if opt.paths.is_empty() {
        base_paths.push(PathBuf::from("./"));
    } else {
        for p in &opt.paths {
            base_paths.push(PathBuf::from(p));
        }
    }

    let keyword = if opt.key_from_file {
        match read_from_file(&opt.keyword) {
            Ok(x) => {
                if !x.is_empty() {
                    x
                } else {
                    console.write(
                        ConsoleTextKind::Error,
                        &format!("Error: file is empty @ {:?}\n", opt.keyword),
                    );
                    exit(1, &mut console);
                }
            }
            Err(e) => {
                console.write(
                    ConsoleTextKind::Error,
                    &format!("Error: {} @ {:?}\n", decode_error(e.kind()), opt.keyword),
                );
                exit(1, &mut console);
            }
        }
    } else {
        opt.keyword.into_bytes()
    };

    let replacement = if opt.rep_from_file {
        match read_from_file(&opt.replacement) {
            Ok(x) => x,
            Err(e) => {
                console.write(
                    ConsoleTextKind::Error,
                    &format!("Error: {} @ {:?}\n", decode_error(e.kind()), opt.replacement),
                );
                exit(1, &mut console);
            }
        }
    } else {
        opt.replacement.into_bytes()
    };

    // ---------------------------------------------------------------------------------------------
    // Pipeline Construct
    // ---------------------------------------------------------------------------------------------

    let id_finder = 0;
    let id_sorter = 1;
    let id_replacer = 2;
    let id_matcher = 3;

    let matcher_num = cmp::min(8, opt.max_threads);

    let (tx_finder, rx_finder) = unbounded();
    let (tx_replacer, rx_replacer) = unbounded();
    let (tx_main, rx_main) = unbounded();

    let mut tx_matcher = Vec::new();
    let mut rx_sorter = Vec::new();

    let mut finder = PipelineFinder::new();
    let mut sorter = PipelineSorter::new(matcher_num);
    let mut replacer = PipelineReplacer::new(&keyword, &replacement, opt.regex);

    finder.is_recursive = opt.recursive;
    finder.follow_symlink = opt.symlink;
    finder.skip_vcs = opt.skip_vcs;
    finder.skip_gitignore = opt.skip_gitignore;
    finder.print_skipped = opt.skipped | opt.verbose;
    finder.find_parent_ignore = opt.parent_ignore;
    sorter.through = !opt.fixed_order;
    replacer.is_color = opt.color;
    replacer.is_interactive = opt.interactive;
    replacer.preserve_time = opt.preserve_time;
    replacer.print_file = opt.file;
    replacer.print_column = opt.column;
    replacer.print_row = opt.row;

    let use_regex = opt.regex;
    let use_tbm = opt.tbm;
    let skip_binary = !opt.binary;
    let print_skipped = opt.skipped | opt.verbose;
    let print_search = opt.verbose;
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
                matcher.print_search = print_search;
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
                matcher.print_search = print_search;
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
                matcher.print_search = print_search;
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
        console.write(ConsoleTextKind::Info, "\nStatistics\n");
        console.write(
            ConsoleTextKind::Info,
            &format!("  Max threads: {}\n\n", opt.max_threads),
        );
        console.write(ConsoleTextKind::Info, "  Consumed time ( busy / total )\n");
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
