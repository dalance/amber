use pipeline_matcher::PathMatch;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use time;
use util::PipelineInfo;

// ---------------------------------------------------------------------------------------------------------------------
// PipelineQueue
// ---------------------------------------------------------------------------------------------------------------------

pub trait PipelineQueue {
    fn exec( &mut self, rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<PathMatch>> );
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePipelineQueue
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePipelineQueue {
    pub infos : Vec<String>,
    pub errors: Vec<String>,
    map       : HashMap<usize, PathMatch>,
    current_id: usize,
    time_beg  : u64,
    time_end  : u64,
    time_bsy  : u64,
}

impl SimplePipelineQueue {
    pub fn new() -> Self {
        SimplePipelineQueue {
            infos     : Vec::new(),
            errors    : Vec::new(),
            map       : HashMap::new(),
            current_id: 0,
            time_beg  : 0,
            time_end  : 0,
            time_bsy  : 0,
        }
    }
}

impl PipelineQueue for SimplePipelineQueue {
    fn exec( &mut self, rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<PathMatch>> ) {
        loop {
            match rx.recv() {
                Ok( PipelineInfo::Ok( p ) ) => {
                    let beg = time::precise_time_ns();

                    self.map.insert( p.id, p );
                    loop {
                        if !self.map.contains_key( &self.current_id ) {
                            break;
                        }
                        {
                            let ret = self.map.get( &self.current_id ).unwrap();
                            let _ = tx.send( PipelineInfo::Ok( ret.clone() ) );
                        }
                        let _ = self.map.remove( &self.current_id );
                        self.current_id += 1;
                    }

                    let end = time::precise_time_ns();
                    self.time_bsy += end - beg;
                },

                Ok( PipelineInfo::Beg( x ) ) => {
                    self.current_id = x;
                    self.infos  = Vec::new();
                    self.errors = Vec::new();

                    self.time_beg = time::precise_time_ns();
                    let _ = tx.send( PipelineInfo::Beg( x ) );
                },

                Ok( PipelineInfo::End( x ) ) => {
                    if x != self.current_id { continue; }

                    for i in &self.infos  { let _ = tx.send( PipelineInfo::Info( i.clone() ) ); }
                    for e in &self.errors { let _ = tx.send( PipelineInfo::Err ( e.clone() ) ); }

                    self.time_end = time::precise_time_ns();
                    let _ = tx.send( PipelineInfo::Time( self.time_bsy, self.time_end - self.time_beg ) );
                    let _ = tx.send( PipelineInfo::End( x ) );
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

#[cfg(test)]
mod tests {
    use super::*;
    use pipeline_matcher::PathMatch;
    use std::path::PathBuf;
    use std::thread;
    use std::sync::mpsc;
    use util::PipelineInfo;

    #[test]
    fn test_simple_pipeline_queue() {
        let mut queue = SimplePipelineQueue::new();

        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        thread::spawn( move || {
            queue.exec( in_rx, out_tx );
        } );

        let _ = in_tx.send( PipelineInfo::Beg( 0                                                                 ) );
        let _ = in_tx.send( PipelineInfo::Ok ( PathMatch{ id: 2, path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::Ok ( PathMatch{ id: 1, path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::Ok ( PathMatch{ id: 0, path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::End( 3                                                                 ) );

        let mut ret = Vec::new();
        let mut time_bsy = 0;
        let mut time_all = 0;
        loop {
            match out_rx.recv().unwrap() {
                PipelineInfo::Ok  ( x      ) => ret.push( x ),
                PipelineInfo::Time( t0, t1 ) => { time_bsy = t0; time_all = t1; },
                PipelineInfo::End ( _      ) => break,
                _                            => (),
            }
        }

        assert_eq!( ret[0].id, 0 );
        assert_eq!( ret[1].id, 1 );
        assert_eq!( ret[2].id, 2 );

        assert!( time_bsy != 0 );
        assert!( time_all != 0 );
        assert!( time_bsy < time_all );
    }
}

