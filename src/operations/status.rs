use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    output::{JSON_SCHEMA_VERSION, LlmRepoSummary, build_llm_repo_summary},
};

pub fn build_status(target: &Path) -> Result<StatusOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;

    Ok(StatusOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        profile: config.profile,
        profile_source: config.profile_source,
        profile_schema_version: config.profile_schema_version,
        installed_by_version: config.installed_by_version,
        docs_enabled: config.docs.enabled,
        ci_provider: config.ci.provider,
        required_files: config.rules.required_files,
        forbidden_dirs: config.rules.forbidden_dirs,
        semgrep_enabled: config.engines.semgrep.enabled,
        conftest_enabled: config.engines.conftest.enabled,
    })
}

pub fn build_llm_status(target: &Path) -> Result<LlmStatusOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;

    Ok(LlmStatusOutput {
        schema_version: JSON_SCHEMA_VERSION,
        summary: build_llm_repo_summary(&repo_root, &config)?,
    })
}

#[derive(Debug, Serialize)]
pub struct StatusOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub profile: String,
    pub profile_source: String,
    pub profile_schema_version: u32,
    pub installed_by_version: String,
    pub docs_enabled: bool,
    pub ci_provider: String,
    pub required_files: Vec<String>,
    pub forbidden_dirs: Vec<String>,
    pub semgrep_enabled: bool,
    pub conftest_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct LlmStatusOutput {
    pub schema_version: u32,
    pub summary: LlmRepoSummary,
}
