extern crate docopt;
extern crate memmap;
extern crate num_cpus;
extern crate rand;
extern crate regex;
extern crate rustc_serialize;
extern crate scoped_threadpool;
extern crate time;
extern crate tempfile;
extern crate term;

use console::{Console, ConsoleTextKind};
use matcher::{Match, Matcher, QuickSearchMatcher, RegexMatcher};
use path_finder::{PathFinder, SimplePathFinder};
use util::{catch, decode_error, read_from_file, watch_time};
use docopt::Docopt;
use memmap::{Mmap, Protection};
use std::env;
use std::fs;
use std::io;
use std::io::{Write, Error};
use std::path::{Component, Path, PathBuf};
use std::process;
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------------------------------------------

#[allow(dead_code)]
static USAGE_AMBS: &'static str = "
Search <keyword> from current directory or <paths>

Usage:
    ambs [options] ( <keyword> | --key-file <file> )
    ambs [options] ( <keyword> | --key-file <file> ) <paths>...
    ambs ( --help | --version )

Options:
    --key-file <file>          Use file contents as keyword
    --max-threads <num>        Number of max threads ( default: auto )
    --size-per-thread <bytes>  File size per one thread ( default: 1024 * 1024 = 1MB )
    --bin-check-bytes <bytes>  Read size by byte for checking binary ( default: 1024 )
    --regex                    Enable regular expression search
    --column                   Enable column output
    --binary                   Enable binary file search
    --statistics               Enable statistics output
    --skipped                  Enable skipped file output
    --no-interactive           Disable interactive replace
    --no-recursive             Disable recursive directory search
    --no-symlink               Disable symbolic link follow
    --no-color                 Disable colored output
    --no-file                  Disable filename output
    --no-skip-vcs              Disable vcs directory ( .hg/.git/.svn ) skip
    -h --help                  Show this message
    -v --version               Show version
";

#[allow(dead_code)]
static USAGE_AMBR: &'static str = "
Replace <keyword> to <replacement> from current directory or <paths>

Usage:
    ambr [options] ( <keyword> | --key-file <file> ) ( <replacement> | --rep-file <file> )
    ambr [options] ( <keyword> | --key-file <file> ) ( <replacement> | --rep-file <file> ) <paths>...
    ambr ( --help | --version )

Options:
    --key-file <file>          Use file contents as keyword
    --rep-file <file>          Use file contents as replacement
    --max-threads <num>        Number of max threads ( default: auto )
    --size-per-thread <bytes>  File size per one thread ( default: 1024 * 1024 = 1MB )
    --bin-check-bytes <bytes>  Read size by byte for checking binary ( default: 1024 )
    --regex                    Enable regular expression search
    --column                   Enable column output
    --binary                   Enable binary file search
    --statistics               Enable statistics output
    --skipped                  Enable skipped file output
    --no-interactive           Disable interactive replace
    --no-recursive             Disable recursive directory search
    --no-symlink               Disable symbolic link follow
    --no-color                 Disable colored output
    --no-file                  Disable filename output
    --no-skip-vcs              Disable vcs directory ( .hg/.git/.svn ) skip
    -h --help                  Show this message
    -v --version               Show version
";

#[allow(dead_code)]
static VERSION: &'static str = env!( "CARGO_PKG_VERSION" );

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_keyword         : String,
    arg_replacement     : String,
    arg_paths           : Vec<String>,
    flag_key_file       : Option<String>,
    flag_rep_file       : Option<String>,
    flag_max_threads    : Option<usize>,
    flag_size_per_thread: Option<usize>,
    flag_bin_check_bytes: Option<usize>,
    flag_regex          : bool,
    flag_column         : bool,
    flag_binary         : bool,
    flag_statistics     : bool,
    flag_skipped        : bool,
    flag_no_interactive : bool,
    flag_no_recursive   : bool,
    flag_no_symlink     : bool,
    flag_no_color       : bool,
    flag_no_file        : bool,
    flag_no_skip_vcs    : bool,
}

impl Args {
    fn from_ambs( args: &ArgsAmbs ) -> Self {
        Args {
            arg_keyword          : args.arg_keyword         .clone(),
            arg_replacement      : String::new()                    ,
            arg_paths            : args.arg_paths           .clone(),
            flag_key_file        : args.flag_key_file       .clone(),
            flag_rep_file        : None                             ,
            flag_max_threads     : args.flag_max_threads    .clone(),
            flag_size_per_thread : args.flag_size_per_thread.clone(),
            flag_bin_check_bytes : args.flag_bin_check_bytes.clone(),
            flag_regex           : args.flag_regex                  ,
            flag_column          : args.flag_column                 ,
            flag_binary          : args.flag_binary                 ,
            flag_statistics      : args.flag_statistics             ,
            flag_skipped         : args.flag_skipped                ,
            flag_no_interactive  : args.flag_no_interactive         ,
            flag_no_recursive    : args.flag_no_recursive           ,
            flag_no_symlink      : args.flag_no_symlink             ,
            flag_no_color        : args.flag_no_color               ,
            flag_no_file         : args.flag_no_file                ,
            flag_no_skip_vcs     : args.flag_no_skip_vcs            ,
        }
    }
    fn from_ambr( args: &ArgsAmbr ) -> Self {
        Args {
            arg_keyword          : args.arg_keyword         .clone(),
            arg_replacement      : args.arg_replacement     .clone(),
            arg_paths            : args.arg_paths           .clone(),
            flag_key_file        : args.flag_key_file       .clone(),
            flag_rep_file        : args.flag_rep_file       .clone(),
            flag_max_threads     : args.flag_max_threads    .clone(),
            flag_size_per_thread : args.flag_size_per_thread.clone(),
            flag_bin_check_bytes : args.flag_bin_check_bytes.clone(),
            flag_regex           : args.flag_regex                  ,
            flag_column          : args.flag_column                 ,
            flag_binary          : args.flag_binary                 ,
            flag_statistics      : args.flag_statistics             ,
            flag_skipped         : args.flag_skipped                ,
            flag_no_interactive  : args.flag_no_interactive         ,
            flag_no_recursive    : args.flag_no_recursive           ,
            flag_no_symlink      : args.flag_no_symlink             ,
            flag_no_color        : args.flag_no_color               ,
            flag_no_file         : args.flag_no_file                ,
            flag_no_skip_vcs     : args.flag_no_skip_vcs            ,
        }
    }
}

#[derive(RustcDecodable, Debug)]
struct ArgsAmbs {
    arg_keyword         : String,
    arg_paths           : Vec<String>,
    flag_key_file       : Option<String>,
    flag_max_threads    : Option<usize>,
    flag_size_per_thread: Option<usize>,
    flag_bin_check_bytes: Option<usize>,
    flag_regex          : bool,
    flag_column         : bool,
    flag_binary         : bool,
    flag_statistics     : bool,
    flag_skipped        : bool,
    flag_no_interactive : bool,
    flag_no_recursive   : bool,
    flag_no_symlink     : bool,
    flag_no_color       : bool,
    flag_no_file        : bool,
    flag_no_skip_vcs    : bool,
}

#[derive(RustcDecodable, Debug)]
struct ArgsAmbr {
    arg_keyword         : String,
    arg_replacement     : String,
    arg_paths           : Vec<String>,
    flag_key_file       : Option<String>,
    flag_rep_file       : Option<String>,
    flag_max_threads    : Option<usize>,
    flag_size_per_thread: Option<usize>,
    flag_bin_check_bytes: Option<usize>,
    flag_regex          : bool,
    flag_column         : bool,
    flag_binary         : bool,
    flag_statistics     : bool,
    flag_skipped        : bool,
    flag_no_interactive : bool,
    flag_no_recursive   : bool,
    flag_no_symlink     : bool,
    flag_no_color       : bool,
    flag_no_file        : bool,
    flag_no_skip_vcs    : bool,
}

#[derive(PartialEq)]
enum ProgMode {
    Search ,
    Replace,
}

#[allow(dead_code)]
pub fn main() {

    // ---------------------------------------------------------------------------------------------
    // Parse Arguments
    // ---------------------------------------------------------------------------------------------

    // - Judge mode by program name --------------------------------------------
    let args: Vec<String> = env::args().collect();
    let program_path = args[0].clone();
    let program_name = Path::new( &program_path ).file_name();
    let program_mode = match program_name {
        Some( p ) if p.to_str().unwrap_or( "" ).contains( "ambs" ) => ProgMode::Search ,
        Some( p ) if p.to_str().unwrap_or( "" ).contains( "ambr" ) => ProgMode::Replace,
        _                                                          => ProgMode::Search ,
    };

    // - Create config from Docopt ---------------------------------------------
    let version = format!( "Version: {}", VERSION );

    let args = match program_mode {
        ProgMode::Search => {
            let args_ambs: ArgsAmbs = Docopt::new( USAGE_AMBS ).and_then( |d| d.version( Some( version ) ).decode() ).unwrap_or_else( |e| e.exit() );
            Args::from_ambs( &args_ambs )
        },
        ProgMode::Replace => {
            let args_ambr: ArgsAmbr = Docopt::new( USAGE_AMBR ).and_then( |d| d.version( Some( version ) ).decode() ).unwrap_or_else( |e| e.exit() );
            Args::from_ambr( &args_ambr )
        },
    };

    let conf = Config::from_args( &args );

    let mut console = Console::new();
    console.is_color = conf.display_color;

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

    // ---------------------------------------------------------------------------------------------
    // Find Paths
    // ---------------------------------------------------------------------------------------------

    let mut found_paths: Vec<PathBuf> = Vec::new();

    let path_find_time = watch_time ( || {
        let mut finder = SimplePathFinder::new();
        finder.is_recursive   = conf.find_recursive;
        finder.follow_symlink = conf.find_follow_symlink;

        let ret = finder.find( base_paths );
        for r in ret {
            found_paths.push( r );
        }

        for e in &finder.errors {
            console.write( ConsoleTextKind::Error, &format!( "Error: {}\n", e ) );
        }
    } );

    // ---------------------------------------------------------------------------------------------
    // Path Filter
    // ---------------------------------------------------------------------------------------------

    let mut filtered_paths: Vec<PathBuf> = Vec::new();

    let path_filter_time = watch_time ( || {
        for path in &found_paths {
            let result = catch::<_, (), Error> ( || {
                let dir = path.parent().unwrap();
                let is_vcs_dir = dir.components().any( |x| {
                    match x {
                        Component::Normal( p ) if p == ".hg"  => true,
                        Component::Normal( p ) if p == ".git" => true,
                        Component::Normal( p ) if p == ".svn" => true,
                        Component::Normal( p ) if p == ".bzr" => true,
                        _                                     => false,
                    }
                } );
                if is_vcs_dir & conf.filter_vcs {
                    if conf.filter_show {
                        console.write( ConsoleTextKind::Other, &format!( "Skipped: {:?} ( vcs file )\n", path ) );
                    }
                    return Ok(())
                }

                let mmap = try!( Mmap::open_path( &path, Protection::Read ) );
                let src  = unsafe { mmap.as_slice() };

                let mut is_binary = false;
                let bin_check_len = if conf.filter_bin_check_bytes < src.len() { conf.filter_bin_check_bytes } else { src.len() };
                for i in 0..bin_check_len {
                    if src[i] <= 0x08 {
                        is_binary = true;
                    }
                }
                if is_binary & conf.filter_binary {
                    if conf.filter_show {
                        console.write( ConsoleTextKind::Other, &format!( "Skipped: {:?} ( binary file )\n", path ) );
                    }
                    return Ok(())
                }

                filtered_paths.push( path.clone() );

                Ok(())
            } );
            match result {
                Ok ( _ ) => (),
                Err( e ) => console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), path ) ),
            }
        }
    } );

    // ---------------------------------------------------------------------------------------------
    // Match
    // ---------------------------------------------------------------------------------------------

    struct MatchedPath {
        path   : PathBuf,
        matches: Vec<Match>,
    }

    let mut matched: Vec<MatchedPath> = Vec::new();

    let match_time = watch_time ( || {
        let mut matcher_qs    = QuickSearchMatcher::new();
        matcher_qs.max_threads     = conf.match_max_threads;
        matcher_qs.size_per_thread = conf.match_size_per_thread;
        let matcher_regex = RegexMatcher::new();
        let matcher: &Matcher = if conf.match_regex { &matcher_regex } else { &matcher_qs };

        for path in &filtered_paths {
            let result = catch::<_, (), Error> ( || {
                let mmap = try!( Mmap::open_path( &path, Protection::Read ) );
                let src  = unsafe { mmap.as_slice() };
                let ret  = matcher.search( src, &keyword );

                if !ret.is_empty() {
                    matched.push( MatchedPath { path: path.clone(), matches: ret } );
                }

                Ok(())
            } );
            match result {
                Ok ( _ ) => (),
                Err( e ) => console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), path ) ),
            }
        }
    } );

    /*
    let match_time = watch_time ( || {
        let ( tx, rx ) = mpsc::channel();

        let mut thread_paths: Vec<Vec<PathBuf>> = Vec::new();
        for _ in 0..conf.match_threads {
            thread_paths.push( Vec::new() );
        }
        for i in 0..filtered_paths.len() {
            thread_paths[i%conf.match_threads].push( filtered_paths[i].clone() );
        }

        for i in 0..conf.match_threads {
            let tx           = tx.clone();
            let conf         = conf.clone();
            let keyword      = keyword.clone();
            let thread_paths = thread_paths[i].clone();
            thread::spawn( move || {
                let matcher_qs    = QuickSearchMatcher::new();
                let matcher_regex = RegexMatcher::new();
                let matcher: &Matcher = if conf.match_regex { &matcher_regex } else { &matcher_qs };

                let mut rets: Vec<MatchedPath> = Vec::new();

                for path in thread_paths {
                    let mmap = Mmap::open_path( &path, Protection::Read ).unwrap();
                    let src  = unsafe { mmap.as_slice() };
                    let ret  = matcher.search( src, &keyword );

                    if !ret.is_empty() {
                        rets.push( MatchedPath { path: path.clone(), matches: ret } );
                    }
                }
                tx.send( rets )
            } );
        }

        for _ in 0..conf.match_threads {
            let ret = rx.recv().unwrap();
            for r in ret {
                matched.push( r );
            }
        }
    } );
    */

    // ---------------------------------------------------------------------------------------------
    // Replace
    // ---------------------------------------------------------------------------------------------

    let replace_time = watch_time ( || {
        if program_mode == ProgMode::Search { return }

        for mpath in &matched {

            let result = catch::<_, (), Error> ( || {
                let tmpfile_dir = mpath.path.parent().unwrap();
                let mut tmpfile = try!( NamedTempFile::new_in( tmpfile_dir ) );

                {
                    let mmap = try!( Mmap::open_path( &mpath.path, Protection::Read ) );
                    let src  = unsafe { mmap.as_slice() };

                    let mut i = 0;
                    let mut all_replace = false;
                    for m in &mpath.matches {
                        try!( tmpfile.write_all( &src[i..m.beg] ) );

                        let mut do_replace = true;
                        if conf.replace_interactive & !all_replace {
                            if conf.display_file {
                                console.write( ConsoleTextKind::Filename, mpath.path.to_str().unwrap() );
                                console.write( ConsoleTextKind::Other, ": " );
                            }
                            console.write_match_line( src, m );

                            loop {
                                console.write( ConsoleTextKind::Other, "Replace keyword? ( Yes[Y], No[N], All[A], Quit[Q] ):" );
                                let mut buf = String::new();
                                io::stdin().read_line( &mut buf ).unwrap();
                                match buf.trim().to_lowercase().as_ref() {
                                    "y"    => { do_replace  = true ; break },
                                    "yes"  => { do_replace  = true ; break },
                                    "n"    => { do_replace  = false; break },
                                    "no"   => { do_replace  = false; break },
                                    "a"    => { all_replace = true ; break },
                                    "all"  => { all_replace = true ; break },
                                    "q"    => { let _ = tmpfile.close(); process::exit( 0 ) },
                                    "quit" => { let _ = tmpfile.close(); process::exit( 0 ) },
                                    _      => continue,
                                }
                            }
                        }

                        if do_replace {
                            try!( tmpfile.write_all( &replacement ) );
                        } else {
                            try!( tmpfile.write_all( &src[m.beg..m.end] ) );
                        }
                        i = m.end;
                    }

                    if i < src.len() {
                        try!( tmpfile.write_all( &src[i..src.len()] ) );
                    }
                    try!( tmpfile.flush() );
                }

                let metadata = try!( fs::metadata( &mpath.path ) );

                try!( fs::rename( tmpfile.path(), &mpath.path ) );
                try!( fs::set_permissions( &mpath.path, metadata.permissions() ) );

                Ok(())
            } );
            match result {
                Ok ( _ ) => (),
                Err( e ) => console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), mpath.path ) ),
            }
        }
    } );

    // ---------------------------------------------------------------------------------------------
    // Display
    // ---------------------------------------------------------------------------------------------

    let display_time = watch_time ( || {
        if program_mode == ProgMode::Replace { return }

        for mpath in &matched {
            let result = catch::<_, (), Error> ( || {
                let mmap = try!( Mmap::open_path( &mpath.path, Protection::Read ) );
                let src  = unsafe { mmap.as_slice() };

                let mut pos     = 0;
                let mut column  = 0;
                let mut last_lf = 0;
                for m in &mpath.matches {
                    if conf.display_file {
                        console.write( ConsoleTextKind::Filename, mpath.path.to_str().unwrap() );
                        console.write( ConsoleTextKind::Other, ":" );
                    }
                    if conf.display_column {
                        while pos < m.beg {
                            if src[pos] == 0x0a {
                                column += 1;
                                last_lf = pos;
                            }
                            pos += 1;
                        }
                        console.write( ConsoleTextKind::Other, &format!( "{}:{}:", column + 1, m.beg - last_lf ) );
                    }

                    console.write_match_line( src, m );
                }

                Ok(())
            } );
            match result {
                Ok ( _ ) => (),
                Err( e ) => console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), mpath.path ) ),
            }
        }
    } );

    if conf.display_statistics {
        console.write( ConsoleTextKind::Other, &format!( "\nStatistics\n" ) );
        console.write( ConsoleTextKind::Other, &format!( "  Max Threads        : {}\n\n" , conf.match_max_threads ) );
        console.write( ConsoleTextKind::Other, &format!( "  Path Find Time     : {}s\n"  , path_find_time   as f64 / 1000000000.0 ) );
        console.write( ConsoleTextKind::Other, &format!( "  Path Filter Time   : {}s\n"  , path_filter_time as f64 / 1000000000.0 ) );
        console.write( ConsoleTextKind::Other, &format!( "  Match Time         : {}s\n"  , match_time       as f64 / 1000000000.0 ) );
        if program_mode == ProgMode::Replace {
        console.write( ConsoleTextKind::Other, &format!( "  Replace Time       : {}s\n"  , replace_time     as f64 / 1000000000.0 ) );
        };
        console.write( ConsoleTextKind::Other, &format!( "  Display Time       : {}s\n\n", display_time     as f64 / 1000000000.0 ) );
        console.write( ConsoleTextKind::Other, &format!( "  Found File Count   : {}\n"   , found_paths.len()    ) );
        console.write( ConsoleTextKind::Other, &format!( "  Filtered File Count: {}\n"   , filtered_paths.len() ) );
        console.write( ConsoleTextKind::Other, &format!( "  Matched File Count : {}\n"   , matched.len()        ) );
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Config {
    find_recursive        : bool ,
    find_follow_symlink   : bool ,
    filter_vcs            : bool ,
    filter_binary         : bool ,
    filter_show           : bool ,
    filter_bin_check_bytes: usize,
    match_regex           : bool ,
    match_max_threads     : usize,
    match_size_per_thread : usize,
    replace_interactive   : bool ,
    display_color         : bool ,
    display_file          : bool ,
    display_column        : bool ,
    display_statistics    : bool ,
}

impl Config {
    fn from_args( args: &Args ) -> Self {
        Config {
            find_recursive        : !args.flag_no_recursive,
            find_follow_symlink   : !args.flag_no_symlink,
            filter_vcs            : !args.flag_no_skip_vcs,
            filter_binary         : !args.flag_binary,
            filter_show           : args.flag_skipped,
            filter_bin_check_bytes: args.flag_bin_check_bytes.unwrap_or( 1024 ),
            match_regex           : args.flag_regex,
            match_max_threads     : args.flag_max_threads.unwrap_or( num_cpus::get() ),
            match_size_per_thread : args.flag_size_per_thread.unwrap_or( 1024 * 1024 ),
            replace_interactive   : !args.flag_no_interactive,
            display_color         : !args.flag_no_color,
            display_file          : !args.flag_no_file,
            display_column        : args.flag_column,
            display_statistics    : args.flag_statistics,
        }
    }
}

