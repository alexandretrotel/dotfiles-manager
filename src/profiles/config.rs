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
