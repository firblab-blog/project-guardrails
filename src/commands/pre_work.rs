use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{OutputFormat, PreWorkArgs},
    config::GuardrailsConfig,
    output::{JSON_SCHEMA_VERSION, LlmRepoSummary, build_llm_repo_summary},
    state::{ensure_state_layout, runs},
};

pub fn run(args: PreWorkArgs) -> Result<()> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    ensure_state_layout(&repo_root)?;

    let run_id = runs::generate_run_id(&repo_root);
    let summary = build_llm_repo_summary(&repo_root, &config)?;
    let run_path = runs::run_relative_path("pre-work", &run_id);
    let output = PreWorkOutput {
        schema_version: JSON_SCHEMA_VERSION,
        run_id,
        run_path,
        summary,
    };
    runs::write_json_run(&repo_root, "pre-work", &output.run_id, &output)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Guardrails pre-work");
    println!("run_id={}", output.run_id);
    println!("run_path={}", repo_root.join(&output.run_path).display());
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

#[derive(Debug, Serialize)]
struct PreWorkOutput {
    schema_version: u32,
    run_id: String,
    run_path: String,
    summary: LlmRepoSummary,
}
