use ignore::{Ignore, IgnoreGit, IgnoreVcs};
use pipeline::{PipelineFork, PipelineInfo};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use time;

// ---------------------------------------------------------------------------------------------------------------------
// PathInfo
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug,Clone)]
pub struct PathInfo {
    pub path: PathBuf,
    pub len : u64    ,
}

// ---------------------------------------------------------------------------------------------------------------------
// PipelineFinder
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelineFinder {
    pub is_recursive      : bool,
    pub follow_symlink    : bool,
    pub skip_vcs          : bool,
    pub skip_gitignore    : bool,
    pub skip_hgignore     : bool,
    pub skip_ambignore    : bool,
    pub print_skipped     : bool,
    pub find_parent_ignore: bool,
    pub infos             : Vec<String>,
    pub errors            : Vec<String>,
    time_beg              : u64,
    time_end              : u64,
    time_bsy              : u64,
    seq_no                : usize,
    current_tx            : usize,
    ignore_vcs            : IgnoreVcs,
    ignore_git            : Vec<IgnoreGit>,
}

impl PipelineFinder {
    pub fn new() -> Self {
        PipelineFinder {
            is_recursive      : true,
            follow_symlink    : true,
            skip_vcs          : true,
            skip_gitignore    : true,
            skip_hgignore     : true,
            skip_ambignore    : true,
            print_skipped     : false,
            find_parent_ignore: true,
            infos             : Vec::new(),
            errors            : Vec::new(),
            time_beg          : 0,
            time_end          : 0,
            time_bsy          : 0,
            seq_no            : 0,
            current_tx        : 0,
            ignore_vcs        : IgnoreVcs::new(),
            ignore_git        : Vec::new(),
        }
    }

    fn find_path( &mut self, base: PathBuf, tx: &Vec<Sender<PipelineInfo<PathInfo>>> ) {

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
                Err( e ) => { self.errors.push( format!( "Error: {} @ {}", e, base.to_str().unwrap() ) ); return; },
            };

            let gitignore_exist = self.push_gitignore( &base );

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
                            if ( find_dir | find_symlink ) & self.check_path( &entry.path(), true ) {
                                self.find_path( entry.path(), &tx );
                            }
                        }
                    },
                    Err( e ) => self.errors.push( format!( "Error: {}", e ) ),
                };
            }

            self.pop_gitignore( gitignore_exist )
        }
    }

    fn send_path( &mut self, path: PathBuf, len: u64, tx: &Vec<Sender<PipelineInfo<PathInfo>>> ) {
        if self.check_path( &path, false ) {
            let _ = tx[self.current_tx].send( PipelineInfo::SeqDat( self.seq_no, PathInfo{ path: path, len: len } ) );
            self.seq_no += 1;
            self.current_tx = if self.current_tx == tx.len() - 1 { 0 } else { self.current_tx + 1 };
        }
    }

    fn push_gitignore( &mut self, path: &PathBuf ) -> bool {
        if !self.skip_gitignore { return false }

        let reader = fs::read_dir( &path ).unwrap();
        for i in reader {
            match i {
                Ok( entry ) => {
                    if entry.path().ends_with( ".gitignore" ) {
                        self.ignore_git.push( IgnoreGit::new( &entry.path() ) );
                        return true;
                    }
                },
                Err( e ) => self.errors.push( format!( "Error: {}", e ) ),
            }
        }
        false
    }

    fn pop_gitignore( &mut self, exist: bool ) {
        if exist {
            let _ = self.ignore_git.pop();
        }
    }

    fn check_path( &mut self, path: &PathBuf, is_dir: bool ) -> bool {
        let ok_vcs = if self.skip_vcs {
            !self.ignore_vcs.is_ignore( &path, is_dir )
        } else {
            true
        };

        let ok_git = if self.skip_gitignore && !self.ignore_git.is_empty() {
            !self.ignore_git.last().unwrap().is_ignore( &path, is_dir )
        } else {
            true
        };

        if !ok_vcs & self.print_skipped {
            self.infos.push( format!( "Skipped: {:?} ( vcs file )\n", path ) );
        }

        if !ok_git & self.print_skipped {
            self.infos.push( format!( "Skipped: {:?} ( .gitignore )\n", path ) );
        }

        ok_vcs && ok_git
    }

    fn set_default_gitignore( &mut self, base: &PathBuf ) -> PathBuf {
        if !self.skip_gitignore     { return base.clone() }
        if !self.find_parent_ignore { return base.clone() }

        let base_abs = match base.canonicalize() {
            Ok ( x ) => x,
            Err( e ) => { self.errors.push( format!( "Error: {} @ {}", e, base.to_str().unwrap() ) ); return base.clone(); },
        };

        let mut parent = base_abs.parent();
        while parent.is_some() {
            if self.push_gitignore( &PathBuf::from( parent.unwrap() ) ) {
                self.infos.push( format!( "Found .gitignore at the parent directory: {:?}\n", parent.unwrap() ) );
                return base_abs.clone();
            }
            parent = parent.unwrap().parent();
        }

        return base.clone();
    }
}

impl PipelineFork<PathBuf, PathInfo> for PipelineFinder {
    fn setup( &mut self, id: usize, rx: Receiver<PipelineInfo<PathBuf>>, tx: Vec<Sender<PipelineInfo<PathInfo>>> ) {
        self.infos  = Vec::new();
        self.errors = Vec::new();
        let mut seq_beg_arrived = false;

        loop {
            match rx.recv() {
                Ok( PipelineInfo::SeqDat( _, p ) ) => {
                    let beg = time::precise_time_ns();

                    let p = self.set_default_gitignore( &p );
                    self.find_path( p, &tx );

                    let end = time::precise_time_ns();
                    self.time_bsy += end - beg;
                },

                Ok( PipelineInfo::SeqBeg( x ) ) => {
                    if !seq_beg_arrived {
                        self.seq_no = x;
                        self.time_beg = time::precise_time_ns();

                        for tx in &tx {
                            let _ = tx.send( PipelineInfo::SeqBeg( x ) );
                        }

                        seq_beg_arrived = true;
                    }
                },

                Ok( PipelineInfo::SeqEnd( _ ) ) => {
                    for i in &self.infos  { let _ = tx[0].send( PipelineInfo::MsgInfo( id, i.clone() ) ); }
                    for e in &self.errors { let _ = tx[0].send( PipelineInfo::MsgErr ( id, e.clone() ) ); }

                    self.time_end = time::precise_time_ns();
                    let _ = tx[0].send( PipelineInfo::MsgTime( id, self.time_bsy, self.time_end - self.time_beg ) );

                    for tx in &tx {
                        let _ = tx.send( PipelineInfo::SeqEnd( self.seq_no ) );
                    }

                    break;
                },

                Ok ( PipelineInfo::MsgInfo( i, e      ) ) => { let _ = tx[0].send( PipelineInfo::MsgInfo( i, e      ) ); },
                Ok ( PipelineInfo::MsgErr ( i, e      ) ) => { let _ = tx[0].send( PipelineInfo::MsgErr ( i, e      ) ); },
                Ok ( PipelineInfo::MsgTime( i, t0, t1 ) ) => { let _ = tx[0].send( PipelineInfo::MsgTime( i, t0, t1 ) ); },
                Err( _                                  ) => break,
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
    use pipeline::{PipelineFork, PipelineInfo};

    fn test<T: 'static+PipelineFork<PathBuf, PathInfo>+Send>( mut finder: T, path: String ) -> Vec<PathInfo> {
        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        thread::spawn( move || {
            finder.setup( 0, in_rx, vec![out_tx] );
        } );
        let _ = in_tx.send( PipelineInfo::SeqBeg( 0                        ) );
        let _ = in_tx.send( PipelineInfo::SeqDat( 0, PathBuf::from( path ) ) );
        let _ = in_tx.send( PipelineInfo::SeqEnd( 1                        ) );

        let mut ret = Vec::new();
        loop {
            match out_rx.recv().unwrap() {
                PipelineInfo::SeqDat( _, x ) => ret.push( x ),
                PipelineInfo::SeqEnd( _    ) => break,
                _                            => (),
            }
        }

        ret
    }

    #[test]
    fn pipeline_finder_default() {
        let finder = PipelineFinder::new();
        let ret = test( finder, "./".to_string() );

        assert!(  ret.iter().any( |x| x.path == PathBuf::from( "./Cargo.toml"     ) ) );
        assert!(  ret.iter().any( |x| x.path == PathBuf::from( "./src/ambr.rs"    ) ) );
        assert!(  ret.iter().any( |x| x.path == PathBuf::from( "./src/ambs.rs"    ) ) );
        assert!(  ret.iter().any( |x| x.path == PathBuf::from( "./src/console.rs" ) ) );
        assert!(  ret.iter().any( |x| x.path == PathBuf::from( "./src/lib.rs"     ) ) );
        assert!(  ret.iter().any( |x| x.path == PathBuf::from( "./src/matcher.rs" ) ) );
        assert!(  ret.iter().any( |x| x.path == PathBuf::from( "./src/util.rs"    ) ) );
        assert!( !ret.iter().any( |x| x.path == PathBuf::from( "./.git/config"    ) ) );
    }

    #[test]
    fn pipeline_finder_not_skip_vcs() {
        let mut finder = PipelineFinder::new();
        finder.skip_vcs = false;
        let ret = test( finder, "./".to_string() );

        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./Cargo.toml"     ) ) );
        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./src/ambr.rs"    ) ) );
        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./src/ambs.rs"    ) ) );
        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./src/console.rs" ) ) );
        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./src/lib.rs"     ) ) );
        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./src/matcher.rs" ) ) );
        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./src/util.rs"    ) ) );
        assert!( ret.iter().any( |x| x.path == PathBuf::from( "./.git/config"    ) ) );
    }
}

