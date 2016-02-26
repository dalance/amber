extern crate amber;
extern crate docopt;
extern crate num_cpus;
extern crate rustc_serialize;

use amber::console::{Console, ConsoleTextKind};
use amber::matcher::{RegexMatcher, QuickSearchMatcher, TbmMatcher};
use amber::pipeline::{Pipeline, PipelineFork, PipelineJoin, PipelineInfo};
use amber::pipeline_finder::PipelineFinder;
use amber::pipeline_matcher::PipelineMatcher;
use amber::pipeline_sorter::PipelineSorter;
use amber::pipeline_printer::PipelinePrinter;
use amber::util::{decode_error, read_from_file};
use docopt::Docopt;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::thread;

// ---------------------------------------------------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------------------------------------------------

#[allow(dead_code)]
static USAGE: &'static str = "
Search <keyword> from current directory or <paths>

Usage:
    ambs [options] ( <keyword> | --key-file <file> )
    ambs [options] ( <keyword> | --key-file <file> ) <paths>...
    ambs ( --help | --version )

Options:
    --key-file <file>          Use file contents as keyword
    --max-threads <num>        Number of max threads [default: num_cpus]
    --size-per-thread <bytes>  File size per one thread [default: 1048576]
    --bin-check-bytes <bytes>  Read size by byte for checking binary [default: 256]
    --regex                    Enable regular expression search
    --column                   Enable column output
    --binary                   Enable binary file search
    --statistics               Enable statistics output
    --skipped                  Enable skipped file output
    --no-recursive             Disable recursive directory search
    --no-symlink               Disable symbolic link follow
    --no-color                 Disable colored output
    --no-file                  Disable filename output
    --no-skip-vcs              Disable vcs directory ( .hg/.git/.svn ) skip
    --no-skip-gitignore        Disable .gitignore skip
    --no-fixed-order           Disable output order guarantee
    --no-parent-ignore         Disable .*ignore file search at parent directories
    -h --help                  Show this message
    -v --version               Show version

Experimental Options:
    --mmap-bytes <bytes>       Minimum size by byte for using mmap [default: 1048576]
    --tbm                      Enable TBM matcher
    --sse                      Enable SSE 4.2
";

#[allow(dead_code)]
static VERSION: &'static str = env!( "CARGO_PKG_VERSION" );
static BUILD_TIME  : Option<&'static str> = option_env!( "BUILD_TIME"   );
static GIT_REVISION: Option<&'static str> = option_env!( "GIT_REVISION" );

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_keyword           : String,
    arg_paths             : Vec<String>,
    flag_key_file         : Option<String>,
    flag_max_threads      : usize,
    flag_size_per_thread  : usize,
    flag_bin_check_bytes  : usize,
    flag_mmap_bytes       : u64,
    flag_regex            : bool,
    flag_column           : bool,
    flag_binary           : bool,
    flag_statistics       : bool,
    flag_skipped          : bool,
    flag_no_recursive     : bool,
    flag_no_symlink       : bool,
    flag_no_color         : bool,
    flag_no_file          : bool,
    flag_no_skip_vcs      : bool,
    flag_no_skip_gitignore: bool,
    flag_no_fixed_order   : bool,
    flag_no_parent_ignore : bool,
    flag_tbm              : bool,
    flag_sse              : bool,
}

// ---------------------------------------------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------------------------------------------

#[allow(dead_code)]
fn main() {

    // ---------------------------------------------------------------------------------------------
    // Parse Arguments
    // ---------------------------------------------------------------------------------------------

    // - Create config from Docopt ---------------------------------------------
    let version = if BUILD_TIME.is_some() {
        format!( "ambs version {} ( {} {} )", VERSION, GIT_REVISION.unwrap_or( "" ), BUILD_TIME.unwrap() )
    } else {
        format!( "ambs version {}", VERSION )
    };

    let usage = String::from( USAGE ).replace( "num_cpus", &format!( "{}", num_cpus::get() ) );
    let args: Args = Docopt::new( usage ).and_then( |d| d.version( Some( version ) ).decode() ).unwrap_or_else( |e| e.exit() );

    let mut console = Console::new();
    console.is_color = !args.flag_no_color;

    // - Set base path, keyword and replacement --------------------------------
    let mut base_paths:Vec<PathBuf> = Vec::new();
    if args.arg_paths.is_empty() {
        base_paths.push( PathBuf::from( "./" ) );
    } else {
        for p in &args.arg_paths {
            base_paths.push( PathBuf::from( p ) );
        }
    }

    let keyword = match args.flag_key_file {
        Some( f ) => {
            match read_from_file( &f ) {
                Ok ( x ) => {
                    if x.len() != 0 {
                        x
                    } else {
                        console.write( ConsoleTextKind::Error, &format!( "Error: file is empty @ {:?}\n", f ) );
                        process::exit( 1 );
                    }
                },
                Err( e ) => {
                    console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), f ) );
                    process::exit( 1 );
                },
            }
        },
        None => args.arg_keyword.clone().into_bytes()
    };

    // ---------------------------------------------------------------------------------------------
    // Pipeline Construct
    // ---------------------------------------------------------------------------------------------

    let id_finder   = 0;
    let id_sorter   = 1;
    let id_printer  = 2;
    let id_matcher  = 3;

    let matcher_num = args.flag_max_threads;

    let ( tx_finder  , rx_finder   ) = mpsc::channel();
    let ( tx_printer , rx_printer  ) = mpsc::channel();
    let ( tx_main    , rx_main     ) = mpsc::channel();

    let mut tx_matcher = Vec::new();
    let mut rx_sorter  = Vec::new();

    let mut finder   = PipelineFinder::new();
    let mut sorter   = PipelineSorter::new( matcher_num );
    let mut printer  = PipelinePrinter::new();

    finder.is_recursive       = !args.flag_no_recursive;
    finder.follow_symlink     = !args.flag_no_symlink;
    finder.skip_vcs           = !args.flag_no_skip_vcs;
    finder.skip_gitignore     = !args.flag_no_skip_gitignore;
    finder.print_skipped      = args.flag_skipped;
    finder.find_parent_ignore = !args.flag_no_parent_ignore;
    sorter.through            = args.flag_no_fixed_order;
    printer.is_color          = !args.flag_no_color;
    printer.print_file        = !args.flag_no_file;
    printer.print_column      = args.flag_column;

    let use_regex          = args.flag_regex          ;
    let use_tbm            = args.flag_tbm            ;
    let skip_binary        = !args.flag_binary        ;
    let print_skipped      = args.flag_skipped        ;
    let binary_check_bytes = args.flag_bin_check_bytes;
    let mmap_bytes         = args.flag_mmap_bytes     ;
    let max_threads        = args.flag_max_threads    ;
    let size_per_thread    = args.flag_size_per_thread;

    for i in 0..matcher_num {
        let keyword = keyword.clone();
        let ( tx_in , rx_in  ) = mpsc::channel();
        let ( tx_out, rx_out ) = mpsc::channel();
        tx_matcher.push( tx_in  );
        rx_sorter .push( rx_out );

        let _ = thread::Builder::new().name( "matcher".to_string() ).spawn( move || {
            if use_regex {
                let m = RegexMatcher::new();
                let mut matcher = PipelineMatcher::new( m, &keyword );
                matcher.skip_binary        = skip_binary       ;
                matcher.print_skipped      = print_skipped     ;
                matcher.binary_check_bytes = binary_check_bytes;
                matcher.mmap_bytes         = mmap_bytes        ;
                matcher.setup( id_matcher + i, rx_in, tx_out );
            } else if use_tbm {
                let mut m = TbmMatcher::new();
                m.max_threads     = max_threads;
                m.size_per_thread = size_per_thread;
                let mut matcher = PipelineMatcher::new( m, &keyword );
                matcher.skip_binary        = skip_binary       ;
                matcher.print_skipped      = print_skipped     ;
                matcher.binary_check_bytes = binary_check_bytes;
                matcher.mmap_bytes         = mmap_bytes        ;
                matcher.setup( id_matcher + i, rx_in, tx_out );
            } else {
                let mut m = QuickSearchMatcher::new();
                m.max_threads     = max_threads;
                m.size_per_thread = size_per_thread;
                let mut matcher = PipelineMatcher::new( m, &keyword );
                matcher.skip_binary        = skip_binary       ;
                matcher.print_skipped      = print_skipped     ;
                matcher.binary_check_bytes = binary_check_bytes;
                matcher.mmap_bytes         = mmap_bytes        ;
                matcher.setup( id_matcher + i, rx_in, tx_out );
            };
        } );
    }

    let _ = thread::Builder::new().name( "finder".to_string() ).spawn( move || {
        finder.setup( id_finder, rx_finder, tx_matcher );
    } );

    let _ = thread::Builder::new().name( "sorter".to_string() ).spawn( move || {
        sorter.setup( id_sorter, rx_sorter, tx_printer );
    } );

    let _ = thread::Builder::new().name( "printer".to_string() ).spawn( move || {
        printer.setup( id_printer, rx_printer, tx_main );
    } );

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let mut seq_no = 0;
    let _ = tx_finder.send( PipelineInfo::SeqBeg( seq_no ) );
    for p in base_paths {
        let _ = tx_finder.send( PipelineInfo::SeqDat( seq_no, p ) );
        seq_no += 1;
    }
    let _ = tx_finder.send( PipelineInfo::SeqEnd( seq_no ) );

    let mut time_finder_bsy   = 0;
    let mut time_finder_all   = 0;
    let mut time_sorter_bsy   = 0;
    let mut time_sorter_all   = 0;
    let mut time_printer_bsy  = 0;
    let mut time_printer_all  = 0;

    let mut time_matcher_bsy = Vec::new();
    let mut time_matcher_all = Vec::new();
    for _ in 0..matcher_num {
        time_matcher_bsy.push( 0 );
        time_matcher_all.push( 0 );
    }

    let mut count_finder  = 0;
    let mut count_matcher = 0;

    loop {
        match rx_main.try_recv() {
            Ok ( PipelineInfo::SeqEnd ( _          ) ) => break,
            Ok ( PipelineInfo::MsgTime( id, t0, t1 ) ) if id == id_finder   => { time_finder_bsy   = t0; time_finder_all   = t1; },
            Ok ( PipelineInfo::MsgTime( id, t0, t1 ) ) if id == id_sorter   => { time_sorter_bsy   = t0; time_sorter_all   = t1; },
            Ok ( PipelineInfo::MsgTime( id, t0, t1 ) ) if id == id_printer  => { time_printer_bsy  = t0; time_printer_all  = t1; },
            Ok ( PipelineInfo::MsgTime( id, t0, t1 ) ) => { time_matcher_bsy[id-id_matcher] = t0; time_matcher_all[id-id_matcher] = t1; },
            Ok ( PipelineInfo::MsgInfo( _id, s     ) ) => console.write( ConsoleTextKind::Info , &format!( "{}\n", s ) ),
            Ok ( PipelineInfo::MsgErr ( _id, s     ) ) => console.write( ConsoleTextKind::Error, &format!( "{}\n", s ) ),
            Ok ( _                                 ) => (),
            Err( _                                 ) => (),
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let sec_finder_bsy   = time_finder_bsy   as f64 / 1000000000.0;
    let sec_finder_all   = time_finder_all   as f64 / 1000000000.0;
    let sec_sorter_bsy   = time_sorter_bsy   as f64 / 1000000000.0;
    let sec_sorter_all   = time_sorter_all   as f64 / 1000000000.0;
    let sec_printer_bsy  = time_printer_bsy  as f64 / 1000000000.0;
    let sec_printer_all  = time_printer_all  as f64 / 1000000000.0;

    let mut sec_matcher_bsy = Vec::new();
    let mut sec_matcher_all = Vec::new();
    for i in 0..matcher_num {
        sec_matcher_bsy.push( time_matcher_bsy[i] as f64 / 1000000000.0 );
        sec_matcher_all.push( time_matcher_all[i] as f64 / 1000000000.0 );
    }

    if args.flag_statistics {
        console.write( ConsoleTextKind::Info, &format!( "\nStatistics\n" ) );
        console.write( ConsoleTextKind::Info, &format!( "  Max threads: {}\n\n" , args.flag_max_threads ) );
        console.write( ConsoleTextKind::Info, &format!( "  Consumed time ( busy / total )\n" ) );
        console.write( ConsoleTextKind::Info, &format!( "    Find     : {}s / {}s\n"  , sec_finder_bsy  , sec_finder_all   ) );
        for i in 0..matcher_num {
        console.write( ConsoleTextKind::Info, &format!( "    Match{:02}  : {}s / {}s\n"  , i, sec_matcher_bsy[i], sec_matcher_all[i] ) );
        }
        console.write( ConsoleTextKind::Info, &format!( "    Sort     : {}s / {}s\n\n", sec_sorter_bsy  , sec_sorter_all   ) );
        console.write( ConsoleTextKind::Info, &format!( "    Display  : {}s / {}s\n\n", sec_printer_bsy , sec_printer_all  ) );
        console.write( ConsoleTextKind::Info, &format!( "  Path count\n" ) );
        console.write( ConsoleTextKind::Info, &format!( "    Found    : {}\n"   , count_finder  ) );
        console.write( ConsoleTextKind::Info, &format!( "    Matched  : {}\n"   , count_matcher ) );
    }

    console.reset();
}
