use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    diagnostics::Diagnostic,
    output::JSON_SCHEMA_VERSION,
    state::tasks::{
        TaskPriority, TaskRecord, TaskRefs, TaskStatus, TaskSummary, find_task, lint_tasks,
        load_valid_tasks, next_task_id, save_task,
    },
    state::{ensure_state_layout, is_kebab_case},
};

pub fn list_tasks(target: &Path, options: TaskListOptions) -> Result<TaskListOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    ensure_state_layout(&repo_root)?;
    let tasks = load_valid_tasks(&repo_root)?;
    let filtered = tasks
        .into_iter()
        .filter(|task| {
            options
                .status
                .is_none_or(|status| task.frontmatter.status == status)
                && options
                    .owner
                    .as_ref()
                    .is_none_or(|owner| task.frontmatter.owner.as_deref() == Some(owner.as_str()))
        })
        .map(|task| task.summary())
        .collect::<Vec<_>>();

    Ok(TaskListOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        tasks: filtered,
    })
}

pub fn get_task(target: &Path, id: u32) -> Result<TaskGetOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let task = find_task(&load_valid_tasks(&repo_root)?, id)?;

    Ok(TaskGetOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        task: task_to_output(&task),
    })
}

pub fn task_raw_body(target: &Path, id: u32) -> Result<(String, String)> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let task = find_task(&load_valid_tasks(&repo_root)?, id)?;
    let raw = std::fs::read_to_string(repo_root.join(&task.path))?;

    Ok((repo_root.join(&task.path).display().to_string(), raw))
}

pub fn create_task(target: &Path, input: TaskCreateInput) -> Result<TaskGetOutput> {
    if !is_kebab_case(&input.slug) {
        bail!("task slugs must use kebab-case");
    }

    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let tasks = load_valid_tasks(&repo_root)?;
    let task = TaskRecord::new(
        next_task_id(&tasks),
        &input.slug,
        input.title.as_deref(),
        input.priority,
        input.owner.as_deref(),
    );
    save_task(&repo_root, &task)?;
    task_mutation_output(&repo_root, &task)
}

pub fn claim_task(target: &Path, id: u32, owner: String) -> Result<TaskGetOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let mut task = find_task(&load_valid_tasks(&repo_root)?, id)?;
    task.set_owner(Some(owner));
    task.set_status(TaskStatus::InProgress)?;
    save_task(&repo_root, &task)?;
    task_mutation_output(&repo_root, &task)
}

pub fn update_task(target: &Path, id: u32, input: TaskUpdateInput) -> Result<TaskGetOutput> {
    if input.status.is_none() && input.owner.is_none() && input.priority.is_none() {
        bail!("tasks update requires at least one of --status, --owner, or --priority");
    }

    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let mut task = find_task(&load_valid_tasks(&repo_root)?, id)?;

    if let Some(owner) = input.owner {
        task.set_owner(Some(owner));
    }
    if let Some(priority) = input.priority {
        task.set_priority(Some(priority));
    }
    if let Some(status) = input.status {
        task.set_status(status)?;
    }

    save_task(&repo_root, &task)?;
    task_mutation_output(&repo_root, &task)
}

pub fn close_task(target: &Path, id: u32, commit: &str) -> Result<TaskGetOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let mut task = find_task(&load_valid_tasks(&repo_root)?, id)?;
    task.add_commit(commit);
    task.set_status(TaskStatus::Done)?;
    save_task(&repo_root, &task)?;
    task_mutation_output(&repo_root, &task)
}

pub fn lint_task_state(target: &Path) -> Result<TaskLintOutput> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(target)?;
    let diagnostics = lint_tasks(&repo_root)?;

    Ok(TaskLintOutput {
        schema_version: JSON_SCHEMA_VERSION,
        ok: diagnostics.is_empty(),
        repo_root: repo_root.display().to_string(),
        diagnostics: diagnostics.diagnostics().to_vec(),
    })
}

fn task_mutation_output(repo_root: &Path, task: &TaskRecord) -> Result<TaskGetOutput> {
    Ok(TaskGetOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        task: task_to_output(task),
    })
}

fn task_to_output(task: &TaskRecord) -> TaskOutput {
    TaskOutput {
        path: task.path.clone(),
        id: task.id_string(),
        slug: task.frontmatter.slug.clone(),
        title: task.frontmatter.title.clone(),
        status: task.frontmatter.status,
        owner: task.frontmatter.owner.clone(),
        priority: task.frontmatter.priority,
        created: task.frontmatter.created.clone(),
        updated: task.frontmatter.updated.clone(),
        refs: task.frontmatter.refs.clone(),
        commits: task.frontmatter.commits.clone(),
        body: task.body.clone(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskListOptions {
    pub status: Option<TaskStatus>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskCreateInput {
    pub slug: String,
    pub title: Option<String>,
    pub priority: Option<TaskPriority>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskUpdateInput {
    pub status: Option<TaskStatus>,
    pub owner: Option<String>,
    pub priority: Option<TaskPriority>,
}

#[derive(Debug, Serialize)]
pub struct TaskListOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub tasks: Vec<TaskSummary>,
}

#[derive(Debug, Serialize)]
pub struct TaskGetOutput {
    pub schema_version: u32,
    pub repo_root: String,
    pub task: TaskOutput,
}

#[derive(Debug, Serialize)]
pub struct TaskLintOutput {
    pub schema_version: u32,
    pub ok: bool,
    pub repo_root: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct TaskOutput {
    pub path: String,
    pub id: String,
    pub slug: String,
    pub title: String,
    pub status: TaskStatus,
    pub owner: Option<String>,
    pub priority: Option<TaskPriority>,
    pub created: String,
    pub updated: String,
    pub refs: TaskRefs,
    pub commits: Vec<String>,
    pub body: String,
}
