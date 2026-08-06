use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::context::Dfm;

/// Metadata for one profile entry in [`ProfileConfig`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileDefinition {
    pub description: Option<String>,
}

/// The on-disk list of known profiles (`profiles.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub version: String,
    pub profiles: HashMap<String, ProfileDefinition>,
}

impl Default for ProfileConfig {
    /// Empty profile list, versioned `1.0.0`.
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            profiles: HashMap::new(),
        }
    }
}

impl ProfileConfig {
    /// Read and parse the profile config at `path`.
    pub fn load(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Load `ctx`'s profile config, or an empty default if it doesn't exist
    /// or fails to parse.
    pub fn load_or_default(ctx: &Dfm) -> Self {
        Self::load(&ctx.profiles_config_path()).unwrap_or_default()
    }

    /// Write the profile config to `path` as pretty-printed JSON, sorted by
    /// profile name for a stable diff.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let sorted_profiles: BTreeMap<&String, &ProfileDefinition> = self.profiles.iter().collect();
        let sorted_config = serde_json::json!({
            "version": self.version,
            "profiles": sorted_profiles
        });

        let content = serde_json::to_string_pretty(&sorted_config)?;
        fs::write(path, content)
    }

    /// Look up a profile's definition by name.
    pub fn get_profile(&self, name: &str) -> Option<&ProfileDefinition> {
        self.profiles.get(name)
    }

    /// Whether a profile with this name exists.
    pub fn profile_exists(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    /// All profile names, sorted alphabetically.
    pub fn list_profiles(&self) -> Vec<&String> {
        let mut names: Vec<_> = self.profiles.keys().collect();
        names.sort();
        names
    }

    /// Insert (or overwrite) a profile definition.
    pub fn create_profile(&mut self, name: &str, description: Option<String>) {
        self.profiles
            .insert(name.to_string(), ProfileDefinition { description });
    }

    /// Remove a profile definition. Returns whether it existed.
    pub fn delete_profile(&mut self, name: &str) -> bool {
        self.profiles.remove(name).is_some()
    }

    /// Write an empty default config if none exists. Returns whether a file
    /// was created.
    pub fn save_default_if_missing(ctx: &Dfm) -> io::Result<bool> {
        let path = ctx.profiles_config_path();
        if path.exists() {
            return Ok(false);
        }

        ProfileConfig::default().save(&path)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty_and_versioned() {
        let config = ProfileConfig::default();
        assert_eq!(config.version, "1.0.0");
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn create_profile_inserts_definition() {
        let mut config = ProfileConfig::default();
        config.create_profile("work", Some("work laptop".to_string()));

        let profile = config.get_profile("work").unwrap();
        assert_eq!(profile.description, Some("work laptop".to_string()));
    }

    #[test]
    fn create_profile_overwrites_existing() {
        let mut config = ProfileConfig::default();
        config.create_profile("work", Some("first".to_string()));
        config.create_profile("work", Some("second".to_string()));

        assert_eq!(
            config.get_profile("work").unwrap().description,
            Some("second".to_string())
        );
    }

    #[test]
    fn delete_profile_removes_existing_returns_true() {
        let mut config = ProfileConfig::default();
        config.create_profile("work", None);

        assert!(config.delete_profile("work"));
        assert!(!config.profile_exists("work"));
    }

    #[test]
    fn delete_profile_missing_returns_false() {
        let mut config = ProfileConfig::default();
        assert!(!config.delete_profile("missing"));
    }

    #[test]
    fn profile_exists_true_for_known_and_false_for_unknown() {
        let mut config = ProfileConfig::default();
        config.create_profile("work", None);

        assert!(config.profile_exists("work"));
        assert!(!config.profile_exists("personal"));
    }

    #[test]
    fn list_profiles_is_sorted_alphabetically() {
        let mut config = ProfileConfig::default();
        config.create_profile("zeta", None);
        config.create_profile("alpha", None);
        config.create_profile("mid", None);

        assert_eq!(config.list_profiles(), vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn load_missing_file_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.json");
        assert!(ProfileConfig::load(&path).is_err());
    }

    #[test]
    fn load_or_default_returns_default_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let config = ProfileConfig::load_or_default(&ctx);
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("profiles.json");

        let mut config = ProfileConfig::default();
        config.create_profile("work", Some("work laptop".to_string()));
        config.create_profile("personal", None);
        config.save(&path).unwrap();

        let loaded = ProfileConfig::load(&path).unwrap();
        assert_eq!(loaded.version, config.version);
        assert_eq!(loaded.list_profiles(), config.list_profiles());
        assert_eq!(
            loaded.get_profile("work").unwrap().description,
            Some("work laptop".to_string())
        );
    }

    #[test]
    fn save_writes_profiles_sorted_by_name_as_pretty_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");

        let mut config = ProfileConfig::default();
        config.create_profile("zeta", None);
        config.create_profile("alpha", None);
        config.save(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains('\n'), "expected pretty-printed JSON");

        let alpha_pos = content.find("\"alpha\"").unwrap();
        let zeta_pos = content.find("\"zeta\"").unwrap();
        assert!(alpha_pos < zeta_pos);
    }

    #[test]
    fn save_default_if_missing_creates_file_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());

        let created = ProfileConfig::save_default_if_missing(&ctx).unwrap();
        assert!(created);
        assert!(ctx.profiles_config_path().exists());
    }

    #[test]
    fn save_default_if_missing_leaves_existing_file_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Dfm::with_root(dir.path());
        let path = ctx.profiles_config_path();

        let mut config = ProfileConfig::default();
        config.create_profile("work", None);
        config.save(&path).unwrap();
        let original_content = fs::read_to_string(&path).unwrap();

        let created = ProfileConfig::save_default_if_missing(&ctx).unwrap();
        assert!(!created);
        assert_eq!(fs::read_to_string(&path).unwrap(), original_content);
    }
}
