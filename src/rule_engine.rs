use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};

use crate::{
    config::{ConftestEngineConfig, GuardrailsConfig, SemgrepEngineConfig},
    diagnostics::Diagnostic,
};

pub fn run_external_engines(repo_root: &Path, config: &GuardrailsConfig) -> Result<()> {
    if config.engines.semgrep.enabled {
        run_semgrep(repo_root, &config.engines.semgrep)?;
    }

    if config.engines.conftest.enabled {
        run_conftest(repo_root, &config.engines.conftest)?;
    }

    Ok(())
}

pub fn diagnose_external_engines(repo_root: &Path, config: &GuardrailsConfig) -> Vec<Diagnostic> {
    let mut failures = Vec::new();

    if config.engines.semgrep.enabled {
        diagnose_semgrep(repo_root, &config.engines.semgrep, &mut failures);
    }

    if config.engines.conftest.enabled {
        diagnose_conftest(repo_root, &config.engines.conftest, &mut failures);
    }

    failures
}

fn run_semgrep(repo_root: &Path, config: &SemgrepEngineConfig) -> Result<()> {
    if config.config_paths.is_empty() {
        bail!("semgrep is enabled but no config_paths were configured");
    }

    ensure_binary_available(&config.binary)?;

    let mut command = Command::new(&config.binary);
    command.arg("scan");
    for config_path in &config.config_paths {
        command.arg("--config");
        command.arg(repo_root.join(config_path));
    }
    for arg in &config.extra_args {
        command.arg(arg);
    }
    command.arg(repo_root);

    run_command(command, "semgrep")
}

fn run_conftest(repo_root: &Path, config: &ConftestEngineConfig) -> Result<()> {
    if config.policy_paths.is_empty() {
        bail!("conftest is enabled but no policy_paths were configured");
    }

    ensure_binary_available(&config.binary)?;

    let mut command = Command::new(&config.binary);
    command.arg("test");
    command.arg(repo_root);
    for policy_path in &config.policy_paths {
        command.arg("--policy");
        command.arg(repo_root.join(policy_path));
    }
    for arg in &config.extra_args {
        command.arg(arg);
    }

    run_command(command, "conftest")
}

fn diagnose_semgrep(
    repo_root: &Path,
    config: &SemgrepEngineConfig,
    failures: &mut Vec<Diagnostic>,
) {
    if config.config_paths.is_empty() {
        failures.push(Diagnostic::new(
            "semgrep_config_missing",
            "semgrep is enabled but no config_paths were configured",
        ));
    }

    for config_path in &config.config_paths {
        let path = repo_root.join(config_path);
        if !path.exists() {
            failures.push(Diagnostic::new(
                "semgrep_path_missing",
                format!("semgrep config path missing: {}", config_path),
            ));
        }
    }

    if let Err(error) = ensure_binary_available(&config.binary) {
        failures.push(Diagnostic::new("semgrep_binary_error", error.to_string()));
    }
}

fn diagnose_conftest(
    repo_root: &Path,
    config: &ConftestEngineConfig,
    failures: &mut Vec<Diagnostic>,
) {
    if config.policy_paths.is_empty() {
        failures.push(Diagnostic::new(
            "conftest_policy_missing",
            "conftest is enabled but no policy_paths were configured",
        ));
    }

    for policy_path in &config.policy_paths {
        let path = repo_root.join(policy_path);
        if !path.exists() {
            failures.push(Diagnostic::new(
                "conftest_path_missing",
                format!("conftest policy path missing: {}", policy_path),
            ));
        }
    }

    if let Err(error) = ensure_binary_available(&config.binary) {
        failures.push(Diagnostic::new("conftest_binary_error", error.to_string()));
    }
}

fn ensure_binary_available(binary: &str) -> Result<()> {
    let status = Command::new(binary)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("failed to execute {binary}; is it installed?"))?;

    if status.success() {
        return Ok(());
    }

    bail!("binary {binary} is present but did not respond successfully to --version")
}

fn run_command(mut command: Command, label: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to execute {label}"))?;

    if status.success() {
        return Ok(());
    }

    bail!("{label} checks failed")
}
