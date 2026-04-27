use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use crate::{
    config::GuardrailsConfig,
    diagnostics::{Diagnostic, DiagnosticReport},
    output::JSON_SCHEMA_VERSION,
    state::{self, handoffs, now_timestamp, tasks},
};

pub fn build_timeline(target: &Path) -> Result<TimelineOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    build_timeline_for_repo(&repo_root)
}

pub fn build_timeline_for_repo(repo_root: &Path) -> Result<TimelineOutput> {
    let mut diagnostics = DiagnosticReport::default();
    let mut events = Vec::new();

    let task_collection = tasks::load_collection(repo_root)?;
    diagnostics.extend(task_collection.diagnostics.diagnostics().to_vec());
    for task in task_collection.tasks {
        events.extend(task_events(&task));
    }

    let handoff_collection = handoffs::load_collection(repo_root)?;
    diagnostics.extend(handoff_collection.diagnostics.diagnostics().to_vec());
    for handoff in handoff_collection.handoffs {
        events.extend(handoff_events(&handoff));
    }

    events.extend(pre_work_run_events(repo_root, &mut diagnostics)?);
    events.sort_by(|left, right| {
        right
            .timestamp
            .cmp(&left.timestamp)
            .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
            .then_with(|| left.action.as_str().cmp(right.action.as_str()))
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.path.cmp(&right.path))
    });

    Ok(TimelineOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        generated_at: now_timestamp(),
        events,
        diagnostics: diagnostics.diagnostics().to_vec(),
    })
}

fn task_events(task: &tasks::TaskRecord) -> Vec<TimelineEvent> {
    let mut events = vec![TimelineEvent {
        timestamp: task.frontmatter.created.clone(),
        kind: TimelineKind::Task,
        action: TimelineAction::Created,
        id: task.id_string(),
        title: Some(task.frontmatter.title.clone()),
        status: Some(task.frontmatter.status.as_str().to_string()),
        path: task.path.clone(),
        task_ids: Vec::new(),
    }];

    if task.frontmatter.updated != task.frontmatter.created {
        events.push(TimelineEvent {
            timestamp: task.frontmatter.updated.clone(),
            kind: TimelineKind::Task,
            action: TimelineAction::Updated,
            id: task.id_string(),
            title: Some(task.frontmatter.title.clone()),
            status: Some(task.frontmatter.status.as_str().to_string()),
            path: task.path.clone(),
            task_ids: Vec::new(),
        });
    }

    events
}

fn handoff_events(handoff: &handoffs::HandoffRecord) -> Vec<TimelineEvent> {
    let task_ids = handoff
        .frontmatter
        .task_ids
        .iter()
        .map(|id| format!("{id:04}"))
        .collect::<Vec<_>>();
    let mut events = vec![TimelineEvent {
        timestamp: handoff.frontmatter.created.clone(),
        kind: TimelineKind::Handoff,
        action: TimelineAction::Created,
        id: handoff.id_string(),
        title: Some(handoff.frontmatter.title.clone()),
        status: Some(handoff_status_text(handoff.frontmatter.status).to_string()),
        path: handoff.path.clone(),
        task_ids: task_ids.clone(),
    }];

    if handoff.frontmatter.updated != handoff.frontmatter.created {
        events.push(TimelineEvent {
            timestamp: handoff.frontmatter.updated.clone(),
            kind: TimelineKind::Handoff,
            action: TimelineAction::Updated,
            id: handoff.id_string(),
            title: Some(handoff.frontmatter.title.clone()),
            status: Some(handoff_status_text(handoff.frontmatter.status).to_string()),
            path: handoff.path.clone(),
            task_ids,
        });
    }

    events
}

fn pre_work_run_events(
    repo_root: &Path,
    diagnostics: &mut DiagnosticReport,
) -> Result<Vec<TimelineEvent>> {
    let runs_dir = state::runs_dir(repo_root);
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut events = Vec::new();
    for entry in
        fs::read_dir(&runs_dir).with_context(|| format!("failed to read {}", runs_dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", runs_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let Some(file_stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if !file_stem.starts_with("pre-work-") {
            continue;
        }

        let relative = relative_path(repo_root, &path)?;
        match parse_pre_work_run(&path, &relative) {
            Ok(event) => events.push(event),
            Err(error) => diagnostics.push(Diagnostic::new(
                "run_parse_error",
                format!("failed to parse {}: {}", path.display(), error),
            )),
        }
    }

    Ok(events)
}

fn parse_pre_work_run(path: &Path, relative_path: &str) -> Result<TimelineEvent> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw).context("failed to parse JSON run record")?;
    let fallback_run_id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.strip_prefix("pre-work-"))
        .context("pre-work run filename does not contain a run id")?;
    let run_id = value
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or(fallback_run_id);
    let timestamp = value
        .pointer("/summary/generated_at")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| infer_timestamp_from_run_id(run_id))
        .with_context(|| format!("failed to infer timestamp for run {run_id}"))?;

    Ok(TimelineEvent {
        timestamp,
        kind: TimelineKind::PreWorkRun,
        action: TimelineAction::Recorded,
        id: run_id.to_string(),
        title: Some("Pre-work run".to_string()),
        status: None,
        path: relative_path.to_string(),
        task_ids: Vec::new(),
    })
}

fn infer_timestamp_from_run_id(run_id: &str) -> Option<String> {
    let timestamp = run_id.split('-').next()?;
    if timestamp.len() != 16
        || !timestamp[..8].bytes().all(|byte| byte.is_ascii_digit())
        || &timestamp[8..9] != "T"
        || !timestamp[9..15].bytes().all(|byte| byte.is_ascii_digit())
        || &timestamp[15..16] != "Z"
    {
        return None;
    }

    Some(format!(
        "{}-{}-{}T{}:{}:{}Z",
        &timestamp[0..4],
        &timestamp[4..6],
        &timestamp[6..8],
        &timestamp[9..11],
        &timestamp[11..13],
        &timestamp[13..15],
    ))
}

fn relative_path(repo_root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(repo_root)
        .with_context(|| format!("failed to relativize {}", path.display()))?
        .to_string_lossy()
        .replace('\\', "/"))
}

fn handoff_status_text(status: handoffs::HandoffStatus) -> &'static str {
    match status {
        handoffs::HandoffStatus::Open => "open",
        handoffs::HandoffStatus::Closed => "closed",
    }
}

#[derive(Debug, Serialize)]
pub struct TimelineOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub generated_at: String,
    pub events: Vec<TimelineEvent>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub timestamp: String,
    pub kind: TimelineKind,
    pub action: TimelineAction,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineKind {
    PreWorkRun,
    Task,
    Handoff,
}

impl TimelineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreWorkRun => "pre_work_run",
            Self::Task => "task",
            Self::Handoff => "handoff",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineAction {
    Created,
    Updated,
    Recorded,
}

impl TimelineAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Recorded => "recorded",
        }
    }
}
