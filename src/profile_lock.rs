use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{config::GuardrailsConfig, managed_block::sha256_bytes, profile::ResolvedProfile};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLock {
    #[serde(default = "default_lock_version")]
    pub version: u32,
    pub profile: String,
    pub profile_source: String,
    pub profile_schema_version: u32,
    pub config_version: u32,
    pub installed_by_version: String,
    #[serde(default)]
    pub managed_paths: Vec<ManagedPathEntry>,
}

impl ProfileLock {
    pub fn new(
        profile: &ResolvedProfile,
        config: &GuardrailsConfig,
        managed_paths: Vec<ManagedPathEntry>,
    ) -> Self {
        Self {
            version: default_lock_version(),
            profile: profile.profile.name.clone(),
            profile_source: profile.source.clone(),
            profile_schema_version: profile.profile.schema_version,
            config_version: config.version,
            installed_by_version: config.installed_by_version.clone(),
            managed_paths,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        match toml::from_str(&raw) {
            Ok(lock) => Ok(lock),
            Err(_) => Ok(parse_legacy_lock(&raw)),
        }
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let serialized =
            toml::to_string_pretty(self).context("failed to serialize profile lock")?;
        fs::write(path, serialized).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn managed_path_strings(&self) -> Vec<String> {
        let mut paths = self
            .managed_paths
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        paths
    }

    pub fn removable_stale_paths(&self, stale_paths: &[String]) -> Vec<String> {
        let mut removable = self
            .managed_paths
            .iter()
            .filter(|entry| {
                entry.stale_action == StaleAction::Remove && stale_paths.contains(&entry.path)
            })
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        removable.sort();
        removable.dedup();
        removable
    }

    pub fn preserved_stale_paths(&self, stale_paths: &[String]) -> Vec<String> {
        let mut preserved = self
            .managed_paths
            .iter()
            .filter(|entry| {
                entry.stale_action == StaleAction::Preserve && stale_paths.contains(&entry.path)
            })
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        preserved.sort();
        preserved.dedup();
        preserved
    }

    pub fn managed_path_entry(&self, path: &str) -> Option<&ManagedPathEntry> {
        self.managed_paths.iter().find(|entry| entry.path == path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedPathEntry {
    pub path: String,
    #[serde(default)]
    pub stale_action: StaleAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub managed_blocks: Vec<ManagedBlockEntry>,
}

impl ManagedPathEntry {
    pub fn review(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            stale_action: StaleAction::Review,
            installed_sha256: None,
            managed_blocks: Vec::new(),
        }
    }

    pub fn remove(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            stale_action: StaleAction::Remove,
            installed_sha256: None,
            managed_blocks: Vec::new(),
        }
    }

    pub fn preserve(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            stale_action: StaleAction::Preserve,
            installed_sha256: None,
            managed_blocks: Vec::new(),
        }
    }

    pub fn with_installed_sha256(mut self, installed_sha256: Option<String>) -> Self {
        self.installed_sha256 = installed_sha256;
        self
    }

    pub fn with_managed_blocks(mut self, managed_blocks: Vec<ManagedBlockEntry>) -> Self {
        self.managed_blocks = managed_blocks;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedBlockEntry {
    pub id: String,
    pub generator: String,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StaleAction {
    Remove,
    Preserve,
    #[default]
    Review,
}

fn default_lock_version() -> u32 {
    2
}

fn parse_legacy_lock(raw: &str) -> ProfileLock {
    let mut profile = String::new();
    let mut profile_source = String::new();
    let mut profile_schema_version = 0;
    let mut config_version = 0;
    let mut installed_by_version = String::new();
    let mut managed_paths = Vec::new();

    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("profile=") {
            profile = value.to_string();
        } else if let Some(value) = line.strip_prefix("profile_source=") {
            profile_source = value.to_string();
        } else if let Some(value) = line.strip_prefix("profile_schema_version=") {
            profile_schema_version = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("config_version=") {
            config_version = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("installed_by_version=") {
            installed_by_version = value.to_string();
        } else if let Some(value) = line.strip_prefix("managed_path=") {
            managed_paths.push(default_managed_path_entry(value));
        }
    }

    managed_paths.sort_by(|left, right| left.path.cmp(&right.path));
    managed_paths.dedup_by(|left, right| left.path == right.path);

    ProfileLock {
        version: default_lock_version(),
        profile,
        profile_source,
        profile_schema_version,
        config_version,
        installed_by_version,
        managed_paths,
    }
}

pub fn default_managed_path_entry(path: &str) -> ManagedPathEntry {
    if is_auto_removable_path(path) {
        ManagedPathEntry::remove(path)
    } else {
        ManagedPathEntry::review(path)
    }
}

pub fn is_auto_removable_path(path: &str) -> bool {
    matches!(
        path,
        ".github/workflows/guardrails.yml" | ".gitlab-ci.guardrails.yml"
    )
}

pub fn installed_sha256_for_path(path: &Path) -> Result<Option<String>> {
    if path.is_dir() {
        return Ok(None);
    }

    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(Some(sha256_bytes(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::{ProfileLock, StaleAction};

    #[test]
    fn parse_legacy_line_lock_defaults_new_fields() {
        let lock = ProfileLock::load_from_fixture(
            "profile=minimal\nprofile_source=built-in:minimal\nprofile_schema_version=1\nconfig_version=1\ninstalled_by_version=0.1.12\nmanaged_path=AGENTS.md\n",
        );

        assert_eq!(lock.version, 2);
        assert_eq!(lock.managed_paths.len(), 1);
        assert_eq!(lock.managed_paths[0].stale_action, StaleAction::Review);
        assert!(lock.managed_paths[0].installed_sha256.is_none());
        assert!(lock.managed_paths[0].managed_blocks.is_empty());
    }

    #[test]
    fn deserialize_v1_toml_lock_defaults_new_fields() {
        let raw = r#"
version = 1
profile = "minimal"
profile_source = "built-in:minimal"
profile_schema_version = 1
config_version = 1
installed_by_version = "0.1.12"

[[managed_paths]]
path = "AGENTS.md"
stale_action = "review"
"#;

        let lock: ProfileLock = toml::from_str(raw).expect("lock");
        assert_eq!(lock.version, 1);
        assert_eq!(lock.managed_paths.len(), 1);
        assert!(lock.managed_paths[0].installed_sha256.is_none());
        assert!(lock.managed_paths[0].managed_blocks.is_empty());
    }

    impl ProfileLock {
        fn load_from_fixture(raw: &str) -> Self {
            match toml::from_str(raw) {
                Ok(lock) => lock,
                Err(_) => super::parse_legacy_lock(raw),
            }
        }
    }
}
