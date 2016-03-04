extern crate time;

use std::fs::File;
use std::io::{BufReader, Read, Error, ErrorKind};

// ---------------------------------------------------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------------------------------------------------

#[cfg(feature = "statistics")]
macro_rules! watch_time (
    ( $total:expr, $func:block ) => (
        {
            let beg = time::precise_time_ns();
            $func;
            let end = time::precise_time_ns();
            $total += end - beg;
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

pub fn watch_time<F>( closure: F ) -> u64 where F: FnOnce() -> () {
    let start = time::precise_time_ns();
    closure();
    let end = time::precise_time_ns();
    end - start
}

pub fn catch<F, T, E>( closure: F ) -> Result<T, E> where F: FnOnce() -> Result<T, E> {
    closure()
}

pub fn read_from_file( path: &str ) -> Result<Vec<u8>, Error> {
    let file = match File::open( path ) {
        Ok ( x ) => x,
        Err( e ) => return Err( e ),
    };
    let mut reader = BufReader::new( file );
    let mut ret: String = String::new();
    let _ = reader.read_to_string( &mut ret );
    Ok( ret.into_bytes() )
}

pub fn decode_error( e: ErrorKind ) -> &'static str {
    match e {
        ErrorKind::NotFound          => "file not found",
        ErrorKind::PermissionDenied  => "permission denied",
        ErrorKind::ConnectionRefused => "connection refused",
        ErrorKind::ConnectionReset   => "connection reset",
        ErrorKind::ConnectionAborted => "connection aborted",
        ErrorKind::NotConnected      => "not connected",
        ErrorKind::AddrInUse         => "address is in use",
        ErrorKind::AddrNotAvailable  => "address is not available",
        ErrorKind::BrokenPipe        => "broken pipe",
        ErrorKind::AlreadyExists     => "file is already exists",
        ErrorKind::WouldBlock        => "world be blocked",
        ErrorKind::InvalidInput      => "invalid parameter",
        ErrorKind::InvalidData       => "invalid data",
        ErrorKind::TimedOut          => "operation timeout",
        ErrorKind::WriteZero         => "write size is zero",
        ErrorKind::Interrupted       => "interrupted",
        ErrorKind::Other             => "unknown",
        _                            => "unknown",
    }
}

pub enum PipelineInfo<T> {
    Beg ( usize    ),
    Ok  ( T        ),
    Info( String   ),
    Err ( String   ),
    Time( u64, u64 ),
    End ( usize    ),
}
