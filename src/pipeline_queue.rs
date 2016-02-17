use pipeline::{PipelineInfo, PipelineJoin};
use pipeline_matcher::PathMatch;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use time;

// ---------------------------------------------------------------------------------------------------------------------
// PipelineQueue
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelineQueue {
    pub infos : Vec<String>,
    pub errors: Vec<String>,
    map       : HashMap<usize, PathMatch>,
    msg_id    : usize,
    time_beg  : u64,
    time_end  : u64,
    time_bsy  : u64,
}

impl PipelineQueue {
    pub fn new() -> Self {
        PipelineQueue {
            infos   : Vec::new(),
            errors  : Vec::new(),
            map     : HashMap::new(),
            msg_id  : 0,
            time_beg: 0,
            time_end: 0,
            time_bsy: 0,
        }
    }
}

impl PipelineJoin<PathMatch, PathMatch> for PipelineQueue {
    fn setup( &mut self, id: usize, rx: Vec<Receiver<PipelineInfo<PathMatch>>>, tx: Sender<PipelineInfo<PathMatch>> ) {
        loop {
            for rx in &rx {
                match rx.recv() {
                    Ok( PipelineInfo::Ok( x, p ) ) => {
                        let beg = time::precise_time_ns();

                        self.map.insert( x, p );
                        loop {
                            if !self.map.contains_key( &self.msg_id ) {
                                break;
                            }
                            {
                                let ret = self.map.get( &self.msg_id ).unwrap();
                                let _ = tx.send( PipelineInfo::Ok( self.msg_id, ret.clone() ) );
                            }
                            let _ = self.map.remove( &self.msg_id );
                            self.msg_id += 1;
                        }

                        let end = time::precise_time_ns();
                        self.time_bsy += end - beg;
                    },

                    Ok( PipelineInfo::Beg( x ) ) => {
                        self.msg_id = x;
                        self.infos  = Vec::new();
                        self.errors = Vec::new();

                        self.time_beg = time::precise_time_ns();
                        let _ = tx.send( PipelineInfo::Beg( x ) );
                    },

                    Ok( PipelineInfo::End( x ) ) => {
                        if x != self.msg_id { continue; }

                        for i in &self.infos  { let _ = tx.send( PipelineInfo::Info( id, i.clone() ) ); }
                        for e in &self.errors { let _ = tx.send( PipelineInfo::Err ( id, e.clone() ) ); }

                        self.time_end = time::precise_time_ns();
                        let _ = tx.send( PipelineInfo::Time( id, self.time_bsy, self.time_end - self.time_beg ) );
                        let _ = tx.send( PipelineInfo::End( x ) );
                        break;
                    },

                    Ok ( PipelineInfo::Info( i, e      ) ) => { let _ = tx.send( PipelineInfo::Info( i, e      ) ); },
                    Ok ( PipelineInfo::Err ( i, e      ) ) => { let _ = tx.send( PipelineInfo::Err ( i, e      ) ); },
                    Ok ( PipelineInfo::Time( i, t0, t1 ) ) => { let _ = tx.send( PipelineInfo::Time( i, t0, t1 ) ); },
                    Err( _                               ) => break,
                }
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
    use pipeline::{Pipeline, PipelineInfo, PipelineJoin};
    use pipeline_matcher::PathMatch;
    use std::path::PathBuf;
    use std::thread;
    use std::sync::mpsc;

    #[test]
    fn pipeline_queue() {
        let mut queue = PipelineQueue::new();

        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        thread::spawn( move || {
            queue.setup( 0, vec![in_rx], out_tx );
        } );

        let _ = in_tx.send( PipelineInfo::Beg( 0                                                                ) );
        let _ = in_tx.send( PipelineInfo::Ok ( 2, PathMatch{ path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::Ok ( 1, PathMatch{ path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::Ok ( 0, PathMatch{ path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::End( 3                                                                ) );

        let mut ret = Vec::new();
        let mut time_bsy = 0;
        let mut time_all = 0;
        loop {
            match out_rx.recv().unwrap() {
                PipelineInfo::Ok  ( x, _      ) => ret.push( x ),
                PipelineInfo::Time( _, t0, t1 ) => { time_bsy = t0; time_all = t1; },
                PipelineInfo::End ( _         ) => break,
                _                               => (),
            }
        }

        assert_eq!( ret[0], 0 );
        assert_eq!( ret[1], 1 );
        assert_eq!( ret[2], 2 );

        assert!( time_bsy != 0 );
        assert!( time_all != 0 );
        assert!( time_bsy < time_all );
    }
}

