use crate::console::{Console, ConsoleTextKind};
use crate::pipeline::{Pipeline, PipelineInfo};
use crate::pipeline_matcher::PathMatch;
use crate::util::{catch, decode_error};
use crossbeam::channel::{Receiver, Sender};
use memmap::Mmap;
use std::fs::File;
use std::io::Error;
use std::ops::Deref;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------------------------------
// PipelinePrinter
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelinePrinter {
    pub is_color: bool,
    pub print_file: bool,
    pub print_column: bool,
    pub print_row: bool,
    pub print_line_by_match: bool,
    pub infos: Vec<String>,
    pub errors: Vec<String>,
    console: Console,
    time_beg: Instant,
    time_bsy: Duration,
}

impl Default for PipelinePrinter {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelinePrinter {
    pub fn new() -> Self {
        PipelinePrinter {
            is_color: true,
            print_file: true,
            print_column: false,
            print_row: false,
            print_line_by_match: false,
            infos: Vec::new(),
            errors: Vec::new(),
            console: Console::new(),
            time_beg: Instant::now(),
            time_bsy: Duration::new(0, 0),
        }
    }

    fn print_match(&mut self, pm: PathMatch) {
        if pm.matches.is_empty() {
            return;
        }
        self.console.is_color = self.is_color;

        let result = catch::<_, (), Error>(|| {
            let file = File::open(&pm.path)?;
            let mmap = unsafe { Mmap::map(&file) }?;
            let src = mmap.deref();

            let mut pos = 0;
            let mut column = 0;
            let mut last_lf = 0;
            let mut last_line_beg = usize::MAX;
            let mut last_m_end = usize::MAX;

            if self.print_line_by_match {
                for m in &pm.matches {
                    if self.print_file {
                        self.console.write(ConsoleTextKind::Filename, pm.path.to_str().unwrap());
                        self.console.write(ConsoleTextKind::Filename, ":");
                    }
                    if self.print_column | self.print_row {
                        while pos < m.beg {
                            if src[pos] == 0x0a {
                                column += 1;
                                last_lf = pos;
                            }
                            pos += 1;
                        }
                        if self.print_column {
                            self.console.write(ConsoleTextKind::Other, &format!("{}:", column + 1));
                        }
                        if self.print_row {
                            self.console
                                .write(ConsoleTextKind::Other, &format!("{}:", m.beg - last_lf));
                        }
                    }

                    self.console.write_match_line(src, m);
                }
            } else {
                for m in &pm.matches {
                    let line_beg = Console::get_line_beg(src, m.beg);

                    if last_line_beg != line_beg {
                        if last_m_end != usize::MAX {
                            let line_end = Console::get_line_end(src, last_m_end);
                            self.console.write_to_linebreak(src, last_m_end, line_end);
                        }

                        if self.print_file {
                            self.console.write(ConsoleTextKind::Filename, pm.path.to_str().unwrap());
                            self.console.write(ConsoleTextKind::Filename, ":");
                        }
                        if self.print_column | self.print_row {
                            while pos < m.beg {
                                if src[pos] == 0x0a {
                                    column += 1;
                                    last_lf = pos;
                                }
                                pos += 1;
                            }
                            if self.print_column {
                                self.console.write(ConsoleTextKind::Other, &format!("{}:", column + 1));
                            }
                            if self.print_row {
                                self.console
                                    .write(ConsoleTextKind::Other, &format!("{}:", m.beg - last_lf));
                            }
                        }

                        self.console.write_match_part(src, m, line_beg);
                    } else {
                        self.console.write_match_part(src, m, last_m_end);
                    }

                    last_line_beg = line_beg;
                    last_m_end = m.end;
                }

                if last_m_end != usize::MAX {
                    let line_end = Console::get_line_end(src, last_m_end);
                    self.console.write_to_linebreak(src, last_m_end, line_end);
                }
            }

            Ok(())
        });
        match result {
            Ok(_) => (),
            Err(e) => self.console.write(
                ConsoleTextKind::Error,
                &format!("Error: {} @ {:?}\n", decode_error(e.kind()), pm.path),
            ),
        }
    }
}

impl Pipeline<PathMatch, ()> for PipelinePrinter {
    fn setup(&mut self, id: usize, rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<()>>) {
        self.infos = Vec::new();
        self.errors = Vec::new();
        let mut seq_beg_arrived = false;

        loop {
            match rx.recv() {
                Ok(PipelineInfo::SeqDat(x, pm)) => {
                    watch_time!(self.time_bsy, {
                        self.print_match(pm);
                        let _ = tx.send(PipelineInfo::SeqDat(x, ()));
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

                Ok(PipelineInfo::MsgDebug(_, e)) => {
                    self.console.write(ConsoleTextKind::Info, &format!("{}\n", e));
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
