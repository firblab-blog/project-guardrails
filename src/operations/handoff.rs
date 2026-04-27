use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    git,
    output::JSON_SCHEMA_VERSION,
    state::{
        ensure_state_layout,
        handoffs::{
            HandoffRecord, HandoffSummary, find_handoff, load_all, load_template, next_handoff_id,
        },
        is_kebab_case,
        tasks::validate_task_ids_exist,
    },
};

pub fn load_handoff_template(target: &Path) -> Result<String> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    load_template(&repo_root)
}

pub fn list_handoffs(target: &Path) -> Result<HandoffListOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let handoffs = load_all(&repo_root)?;

    Ok(HandoffListOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        handoffs: handoffs.iter().map(handoff_summary).collect(),
    })
}

pub fn create_handoff(target: &Path, input: HandoffCreateInput) -> Result<HandoffGetOutput> {
    if !is_kebab_case(&input.slug) {
        bail!("handoff slugs must use kebab-case");
    }

    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    ensure_state_layout(&repo_root)?;
    if !input.task_ids.is_empty() {
        validate_task_ids_exist(&repo_root, &input.task_ids)?;
    }

    let handoffs = load_all(&repo_root)?;
    let body = if input.from_git {
        git::draft_handoff_body_from_git(&repo_root)
    } else {
        load_template(&repo_root)?
    };
    let record = HandoffRecord::new(
        next_handoff_id(&handoffs),
        &input.slug,
        input.title.as_deref(),
        input.task_ids,
        body,
    );
    record.write(&repo_root)?;

    Ok(HandoffGetOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        handoff: handoff_output(&record),
    })
}

pub fn close_handoff(target: &Path, id: u32) -> Result<HandoffGetOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let mut handoff = find_handoff(&load_all(&repo_root)?, id)?;
    handoff.close()?;
    handoff.write(&repo_root)?;

    Ok(HandoffGetOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        handoff: handoff_output(&handoff),
    })
}

fn handoff_summary(handoff: &HandoffRecord) -> HandoffSummary {
    handoff.summary()
}

fn handoff_output(handoff: &HandoffRecord) -> HandoffOutput {
    HandoffOutput {
        summary: handoff.summary(),
        body: handoff.body.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct HandoffCreateInput {
    pub slug: String,
    pub title: Option<String>,
    pub task_ids: Vec<u32>,
    pub from_git: bool,
}

#[derive(Debug, Serialize)]
pub struct HandoffListOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub handoffs: Vec<HandoffSummary>,
}

#[derive(Debug, Serialize)]
pub struct HandoffGetOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub handoff: HandoffOutput,
}

#[derive(Debug, Serialize)]
pub struct HandoffOutput {
    #[serde(flatten)]
    pub summary: HandoffSummary,
    pub body: String,
}
