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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_git_repo_fails_when_no_git_dir_exists() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let result = ensure_git_repo(&ctx);

        assert!(matches!(result, Err(Error::NoGitRepository { .. })));
    }

    #[test]
    fn ensure_git_repo_succeeds_and_writes_gitignore_when_already_initialized() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        run_cmd("git", &["init"], Some(dir.path())).unwrap();

        ensure_git_repo(&ctx).unwrap();

        assert!(dir.path().join(".gitignore").exists());
    }

    #[test]
    fn ensure_git_repo_is_a_noop_when_called_twice() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        run_cmd("git", &["init"], Some(dir.path())).unwrap();

        ensure_git_repo(&ctx).unwrap();
        ensure_git_repo(&ctx).unwrap();

        assert!(dir.path().join(".git").exists());
    }

    #[test]
    fn init_repo_if_missing_creates_a_new_repo() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let report = init_repo_if_missing(&ctx).unwrap();

        assert!(report.initialized);
        assert!(report.gitignore_created);
        assert!(dir.path().join(".git").exists());
        assert!(dir.path().join(".gitignore").exists());
    }

    #[test]
    fn init_repo_if_missing_reports_not_initialized_when_repo_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        run_cmd("git", &["init"], Some(dir.path())).unwrap();

        let report = init_repo_if_missing(&ctx).unwrap();

        assert!(!report.initialized);
        assert!(report.gitignore_created);
    }

    #[test]
    fn init_repo_if_missing_does_not_recreate_an_existing_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        run_cmd("git", &["init"], Some(dir.path())).unwrap();
        fs::write(dir.path().join(".gitignore"), "custom\n").unwrap();

        let report = init_repo_if_missing(&ctx).unwrap();

        assert!(!report.initialized);
        assert!(!report.gitignore_created);
        assert_eq!(
            fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
            "custom\n"
        );
    }

    #[test]
    fn run_cmd_passthrough_succeeds_for_a_valid_command() {
        let result = run_cmd_passthrough("git", &["--version"], None);
        assert!(result.is_ok());
    }

    #[test]
    fn run_cmd_passthrough_errors_on_invalid_git_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        run_cmd("git", &["init"], Some(dir.path())).unwrap();

        let result =
            run_cmd_passthrough("git", &["this-is-not-a-git-subcommand"], Some(dir.path()));

        assert!(result.is_err());
    }

    #[test]
    fn run_cmd_passthrough_errors_when_dir_does_not_exist() {
        let result = run_cmd_passthrough("git", &["status"], Some(Path::new("/no/such/dir")));
        assert!(result.is_err());
    }

    #[test]
    fn passthrough_runs_git_status_in_repo() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        run_cmd("git", &["init"], Some(dir.path())).unwrap();

        let result = passthrough(&ctx, &["status".to_string()]);

        assert!(result.is_ok());
    }
}
