use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    diagnostics::{Diagnostic, RepoCheckStatus, collect_doctor_diagnostics, collect_repo_statuses},
    output::JSON_SCHEMA_VERSION,
};

pub fn run_doctor(target: &Path) -> Result<DoctorOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;
    let report = collect_doctor_diagnostics(&repo_root, &config);
    let statuses = collect_repo_statuses(&repo_root, &config);

    Ok(DoctorOutput {
        schema_version: JSON_SCHEMA_VERSION,
        ok: report.is_empty(),
        repo_root: repo_root.display().to_string(),
        profile: config.profile,
        profile_source: config.profile_source,
        installed_by_version: config.installed_by_version,
        semgrep_engine: if config.engines.semgrep.enabled {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        },
        conftest_engine: if config.engines.conftest.enabled {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        },
        statuses,
        diagnostics: report.diagnostics().to_vec(),
    })
}

#[derive(Debug, Serialize)]
pub struct DoctorOutput {
    pub schema_version: u32,
    pub ok: bool,
    pub repo_root: String,
    pub profile: String,
    pub profile_source: String,
    pub installed_by_version: String,
    pub semgrep_engine: String,
    pub conftest_engine: String,
    pub statuses: Vec<RepoCheckStatus>,
    pub diagnostics: Vec<Diagnostic>,
}
