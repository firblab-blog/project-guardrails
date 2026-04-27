use anyhow::{Result, bail};

use crate::{
    cli::{OutputFormat, TargetArgs},
    diagnostics::DiagnosticReport,
    operations::timeline::TimelineOutput,
};

pub fn run(args: TargetArgs) -> Result<()> {
    let output = crate::operations::timeline::build_timeline(&args.target)?;

    if !output.diagnostics.is_empty() {
        if matches!(args.format, OutputFormat::Json) {
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            output_diagnostics(&output).print_stderr();
        }
        bail!("timeline state is invalid");
    }

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_timeline(&output);
    }

    Ok(())
}

fn output_diagnostics(output: &TimelineOutput) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();
    report.extend(output.diagnostics.clone());
    report
}

fn print_timeline(output: &TimelineOutput) {
    println!("Guardrails timeline");
    println!("repo_root={}", output.repo_root);
    println!("generated_at={}", output.generated_at);
    println!("source=.guardrails/state (not a complete audit log)");

    if output.events.is_empty() {
        println!("events=none (no repo-local guardrails state events found)");
        return;
    }

    println!("events:");
    for event in &output.events {
        let status = event
            .status
            .as_ref()
            .map(|value| format!(" status={value}"))
            .unwrap_or_default();
        let task_ids = if event.task_ids.is_empty() {
            String::new()
        } else {
            format!(" task_ids={}", event.task_ids.join(","))
        };
        println!(
            "- {} {} {} {}{} title=\"{}\" path={}{}",
            event.timestamp,
            event.kind.as_str(),
            event.action.as_str(),
            event.id,
            status,
            event.title.as_deref().unwrap_or("-"),
            event.path,
            task_ids
        );
    }
}
