#![feature(test)]

extern crate amber;
extern crate test;

//use amber::path_finder::{PathFinder, SimplePathFinder};
use std::path::PathBuf;
use test::Bencher;

// ---------------------------------------------------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------------------------------------------------

//#[bench]
//fn bench_simple_path_finder( b: &mut Bencher ) {
//    b.iter( || {
//        let mut finder = SimplePathFinder::new();
//        finder.find( vec![PathBuf::from( "/usr/share" )] );
//    } );
//}
