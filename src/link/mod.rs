//! Clone an existing dotfiles repo into the dfm root, for onboarding a new
//! machine.

use std::fs;

use crate::context::Dfm;
use crate::error::{Error, Result};
use crate::git;

/// What a link run did.
#[derive(Debug, Clone)]
pub struct LinkReport {
    /// The URL that was cloned.
    pub url: String,
}

/// Clone `repo` (a URL or `owner/repo` shorthand) into the dfm root.
/// Fails if the root already exists and is not empty.
pub fn run(ctx: &Dfm, repo: &str) -> Result<LinkReport> {
    let url = resolve_clone_url(repo)?;
    let root = ctx.root();

    if let Ok(mut entries) = fs::read_dir(root)
        && entries.next().is_some()
    {
        return Err(Error::DataDirAlreadyExists {
            path: root.to_path_buf(),
        });
    }

    git::run_cmd_passthrough("git", &["clone", &url, &root.to_string_lossy()], None)?;
    git::ensure_git_repo(ctx)?;

    Ok(LinkReport { url })
}

/// Resolve `repo` to a clone URL: used as-is when it already looks like a
/// URL or SSH remote, otherwise treated as `owner/repo` shorthand for a
/// GitHub HTTPS URL.
fn resolve_clone_url(repo: &str) -> Result<String> {
    if repo.contains("://") || repo.starts_with("git@") {
        return Ok(repo.to_string());
    }

    let name = repo.strip_suffix(".git").unwrap_or(repo);
    match name.split_once('/') {
        Some((owner, repo_name)) if !owner.is_empty() && !repo_name.is_empty() => {
            Ok(format!("https://github.com/{owner}/{repo_name}.git"))
        }
        _ => Err(Error::InvalidRepoSpec(repo.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_resolves_to_https_github_url() {
        assert_eq!(
            resolve_clone_url("alexandretrotel/dotfiles").unwrap(),
            "https://github.com/alexandretrotel/dotfiles.git"
        );
    }

    #[test]
    fn shorthand_strips_existing_git_suffix() {
        assert_eq!(
            resolve_clone_url("alexandretrotel/dotfiles.git").unwrap(),
            "https://github.com/alexandretrotel/dotfiles.git"
        );
    }

    #[test]
    fn https_url_passes_through_unchanged() {
        let url = "https://github.com/alexandretrotel/dotfiles.git";
        assert_eq!(resolve_clone_url(url).unwrap(), url);
    }

    #[test]
    fn ssh_url_passes_through_unchanged() {
        let url = "git@github.com:alexandretrotel/dotfiles.git";
        assert_eq!(resolve_clone_url(url).unwrap(), url);
    }

    #[test]
    fn invalid_spec_is_rejected() {
        assert!(resolve_clone_url("not-a-valid-spec").is_err());
        assert!(resolve_clone_url("/").is_err());
    }
}
