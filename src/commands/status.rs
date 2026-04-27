use anyhow::Result;

use crate::{
    cli::{OutputFormat, StatusArgs},
    operations::status::{build_llm_status, build_status},
};

pub fn run(args: StatusArgs) -> Result<()> {
    if args.for_llm {
        let output = build_llm_status(&args.target.target)?;
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let output = build_status(&args.target.target)?;

    if matches!(args.target.format, OutputFormat::Json) {
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
