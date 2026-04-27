use std::path::Path;

use anyhow::{Result, bail};

use crate::{
    cli::{
        OutputFormat, TasksClaimArgs, TasksCloseArgs, TasksCommand, TasksGetArgs, TasksLintArgs,
        TasksListArgs, TasksNewArgs, TasksUpdateArgs,
    },
    operations::tasks::{
        TaskCreateInput, TaskGetOutput, TaskListOptions, TaskUpdateInput, claim_task, close_task,
        create_task, get_task, lint_task_state, list_tasks, task_raw_body, update_task,
    },
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
    let output = list_tasks(
        &args.target.target,
        TaskListOptions {
            status: args.status,
            owner: args.owner,
        },
    )?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Guardrails tasks");
    println!("repo_root={}", output.repo_root);
    if output.tasks.is_empty() {
        println!("tasks=none");
        return Ok(());
    }

    for task in output.tasks {
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
    if matches!(args.target.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&get_task(&args.target.target, args.id)?)?
        );
        return Ok(());
    }

    let (path, raw) = task_raw_body(&args.target.target, args.id)?;
    println!("path={path}");
    println!("{raw}");
    Ok(())
}

fn new(args: TasksNewArgs) -> Result<()> {
    let output = create_task(
        &args.target.target,
        TaskCreateInput {
            slug: args.slug,
            title: args.title,
            priority: args.priority,
            owner: args.owner,
        },
    )?;
    print_task_mutation(
        args.target.format,
        "Created task",
        &output,
        "task created under .guardrails/state/tasks/",
    )
}

fn claim(args: TasksClaimArgs) -> Result<()> {
    let output = claim_task(&args.target.target, args.id, args.owner)?;
    print_task_mutation(
        args.target.format,
        "Claimed task",
        &output,
        "task owner recorded and status moved to in_progress",
    )
}

fn update(args: TasksUpdateArgs) -> Result<()> {
    let output = update_task(
        &args.target.target,
        args.id,
        TaskUpdateInput {
            status: args.status,
            owner: args.owner,
            priority: args.priority,
        },
    )?;
    print_task_mutation(
        args.target.format,
        "Updated task",
        &output,
        "task frontmatter updated",
    )
}

fn close(args: TasksCloseArgs) -> Result<()> {
    let output = close_task(&args.target.target, args.id, &args.commit)?;
    print_task_mutation(
        args.target.format,
        "Closed task",
        &output,
        "task marked done and commit recorded",
    )
}

fn lint(args: TasksLintArgs) -> Result<()> {
    let output = lint_task_state(&args.target.target)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if output.ok {
        println!("Task lint passed.");
    } else {
        for diagnostic in &output.diagnostics {
            eprintln!("[{}] {}", diagnostic.code, diagnostic.message);
        }
    }

    if output.ok {
        return Ok(());
    }

    bail!("task lint found {} issue(s)", output.diagnostics.len())
}

fn print_task_mutation(
    format: OutputFormat,
    label: &str,
    output: &TaskGetOutput,
    detail: &str,
) -> Result<()> {
    if matches!(format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(output)?);
        return Ok(());
    }

    println!("{label} {}", output.task.id);
    println!(
        "path={}",
        Path::new(&output.repo_root)
            .join(&output.task.path)
            .display()
    );
    println!("status={}", output.task.status.as_str());
    println!("detail={detail}");
    Ok(())
}
