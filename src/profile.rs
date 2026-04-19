use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::{ConftestEngineConfig, SemgrepEngineConfig};

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub default_ci: String,
    #[serde(default = "default_root_markers")]
    pub root_markers: Vec<String>,
    pub docs_enabled: bool,
    pub required_docs: Vec<String>,
    pub required_files: Vec<String>,
    pub forbidden_dirs: Vec<String>,
    pub includes_handoff: bool,
    #[serde(default)]
    pub workflow_paths: BTreeMap<String, String>,
    #[serde(default)]
    pub semgrep: SemgrepEngineConfig,
    #[serde(default)]
    pub conftest: ConftestEngineConfig,
}

#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub profile: Profile,
    pub source: String,
    pub root_dir: PathBuf,
}

impl ResolvedProfile {
    pub fn load(profile_name: &str, profile_path: Option<&Path>) -> Result<Self> {
        let (profile_path, source) = match profile_path {
            Some(path) => {
                let resolved = resolve_profile_path(path)?;
                let label = format!("custom:{}", resolved.display());
                (resolved, label)
            }
            None => {
                let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                let built_in = manifest_dir
                    .join("profiles")
                    .join(profile_name)
                    .join("profile.toml");
                (built_in, format!("built-in:{profile_name}"))
            }
        };

        let raw = fs::read_to_string(&profile_path)
            .with_context(|| format!("failed to read {}", profile_path.display()))?;
        let profile: Profile = toml::from_str(&raw)
            .with_context(|| format!("failed to parse {}", profile_path.display()))?;
        let root_dir = profile_path
            .parent()
            .map(Path::to_path_buf)
            .context("profile path had no parent directory")?;

        Ok(Self {
            profile,
            source,
            root_dir,
        })
    }

    pub fn shared_template_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("templates")
            .join("shared")
    }

    pub fn profile_template_dir(&self) -> PathBuf {
        self.root_dir.join("templates")
    }

    pub fn profile_assets_dir(&self) -> PathBuf {
        self.root_dir.join("assets")
    }

    pub fn template_candidates(&self, template_relative: &str) -> Vec<PathBuf> {
        vec![
            self.profile_template_dir().join(template_relative),
            Self::shared_template_dir().join(template_relative),
        ]
    }

    pub fn ci_template_candidates(&self, ci_provider: &str) -> Vec<PathBuf> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        match ci_provider {
            "github" => vec![
                self.profile_template_dir()
                    .join(".github")
                    .join("workflows")
                    .join("guardrails.yml"),
                manifest_dir
                    .join("templates")
                    .join("github")
                    .join(".github")
                    .join("workflows")
                    .join("guardrails.yml"),
            ],
            "gitlab" => vec![
                self.profile_template_dir()
                    .join(".gitlab-ci.guardrails.yml"),
                manifest_dir
                    .join("templates")
                    .join("gitlab")
                    .join(".gitlab-ci.guardrails.yml"),
            ],
            _ => Vec::new(),
        }
    }

    pub fn workflow_path_for_provider(&self, ci_provider: &str) -> Option<String> {
        if ci_provider == "none" {
            return None;
        }

        self.profile
            .workflow_paths
            .get(ci_provider)
            .cloned()
            .or_else(|| default_workflow_path(ci_provider).map(str::to_string))
    }
}

fn default_root_markers() -> Vec<String> {
    vec![String::from(".git")]
}

fn default_workflow_path(provider: &str) -> Option<&'static str> {
    match provider {
        "github" => Some(".github/workflows/guardrails.yml"),
        "gitlab" => Some(".gitlab-ci.guardrails.yml"),
        _ => None,
    }
}

fn resolve_profile_path(path: &Path) -> Result<PathBuf> {
    let resolved = if path.is_dir() {
        path.join("profile.toml")
    } else {
        path.to_path_buf()
    };

    fs::canonicalize(&resolved).with_context(|| format!("failed to resolve {}", resolved.display()))
}
