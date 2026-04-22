use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{
        HandoffArgs, HandoffCloseArgs, HandoffCommand, HandoffNewArgs, OutputFormat, TargetArgs,
    },
    config::GuardrailsConfig,
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

pub fn run(args: HandoffArgs) -> Result<()> {
    match args.command {
        None => print_template(args.target),
        Some(HandoffCommand::Print(args)) => print_template(args),
        Some(HandoffCommand::List(args)) => list(args),
        Some(HandoffCommand::New(args)) => new_handoff(args),
        Some(HandoffCommand::Close(args)) => close_handoff(args),
    }
}

fn print_template(args: TargetArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target)?;
    let contents = load_template(&repo_root)?;
    eprintln!(
        "`project-guardrails handoff` prints the template for compatibility; use `project-guardrails handoff new` for durable repo-local handoffs."
    );
    println!("{contents}");
    Ok(())
}

fn list(args: TargetArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target)?;
    let handoffs = load_all(&repo_root)?;

    if matches!(args.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&HandoffListOutput {
                schema_version: JSON_SCHEMA_VERSION,
                repo_root: repo_root.display().to_string(),
                handoffs: handoffs.iter().map(handoff_summary).collect(),
            })?
        );
        return Ok(());
    }

    println!("Guardrails handoffs");
    println!("repo_root={}", repo_root.display());
    if handoffs.is_empty() {
        println!("handoffs=none");
        return Ok(());
    }

    for handoff in handoffs {
        println!(
            "{} {} {} {}",
            handoff.id_string(),
            format!("{:?}", handoff.frontmatter.status).to_lowercase(),
            handoff.frontmatter.task_ids.len(),
            handoff.frontmatter.slug
        );
    }

    Ok(())
}

fn new_handoff(args: HandoffNewArgs) -> Result<()> {
    if !is_kebab_case(&args.slug) {
        bail!("handoff slugs must use kebab-case");
    }

    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    ensure_state_layout(&repo_root)?;
    if !args.task_ids.is_empty() {
        validate_task_ids_exist(&repo_root, &args.task_ids)?;
    }

    let handoffs = load_all(&repo_root)?;
    let record = HandoffRecord::new(
        next_handoff_id(&handoffs),
        &args.slug,
        args.title.as_deref(),
        args.task_ids,
        load_template(&repo_root)?,
    );
    record.write(&repo_root)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&HandoffGetOutput {
                schema_version: JSON_SCHEMA_VERSION,
                repo_root: repo_root.display().to_string(),
                handoff: handoff_output(&record),
            })?
        );
        return Ok(());
    }

    println!("Created handoff {}", record.id_string());
    println!("path={}", repo_root.join(&record.path).display());
    Ok(())
}

fn close_handoff(args: HandoffCloseArgs) -> Result<()> {
    let (repo_root, _) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let mut handoff = find_handoff(&load_all(&repo_root)?, args.id)?;
    handoff.close()?;
    handoff.write(&repo_root)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&HandoffGetOutput {
                schema_version: JSON_SCHEMA_VERSION,
                repo_root: repo_root.display().to_string(),
                handoff: handoff_output(&handoff),
            })?
        );
        return Ok(());
    }

    println!("Closed handoff {}", handoff.id_string());
    println!("path={}", repo_root.join(&handoff.path).display());
    Ok(())
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

#[derive(Debug, Serialize)]
struct HandoffListOutput {
    schema_version: u32,
    repo_root: String,
    handoffs: Vec<HandoffSummary>,
}

#[derive(Debug, Serialize)]
struct HandoffGetOutput {
    schema_version: u32,
    repo_root: String,
    handoff: HandoffOutput,
}

#[derive(Debug, Serialize)]
struct HandoffOutput {
    #[serde(flatten)]
    summary: HandoffSummary,
    body: String,
}
