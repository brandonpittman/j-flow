//! Shared helpers for integration tests.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// Helper to create a jj git repo in a temp directory
pub fn create_jj_repo() -> TempDir {
    let dir = tempdir().unwrap();

    // Initialize jj git repo
    std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to init jj repo");

    // Set up user for commits
    std::process::Command::new("jj")
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to set user.name");

    std::process::Command::new("jj")
        .args(["config", "set", "--repo", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to set user.email");

    dir
}

/// Helper to create a jj repo with a local bare git remote as "origin"
pub fn create_jj_repo_with_remote() -> (TempDir, TempDir) {
    let (repo_dir, remote_dir) = create_jj_repo_with_empty_remote();

    // Create an initial commit with a description and push as main.
    // Pushing the working-copy commit makes it immutable, so jj moves @
    // to a new empty child — that becomes the working commit.
    jj(&repo_dir, &["describe", "-m", "Initial commit"]);
    jj(&repo_dir, &["bookmark", "create", "main", "-r", "@"]);
    jj(&repo_dir, &["git", "push", "--bookmark", "main"]);

    (repo_dir, remote_dir)
}

/// Like `create_jj_repo_with_remote`, but nothing is ever pushed —
/// the bare remote has no branches.
pub fn create_jj_repo_with_empty_remote() -> (TempDir, TempDir) {
    // Create a bare git repo to act as "origin"
    let remote_dir = tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_dir.path())
        .output()
        .expect("Failed to init bare git repo");

    let repo_dir = create_jj_repo();

    // Add the bare repo as origin remote
    let remote_path = remote_dir.path().to_str().unwrap().to_string();
    jj(&repo_dir, &["git", "remote", "add", "origin", &remote_path]);

    (repo_dir, remote_dir)
}

/// Helper to create .jflow.toml config
pub fn create_jflow_config(dir: &Path) {
    create_jflow_config_with(
        dir,
        r#"
[github]
push_style = "squash"
"#,
    );
}

/// Create .jflow.toml with the standard `[remote]` section plus extra TOML
/// sections (e.g. `[github]` with a different push_style, or `[bookmarks]`
/// with a prefix). `extra` must not repeat the `[remote]` table.
pub fn create_jflow_config_with(dir: &Path, extra: &str) {
    let config = format!(
        r#"
[remote]
name = "origin"
primary = "main"

{extra}
"#
    );
    fs::write(dir.join(".jflow.toml"), config).unwrap();
}

/// Run a jj command in the given repo, panicking on spawn failure.
pub fn jj(repo: &TempDir, args: &[&str]) {
    let output = std::process::Command::new("jj")
        .args(args)
        .current_dir(repo.path())
        .output()
        .unwrap_or_else(|e| panic!("Failed to run jj {:?}: {}", args, e));
    assert!(
        output.status.success(),
        "jj {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Resolve a ref in a (bare) git repo to a commit hash. Panics if missing.
pub fn git_rev_parse(bare: &Path, git_ref: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", git_ref])
        .current_dir(bare)
        .output()
        .expect("Failed to run git rev-parse");
    assert!(
        output.status.success(),
        "git rev-parse {} failed: {}",
        git_ref,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Whether a ref exists in a (bare) git repo.
pub fn git_ref_exists(bare: &Path, git_ref: &str) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", git_ref])
        .current_dir(bare)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the commit id of a revision in a jj repo.
pub fn jj_commit_id(repo: &TempDir, rev: &str) -> String {
    let output = std::process::Command::new("jj")
        .args(["log", "-r", rev, "--no-graph", "-T", "commit_id"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to run jj log");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Get the git tree id of a revision in a jj repo (jj 0.42 colocates by
/// default, so the workspace root has a .git; fall back to jj's internal
/// store for non-colocated repos).
pub fn jj_tree_id(repo: &TempDir, rev: &str) -> String {
    let commit_id = jj_commit_id(repo, rev);
    let git_dir = if repo.path().join(".git").exists() {
        repo.path().join(".git")
    } else {
        repo.path().join(".jj/repo/store/git")
    };
    let output = std::process::Command::new("git")
        .args([
            "--git-dir",
            git_dir.to_str().unwrap(),
            "rev-parse",
            &format!("{}^{{tree}}", commit_id),
        ])
        .output()
        .expect("Failed to run git rev-parse for tree");
    assert!(
        output.status.success(),
        "tree lookup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Fake `gh` shim: writes an executable `gh` script into a fresh tempdir.
/// The script logs its argv to a log file and returns canned responses:
/// `--version` succeeds, `pr view` exits 1 (no PR exists), `pr create`
/// prints a fake PR URL. Returns (shim dir, PATH with shim prepended,
/// argv log path). Keep the TempDir alive for the duration of the test.
pub fn gh_shim() -> (TempDir, String, PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let log = dir.path().join("gh.log");
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> "{log}"
case "$1" in
  --version) echo "gh version 2.0.0"; exit 0 ;;
  pr)
    case "$2" in
      view) exit 1 ;;
      create) echo "https://github.com/example/repo/pull/1"; exit 0 ;;
    esac ;;
esac
exit 0
"#,
        log = log.display()
    );
    let gh = dir.path().join("gh");
    fs::write(&gh, script).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();

    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap()
    );
    (dir, path, log)
}

/// A PATH containing ONLY symlinks to the real `jj` and `git`, so `gh`
/// cannot resolve. (A failing shim is not enough: jf treats any spawnable
/// `gh` as available, ignoring its exit code.)
pub fn path_without_gh() -> (TempDir, String) {
    let dir = tempdir().unwrap();
    for tool in ["jj", "git"] {
        let real = std::env::split_paths(&std::env::var("PATH").unwrap())
            .map(|d| d.join(tool))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("{} not found on PATH", tool));
        std::os::unix::fs::symlink(real, dir.path().join(tool)).unwrap();
    }
    assert!(!dir.path().join("gh").exists());
    let path = dir.path().display().to_string();
    (dir, path)
}
