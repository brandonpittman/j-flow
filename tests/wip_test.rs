//! End-to-end tests for `jf wip`: syncing work-in-progress between
//! machines via the personal wip/<user> bookmark. No gh involved.

mod common;

use assert_cmd::Command;
use common::*;
use predicates::prelude::*;

// Test repos use user.name "Test User" → bookmark wip/test-user
const WIP_REF: &str = "refs/heads/wip/test-user";

/// Repo with one described WIP change in the stack.
fn wip_setup() -> (tempfile::TempDir, tempfile::TempDir) {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    std::fs::write(repo.path().join("wip.txt"), "in progress").unwrap();
    jj(&repo, &["describe", "-m", "WIP work"]);
    (repo, remote)
}

fn jf(repo: &tempfile::TempDir, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("jf")
        .unwrap()
        .args(args)
        .current_dir(repo.path())
        .assert()
}

#[test]
fn test_wip_status_without_branch() {
    let (repo, _remote) = wip_setup();

    jf(&repo, &["wip"])
        .success()
        .stdout(predicate::str::contains("No wip branch found"));
}

#[test]
fn test_wip_push_creates_remote_branch() {
    let (repo, remote) = wip_setup();

    jf(&repo, &["wip", "push"]).success();

    assert!(git_ref_exists(remote.path(), WIP_REF));
    let sha = git_rev_parse(remote.path(), WIP_REF);
    assert_eq!(sha, jj_commit_id(&repo, "wip/test-user"));
}

#[test]
fn test_wip_status_lists_changes_after_push() {
    let (repo, _remote) = wip_setup();
    jf(&repo, &["wip", "push"]).success();

    jf(&repo, &["wip"])
        .success()
        .stdout(predicate::str::contains("WIP work"));
}

#[test]
fn test_wip_push_refuses_overwrite_then_force() {
    let (repo, remote) = wip_setup();
    jf(&repo, &["wip", "push"]).success();
    let sha1 = git_rev_parse(remote.path(), WIP_REF);

    // More work on the (moved) working copy
    std::fs::write(repo.path().join("more.txt"), "more").unwrap();
    jj(&repo, &["describe", "-m", "More WIP"]);

    jf(&repo, &["wip", "push"])
        .success() // prints error + hint, exits 0
        .stderr(predicate::str::contains("already exists"));
    assert_eq!(
        git_rev_parse(remote.path(), WIP_REF),
        sha1,
        "refused push must not move the branch"
    );

    jf(&repo, &["wip", "push", "--force"]).success();
    assert_ne!(
        git_rev_parse(remote.path(), WIP_REF),
        sha1,
        "--force must move the branch"
    );
}

#[test]
fn test_wip_pull_round_trip_on_second_machine() {
    // Machine A pushes wip
    let (repo_a, remote) = wip_setup();
    jf(&repo_a, &["wip", "push"]).success();

    // Machine B: fresh clone, empty stack
    let repo_b = create_jj_clone(remote.path());
    create_jflow_config(repo_b.path());

    jf(&repo_b, &["wip", "pull"]).success();

    // The WIP change is now in B's stack, with the file content
    let stack = std::process::Command::new("jj")
        .args([
            "log",
            "--no-graph",
            "-T",
            "description.first_line() ++ \"\\n\"",
            "-r",
            "::@ ~ ::main@origin",
        ])
        .current_dir(repo_b.path())
        .output()
        .expect("Failed to log stack");
    let stack = String::from_utf8_lossy(&stack.stdout);
    assert!(
        stack.contains("WIP work"),
        "pulled wip change must be in the stack; stack:\n{stack}"
    );
    assert!(
        repo_b.path().join("wip.txt").exists(),
        "working copy must contain the wip file"
    );
}

#[test]
fn test_wip_clean_refuses_without_prs_then_force() {
    let (repo, remote) = wip_setup();
    jf(&repo, &["wip", "push"]).success();

    // Changes aren't in any PR — clean must refuse
    jf(&repo, &["wip", "clean"])
        .success()
        .stderr(predicate::str::contains("Cannot clean"));
    assert!(git_ref_exists(remote.path(), WIP_REF));

    // --force deletes local and remote
    jf(&repo, &["wip", "clean", "--force"]).success();
    assert!(
        !git_ref_exists(remote.path(), WIP_REF),
        "remote wip branch must be deleted"
    );
}
