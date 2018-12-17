#![cfg_attr(feature = "sse", feature(asm))]

#[macro_use]
pub mod util;
pub mod console;
pub mod ignore;
pub mod matcher;
pub mod pipeline;
pub mod pipeline_finder;
pub mod pipeline_matcher;
pub mod pipeline_printer;
pub mod pipeline_replacer;
pub mod pipeline_sorter;
