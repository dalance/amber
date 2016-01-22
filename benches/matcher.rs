#![feature(test)]

extern crate amber;
extern crate rand;
extern crate test;

use amber::matcher::{Matcher, QuickSearchMatcher, BruteForceMatcher};
use rand::{Rng, StdRng, SeedableRng};
use test::Bencher;

// ---------------------------------------------------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------------------------------------------------

fn make_src() -> Box<[u8]> {
    let seed: &[_] = &[1, 2, 3, 4];
    let mut rng: StdRng = SeedableRng::from_seed( seed );

    const SRC_LEN: usize = 1024 * 1024;
    let mut src = Box::new( [0u8;SRC_LEN] );
    for i in 0..SRC_LEN {
        src[i] = rng.gen();
    }
    src
}

fn make_pat( src: &[u8] ) -> Box<[u8]> {
    let seed: &[_] = &[1, 2, 3, 4];
    let mut rng: StdRng = SeedableRng::from_seed( seed );

    const PAT_LEN: usize = 16;
    let src_len = src.len();
    let mut pat = Box::new( [0u8;PAT_LEN] );
    let pos = rng.gen::<usize>() % ( src_len - PAT_LEN );
    for i in 0..PAT_LEN {
        pat[i] = src[i+pos];
    }
    pat
}

#[bench]
fn bench_brute_force_matcher( b: &mut Bencher ) {
    let src = make_src();

    b.iter( || {
        let pat = make_pat( &src );
        let matcher = BruteForceMatcher::new();
        let ret = matcher.search( &*src, &*pat );
        assert!( ret.len() > 0 );
    } );
}

#[bench]
fn bench_quick_search_matcher_thread1( b: &mut Bencher ) {
    let src = make_src();

    b.iter( || {
        let pat = make_pat( &src );
        let mut matcher = QuickSearchMatcher::new();
        matcher.max_threads = 1;
        let ret = matcher.search( &*src, &*pat );
        assert!( ret.len() > 0 );
    } );
}

#[bench]
fn bench_quick_search_matcher_thread4( b: &mut Bencher ) {
    let src = make_src();

    b.iter( || {
        let pat = make_pat( &src );
        let mut matcher = QuickSearchMatcher::new();
        matcher.max_threads = 4;
        let ret = matcher.search( &*src, &*pat );
        assert!( ret.len() > 0 );
    } );
}
