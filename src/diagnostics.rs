use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{config::GuardrailsConfig, rule_engine};
use serde::Serialize;

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

    pub fn len(&self) -> usize {
        self.diagnostics.len()
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

            validate_required_doc(report, required, &path);
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

        validate_required_file(report, required, &path);
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

    report.extend(rule_engine::diagnose_external_engines(repo_root, config));
}

fn validate_required_doc(report: &mut DiagnosticReport, relative_path: &str, path: &Path) {
    validate_text_file(report, relative_path, path, "required_doc");

    if let Some(starter) = starter_content_rule(relative_path) {
        detect_starter_content(report, relative_path, path, "required_doc", starter);
    }
}

fn validate_required_file(report: &mut DiagnosticReport, relative_path: &str, path: &Path) {
    if !matches!(relative_path, "README.md" | "AGENTS.md") {
        return;
    }

    validate_text_file(report, relative_path, path, "required_file");

    if let Some(starter) = starter_content_rule(relative_path) {
        detect_starter_content(report, relative_path, path, "required_file", starter);
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
    starter: StarterContentRule,
) {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    let matched_markers = starter
        .markers
        .iter()
        .filter(|marker| contents.contains(**marker))
        .count();

    if matched_markers >= starter.threshold {
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

#[derive(Clone, Copy)]
struct StarterContentRule {
    markers: &'static [&'static str],
    threshold: usize,
}

fn starter_content_rule(relative_path: &str) -> Option<StarterContentRule> {
    match relative_path {
        "AGENTS.md" => Some(StarterContentRule {
            markers: &[
                "Describe what this repository exists to do.",
                "state the approved implementation center",
                "state the main non-goals",
                "state what contributors should read before substantial work",
                "state what should never be widened casually",
            ],
            threshold: 2,
        }),
        "docs/project/implementation-tracker.md" => Some(StarterContentRule {
            markers: &[
                "define the current narrow slice of work",
                "replace this line with the next valid step",
                "replace this with explicit non-goals that prevent drift",
                "replace with assumptions validated by current work",
                "replace with unresolved decisions that could change scope",
            ],
            threshold: 2,
        }),
        "docs/project/handoff-template.md" => Some(StarterContentRule {
            markers: &[
                "what changed",
                "what was validated",
                "what remains intentionally unimplemented",
                "list the docs updated in the same change",
                "recommend the narrowest next step",
            ],
            threshold: 2,
        }),
        _ => None,
    }
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
