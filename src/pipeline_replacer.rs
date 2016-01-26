use console::{Console, ConsoleTextKind};
use memmap::{Mmap, Protection};
use pipeline_matcher::PathMatch;
use std::env;
use std::fs;
use std::io;
use std::io::{Write, Error};
use std::process;
use std::sync::mpsc::{Receiver, Sender};
use time;
use util::{catch, decode_error, PipelineInfo};
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------------------------------------------------
// PipelineReplacer
// ---------------------------------------------------------------------------------------------------------------------

pub trait PipelineReplacer {
    fn replace( &mut self, replacement: &[u8], rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<()>> );
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePipelineReplacer
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePipelineReplacer {
    pub is_color      : bool,
    pub is_interactive: bool,
    pub print_file    : bool,
    pub print_column  : bool,
    pub infos         : Vec<String>,
    pub errors        : Vec<String>,
    console           : Console,
    all_replace       : bool,
    time_beg          : u64,
    time_end          : u64,
    time_bsy          : u64,
}

impl SimplePipelineReplacer {
    pub fn new() -> Self {
        SimplePipelineReplacer {
            is_color      : true,
            is_interactive: true,
            print_file    : true,
            print_column  : true,
            infos         : Vec::new(),
            errors        : Vec::new(),
            console       : Console::new(),
            all_replace   : false,
            time_beg      : 0,
            time_end      : 0,
            time_bsy      : 0,
        }
    }

    fn replace_match( &mut self, replacement: &[u8], pm: PathMatch ) {
        self.console.is_color = self.is_color;

        let result = catch::<_, (), Error> ( || {
            let tmpfile_dir = env::temp_dir();
            let mut tmpfile = try!( NamedTempFile::new_in( tmpfile_dir ) );

            {
                let mmap = try!( Mmap::open_path( &pm.path, Protection::Read ) );
                let src  = unsafe { mmap.as_slice() };

                let mut i       = 0;
                let mut pos     = 0;
                let mut column  = 0;
                let mut last_lf = 0;
                for m in &pm.matches {
                    try!( tmpfile.write_all( &src[i..m.beg] ) );

                    let mut do_replace = true;
                    if self.is_interactive & !self.all_replace {
                        if self.print_file {
                            self.console.write( ConsoleTextKind::Filename, pm.path.to_str().unwrap() );
                            self.console.write( ConsoleTextKind::Other, ": " );
                        }
                        if self.print_column {
                            while pos < m.beg {
                                if src[pos] == 0x0a {
                                    column += 1;
                                    last_lf = pos;
                                }
                                pos += 1;
                            }
                            self.console.write( ConsoleTextKind::Other, &format!( "{}:{}:", column + 1, m.beg - last_lf ) );
                        }

                        self.console.write_match_line( src, m );

                        loop {
                            self.console.write( ConsoleTextKind::Other, "Replace keyword? ( Yes[Y], No[N], All[A], Quit[Q] ):" );
                            let mut buf = String::new();
                            io::stdin().read_line( &mut buf ).unwrap();
                            match buf.trim().to_lowercase().as_ref() {
                                "y"    => { do_replace = true ; break },
                                "yes"  => { do_replace = true ; break },
                                "n"    => { do_replace = false; break },
                                "no"   => { do_replace = false; break },
                                "a"    => { self.all_replace = true ; break },
                                "all"  => { self.all_replace = true ; break },
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

            let metadata = try!( fs::metadata( &pm.path ) );

            try!( fs::rename( tmpfile.path(), &pm.path ) );
            try!( fs::set_permissions( &pm.path, metadata.permissions() ) );

            Ok(())
        } );
        match result {
            Ok ( _ ) => (),
            Err( e ) => self.console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), pm.path ) ),
        }
    }
}

impl PipelineReplacer for SimplePipelineReplacer {
    fn replace( &mut self, replacement: &[u8], rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<()>> ) {
        loop {
            match rx.recv() {
                Ok( PipelineInfo::Ok( pm ) ) => {
                    let beg = time::precise_time_ns();

                    self.replace_match( replacement, pm );
                    let _ = tx.send( PipelineInfo::Ok( () ) );

                    let end = time::precise_time_ns();
                    self.time_bsy += end - beg;
                },
                Ok( PipelineInfo::Begin ) => {
                    self.infos  = Vec::new();
                    self.errors = Vec::new();

                    self.time_beg = time::precise_time_ns();
                    let _ = tx.send( PipelineInfo::Begin );
                },
                Ok( PipelineInfo::End ) => {
                    for i in &self.infos  { let _ = tx.send( PipelineInfo::Info( i.clone() ) ); }
                    for e in &self.errors { let _ = tx.send( PipelineInfo::Err ( e.clone() ) ); }

                    self.time_end = time::precise_time_ns();
                    let _ = tx.send( PipelineInfo::Time( self.time_bsy, self.time_end - self.time_beg ) );
                    let _ = tx.send( PipelineInfo::End );
                    break;
                },
                Ok ( PipelineInfo::Info( e      ) ) => { let _ = tx.send( PipelineInfo::Info( e      ) ); },
                Ok ( PipelineInfo::Err ( e      ) ) => { let _ = tx.send( PipelineInfo::Err ( e      ) ); },
                Ok ( PipelineInfo::Time( t0, t1 ) ) => { let _ = tx.send( PipelineInfo::Time( t0, t1 ) ); },
                Err( _                            ) => break,
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------------------------------------------------

//#[cfg(test)]
//mod tests {
//    use super::*;
//    use matcher::QuickSearchMatcher;
//    use std::path::PathBuf;
//    use std::thread;
//    use std::sync::mpsc;
//    use util::PipelineInfo;
//
//    #[test]
//    fn test_simple_pipeline_printer() {
//    }
//}

