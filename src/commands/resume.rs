use anyhow::Result;

use crate::{
    cli::{OutputFormat, TargetArgs},
    git::{GitContinuity, GitContinuityStatus},
    operations::resume::ResumeOutput,
    state::{handoffs::HandoffStatus, tasks::TaskPriority},
};

pub fn run(args: TargetArgs) -> Result<()> {
    let output = crate::operations::resume::build_resume(&args.target)?;

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_resume(&output);
    }

    Ok(())
}

fn print_resume(output: &ResumeOutput) {
    println!("Guardrails resume");
    println!("repo_root={}", output.repo_root);
    println!("generated_at={}", output.generated_at);

    if let Some(handoff) = &output.latest_handoff {
        println!("latest_handoff:");
        println!(
            "- {} {} {} {}",
            handoff.summary.id,
            handoff_status_text(handoff.summary.status),
            handoff.summary.updated,
            handoff.summary.path
        );
        println!("handoff_body={}", handoff.body_path);
        if !handoff.body_excerpt.is_empty() {
            println!("handoff_excerpt:");
            for line in handoff.body_excerpt.lines() {
                println!("> {line}");
            }
        }
    } else {
        println!("latest_handoff=none");
    }

    if output.linked_active_tasks.is_empty() {
        println!("linked_active_tasks=none");
    } else {
        println!("linked_active_tasks:");
        for task in &output.linked_active_tasks {
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

    print_git_continuity(&output.git);

    println!(
        "doctor_status={}",
        if output.doctor.ok { "ok" } else { "issues" }
    );
    if output.doctor.diagnostics.is_empty() {
        println!("diagnostics=none");
    } else {
        println!("diagnostics:");
        for diagnostic in &output.doctor.diagnostics {
            println!("- {} {}", diagnostic.code, diagnostic.message);
        }
    }

    println!("next_step={}", output.next_step.command);
    println!("next_step_reason={}", output.next_step.reason);
}

fn print_git_continuity(git: &GitContinuity) {
    println!("git_status={}", git_status_text(git.status));
    if let Some(timestamp) = &git.handoff_timestamp {
        println!("git_handoff_timestamp={timestamp}");
    }
    if let Some(commit) = &git.baseline_commit {
        println!("git_baseline_commit={commit}");
    }

    if git.changed_since_handoff.is_empty() {
        println!("git_changed_since_handoff=none");
    } else {
        println!("git_changed_since_handoff:");
        for path in &git.changed_since_handoff {
            println!("- {path}");
        }
    }

    print_git_paths("git_staged", &git.staged_paths);
    print_git_paths("git_unstaged", &git.unstaged_paths);
    print_git_paths("git_untracked", &git.untracked_paths);

    if git.diagnostics.is_empty() {
        println!("git_diagnostics=none");
    } else {
        println!("git_diagnostics:");
        for diagnostic in &git.diagnostics {
            println!("- {} {}", diagnostic.code, diagnostic.message);
        }
    }
}

fn git_status_text(status: GitContinuityStatus) -> &'static str {
    match status {
        GitContinuityStatus::Available => "available",
        GitContinuityStatus::NoHandoff => "no_handoff",
        GitContinuityStatus::Unavailable => "unavailable",
        GitContinuityStatus::InsufficientBaseline => "insufficient_baseline",
    }
}

fn print_git_paths(label: &str, paths: &[String]) {
    if paths.is_empty() {
        println!("{label}=none");
        return;
    }

    println!("{label}:");
    for path in paths {
        println!("- {path}");
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
