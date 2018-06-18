use console::{Console, ConsoleTextKind};
use crossbeam_channel::{Receiver, Sender};
use ctrlc;
use getch::Getch;
use memmap::Mmap;
use pipeline::{Pipeline, PipelineInfo};
use pipeline_matcher::PathMatch;
use std::fs::{self, File};
use std::io::{Error, Write};
use std::ops::Deref;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use util::{catch, decode_error, exit};

// ---------------------------------------------------------------------------------------------------------------------
// PipelineReplacer
// ---------------------------------------------------------------------------------------------------------------------

pub struct PipelineReplacer {
    pub is_color: bool,
    pub is_interactive: bool,
    pub print_file: bool,
    pub print_column: bool,
    pub print_row: bool,
    pub infos: Vec<String>,
    pub errors: Vec<String>,
    console: Console,
    all_replace: bool,
    replacement: Vec<u8>,
    time_beg: Instant,
    time_bsy: Duration,
}

impl PipelineReplacer {
    pub fn new(replacement: &[u8]) -> Self {
        PipelineReplacer {
            is_color: true,
            is_interactive: true,
            print_file: true,
            print_column: false,
            print_row: false,
            infos: Vec::new(),
            errors: Vec::new(),
            console: Console::new(),
            all_replace: false,
            replacement: Vec::from(replacement),
            time_beg: Instant::now(),
            time_bsy: Duration::new(0, 0),
        }
    }

    fn replace_match(&mut self, pm: PathMatch) {
        if pm.matches.is_empty() {
            return;
        }
        self.console.is_color = self.is_color;

        let result = catch::<_, (), Error>(|| {
            let mut tmpfile = try!(NamedTempFile::new_in(pm.path.parent().unwrap_or(&pm.path)));

            let tmpfile_path = tmpfile.path().to_path_buf();
            let _ = ctrlc::set_handler(move || {
                let path = tmpfile_path.clone();
                let mut console = Console::new();
                console.write(
                    ConsoleTextKind::Info,
                    &format!("\nCleanup temporary file: {:?}\n", path),
                );
                let _ = fs::remove_file(path);
                exit(0, &mut console);
            });

            {
                let file = try!(File::open(&pm.path));
                let mmap = try!(unsafe { Mmap::map(&file) });
                let src = mmap.deref();

                let mut i = 0;
                let mut pos = 0;
                let mut column = 0;
                let mut last_lf = 0;
                for m in &pm.matches {
                    try!(tmpfile.write_all(&src[i..m.beg]));

                    let mut do_replace = true;
                    if self.is_interactive & !self.all_replace {
                        if self.print_file {
                            self.console.write(ConsoleTextKind::Filename, pm.path.to_str().unwrap());
                            self.console.write(ConsoleTextKind::Other, ": ");
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

                        loop {
                            self.console.write(
                                ConsoleTextKind::Other,
                                "Replace keyword? ( Yes[Y], No[N], All[A], Quit[Q] ):",
                            );
                            self.console.flush();
                            let getch = Getch::new();
                            let key = getch.getch()?;
                            let key = char::from(key);
                            self.console.write(ConsoleTextKind::Other, &format!("{}\n", key));
                            match key.to_ascii_lowercase() {
                                'y' => {
                                    do_replace = true;
                                    break;
                                }
                                'n' => {
                                    do_replace = false;
                                    break;
                                }
                                'a' => {
                                    self.all_replace = true;
                                    break;
                                }
                                'q' => {
                                    let _ = tmpfile.close();
                                    exit(0, &mut self.console);
                                }
                                _ => continue,
                            }
                        }
                    }

                    if do_replace {
                        try!(tmpfile.write_all(&self.replacement));
                    } else {
                        try!(tmpfile.write_all(&src[m.beg..m.end]));
                    }
                    i = m.end;
                }

                if i < src.len() {
                    try!(tmpfile.write_all(&src[i..src.len()]));
                }
                try!(tmpfile.flush());
            }

            let real_path = try!(fs::canonicalize(&pm.path));

            let metadata = try!(fs::metadata(&real_path));

            try!(fs::set_permissions(tmpfile.path(), metadata.permissions()));
            try!(tmpfile.persist(&real_path));

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

impl Pipeline<PathMatch, ()> for PipelineReplacer {
    fn setup(&mut self, id: usize, rx: Receiver<PipelineInfo<PathMatch>>, tx: Sender<PipelineInfo<()>>) {
        self.infos = Vec::new();
        self.errors = Vec::new();
        let mut seq_beg_arrived = false;

        loop {
            match rx.recv() {
                Ok(PipelineInfo::SeqDat(x, pm)) => {
                    watch_time!(self.time_bsy, {
                        self.replace_match(pm);
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
