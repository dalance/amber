use glob::{MatchOptions, Pattern};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

// ---------------------------------------------------------------------------------------------------------------------
// Ignore
// ---------------------------------------------------------------------------------------------------------------------

pub trait Ignore {
    fn is_ignore(&self, path: &PathBuf, is_dir: bool) -> bool;
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
                ".hg".to_string(),
                ".git".to_string(),
                ".bzr".to_string(),
            ],
        }
    }
}

impl Ignore for IgnoreVcs {
    fn is_ignore(&self, path: &PathBuf, is_dir: bool) -> bool {
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

pub struct IgnoreGitPat {
    pat: Pattern,
    head: u8,
    tail: u8,
}

pub struct IgnoreGit {
    file_name: Vec<IgnoreGitPat>,
    file_path: Vec<IgnoreGitPat>,
    dir_name: Vec<IgnoreGitPat>,
    dir_path: Vec<IgnoreGitPat>,
    opt: MatchOptions,
}

impl IgnoreGit {
    pub fn new(path: &PathBuf) -> Self {
        let (f_name, f_path, d_name, d_path) = IgnoreGit::parse(&path);
        IgnoreGit {
            file_name: f_name,
            file_path: f_path,
            dir_name: d_name,
            dir_path: d_path,
            opt: MatchOptions {
                case_sensitive: true,
                require_literal_separator: true,
                require_literal_leading_dot: true,
            },
        }
    }

    fn parse(
        path: &PathBuf,
    ) -> (
        Vec<IgnoreGitPat>,
        Vec<IgnoreGitPat>,
        Vec<IgnoreGitPat>,
        Vec<IgnoreGitPat>,
    ) {
        let mut file_name = Vec::new();
        let mut file_path = Vec::new();
        let mut dir_name = Vec::new();
        let mut dir_path = Vec::new();

        let f = if let Ok(x) = File::open(&path) {
            x
        } else {
            return (file_name, file_path, dir_name, dir_path);
        };
        let f = BufReader::new(f);

        let base = path.parent().unwrap().to_string_lossy();

        for line in f.lines() {
            let s = line.unwrap();
            let s = s.trim().to_string();

            if s == "" || s.starts_with("#") {
                continue;
            } else if s.starts_with("!") {
                // not yet implemented
            } else if !s.contains("/") {
                if let Ok(x) = Pattern::new(&s) {
                    let (head, tail) = IgnoreGit::extract_fix_pat(&s);
                    file_name.push(IgnoreGitPat {
                        pat: x.clone(),
                        head: head.clone(),
                        tail: tail.clone(),
                    });
                    dir_name.push(IgnoreGitPat {
                        pat: x,
                        head: head.clone(),
                        tail: tail.clone(),
                    })
                }
            } else if s.ends_with("/") && s.find("/").unwrap() < s.len() - 1 {
                let p = IgnoreGit::concat_path(&base, &s);
                if let Ok(x) = Pattern::new(&p) {
                    let (head, tail) = IgnoreGit::extract_fix_pat(&p);
                    dir_path.push(IgnoreGitPat {
                        pat: x,
                        head: head.clone(),
                        tail: tail.clone(),
                    })
                }
            } else if s.ends_with("/") {
                let p = IgnoreGit::normalize(&s);
                if let Ok(x) = Pattern::new(&p) {
                    let (head, tail) = IgnoreGit::extract_fix_pat(&p);
                    dir_name.push(IgnoreGitPat {
                        pat: x,
                        head: head.clone(),
                        tail: tail.clone(),
                    })
                }
            } else {
                let p = IgnoreGit::concat_path(&base, &s);
                if let Ok(x) = Pattern::new(&p) {
                    let (head, tail) = IgnoreGit::extract_fix_pat(&p);
                    file_path.push(IgnoreGitPat {
                        pat: x.clone(),
                        head: head.clone(),
                        tail: tail.clone(),
                    });
                    dir_path.push(IgnoreGitPat {
                        pat: x,
                        head: head.clone(),
                        tail: tail.clone(),
                    })
                }
            }
        }

        (file_name, file_path, dir_name, dir_path)
    }

    fn concat_path(s0: &str, s1: &str) -> String {
        let ret = if s1.starts_with("/") {
            format!("{}{}", s0, s1)
        } else {
            format!("{}/{}", s0, s1)
        };
        IgnoreGit::normalize(&ret)
    }

    fn normalize(s: &str) -> String {
        if s.ends_with("/") {
            let mut s2 = String::from(s);
            s2.truncate(s.len() - 1);
            s2
        } else {
            String::from(s)
        }
    }

    fn extract_fix_pat(p: &str) -> (u8, u8) {
        let len = p.len();

        let mut head_check = !p.starts_with("\\");
        head_check &= !p.starts_with("*");
        head_check &= !p.starts_with("?");
        head_check &= !p.starts_with("[");

        let mut tail_check = !p.ends_with("*");
        tail_check &= !p.ends_with("?");
        tail_check &= !p.ends_with("]");

        let head = if head_check { p.as_bytes()[0] } else { 0 };

        let tail = if tail_check { p.as_bytes()[len - 1] } else { 0 };

        (head, tail)
    }

    fn is_ignore_sub(&self, path: &PathBuf, names: &Vec<IgnoreGitPat>, paths: &Vec<IgnoreGitPat>) -> bool {
        let path_str = path.to_string_lossy();
        let name_str = if let Some(x) = path.file_name() {
            x.to_string_lossy()
        } else {
            return false;
        };

        let name_ptr = name_str.as_bytes().as_ptr();
        let path_ptr = path_str.as_bytes().as_ptr();
        let name_end = (name_str.len() - 1) as isize;
        let path_end = (path_str.len() - 1) as isize;

        for p in names {
            unsafe {
                if (p.head != 0) && (*name_ptr != p.head) {
                    continue;
                }
                if (p.tail != 0) && (*name_ptr.offset(name_end) != p.tail) {
                    continue;
                }
            }
            if p.pat.matches_with(&name_str, &self.opt) {
                return true;
            }
        }

        for p in paths {
            unsafe {
                if (p.head != 0) && (*path_ptr != p.head) {
                    continue;
                }
                if (p.tail != 0) && (*path_ptr.offset(path_end) != p.tail) {
                    continue;
                }
            }
            if p.pat.matches_with(&path_str, &self.opt) {
                return true;
            }
        }

        false
    }
}

impl Ignore for IgnoreGit {
    fn is_ignore(&self, path: &PathBuf, is_dir: bool) -> bool {
        if is_dir {
            self.is_ignore_sub(path, &self.dir_name, &self.dir_path)
        } else {
            self.is_ignore_sub(path, &self.file_name, &self.file_path)
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
        let ignore = IgnoreGit::new(&PathBuf::from("./test/.gitignore"));

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
