extern crate term;

use crate::matcher::Match;
use std::io;
use std::io::Write;
use std::process;
use term::color::Color;
use term::{StderrTerminal, StdoutTerminal};

// ---------------------------------------------------------------------------------------------------------------------
// Console
// ---------------------------------------------------------------------------------------------------------------------

pub enum ConsoleTextKind {
    Filename,
    Text,
    MatchText,
    Other,
    Info,
    Error,
}

pub struct Console {
    pub is_color: bool,
    term_stdout: Box<StdoutTerminal>,
    term_stderr: Box<StderrTerminal>,
    color_out: Color,
    color_err: Color,
    colored_out: bool,
    colored_err: bool,
}

impl Console {
    pub fn new() -> Self {
        Console {
            term_stdout: term::stdout().unwrap_or_else(|| {
                process::exit(1);
            }),
            term_stderr: term::stderr().unwrap_or_else(|| {
                process::exit(1);
            }),
            is_color: true,
            color_out: term::color::BLACK,
            color_err: term::color::BLACK,
            colored_out: false,
            colored_err: false,
        }
    }

    pub fn carriage_return(&mut self) {
        let _ = self.term_stdout.carriage_return();
    }

    pub fn cursor_up(&mut self) {
        let _ = self.term_stdout.cursor_up();
    }

    pub fn delete_line(&mut self) {
        let _ = self.term_stdout.delete_line();
    }

    pub fn write_with_clear(&mut self, kind: ConsoleTextKind, val: &str) {
        self.carriage_return();
        self.delete_line();
        self.write(kind, val);
    }

    pub fn write(&mut self, kind: ConsoleTextKind, val: &str) {
        let color = match kind {
            ConsoleTextKind::Filename => term::color::BRIGHT_GREEN,
            ConsoleTextKind::Text => term::color::WHITE,
            ConsoleTextKind::MatchText => term::color::BRIGHT_YELLOW,
            ConsoleTextKind::Other => term::color::BRIGHT_CYAN,
            ConsoleTextKind::Info => term::color::BRIGHT_CYAN,
            ConsoleTextKind::Error => term::color::BRIGHT_RED,
        };

        match kind {
            ConsoleTextKind::Error => self.write_stderr(val, color),
            ConsoleTextKind::Info => self.write_stderr(val, color),
            _ => self.write_stdout(val, color),
        }
    }

    pub fn flush(&mut self) {
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
    }

    pub fn reset(&mut self) {
        self.term_stdout.reset().unwrap_or_else(|_| {
            process::exit(1);
        });
        self.term_stderr.reset().unwrap_or_else(|_| {
            process::exit(1);
        });
    }

    pub fn write_match_line(&mut self, src: &[u8], m: &Match) {
        let mut beg = m.beg;
        let mut end = m.end;
        while beg > 0 {
            if src[beg] == 0x0d || src[beg] == 0x0a {
                beg += 1;
                break;
            }
            beg -= 1;
        }
        while src.len() > end {
            if src[end] == 0x0d || src[end] == 0x0a {
                end -= 1;
                break;
            }
            end += 1;
        }
        if src.len() <= end {
            end = src.len()
        } else {
            end += 1
        };

        if beg < m.beg {
            self.write(ConsoleTextKind::Text, &String::from_utf8_lossy(&src[beg..m.beg]));
        }
        self.write(ConsoleTextKind::MatchText, &String::from_utf8_lossy(&src[m.beg..m.end]));
        if m.end < end {
            self.write(ConsoleTextKind::Text, &String::from_utf8_lossy(&src[m.end..end]));
        }
        self.write(ConsoleTextKind::Text, "\n");
    }

    pub fn write_replace_line(&mut self, src: &[u8], m: &Match, rep: &[u8]) {
        let mut beg = m.beg;
        let mut end = m.end;
        while beg > 0 {
            if src[beg] == 0x0d || src[beg] == 0x0a {
                beg += 1;
                break;
            }
            beg -= 1;
        }
        while src.len() > end {
            if src[end] == 0x0d || src[end] == 0x0a {
                end -= 1;
                break;
            }
            end += 1;
        }
        if src.len() <= end {
            end = src.len()
        } else {
            end += 1
        };

        if beg < m.beg {
            self.write(ConsoleTextKind::Text, &String::from_utf8_lossy(&src[beg..m.beg]));
        }
        self.write(ConsoleTextKind::MatchText, &String::from_utf8_lossy(&rep));
        if m.end < end {
            self.write(ConsoleTextKind::Text, &String::from_utf8_lossy(&src[m.end..end]));
        }
        self.write(ConsoleTextKind::Text, "\n");
    }

    fn write_stdout(&mut self, val: &str, color: Color) {
        if self.is_color {
            if self.color_out != color {
                self.term_stdout.fg(color).unwrap_or_else(|_| {
                    process::exit(1);
                });
                self.color_out = color;
                self.colored_out = true;
            }
        }

        write!(self.term_stdout, "{}", val).unwrap_or_else(|_| {
            process::exit(1);
        });

        //if self.is_color {
        //    self.term_stdout.reset().unwrap_or_else( |_| { process::exit( 1 ); } );
        //}

        //let _ = io::stdout().flush();
    }

    fn write_stderr(&mut self, val: &str, color: Color) {
        if self.is_color {
            if self.color_err != color {
                self.term_stderr.fg(color).unwrap_or_else(|_| {
                    process::exit(1);
                });
                self.color_err = color;
                self.colored_err = true;
            }
        }

        write!(self.term_stderr, "{}", val).unwrap_or_else(|_| {
            process::exit(1);
        });

        //if self.is_color {
        //    self.term_stderr.reset().unwrap_or_else( |_| { process::exit( 1 ); } );
        //}

        //let _ = io::stderr().flush();
    }
}
