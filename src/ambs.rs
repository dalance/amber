extern crate amber;
extern crate docopt;
extern crate num_cpus;
extern crate rustc_serialize;

use amber::console::{Console, ConsoleTextKind};
use amber::matcher::{Matcher, RegexMatcher, QuickSearchMatcher, TbmMatcher};
use amber::pipeline_filter::{PipelineFilter, SimplePipelineFilter};
use amber::pipeline_finder::{PipelineFinder, SimplePipelineFinder};
use amber::pipeline_matcher::{PipelineMatcher, SimplePipelineMatcher};
use amber::pipeline_printer::{PipelinePrinter, SimplePipelinePrinter};
use amber::util::{decode_error, read_from_file, PipelineInfo};
use docopt::Docopt;
use std::cmp;
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
    --bin-check-bytes <bytes>  Read size by byte for checking binary [default: 1024]
    --regex                    Enable regular expression search
    --column                   Enable column output
    --binary                   Enable binary file search
    --statistics               Enable statistics output
    --skipped                  Enable skipped file output
    --no-progress              Disable progress output
    --no-recursive             Disable recursive directory search
    --no-symlink               Disable symbolic link follow
    --no-color                 Disable colored output
    --no-file                  Disable filename output
    --no-skip-vcs              Disable vcs directory ( .hg/.git/.svn ) skip
    -h --help                  Show this message
    -v --version               Show version

Experimental Options:
    --tbm                      Enable TBM matcher
    --sse                      Enable SSE 4.2
";

#[allow(dead_code)]
static VERSION: &'static str = env!( "CARGO_PKG_VERSION" );

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_keyword         : String,
    arg_paths           : Vec<String>,
    flag_key_file       : Option<String>,
    flag_max_threads    : usize,
    flag_size_per_thread: usize,
    flag_bin_check_bytes: usize,
    flag_regex          : bool,
    flag_column         : bool,
    flag_binary         : bool,
    flag_statistics     : bool,
    flag_skipped        : bool,
    flag_no_progress    : bool,
    flag_no_recursive   : bool,
    flag_no_symlink     : bool,
    flag_no_color       : bool,
    flag_no_file        : bool,
    flag_no_skip_vcs    : bool,
    flag_tbm            : bool,
    flag_sse            : bool,
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
    let version = format!( "ambs version {}", VERSION );

    let usage = String::from( USAGE ).replace( "num_cpus", &format!( "{}", num_cpus::get() * 4 ) );
    let args: Args = Docopt::new( usage ).and_then( |d| d.version( Some( version ) ).decode() ).unwrap_or_else( |e| e.exit() );

    let mut console = Console::new();
    console.is_color = !args.flag_no_color;

    // - Set base path, keyword and replacemente -------------------------------
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

    let ( finder_in_tx   , finder_in_rx    ) = mpsc::channel();
    let ( finder_out_tx  , finder_out_rx   ) = mpsc::channel();
    let ( filter_in_tx   , filter_in_rx    ) = mpsc::channel();
    let ( filter_out_tx  , filter_out_rx   ) = mpsc::channel();
    let ( matcher_in_tx  , matcher_in_rx   ) = mpsc::channel();
    let ( matcher_out_tx , matcher_out_rx  ) = mpsc::channel();
    let ( printer_in_tx  , printer_in_rx   ) = mpsc::channel();
    let ( printer_out_tx , printer_out_rx  ) = mpsc::channel();

    let mut finder   = SimplePipelineFinder::new();
    let mut filter   = SimplePipelineFilter::new();
    let mut matcher  = SimplePipelineMatcher::new();
    let mut printer  = SimplePipelinePrinter::new();

    finder.is_recursive        = !args.flag_no_recursive;
    finder.follow_symlink      = !args.flag_no_symlink;
    filter.skip_vcs            = !args.flag_no_skip_vcs;
    filter.print_skipped       = args.flag_skipped;
    matcher.skip_binary        = !args.flag_binary;
    matcher.print_skipped      = args.flag_skipped;
    matcher.binary_check_bytes = args.flag_bin_check_bytes;
    printer.is_color           = !args.flag_no_color;
    printer.print_file         = !args.flag_no_file;
    printer.print_column       = args.flag_column;

    let max_threads     = cmp::max( args.flag_max_threads - 4, 1 );
    let size_per_thread = args.flag_size_per_thread;
    let regex           = args.flag_regex;
    let tbm             = args.flag_tbm;
    let sse             = args.flag_sse;

    let _ = thread::Builder::new().name( "finder".to_string() ).spawn( move || {
        finder.find( finder_in_rx, finder_out_tx );
    } );

    let _ = thread::Builder::new().name( "filter".to_string() ).spawn( move || {
        filter.filter( filter_in_rx, filter_out_tx );
    } );

    let _ = thread::Builder::new().name( "matcher".to_string() ).spawn( move || {
        let mut m_qs    = QuickSearchMatcher::new();
        let mut m_tbm   = TbmMatcher::new();
        let     m_regex = RegexMatcher::new();
        m_qs.max_threads      = max_threads;
        m_qs.size_per_thread  = size_per_thread;
        m_qs.use_sse          = sse;
        m_tbm.max_threads     = max_threads;
        m_tbm.size_per_thread = size_per_thread;
        m_tbm.use_sse         = sse;
        let m: &Matcher = if regex { &m_regex } else if tbm { &m_tbm } else { &m_qs };

        matcher.search( m, &keyword, matcher_in_rx, matcher_out_tx );
    } );

    let _ = thread::Builder::new().name( "printer".to_string() ).spawn( move || {
        printer.print( printer_in_rx, printer_out_tx );
    } );

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let _ = finder_in_tx.send( PipelineInfo::Begin );
    for p in base_paths {
        let _ = finder_in_tx.send( PipelineInfo::Ok( p ) );
    }
    let _ = finder_in_tx.send( PipelineInfo::End );

    let mut time_finder_bsy   = 0;
    let mut time_finder_all   = 0;
    let mut time_filter_bsy   = 0;
    let mut time_filter_all   = 0;
    let mut time_matcher_bsy  = 0;
    let mut time_matcher_all  = 0;
    let mut time_printer_bsy  = 0;
    let mut time_printer_all  = 0;

    let mut count_finder  = 0;
    let mut count_filter  = 0;
    let mut count_matcher = 0;

    loop {
        match finder_out_rx.try_recv() {
            Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_finder_bsy = t0; time_finder_all = t1; },
            Ok ( PipelineInfo::Ok  ( x      ) ) => { count_finder += 1; let _ = filter_in_tx.send( PipelineInfo::Ok( x ) ); },
            Ok ( i                            ) => { let _ = filter_in_tx.send( i ); },
            Err( _                            ) => (),
        }
        match filter_out_rx.try_recv() {
            Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_filter_bsy = t0; time_filter_all = t1; },
            Ok ( PipelineInfo::Ok  ( x      ) ) => { count_filter += 1; let _ = matcher_in_tx.send( PipelineInfo::Ok( x ) ); },
            Ok ( i                            ) => { let _ = matcher_in_tx.send( i ); },
            Err( _                            ) => (),
        }
        match matcher_out_rx.try_recv() {
            Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_matcher_bsy = t0; time_matcher_all = t1; },
            Ok ( PipelineInfo::Ok  ( x      ) ) => { count_matcher += 1; let _ = printer_in_tx.send( PipelineInfo::Ok( x ) ); },
            Ok ( i                            ) => { let _ = printer_in_tx.send( i ); },
            Err( _                            ) => (),
        }
        match printer_out_rx.try_recv() {
            Ok ( PipelineInfo::Time( t0, t1 ) ) => { time_printer_bsy = t0; time_printer_all = t1; },
            Ok ( PipelineInfo::Info( i      ) ) => console.write( ConsoleTextKind::Info , &format!( "{}\n", i ) ),
            Ok ( PipelineInfo::Err ( e      ) ) => console.write( ConsoleTextKind::Error, &format!( "{}\n", e ) ),
            Ok ( PipelineInfo::End            ) => break,
            Ok ( _                            ) => (),
            Err( _                            ) => (),
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Pipeline Flow
    // ---------------------------------------------------------------------------------------------

    let sec_finder_bsy   = time_finder_bsy   as f64 / 1000000000.0;
    let sec_finder_all   = time_finder_all   as f64 / 1000000000.0;
    let sec_filter_bsy   = time_filter_bsy   as f64 / 1000000000.0;
    let sec_filter_all   = time_filter_all   as f64 / 1000000000.0;
    let sec_matcher_bsy  = time_matcher_bsy  as f64 / 1000000000.0;
    let sec_matcher_all  = time_matcher_all  as f64 / 1000000000.0;
    let sec_printer_bsy  = time_printer_bsy  as f64 / 1000000000.0;
    let sec_printer_all  = time_printer_all  as f64 / 1000000000.0;

    if args.flag_statistics {
        console.write( ConsoleTextKind::Info, &format!( "\nStatistics\n" ) );
        console.write( ConsoleTextKind::Info, &format!( "  Max threads: {}\n\n" , args.flag_max_threads ) );
        console.write( ConsoleTextKind::Info, &format!( "  Consumed time ( busy / total )\n" ) );
        console.write( ConsoleTextKind::Info, &format!( "    Find     : {}s / {}s\n"  , sec_finder_bsy  , sec_finder_all   ) );
        console.write( ConsoleTextKind::Info, &format!( "    Filter   : {}s / {}s\n"  , sec_filter_bsy  , sec_filter_all   ) );
        console.write( ConsoleTextKind::Info, &format!( "    Match    : {}s / {}s\n"  , sec_matcher_bsy , sec_matcher_all  ) );
        console.write( ConsoleTextKind::Info, &format!( "    Display  : {}s / {}s\n\n", sec_printer_bsy , sec_printer_all  ) );
        console.write( ConsoleTextKind::Info, &format!( "  Path count\n" ) );
        console.write( ConsoleTextKind::Info, &format!( "    Found    : {}\n"   , count_finder  ) );
        console.write( ConsoleTextKind::Info, &format!( "    Filtered : {}\n"   , count_filter  ) );
        console.write( ConsoleTextKind::Info, &format!( "    Matched  : {}\n"   , count_matcher ) );
    }
}
