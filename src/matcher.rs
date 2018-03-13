use regex::Regex;
use rlibc::memcmp;
use scoped_threadpool::Pool;
use std::cmp;
use std::collections::HashMap;
use std::str;
use std::sync::mpsc;

// ---------------------------------------------------------------------------------------------------------------------
// Matcher
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug,Clone)]
pub struct Match {
    pub beg      : usize,
    pub end      : usize,
    pub sub_match: Vec<Match>,
}

pub trait Matcher {
    fn search( &self, src: &[u8], pat: &[u8] ) -> Vec<Match>;
}

// ---------------------------------------------------------------------------------------------------------------------
// macro
// ---------------------------------------------------------------------------------------------------------------------

#[cfg(feature = "sse")]
macro_rules! cmp_pat_sse (
    ( $src:expr, $pat:expr, $pat_len:expr, $pat_len_by_dq:expr, $do_mismatch:block ) => (
        {
            let mut src_ptr = &( $src ) as *const u8 as usize;
            let mut pat_ptr = &( $pat ) as *const u8 as usize;
            let mut pat_rest = $pat_len;
            for _ in 0 .. $pat_len_by_dq {
                unsafe {
                    let ret_cmp: u64;
                    asm!(
                        "movdqu ($1), %xmm0                 \n\
                         pcmpestri $$0b0011000, ($2), %xmm0 \n\
                        "
                        : "={rcx}"( ret_cmp )
                        : "r"( pat_ptr ), "r"( src_ptr ), "{rdx}"( pat_rest ), "{rax}"( pat_rest )
                        : "{xmm0}"
                        :
                    );
                    if ret_cmp != 16 {
                        $do_mismatch
                    }
                    src_ptr  += 16;
                    pat_ptr  += 16;
                    pat_rest -= 16;
                }
            }
        }
    );
);

// ---------------------------------------------------------------------------------------------------------------------
// BruteForceMatcher
// ---------------------------------------------------------------------------------------------------------------------

pub struct BruteForceMatcher;

impl BruteForceMatcher {
    pub fn new() -> Self {
        BruteForceMatcher
    }
}

impl Matcher for BruteForceMatcher {
    fn search( &self, src: &[u8], pat: &[u8] ) -> Vec<Match> {
        let src_len = src.len();
        let pat_len = pat.len();
        let mut ret = Vec::new();

        let mut i = 0;
        while i < src_len - pat_len + 1 {
            if src[i] == pat[0] {
                let mut success = true;
                for j in 1 .. pat_len {
                    if src[i+j] != pat[j] {
                        success = false;
                        break;
                    }
                }

                if success {
                    ret.push( Match { beg: i, end: i + pat_len, sub_match: Vec::new() } );
                    i = i + pat_len - 1;
                }
            }

            i += 1;
        }

        ret
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// QuickSearchMatcher
// ---------------------------------------------------------------------------------------------------------------------

pub struct QuickSearchMatcher {
    pub max_threads    : usize,
    pub size_per_thread: usize,
}

impl QuickSearchMatcher {
    pub fn new() -> Self {
        QuickSearchMatcher {
            max_threads    : 4,
            size_per_thread: 1024 * 1024,
        }
    }

    fn search_sub( &self, src: &[u8], pat: &[u8], qs_table: &[usize;256], beg: usize, end: usize ) -> Vec<Match> {
        let src_len = src.len();
        let pat_len = pat.len();
        let mut ret = Vec::new();

        let src_ptr = src.as_ptr();
        let pat_ptr = pat.as_ptr();
        let qs_ptr  = qs_table.as_ptr();

        let mut i = beg;
        while i < end {
            if src_len < i+pat_len { break; }

            let success;
            unsafe {
                let ret = memcmp( src_ptr.offset( i as isize ), pat_ptr, pat_len );
                success = if ret == 0 { true } else { false };
            }

            if success {
                if MatcherUtil::check_char_boundary( src, i ) {
                    ret.push( Match { beg: i, end: i + pat_len, sub_match: Vec::new() } );
                    i += pat_len;
                    continue;
                }
            }

            if src_len <= i+pat_len { break; }
            unsafe {
                let t = *src_ptr.offset( ( i+pat_len ) as isize ) as isize;
                i += *qs_ptr.offset( t );
            }
        }

        ret
    }
}

impl Matcher for QuickSearchMatcher {
    fn search( &self, src: &[u8], pat: &[u8] ) -> Vec<Match> {
        let src_len = src.len();
        let pat_len = pat.len();

        let mut qs_table: [usize;256] = [pat_len+1;256];
        let mut i = 0;
        while i < pat_len {
            qs_table[pat[i] as usize] = pat_len - i;
            i += 1;
        }

        let thread_num = cmp::min( src_len / self.size_per_thread + 1, self.max_threads );

        if thread_num == 1 {
            self.search_sub( src, pat, &qs_table, 0, src_len )
        } else {
            let ( tx, rx ) = mpsc::channel();
            let mut pool = Pool::new( thread_num as u32 );

            pool.scoped( |scoped| {
                for i in 0..thread_num {
                    let tx  = tx.clone();
                    let beg = src_len * i / thread_num;
                    let end = src_len * ( i + 1 ) / thread_num;
                    scoped.execute( move || {
                        let tmp = self.search_sub( src, pat, &qs_table, beg, end );
                        let _ = tx.send( ( i, tmp ) );
                    } );
                }
            } );

            let mut rets = HashMap::new();
            for _ in 0..thread_num {
                let ( i, tmp ) = rx.recv().unwrap();
                rets.insert( i, tmp );
            }

            let mut ret = Vec::new();
            for i in 0..thread_num {
                let tmp = rets.get( &i ).unwrap();
                for t in tmp {
                    ret.push( t.clone() );
                }
            }
            ret
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// TbmMatcher
// ---------------------------------------------------------------------------------------------------------------------

pub struct TbmMatcher {
    pub max_threads    : usize,
    pub size_per_thread: usize,
}

impl TbmMatcher {
    pub fn new() -> Self {
        TbmMatcher {
            max_threads    : 4,
            size_per_thread: 1024 * 1024,
        }
    }

    fn search_sub( &self, src: &[u8], pat: &[u8], qs_table: &[usize;256], md2: usize, beg: usize, end: usize ) -> Vec<Match> {
        let src_len = src.len();
        let pat_len = pat.len();
        let mut ret = Vec::new();

        let src_ptr = src.as_ptr();
        let pat_ptr = pat.as_ptr();

        let mut i = beg + pat_len - 1;
        'outer: while i < end {
            let mut k = qs_table[src[i] as usize];
            while k != 0 {
                i += k;
                if i >= src_len { break 'outer; }
                k = qs_table[src[i] as usize];
            }

            if i >= end {
                break;
            }

            unsafe {
                let ret = memcmp( src_ptr.offset( ( i + 1 - pat_len ) as isize ), pat_ptr, pat_len );
                if ret != 0 {
                    i += md2;
                    continue 'outer;
                }
            }

            if MatcherUtil::check_char_boundary( src, i + 1 - pat_len ) {
                ret.push( Match { beg: i + 1 - pat_len, end: i + 1, sub_match: Vec::new() } );
                i += pat_len;
                continue;
            }

            i += md2;
        }

        ret
    }
}

impl Matcher for TbmMatcher {
    fn search( &self, src: &[u8], pat: &[u8] ) -> Vec<Match> {
        let src_len = src.len();
        let pat_len = pat.len();

        let mut qs_table: [usize;256] = [pat_len;256];
        for i in 0 .. pat_len {
            qs_table[pat[i] as usize] = pat_len - 1 - i;
        }

        let     pe: isize = pat_len as isize - 1;
        let mut p : isize = pe - 1;
        while p >= 0 {
            if pat[p as usize] == pat[pe as usize] {
                break;
            }
            p -= 1;
        }
        let md2 = ( pe - p ) as usize;

        let thread_num = cmp::min( src_len / self.size_per_thread + 1, self.max_threads );

        if thread_num == 1 {
            self.search_sub( src, pat, &qs_table, md2, 0, src_len )
        } else {
            let ( tx, rx ) = mpsc::channel();
            let mut pool = Pool::new( thread_num as u32 );

            pool.scoped( |scoped| {
                for i in 0..thread_num {
                    let tx  = tx.clone();
                    let beg = src_len * i / thread_num;
                    let end = src_len * ( i + 1 ) / thread_num;
                    scoped.execute( move || {
                        let tmp = self.search_sub( src, pat, &qs_table, md2, beg, end );
                        let _ = tx.send( ( i, tmp ) );
                    } );
                }
            } );

            let mut rets = HashMap::new();
            for _ in 0..thread_num {
                let ( i, tmp ) = rx.recv().unwrap();
                rets.insert( i, tmp );
            }

            let mut ret = Vec::new();
            for i in 0..thread_num {
                let tmp = rets.get( &i ).unwrap();
                for t in tmp {
                    ret.push( t.clone() );
                }
            }
            ret
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// FjsMatcher
// ---------------------------------------------------------------------------------------------------------------------

pub struct FjsMatcher {
    pub max_threads    : usize,
    pub size_per_thread: usize,
    pub use_sse        : bool ,
}

impl FjsMatcher {
    pub fn new() -> Self {
        FjsMatcher {
            max_threads    : 4,
            size_per_thread: 1024 * 1024,
            use_sse        : false,
        }
    }

    #[allow(unused_variables)]
    fn search_sub( &self, src: &[u8], pat: &[u8], betap: &[isize;101], delta: &[usize;256], beg: usize, end: usize ) -> Vec<Match> {
        let src_len = src.len();
        let pat_len = pat.len();
        let mut ret = Vec::new();

        let mut i = 0;
        let mut j = 0;
        let     mp = pat_len - 1;
        let mut ip = mp + beg;
        let mut prev: isize = -( pat_len as isize );

        while ip < end {
            if j <= 0 {
                if ip + 1 >= src_len {
                    return ret;
                }
                while pat[mp] != src[ip] {
                    ip += delta[src[ip+1] as usize];
                    if ip >= src_len {
                        return ret;
                    }
                }
                j = 0;
                i = ip - mp;
                while j < mp && src[i] == pat[j] {
                    i += 1;
                    j += 1;
                }
                if j == mp {
                    if MatcherUtil::check_char_boundary( src, i - mp ) {
                        if prev + pat_len as isize <= ( i - mp ) as isize {
                            ret.push( Match { beg: i - mp, end: i - mp + pat_len, sub_match: Vec::new() } );
                            prev = ( i - mp ) as isize;
                        }
                        i += 1;
                        j += 1;
                    }
                }
                if j <= 0 {
                    i += 1;
                } else {
                    j = betap[j] as usize;
                }
            } else {
                while j < pat_len && src[i] == pat[j] {
                    i += 1;
                    j += 1;
                }
                if j == pat_len {
                    if MatcherUtil::check_char_boundary( src, i - pat_len ) {
                        if prev + pat_len as isize <= ( i - pat_len ) as isize {
                            ret.push( Match { beg: i - pat_len, end: i, sub_match: Vec::new() } );
                            prev = ( i - pat_len ) as isize;
                        }
                    }
                }
                j = betap[j] as usize;
            }
            ip = i + mp - j;
        }

        ret
    }
}

impl Matcher for FjsMatcher {
    fn search( &self, src: &[u8], pat: &[u8] ) -> Vec<Match> {
        let src_len = src.len();
        let pat_len = pat.len();

        let mut betap: [isize;101] = [-1;101];
        let mut delta: [usize;256] = [pat_len;256];

        let mut i = 0;
        let mut j = betap[0];
        while i < pat_len {
            while j > -1 && pat[i] != pat[j as usize] {
                j = betap[j as usize];
            }
            i += 1;
            j += 1;
            if i < pat_len && pat[i] == pat[j as usize] {
                betap[i] = betap[j as usize];
            } else {
                betap[i] = j;
            }
        }

        for i in 0 .. pat_len {
            delta[pat[i] as usize] = pat_len - i;
        }

        let thread_num = cmp::min( src_len / self.size_per_thread + 1, self.max_threads );

        if thread_num == 1 {
            self.search_sub( src, pat, &betap, &delta, 0, src_len )
        } else {
            let ( tx, rx ) = mpsc::channel();
            let mut pool = Pool::new( thread_num as u32 );

            pool.scoped( |scoped| {
                for i in 0..thread_num {
                    let tx  = tx.clone();
                    let beg = src_len * i / thread_num;
                    let end = src_len * ( i + 1 ) / thread_num;
                    scoped.execute( move || {
                        let tmp = self.search_sub( src, pat, &betap, &delta, beg, end );
                        let _ = tx.send( ( i, tmp ) );
                    } );
                }
            } );

            let mut rets = HashMap::new();
            for _ in 0..thread_num {
                let ( i, tmp ) = rx.recv().unwrap();
                rets.insert( i, tmp );
            }

            let mut ret = Vec::new();
            for i in 0..thread_num {
                let tmp = rets.get( &i ).unwrap();
                for t in tmp {
                    ret.push( t.clone() );
                }
            }
            ret
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// RegexMatcher
// ---------------------------------------------------------------------------------------------------------------------

pub struct RegexMatcher;

impl RegexMatcher {
    pub fn new() -> Self {
        RegexMatcher
    }
}

impl Matcher for RegexMatcher {
    fn search( &self, src: &[u8], pat: &[u8] ) -> Vec<Match> {
        let pat_str = match str::from_utf8( pat ) {
            Ok ( x ) => x,
            Err( _ ) => return Vec::new(),
        };

        let src_str = match str::from_utf8( src ) {
            Ok ( x ) => x,
            Err( _ ) => return Vec::new(),
        };

        let re = match Regex::new( pat_str ) {
            Ok ( x ) => x,
            Err( _ ) => return Vec::new(),
        };

        let result = re.find_iter( src_str );

        let mut ret = Vec::new();
        for r in result {
            let ( beg, end ) = r;
            ret.push( Match{ beg: beg, end: end, sub_match: Vec::new() });
        }
        ret
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// MatcherUtil
// ---------------------------------------------------------------------------------------------------------------------

struct MatcherUtil;

impl MatcherUtil {
    fn check_char_boundary( src: &[u8], pos: usize ) -> bool {
        let mut pos_ascii = if pos == 0 { 0 } else { pos - 1 };
        while pos_ascii > 0 {
            if src[pos_ascii] <= 0x7f { break }
            pos_ascii -= 1;
        }

        let mut check_pos = pos_ascii;
        while check_pos < pos {
            let char_width = MatcherUtil::check_char_width( src, check_pos );
            check_pos += char_width;
        }

        check_pos == pos
    }

    fn check_char_width( src: &[u8], pos: usize ) -> usize {
        let src_len = src.len();
        let pos0 = pos;
        let pos1 = if pos + 1 >= src_len { src_len - 1 } else { pos + 1 };
        let pos2 = if pos + 2 >= src_len { src_len - 1 } else { pos + 2 };
        let pos3 = if pos + 3 >= src_len { src_len - 1 } else { pos + 3 };
        let pos4 = if pos + 4 >= src_len { src_len - 1 } else { pos + 4 };
        let pos5 = if pos + 5 >= src_len { src_len - 1 } else { pos + 5 };
        match ( src[pos0], src[pos1], src[pos2], src[pos3], src[pos4], src[pos5] ) {
            ( 0x00...0x7f, _          , _          , _          , _          , _           ) => ( 1 ), // ASCII
            ( 0xc2...0xdf, 0x80...0xbf, _          , _          , _          , _           ) => ( 2 ), // UTF-8
            ( 0xe0...0xef, 0x80...0xbf, 0x80...0xbf, _          , _          , _           ) => ( 3 ), // UTF-8
            ( 0xf0...0xf7, 0x80...0xbf, 0x80...0xbf, 0x80...0xbf, _          , _           ) => ( 4 ), // UTF-8
            ( 0xf8...0xfb, 0x80...0xbf, 0x80...0xbf, 0x80...0xbf, 0x80...0xbf, _           ) => ( 5 ), // UTF-8
            ( 0xfc...0xfd, 0x80...0xbf, 0x80...0xbf, 0x80...0xbf, 0x80...0xbf, 0x80...0xbf ) => ( 6 ), // UTF-8
            ( 0x8e       , 0xa1...0xdf, _          , _          , _          , _           ) => ( 2 ), // EUC-JP
            ( 0xa1...0xfe, 0xa1...0xfe, _          , _          , _          , _           ) => ( 2 ), // EUC-JP
            ( 0xa1...0xdf, _          , _          , _          , _          , _           ) => ( 1 ), // ShiftJIS
            ( 0x81...0x9f, 0x40...0x7e, _          , _          , _          , _           ) => ( 2 ), // ShiftJIS
            ( 0x81...0x9f, 0x80...0xfc, _          , _          , _          , _           ) => ( 2 ), // ShiftJIS
            ( 0xe0...0xef, 0x40...0x7e, _          , _          , _          , _           ) => ( 2 ), // ShiftJIS
            ( 0xe0...0xef, 0x80...0xfc, _          , _          , _          , _           ) => ( 2 ), // ShiftJIS
            _                                                                                => ( 1 ), // Unknown
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_matcher<T:Matcher>( m: &T ) {
        let src = "abcabcaaaaabc".to_string().into_bytes();
        let pat = "a".to_string().into_bytes();
        let ret = m.search( &src, &pat );
        assert_eq!( ret.len(), 7 );
        assert_eq!( ( 0,  1  ), ( ret[0].beg, ret[0].end ) );
        assert_eq!( ( 3,  4  ), ( ret[1].beg, ret[1].end ) );
        assert_eq!( ( 6,  7  ), ( ret[2].beg, ret[2].end ) );
        assert_eq!( ( 7,  8  ), ( ret[3].beg, ret[3].end ) );
        assert_eq!( ( 8,  9  ), ( ret[4].beg, ret[4].end ) );
        assert_eq!( ( 9,  10 ), ( ret[5].beg, ret[5].end ) );
        assert_eq!( ( 10, 11 ), ( ret[6].beg, ret[6].end ) );

        let src = "abcabcaaaaabc".to_string().into_bytes();
        let pat = "abc".to_string().into_bytes();
        let ret = m.search( &src, &pat );
        assert_eq!( ret.len(), 3 );
        assert_eq!( ( 0,  3  ), ( ret[0].beg, ret[0].end ) );
        assert_eq!( ( 3,  6  ), ( ret[1].beg, ret[1].end ) );
        assert_eq!( ( 10, 13 ), ( ret[2].beg, ret[2].end ) );

        let src = "abcabcaaaaabc".to_string().into_bytes();
        let pat = "aaa".to_string().into_bytes();
        let ret = m.search( &src, &pat );
        assert_eq!( ret.len(), 1 );
        assert_eq!( ( 6, 9 ), ( ret[0].beg, ret[0].end ) );

        let src = "abcabcaaaaabc".to_string().into_bytes();
        let pat = "abcabcaaaaabc".to_string().into_bytes();
        let ret = m.search( &src, &pat );
        assert_eq!( ret.len(), 1 );
        assert_eq!( ( 0, 13 ), ( ret[0].beg, ret[0].end ) );

        let src = "abcabcaaaaabc".to_string().into_bytes();
        let pat = "あ".to_string().into_bytes();
        let ret = m.search( &src, &pat );
        assert!( ret.is_empty() );

        let src = "abcabcあいうえおaあああaaaabc".to_string().into_bytes();
        let pat = "あ".to_string().into_bytes();
        let ret = m.search( &src, &pat );
        assert_eq!( ret.len(), 4 );
        assert_eq!( ( 6 , 9  ), ( ret[0].beg, ret[0].end ) );
        assert_eq!( ( 22, 25 ), ( ret[1].beg, ret[1].end ) );
        assert_eq!( ( 25, 28 ), ( ret[2].beg, ret[2].end ) );
        assert_eq!( ( 28, 31 ), ( ret[3].beg, ret[3].end ) );
    }

    #[test]
    fn test_brute_force_matcher() {
        let matcher = BruteForceMatcher::new();
        test_matcher( &matcher );
    }

    #[test]
    fn test_quick_search_matcher() {
        let matcher = QuickSearchMatcher::new();
        test_matcher( &matcher );
    }

    #[cfg(feature = "sse")]
    #[test]
    fn test_quick_search_matcher_sse() {
        let mut matcher = QuickSearchMatcher::new();
        matcher.use_sse = true;
        test_matcher( &matcher );
    }

    #[test]
    fn test_tbm_matcher() {
        let matcher = TbmMatcher::new();
        test_matcher( &matcher );
    }

    #[cfg(feature = "sse")]
    #[test]
    fn test_tbm_matcher_sse() {
        let mut matcher = TbmMatcher::new();
        matcher.use_sse = true;
        test_matcher( &matcher );
    }

    //#[test]
    //fn test_fjs_matcher() {
    //    let matcher = FjsMatcher::new();
    //    test_matcher( &matcher );
    //}

    #[test]
    fn test_regex_matcher() {
        let matcher = RegexMatcher::new();
        test_matcher( &matcher );
    }
}

