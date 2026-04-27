use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{OutputFormat, TargetArgs},
    config::GuardrailsConfig,
    diagnostics::Diagnostic,
    enforcement::collect_pre_commit_diagnostics,
    output::JSON_SCHEMA_VERSION,
};

pub fn run(args: TargetArgs) -> Result<()> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(&args.target)?;
    let (staged_paths, report) = collect_pre_commit_diagnostics(&repo_root, &config)?;

    if !report.is_empty() {
        if matches!(args.format, OutputFormat::Json) {
            println!(
                "{}",
                serde_json::to_string_pretty(&PreCommitOutput {
                    schema_version: JSON_SCHEMA_VERSION,
                    ok: false,
                    repo_root: repo_root.display().to_string(),
                    staged_paths,
                    diagnostics: report.diagnostics().to_vec(),
                })?
            );
        } else {
            report.print_stderr();
        }
        bail!("pre-commit checks failed");
    }

    if matches!(args.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&PreCommitOutput {
                schema_version: JSON_SCHEMA_VERSION,
                ok: true,
                repo_root: repo_root.display().to_string(),
                staged_paths,
                diagnostics: Vec::new(),
            })?
        );
    } else {
        println!("Pre-commit checks passed.");
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct PreCommitOutput {
    schema_version: u32,
    ok: bool,
    repo_root: String,
    staged_paths: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}
