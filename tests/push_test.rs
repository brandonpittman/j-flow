//! Integration tests for the `jf push` squash workflow.
//!
//! These exercise the real (non-dry-run) push path against a local bare
//! git remote, with `gh` controlled via a PATH shim (see tests/common).

mod common;

use assert_cmd::Command;
use common::*;
use predicates::prelude::*;

#[test]
fn test_push_append_creates_remote_branch() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    std::fs::write(repo.path().join("f.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "--append", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    // Branch exists and its tree matches the local change
    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
    let remote_tree = git_rev_parse(remote.path(), "refs/heads/feat^{tree}");
    assert_eq!(remote_tree, jj_tree_id(&repo, "@"));
}

#[test]
fn test_push_append_adds_commit_on_amend() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    std::fs::write(repo.path().join("f.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "--append", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();
    let head1 = git_rev_parse(remote.path(), "refs/heads/feat");

    // Amend the change: new content, same change
    std::fs::write(repo.path().join("f.txt"), "v2").unwrap();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "--append"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();
    let head2 = git_rev_parse(remote.path(), "refs/heads/feat");

    // Append semantics: new commit ON TOP of the previous head, not a force-move
    assert_ne!(head1, head2);
    let parent = git_rev_parse(remote.path(), "refs/heads/feat^");
    assert_eq!(parent, head1, "append must stack a commit on the old head");
    let remote_tree = git_rev_parse(remote.path(), "refs/heads/feat^{tree}");
    assert_eq!(remote_tree, jj_tree_id(&repo, "@"), "final tree must match local");
}

#[test]
fn test_push_append_noop_when_unchanged() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    std::fs::write(repo.path().join("f.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "--append", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();
    let head1 = git_rev_parse(remote.path(), "refs/heads/feat");

    // Push again with nothing changed — no extra commit
    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "--append"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();
    let head2 = git_rev_parse(remote.path(), "refs/heads/feat");

    assert_eq!(head1, head2, "unchanged push must not add commits");
}

#[test]
fn test_push_creates_remote_branch() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "add-feature"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    let sha = git_rev_parse(remote.path(), "refs/heads/add-feature");
    assert_eq!(sha, jj_commit_id(&repo, "@"));
}

#[test]
fn test_push_applies_bookmark_prefix() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config_with(
        repo.path(),
        r#"
[github]
push_style = "squash"

[bookmarks]
prefix = "jf/"
"#,
    );
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "add-feature"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    assert!(git_ref_exists(remote.path(), "refs/heads/jf/add-feature"));
}

#[test]
fn test_push_after_amend_moves_remote_ref() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();
    let sha1 = git_rev_parse(remote.path(), "refs/heads/feat");

    // Rewrite the commit; the bookmark follows the change
    jj(&repo, &["describe", "-m", "Amended message"]);

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();
    let sha2 = git_rev_parse(remote.path(), "refs/heads/feat");

    assert_ne!(sha1, sha2, "squash push must move the remote ref");
    assert_eq!(sha2, jj_commit_id(&repo, "@"));
}

#[test]
fn test_push_stack_creates_prs_with_correct_bases() {
    let (repo, _remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    jj(&repo, &["describe", "-m", "Feature A"]);
    jj(&repo, &["bookmark", "create", "feat-a", "-r", "@"]);
    jj(&repo, &["new", "-m", "Feature B"]);
    jj(&repo, &["bookmark", "create", "feat-b", "-r", "@"]);
    let (_shim, path, log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    let log = std::fs::read_to_string(log).unwrap();
    assert!(
        log.contains("pr create --head feat-a --base main --title Feature A"),
        "bottom PR should be based on main; gh log:\n{log}"
    );
    assert!(
        log.contains("pr create --head feat-b --base feat-a --title Feature B"),
        "stacked PR should be based on parent bookmark; gh log:\n{log}"
    );
    assert_eq!(log.matches("pr create").count(), 2);
}

#[test]
fn test_push_without_gh_skips_pr_creation() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_bin, path) = path_without_gh();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Creating pull request").not());

    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
}

#[test]
fn test_push_creates_primary_when_remote_empty() {
    // Remote exists but has no branches yet — jf push must create main first
    let (repo, remote) = create_jj_repo_with_empty_remote();
    create_jflow_config(repo.path());
    jj(&repo, &["describe", "-m", "Initial commit"]);
    jj(&repo, &["new", "-m", "Feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    assert!(git_ref_exists(remote.path(), "refs/heads/main"));
    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
}

#[test]
fn test_push_skips_empty_undescribed_changes() {
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    std::fs::write(repo.path().join("f.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);
    // Fresh empty working copy on top — jj's natural resting state
    jj(&repo, &["new"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    // The real change was pushed; the empty placeholder was ignored
    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
}

#[test]
fn test_status_append_synced_after_push() {
    let (repo, _remote) = create_jj_repo_with_remote();
    create_jflow_config_with(
        repo.path(),
        r#"
[github]
push_style = "append"
"#,
    );
    std::fs::write(repo.path().join("f.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["status"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{2713}"))
        .stdout(predicate::str::contains("diverged").not())
        .stdout(predicate::str::contains("needs push").not());
}

#[test]
fn test_status_append_needs_push_after_amend() {
    let (repo, _remote) = create_jj_repo_with_remote();
    create_jflow_config_with(
        repo.path(),
        r#"
[github]
push_style = "append"
"#,
    );
    std::fs::write(repo.path().join("f.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success();

    // Amend: local tree now differs from the remote branch head's tree
    std::fs::write(repo.path().join("f.txt"), "v2").unwrap();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["status"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("needs push"))
        .stdout(predicate::str::contains("diverged").not());
}

#[test]
fn test_push_rejects_empty_description() {
    let (repo, _remote) = create_jj_repo_with_remote();
    create_jflow_config(repo.path());
    // Non-empty change, but no description
    std::fs::write(repo.path().join("f.txt"), "x").unwrap();
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("must have descriptions"));
}

#[test]
fn test_push_append_from_config() {
    // push_style = "append" in config, no flag
    let (repo, remote) = create_jj_repo_with_remote();
    create_jflow_config_with(
        repo.path(),
        r#"
[github]
push_style = "append"
"#,
    );
    std::fs::write(repo.path().join("f.txt"), "v1").unwrap();
    jj(&repo, &["describe", "-m", "Add feature"]);
    let (_shim, path, _log) = gh_shim();

    Command::cargo_bin("jf")
        .unwrap()
        .args(["push", "-b", "feat"])
        .env("PATH", &path)
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("style: append"));

    assert!(git_ref_exists(remote.path(), "refs/heads/feat"));
}
