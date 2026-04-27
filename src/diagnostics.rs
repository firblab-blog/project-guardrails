use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    managed_block::{parse_managed_blocks, render_declared_block, sha256_text},
    profile::ResolvedProfile,
    rule_engine,
    state::{handoffs, tasks},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
}

impl Diagnostic {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiagnosticReport {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReport {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn print_stderr(&self) {
        for diagnostic in &self.diagnostics {
            eprintln!("[{}] {}", diagnostic.code, diagnostic.message);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoCheckStatus {
    pub label: &'static str,
    pub relative_path: PathBuf,
    pub status: &'static str,
}

pub fn collect_doctor_diagnostics(repo_root: &Path, config: &GuardrailsConfig) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();

    for marker in &config.project.root_markers {
        let marker_path = repo_root.join(marker);
        if !marker_path.exists() {
            report.push(Diagnostic::new(
                "root_marker_missing",
                format!("missing root marker: {}", marker),
            ));
        }
    }

    extend_shared_diagnostics(&mut report, repo_root, config);
    report
}

pub fn collect_check_diagnostics(repo_root: &Path, config: &GuardrailsConfig) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();
    extend_shared_diagnostics(&mut report, repo_root, config);
    report
}

fn extend_shared_diagnostics(
    report: &mut DiagnosticReport,
    repo_root: &Path,
    config: &GuardrailsConfig,
) {
    let resolved_profile = ResolvedProfile::load_from_config(config).ok();
    let config_path = repo_root.join(".guardrails/guardrails.toml");
    if !config_path.exists() {
        report.push(Diagnostic::new(
            "config_missing",
            "missing config: .guardrails/guardrails.toml",
        ));
    }

    if config.docs.enabled {
        for required in &config.docs.required {
            let path = repo_root.join(required);
            if !path.exists() {
                report.push(Diagnostic::new(
                    "required_doc_missing",
                    format!("missing required doc: {}", required),
                ));
                continue;
            }

            validate_required_doc(report, resolved_profile.as_ref(), required, &path);
        }
    }

    for required in &config.rules.required_files {
        let path = repo_root.join(required);
        if !path.exists() {
            report.push(Diagnostic::new(
                "required_file_missing",
                format!("missing required file: {}", required),
            ));
            continue;
        }

        validate_required_file(report, resolved_profile.as_ref(), required, &path);
    }

    for forbidden in &config.rules.forbidden_dirs {
        let path = repo_root.join(forbidden);
        if path.exists() {
            report.push(Diagnostic::new(
                "forbidden_dir_present",
                format!("forbidden directory present: {}", forbidden),
            ));
        }
    }

    if let Some(workflow_path) = config.ci.workflow_path.as_ref() {
        let path = repo_root.join(workflow_path);
        if !path.exists() {
            report.push(Diagnostic::new(
                "ci_workflow_missing",
                format!("missing CI workflow: {}", workflow_path),
            ));
        }
    }

    if let Ok(task_diagnostics) = tasks::lint_tasks(repo_root) {
        report.extend(task_diagnostics.diagnostics().to_vec());
    }
    if let Ok(handoff_diagnostics) = handoffs::lint_handoffs(repo_root) {
        report.extend(handoff_diagnostics.diagnostics().to_vec());
    }
    report.extend(collect_freshness_diagnostics(
        repo_root,
        config,
        resolved_profile.as_ref(),
    ));

    report.extend(rule_engine::diagnose_external_engines(repo_root, config));
}

fn validate_required_doc(
    report: &mut DiagnosticReport,
    profile: Option<&ResolvedProfile>,
    relative_path: &str,
    path: &Path,
) {
    validate_text_file(report, relative_path, path, "required_doc");

    if let Some(starter) = profile.and_then(|profile| profile.starter_content_rule(relative_path)) {
        detect_starter_content(
            report,
            relative_path,
            path,
            "required_doc",
            &starter.markers,
            starter.threshold,
        );
    }
}

fn validate_required_file(
    report: &mut DiagnosticReport,
    profile: Option<&ResolvedProfile>,
    relative_path: &str,
    path: &Path,
) {
    if !matches!(relative_path, "README.md" | "AGENTS.md") {
        return;
    }

    validate_text_file(report, relative_path, path, "required_file");

    if let Some(starter) = profile.and_then(|profile| profile.starter_content_rule(relative_path)) {
        detect_starter_content(
            report,
            relative_path,
            path,
            "required_file",
            &starter.markers,
            starter.threshold,
        );
    }
}

fn validate_text_file(
    report: &mut DiagnosticReport,
    relative_path: &str,
    path: &Path,
    diagnostic_prefix: &'static str,
) {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => {
            report.push(Diagnostic::new(
                diagnostic_code(diagnostic_prefix, "unreadable"),
                format!("failed to read {} as text: {}", relative_path, error),
            ));
            return;
        }
    };

    if contents.trim().is_empty() {
        report.push(Diagnostic::new(
            diagnostic_code(diagnostic_prefix, "empty"),
            format!("{relative_path} exists but is empty"),
        ));
    }
}

fn detect_starter_content(
    report: &mut DiagnosticReport,
    relative_path: &str,
    path: &Path,
    diagnostic_prefix: &'static str,
    markers: &[String],
    threshold: usize,
) {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    let matched_markers = markers
        .iter()
        .filter(|marker| contents.contains(marker.as_str()))
        .count();

    if matched_markers >= threshold {
        report.push(Diagnostic::new(
            diagnostic_code(diagnostic_prefix, "starter_content"),
            format!(
                "{} still contains stock starter content; replace the placeholder guidance with repo-specific content",
                relative_path
            ),
        ));
    }
}

fn diagnostic_code(prefix: &'static str, suffix: &'static str) -> &'static str {
    match (prefix, suffix) {
        ("required_doc", "unreadable") => "required_doc_unreadable",
        ("required_doc", "empty") => "required_doc_empty",
        ("required_doc", "starter_content") => "required_doc_starter_content",
        ("required_file", "unreadable") => "required_file_unreadable",
        ("required_file", "empty") => "required_file_empty",
        ("required_file", "starter_content") => "required_file_starter_content",
        _ => "diagnostic_unknown",
    }
}

fn collect_freshness_diagnostics(
    repo_root: &Path,
    config: &GuardrailsConfig,
    profile: Option<&ResolvedProfile>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(collect_stale_required_doc_diagnostics(repo_root, config));
    diagnostics.extend(collect_managed_block_diagnostics(
        repo_root, config, profile,
    ));
    diagnostics.extend(collect_tracker_sync_diagnostics(repo_root));
    diagnostics.extend(collect_handoff_recency_diagnostics(repo_root));

    diagnostics
}

fn collect_stale_required_doc_diagnostics(
    repo_root: &Path,
    config: &GuardrailsConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let stale_after = Duration::from_secs(30 * 24 * 60 * 60);

    for relative_path in config.docs.required.iter().filter(|path| {
        matches!(
            path.as_str(),
            "docs/project/implementation-tracker.md" | "docs/project/decision-log.md"
        )
    }) {
        let path = repo_root.join(relative_path);
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let Ok(age) = SystemTime::now().duration_since(modified) else {
            continue;
        };

        if age >= stale_after {
            let days = age.as_secs() / (24 * 60 * 60);
            diagnostics.push(Diagnostic::new(
                "required_doc_stale_age",
                format!(
                    "{relative_path} is {days} day(s) old; refresh the repo-local guidance so the tracked context stays current"
                ),
            ));
        }
    }

    diagnostics
}

fn collect_managed_block_diagnostics(
    repo_root: &Path,
    config: &GuardrailsConfig,
    profile: Option<&ResolvedProfile>,
) -> Vec<Diagnostic> {
    let Some(profile) = profile else {
        return Vec::new();
    };

    let mut diagnostics = Vec::new();
    for block in &profile.profile.managed_blocks {
        let path = repo_root.join(&block.path);
        if !path.exists() {
            continue;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) => {
                diagnostics.push(Diagnostic::new(
                    "managed_block_unreadable",
                    format!(
                        "failed to read {} for managed-block checks: {}",
                        block.path, error
                    ),
                ));
                continue;
            }
        };
        let parsed = match parse_managed_blocks(&contents) {
            Ok(parsed) => parsed,
            Err(error) => {
                diagnostics.push(Diagnostic::new(
                    "managed_block_invalid",
                    format!("{} has invalid managed block markup: {}", block.path, error),
                ));
                continue;
            }
        };
        let Some(existing) = parsed.iter().find(|existing| existing.id == block.id) else {
            diagnostics.push(Diagnostic::new(
                "managed_block_missing",
                format!(
                    "{} is missing managed block `{}`; refresh the declared managed blocks instead of rewriting the whole file",
                    block.path, block.id
                ),
            ));
            continue;
        };
        let expected = match render_declared_block(repo_root, config, block) {
            Ok(expected) => expected,
            Err(error) => {
                diagnostics.push(Diagnostic::new(
                    "managed_block_generator_error",
                    format!(
                        "failed to render managed block `{}` for {}: {}",
                        block.id, block.path, error
                    ),
                ));
                continue;
            }
        };

        if sha256_text(&existing.content) != sha256_text(&expected.content) {
            diagnostics.push(Diagnostic::new(
                "managed_block_stale",
                format!(
                    "{} managed block `{}` is stale; refresh the declared managed blocks to resync repo-local context",
                    block.path, block.id
                ),
            ));
        }
    }

    diagnostics
}

fn collect_tracker_sync_diagnostics(repo_root: &Path) -> Vec<Diagnostic> {
    let tracker_path = repo_root.join("docs/project/implementation-tracker.md");
    let tracker = match fs::read_to_string(&tracker_path) {
        Ok(tracker) => tracker,
        Err(_) => return Vec::new(),
    };
    let tasks = match tasks::load_collection(repo_root) {
        Ok(collection) => collection.tasks,
        Err(_) => return Vec::new(),
    };

    tasks.into_iter()
        .filter(|task| {
            matches!(
                task.frontmatter.status,
                tasks::TaskStatus::Approved
                    | tasks::TaskStatus::InProgress
                    | tasks::TaskStatus::Blocked
            )
        })
        .filter_map(|task| {
            let id = task.id_string();
            let slug = format!("{id}-{}", task.frontmatter.slug);
            if tracker.contains(&id) || tracker.contains(&slug) {
                None
            } else {
                Some(Diagnostic::new(
                    "task_tracker_sync_missing",
                    format!(
                        "implementation tracker does not mention active task {} (`{}`); keep the tracker synchronized with approved work",
                        id, task.frontmatter.slug
                    ),
                ))
            }
        })
        .collect()
}

fn collect_handoff_recency_diagnostics(repo_root: &Path) -> Vec<Diagnostic> {
    let active_tasks = match tasks::load_collection(repo_root) {
        Ok(collection) => collection
            .tasks
            .into_iter()
            .filter(|task| {
                matches!(
                    task.frontmatter.status,
                    tasks::TaskStatus::Approved
                        | tasks::TaskStatus::InProgress
                        | tasks::TaskStatus::Blocked
                )
            })
            .collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    if active_tasks.is_empty() {
        return Vec::new();
    }

    let handoffs = match handoffs::load_all(repo_root) {
        Ok(handoffs) => handoffs,
        Err(_) => return Vec::new(),
    };
    let latest = handoffs
        .iter()
        .max_by(|left, right| left.frontmatter.updated.cmp(&right.frontmatter.updated));

    let Some(latest) = latest else {
        return vec![Diagnostic::new(
            "handoff_missing_recent",
            "active tasks exist but no durable handoff has been recorded under `.guardrails/state/handoffs/`",
        )];
    };

    let Ok(updated) = DateTime::parse_from_rfc3339(&latest.frontmatter.updated) else {
        return Vec::new();
    };
    let age = Utc::now().signed_duration_since(updated.with_timezone(&Utc));
    if age.num_days() >= 14 {
        return vec![Diagnostic::new(
            "handoff_stale",
            format!(
                "latest handoff {} is {} day(s) old while active tasks remain; record a fresher handoff-quality state summary",
                latest.id_string(),
                age.num_days()
            ),
        )];
    }

    Vec::new()
}

pub fn collect_repo_statuses(repo_root: &Path, config: &GuardrailsConfig) -> Vec<RepoCheckStatus> {
    let mut statuses = Vec::new();

    for marker in &config.project.root_markers {
        statuses.push(status_for(
            "root_marker",
            PathBuf::from(marker),
            repo_root.join(marker).exists(),
        ));
    }

    statuses.push(status_for(
        "config",
        PathBuf::from(".guardrails/guardrails.toml"),
        repo_root.join(".guardrails/guardrails.toml").exists(),
    ));

    if config.docs.enabled {
        for required in &config.docs.required {
            statuses.push(status_for(
                "required_doc",
                PathBuf::from(required),
                repo_root.join(required).exists(),
            ));
        }
    }

    for required in &config.rules.required_files {
        statuses.push(status_for(
            "required_file",
            PathBuf::from(required),
            repo_root.join(required).exists(),
        ));
    }

    if let Some(workflow_path) = config.ci.workflow_path.as_ref() {
        statuses.push(status_for(
            "ci_workflow",
            PathBuf::from(workflow_path),
            repo_root.join(workflow_path).exists(),
        ));
    }

    statuses
}

fn status_for(label: &'static str, relative_path: PathBuf, exists: bool) -> RepoCheckStatus {
    RepoCheckStatus {
        label,
        relative_path,
        status: if exists { "ok" } else { "missing" },
    }
}
