use pipeline::{PipelineInfo, PipelineJoin};
use pipeline_matcher::PathMatch;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use time;

// ---------------------------------------------------------------------------------------------------------------------
// PipelineSorter
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelineSorter {
    pub infos : Vec<String>,
    pub errors: Vec<String>,
    map       : HashMap<usize, PathMatch>,
    seq_no    : usize,
    join_num  : usize,
    time_beg  : u64,
    time_end  : u64,
    time_bsy  : u64,
}

impl PipelineSorter {
    pub fn new( num: usize ) -> Self {
        PipelineSorter {
            infos   : Vec::new(),
            errors  : Vec::new(),
            map     : HashMap::new(),
            seq_no  : 0,
            join_num: num,
            time_beg: 0,
            time_end: 0,
            time_bsy: 0,
        }
    }
}

impl PipelineJoin<PathMatch, PathMatch> for PipelineSorter {
    fn setup( &mut self, id: usize, rx: Vec<Receiver<PipelineInfo<PathMatch>>>, tx: Sender<PipelineInfo<PathMatch>> ) {
        self.infos  = Vec::new();
        self.errors = Vec::new();
        let mut seq_beg_arrived = false;
        let mut end_num = 0;

        loop {
            for rx in &rx {
                match rx.recv() {
                    Ok( PipelineInfo::SeqDat( x, p ) ) => {
                        let beg = time::precise_time_ns();

                        self.map.insert( x, p );
                        loop {
                            if !self.map.contains_key( &self.seq_no ) {
                                break;
                            }
                            {
                                let ret = self.map.get( &self.seq_no ).unwrap();
                                let _ = tx.send( PipelineInfo::SeqDat( self.seq_no, ret.clone() ) );
                            }
                            let _ = self.map.remove( &self.seq_no );
                            self.seq_no += 1;
                        }

                        let end = time::precise_time_ns();
                        self.time_bsy += end - beg;
                    },

                    Ok( PipelineInfo::SeqBeg( x ) ) => {
                        if !seq_beg_arrived {
                            self.seq_no = x;
                            self.time_beg = time::precise_time_ns();
                            let _ = tx.send( PipelineInfo::SeqBeg( x ) );
                            seq_beg_arrived = true;
                        }
                    },

                    Ok( PipelineInfo::SeqEnd( x ) ) => {
                        end_num += 1;
                        if end_num != self.join_num { continue; }
                        //if x != self.seq_no { continue; }

                        for i in &self.infos  { let _ = tx.send( PipelineInfo::MsgInfo( id, i.clone() ) ); }
                        for e in &self.errors { let _ = tx.send( PipelineInfo::MsgErr ( id, e.clone() ) ); }

                        self.time_end = time::precise_time_ns();
                        let _ = tx.send( PipelineInfo::MsgTime( id, self.time_bsy, self.time_end - self.time_beg ) );
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
    fn pipeline_sorter() {
        let mut sorter = PipelineSorter::new( 1 );

        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        thread::spawn( move || {
            sorter.setup( 0, vec![in_rx], out_tx );
        } );

        let _ = in_tx.send( PipelineInfo::SeqBeg( 0                                                                ) );
        let _ = in_tx.send( PipelineInfo::SeqDat( 2, PathMatch{ path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::SeqDat( 1, PathMatch{ path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::SeqDat( 0, PathMatch{ path: PathBuf::from( "./" ), matches: Vec::new() } ) );
        let _ = in_tx.send( PipelineInfo::SeqEnd( 3                                                                ) );

        let mut ret = Vec::new();
        let mut time_bsy = 0;
        let mut time_all = 0;
        loop {
            match out_rx.recv().unwrap() {
                PipelineInfo::SeqDat ( x, _      ) => ret.push( x ),
                PipelineInfo::SeqEnd ( _         ) => break,
                PipelineInfo::MsgTime( _, t0, t1 ) => { time_bsy = t0; time_all = t1; },
                _                                  => (),
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

