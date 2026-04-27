use std::path::Path;

use anyhow::Result;

use crate::{
    cli::{OutputFormat, PreWorkArgs},
    operations::pre_work::record_pre_work,
};

pub fn run(args: PreWorkArgs) -> Result<()> {
    let output = record_pre_work(&args.target.target)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Guardrails pre-work");
    println!("run_id={}", output.run_id);
    println!(
        "run_path={}",
        Path::new(&output.summary.repo_root)
            .join(&output.run_path)
            .display()
    );
    println!("repo_root={}", output.summary.repo_root);
    println!(
        "profile={} ({})",
        output.summary.profile.name, output.summary.profile.source
    );
    println!(
        "doctor_status={}",
        if output.summary.doctor.ok {
            "ok"
        } else {
            "issues"
        }
    );

    if !output.summary.repo.forbidden_dirs.is_empty() {
        println!(
            "forbidden_dirs={}",
            output.summary.repo.forbidden_dirs.join(", ")
        );
    }

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
            println!("- {} {} {}", task.id, task.status.as_str(), task.path);
        }
    }

    if output.summary.recent_handoffs.is_empty() {
        println!("recent_handoffs=none");
    } else {
        println!("recent_handoffs:");
        for handoff in &output.summary.recent_handoffs {
            println!(
                "- {} {} {}",
                handoff.id,
                format!("{:?}", handoff.status).to_lowercase(),
                handoff.path
            );
        }
    }

    Ok(())
}
