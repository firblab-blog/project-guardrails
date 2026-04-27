use anyhow::Result;

use crate::{
    cli::{OutputFormat, TargetArgs},
    operations::brief::{BriefOutput, build_brief},
    state::{handoffs::HandoffStatus, tasks::TaskPriority},
};

pub fn run(args: TargetArgs) -> Result<()> {
    let output = build_brief(&args.target)?;

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_brief(&output);
    }

    Ok(())
}

fn print_brief(output: &BriefOutput) {
    println!("Guardrails brief");
    println!("repo_root={}", output.repo_root);
    println!(
        "profile={} ({})",
        output.summary.profile.name, output.summary.profile.source
    );
    println!("generated_at={}", output.generated_at);
    println!(
        "doctor_status={}",
        if output.summary.doctor.ok {
            "ok"
        } else {
            "issues"
        }
    );

    println!("required_reading:");
    for item in &output.summary.required_reading {
        println!(
            "- {} {} {}",
            item.topic,
            if item.exists { "ok" } else { "missing" },
            item.path
        );
    }

    if output.summary.active_tasks.is_empty() {
        println!("active_tasks=none");
    } else {
        println!("active_tasks:");
        for task in &output.summary.active_tasks {
            println!(
                "- {} {} {} {} {} {}",
                task.id,
                task.status.as_str(),
                priority_text(task.priority),
                task.owner.as_deref().unwrap_or("unowned"),
                task.title,
                task.path
            );
        }
    }

    if let Some(handoff) = &output.summary.latest_handoff {
        println!("latest_handoff:");
        println!(
            "- {} {} {} {}",
            handoff.id,
            handoff_status_text(handoff.status),
            handoff.updated,
            handoff.path
        );
    } else {
        println!("latest_handoff=none");
    }

    if output.summary.doctor.diagnostics.is_empty() {
        println!("diagnostics=none");
    } else {
        println!("diagnostics:");
        for diagnostic in &output.summary.doctor.diagnostics {
            println!("- {} {}", diagnostic.code, diagnostic.message);
        }
    }

    println!("next_commands:");
    for command in &output.brief.recommended_commands {
        println!("- {}", command);
    }
}

fn priority_text(priority: Option<TaskPriority>) -> &'static str {
    priority.map(TaskPriority::as_str).unwrap_or("no_priority")
}

fn handoff_status_text(status: HandoffStatus) -> &'static str {
    match status {
        HandoffStatus::Open => "open",
        HandoffStatus::Closed => "closed",
    }
}
