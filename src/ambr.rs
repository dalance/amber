extern crate amber;
extern crate docopt;
extern crate num_cpus;
extern crate rustc_serialize;

use amber::console::{Console, ConsoleTextKind};
use amber::matcher::{Matcher, RegexMatcher, QuickSearchMatcher, TbmMatcher};
use amber::pipeline::{PipelineInfo};
//use amber::pipeline_finder::{PipelineFinder, SimplePipelineFinder};
//use amber::pipeline_matcher::{PipelineMatcher, SimplePipelineMatcher};
//use amber::pipeline_queue::{PipelineQueue, SimplePipelineQueue};
//use amber::pipeline_replacer::{PipelineReplacer, SimplePipelineReplacer};
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
Replace <keyword> to <replacement> from current directory or <paths>

Usage:
    ambr [options] ( <keyword> | --key-file <file> ) ( <replacement> | --rep-file <file> )
    ambr [options] ( <keyword> | --key-file <file> ) ( <replacement> | --rep-file <file> ) <paths>...
    ambr ( --help | --version )

Options:
    --key-file <file>          Use file contents as keyword
    --rep-file <file>          Use file contents as replacement
    --max-threads <num>        Number of max threads [default: num_cpus]
    --size-per-thread <bytes>  File size per one thread [default: 1048576]
    --bin-check-bytes <bytes>  Read size by byte for checking binary [default: 256]
    --regex                    Enable regular expression search
    --column                   Enable column output
    --binary                   Enable binary file search
    --statistics               Enable statistics output
    --skipped                  Enable skipped file output
    --no-progress              Disable progress output
    --no-interactive           Disable interactive replace
    --no-recursive             Disable recursive directory search
    --no-symlink               Disable symbolic link follow
    --no-color                 Disable colored output
    --no-file                  Disable filename output
    --no-skip-vcs              Disable vcs directory ( .hg/.git/.svn ) skip
    --no-skip-gitignore        Disable .gitignore skip
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
    arg_replacement       : String,
    arg_paths             : Vec<String>,
    flag_key_file         : Option<String>,
    flag_rep_file         : Option<String>,
    flag_max_threads      : usize,
    flag_size_per_thread  : usize,
    flag_bin_check_bytes  : usize,
    flag_mmap_bytes       : u64,
    flag_regex            : bool,
    flag_column           : bool,
    flag_binary           : bool,
    flag_statistics       : bool,
    flag_skipped          : bool,
    flag_no_progress      : bool,
    flag_no_interactive   : bool,
    flag_no_recursive     : bool,
    flag_no_symlink       : bool,
    flag_no_color         : bool,
    flag_no_file          : bool,
    flag_no_skip_vcs      : bool,
    flag_no_skip_gitignore: bool,
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

    let replacement = match args.flag_rep_file {
        Some( f ) => {
            match read_from_file( &f ) {
                Ok ( x ) => x,
                Err( e ) => {
                    console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), f ) );
                    process::exit( 1 );
                },
            }
        },
        None => args.arg_replacement.clone().into_bytes()
    };

    /*
    // ---------------------------------------------------------------------------------------------
    // Pipeline Construct
    // ---------------------------------------------------------------------------------------------

    let matcher_num = args.flag_max_threads;

    let ( finder_in_tx   , finder_in_rx    ) = mpsc::channel();
    let ( finder_out_tx  , finder_out_rx   ) = mpsc::channel();
    let ( queue_in_tx    , queue_in_rx     ) = mpsc::channel();
    let ( queue_out_tx   , queue_out_rx    ) = mpsc::channel();
    let ( replacer_in_tx , replacer_in_rx  ) = mpsc::channel();
    let ( replacer_out_tx, replacer_out_rx ) = mpsc::channel();

    let mut matcher_in_tx  = Vec::new();
    let mut matcher_out_rx = Vec::new();

    let mut finder   = SimplePipelineFinder::new();
    let mut queue    = SimplePipelineQueue::new();
    let mut replacer = SimplePipelineReplacer::new();

    finder.is_recursive     = !args.flag_no_recursive;
    finder.follow_symlink   = !args.flag_no_symlink;
    finder.skip_vcs         = !args.flag_no_skip_vcs;
    finder.skip_gitignore   = !args.flag_no_skip_gitignore;
    finder.print_skipped    = args.flag_skipped;
    replacer.is_color       = !args.flag_no_color;
    replacer.is_interactive = !args.flag_no_interactive;
    replacer.print_file     = !args.flag_no_file;
    replacer.print_column   = args.flag_column;

    let _ = thread::Builder::new().name( "finder".to_string() ).spawn( move || {
        finder.find( finder_in_rx, finder_out_tx );
    } );

    for _ in 0..matcher_num {
        let keyword = keyword.clone();
        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        matcher_in_tx .push( in_tx  );
        matcher_out_rx.push( out_rx );

        let mut matcher = SimplePipelineMatcher::new();
        matcher.skip_binary        = !args.flag_binary;
        matcher.print_skipped      = args.flag_skipped;
        matcher.binary_check_bytes = args.flag_bin_check_bytes;
        matcher.mmap_bytes         = args.flag_mmap_bytes;

        let max_threads     = args.flag_max_threads;
        let size_per_thread = args.flag_size_per_thread;
        let use_regex       = args.flag_regex;
        let use_tbm         = args.flag_tbm;
        let use_sse         = args.flag_sse;

        let _ = thread::Builder::new().name( "matcher".to_string() ).spawn( move || {
            let mut m_qs    = QuickSearchMatcher::new();
            let mut m_tbm   = TbmMatcher::new();
            let     m_regex = RegexMatcher::new();
            m_qs.max_threads      = max_threads;
            m_qs.size_per_thread  = size_per_thread;
            m_qs.use_sse          = use_sse;
            m_tbm.max_threads     = max_threads;
            m_tbm.size_per_thread = size_per_thread;
            m_tbm.use_sse         = use_sse;
            let m: &Matcher = if use_regex { &m_regex } else if use_tbm { &m_tbm } else { &m_qs };

            matcher.search( m, &keyword, in_rx, out_tx );
        } );
    }

    let _ = thread::Builder::new().name( "queue".to_string() ).spawn( move || {
        queue.exec( queue_in_rx, queue_out_tx );
    } );

    let _ = thread::Builder::new().name( "replacer".to_string() ).spawn( move || {
        replacer.replace( &replacement, replacer_in_rx, replacer_out_tx );
    } );

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let _ = finder_in_tx.send( PipelineInfo::Beg( 0 ) );
    for p in base_paths {
        let _ = finder_in_tx.send( PipelineInfo::Ok( p ) );
    }
    let _ = finder_in_tx.send( PipelineInfo::End( 1 ) );

    let mut time_finder_bsy   = 0;
    let mut time_finder_all   = 0;
    let mut time_queue_bsy    = 0;
    let mut time_queue_all    = 0;
    let mut time_replacer_bsy = 0;
    let mut time_replacer_all = 0;

    let mut time_matcher_bsy = Vec::new();
    let mut time_matcher_all = Vec::new();
    for _ in 0..matcher_num {
        time_matcher_bsy.push( 0 );
        time_matcher_all.push( 0 );
    }

    let mut count_finder  = 0;
    let mut count_matcher = 0;

    loop {
        match finder_out_rx.try_recv() {
            Ok ( PipelineInfo::Ok( x ) ) => {
                let _ = matcher_in_tx[count_finder % matcher_num].send( PipelineInfo::Ok( x ) );
                count_finder += 1;
            },
            Ok ( PipelineInfo::End( x ) ) => {
                for tx in &matcher_in_tx {
                    let _ = tx.send( PipelineInfo::End( x ) );
                }
            },
            Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_finder_bsy = t0; time_finder_all = t1; },
            Ok ( i                            ) => { let _ = matcher_in_tx[0].send( i ); },
            Err( _                            ) => (),
        }

        for i in 0..matcher_num {
            match matcher_out_rx[i].try_recv() {
                Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_matcher_bsy[i] = t0; time_matcher_all[i] = t1; },
                Ok ( PipelineInfo::Ok  ( x      ) ) => {
                    if !x.matches.is_empty() {
                        count_matcher += 1;
                    }
                    let _ = queue_in_tx.send( PipelineInfo::Ok( x ) );
                },
                Ok ( i                            ) => { let _ = queue_in_tx.send( i ); },
                Err( _                            ) => (),
            }
        }

        match queue_out_rx.try_recv() {
            Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_queue_bsy = t0; time_queue_all = t1; },
            Ok ( i                            ) => { let _ = replacer_in_tx.send( i ); },
            Err( _                            ) => (),
        }

        match replacer_out_rx.try_recv() {
            Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_replacer_bsy = t0; time_replacer_all = t1; },
            Ok ( PipelineInfo::Info( i      ) ) => console.write( ConsoleTextKind::Info , &format!( "{}\n", i ) ),
            Ok ( PipelineInfo::Err ( e      ) ) => console.write( ConsoleTextKind::Error, &format!( "{}\n", e ) ),
            Ok ( PipelineInfo::End ( _      ) ) => break,
            Ok ( _                            ) => (),
            Err( _                            ) => (),
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let sec_finder_bsy   = time_finder_bsy   as f64 / 1000000000.0;
    let sec_finder_all   = time_finder_all   as f64 / 1000000000.0;
    let sec_queue_bsy    = time_queue_bsy    as f64 / 1000000000.0;
    let sec_queue_all    = time_queue_all    as f64 / 1000000000.0;
    let sec_replacer_bsy = time_replacer_bsy as f64 / 1000000000.0;
    let sec_replacer_all = time_replacer_all as f64 / 1000000000.0;

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
        console.write( ConsoleTextKind::Info, &format!( "    Queue    : {}s / {}s\n\n", sec_queue_bsy   , sec_queue_all    ) );
        console.write( ConsoleTextKind::Info, &format!( "    Replace  : {}s / {}s\n"  , sec_replacer_bsy, sec_replacer_all ) );
        console.write( ConsoleTextKind::Info, &format!( "  Path count\n" ) );
        console.write( ConsoleTextKind::Info, &format!( "    Found    : {}\n"   , count_finder  ) );
        console.write( ConsoleTextKind::Info, &format!( "    Matched  : {}\n"   , count_matcher ) );
    }

    console.reset();
    */
}
