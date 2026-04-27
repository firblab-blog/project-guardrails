use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    output::{JSON_SCHEMA_VERSION, LlmRepoSummary, build_llm_repo_summary},
};

pub fn build_brief(target: &Path) -> Result<BriefOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;
    let summary = build_llm_repo_summary(&repo_root, &config)?;
    let brief = BriefDetails::from_summary(&summary);

    Ok(BriefOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        generated_at: summary.generated_at.clone(),
        summary,
        brief,
    })
}

fn recommended_commands(summary: &LlmRepoSummary) -> Vec<String> {
    let mut commands = Vec::new();
    let diagnostic_codes = summary
        .doctor
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();

    if diagnostic_codes.iter().any(|code| {
        matches!(
            *code,
            "managed_block_stale"
                | "managed_block_missing"
                | "managed_block_invalid"
                | "managed_block_unreadable"
                | "managed_block_generator_error"
                | "task_tracker_sync_missing"
                | "handoff_stale"
                | "handoff_missing_recent"
                | "required_doc_stale_age"
        )
    }) {
        commands.push("project-guardrails refresh --target . --check".to_string());
    }

    commands.push("project-guardrails tasks list --target .".to_string());

    if summary.doctor.ok {
        commands.push("project-guardrails pre-work --target .".to_string());
    } else {
        commands.push("project-guardrails check --target .".to_string());
    }

    commands
}

#[derive(Debug, Serialize)]
pub struct BriefOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub generated_at: String,
    pub summary: LlmRepoSummary,
    pub brief: BriefDetails,
}

#[derive(Debug, Serialize)]
pub struct BriefDetails {
    pub active_task_count: usize,
    pub recent_handoff_count: usize,
    pub diagnostic_count: usize,
    pub recommended_commands: Vec<String>,
}

impl BriefDetails {
    fn from_summary(summary: &LlmRepoSummary) -> Self {
        Self {
            active_task_count: summary.active_tasks.len(),
            recent_handoff_count: summary.recent_handoffs.len(),
            diagnostic_count: summary.doctor.diagnostics.len(),
            recommended_commands: recommended_commands(summary),
        }
    }
}
