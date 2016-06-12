use console::{Console, ConsoleTextKind};
use memmap::{Mmap, Protection};
use pipeline::{Pipeline, PipelineInfo};
use pipeline_matcher::PathMatch;
use std::io::Error;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};
use util::{catch, decode_error};

// ---------------------------------------------------------------------------------------------------------------------
// PipelinePrinter
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelinePrinter {
    pub is_color    : bool,
    pub print_file  : bool,
    pub print_column: bool,
    pub print_row   : bool,
    pub infos       : Vec<String>,
    pub errors      : Vec<String>,
    console         : Console,
    time_beg        : Instant,
    time_bsy        : Duration,
}

impl PipelinePrinter {
    pub fn new() -> Self {
        PipelinePrinter {
            is_color    : true,
            print_file  : true,
            print_column: false,
            print_row   : false,
            infos       : Vec::new(),
            errors      : Vec::new(),
            console     : Console::new(),
            time_beg    : Instant::now(),
            time_bsy    : Duration::new(0, 0),
        }
    }

    fn print_match( &mut self, pm: PathMatch ) {
        if pm.matches.is_empty() { return; }
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
                    self.console.write( ConsoleTextKind::Filename, ":" );
                }
                if self.print_column | self.print_row {
                    while pos < m.beg {
                        if src[pos] == 0x0a {
                            column += 1;
                            last_lf = pos;
                        }
                        pos += 1;
                    }
                    if self.print_column {
                        self.console.write( ConsoleTextKind::Other, &format!( "{}:", column + 1 ) );
                    }
                    if self.print_row {
                        self.console.write( ConsoleTextKind::Other, &format!( "{}:", m.beg - last_lf ) );
                    }
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

impl Pipeline<PathMatch, ()> for PipelinePrinter {
    fn setup( &mut self, id: usize, rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<()>> ) {
        self.infos  = Vec::new();
        self.errors = Vec::new();
        let mut seq_beg_arrived = false;

        loop {
            match rx.recv() {
                Ok( PipelineInfo::SeqDat( x, pm ) ) => {
                    watch_time!( self.time_bsy, {
                        self.print_match( pm );
                        let _ = tx.send( PipelineInfo::SeqDat( x, () ) );
                    } );
                },

                Ok( PipelineInfo::SeqBeg( x ) ) => {
                    if !seq_beg_arrived {
                        self.time_beg = Instant::now();
                        let _ = tx.send( PipelineInfo::SeqBeg( x ) );
                        seq_beg_arrived = true;
                    }
                },

                Ok( PipelineInfo::SeqEnd( x ) ) => {
                    for i in &self.infos  { let _ = tx.send( PipelineInfo::MsgInfo( id, i.clone() ) ); }
                    for e in &self.errors { let _ = tx.send( PipelineInfo::MsgErr ( id, e.clone() ) ); }

                    let _ = tx.send( PipelineInfo::MsgTime( id, self.time_bsy, self.time_beg.elapsed() ) );
                    let _ = tx.send( PipelineInfo::SeqEnd( x ) );
                    break;
                },

                Ok ( PipelineInfo::MsgInfo( i, e      ) ) => { let _ = tx.send( PipelineInfo::MsgInfo( i, e      ) ); },
                Ok ( PipelineInfo::MsgErr ( i, e      ) ) => { let _ = tx.send( PipelineInfo::MsgErr ( i, e      ) ); },
                Ok ( PipelineInfo::MsgTime( i, t0, t1 ) ) => { let _ = tx.send( PipelineInfo::MsgTime( i, t0, t1 ) ); },
                Err( _                                  ) => break,
            }
        }
    }
}
