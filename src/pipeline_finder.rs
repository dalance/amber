use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use time;
use util::PipelineInfo;

// ---------------------------------------------------------------------------------------------------------------------
// PipelineFinder
// ---------------------------------------------------------------------------------------------------------------------

pub trait PipelineFinder {
    fn find( &mut self, rx: Receiver<PipelineInfo<PathBuf>>, tx: Sender<PipelineInfo<PathBuf>> );
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePipelineFinder
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePipelineFinder {
    pub is_recursive  : bool,
    pub follow_symlink: bool,
    pub infos         : Vec<String>,
    pub errors        : Vec<String>,
    time_beg          : u64,
    time_end          : u64,
    time_bsy          : u64,
}

impl SimplePipelineFinder {
    pub fn new() -> Self {
        SimplePipelineFinder {
            is_recursive  : true,
            follow_symlink: true,
            infos         : Vec::new(),
            errors        : Vec::new(),
            time_beg      : 0,
            time_end      : 0,
            time_bsy      : 0,
        }
    }

    fn find_path( &mut self, base: PathBuf ) -> Vec<PathBuf> {
        let mut ret: Vec<PathBuf> = Vec::new();

        let attr = match fs::metadata( &base ) {
            Ok ( x ) => x,
            Err( e ) => { self.errors.push( format!( "Error: {} @ {}", e, base.to_str().unwrap() ) ); return Vec::new() },
        };

        if attr.is_file() {
            if attr.len() != 0 {
                ret.push( base );
            }
        } else {
            let reader = match fs::read_dir( &base ) {
                Ok ( x ) => x,
                Err( e ) => { self.errors.push( format!( "Error: {} @ {}", e, base.to_str().unwrap() ) ); return Vec::new() },
            };

            for i in reader {
                match i {
                    Ok( entry ) => {
                        let file_type = match entry.file_type() {
                            Ok ( x ) => x,
                            Err( e ) => { self.errors.push( format!( "Error: {}", e ) ); continue },
                        };
                        if file_type.is_file() {
                            let metadata = match entry.metadata() {
                                Ok ( x ) => x,
                                Err( e ) => { self.errors.push( format!( "Error: {}", e ) ); continue },
                            };
                            if metadata.len() != 0 {
                                ret.push( entry.path() );
                            }
                        } else {
                            let find_dir     = file_type.is_dir()     & self.is_recursive;
                            let find_symlink = file_type.is_symlink() & self.is_recursive & self.follow_symlink;
                            if find_dir | find_symlink {
                                let sub_ret = self.find_path( entry.path() );
                                for r in sub_ret {
                                    ret.push( r );
                                }
                            }
                        }
                    },
                    Err( e ) => self.errors.push( format!( "Error: {}", e ) ),
                };
            }
        }

        ret
    }
}

impl PipelineFinder for SimplePipelineFinder {
    fn find( &mut self, rx: Receiver<PipelineInfo<PathBuf>>, tx: Sender<PipelineInfo<PathBuf>> ) {
        loop {
            match rx.recv() {
                Ok( PipelineInfo::Ok( p ) ) => {
                    let beg = time::precise_time_ns();

                    let path = self.find_path( p );
                    for p in path { let _ = tx.send( PipelineInfo::Ok( p ) ); }

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
                Ok ( x ) => { let _ = tx.send( x ); },
                Err( _ ) => break,
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
    use std::path::PathBuf;
    use std::thread;
    use std::sync::mpsc;
    use util::PipelineInfo;

    #[test]
    fn test_simple_pipeline_finder() {
        let mut finder = SimplePipelineFinder::new();

        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        thread::spawn( move || {
            finder.find( in_rx, out_tx );
        } );
        let _ = in_tx.send( PipelineInfo::Begin );
        let _ = in_tx.send( PipelineInfo::Ok( PathBuf::from( "./" ) ) );
        let _ = in_tx.send( PipelineInfo::End );

        let mut ret = Vec::new();
        let mut time_bsy = 0;
        let mut time_all = 0;
        loop {
            match out_rx.recv().unwrap() {
                PipelineInfo::Ok  ( x      ) => ret.push( x ),
                PipelineInfo::Time( t0, t1 ) => { time_bsy = t0; time_all = t1; },
                PipelineInfo::End            => break,
                _                            => (),
            }
        }

        assert!( ret.contains( &PathBuf::from( "./Cargo.toml"         ) ) );
        assert!( ret.contains( &PathBuf::from( "./src/ambr.rs"        ) ) );
        assert!( ret.contains( &PathBuf::from( "./src/ambs.rs"        ) ) );
        assert!( ret.contains( &PathBuf::from( "./src/console.rs"     ) ) );
        assert!( ret.contains( &PathBuf::from( "./src/lib.rs"         ) ) );
        assert!( ret.contains( &PathBuf::from( "./src/matcher.rs"     ) ) );
        assert!( ret.contains( &PathBuf::from( "./src/util.rs"        ) ) );

        assert!( time_bsy != 0 );
        assert!( time_all != 0 );
        assert!( time_bsy < time_all );
    }
}

