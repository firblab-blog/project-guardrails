use anyhow::{Result, bail};

use crate::{
    cli::{OutputFormat, TargetArgs},
    operations::doctor::{DoctorOutput, run_doctor},
};

pub fn run(args: TargetArgs) -> Result<()> {
    let output = run_doctor(&args.target)?;

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);

        if output.ok {
            return Ok(());
        }

        bail!("doctor found {} issue(s)", output.diagnostics.len());
    }

    print_doctor(&output);

    if output.ok {
        return Ok(());
    }

    for diagnostic in &output.diagnostics {
        eprintln!("[{}] {}", diagnostic.code, diagnostic.message);
    }

    bail!("doctor found {} issue(s)", output.diagnostics.len())
}

fn print_doctor(output: &DoctorOutput) {
    println!("Guardrails doctor");
    println!("repo_root={}", output.repo_root);
    println!("profile={}", output.profile);
    println!("profile_source={}", output.profile_source);
    println!("installed_by_version={}", output.installed_by_version);

    for status in &output.statuses {
        println!(
            "{}:{}={}",
            status.label,
            status.relative_path.display(),
            status.status
        );
    }

    println!("semgrep_engine={}", output.semgrep_engine);
    println!("conftest_engine={}", output.conftest_engine);

    if output.ok {
        println!("Doctor checks passed.");
    }
}
