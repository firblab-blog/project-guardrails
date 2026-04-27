use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{
    config::{
        ConftestEngineConfig, EvidenceRequirementConfig, ForbiddenPatternConfig, GuardrailsConfig,
        LinkRequirementConfig, SemgrepEngineConfig, TaskReferenceRuleConfig,
    },
    managed_block::ManagedBlockPlacement,
};

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
    pub task_references: TaskReferenceRuleConfig,
    #[serde(default)]
    pub link_requirements: Vec<LinkRequirementConfig>,
    #[serde(default)]
    pub evidence_requirements: Vec<EvidenceRequirementConfig>,
    #[serde(default)]
    pub forbidden_patterns: Vec<ForbiddenPatternConfig>,
    #[serde(default)]
    pub starter_content: Vec<StarterContentRuleConfig>,
    #[serde(default)]
    pub managed_blocks: Vec<ManagedBlockConfig>,
    #[serde(default)]
    pub adapter_targets: Vec<AdapterTargetConfig>,
    #[serde(default)]
    pub rule_packs: ProfileRulePacksConfig,
    #[serde(default)]
    pub semgrep: SemgrepEngineConfig,
    #[serde(default)]
    pub conftest: ConftestEngineConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StarterContentRuleConfig {
    pub path: String,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default = "default_starter_threshold")]
    pub threshold: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ManagedBlockConfig {
    pub path: String,
    pub id: String,
    pub generator: String,
    #[serde(default)]
    pub placement: ManagedBlockPlacement,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AdapterTargetConfig {
    pub path: String,
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct ProfileRulePacksConfig {
    #[serde(default)]
    pub default_enabled: Vec<String>,
    #[serde(default)]
    pub available: Vec<RulePackConfig>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct RulePackConfig {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub required_docs: Vec<String>,
    #[serde(default)]
    pub required_files: Vec<String>,
    #[serde(default)]
    pub link_requirements: Vec<LinkRequirementConfig>,
    #[serde(default)]
    pub evidence_requirements: Vec<EvidenceRequirementConfig>,
    #[serde(default)]
    pub forbidden_patterns: Vec<ForbiddenPatternConfig>,
    #[serde(default)]
    pub semgrep_config_paths: Vec<String>,
    #[serde(default)]
    pub conftest_policy_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub profile: Profile,
    pub source: String,
    pub root_dir: Option<PathBuf>,
    built_in: Option<BuiltInProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltInProfile {
    Minimal,
    DocsDriven,
    Guardrails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltInProfileInfo {
    pub name: &'static str,
    pub summary: &'static str,
    pub description: &'static str,
    pub is_default: bool,
    pub is_opt_in: bool,
}

impl ResolvedProfile {
    pub fn load(profile_name: &str, profile_path: Option<&Path>) -> Result<Self> {
        let (profile, source, root_dir, built_in) = match profile_path {
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
                (profile, label, Some(root_dir), None)
            }
            None => match BuiltInProfile::from_name(profile_name) {
                Some(built_in) => (
                    toml::from_str(built_in.profile_toml()).with_context(|| {
                        format!("failed to parse built-in profile {profile_name}")
                    })?,
                    format!("built-in:{profile_name}"),
                    None,
                    Some(built_in),
                ),
                None => bail!("unknown built-in profile: {profile_name}"),
            },
        };

        Ok(Self {
            profile,
            source,
            root_dir,
            built_in,
        })
    }

    pub fn has_profile_assets(&self) -> bool {
        self.profile_assets_dir().is_some_and(|path| path.exists())
    }

    pub fn load_from_config(config: &GuardrailsConfig) -> Result<Self> {
        match config.profile_source.strip_prefix("built-in:") {
            Some(profile_name) => Self::load(profile_name, None),
            None => match config.profile_source.strip_prefix("custom:") {
                Some(path) => Self::load(&config.profile, Some(Path::new(path))),
                None => Self::load(&config.profile, None),
            },
        }
    }

    pub fn profile_template_dir(&self) -> Option<PathBuf> {
        self.root_dir.as_ref().map(|root| root.join("templates"))
    }

    pub fn profile_assets_dir(&self) -> Option<PathBuf> {
        self.root_dir.as_ref().map(|root| root.join("assets"))
    }

    pub fn template_content(&self, template_relative: &str) -> Option<&'static str> {
        if let Some(content) = self
            .built_in
            .and_then(|profile| profile.template_content(template_relative))
        {
            return Some(content);
        }

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
            ".pre-commit-config.yaml" => {
                Some(include_str!("../templates/shared/.pre-commit-config.yaml"))
            }
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
        if let Some(content) = self
            .built_in
            .and_then(|profile| profile.ci_template_content(ci_provider))
        {
            return Some(content);
        }

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

    pub fn starter_content_rule(&self, relative_path: &str) -> Option<&StarterContentRuleConfig> {
        self.profile
            .starter_content
            .iter()
            .find(|rule| rule.path == relative_path)
    }

    pub fn managed_blocks_for_path(&self, relative_path: &str) -> Vec<ManagedBlockConfig> {
        self.profile
            .managed_blocks
            .iter()
            .filter(|block| block.path == relative_path)
            .cloned()
            .collect()
    }

    pub fn apply_rule_packs(&self, config: &mut GuardrailsConfig) -> Result<()> {
        let enabled_packs = config.rules.rule_packs.enabled.clone();
        if enabled_packs.is_empty() {
            return Ok(());
        }

        for pack_name in enabled_packs {
            let pack = self
                .profile
                .rule_packs
                .available
                .iter()
                .find(|pack| pack.name == pack_name)
                .with_context(|| {
                    format!(
                        "rule pack `{pack_name}` is enabled but not declared by profile `{}`",
                        self.profile.name
                    )
                })?;
            apply_rule_pack(config, pack);
        }

        Ok(())
    }
}

fn apply_rule_pack(config: &mut GuardrailsConfig, pack: &RulePackConfig) {
    extend_unique(&mut config.docs.required, &pack.required_docs);
    extend_unique(&mut config.rules.required_files, &pack.required_files);
    extend_unique(&mut config.rules.link_requirements, &pack.link_requirements);
    extend_unique(
        &mut config.rules.evidence_requirements,
        &pack.evidence_requirements,
    );
    extend_unique(
        &mut config.rules.forbidden_patterns,
        &pack.forbidden_patterns,
    );

    if !pack.semgrep_config_paths.is_empty() {
        config.engines.semgrep.enabled = true;
        extend_unique(
            &mut config.engines.semgrep.config_paths,
            &pack.semgrep_config_paths,
        );
    }

    if !pack.conftest_policy_paths.is_empty() {
        config.engines.conftest.enabled = true;
        extend_unique(
            &mut config.engines.conftest.policy_paths,
            &pack.conftest_policy_paths,
        );
    }
}

fn extend_unique<T>(target: &mut Vec<T>, additions: &[T])
where
    T: Clone + PartialEq,
{
    for value in additions {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

impl BuiltInProfile {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "minimal" => Some(Self::Minimal),
            "docs-driven" => Some(Self::DocsDriven),
            "guardrails" => Some(Self::Guardrails),
            _ => None,
        }
    }

    fn profile_toml(self) -> &'static str {
        match self {
            Self::Minimal => include_str!("../profiles/minimal/profile.toml"),
            Self::DocsDriven => include_str!("../profiles/docs-driven/profile.toml"),
            Self::Guardrails => include_str!("../profiles/guardrails/profile.toml"),
        }
    }

    fn info(self) -> BuiltInProfileInfo {
        match self {
            Self::Minimal => BuiltInProfileInfo {
                name: "minimal",
                summary: "Neutral cross-language baseline with local config, AGENTS, tracker, handoff, and optional CI wiring.",
                description: "Smallest neutral built-in profile for teams that want a portable starting point.",
                is_default: true,
                is_opt_in: false,
            },
            Self::DocsDriven => BuiltInProfileInfo {
                name: "docs-driven",
                summary: "Neutral baseline plus a required decision log for teams that want stronger documentation discipline.",
                description: "Use this when you want the minimal baseline and a required docs/project/decision-log.md.",
                is_default: false,
                is_opt_in: false,
            },
            Self::Guardrails => BuiltInProfileInfo {
                name: "guardrails",
                summary: "Opt-in FirbLab-style doctrine profile with seeded AGENTS, tracker, decision log, handoff, and curated best-practice docs.",
                description: "Opinionated built-in profile for teams that want seeded operating doctrine without making it the default bootstrap path.",
                is_default: false,
                is_opt_in: true,
            },
        }
    }

    fn template_content(self, template_relative: &str) -> Option<&'static str> {
        match (self, template_relative) {
            (Self::Guardrails, "AGENTS.md") => {
                Some(include_str!("../profiles/guardrails/templates/AGENTS.md"))
            }
            (Self::Guardrails, "CLAUDE.md") => {
                Some(include_str!("../profiles/guardrails/templates/CLAUDE.md"))
            }
            (Self::Guardrails, ".cursor/rules/project-guardrails.mdc") => Some(include_str!(
                "../profiles/guardrails/templates/.cursor/rules/project-guardrails.mdc"
            )),
            (Self::Guardrails, "docs/project/implementation-tracker.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/project/implementation-tracker.md"
            )),
            (Self::Guardrails, "docs/project/decision-log.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/project/decision-log.md"
            )),
            (Self::Guardrails, "docs/project/handoff-template.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/project/handoff-template.md"
            )),
            (Self::Guardrails, "docs/project/implementation-invariants.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/project/implementation-invariants.md"
            )),
            (Self::Guardrails, "docs/best-practices/change-safety.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/best-practices/change-safety.md"
            )),
            (Self::Guardrails, "docs/best-practices/ci-and-enforcement.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/best-practices/ci-and-enforcement.md"
            )),
            (Self::Guardrails, "docs/best-practices/docs-and-handoffs.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/best-practices/docs-and-handoffs.md"
            )),
            (Self::Guardrails, "docs/best-practices/repo-shaping.md") => Some(include_str!(
                "../profiles/guardrails/templates/docs/best-practices/repo-shaping.md"
            )),
            _ => None,
        }
    }

    fn ci_template_content(self, _ci_provider: &str) -> Option<&'static str> {
        None
    }
}

pub fn built_in_profile_infos() -> Vec<BuiltInProfileInfo> {
    [
        BuiltInProfile::Minimal,
        BuiltInProfile::DocsDriven,
        BuiltInProfile::Guardrails,
    ]
    .into_iter()
    .map(BuiltInProfile::info)
    .collect()
}

pub fn built_in_profile_info(name: &str) -> Option<BuiltInProfileInfo> {
    BuiltInProfile::from_name(name).map(BuiltInProfile::info)
}

fn default_root_markers() -> Vec<String> {
    vec![String::from(".git")]
}

fn default_starter_threshold() -> usize {
    2
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
