use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    diagnostics::{Diagnostic, collect_check_diagnostics},
    output::JSON_SCHEMA_VERSION,
    rule_engine,
};

pub fn run_check(target: &Path) -> Result<CheckOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;
    let report = collect_check_diagnostics(&repo_root, &config);

    if !report.is_empty() {
        return Ok(CheckOutput {
            schema_version: JSON_SCHEMA_VERSION,
            ok: false,
            repo_root: repo_root.display().to_string(),
            diagnostics: report.diagnostics().to_vec(),
        });
    }

    rule_engine::run_external_engines(&repo_root, &config)?;

    Ok(CheckOutput {
        schema_version: JSON_SCHEMA_VERSION,
        ok: true,
        repo_root: repo_root.display().to_string(),
        diagnostics: Vec::new(),
    })
}

#[derive(Debug, Serialize)]
pub struct CheckOutput {
    pub schema_version: u32,
    pub ok: bool,
    pub repo_root: String,
    pub diagnostics: Vec<Diagnostic>,
}
