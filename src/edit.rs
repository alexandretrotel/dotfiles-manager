//! Opens one of dfm's own registry/config files in an external editor.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::context::Dfm;
use crate::error::{Error, Result};

/// Editor used when `open`'s `editor` argument is `None` and neither
/// `$VISUAL` nor `$EDITOR` is set.
const DEFAULT_EDITOR: &str = "vi";

/// One of dfm's own on-disk JSON files that `dfm edit` can open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryTarget {
    /// `config.registry.json`.
    Config,
    /// `package.registry.json`.
    Package,
    /// `encrypted.registry.json`.
    Encrypted,
    /// `profiles.json`.
    Profiles,
}

impl RegistryTarget {
    /// Path to the file this target refers to.
    pub fn path(&self, ctx: &Dfm) -> PathBuf {
        match self {
            RegistryTarget::Config => ctx.config_registry_path(),
            RegistryTarget::Package => ctx.package_registry_path(),
            RegistryTarget::Encrypted => ctx.encrypted_registry_path(),
            RegistryTarget::Profiles => ctx.profiles_config_path(),
        }
    }
}

/// Open `target`'s file in an editor, inheriting this process's stdio so
/// interactive editors (vi, nano, emacs, ...) work normally. The file's
/// parent directory is created first, but the file itself is left for the
/// editor to create on save if it doesn't exist yet.
///
/// The editor to run is chosen from, in order: `editor` (typically the
/// CLI's `--editor` flag), `$VISUAL`, `$EDITOR`, then a `vi` fallback. It
/// may be a bare binary name (`nano`) or a full command with arguments
/// (`code --wait`); the first whitespace-separated token is the program,
/// the rest are args passed before the file path.
pub fn open(ctx: &Dfm, target: RegistryTarget, editor: Option<&str>) -> Result<()> {
    let path = target.path(ctx);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let resolved;
    let editor = match editor {
        Some(e) => e,
        None => {
            resolved = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());
            &resolved
        }
    };

    let mut parts = editor.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| Error::Message("Editor command is empty".to_string()))?;
    let editor_args: Vec<&str> = parts.collect();

    let status = Command::new(program)
        .args(&editor_args)
        .arg(&path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| Error::Message(format!("Failed to launch editor '{}': {}", editor, e)))?;

    if !status.success() {
        return Err(Error::Message(format!(
            "Editor '{}' exited with status {}",
            editor, status
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_target_paths_match_context_paths() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        assert_eq!(
            RegistryTarget::Config.path(&ctx),
            ctx.config_registry_path()
        );
        assert_eq!(
            RegistryTarget::Package.path(&ctx),
            ctx.package_registry_path()
        );
        assert_eq!(
            RegistryTarget::Encrypted.path(&ctx),
            ctx.encrypted_registry_path()
        );
        assert_eq!(
            RegistryTarget::Profiles.path(&ctx),
            ctx.profiles_config_path()
        );
    }

    #[test]
    fn open_creates_parent_directory_before_launching_editor() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        // `true` ignores its arguments and exits 0 without touching the
        // file, so this only exercises the parent-directory setup and a
        // successful editor run.
        let result = open(&ctx, RegistryTarget::Config, Some("true"));

        assert!(result.is_ok());
        assert!(ctx.config_registry_path().parent().unwrap().is_dir());
    }

    #[test]
    fn open_errors_when_editor_exits_non_zero() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let result = open(&ctx, RegistryTarget::Config, Some("false"));

        assert!(result.is_err());
    }

    #[test]
    fn open_errors_when_editor_binary_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let result = open(
            &ctx,
            RegistryTarget::Config,
            Some("this-editor-definitely-does-not-exist-xyz"),
        );

        assert!(result.is_err());
    }

    #[test]
    fn open_splits_custom_command_into_program_and_args() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        // `true` ignores its arguments and exits 0, standing in for a
        // custom multi-token editor command (e.g. `code --wait`).
        let result = open(&ctx, RegistryTarget::Config, Some("true --some-flag"));

        assert!(result.is_ok());
    }

    #[test]
    fn open_errors_on_empty_editor_string() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let result = open(&ctx, RegistryTarget::Config, Some("   "));

        assert!(matches!(result, Err(Error::Message(_))));
    }
}
