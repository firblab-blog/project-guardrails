use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::{OutputFormat, TargetArgs},
    config::GuardrailsConfig,
    diagnostics::{Diagnostic, RepoCheckStatus, collect_doctor_diagnostics, collect_repo_statuses},
    output::JSON_SCHEMA_VERSION,
};

pub fn run(args: TargetArgs) -> Result<()> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(&args.target)?;
    let report = collect_doctor_diagnostics(&repo_root, &config);
    let statuses = collect_repo_statuses(&repo_root, &config);

    if matches!(args.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&DoctorOutput {
                schema_version: JSON_SCHEMA_VERSION,
                ok: report.is_empty(),
                repo_root: repo_root.display().to_string(),
                profile: config.profile.clone(),
                profile_source: config.profile_source.clone(),
                installed_by_version: config.installed_by_version.clone(),
                semgrep_engine: if config.engines.semgrep.enabled {
                    "enabled".to_string()
                } else {
                    "disabled".to_string()
                },
                conftest_engine: if config.engines.conftest.enabled {
                    "enabled".to_string()
                } else {
                    "disabled".to_string()
                },
                statuses,
                diagnostics: report.diagnostics().to_vec(),
            })?
        );

        if report.is_empty() {
            return Ok(());
        }

        bail!("doctor found {} issue(s)", report.len());
    }

    println!("Guardrails doctor");
    println!("repo_root={}", repo_root.display());
    println!("profile={}", config.profile);
    println!("profile_source={}", config.profile_source);
    println!("installed_by_version={}", config.installed_by_version);

    for status in statuses {
        println!(
            "{}:{}={}",
            status.label,
            status.relative_path.display(),
            status.status
        );
    }

    println!(
        "semgrep_engine={}",
        if config.engines.semgrep.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "conftest_engine={}",
        if config.engines.conftest.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );

    if report.is_empty() {
        println!("Doctor checks passed.");
        return Ok(());
    }

    report.print_stderr();

    bail!("doctor found {} issue(s)", report.len())
}

#[derive(Debug, Serialize)]
struct DoctorOutput {
    schema_version: u32,
    ok: bool,
    repo_root: String,
    profile: String,
    profile_source: String,
    installed_by_version: String,
    semgrep_engine: String,
    conftest_engine: String,
    statuses: Vec<RepoCheckStatus>,
    diagnostics: Vec<Diagnostic>,
}
