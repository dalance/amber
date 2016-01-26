use std::io::Error;
use std::path::{Component, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use time;
use util::{catch, decode_error, PipelineInfo};

// ---------------------------------------------------------------------------------------------------------------------
// PipelineFilter
// ---------------------------------------------------------------------------------------------------------------------

pub trait PipelineFilter {
    fn filter( &mut self, rx: Receiver<PipelineInfo<PathBuf>>, tx: Sender<PipelineInfo<PathBuf>> );
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePipelineFilter
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePipelineFilter {
    pub skip_vcs     : bool,
    pub print_skipped: bool,
    pub infos        : Vec<String>,
    pub errors       : Vec<String>,
    time_beg         : u64,
    time_end         : u64,
    time_bsy         : u64,
}

impl SimplePipelineFilter {
    pub fn new() -> Self {
        SimplePipelineFilter {
            skip_vcs     : true,
            print_skipped: false,
            infos        : Vec::new(),
            errors       : Vec::new(),
            time_beg     : 0,
            time_end     : 0,
            time_bsy     : 0,
        }
    }

    fn filter_path( &mut self, path: PathBuf ) -> Option<PathBuf> {
        let path_org = path.clone();

        let result = catch::<_, Option<PathBuf>, Error> ( || {
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
                    return Ok( None )
                }
            }

            return Ok( Some( path ) )

        } );

        match result {
            Ok ( r ) => r,
            Err( e ) => {
                self.errors.push( format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), path_org ) );
                None
            },
        }
    }
}

impl PipelineFilter for SimplePipelineFilter {
    fn filter( &mut self, rx: Receiver<PipelineInfo<PathBuf>>, tx: Sender<PipelineInfo<PathBuf>> ) {
        loop {
            match rx.recv() {
                Ok( PipelineInfo::Ok( p ) ) => {
                    let beg = time::precise_time_ns();

                    let path = self.filter_path( p );
                    match path {
                        Some( p ) => { let _ = tx.send( PipelineInfo::Ok( p ) ); },
                        None      => (),
                    }

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
    fn test_simple_pipeline_filter() {
        let mut filter = SimplePipelineFilter::new();

        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        thread::spawn( move || {
            filter.filter( in_rx, out_tx );
        } );

        let _ = in_tx.send( PipelineInfo::Begin );
        let _ = in_tx.send( PipelineInfo::Ok( PathBuf::from( "./src/pipeline_filter.rs" ) ) );
        let _ = in_tx.send( PipelineInfo::Ok( PathBuf::from( "./.git/aaaa"              ) ) );
        let _ = in_tx.send( PipelineInfo::Ok( PathBuf::from( "./.hg/aaaa"               ) ) );
        let _ = in_tx.send( PipelineInfo::Ok( PathBuf::from( "./.svn/aaaa"              ) ) );
        let _ = in_tx.send( PipelineInfo::Ok( PathBuf::from( "./.bzr/aaaa"              ) ) );
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

        assert!(  ret.contains( &PathBuf::from( "./src/pipeline_filter.rs" ) ) );
        assert!( !ret.contains( &PathBuf::from( "./.git/aaaa"              ) ) );
        assert!( !ret.contains( &PathBuf::from( "./.hg/aaaa"               ) ) );
        assert!( !ret.contains( &PathBuf::from( "./.svn/aaaa"              ) ) );
        assert!( !ret.contains( &PathBuf::from( "./.bzr/aaaa"              ) ) );

        assert!( time_bsy != 0 );
        assert!( time_all != 0 );
        assert!( time_bsy < time_all );
    }
}

