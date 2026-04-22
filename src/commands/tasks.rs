use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{
        OutputFormat, TasksClaimArgs, TasksCloseArgs, TasksCommand, TasksGetArgs, TasksLintArgs,
        TasksListArgs, TasksNewArgs, TasksUpdateArgs,
    },
    config::GuardrailsConfig,
    output::JSON_SCHEMA_VERSION,
    state::tasks::{
        TaskRecord, TaskStatus, TaskSummary, find_task, lint_tasks, load_valid_tasks, next_task_id,
        save_task,
    },
    state::{ensure_state_layout, is_kebab_case},
};

pub fn run(command: TasksCommand) -> Result<()> {
    match command {
        TasksCommand::List(args) => list(args),
        TasksCommand::Get(args) => get(args),
        TasksCommand::New(args) => new(args),
        TasksCommand::Claim(args) => claim(args),
        TasksCommand::Update(args) => update(args),
        TasksCommand::Close(args) => close(args),
        TasksCommand::Lint(args) => lint(args),
    }
}

fn list(args: TasksListArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    ensure_state_layout(&repo_root)?;
    let tasks = load_valid_tasks(&repo_root)?;
    let filtered = tasks
        .into_iter()
        .filter(|task| {
            args.status
                .is_none_or(|status| task.frontmatter.status == status)
                && args
                    .owner
                    .as_ref()
                    .is_none_or(|owner| task.frontmatter.owner.as_deref() == Some(owner.as_str()))
        })
        .map(|task| task.summary())
        .collect::<Vec<_>>();

    if matches!(args.target.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&TaskListOutput {
                schema_version: JSON_SCHEMA_VERSION,
                repo_root: repo_root.display().to_string(),
                tasks: filtered,
            })?
        );
        return Ok(());
    }

    println!("Guardrails tasks");
    println!("repo_root={}", repo_root.display());
    if filtered.is_empty() {
        println!("tasks=none");
        return Ok(());
    }

    for task in filtered {
        println!(
            "{} {} {} {} {}",
            task.id,
            task.status.as_str(),
            task.priority.map(|value| value.as_str()).unwrap_or("-"),
            task.owner.as_deref().unwrap_or("-"),
            task.slug
        );
    }

    Ok(())
}

fn get(args: TasksGetArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let task = find_task(&load_valid_tasks(&repo_root)?, args.id)?;
    let raw = std::fs::read_to_string(repo_root.join(&task.path))?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&TaskGetOutput {
                schema_version: JSON_SCHEMA_VERSION,
                repo_root: repo_root.display().to_string(),
                task: task_to_output(&task),
            })?
        );
        return Ok(());
    }

    println!("path={}", repo_root.join(&task.path).display());
    println!("{raw}");
    Ok(())
}

fn new(args: TasksNewArgs) -> Result<()> {
    if !is_kebab_case(&args.slug) {
        bail!("task slugs must use kebab-case");
    }

    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let tasks = load_valid_tasks(&repo_root)?;
    let task = TaskRecord::new(
        next_task_id(&tasks),
        &args.slug,
        args.title.as_deref(),
        args.priority,
        args.owner.as_deref(),
    );
    save_task(&repo_root, &task)?;
    print_task_mutation(
        args.target.format,
        &repo_root,
        "Created task",
        &task,
        "task created under .guardrails/state/tasks/",
    )
}

fn claim(args: TasksClaimArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let mut task = find_task(&load_valid_tasks(&repo_root)?, args.id)?;
    task.set_owner(Some(args.owner));
    task.set_status(TaskStatus::InProgress)?;
    save_task(&repo_root, &task)?;
    print_task_mutation(
        args.target.format,
        &repo_root,
        "Claimed task",
        &task,
        "task owner recorded and status moved to in_progress",
    )
}

fn update(args: TasksUpdateArgs) -> Result<()> {
    if args.status.is_none() && args.owner.is_none() && args.priority.is_none() {
        bail!("tasks update requires at least one of --status, --owner, or --priority");
    }

    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let mut task = find_task(&load_valid_tasks(&repo_root)?, args.id)?;

    if let Some(owner) = args.owner {
        task.set_owner(Some(owner));
    }
    if let Some(priority) = args.priority {
        task.set_priority(Some(priority));
    }
    if let Some(status) = args.status {
        task.set_status(status)?;
    }

    save_task(&repo_root, &task)?;
    print_task_mutation(
        args.target.format,
        &repo_root,
        "Updated task",
        &task,
        "task frontmatter updated",
    )
}

fn close(args: TasksCloseArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let mut task = find_task(&load_valid_tasks(&repo_root)?, args.id)?;
    task.add_commit(&args.commit);
    task.set_status(TaskStatus::Done)?;
    save_task(&repo_root, &task)?;
    print_task_mutation(
        args.target.format,
        &repo_root,
        "Closed task",
        &task,
        "task marked done and commit recorded",
    )
}

fn lint(args: TasksLintArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let diagnostics = lint_tasks(&repo_root)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&TaskLintOutput {
                schema_version: JSON_SCHEMA_VERSION,
                ok: diagnostics.is_empty(),
                repo_root: repo_root.display().to_string(),
                diagnostics: diagnostics.diagnostics().to_vec(),
            })?
        );
    } else if diagnostics.is_empty() {
        println!("Task lint passed.");
    } else {
        diagnostics.print_stderr();
    }

    if diagnostics.is_empty() {
        return Ok(());
    }

    bail!("task lint found {} issue(s)", diagnostics.len())
}

fn print_task_mutation(
    format: OutputFormat,
    repo_root: &std::path::Path,
    label: &str,
    task: &TaskRecord,
    detail: &str,
) -> Result<()> {
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&TaskGetOutput {
                schema_version: JSON_SCHEMA_VERSION,
                repo_root: repo_root.display().to_string(),
                task: task_to_output(task),
            })?
        );
        return Ok(());
    }

    println!("{label} {}", task.id_string());
    println!("path={}", repo_root.join(&task.path).display());
    println!("status={}", task.frontmatter.status.as_str());
    println!("detail={detail}");
    Ok(())
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

#[derive(Debug, Serialize)]
struct TaskListOutput {
    schema_version: u32,
    repo_root: String,
    tasks: Vec<TaskSummary>,
}

#[derive(Debug, Serialize)]
struct TaskGetOutput {
    schema_version: u32,
    repo_root: String,
    task: TaskOutput,
}

#[derive(Debug, Serialize)]
struct TaskLintOutput {
    schema_version: u32,
    ok: bool,
    repo_root: String,
    diagnostics: Vec<crate::diagnostics::Diagnostic>,
}

#[derive(Debug, Serialize)]
struct TaskOutput {
    path: String,
    id: String,
    slug: String,
    title: String,
    status: crate::state::tasks::TaskStatus,
    owner: Option<String>,
    priority: Option<crate::state::tasks::TaskPriority>,
    created: String,
    updated: String,
    refs: crate::state::tasks::TaskRefs,
    commits: Vec<String>,
    body: String,
}
