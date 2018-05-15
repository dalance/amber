use matcher::{Match, Matcher};
use memmap::{Mmap, Protection};
use pipeline::{Pipeline, PipelineInfo};
use pipeline_finder::PathInfo;
use std::fs::File;
use std::io::{Error, Read};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};
use util::{catch, decode_error};

// ---------------------------------------------------------------------------------------------------------------------
// PathMatch
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PathMatch {
    pub path: PathBuf,
    pub matches: Vec<Match>,
}

// ---------------------------------------------------------------------------------------------------------------------
// PipelineMatcher
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelineMatcher<T: Matcher> {
    pub skip_binary: bool,
    pub print_skipped: bool,
    pub binary_check_bytes: usize,
    pub mmap_bytes: u64,
    pub infos: Vec<String>,
    pub errors: Vec<String>,
    time_beg: Instant,
    time_bsy: Duration,
    matcher: T,
    keyword: Vec<u8>,
}

impl<T: Matcher> PipelineMatcher<T> {
    pub fn new(matcher: T, keyword: &[u8]) -> Self {
        PipelineMatcher {
            skip_binary: true,
            print_skipped: false,
            binary_check_bytes: 128,
            mmap_bytes: 1024 * 1024,
            infos: Vec::new(),
            errors: Vec::new(),
            time_beg: Instant::now(),
            time_bsy: Duration::new(0, 0),
            matcher: matcher,
            keyword: Vec::from(keyword),
        }
    }

    fn search_path(&mut self, info: PathInfo) -> PathMatch {
        let path_org = info.path.clone();

        let result = catch::<_, PathMatch, Error>(|| {
            let mmap;
            let mut buf = Vec::new();
            let src = if info.len > self.mmap_bytes {
                mmap = try!(Mmap::open_path(&info.path, Protection::Read));
                unsafe { mmap.as_slice() }
            } else {
                let mut f = try!(File::open(&info.path));
                try!(f.read_to_end(&mut buf));
                &buf[..]
            };

            if self.skip_binary {
                let mut is_binary = false;
                let check_bytes = if self.binary_check_bytes < src.len() {
                    self.binary_check_bytes
                } else {
                    src.len()
                };
                for i in 0..check_bytes {
                    if src[i] <= 0x08 {
                        is_binary = true;
                        break;
                    }
                }
                if is_binary {
                    if self.print_skipped {
                        self.infos.push(format!("Skipped: {:?} ( binary file )\n", info.path));
                    }
                    return Ok(PathMatch {
                        path: info.path.clone(),
                        matches: Vec::new(),
                    });
                }
            }

            let ret = self.matcher.search(src, &self.keyword);

            Ok(PathMatch {
                path: info.path.clone(),
                matches: ret,
            })
        });

        match result {
            Ok(x) => x,
            Err(e) => {
                self.errors
                    .push(format!("Error: {} @ {:?}\n", decode_error(e.kind()), path_org));
                PathMatch {
                    path: info.path.clone(),
                    matches: Vec::new(),
                }
            }
        }
    }
}

impl<T: Matcher> Pipeline<PathInfo, PathMatch> for PipelineMatcher<T> {
    fn setup(&mut self, id: usize, rx: Receiver<PipelineInfo<PathInfo>>, tx: Sender<PipelineInfo<PathMatch>>) {
        self.infos = Vec::new();
        self.errors = Vec::new();
        let mut seq_beg_arrived = false;

        loop {
            match rx.recv() {
                Ok(PipelineInfo::SeqDat(x, p)) => {
                    watch_time!(self.time_bsy, {
                        let ret = self.search_path(p);
                        let _ = tx.send(PipelineInfo::SeqDat(x, ret));
                    });
                }

                Ok(PipelineInfo::SeqBeg(x)) => {
                    if !seq_beg_arrived {
                        self.time_beg = Instant::now();
                        let _ = tx.send(PipelineInfo::SeqBeg(x));
                        seq_beg_arrived = true;
                    }
                }

                Ok(PipelineInfo::SeqEnd(x)) => {
                    for i in &self.infos {
                        let _ = tx.send(PipelineInfo::MsgInfo(id, i.clone()));
                    }
                    for e in &self.errors {
                        let _ = tx.send(PipelineInfo::MsgErr(id, e.clone()));
                    }

                    let _ = tx.send(PipelineInfo::MsgTime(id, self.time_bsy, self.time_beg.elapsed()));
                    let _ = tx.send(PipelineInfo::SeqEnd(x));
                    break;
                }

                Ok(PipelineInfo::MsgInfo(i, e)) => {
                    let _ = tx.send(PipelineInfo::MsgInfo(i, e));
                }
                Ok(PipelineInfo::MsgErr(i, e)) => {
                    let _ = tx.send(PipelineInfo::MsgErr(i, e));
                }
                Ok(PipelineInfo::MsgTime(i, t0, t1)) => {
                    let _ = tx.send(PipelineInfo::MsgTime(i, t0, t1));
                }
                Err(_) => break,
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
    use pipeline::{Pipeline, PipelineInfo};
    use pipeline_finder::PathInfo;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn pipeline_matcher() {
        let qs = QuickSearchMatcher::new();
        let mut matcher = PipelineMatcher::new(qs, &"amber".to_string().into_bytes());

        let (in_tx, in_rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::channel();
        thread::spawn(move || {
            matcher.setup(0, in_rx, out_tx);
        });

        let _ = in_tx.send(PipelineInfo::SeqBeg(0));
        let _ = in_tx.send(PipelineInfo::SeqDat(
            0,
            PathInfo {
                path: PathBuf::from("./src/ambs.rs"),
                len: 1,
            },
        ));
        let _ = in_tx.send(PipelineInfo::SeqDat(
            1,
            PathInfo {
                path: PathBuf::from("./src/ambr.rs"),
                len: 1,
            },
        ));
        let _ = in_tx.send(PipelineInfo::SeqDat(
            2,
            PathInfo {
                path: PathBuf::from("./src/util.rs"),
                len: 1,
            },
        ));
        let _ = in_tx.send(PipelineInfo::SeqEnd(3));

        let mut ret = Vec::new();
        loop {
            match out_rx.recv().unwrap() {
                PipelineInfo::SeqDat(_, x) => ret.push(x),
                PipelineInfo::SeqEnd(_) => break,
                _ => (),
            }
        }

        for r in ret {
            if r.path == PathBuf::from("./src/ambs.rs") {
                assert!(!r.matches.is_empty());
            }
            if r.path == PathBuf::from("./src/ambr.rs") {
                assert!(!r.matches.is_empty());
            }
            if r.path == PathBuf::from("./src/util.rs") {
                assert!(r.matches.is_empty());
            }
        }
    }
}
