use std::fs;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{CommitMsgCheckArgs, OutputFormat},
    config::GuardrailsConfig,
    diagnostics::Diagnostic,
    enforcement::collect_commit_msg_diagnostics,
    output::JSON_SCHEMA_VERSION,
};

pub fn run(args: CommitMsgCheckArgs) -> Result<()> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(&args.target.target)?;
    let message = fs::read_to_string(&args.message_file)?;
    let (task_ids, report) = collect_commit_msg_diagnostics(&repo_root, &config, &message)?;

    if !report.is_empty() {
        if matches!(args.target.format, OutputFormat::Json) {
            println!(
                "{}",
                serde_json::to_string_pretty(&CommitMsgCheckOutput {
                    schema_version: JSON_SCHEMA_VERSION,
                    ok: false,
                    repo_root: repo_root.display().to_string(),
                    task_ids,
                    diagnostics: report.diagnostics().to_vec(),
                })?
            );
        } else {
            report.print_stderr();
        }
        bail!("commit message checks failed");
    }

    if matches!(args.target.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&CommitMsgCheckOutput {
                schema_version: JSON_SCHEMA_VERSION,
                ok: true,
                repo_root: repo_root.display().to_string(),
                task_ids,
                diagnostics: Vec::new(),
            })?
        );
    } else {
        println!("Commit message checks passed.");
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct CommitMsgCheckOutput {
    schema_version: u32,
    ok: bool,
    repo_root: String,
    task_ids: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}
