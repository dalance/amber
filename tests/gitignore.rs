use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("nested")).unwrap();
    for (path, contents) in [
        (".gitignore", "*.log\n"),
        ("nested/.gitignore", "*.tmp\n!keep.log\n"),
        ("visible.txt", "needle\n"),
        ("nested/visible.txt", "needle\n"),
        ("nested/ignored.log", "needle\n"),
        ("nested/ignored.tmp", "needle\n"),
        ("nested/keep.log", "needle\n"),
    ] {
        fs::write(dir.path().join(path), contents).unwrap();
    }
    dir
}

fn command(binary: &str, dir: &TempDir) -> Command {
    let mut command = Command::new(binary);
    command.current_dir(dir.path()).env("TERM", "xterm").args([
        "--no-color",
        "--no-parent-ignore",
        "--no-symlink",
        "--max-threads",
        "2",
    ]);
    command
}

#[test]
fn search_inherits_parent_patterns_and_honors_child_whitelist() {
    let dir = fixture();
    let output = command(env!("CARGO_BIN_EXE_ambs"), &dir)
        .args(["needle", "."])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches("visible.txt").count(), 2);
    assert!(stdout.contains("keep.log"));
    assert!(!stdout.contains("ignored.log"));
    assert!(!stdout.contains("ignored.tmp"));
}

#[test]
fn replace_leaves_inherited_ignored_files_unchanged() {
    let dir = fixture();
    let output = command(env!("CARGO_BIN_EXE_ambr"), &dir)
        .args(["--no-interactive", "needle", "replacement", "."])
        .output()
        .unwrap();
    assert!(output.status.success());
    for path in ["visible.txt", "nested/visible.txt", "nested/keep.log"] {
        assert_eq!(fs::read(dir.path().join(path)).unwrap(), b"replacement\n");
    }
    for path in ["nested/ignored.log", "nested/ignored.tmp"] {
        assert_eq!(fs::read(dir.path().join(path)).unwrap(), b"needle\n");
    }
    assert_eq!(fs::read(dir.path().join(".gitignore")).unwrap(), b"*.log\n");
    assert_eq!(
        fs::read(dir.path().join("nested/.gitignore")).unwrap(),
        b"*.tmp\n!keep.log\n"
    );
}
