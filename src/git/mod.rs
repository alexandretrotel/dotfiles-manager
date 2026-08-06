use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::context::Dfm;
use crate::error::{Error, Result};
use crate::utils::process::run_cmd;

/// Repository setup actions performed while preparing the dfm root.
#[derive(Debug, Clone, Default)]
pub struct InitReport {
    /// A new git repository was initialized (false when one already existed).
    pub initialized: bool,
    /// A default `.gitignore` was written because none existed.
    pub gitignore_created: bool,
}

/// Run a git command in the dfm repository, inheriting this process's
/// stdio (output streams directly to the terminal).
pub fn passthrough(ctx: &Dfm, args: &[String]) -> Result<()> {
    ensure_git_repo(ctx)?;
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_cmd_passthrough("git", &args_ref, Some(ctx.root()))
}

/// Run `cmd` with inherited stdio (output streams directly to the
/// terminal), returning an error on non-zero exit.
pub(crate) fn run_cmd_passthrough(cmd: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
    let mut command = Command::new(cmd);
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(d) = dir {
        command.current_dir(d);
    }

    let status = command.status()?;
    if !status.success() {
        return Err(Error::Message(format!(
            "{} exited with status {}",
            cmd, status
        )));
    }

    Ok(())
}

/// Fail unless the dfm root already is a git repository; also make sure a
/// default `.gitignore` exists.
pub fn ensure_git_repo(ctx: &Dfm) -> Result<()> {
    if !ctx.root().join(".git").exists() {
        return Err(Error::NoGitRepository {
            path: ctx.root().to_path_buf(),
        });
    }

    ensure_gitignore_exists(ctx.root())?;
    Ok(())
}

/// Initialize a git repository in the dfm root when none exists.
pub fn init_repo_if_missing(ctx: &Dfm) -> Result<InitReport> {
    let root = ctx.root();
    if root.join(".git").exists() {
        let gitignore_created = ensure_gitignore_exists(root)?;
        return Ok(InitReport {
            initialized: false,
            gitignore_created,
        });
    }

    run_cmd("git", &["init"], Some(root))?;
    run_cmd("git", &["branch", "-M", "main"], Some(root))?;
    let gitignore_created = ensure_gitignore_exists(root)?;
    Ok(InitReport {
        initialized: true,
        gitignore_created,
    })
}

/// Write a default `.gitignore` under `root` if none exists. Returns
/// whether a file was created.
fn ensure_gitignore_exists(root: &Path) -> Result<bool> {
    let gitignore_path = root.join(".gitignore");
    if gitignore_path.exists() {
        return Ok(false);
    }

    let default_gitignore = "# dfm
.active-profile

# log files
*.log

# temporary files
.DS_Store
Thumbs.db

# os generated files
*~
*.swp
*.swo
";
    fs::write(&gitignore_path, default_gitignore)?;
    Ok(true)
}
