use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------------------------------------------------
// PathFinder
// ---------------------------------------------------------------------------------------------------------------------

pub trait PathFinder {
    fn find( &mut self, base: Vec<PathBuf> ) -> Vec<PathBuf>;
}

// ---------------------------------------------------------------------------------------------------------------------
// SimplePathFinder
// ---------------------------------------------------------------------------------------------------------------------

pub struct SimplePathFinder {
    pub is_recursive  : bool,
    pub follow_symlink: bool,
    pub skip_vcs      : bool,
    pub errors        : Vec<String>,
}

impl SimplePathFinder {
    pub fn new() -> Self {
        SimplePathFinder {
            is_recursive  : true,
            follow_symlink: true,
            skip_vcs      : true,
            errors        : Vec::new(),
        }
    }

    fn find_path( &mut self, base: PathBuf ) -> Vec<PathBuf> {
        self.errors = Vec::new();
        let mut ret: Vec<PathBuf> = Vec::new();

        let attr = match fs::metadata( &base ) {
            Ok ( x ) => x,
            Err( e ) => { self.errors.push( format!( "{} @ {}", e, base.to_str().unwrap() ) ); return Vec::new() },
        };

        if attr.is_file() {
            if attr.len() != 0 {
                ret.push( base );
            }
        } else {
            let reader = match fs::read_dir( &base ) {
                Ok ( x ) => x,
                Err( e ) => { self.errors.push( format!( "{} @ {}", e, base.to_str().unwrap() ) ); return Vec::new() },
            };

            for i in reader {
                match i {
                    Ok( entry ) => {
                        let file_type = match entry.file_type() {
                            Ok ( x ) => x,
                            Err( e ) => { self.errors.push( format!( "{}", e ) ); continue },
                        };
                        if file_type.is_file() {
                            let metadata = match entry.metadata() {
                                Ok ( x ) => x,
                                Err( e ) => { self.errors.push( format!( "{}", e ) ); continue },
                            };
                            if metadata.len() != 0 {
                                ret.push( entry.path() );
                            }
                        } else {
                            let find_dir     = file_type.is_dir()     & self.is_recursive;
                            let find_symlink = file_type.is_symlink() & self.is_recursive & self.follow_symlink;
                            if find_dir | find_symlink {
                                let sub_ret = self.find_path( entry.path() );
                                for r in sub_ret {
                                    ret.push( r );
                                }
                            }
                        }
                    },
                    Err( e ) => self.errors.push( format!( "{}", e ) ),
                };
            }
        }

        ret
    }
}

impl PathFinder for SimplePathFinder {
    fn find( &mut self, base: Vec<PathBuf> ) -> Vec<PathBuf> {
        self.errors = Vec::new();
        let mut ret: Vec<PathBuf> = Vec::new();

        for b in base {
            let path = self.find_path( b );
            for p in path {
                ret.push( p );
            }
        }

        ret
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
    fn test_simple_path_finder() {
        let mut finder = SimplePathFinder::new();

        let path = finder.find( vec![PathBuf::from( "./" )] );
        assert!( path.contains( &PathBuf::from( "./Cargo.toml" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/ambr.rs" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/ambs.rs" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/bin.rs" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/console.rs" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/lib.rs" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/matcher.rs" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/path_finder.rs" ) ) );
        assert!( path.contains( &PathBuf::from( "./src/util.rs" ) ) );

        let path = finder.find( vec![PathBuf::from( "aa" )] );
        assert!( path.is_empty() );

        let path = finder.find( vec![PathBuf::from( "Cargo.toml" )] );
        assert!( path.contains( &PathBuf::from( "Cargo.toml" ) ) );
    }
}

