#![feature(test)]

extern crate amber;
extern crate rand;
extern crate test;

use amber::matcher::{BruteForceMatcher, FjsMatcher, Matcher, QuickSearchMatcher, TbmMatcher};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use test::Bencher;

// ---------------------------------------------------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------------------------------------------------

fn make_src() -> Box<[u8]> {
    let seed: [u8; 32] = [1; 32];
    let mut rng: StdRng = SeedableRng::from_seed(seed);

    const SRC_LEN: usize = 1024 * 1024 * 4;
    let mut src = Box::new([0u8; SRC_LEN]);
    for i in 0..SRC_LEN {
        src[i] = rng.gen();
    }
    src
}

fn make_pat(src: &[u8]) -> Box<[u8]> {
    let seed: [u8; 32] = [1; 32];
    let mut rng: StdRng = SeedableRng::from_seed(seed);

    const PAT_LEN: usize = 16;
    let src_len = src.len();
    let mut pat = Box::new([0u8; PAT_LEN]);
    let pos = rng.gen::<usize>() % (src_len - PAT_LEN - 1);
    for i in 0..PAT_LEN {
        pat[i] = src[i + pos];
    }
    pat
}

fn bench(b: &mut Bencher, m: &dyn Matcher) {
    let src = make_src();
    let pat = make_pat(&src);

    b.iter(|| {
        let ret = m.search(&*src, &*pat);
        assert!(ret.len() > 0);
    });
}

// ---------------------------------------------------------------------------------------------------------------------
// Normal
// ---------------------------------------------------------------------------------------------------------------------

#[bench]
fn normal_brute_force(b: &mut Bencher) {
    let m = BruteForceMatcher::new();
    bench(b, &m);
}

#[bench]
fn normal_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 1;
    bench(b, &m);
}

#[bench]
fn normal_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 1;
    bench(b, &m);
}

#[bench]
fn normal_fjs(b: &mut Bencher) {
    let mut m = FjsMatcher::new();
    m.max_threads = 1;
    bench(b, &m);
}

// ---------------------------------------------------------------------------------------------------------------------
// Multithread
// ---------------------------------------------------------------------------------------------------------------------

#[bench]
fn thread2_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 2;
    bench(b, &m);
}

#[bench]
fn thread2_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 2;
    bench(b, &m);
}

#[bench]
fn thread2_fjs(b: &mut Bencher) {
    let mut m = FjsMatcher::new();
    m.max_threads = 2;
    bench(b, &m);
}

#[bench]
fn thread4_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 4;
    bench(b, &m);
}

#[bench]
fn thread4_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 4;
    bench(b, &m);
}

#[bench]
fn thread4_fjs(b: &mut Bencher) {
    let mut m = FjsMatcher::new();
    m.max_threads = 4;
    bench(b, &m);
}

#[bench]
fn thread8_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 8;
    bench(b, &m);
}

#[bench]
fn thread8_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 8;
    bench(b, &m);
}

#[bench]
fn thread8_fjs(b: &mut Bencher) {
    let mut m = FjsMatcher::new();
    m.max_threads = 8;
    bench(b, &m);
}

// ---------------------------------------------------------------------------------------------------------------------
// SSE
// ---------------------------------------------------------------------------------------------------------------------

#[cfg(feature = "sse")]
#[bench]
fn sse_thread1_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 1;
    m.use_sse = true;
    bench(b, &m);
}

#[cfg(feature = "sse")]
#[bench]
fn sse_thread1_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 1;
    m.use_sse = true;
    bench(b, &m);
}

#[cfg(feature = "sse")]
#[bench]
fn sse_thread2_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 2;
    m.use_sse = true;
    bench(b, &m);
}

#[cfg(feature = "sse")]
#[bench]
fn sse_thread2_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 2;
    m.use_sse = true;
    bench(b, &m);
}

#[cfg(feature = "sse")]
#[bench]
fn sse_thread4_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 4;
    m.use_sse = true;
    bench(b, &m);
}

#[cfg(feature = "sse")]
#[bench]
fn sse_thread4_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 4;
    m.use_sse = true;
    bench(b, &m);
}

#[cfg(feature = "sse")]
#[bench]
fn sse_thread8_quick_search(b: &mut Bencher) {
    let mut m = QuickSearchMatcher::new();
    m.max_threads = 8;
    m.use_sse = true;
    bench(b, &m);
}

#[cfg(feature = "sse")]
#[bench]
fn sse_thread8_tbm(b: &mut Bencher) {
    let mut m = TbmMatcher::new();
    m.max_threads = 8;
    m.use_sse = true;
    bench(b, &m);
}
