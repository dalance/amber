use crate::matcher::{Match, Matcher};
use crate::pipeline::{Pipeline, PipelineInfo};
use crate::pipeline_finder::PathInfo;
use crate::util::{catch, decode_error};
use crossbeam::channel::{Receiver, Sender};
use memmap::Mmap;
use std::fs::{self, File};
use std::io::{Error, Read};
use std::ops::Deref;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
    pub print_search: bool,
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
            print_search: false,
            binary_check_bytes: 128,
            mmap_bytes: 1024 * 1024,
            infos: Vec::new(),
            errors: Vec::new(),
            time_beg: Instant::now(),
            time_bsy: Duration::new(0, 0),
            matcher,
            keyword: Vec::from(keyword),
        }
    }

    fn search_path(&mut self, info: PathInfo) -> PathMatch {
        let path_org = info.path.clone();

        let result = catch::<_, PathMatch, Error>(|| {
            let attr = match fs::metadata(&info.path) {
                Ok(x) => x,
                Err(e) => {
                    return Err(e);
                }
            };

            let mmap;
            let mut buf = Vec::new();
            let src = if attr.len() > self.mmap_bytes {
                let file = File::open(&info.path)?;
                mmap = unsafe { Mmap::map(&file) }?;
                mmap.deref()
            } else {
                let mut f = File::open(&info.path)?;
                f.read_to_end(&mut buf)?;
                &buf[..]
            };

            if self.skip_binary {
                let mut is_binary = false;
                let check_bytes = if self.binary_check_bytes < src.len() {
                    self.binary_check_bytes
                } else {
                    src.len()
                };
                for byte in src.iter().take(check_bytes) {
                    if byte <= &0x08 {
                        is_binary = true;
                        break;
                    }
                }
                if is_binary {
                    if self.print_skipped {
                        self.infos.push(format!("Skip (binary)    : {:?}", info.path));
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
                    let mut path = None;
                    if self.print_search {
                        path = Some(p.path.clone());
                        let _ = tx.send(PipelineInfo::MsgDebug(id, format!("Search Start     : {:?}", p.path)));
                    }
                    watch_time!(self.time_bsy, {
                        let ret = self.search_path(p);
                        let _ = tx.send(PipelineInfo::SeqDat(x, ret));
                    });
                    if self.print_search {
                        let _ = tx.send(PipelineInfo::MsgDebug(
                            id,
                            format!("Search Finish    : {:?}", path.unwrap()),
                        ));
                    }
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

                Ok(PipelineInfo::MsgDebug(i, e)) => {
                    let _ = tx.send(PipelineInfo::MsgDebug(i, e));
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
    use crate::matcher::QuickSearchMatcher;
    use crate::pipeline::{Pipeline, PipelineInfo};
    use crate::pipeline_finder::PathInfo;
    use crossbeam::channel::unbounded;
    use std::path::PathBuf;
    use std::thread;

    #[test]
    fn pipeline_matcher() {
        let qs = QuickSearchMatcher::new();
        let mut matcher = PipelineMatcher::new(qs, &"amber".to_string().into_bytes());

        let (in_tx, in_rx) = unbounded();
        let (out_tx, out_rx) = unbounded();
        thread::spawn(move || {
            matcher.setup(0, in_rx, out_tx);
        });

        let _ = in_tx.send(PipelineInfo::SeqBeg(0));
        let _ = in_tx.send(PipelineInfo::SeqDat(
            0,
            PathInfo {
                path: PathBuf::from("./src/ambs.rs"),
            },
        ));
        let _ = in_tx.send(PipelineInfo::SeqDat(
            1,
            PathInfo {
                path: PathBuf::from("./src/ambr.rs"),
            },
        ));
        let _ = in_tx.send(PipelineInfo::SeqDat(
            2,
            PathInfo {
                path: PathBuf::from("./src/util.rs"),
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
