use std::path::PathBuf;

use crate::context::Dfm;
use crate::profiles::ProfileConfig;
use crate::registry::{
    ConfigRegistryEntry, EncryptedRegistryEntry, PackageRegistryEntry, Registry, RegistryEntryLike,
};

/// One of dfm's own on-disk JSON files, and what happened when
/// [`fix`] tried to rewrite it.
#[derive(Debug, Clone)]
pub struct FixedFile {
    /// Display label, e.g. `"Config registry"`.
    pub label: &'static str,
    /// Path that was (or would have been) rewritten.
    pub path: PathBuf,
    /// `Ok(true)` if the file existed and was rewritten, `Ok(false)` if it
    /// didn't exist yet (nothing to fix), `Err` with a message if it exists
    /// but could not be parsed or rewritten.
    pub outcome: Result<bool, String>,
}

impl FixedFile {
    /// Whether the file was actually rewritten.
    pub fn was_rewritten(&self) -> bool {
        matches!(self.outcome, Ok(true))
    }

    /// Whether rewriting failed.
    pub fn failed(&self) -> bool {
        self.outcome.is_err()
    }
}

/// Rewrite every one of dfm's own registry/config files —
/// `config.registry.json`, `package.registry.json`,
/// `encrypted.registry.json`, and `profiles.json` — as pretty-printed,
/// deterministically-sorted JSON (the same format [`Registry::save`] and
/// [`ProfileConfig::save`] already produce; `--fix` just re-triggers a
/// save). Files that don't exist yet are left alone.
///
/// This never touches user-owned config files that dfm backs up (e.g. a
/// registered `.bashrc` or an app's `settings.json`) — only dfm's own
/// bookkeeping files under the dfm root.
pub fn fix(ctx: &Dfm) -> Vec<FixedFile> {
    vec![
        fix_registry::<ConfigRegistryEntry>("Config registry", ctx.config_registry_path()),
        fix_registry::<PackageRegistryEntry>("Package registry", ctx.package_registry_path()),
        fix_registry::<EncryptedRegistryEntry>("Encrypted registry", ctx.encrypted_registry_path()),
        fix_profile_config(ctx.profiles_config_path()),
    ]
}

/// Reload and resave a `Registry<T>` file, normalizing its formatting.
fn fix_registry<T>(label: &'static str, path: PathBuf) -> FixedFile
where
    T: RegistryEntryLike + Clone + serde::Serialize + for<'a> serde::Deserialize<'a>,
    Registry<T>: Default,
{
    if !path.exists() {
        return FixedFile {
            label,
            path,
            outcome: Ok(false),
        };
    }

    let outcome = Registry::<T>::load_or_create(&path)
        .and_then(|registry| registry.save(&path))
        .map(|_| true)
        .map_err(|e| e.to_string());

    FixedFile {
        label,
        path,
        outcome,
    }
}

/// Reload and resave `profiles.json`, normalizing its formatting.
fn fix_profile_config(path: PathBuf) -> FixedFile {
    if !path.exists() {
        return FixedFile {
            label: "Profile config",
            path,
            outcome: Ok(false),
        };
    }

    let outcome = ProfileConfig::load(&path)
        .and_then(|config| config.save(&path))
        .map(|_| true)
        .map_err(|e| e.to_string());

    FixedFile {
        label: "Profile config",
        path,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Dfm;

    #[test]
    fn was_rewritten_true_only_for_ok_true() {
        let f = FixedFile {
            label: "x",
            path: PathBuf::from("/tmp/x"),
            outcome: Ok(true),
        };
        assert!(f.was_rewritten());
        assert!(!f.failed());
    }

    #[test]
    fn was_rewritten_false_for_ok_false() {
        let f = FixedFile {
            label: "x",
            path: PathBuf::from("/tmp/x"),
            outcome: Ok(false),
        };
        assert!(!f.was_rewritten());
        assert!(!f.failed());
    }

    #[test]
    fn failed_true_for_err_outcome() {
        let f = FixedFile {
            label: "x",
            path: PathBuf::from("/tmp/x"),
            outcome: Err("broken".to_string()),
        };
        assert!(!f.was_rewritten());
        assert!(f.failed());
    }

    #[test]
    fn fix_reports_ok_false_for_every_file_when_none_exist() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let results = fix(&ctx);

        assert_eq!(results.len(), 4);
        for f in &results {
            assert_eq!(f.outcome, Ok(false));
            assert!(!f.was_rewritten());
            assert!(!f.failed());
        }
    }

    #[test]
    fn fix_rewrites_unsorted_registry_as_sorted_pretty_json() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let unsorted = r#"{"version":"1.0.0","entries":{"zshrc":{"name":"Zsh","description":null,"enabled":true,"backup_path":".zshrc","original_path":"/home/user/.zshrc"},"bashrc":{"name":"Bash","description":null,"enabled":true,"backup_path":".bashrc","original_path":"/home/user/.bashrc"}}}"#;
        std::fs::write(ctx.config_registry_path(), unsorted).unwrap();

        let results = fix(&ctx);
        let config_result = results
            .iter()
            .find(|f| f.label == "Config registry")
            .unwrap();

        assert_eq!(config_result.outcome, Ok(true));
        assert!(config_result.was_rewritten());

        let content = std::fs::read_to_string(ctx.config_registry_path()).unwrap();
        assert!(content.contains('\n'));
        let bashrc_pos = content.find("\"bashrc\"").unwrap();
        let zshrc_pos = content.find("\"zshrc\"").unwrap();
        assert!(bashrc_pos < zshrc_pos);
    }

    #[test]
    fn fix_rewrites_unsorted_profile_config_as_sorted_pretty_json() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let unsorted = r#"{"version":"1.0.0","profiles":{"zulu":{"description":null},"alpha":{"description":null}}}"#;
        std::fs::write(ctx.profiles_config_path(), unsorted).unwrap();

        let results = fix(&ctx);
        let profile_result = results
            .iter()
            .find(|f| f.label == "Profile config")
            .unwrap();

        assert_eq!(profile_result.outcome, Ok(true));

        let content = std::fs::read_to_string(ctx.profiles_config_path()).unwrap();
        assert!(content.contains('\n'));
        let alpha_pos = content.find("\"alpha\"").unwrap();
        let zulu_pos = content.find("\"zulu\"").unwrap();
        assert!(alpha_pos < zulu_pos);
    }

    #[test]
    fn fix_reports_err_for_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        std::fs::write(ctx.encrypted_registry_path(), "not valid json {").unwrap();

        let results = fix(&ctx);
        let encrypted_result = results
            .iter()
            .find(|f| f.label == "Encrypted registry")
            .unwrap();

        assert!(encrypted_result.failed());
        assert!(encrypted_result.outcome.is_err());
    }
}
