use std::fs;
use std::path::{Component, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use time;
use util::PipelineInfo;

// ---------------------------------------------------------------------------------------------------------------------
// PipelineFinder
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug,Clone)]
pub struct PathInfo {
    pub id  : usize  ,
    pub path: PathBuf,
    pub len : u64    ,
}

pub trait PipelineFinder {
    fn find( &mut self, rx: Receiver<PipelineInfo<PathBuf>>, tx: Sender<PipelineInfo<PathInfo>> );
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePipelineFinder
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePipelineFinder {
    pub is_recursive  : bool,
    pub follow_symlink: bool,
    pub skip_vcs      : bool,
    pub print_skipped : bool,
    pub infos         : Vec<String>,
    pub errors        : Vec<String>,
    time_beg          : u64,
    time_end          : u64,
    time_bsy          : u64,
    count             : usize,
}

impl SimplePipelineFinder {
    pub fn new() -> Self {
        SimplePipelineFinder {
            is_recursive  : true,
            follow_symlink: true,
            skip_vcs      : true,
            print_skipped : false,
            infos         : Vec::new(),
            errors        : Vec::new(),
            time_beg      : 0,
            time_end      : 0,
            time_bsy      : 0,
            count         : 0,
        }
    }

    fn find_path( &mut self, base: PathBuf, tx: &Sender<PipelineInfo<PathInfo>> ) {

        let attr = match fs::metadata( &base ) {
            Ok ( x ) => x,
            Err( e ) => { self.errors.push( format!( "Error: {} @ {}", e, base.to_str().unwrap() ) ); return; },
        };

        if attr.is_file() {
            if attr.len() != 0 {
                self.send_path( base, attr.len(), &tx );
            }
        } else {
            let reader = match fs::read_dir( &base ) {
                Ok ( x ) => x,
                Err( e ) => { self.errors.push( format!( "Error: {} @ {}", e, base.to_str().unwrap() ) ); return; /*Vec::new()*/ },
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
                                self.send_path( entry.path(), metadata.len(), &tx );
                            }
                        } else {
                            let find_dir     = file_type.is_dir()     & self.is_recursive;
                            let find_symlink = file_type.is_symlink() & self.is_recursive & self.follow_symlink;
                            if find_dir | find_symlink {
                                self.find_path( entry.path(), &tx );
                            }
                        }
                    },
                    Err( e ) => self.errors.push( format!( "Error: {}", e ) ),
                };
            }
        }
    }

    fn send_path( &mut self, path: PathBuf, len: u64, tx: &Sender<PipelineInfo<PathInfo>> ) {
        if self.filter_path( &path ) {
            let _ = tx.send( PipelineInfo::Ok( PathInfo{ id: self.count, path: path, len: len } ) );
        }
        self.count += 1;
    }

    fn filter_path( &mut self, path: &PathBuf ) -> bool {
        if self.skip_vcs {
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
            if is_vcs_dir {
                if self.print_skipped {
                    self.infos.push( format!( "Skipped: {:?} ( vcs file )\n", path ) );
                }
                return false
            }
        }
        true
    }
}

impl PipelineFinder for SimplePipelineFinder {
    fn find( &mut self, rx: Receiver<PipelineInfo<PathBuf>>, tx: Sender<PipelineInfo<PathInfo>> ) {
        loop {
            match rx.recv() {
                Ok( PipelineInfo::Ok( p ) ) => {
                    let beg = time::precise_time_ns();

                    self.find_path( p, &tx );

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
                Ok( PipelineInfo::Info( e      ) ) => { let _ = tx.send( PipelineInfo::Info( e      ) ); },
                Ok( PipelineInfo::Err ( e      ) ) => { let _ = tx.send( PipelineInfo::Err ( e      ) ); },
                Ok( PipelineInfo::Time( t0, t1 ) ) => { let _ = tx.send( PipelineInfo::Time( t0, t1 ) ); },
                Err( _ )                           => break,
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

