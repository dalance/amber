use std::path::Path;

pub use ignore::gitignore::Gitignore;

// ---------------------------------------------------------------------------------------------------------------------
// Ignore
// ---------------------------------------------------------------------------------------------------------------------

pub trait Ignore {
    fn is_ignore(&self, path: &Path, is_dir: bool) -> bool;
}

// ---------------------------------------------------------------------------------------------------------------------
// IgnoreVcs
// ---------------------------------------------------------------------------------------------------------------------

pub struct IgnoreVcs {
    vcs_dirs: Vec<String>,
}

impl Default for IgnoreVcs {
    fn default() -> Self {
        Self::new()
    }
}

impl IgnoreVcs {
    pub fn new() -> Self {
        IgnoreVcs {
            vcs_dirs: vec![
                ".svn".to_string(),
                ".hg".to_string(),
                ".git".to_string(),
                ".bzr".to_string(),
            ],
        }
    }
}

impl Ignore for IgnoreVcs {
    fn is_ignore(&self, path: &Path, is_dir: bool) -> bool {
        if is_dir {
            for d in &self.vcs_dirs {
                if path.ends_with(d) {
                    return true;
                }
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// IgnoreGit
// ---------------------------------------------------------------------------------------------------------------------

impl Ignore for Gitignore {
    fn is_ignore(&self, path: &Path, is_dir: bool) -> bool {
        match self.matched(path, is_dir) {
            ignore::Match::None | ignore::Match::Whitelist(_) => false,
            ignore::Match::Ignore(_) => true,
        }
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
        let ignore = Gitignore::new(PathBuf::from("./test/.gitignore")).0;

        assert!(!ignore.is_ignore(&PathBuf::from("./test/ao"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/a.o"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/abc.o"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/a.s"), false));
        assert!(!ignore.is_ignore(&PathBuf::from("./test/abc.s"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/d0.t"), false));
        assert!(!ignore.is_ignore(&PathBuf::from("./test/d00.t"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/file"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir0/file"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir1/file"), false));
        assert!(!ignore.is_ignore(&PathBuf::from("./test/x/file"), false));
        assert!(!ignore.is_ignore(&PathBuf::from("./test/x/dir0/file"), false));
        assert!(!ignore.is_ignore(&PathBuf::from("./test/x/dir1/file"), false));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir2"), true));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir3/dir4"), true));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir5/dir6"), true));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir7"), true));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir3/dir7"), true));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir8"), true));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir9/dir10"), true));
        assert!(ignore.is_ignore(&PathBuf::from("./test/dir11/dir12"), true));
    }
}
