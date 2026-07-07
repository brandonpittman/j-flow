//! End-to-end tests for `jf land`: merged-PR detection via a fake `gh`,
//! bookmark/branch cleanup, and rebasing the remaining stack.

mod common;

use assert_cmd::Command;
use common::*;
use predicates::prelude::*;

/// Push a described change as `feat`, then stack a second change on top.
/// Returns the repos; working copy is the "Second change".
fn setup_pushed_stack() -> (tempfile::TempDir, tempfile::TempDir, String) {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    std::fs::write(repo.path().join("feature.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);

    let (_shim, path, _log) = gh_shim();
    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    // jf push made the pushed commit immutable; @ is now its empty child.
    // Turn that into real second work.
    std::fs::write(repo.path().join("second.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Second change"]);

    (repo, remote, path)
}

fn has_local_bookmark(repo: &tempfile::TempDir, name: &str) -> bool {
    let output = std::process::Command::new("jj")
        .args(["bookmark", "list"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to list bookmarks");
    // Bookmark lines look like "feat: abc123 ..." or "feat (deleted)"
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|l| l.starts_with(&format!("{}:", name)) || l.starts_with(&format!("{} ", name)))
}

#[test]
fn test_land_noop_when_nothing_merged() {
    let (repo, remote, _push_path) = setup_pushed_stack();
    // Shim with NO merged PRs
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["land"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No merged PRs found"));

    // Nothing was touched
    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
    assert!(has_local_bookmark(&repo, "feat"));
}

#[test]
fn test_land_specific_bookmark_not_merged() {
    let (repo, remote, _push_path) = setup_pushed_stack();
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["land", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("not merged"));

    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
    assert!(has_local_bookmark(&repo, "feat"));
}

#[test]
fn test_land_cleans_up_merged_pr() {
    let (repo, remote, _push_path) = setup_pushed_stack();

    // GitHub squash-merges the feat PR into main
    let merged_sha = squash_merge_on_remote(remote.path(), "feat", "Add feature (#1)");

    let (_shim, path, _log) = gh_shim_with_merged(&["feat"]);
    Command::cargo_bin("jf")
        .unwrap()
        .args(["land"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    // Bookmark gone locally and on the remote
    assert!(
        !has_local_bookmark(&repo, "feat"),
        "local bookmark must be deleted"
    );
    assert!(
        !git_ref_exists(remote.path(), "refs/heads/feat"),
        "remote branch must be deleted"
    );

    // Remaining stack rebased onto the new main: "Second change" survives
    // and descends from the squash-merge commit
    let stack = std::process::Command::new("jj")
        .args([
            "log",
            "--no-graph",
            "-T",
            "description.first_line() ++ \"\\n\"",
            "-r",
            "::@ ~ ::main@origin",
        ])
        .current_dir(repo.path())
        .output()
        .expect("Failed to log stack");
    let stack = String::from_utf8_lossy(&stack.stdout);
    assert!(
        stack.contains("Second change"),
        "unmerged work must survive; stack:\n{stack}"
    );
    assert!(
        !stack.contains("Add feature"),
        "merged change must not linger as an emptied duplicate; stack:\n{stack}"
    );

    let main_at_origin = std::process::Command::new("jj")
        .args(["log", "--no-graph", "-T", "commit_id", "-r", "main@origin"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to resolve main@origin");
    assert_eq!(
        String::from_utf8_lossy(&main_at_origin.stdout).trim(),
        merged_sha,
        "local view of main@origin must be the squash-merge commit"
    );
}

#[test]
fn test_land_dry_run_reports_but_does_not_touch() {
    let (repo, remote, _push_path) = setup_pushed_stack();
    squash_merge_on_remote(remote.path(), "feat", "Add feature (#1)");

    let (_shim, path, _log) = gh_shim_with_merged(&["feat"]);
    Command::cargo_bin("jf")
        .unwrap()
        .args(["land", "--dry-run"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("feat"));

    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
    assert!(has_local_bookmark(&repo, "feat"));
}
