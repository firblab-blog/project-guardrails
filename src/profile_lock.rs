use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{config::GuardrailsConfig, profile::ResolvedProfile};

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedPathEntry {
    pub path: String,
    #[serde(default)]
    pub stale_action: StaleAction,
}

impl ManagedPathEntry {
    pub fn review(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            stale_action: StaleAction::Review,
        }
    }

    pub fn remove(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            stale_action: StaleAction::Remove,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StaleAction {
    Remove,
    #[default]
    Review,
}

fn default_lock_version() -> u32 {
    1
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
