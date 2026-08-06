use std::path::Path;
use std::process::Command;

use chrono::Utc;

use crate::context::Dfm;
use crate::error::{Error, Result, WrapErr};
use crate::git;
use crate::utils::process::run_cmd;

/// What a sync run did.
#[derive(Debug, Clone)]
pub struct SyncReport {
    /// Commit message used, or `None` when there was nothing to commit.
    pub committed: Option<String>,
}

/// Stage everything, commit when there are changes, and push. Git's push
/// output streams directly to the terminal.
pub fn run(ctx: &Dfm, message: Option<&str>) -> Result<SyncReport> {
    git::ensure_git_repo(ctx)?;
    let repo_dir = ctx.root();

    run_cmd("git", &["add", "."], Some(repo_dir))?;

    let committed = if has_staged_changes(repo_dir)? {
        let message = commit_message(message);
        run_cmd("git", &["commit", "-m", &message], Some(repo_dir))?;
        Some(message)
    } else {
        None
    };

    git::run_cmd_passthrough("git", &["push"], Some(repo_dir))?;
    Ok(SyncReport { committed })
}

/// The given message if non-empty, else a timestamped default.
fn commit_message(message: Option<&str>) -> String {
    match message.map(str::trim).filter(|msg| !msg.is_empty()) {
        Some(msg) => msg.to_string(),
        None => {
            let stamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
            format!("chore: sync dfm ({stamp})")
        }
    }
}

/// Whether `git diff --cached` has anything to report.
fn has_staged_changes(repo: &Path) -> Result<bool> {
    let status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(repo)
        .status()
        .wrap_err("Checking staged changes")?;

    match status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        Some(code) => Err(Error::Message(format!(
            "git diff --cached --quiet exited with status {}",
            code
        ))),
        None => Err(Error::Message(
            "git diff --cached --quiet was terminated by signal".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_bare_remote() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        run_cmd("git", &["init", "--bare"], Some(dir.path())).unwrap();
        dir
    }

    /// A repo with an initial commit already pushed to a local bare
    /// "origin", so `git push` has an upstream to push to.
    fn init_repo_with_commit_and_remote() -> (tempfile::TempDir, tempfile::TempDir, Dfm) {
        let repo_dir = tempfile::tempdir().unwrap();
        let remote_dir = init_bare_remote();

        run_cmd("git", &["init"], Some(repo_dir.path())).unwrap();
        run_cmd(
            "git",
            &["config", "user.email", "test@example.com"],
            Some(repo_dir.path()),
        )
        .unwrap();
        run_cmd(
            "git",
            &["config", "user.name", "Test"],
            Some(repo_dir.path()),
        )
        .unwrap();
        run_cmd("git", &["branch", "-M", "main"], Some(repo_dir.path())).unwrap();
        run_cmd(
            "git",
            &[
                "remote",
                "add",
                "origin",
                &remote_dir.path().to_string_lossy(),
            ],
            Some(repo_dir.path()),
        )
        .unwrap();

        std::fs::write(repo_dir.path().join("README.md"), "init\n").unwrap();
        // Pre-create the `.gitignore` that `ensure_git_repo` would otherwise
        // write on the first `sync::run` call, so that call doesn't itself
        // introduce a staged change.
        std::fs::write(repo_dir.path().join(".gitignore"), "\n").unwrap();
        run_cmd("git", &["add", "."], Some(repo_dir.path())).unwrap();
        run_cmd("git", &["commit", "-m", "init"], Some(repo_dir.path())).unwrap();
        run_cmd(
            "git",
            &["push", "-u", "origin", "main"],
            Some(repo_dir.path()),
        )
        .unwrap();

        let ctx = Dfm::with_root(repo_dir.path());
        (repo_dir, remote_dir, ctx)
    }

    #[test]
    fn run_fails_when_root_is_not_a_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        assert!(run(&ctx, None).is_err());
    }

    #[test]
    fn run_commits_and_pushes_a_new_file() {
        let (repo_dir, _remote_dir, ctx) = init_repo_with_commit_and_remote();
        std::fs::write(repo_dir.path().join("new.txt"), "hello\n").unwrap();

        let report = run(&ctx, Some("add new file")).unwrap();

        assert_eq!(report.committed, Some("add new file".to_string()));
    }

    #[test]
    fn run_reports_no_commit_when_nothing_changed() {
        let (_repo_dir, _remote_dir, ctx) = init_repo_with_commit_and_remote();

        let report = run(&ctx, None).unwrap();

        assert_eq!(report.committed, None);
    }

    #[test]
    fn run_uses_default_message_when_none_given() {
        let (repo_dir, _remote_dir, ctx) = init_repo_with_commit_and_remote();
        std::fs::write(repo_dir.path().join("new.txt"), "hi\n").unwrap();

        let report = run(&ctx, None).unwrap();

        assert!(report.committed.unwrap().starts_with("chore: sync dfm ("));
    }

    #[test]
    fn run_returns_err_instead_of_panicking_when_push_has_no_remote() {
        let repo_dir = tempfile::tempdir().unwrap();
        run_cmd("git", &["init"], Some(repo_dir.path())).unwrap();
        run_cmd(
            "git",
            &["config", "user.email", "test@example.com"],
            Some(repo_dir.path()),
        )
        .unwrap();
        run_cmd(
            "git",
            &["config", "user.name", "Test"],
            Some(repo_dir.path()),
        )
        .unwrap();
        let ctx = Dfm::with_root(repo_dir.path());
        std::fs::write(repo_dir.path().join("file.txt"), "content\n").unwrap();

        let result = run(&ctx, Some("test"));

        assert!(result.is_err());
    }

    #[test]
    fn commit_message_uses_given_message_when_present() {
        assert_eq!(commit_message(Some("fix bug")), "fix bug");
    }

    #[test]
    fn commit_message_trims_surrounding_whitespace() {
        assert_eq!(commit_message(Some("  fix bug  ")), "fix bug");
    }

    #[test]
    fn commit_message_falls_back_to_timestamp_when_none() {
        assert!(commit_message(None).starts_with("chore: sync dfm ("));
    }

    #[test]
    fn commit_message_falls_back_to_timestamp_when_blank() {
        assert!(commit_message(Some("")).starts_with("chore: sync dfm ("));
        assert!(commit_message(Some("   ")).starts_with("chore: sync dfm ("));
    }

    #[test]
    fn has_staged_changes_false_when_nothing_staged() {
        let dir = tempfile::tempdir().unwrap();
        run_cmd("git", &["init"], Some(dir.path())).unwrap();

        assert!(!has_staged_changes(dir.path()).unwrap());
    }

    #[test]
    fn has_staged_changes_true_when_something_staged() {
        let dir = tempfile::tempdir().unwrap();
        run_cmd("git", &["init"], Some(dir.path())).unwrap();
        std::fs::write(dir.path().join("file.txt"), "content\n").unwrap();
        run_cmd("git", &["add", "."], Some(dir.path())).unwrap();

        assert!(has_staged_changes(dir.path()).unwrap());
    }
}
