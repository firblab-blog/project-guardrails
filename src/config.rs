use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    #[serde(default = "default_config_version")]
    pub version: u32,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub profile_source: String,
    #[serde(default)]
    pub profile_schema_version: u32,
    #[serde(default)]
    pub installed_by_version: String,
    pub project: ProjectConfig,
    pub docs: DocsConfig,
    pub rules: RulesConfig,
    pub ci: CiConfig,
    #[serde(default)]
    pub engines: EnginesConfig,
}

impl GuardrailsConfig {
    pub fn load_from_repo(target: &Path) -> Result<(PathBuf, Self)> {
        let repo_root = detect_repo_root(target)?;
        let config_path = repo_root.join(".guardrails/guardrails.toml");
        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        let config: GuardrailsConfig = toml::from_str(&raw)
            .with_context(|| format!("failed to parse {}", config_path.display()))?;
        Ok((repo_root, config))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub root_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsConfig {
    pub enabled: bool,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesConfig {
    #[serde(default)]
    pub required_files: Vec<String>,
    #[serde(default)]
    pub forbidden_dirs: Vec<String>,
    #[serde(default)]
    pub task_references: TaskReferenceRuleConfig,
    #[serde(default)]
    pub link_requirements: Vec<LinkRequirementConfig>,
    #[serde(default)]
    pub forbidden_patterns: Vec<ForbiddenPatternConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskReferenceRuleConfig {
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinkRequirementConfig {
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub required_docs: Vec<String>,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForbiddenPatternConfig {
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiConfig {
    #[serde(default)]
    pub provider: String,
    pub workflow_path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnginesConfig {
    #[serde(default)]
    pub semgrep: SemgrepEngineConfig,
    #[serde(default)]
    pub conftest: ConftestEngineConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemgrepEngineConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_semgrep_binary")]
    pub binary: String,
    #[serde(default)]
    pub config_paths: Vec<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl Default for SemgrepEngineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            binary: default_semgrep_binary(),
            config_paths: Vec::new(),
            extra_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConftestEngineConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_conftest_binary")]
    pub binary: String,
    #[serde(default)]
    pub policy_paths: Vec<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl Default for ConftestEngineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            binary: default_conftest_binary(),
            policy_paths: Vec::new(),
            extra_args: Vec::new(),
        }
    }
}

fn default_config_version() -> u32 {
    1
}

fn default_semgrep_binary() -> String {
    String::from("semgrep")
}

fn default_conftest_binary() -> String {
    String::from("conftest")
}

pub fn detect_repo_root(start: &Path) -> Result<PathBuf> {
    let canonical = if start.exists() {
        fs::canonicalize(start)
            .with_context(|| format!("failed to canonicalize {}", start.display()))?
    } else {
        bail!("target path does not exist: {}", start.display());
    };

    let search_start = if canonical.is_dir() {
        canonical
    } else {
        canonical
            .parent()
            .map(Path::to_path_buf)
            .context("file target had no parent directory")?
    };

    for candidate in search_start.ancestors() {
        if candidate.join(".git").exists() || candidate.join(".guardrails").exists() {
            return Ok(candidate.to_path_buf());
        }
    }

    Ok(search_start)
}

pub fn write_config(path: &Path, config: &GuardrailsConfig) -> Result<()> {
    let serialized = toml::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(path, serialized).with_context(|| format!("failed to write {}", path.display()))
}
