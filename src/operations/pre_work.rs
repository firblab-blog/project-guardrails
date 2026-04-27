use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    output::{JSON_SCHEMA_VERSION, LlmRepoSummary, build_llm_repo_summary},
    state::{ensure_state_layout, runs},
};

pub fn record_pre_work(target: &Path) -> Result<PreWorkOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;
    ensure_state_layout(&repo_root)?;

    let run_id = runs::generate_run_id(&repo_root);
    let summary = build_llm_repo_summary(&repo_root, &config)?;
    let run_path = runs::run_relative_path("pre-work", &run_id);
    let output = PreWorkOutput {
        schema_version: JSON_SCHEMA_VERSION,
        run_id,
        run_path,
        summary,
    };
    runs::write_json_run(&repo_root, "pre-work", &output.run_id, &output)?;

    Ok(output)
}

#[derive(Debug, Serialize)]
pub struct PreWorkOutput {
    pub schema_version: u32,
    pub run_id: String,
    pub run_path: String,
    pub summary: LlmRepoSummary,
}
