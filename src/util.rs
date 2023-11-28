use crate::console::Console;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::process;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------------------------------------------------

#[cfg(feature = "statistics")]
macro_rules! watch_time (
    ( $total:expr, $func:block ) => (
        {
            let beg = Instant::now();
            $func;
            $total += beg.elapsed();
        }
    );
);

#[cfg(not(feature = "statistics"))]
macro_rules! watch_time (
    ( $total:expr, $func:block ) => (
        {
            $func;
        }
    );
);

pub fn watch_time<F>(closure: F) -> Duration
where
    F: FnOnce(),
{
    let start = Instant::now();
    closure();
    start.elapsed()
}

pub fn catch<F, T, E>(closure: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E>,
{
    closure()
}

pub fn as_secsf64(dur: Duration) -> f64 {
    (dur.as_secs() as f64) + (dur.subsec_nanos() as f64 / 1_000_000_000.0)
}

pub fn read_from_file(path: &str) -> Result<Vec<u8>, Error> {
    let file = match File::open(path) {
        Ok(x) => x,
        Err(e) => return Err(e),
    };
    let mut reader = BufReader::new(file);
    let mut ret: String = String::new();
    let _ = reader.read_to_string(&mut ret);
    Ok(ret.into_bytes())
}

pub fn decode_error(e: ErrorKind) -> &'static str {
    match e {
        ErrorKind::NotFound => "file not found",
        ErrorKind::PermissionDenied => "permission denied",
        ErrorKind::ConnectionRefused => "connection refused",
        ErrorKind::ConnectionReset => "connection reset",
        ErrorKind::ConnectionAborted => "connection aborted",
        ErrorKind::NotConnected => "not connected",
        ErrorKind::AddrInUse => "address is in use",
        ErrorKind::AddrNotAvailable => "address is not available",
        ErrorKind::BrokenPipe => "broken pipe",
        ErrorKind::AlreadyExists => "file is already exists",
        ErrorKind::WouldBlock => "world be blocked",
        ErrorKind::InvalidInput => "invalid parameter",
        ErrorKind::InvalidData => "invalid data",
        ErrorKind::TimedOut => "operation timeout",
        ErrorKind::WriteZero => "write size is zero",
        ErrorKind::Interrupted => "interrupted",
        ErrorKind::Other => "unknown",
        _ => "unknown",
    }
}

pub enum PipelineInfo<T> {
    Beg(usize),
    Ok(T),
    Info(String),
    Err(String),
    Time(Duration, Duration),
    End(usize),
}

pub fn exit(code: i32, console: &mut Console) -> ! {
    console.reset();
    console.flush();
    process::exit(code);
}

#[cfg(not(windows))]
pub fn set_c_lflag(c_lflag: Option<termios::tcflag_t>) {
    if let Ok(mut termios) = termios::Termios::from_fd(0) {
        if let Some(c_lflag) = c_lflag {
            termios.c_lflag = c_lflag;
            let _ = termios::tcsetattr(0, termios::TCSADRAIN, &termios);
        }
    }
}

#[cfg(not(windows))]
pub fn get_c_lflag() -> Option<termios::tcflag_t> {
    if let Ok(termios) = termios::Termios::from_fd(0) {
        Some(termios.c_lflag)
    } else {
        None
    }
}
