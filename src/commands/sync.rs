use crate::cli::SyncArgs;
use crate::commands::core::{Command, CommandExecutor};
use crate::commands::git::run_cmd_passthrough;
use crate::utils::display::yellow;
use crate::utils::paths::get_mntn_dir;
use crate::utils::system::run_cmd;
use chrono::Utc;
use eyre::{Result, WrapErr, bail};
use std::path::Path;
use std::process::Command as ProcessCommand;

struct SyncTask {
    message: Option<String>,
}

impl SyncTask {
    fn new(message: Option<String>) -> Self {
        Self { message }
    }

    fn commit_message(&self) -> Result<String> {
        match self
            .message
            .as_ref()
            .map(|msg| msg.trim())
            .filter(|msg| !msg.is_empty())
        {
            Some(msg) => Ok(msg.to_string()),
            None => {
                let stamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
                Ok(format!("chore: sync mntn ({stamp})"))
            }
        }
    }

    fn has_staged_changes(repo: &Path) -> Result<bool> {
        let status = ProcessCommand::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(repo)
            .status()
            .wrap_err("Checking staged changes")?;

        match status.code() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            Some(code) => bail!("git diff --cached --quiet exited with status {}", code),
            None => bail!("git diff --cached --quiet was terminated by signal"),
        }
    }
}

impl Command for SyncTask {
    fn name(&self) -> &str {
        "Sync"
    }

    fn execute(&mut self) -> eyre::Result<()> {
        let repo_dir = get_mntn_dir();
        crate::commands::git::ensure_git_repo(&repo_dir)?;

        run_cmd("git", &["add", "."], Some(&repo_dir))?;
        let staged = Self::has_staged_changes(&repo_dir)?;
        if staged {
            let message = self.commit_message()?;
            run_cmd("git", &["commit", "-m", &message], Some(&repo_dir))?;
        } else {
            println!("{}", yellow("   No changes to commit"));
        }

        run_cmd_passthrough("git", &["push"], Some(&repo_dir))?;
        Ok(())
    }
}

pub(crate) fn run(args: SyncArgs) {
    let mut task = SyncTask::new(args.message);
    CommandExecutor::run(&mut task);
}
