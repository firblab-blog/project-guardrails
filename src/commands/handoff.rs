use std::path::Path;

use anyhow::Result;

use crate::{
    cli::{
        HandoffArgs, HandoffCloseArgs, HandoffCommand, HandoffNewArgs, OutputFormat, TargetArgs,
    },
    operations::handoff::{
        HandoffCreateInput, HandoffGetOutput, HandoffListOutput, close_handoff, create_handoff,
        list_handoffs, load_handoff_template,
    },
    state::handoffs::HandoffStatus,
};

pub fn run(args: HandoffArgs) -> Result<()> {
    match args.command {
        None => print_template(args.target),
        Some(HandoffCommand::Print(args)) => print_template(args),
        Some(HandoffCommand::List(args)) => list(args),
        Some(HandoffCommand::New(args)) => new_handoff(args),
        Some(HandoffCommand::Close(args)) => close(args),
    }
}

fn print_template(args: TargetArgs) -> Result<()> {
    let contents = load_handoff_template(&args.target)?;
    eprintln!(
        "`project-guardrails handoff` prints the template for compatibility; use `project-guardrails handoff new` for durable repo-local handoffs."
    );
    println!("{contents}");
    Ok(())
}

fn list(args: TargetArgs) -> Result<()> {
    let output = list_handoffs(&args.target)?;

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    print_handoff_list(&output);
    Ok(())
}

fn new_handoff(args: HandoffNewArgs) -> Result<()> {
    let output = create_handoff(
        &args.target.target,
        HandoffCreateInput {
            slug: args.slug,
            title: args.title,
            task_ids: args.task_ids,
            from_git: args.from_git,
        },
    )?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    print_handoff_mutation("Created handoff", &output);
    Ok(())
}

fn close(args: HandoffCloseArgs) -> Result<()> {
    let output = close_handoff(&args.target.target, args.id)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    print_handoff_mutation("Closed handoff", &output);
    Ok(())
}

fn print_handoff_list(output: &HandoffListOutput) {
    println!("Guardrails handoffs");
    println!("repo_root={}", output.repo_root);
    if output.handoffs.is_empty() {
        println!("handoffs=none");
        return;
    }

    for handoff in &output.handoffs {
        println!(
            "{} {} {} {}",
            handoff.id,
            handoff_status_text(handoff.status),
            handoff.task_ids.len(),
            handoff.slug
        );
    }
}

fn handoff_status_text(status: HandoffStatus) -> &'static str {
    match status {
        HandoffStatus::Open => "open",
        HandoffStatus::Closed => "closed",
    }
}

fn print_handoff_mutation(label: &str, output: &HandoffGetOutput) {
    println!("{label} {}", output.handoff.summary.id);
    println!(
        "path={}",
        Path::new(&output.repo_root)
            .join(&output.handoff.summary.path)
            .display()
    );
}
