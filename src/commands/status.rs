use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{OutputFormat, TargetArgs},
    config::GuardrailsConfig,
    output::JSON_SCHEMA_VERSION,
};

pub fn run(args: TargetArgs) -> Result<()> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(&args.target)?;
    let output = StatusOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        profile: config.profile,
        profile_source: config.profile_source,
        profile_schema_version: config.profile_schema_version,
        installed_by_version: config.installed_by_version,
        docs_enabled: config.docs.enabled,
        ci_provider: config.ci.provider,
        required_files: config.rules.required_files,
        forbidden_dirs: config.rules.forbidden_dirs,
        semgrep_enabled: config.engines.semgrep.enabled,
        conftest_enabled: config.engines.conftest.enabled,
    };

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Guardrails status");
    println!("repo_root={}", output.repo_root);
    println!("profile={}", output.profile);
    println!("profile_source={}", output.profile_source);
    println!("profile_schema_version={}", output.profile_schema_version);
    println!("installed_by_version={}", output.installed_by_version);
    println!("docs_enabled={}", output.docs_enabled);
    println!("ci_provider={}", output.ci_provider);
    println!("required_files={}", output.required_files.join(", "));
    println!("forbidden_dirs={}", output.forbidden_dirs.join(", "));
    println!("semgrep_enabled={}", output.semgrep_enabled);
    println!("conftest_enabled={}", output.conftest_enabled);
    Ok(())
}

#[derive(Debug, Serialize)]
struct StatusOutput {
    schema_version: u32,
    repo_root: String,
    profile: String,
    profile_source: String,
    profile_schema_version: u32,
    installed_by_version: String,
    docs_enabled: bool,
    ci_provider: String,
    required_files: Vec<String>,
    forbidden_dirs: Vec<String>,
    semgrep_enabled: bool,
    conftest_enabled: bool,
}
