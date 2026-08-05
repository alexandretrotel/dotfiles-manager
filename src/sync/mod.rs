use std::path::Path;
use std::process::Command;

use chrono::Utc;

use crate::context::Dotfm;
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
pub fn run(ctx: &Dotfm, message: Option<&str>) -> Result<SyncReport> {
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
            format!("chore: sync dotfm ({stamp})")
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
