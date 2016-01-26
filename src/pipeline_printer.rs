use console::{Console, ConsoleTextKind};
use memmap::{Mmap, Protection};
use pipeline_matcher::PathMatch;
use std::io::Error;
use std::sync::mpsc::{Receiver, Sender};
use time;
use util::{catch, decode_error, PipelineInfo};

// ---------------------------------------------------------------------------------------------------------------------
// PipelinePrinter
// ---------------------------------------------------------------------------------------------------------------------

pub trait PipelinePrinter {
    fn print( &mut self, rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<()>> );
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePipelinePrinter
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePipelinePrinter {
    pub is_color    : bool,
    pub print_file  : bool,
    pub print_column: bool,
    pub infos       : Vec<String>,
    pub errors      : Vec<String>,
    console         : Console,
    time_beg        : u64,
    time_end        : u64,
    time_bsy        : u64,
}

impl SimplePipelinePrinter {
    pub fn new() -> Self {
        SimplePipelinePrinter {
            is_color    : true,
            print_file  : true,
            print_column: false,
            infos       : Vec::new(),
            errors      : Vec::new(),
            console     : Console::new(),
            time_beg    : 0,
            time_end    : 0,
            time_bsy    : 0,
        }
    }

    fn print_match( &mut self, pm: PathMatch ) {
        self.console.is_color = self.is_color;

        let result = catch::<_, (), Error> ( || {
            let mmap = try!( Mmap::open_path( &pm.path, Protection::Read ) );
            let src  = unsafe { mmap.as_slice() };

            let mut pos     = 0;
            let mut column  = 0;
            let mut last_lf = 0;
            for m in &pm.matches {
                if self.print_file {
                    self.console.write( ConsoleTextKind::Filename, pm.path.to_str().unwrap() );
                    self.console.write( ConsoleTextKind::Other, ":" );
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
            }

            Ok( () )
        } );
        match result {
            Ok ( _ ) => (),
            Err( e ) => self.console.write( ConsoleTextKind::Error, &format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), pm.path ) ),
        }
    }
}

impl PipelinePrinter for SimplePipelinePrinter {
    fn print( &mut self, rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<()>> ) {
        loop {
            match rx.recv() {
                Ok( PipelineInfo::Ok( pm ) ) => {
                    let beg = time::precise_time_ns();

                    self.print_match( pm );
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

