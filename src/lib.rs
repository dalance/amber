#![cfg_attr(feature = "sse", feature(asm))]

extern crate docopt;
extern crate glob;
extern crate memmap;
extern crate num_cpus;
extern crate rand;
extern crate regex;
extern crate rlibc;
extern crate rustc_serialize;
extern crate scoped_threadpool;
extern crate time;
extern crate tempfile;
extern crate term;

pub mod console;
pub mod ignore;
pub mod matcher;
pub mod pipeline_finder;
pub mod pipeline_matcher;
pub mod pipeline_printer;
pub mod pipeline_queue;
pub mod pipeline_replacer;
pub mod util;
