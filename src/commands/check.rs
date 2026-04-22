use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{OutputFormat, TargetArgs},
    config::GuardrailsConfig,
    diagnostics::{Diagnostic, collect_check_diagnostics},
    output::JSON_SCHEMA_VERSION,
    rule_engine,
};

pub fn run(args: TargetArgs) -> Result<()> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(&args.target)?;
    let report = collect_check_diagnostics(&repo_root, &config);

    if !report.is_empty() {
        if matches!(args.format, OutputFormat::Json) {
            println!(
                "{}",
                serde_json::to_string_pretty(&CheckOutput {
                    schema_version: JSON_SCHEMA_VERSION,
                    ok: false,
                    repo_root: repo_root.display().to_string(),
                    diagnostics: report.diagnostics().to_vec(),
                })?
            );
        } else {
            report.print_stderr();
        }
        bail!("guardrails checks failed");
    }

    rule_engine::run_external_engines(&repo_root, &config)?;
    if matches!(args.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&CheckOutput {
                schema_version: JSON_SCHEMA_VERSION,
                ok: true,
                repo_root: repo_root.display().to_string(),
                diagnostics: Vec::new(),
            })?
        );
    } else {
        println!("All configured local checks passed.");
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct CheckOutput {
    schema_version: u32,
    ok: bool,
    repo_root: String,
    diagnostics: Vec<Diagnostic>,
}
