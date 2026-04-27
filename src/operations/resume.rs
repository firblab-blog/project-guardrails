use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    git::{self, GitContinuity},
    output::{JSON_SCHEMA_VERSION, LlmRepoSummary, RepoDoctorSummary, build_llm_repo_summary},
    state::{
        handoffs::{self, HandoffStatus, HandoffSummary},
        tasks::TaskSummary,
    },
};

pub fn build_resume(target: &Path) -> Result<ResumeOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;
    let summary = build_llm_repo_summary(&repo_root, &config)?;
    let latest_handoff = load_latest_handoff(&repo_root, summary.latest_handoff.as_ref())?;
    let git = git::continuity_since_handoff(
        &repo_root,
        latest_handoff.as_ref().map(|handoff| &handoff.summary),
    );
    let linked_active_tasks = linked_active_tasks(&summary, latest_handoff.as_ref());
    let next_step = recommended_next_step(&summary, latest_handoff.as_ref(), &linked_active_tasks);

    Ok(ResumeOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        generated_at: summary.generated_at.clone(),
        latest_handoff,
        linked_active_tasks,
        git,
        doctor: summary.doctor,
        next_step,
    })
}

fn load_latest_handoff(
    repo_root: &Path,
    latest_summary: Option<&HandoffSummary>,
) -> Result<Option<ResumeHandoff>> {
    let Some(latest_summary) = latest_summary else {
        return Ok(None);
    };

    let handoff = handoffs::load_collection(repo_root)?
        .handoffs
        .into_iter()
        .find(|handoff| handoff.path == latest_summary.path);

    Ok(handoff.map(|handoff| ResumeHandoff {
        body_path: handoff.path.clone(),
        body_excerpt: compact_body_excerpt(&handoff.body),
        summary: handoff.summary(),
    }))
}

fn linked_active_tasks(
    summary: &LlmRepoSummary,
    latest_handoff: Option<&ResumeHandoff>,
) -> Vec<TaskSummary> {
    let Some(latest_handoff) = latest_handoff else {
        return Vec::new();
    };

    summary
        .active_tasks
        .iter()
        .filter(|task| {
            latest_handoff
                .summary
                .task_ids
                .iter()
                .any(|id| id == &task.id)
        })
        .cloned()
        .collect()
}

fn compact_body_excerpt(body: &str) -> String {
    let mut excerpt = String::new();
    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !excerpt.is_empty() {
            excerpt.push('\n');
        }
        excerpt.push_str(line);
        if excerpt.len() >= 800 || excerpt.lines().count() >= 8 {
            break;
        }
    }

    if excerpt.len() > 800 {
        let mut end = 797;
        while !excerpt.is_char_boundary(end) {
            end -= 1;
        }
        excerpt.truncate(end);
        excerpt.push_str("...");
    }
    excerpt
}

fn recommended_next_step(
    summary: &LlmRepoSummary,
    latest_handoff: Option<&ResumeHandoff>,
    linked_active_tasks: &[TaskSummary],
) -> ResumeNextStep {
    if has_refresh_worthy_diagnostic(summary) {
        return ResumeNextStep {
            command: "project-guardrails refresh --target . --check".to_string(),
            reason: "current diagnostics indicate stale or unsynchronized repo-local context"
                .to_string(),
        };
    }

    match (latest_handoff.as_ref(), linked_active_tasks.first()) {
        (Some(handoff), Some(task)) if handoff.summary.status == HandoffStatus::Open => {
            return ResumeNextStep {
                command: format!("project-guardrails tasks get {} --target .", task.id),
                reason: format!(
                    "latest open handoff {} links active task {}",
                    handoff.summary.path, task.id
                ),
            };
        }
        _ => {}
    }

    if !summary.active_tasks.is_empty() && linked_active_tasks.is_empty() {
        let reason = if latest_handoff.is_none() {
            "active tasks exist but no durable handoff is available"
        } else {
            "active tasks exist but the latest handoff has no linked active tasks"
        };
        return ResumeNextStep {
            command: "project-guardrails tasks list --target .".to_string(),
            reason: reason.to_string(),
        };
    }

    if summary.doctor.ok {
        return ResumeNextStep {
            command: "project-guardrails pre-work --target .".to_string(),
            reason: "doctor diagnostics are clean and no handoff-linked active task is available"
                .to_string(),
        };
    }

    ResumeNextStep {
        command: "project-guardrails check --target .".to_string(),
        reason: "doctor diagnostics are present and do not map to refresh --check".to_string(),
    }
}

fn has_refresh_worthy_diagnostic(summary: &LlmRepoSummary) -> bool {
    summary
        .doctor
        .diagnostics
        .iter()
        .any(|diagnostic| refresh_worthy_code(diagnostic.code))
}

fn refresh_worthy_code(code: &str) -> bool {
    matches!(
        code,
        "managed_block_stale"
            | "managed_block_missing"
            | "managed_block_invalid"
            | "managed_block_unreadable"
            | "managed_block_generator_error"
            | "task_tracker_sync_missing"
            | "handoff_stale"
            | "required_doc_stale_age"
    )
}

#[derive(Debug, Serialize)]
pub struct ResumeOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub generated_at: String,
    pub latest_handoff: Option<ResumeHandoff>,
    pub linked_active_tasks: Vec<TaskSummary>,
    pub git: GitContinuity,
    pub doctor: RepoDoctorSummary,
    pub next_step: ResumeNextStep,
}

#[derive(Debug, Serialize)]
pub struct ResumeHandoff {
    #[serde(flatten)]
    pub summary: HandoffSummary,
    pub body_path: String,
    pub body_excerpt: String,
}

#[derive(Debug, Serialize)]
pub struct ResumeNextStep {
    pub command: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::Diagnostic,
        output::{RepoConfigOutput, RepoProfileOutput},
        state::tasks::{TaskPriority, TaskStatus},
    };

    #[test]
    fn refresh_diagnostics_win_next_step() {
        let summary = summary_with_tasks(
            vec![task_summary("0001")],
            false,
            vec![Diagnostic::new("managed_block_stale", "stale block")],
        );
        let next = recommended_next_step(&summary, None, &[]);

        assert_eq!(
            next.command,
            "project-guardrails refresh --target . --check"
        );
    }

    #[test]
    fn open_handoff_with_linked_active_task_recommends_task_context() {
        let summary = summary_with_tasks(vec![task_summary("0001")], true, Vec::new());
        let handoff = resume_handoff(HandoffStatus::Open, vec!["0001".to_string()]);
        let linked = linked_active_tasks(&summary, Some(&handoff));
        let next = recommended_next_step(&summary, Some(&handoff), &linked);

        assert_eq!(next.command, "project-guardrails tasks get 0001 --target .");
    }

    #[test]
    fn active_tasks_without_handoff_recommend_task_list() {
        let summary = summary_with_tasks(vec![task_summary("0001")], false, Vec::new());
        let next = recommended_next_step(&summary, None, &[]);

        assert_eq!(next.command, "project-guardrails tasks list --target .");
    }

    #[test]
    fn active_tasks_with_unlinked_handoff_recommend_task_list() {
        let summary = summary_with_tasks(vec![task_summary("0001")], true, Vec::new());
        let handoff = resume_handoff(HandoffStatus::Open, vec!["0002".to_string()]);
        let linked = linked_active_tasks(&summary, Some(&handoff));
        let next = recommended_next_step(&summary, Some(&handoff), &linked);

        assert_eq!(next.command, "project-guardrails tasks list --target .");
    }

    fn summary_with_tasks(
        active_tasks: Vec<TaskSummary>,
        ok: bool,
        diagnostics: Vec<Diagnostic>,
    ) -> LlmRepoSummary {
        LlmRepoSummary {
            repo_root: "/tmp/repo".to_string(),
            generated_at: "2026-04-25T00:00:00Z".to_string(),
            profile: RepoProfileOutput {
                name: "minimal".to_string(),
                source: "built-in:minimal".to_string(),
                schema_version: 1,
                installed_by_version: "0.2.0".to_string(),
            },
            repo: RepoConfigOutput {
                config_version: 1,
                docs_enabled: true,
                ci_provider: "none".to_string(),
                required_docs: Vec::new(),
                required_files: Vec::new(),
                forbidden_dirs: Vec::new(),
                semgrep_enabled: false,
                conftest_enabled: false,
                task_references_required: true,
            },
            required_reading: Vec::new(),
            active_tasks,
            recent_handoffs: Vec::new(),
            latest_handoff: None,
            doctor: RepoDoctorSummary { ok, diagnostics },
        }
    }

    fn task_summary(id: &str) -> TaskSummary {
        TaskSummary {
            id: id.to_string(),
            slug: "resume-core".to_string(),
            title: "Resume Core".to_string(),
            status: TaskStatus::InProgress,
            owner: Some("codex".to_string()),
            priority: Some(TaskPriority::P1),
            updated: "2026-04-25T00:00:00Z".to_string(),
            path: format!(".guardrails/state/tasks/{id}-resume-core.md"),
        }
    }

    fn resume_handoff(status: HandoffStatus, task_ids: Vec<String>) -> ResumeHandoff {
        ResumeHandoff {
            summary: HandoffSummary {
                id: "0001".to_string(),
                slug: "resume-core".to_string(),
                title: "Resume Core".to_string(),
                status,
                created: "2026-04-25T00:00:00Z".to_string(),
                updated: "2026-04-25T00:00:00Z".to_string(),
                task_ids,
                template_path: "docs/project/handoff-template.md".to_string(),
                path: ".guardrails/state/handoffs/0001-resume-core.md".to_string(),
            },
            body_path: ".guardrails/state/handoffs/0001-resume-core.md".to_string(),
            body_excerpt: "# Resume Core".to_string(),
        }
    }
}
