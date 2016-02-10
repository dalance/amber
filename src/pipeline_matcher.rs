use matcher::{Match, Matcher};
use memmap::{Mmap, Protection};
use pipeline_finder::PathInfo;
use std::io::{Error, Read};
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use time;
use util::{catch, decode_error, PipelineInfo};

// ---------------------------------------------------------------------------------------------------------------------
// PipelineMatcher
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug,Clone)]
pub struct PathMatch {
    pub id     : usize     ,
    pub path   : PathBuf   ,
    pub matches: Vec<Match>,
}

pub trait PipelineMatcher {
    fn search( &mut self, matcher: &Matcher, keyword: &[u8], rx: Receiver<PipelineInfo<PathInfo>>, tx: Sender<PipelineInfo<PathMatch>> );
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePipelineMatcher
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePipelineMatcher {
    pub skip_binary       : bool,
    pub print_skipped     : bool,
    pub binary_check_bytes: usize,
    pub infos             : Vec<String>,
    pub errors            : Vec<String>,
    time_beg              : u64,
    time_end              : u64,
    time_bsy              : u64,
}

impl SimplePipelineMatcher {
    pub fn new() -> Self {
        SimplePipelineMatcher {
            skip_binary       : true,
            print_skipped     : false,
            binary_check_bytes: 128,
            infos             : Vec::new(),
            errors            : Vec::new(),
            time_beg          : 0,
            time_end          : 0,
            time_bsy          : 0,
        }
    }

    fn search_path( &mut self, matcher: &Matcher, keyword: &[u8], info: PathInfo ) -> PathMatch {
        let path_org = info.path.clone();

        let result = catch::<_, PathMatch, Error> ( || {

            let mmap;
            let mut buf = Vec::new();
            let src = if info.len > 1024 * 1024 {
                mmap = try!( Mmap::open_path( &info.path, Protection::Read ) );
                unsafe { mmap.as_slice() }
            } else {
                let mut f = try!( File::open( &info.path ) );
                try!( f.read_to_end( &mut buf ) );
                &buf[..]
            };

            if self.skip_binary {
                let mut is_binary = false;
                let check_bytes = if self.binary_check_bytes < src.len() { self.binary_check_bytes } else { src.len() };
                for i in 0..check_bytes {
                    if src[i] <= 0x08 {
                        is_binary = true;
                    }
                }
                if is_binary {
                    if self.print_skipped {
                        self.infos.push( format!( "Skipped: {:?} ( binary file )\n", info.path ) );
                    }
                    return Ok( PathMatch { id: info.id, path: info.path.clone(), matches: Vec::new() } )
                }
            }

            let ret = matcher.search( src, keyword );

            Ok( PathMatch { id: info.id, path: info.path.clone(), matches: ret } )
        } );

        match result {
            Ok ( x ) => x,
            Err( e ) => {
                self.errors.push( format!( "Error: {} @ {:?}\n", decode_error( e.kind() ), path_org ) );
                PathMatch { id: info.id, path: info.path.clone(), matches: Vec::new() }
            },
        }
    }
}

impl PipelineMatcher for SimplePipelineMatcher {
    fn search( &mut self, matcher: &Matcher, keyword: &[u8], rx: Receiver<PipelineInfo<PathInfo>>, tx: Sender<PipelineInfo<PathMatch>> ) {
        loop {
            match rx.recv() {
                Ok( PipelineInfo::Ok( p ) ) => {
                    let beg = time::precise_time_ns();

                    let ret = self.search_path( matcher, keyword, p );
                    //if !ret.matches.is_empty() { let _ = tx.send( PipelineInfo::Ok( ret ) ); }
                    let _ = tx.send( PipelineInfo::Ok( ret ) );

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
    use matcher::QuickSearchMatcher;
    use pipeline_finder::PathInfo;
    use std::path::PathBuf;
    use std::thread;
    use std::sync::mpsc;
    use util::PipelineInfo;

    #[test]
    fn test_simple_pipeline_matcher() {
        let mut matcher = SimplePipelineMatcher::new();
        let qs = QuickSearchMatcher::new();

        let ( in_tx , in_rx  ) = mpsc::channel();
        let ( out_tx, out_rx ) = mpsc::channel();
        thread::spawn( move || {
            matcher.search( &qs, &"amber".to_string().into_bytes(), in_rx, out_tx );
        } );

        let _ = in_tx.send( PipelineInfo::Begin );
        let _ = in_tx.send( PipelineInfo::Ok( PathInfo{ id: 0, path: PathBuf::from( "./src/ambs.rs" ), len: 1 } ) );
        let _ = in_tx.send( PipelineInfo::Ok( PathInfo{ id: 1, path: PathBuf::from( "./src/ambr.rs" ), len: 1 } ) );
        let _ = in_tx.send( PipelineInfo::Ok( PathInfo{ id: 2, path: PathBuf::from( "./src/util.rs" ), len: 1 } ) );
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

        for r in ret {
            if r.path == PathBuf::from( "./src/ambs.rs" ) { assert!( !r.matches.is_empty() ); }
            if r.path == PathBuf::from( "./src/ambr.rs" ) { assert!( !r.matches.is_empty() ); }
            if r.path == PathBuf::from( "./src/util.rs" ) { assert!( r.matches.is_empty()  ); }
        }

        assert!( time_bsy != 0 );
        assert!( time_all != 0 );
        assert!( time_bsy < time_all );
    }
}

