use crate::cli::GitArgs;
use crate::commands::core::{Command, CommandExecutor};
use crate::utils::paths::get_dotfm_dir;
use crate::utils::system::run_cmd;
use color_eyre::Help;
use color_eyre::eyre::Result;
use color_eyre::eyre::{bail, eyre};
use std::fs;
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

struct GitPassthroughTask {
    args: Vec<String>,
}

impl GitPassthroughTask {
    fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}

impl Command for GitPassthroughTask {
    fn name(&self) -> &str {
        "Git"
    }

    fn execute(&mut self) -> Result<()> {
        let args = std::mem::take(&mut self.args);
        run_git_passthrough(args)
    }
}

pub(crate) fn run(args: GitArgs) -> color_eyre::eyre::Result<()> {
    CommandExecutor::run(&mut GitPassthroughTask::new(args.args))
}

fn run_git_passthrough(args: Vec<String>) -> Result<()> {
    let dotfm_dir = get_dotfm_dir();
    ensure_git_repo(&dotfm_dir)?;
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_cmd_passthrough("git", &args_ref, Some(&dotfm_dir))?;
    Ok(())
}

pub(crate) fn run_cmd_passthrough(cmd: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
    let mut command = ProcessCommand::new(cmd);
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
        bail!("{} exited with status {}", cmd, status);
    }

    Ok(())
}

pub(crate) fn ensure_git_repo(dotfm_dir: &Path) -> Result<()> {
    if !dotfm_dir.join(".git").exists() {
        return Err(eyre!("No git repository found in ~/.dotfm"))
            .suggestion("Run 'dotfm backup' to initialize it.");
    }

    ensure_gitignore_exists(dotfm_dir)?;
    Ok(())
}

pub(crate) fn init_repo_if_missing(dotfm_dir: &Path) -> Result<()> {
    if dotfm_dir.join(".git").exists() {
        ensure_gitignore_exists(dotfm_dir)?;
        return Ok(());
    }

    println!("Initializing git repository in {}", dotfm_dir.display());
    run_cmd("git", &["init"], Some(dotfm_dir))?;
    run_cmd("git", &["branch", "-M", "main"], Some(dotfm_dir))?;
    println!("Git repository initialized");
    ensure_gitignore_exists(dotfm_dir)?;
    Ok(())
}

fn ensure_gitignore_exists(dotfm_dir: &Path) -> Result<()> {
    let gitignore_path = dotfm_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let default_gitignore = "# dotfm
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
        println!("Created default .gitignore");
    }
    Ok(())
}
