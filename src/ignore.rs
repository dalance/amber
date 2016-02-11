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

pub struct IgnoreGit {
    pat_name    : Vec<Pattern>,
    pat_name_abs: Vec<Pattern>,
    pat_dir     : Vec<Pattern>,
    pat_dir_abs : Vec<Pattern>,
    opt         : MatchOptions,
}

impl IgnoreGit {
    pub fn new( path: &PathBuf ) -> Self {
        let ( name, name_abs, dir, dir_abs ) = IgnoreGit::parse( &path );
        IgnoreGit {
            pat_name    : name    ,
            pat_name_abs: name_abs,
            pat_dir     : dir     ,
            pat_dir_abs : dir_abs ,
            opt         : MatchOptions {
                case_sensitive             : true,
                require_literal_separator  : true,
                require_literal_leading_dot: true,
            },
        }
    }

    fn parse( path: &PathBuf ) -> ( Vec<Pattern>, Vec<Pattern>, Vec<Pattern>, Vec<Pattern> ) {
        let mut name     = Vec::new();
        let mut name_abs = Vec::new();
        let mut dir      = Vec::new();
        let mut dir_abs  = Vec::new();

        let f = if let Ok( x ) = File::open( &path ) {
            x
        } else { 
            return ( name, name_abs, dir, dir_abs )
        };

        let f = BufReader::new( f );

        let path_abs = path.canonicalize().unwrap();
        let base     = path_abs.parent().unwrap().to_string_lossy();

        for line in f.lines() {
            let s = line.unwrap();
            let s = s.trim().to_string();

            if s == "" || s.starts_with( "#" ) {
                continue;
            } else if s.starts_with( "!" ) {
                // not yet implemented
            } else if !s.contains( "/" ) {
                if let Ok( x ) = Pattern::new( &s ) {
                    name.push( x )
                }
            } else if s.ends_with( "/" ) && s.find( "/" ).unwrap() < s.len() - 1 {
                let p = IgnoreGit::concat_path( &base, &s );
                if let Ok( x ) = Pattern::new( &p ) {
                    dir_abs.push( x )
                }
            } else if s.ends_with( "/" ) {
                let p = IgnoreGit::normalize( &s );
                if let Ok( x ) = Pattern::new( &p ) {
                    dir.push( x )
                }
            } else {
                let p = IgnoreGit::concat_path( &base, &s );
                if let Ok( x ) = Pattern::new( &p ) {
                    name_abs.push( x )
                }
            }
        }

        ( name, name_abs, dir, dir_abs )
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
}

impl Ignore for IgnoreGit {

    fn check_dir ( &self, path: &PathBuf ) -> bool {
        let abs  = if let Ok( x ) = path.canonicalize() {
            x
        } else {
            return true
        };

        let name = if let Some( x ) = path.file_name() {
            x.to_string_lossy()
        } else {
            return true
        };

        for p in &self.pat_dir {
            if p.matches_with( &name, &self.opt ) {
                return false
            }
        }

        for p in &self.pat_dir_abs {
            if p.matches_path_with( &abs, &self.opt ) {
                return false
            }
        }

        for p in &self.pat_name {
            if p.matches_with( &name, &self.opt ) {
                return false
            }
        }

        for p in &self.pat_name_abs {
            if p.matches_path_with( &abs, &self.opt ) {
                return false
            }
        }

        true
    }

    fn check_file( &self, path: &PathBuf ) -> bool {
        let abs  = if let Ok( x ) = path.canonicalize() {
            x
        } else {
            return true
        };

        let name = if let Some( x ) = path.file_name() {
            x.to_string_lossy()
        } else {
            return true
        };

        for p in &self.pat_name {
            if p.matches_with( &name, &self.opt ) {
                return false
            }
        }

        for p in &self.pat_name_abs {
            if p.matches_path_with( &abs, &self.opt ) {
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

