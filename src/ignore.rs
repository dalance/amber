use glob::{MatchOptions, Pattern};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::PathBuf;

// ---------------------------------------------------------------------------------------------------------------------
// Ignore
// ---------------------------------------------------------------------------------------------------------------------

pub trait Ignore {
    fn check_dir ( &self, path: &PathBuf ) -> bool;
    fn check_file( &self, path: &PathBuf ) -> bool;
}

// ---------------------------------------------------------------------------------------------------------------------
// IgnoreVcs
// ---------------------------------------------------------------------------------------------------------------------

pub struct IgnoreVcs {
    vcs_dirs: Vec<String>,
}

impl IgnoreVcs {
    pub fn new() -> Self {
        IgnoreVcs {
            vcs_dirs: vec![
                ".svn".to_string(),
                ".hg" .to_string(),
                ".git".to_string(),
                ".bzr".to_string(),
            ],
        }
    }
}

impl Ignore for IgnoreVcs {

    fn check_dir ( &self, path: &PathBuf ) -> bool {
        for d in &self.vcs_dirs {
            if path.ends_with( d ) {
                return false
            }
        }
        true
    }

    fn check_file( &self, _path: &PathBuf ) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// IgnoreGit
// ---------------------------------------------------------------------------------------------------------------------

pub struct IgnoreGitPat {
    pat : Pattern,
    head: String ,
    tail: String ,
}

pub struct IgnoreGit {
    file_name: Vec<IgnoreGitPat>,
    file_path: Vec<IgnoreGitPat>,
    dir_name : Vec<IgnoreGitPat>,
    dir_path : Vec<IgnoreGitPat>,
    opt      : MatchOptions,
}

impl IgnoreGit {
    pub fn new( path: &PathBuf ) -> Self {
        let ( f_name, f_path, d_name, d_path ) = IgnoreGit::parse( &path );
        IgnoreGit {
            file_name: f_name,
            file_path: f_path,
            dir_name : d_name,
            dir_path : d_path,
            opt      : MatchOptions {
                case_sensitive             : true,
                require_literal_separator  : true,
                require_literal_leading_dot: true,
            },
        }
    }

    fn parse( path: &PathBuf ) -> ( Vec<IgnoreGitPat>, Vec<IgnoreGitPat>, Vec<IgnoreGitPat>, Vec<IgnoreGitPat> ) {
        let mut file_name = Vec::new();
        let mut file_path = Vec::new();
        let mut dir_name  = Vec::new();
        let mut dir_path  = Vec::new();

        let f = if let Ok( x ) = File::open( &path ) {
            x
        } else {
            return ( file_name, file_path, dir_name, dir_path )
        };
        let f = BufReader::new( f );

        let base = path.parent().unwrap().to_string_lossy();

        for line in f.lines() {
            let s = line.unwrap();
            let s = s.trim().to_string();

            if s == "" || s.starts_with( "#" ) {
                continue;
            } else if s.starts_with( "!" ) {
                // not yet implemented
            } else if !s.contains( "/" ) {
                if let Ok( x ) = Pattern::new( &s ) {
                    let ( head, tail ) = IgnoreGit::extract_fix_pat( &s );
                    file_name.push( IgnoreGitPat{ pat: x.clone(), head: head.clone(), tail: tail.clone() } );
                    dir_name .push( IgnoreGitPat{ pat: x        , head: head.clone(), tail: tail.clone() } )
                }
            } else if s.ends_with( "/" ) && s.find( "/" ).unwrap() < s.len() - 1 {
                let p = IgnoreGit::concat_path( &base, &s );
                if let Ok( x ) = Pattern::new( &p ) {
                    let ( head, tail ) = IgnoreGit::extract_fix_pat( &p );
                    dir_path.push( IgnoreGitPat{ pat: x, head: head.clone(), tail: tail.clone() } )
                }
            } else if s.ends_with( "/" ) {
                let p = IgnoreGit::normalize( &s );
                if let Ok( x ) = Pattern::new( &p ) {
                    let ( head, tail ) = IgnoreGit::extract_fix_pat( &p );
                    dir_name.push( IgnoreGitPat{ pat: x, head: head.clone(), tail: tail.clone() } )
                }
            } else {
                let p = IgnoreGit::concat_path( &base, &s );
                if let Ok( x ) = Pattern::new( &p ) {
                    let ( head, tail ) = IgnoreGit::extract_fix_pat( &p );
                    file_path.push( IgnoreGitPat{ pat: x.clone(), head: head.clone(), tail: tail.clone() } );
                    dir_path .push( IgnoreGitPat{ pat: x        , head: head.clone(), tail: tail.clone() } )
                }
            }
        }

        ( file_name, file_path, dir_name, dir_path )
    }

    fn concat_path( s0: &str, s1: &str ) -> String {
        let ret = if s1.starts_with( "/" ) {
            format!( "{}{}" , s0, s1 )
        } else {
            format!( "{}/{}", s0, s1 )
        };
        IgnoreGit::normalize( &ret )
    }

    fn normalize( s: &str ) -> String {
        if s.ends_with( "/" ) {
            let mut s2 = String::from( s );
            s2.truncate( s.len() - 1 );
            s2
        } else {
            String::from( s )
        }
    }

    fn extract_fix_pat( p: &str ) -> ( String, String ) {
        let len = p.len();

        let head_p0 = p.find( "\\" ).unwrap_or( len );
        let head_p1 = p.find( "*"  ).unwrap_or( len );
        let head_p2 = p.find( "?"  ).unwrap_or( len );
        let head_p3 = p.find( "["  ).unwrap_or( len );
        let head_ps = [head_p0, head_p1, head_p2, head_p3];
        let head_p  = head_ps.iter().min().unwrap_or( &len );

        let tail_p0 = p.rfind( "*" ).map( |x| x + 1 ).unwrap_or( 0 );
        let tail_p1 = p.rfind( "?" ).map( |x| x + 1 ).unwrap_or( 0 );
        let tail_p2 = p.rfind( "]" ).map( |x| x + 1 ).unwrap_or( 0 );
        let tail_ps = [tail_p0, tail_p1, tail_p2];
        let tail_p  = tail_ps.iter().max().unwrap_or( &len );

        let head = if head_p == &0 {
            String::from( "" )
        } else {
            String::from( &p[0..*head_p] )
        };

        let tail = if tail_p == &len {
            String::from( "" )
        } else {
            String::from( &p[*tail_p..len] )
        };

        ( head, tail )
    }
}

impl Ignore for IgnoreGit {

    fn check_dir ( &self, path: &PathBuf ) -> bool {

        let path_str = path.to_string_lossy();
        let name_str = if let Some( x ) = path.file_name() {
            x.to_string_lossy()
        } else {
            return true
        };

        for p in &self.dir_name {
            if !name_str.starts_with( &p.head ) || !name_str.ends_with( &p.tail ) {
                continue;
            }
            if p.pat.matches_with( &name_str, &self.opt ) {
                return false
            }
        }

        for p in &self.dir_path {
            if !path_str.starts_with( &p.head ) || !path_str.ends_with( &p.tail ) {
                continue;
            }
            if p.pat.matches_with( &path_str, &self.opt ) {
                return false
            }
        }

        true
    }

    fn check_file( &self, path: &PathBuf ) -> bool {

        let path_str = path.to_string_lossy();
        let name_str = if let Some( x ) = path.file_name() {
            x.to_string_lossy()
        } else {
            return true
        };

        for p in &self.file_name {
            if !name_str.starts_with( &p.head ) || !name_str.ends_with( &p.tail ) {
                continue;
            }
            if p.pat.matches_with( &name_str, &self.opt ) {
                return false
            }
        }

        for p in &self.file_path {
            if !path_str.starts_with( &p.head ) || !path_str.ends_with( &p.tail ) {
                continue;
            }
            if p.pat.matches_with( &path_str, &self.opt ) {
                return false
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn ignore_git() {
        let ignore = IgnoreGit::new( &PathBuf::from( "./test/.gitignore" ) );

        assert!(  ignore.check_file( &PathBuf::from( "./test/ao"          ) ) );
        assert!( !ignore.check_file( &PathBuf::from( "./test/a.o"         ) ) );
        assert!( !ignore.check_file( &PathBuf::from( "./test/abc.o"       ) ) );
        assert!( !ignore.check_file( &PathBuf::from( "./test/a.s"         ) ) );
        assert!(  ignore.check_file( &PathBuf::from( "./test/abc.s"       ) ) );
        assert!( !ignore.check_file( &PathBuf::from( "./test/d0.t"        ) ) );
        assert!(  ignore.check_file( &PathBuf::from( "./test/d00.t"       ) ) );
        assert!( !ignore.check_file( &PathBuf::from( "./test/file"        ) ) );
        assert!( !ignore.check_file( &PathBuf::from( "./test/dir0/file"   ) ) );
        assert!( !ignore.check_file( &PathBuf::from( "./test/dir1/file"   ) ) );
        assert!(  ignore.check_file( &PathBuf::from( "./test/x/file"      ) ) );
        assert!(  ignore.check_file( &PathBuf::from( "./test/x/dir0/file" ) ) );
        assert!(  ignore.check_file( &PathBuf::from( "./test/x/dir1/file" ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir2"        ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir3/dir4"   ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir5/dir6"   ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir7"        ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir3/dir7"   ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir8"        ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir9/dir10"  ) ) );
        assert!( !ignore.check_dir ( &PathBuf::from( "./test/dir11/dir12" ) ) );
    }
}

