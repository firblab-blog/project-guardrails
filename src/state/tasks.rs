use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::diagnostics::{Diagnostic, DiagnosticReport};

use super::{
    TASKS_DIR, is_kebab_case, now_timestamp, parse_toml_frontmatter, render_toml_frontmatter,
    tasks_dir, title_from_slug,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Proposed,
    Approved,
    InProgress,
    Blocked,
    Done,
    Dropped,
}

impl TaskStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        if self == next {
            return true;
        }

        matches!(
            (self, next),
            (Self::Proposed, Self::Approved)
                | (Self::Proposed, Self::InProgress)
                | (Self::Proposed, Self::Dropped)
                | (Self::Approved, Self::InProgress)
                | (Self::Approved, Self::Blocked)
                | (Self::Approved, Self::Done)
                | (Self::Approved, Self::Dropped)
                | (Self::InProgress, Self::Blocked)
                | (Self::InProgress, Self::Done)
                | (Self::Blocked, Self::InProgress)
                | (Self::Blocked, Self::Dropped)
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Dropped => "dropped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum TaskPriority {
    #[serde(rename = "p0")]
    P0,
    #[serde(rename = "p1")]
    P1,
    #[serde(rename = "p2")]
    P2,
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "p0",
            Self::P1 => "p1",
            Self::P2 => "p2",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRefs {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracker: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<String>,
}

impl TaskRefs {
    fn is_empty(&self) -> bool {
        self.tracker.is_empty() && self.code.is_empty() && self.docs.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskFrontmatter {
    pub id: u32,
    pub slug: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<TaskPriority>,
    pub created: String,
    pub updated: String,
    #[serde(default, skip_serializing_if = "TaskRefs::is_empty")]
    pub refs: TaskRefs,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRecord {
    pub path: String,
    pub frontmatter: TaskFrontmatter,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub status: TaskStatus,
    pub owner: Option<String>,
    pub priority: Option<TaskPriority>,
    pub updated: String,
    pub path: String,
}

#[derive(Debug, Default)]
pub struct TaskCollection {
    pub tasks: Vec<TaskRecord>,
    pub diagnostics: DiagnosticReport,
}

impl TaskRecord {
    pub fn new(
        id: u32,
        slug: &str,
        title: Option<&str>,
        priority: Option<TaskPriority>,
        owner: Option<&str>,
    ) -> Self {
        let timestamp = now_timestamp();
        let title = title
            .map(str::to_string)
            .unwrap_or_else(|| title_from_slug(slug));
        let path = task_relative_path(id, slug);
        Self {
            path,
            frontmatter: TaskFrontmatter {
                id,
                slug: slug.to_string(),
                title: title.clone(),
                status: if owner.is_some() {
                    TaskStatus::InProgress
                } else {
                    TaskStatus::Proposed
                },
                owner: owner.map(str::to_string),
                priority,
                created: timestamp.clone(),
                updated: timestamp,
                refs: TaskRefs {
                    tracker: vec!["docs/project/implementation-tracker.md".to_string()],
                    ..TaskRefs::default()
                },
                commits: Vec::new(),
            },
            body: default_task_body(&title),
        }
    }

    pub fn id_string(&self) -> String {
        format!("{:04}", self.frontmatter.id)
    }

    pub fn absolute_path(&self, repo_root: &Path) -> PathBuf {
        repo_root.join(&self.path)
    }

    pub fn summary(&self) -> TaskSummary {
        TaskSummary {
            id: self.id_string(),
            slug: self.frontmatter.slug.clone(),
            title: self.frontmatter.title.clone(),
            status: self.frontmatter.status,
            owner: self.frontmatter.owner.clone(),
            priority: self.frontmatter.priority,
            updated: self.frontmatter.updated.clone(),
            path: self.path.clone(),
        }
    }

    pub fn set_status(&mut self, status: TaskStatus) -> Result<()> {
        if !self.frontmatter.status.can_transition_to(status) {
            bail!(
                "invalid task transition for {}: {} -> {}",
                self.id_string(),
                self.frontmatter.status.as_str(),
                status.as_str()
            );
        }

        self.frontmatter.status = status;
        self.touch();
        Ok(())
    }

    pub fn set_owner(&mut self, owner: Option<String>) {
        self.frontmatter.owner = owner;
        self.touch();
    }

    pub fn set_priority(&mut self, priority: Option<TaskPriority>) {
        self.frontmatter.priority = priority;
        self.touch();
    }

    pub fn add_commit(&mut self, commit: &str) {
        if !self
            .frontmatter
            .commits
            .iter()
            .any(|existing| existing == commit)
        {
            self.frontmatter.commits.push(commit.to_string());
        }
        self.touch();
    }

    pub fn write(&self, repo_root: &Path) -> Result<()> {
        let destination = self.absolute_path(repo_root);
        let rendered = render_toml_frontmatter(&self.frontmatter, &self.body)?;
        fs::write(&destination, rendered)
            .with_context(|| format!("failed to write {}", destination.display()))
    }

    fn touch(&mut self) {
        self.frontmatter.updated = now_timestamp();
    }
}

pub fn load_collection(repo_root: &Path) -> Result<TaskCollection> {
    let task_dir = tasks_dir(repo_root);
    if !task_dir.exists() {
        return Ok(TaskCollection::default());
    }
    let mut collection = TaskCollection::default();

    for entry in
        fs::read_dir(&task_dir).with_context(|| format!("failed to read {}", task_dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", task_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        match parse_task_file(repo_root, &path) {
            Ok(task) => collection.tasks.push(task),
            Err(error) => collection.diagnostics.push(Diagnostic::new(
                "task_parse_error",
                format!("failed to parse {}: {}", path.display(), error),
            )),
        }
    }

    collection.tasks.sort_by_key(|task| task.frontmatter.id);
    collection.diagnostics.extend(
        validate_task_collection(repo_root, &collection.tasks)
            .diagnostics()
            .to_vec(),
    );
    Ok(collection)
}

pub fn load_valid_tasks(repo_root: &Path) -> Result<Vec<TaskRecord>> {
    let collection = load_collection(repo_root)?;
    if collection.diagnostics.is_empty() {
        return Ok(collection.tasks);
    }

    collection.diagnostics.print_stderr();
    bail!(
        "task state is invalid; run `project-guardrails tasks lint --target {}`",
        repo_root.display()
    )
}

pub fn next_task_id(tasks: &[TaskRecord]) -> u32 {
    tasks
        .iter()
        .map(|task| task.frontmatter.id)
        .max()
        .unwrap_or(0)
        + 1
}

pub fn find_task(tasks: &[TaskRecord], id: u32) -> Result<TaskRecord> {
    tasks
        .iter()
        .find(|task| task.frontmatter.id == id)
        .cloned()
        .with_context(|| format!("task {:04} was not found", id))
}

pub fn save_task(repo_root: &Path, task: &TaskRecord) -> Result<()> {
    task.write(repo_root)
}

pub fn lint_tasks(repo_root: &Path) -> Result<DiagnosticReport> {
    Ok(load_collection(repo_root)?.diagnostics)
}

pub fn validate_task_ids_exist(repo_root: &Path, ids: &[u32]) -> Result<()> {
    let tasks = load_valid_tasks(repo_root)?;
    let known = tasks
        .iter()
        .map(|task| task.frontmatter.id)
        .collect::<BTreeSet<_>>();
    for id in ids {
        if !known.contains(id) {
            bail!("task {:04} was not found", id);
        }
    }
    Ok(())
}

pub fn task_relative_path(id: u32, slug: &str) -> String {
    format!("{TASKS_DIR}/{id:04}-{slug}.md")
}

fn parse_task_file(repo_root: &Path, path: &Path) -> Result<TaskRecord> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let (frontmatter, body): (TaskFrontmatter, String) = parse_toml_frontmatter(&raw)?;
    let relative = path
        .strip_prefix(repo_root)
        .with_context(|| format!("failed to relativize {}", path.display()))?
        .to_string_lossy()
        .replace('\\', "/");

    Ok(TaskRecord {
        path: relative,
        frontmatter,
        body,
    })
}

fn validate_task_collection(repo_root: &Path, tasks: &[TaskRecord]) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();
    let mut ids = BTreeMap::<u32, Vec<&str>>::new();

    for task in tasks {
        ids.entry(task.frontmatter.id).or_default().push(&task.path);
        report.extend(validate_single_task(repo_root, task).diagnostics().to_vec());
    }

    for (id, paths) in ids {
        if paths.len() > 1 {
            report.push(Diagnostic::new(
                "task_duplicate_id",
                format!(
                    "task {:04} is defined more than once: {}",
                    id,
                    paths.join(", ")
                ),
            ));
        }
    }

    report
}

fn validate_single_task(repo_root: &Path, task: &TaskRecord) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();
    let expected_path = task_relative_path(task.frontmatter.id, &task.frontmatter.slug);

    if task.path != expected_path {
        report.push(Diagnostic::new(
            "task_filename_mismatch",
            format!(
                "{} should be named {} to match id/slug",
                task.path, expected_path
            ),
        ));
    }

    if !is_kebab_case(&task.frontmatter.slug) {
        report.push(Diagnostic::new(
            "task_slug_invalid",
            format!(
                "{} has an invalid slug `{}`",
                task.path, task.frontmatter.slug
            ),
        ));
    }

    if task.frontmatter.title.trim().is_empty() {
        report.push(Diagnostic::new(
            "task_title_missing",
            format!("{} is missing a task title", task.path),
        ));
    }

    if task.body.trim().is_empty() {
        report.push(Diagnostic::new(
            "task_body_empty",
            format!("{} is missing markdown body content", task.path),
        ));
    }

    if task.frontmatter.status == TaskStatus::InProgress
        && task
            .frontmatter
            .owner
            .as_ref()
            .is_none_or(|owner| owner.trim().is_empty())
    {
        report.push(Diagnostic::new(
            "task_owner_required",
            format!("{} is in_progress but has no owner", task.path),
        ));
    }

    if task.frontmatter.status == TaskStatus::Done && task.frontmatter.commits.is_empty() {
        report.push(Diagnostic::new(
            "task_commit_required",
            format!("{} is done but has no commits recorded", task.path),
        ));
    }

    if matches!(
        task.frontmatter.status,
        TaskStatus::Approved | TaskStatus::InProgress | TaskStatus::Blocked
    ) && !task.frontmatter.refs.tracker.iter().any(|reference| {
        reference.split('#').next() == Some("docs/project/implementation-tracker.md")
    }) {
        report.push(Diagnostic::new(
            "task_tracker_ref_missing",
            format!(
                "{} is active but does not reference docs/project/implementation-tracker.md",
                task.path
            ),
        ));
    }

    report.extend(
        validate_refs(repo_root, task, &task.frontmatter.refs.code, "code")
            .diagnostics()
            .to_vec(),
    );
    report.extend(
        validate_refs(repo_root, task, &task.frontmatter.refs.docs, "docs")
            .diagnostics()
            .to_vec(),
    );
    report.extend(
        validate_refs(repo_root, task, &task.frontmatter.refs.tracker, "tracker")
            .diagnostics()
            .to_vec(),
    );

    report
}

fn validate_refs(
    repo_root: &Path,
    task: &TaskRecord,
    refs: &[String],
    label: &'static str,
) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();

    for reference in refs {
        let path = reference.split('#').next().unwrap_or(reference);
        if path.is_empty() {
            continue;
        }

        if !repo_root.join(path).exists() {
            report.push(Diagnostic::new(
                "task_ref_missing",
                format!("{} references missing {} path `{}`", task.path, label, path),
            ));
        }
    }

    report
}

fn default_task_body(title: &str) -> String {
    format!(
        "# {title}\n\n## Intent\n\nDescribe the approved slice of work and why it matters.\n\n## Acceptance\n\n- define what has to be true for this task to be done\n\n## Non-Goals\n\n- list the things this task should not widen into\n\n## Working Notes\n\n- capture progress, constraints, and handoff-quality notes here\n"
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use super::{
        TaskPriority, TaskRecord, TaskStatus, load_collection, save_task, task_relative_path,
        validate_task_collection,
    };
    use crate::state::ensure_state_layout;

    #[test]
    fn task_round_trip_preserves_frontmatter_and_body() {
        let temp = TempDir::new().expect("temp dir");
        ensure_state_layout(temp.path()).expect("state layout");
        std::fs::create_dir_all(temp.path().join("docs/project")).expect("docs dir");
        std::fs::write(
            temp.path().join("docs/project/implementation-tracker.md"),
            "# Tracker\n",
        )
        .expect("tracker");
        let task = TaskRecord::new(
            1,
            "example-task",
            Some("Example Task"),
            Some(TaskPriority::P1),
            Some("codex"),
        );
        save_task(temp.path(), &task).expect("save task");

        let collection = load_collection(temp.path()).expect("load collection");
        assert!(collection.diagnostics.is_empty());
        assert_eq!(collection.tasks.len(), 1);
        assert_eq!(collection.tasks[0].frontmatter.slug, "example-task");
        assert!(collection.tasks[0].body.contains("## Intent"));
    }

    #[test]
    fn task_validation_reports_duplicate_ids() {
        let first = TaskRecord::new(7, "first-task", None, None, None);
        let mut second = TaskRecord::new(7, "second-task", None, None, None);
        second.path = task_relative_path(7, "second-task");

        let diagnostics = validate_task_collection(Path::new("."), &[first, second]);
        assert!(
            diagnostics
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "task_duplicate_id")
        );
    }

    #[test]
    fn task_status_transitions_reject_done_back_to_in_progress() {
        assert!(!TaskStatus::Done.can_transition_to(TaskStatus::InProgress));
        assert!(TaskStatus::Approved.can_transition_to(TaskStatus::InProgress));
    }
}
