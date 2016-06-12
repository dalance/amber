use pipeline::{PipelineInfo, PipelineJoin};
use pipeline_matcher::PathMatch;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------------------------------
// PipelineSorter
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelineSorter {
    pub infos  : Vec<String>,
    pub errors : Vec<String>,
    pub through: bool,
    map        : HashMap<usize, PathMatch>,
    seq_no     : usize,
    join_num   : usize,
    time_beg   : Instant,
    time_bsy   : Duration,
}

impl PipelineSorter {
    pub fn new( num: usize ) -> Self {
        PipelineSorter {
            infos   : Vec::new(),
            errors  : Vec::new(),
            through : false,
            map     : HashMap::new(),
            seq_no  : 0,
            join_num: num,
            time_beg: Instant::now(),
            time_bsy: Duration::new(0, 0),
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
                        watch_time!( self.time_bsy, {
                            if self.through {
                                let _ = tx.send( PipelineInfo::SeqDat( x, p ) );
                            } else {
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
                            }
                        } );
                    },

                    Ok( PipelineInfo::SeqBeg( x ) ) => {
                        if !seq_beg_arrived {
                            self.seq_no = x;
                            self.time_beg = Instant::now();
                            let _ = tx.send( PipelineInfo::SeqBeg( x ) );
                            seq_beg_arrived = true;
                        }
                    },

                    Ok( PipelineInfo::SeqEnd( x ) ) => {
                        end_num += 1;
                        if end_num != self.join_num { continue; }

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
        loop {
            match out_rx.recv().unwrap() {
                PipelineInfo::SeqDat ( x, _ ) => ret.push( x ),
                PipelineInfo::SeqEnd ( _    ) => break,
                _                             => (),
            }
        }

        assert_eq!( ret[0], 0 );
        assert_eq!( ret[1], 1 );
        assert_eq!( ret[2], 2 );
    }
}

