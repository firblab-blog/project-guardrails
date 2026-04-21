use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
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
    pub root_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltInProfile {
    Minimal,
    DocsDriven,
}

impl ResolvedProfile {
    pub fn load(profile_name: &str, profile_path: Option<&Path>) -> Result<Self> {
        let (profile, source, root_dir) = match profile_path {
            Some(path) => {
                let resolved = resolve_profile_path(path)?;
                let label = format!("custom:{}", resolved.display());
                let raw = fs::read_to_string(&resolved)
                    .with_context(|| format!("failed to read {}", resolved.display()))?;
                let profile: Profile = toml::from_str(&raw)
                    .with_context(|| format!("failed to parse {}", resolved.display()))?;
                let root_dir = resolved
                    .parent()
                    .map(Path::to_path_buf)
                    .context("profile path had no parent directory")?;
                (profile, label, Some(root_dir))
            }
            None => match BuiltInProfile::from_name(profile_name) {
                Some(built_in) => (
                    toml::from_str(built_in.profile_toml()).with_context(|| {
                        format!("failed to parse built-in profile {profile_name}")
                    })?,
                    format!("built-in:{profile_name}"),
                    None,
                ),
                None => bail!("unknown built-in profile: {profile_name}"),
            },
        };

        Ok(Self {
            profile,
            source,
            root_dir,
        })
    }

    pub fn has_profile_assets(&self) -> bool {
        self.profile_assets_dir().is_some_and(|path| path.exists())
    }

    pub fn profile_template_dir(&self) -> Option<PathBuf> {
        self.root_dir.as_ref().map(|root| root.join("templates"))
    }

    pub fn profile_assets_dir(&self) -> Option<PathBuf> {
        self.root_dir.as_ref().map(|root| root.join("assets"))
    }

    pub fn template_content(&self, template_relative: &str) -> Option<&'static str> {
        match template_relative {
            "AGENTS.md" => Some(include_str!("../templates/shared/AGENTS.md")),
            "docs/project/implementation-tracker.md" => Some(include_str!(
                "../templates/shared/docs/project/implementation-tracker.md"
            )),
            "docs/project/handoff-template.md" => Some(include_str!(
                "../templates/shared/docs/project/handoff-template.md"
            )),
            "docs/project/decision-log.md" => Some(include_str!(
                "../templates/shared/docs/project/decision-log.md"
            )),
            "docs/project/implementation-invariants.md" => Some(include_str!(
                "../templates/shared/docs/project/implementation-invariants.md"
            )),
            _ => None,
        }
    }

    pub fn template_candidates(&self, template_relative: &str) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(profile_dir) = self.profile_template_dir() {
            candidates.push(profile_dir.join(template_relative));
        }
        candidates
    }

    pub fn ci_template_candidates(&self, ci_provider: &str) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        match ci_provider {
            "github" => {
                if let Some(profile_dir) = self.profile_template_dir() {
                    candidates.push(
                        profile_dir
                            .join(".github")
                            .join("workflows")
                            .join("guardrails.yml"),
                    );
                }
            }
            "gitlab" => {
                if let Some(profile_dir) = self.profile_template_dir() {
                    candidates.push(profile_dir.join(".gitlab-ci.guardrails.yml"));
                }
            }
            _ => {}
        }
        candidates
    }

    pub fn ci_template_content(&self, ci_provider: &str) -> Option<&'static str> {
        match ci_provider {
            "github" => Some(include_str!(
                "../templates/github/.github/workflows/guardrails.yml"
            )),
            "gitlab" => Some(include_str!(
                "../templates/gitlab/.gitlab-ci.guardrails.yml"
            )),
            _ => None,
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

impl BuiltInProfile {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "minimal" => Some(Self::Minimal),
            "docs-driven" => Some(Self::DocsDriven),
            _ => None,
        }
    }

    fn profile_toml(self) -> &'static str {
        match self {
            Self::Minimal => include_str!("../profiles/minimal/profile.toml"),
            Self::DocsDriven => include_str!("../profiles/docs-driven/profile.toml"),
        }
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
