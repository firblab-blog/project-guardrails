use anyhow::Result;
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    diagnostics::{Diagnostic, collect_doctor_diagnostics},
    state::{
        handoffs::{self, HandoffSummary},
        tasks::{self, TaskStatus, TaskSummary},
    },
};

pub const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct RepoProfileOutput {
    pub name: String,
    pub source: String,
    pub schema_version: u32,
    pub installed_by_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoConfigOutput {
    pub config_version: u32,
    pub docs_enabled: bool,
    pub ci_provider: String,
    pub required_docs: Vec<String>,
    pub required_files: Vec<String>,
    pub forbidden_dirs: Vec<String>,
    pub semgrep_enabled: bool,
    pub conftest_enabled: bool,
    pub task_references_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingItem {
    pub topic: &'static str,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoDoctorSummary {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmRepoSummary {
    pub repo_root: String,
    pub generated_at: String,
    pub profile: RepoProfileOutput,
    pub repo: RepoConfigOutput,
    pub required_reading: Vec<ReadingItem>,
    pub active_tasks: Vec<TaskSummary>,
    pub recent_handoffs: Vec<HandoffSummary>,
    pub latest_handoff: Option<HandoffSummary>,
    pub doctor: RepoDoctorSummary,
}

pub fn build_llm_repo_summary(
    repo_root: &std::path::Path,
    config: &GuardrailsConfig,
) -> Result<LlmRepoSummary> {
    let task_collection = tasks::load_collection(repo_root)?;
    let active_tasks = task_collection
        .tasks
        .iter()
        .filter(|task| {
            matches!(
                task.frontmatter.status,
                TaskStatus::Approved | TaskStatus::InProgress | TaskStatus::Blocked
            )
        })
        .map(|task| task.summary())
        .collect::<Vec<_>>();

    let handoff_collection = handoffs::load_collection(repo_root)?;
    let mut recent_handoffs = handoff_collection
        .handoffs
        .iter()
        .map(|handoff| handoff.summary())
        .collect::<Vec<_>>();
    recent_handoffs.sort_by(|left, right| right.updated.cmp(&left.updated));
    recent_handoffs.truncate(5);
    let latest_handoff = recent_handoffs.first().cloned();

    let doctor_report = collect_doctor_diagnostics(repo_root, config);

    Ok(LlmRepoSummary {
        repo_root: repo_root.display().to_string(),
        generated_at: crate::state::now_timestamp(),
        profile: RepoProfileOutput {
            name: config.profile.clone(),
            source: config.profile_source.clone(),
            schema_version: config.profile_schema_version,
            installed_by_version: config.installed_by_version.clone(),
        },
        repo: RepoConfigOutput {
            config_version: config.version,
            docs_enabled: config.docs.enabled,
            ci_provider: config.ci.provider.clone(),
            required_docs: config.docs.required.clone(),
            required_files: config.rules.required_files.clone(),
            forbidden_dirs: config.rules.forbidden_dirs.clone(),
            semgrep_enabled: config.engines.semgrep.enabled,
            conftest_enabled: config.engines.conftest.enabled,
            task_references_required: config.rules.task_references.required,
        },
        required_reading: build_required_reading(
            repo_root,
            config,
            &active_tasks,
            &recent_handoffs,
        ),
        active_tasks,
        recent_handoffs,
        latest_handoff,
        doctor: RepoDoctorSummary {
            ok: doctor_report.is_empty(),
            diagnostics: doctor_report.diagnostics().to_vec(),
        },
    })
}

fn build_required_reading(
    repo_root: &std::path::Path,
    config: &GuardrailsConfig,
    active_tasks: &[TaskSummary],
    recent_handoffs: &[HandoffSummary],
) -> Vec<ReadingItem> {
    let mut items = vec![
        reading_item(repo_root, "repo_intent", "AGENTS.md"),
        reading_item(
            repo_root,
            "approved_focus",
            "docs/project/implementation-tracker.md",
        ),
    ];

    if config
        .docs
        .required
        .iter()
        .any(|path| path == "docs/project/decision-log.md")
        || repo_root.join("docs/project/decision-log.md").exists()
    {
        items.push(reading_item(
            repo_root,
            "decision_log",
            "docs/project/decision-log.md",
        ));
    }

    if config
        .docs
        .required
        .iter()
        .any(|path| path == "docs/project/handoff-template.md")
        || repo_root.join("docs/project/handoff-template.md").exists()
    {
        items.push(reading_item(
            repo_root,
            "handoff_template",
            "docs/project/handoff-template.md",
        ));
    }

    items.push(reading_item(repo_root, "non_goals", "AGENTS.md"));

    items.extend(active_tasks.iter().map(|task| ReadingItem {
        topic: "active_task",
        path: task.path.clone(),
        exists: repo_root.join(&task.path).exists(),
    }));

    items.extend(recent_handoffs.iter().take(3).map(|handoff| ReadingItem {
        topic: "recent_handoff",
        path: handoff.path.clone(),
        exists: repo_root.join(&handoff.path).exists(),
    }));

    items
}

fn reading_item(repo_root: &std::path::Path, topic: &'static str, path: &str) -> ReadingItem {
    ReadingItem {
        topic,
        path: path.to_string(),
        exists: repo_root.join(path).exists(),
    }
}
