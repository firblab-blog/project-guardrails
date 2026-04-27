use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::{
    cli::{CiProvider, InitArgs, OutputFormat, UpgradeArgs},
    commands::init,
    config::GuardrailsConfig,
    output::JSON_SCHEMA_VERSION,
    profile::ResolvedProfile,
    profile_lock::ProfileLock,
};

pub fn run(args: UpgradeArgs) -> Result<()> {
    if args.plan && args.apply {
        bail!("choose either --plan or --apply, not both");
    }

    if !args.plan && !args.apply {
        bail!("upgrade requires either --plan or --apply");
    }

    let (repo_root, current) = GuardrailsConfig::load_from_repo(&args.target)?;

    let target_profile_name = args
        .profile
        .clone()
        .unwrap_or_else(|| current.profile.clone());
    let target_ci = args
        .ci
        .as_ref()
        .map(|ci| ci.as_str().to_string())
        .unwrap_or_else(|| current.ci.provider.clone());
    let target_profile = ResolvedProfile::load(&target_profile_name, args.profile_path.as_deref())?;
    let target_config = build_target_config(&repo_root, &target_profile, &target_ci)?;
    let target_managed_paths =
        init::collect_managed_paths(&target_profile, &target_config, &target_profile.profile)?;

    let installed_profile_lock = read_profile_lock(&repo_root)?;
    let installed_managed_paths = installed_profile_lock.managed_path_strings();
    let stale_paths =
        collect_stale_paths(&repo_root, &installed_managed_paths, &target_managed_paths);
    let removable_stale_paths = installed_profile_lock.removable_stale_paths(&stale_paths);
    let preserved_stale_paths = installed_profile_lock.preserved_stale_paths(&stale_paths);
    let review_stale_paths =
        collect_review_stale_paths(&stale_paths, &removable_stale_paths, &preserved_stale_paths);
    let summary = build_upgrade_summary(UpgradeSummaryInput {
        repo_root: &repo_root,
        current: &current,
        target_profile: &target_profile,
        target_ci: &target_ci,
        stale_paths: &stale_paths,
        removable_stale_paths: &removable_stale_paths,
        preserved_stale_paths: &preserved_stale_paths,
        review_stale_paths: &review_stale_paths,
    });

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_upgrade_summary(&summary);
    }

    if args.plan {
        return Ok(());
    }

    println!("upgrade_action=apply");
    remove_stale_paths(&repo_root, &removable_stale_paths)?;
    init::run(InitArgs {
        target: repo_root,
        profile: target_profile.profile.name.clone(),
        profile_path: args.profile_path,
        ci: Some(args.ci.unwrap_or(match target_ci.as_str() {
            "gitlab" => CiProvider::Gitlab,
            "none" => CiProvider::None,
            _ => CiProvider::Github,
        })),
        dry_run: false,
        force: true,
    })?;

    Ok(())
}

struct UpgradeSummaryInput<'a> {
    repo_root: &'a Path,
    current: &'a GuardrailsConfig,
    target_profile: &'a ResolvedProfile,
    target_ci: &'a str,
    stale_paths: &'a [String],
    removable_stale_paths: &'a [String],
    preserved_stale_paths: &'a [String],
    review_stale_paths: &'a [String],
}

fn build_upgrade_summary(input: UpgradeSummaryInput<'_>) -> UpgradeSummary {
    let UpgradeSummaryInput {
        repo_root,
        current,
        target_profile,
        target_ci,
        stale_paths,
        removable_stale_paths,
        preserved_stale_paths,
        review_stale_paths,
    } = input;

    UpgradeSummary {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        current: UpgradeState {
            profile: current.profile.clone(),
            profile_source: current.profile_source.clone(),
            profile_schema_version: current.profile_schema_version,
            installed_by_version: current.installed_by_version.clone(),
            ci_provider: current.ci.provider.clone(),
        },
        target: UpgradeState {
            profile: target_profile.profile.name.clone(),
            profile_source: target_profile.source.clone(),
            profile_schema_version: target_profile.profile.schema_version,
            installed_by_version: env!("CARGO_PKG_VERSION").to_string(),
            ci_provider: target_ci.to_string(),
        },
        changes: vec![
            build_change("profile", &current.profile, &target_profile.profile.name),
            build_change(
                "profile_source",
                &current.profile_source,
                &target_profile.source,
            ),
            build_change(
                "profile_schema_version",
                &current.profile_schema_version.to_string(),
                &target_profile.profile.schema_version.to_string(),
            ),
            build_change("ci_provider", &current.ci.provider, target_ci),
            build_change(
                "installer_version",
                &current.installed_by_version,
                env!("CARGO_PKG_VERSION"),
            ),
        ],
        stale_paths: stale_paths.to_vec(),
        removable_stale_paths: removable_stale_paths.to_vec(),
        preserved_stale_paths: preserved_stale_paths.to_vec(),
        review_stale_paths: review_stale_paths.to_vec(),
        planned_actions: vec![
            String::from("reapply the selected profile against the target repo"),
            if stale_paths.is_empty() {
                String::from("no stale managed files are scheduled for removal")
            } else {
                format!(
                    "remove {} stale managed file(s) before reapplying",
                    removable_stale_paths.len()
                )
            },
            if preserved_stale_paths.is_empty() {
                String::from("no durable state paths are marked preserve in this upgrade")
            } else {
                format!(
                    "preserve {} durable state path(s) during the reapply flow",
                    preserved_stale_paths.len()
                )
            },
            if review_stale_paths.is_empty() {
                String::from("no stale managed files require manual review")
            } else {
                format!(
                    "review {} stale managed file(s) that are not auto-removed",
                    review_stale_paths.len()
                )
            },
            String::from(
                "review `.guardrails/guardrails.toml` and `.guardrails/profile.lock` changes",
            ),
            String::from("review generated docs, assets, and CI template changes"),
            String::from("rerun `project-guardrails doctor` and `project-guardrails check`"),
        ],
    }
}

fn print_upgrade_summary(summary: &UpgradeSummary) {
    println!("Guardrails upgrade plan");
    println!("repo_root={}", summary.repo_root);
    println!("current.profile={}", summary.current.profile);
    println!("current.profile_source={}", summary.current.profile_source);
    println!(
        "current.profile_schema_version={}",
        summary.current.profile_schema_version
    );
    println!(
        "current.installed_by_version={}",
        summary.current.installed_by_version
    );
    println!("target.profile={}", summary.target.profile);
    println!("target.profile_source={}", summary.target.profile_source);
    println!(
        "target.profile_schema_version={}",
        summary.target.profile_schema_version
    );
    println!("target.ci_provider={}", summary.target.ci_provider);
    println!(
        "target.installer_version={}",
        summary.target.installed_by_version
    );

    for change in &summary.changes {
        if change.changed {
            println!(
                "change.{}={} -> {}",
                change.field, change.current, change.target
            );
        } else {
            println!("change.{}=unchanged ({})", change.field, change.current);
        }
    }

    if summary.stale_paths.is_empty() {
        println!("stale_paths=none");
    } else {
        println!("stale_paths:");
        for path in &summary.stale_paths {
            println!("  - {}", path);
        }
    }

    if summary.removable_stale_paths.is_empty() {
        println!("removable_stale_paths=none");
    } else {
        println!("removable_stale_paths:");
        for path in &summary.removable_stale_paths {
            println!("  - {}", path);
        }
    }

    if summary.preserved_stale_paths.is_empty() {
        println!("preserved_stale_paths=none");
    } else {
        println!("preserved_stale_paths:");
        for path in &summary.preserved_stale_paths {
            println!("  - {}", path);
        }
    }

    if summary.review_stale_paths.is_empty() {
        println!("review_stale_paths=none");
    } else {
        println!("review_stale_paths:");
        for path in &summary.review_stale_paths {
            println!("  - {}", path);
        }
    }

    println!("planned_actions:");
    for action in &summary.planned_actions {
        println!("  - {}", action);
    }
}

fn build_target_config(
    target_root: &Path,
    target_profile: &ResolvedProfile,
    target_ci: &str,
) -> Result<GuardrailsConfig> {
    let mut config = GuardrailsConfig {
        version: 1,
        profile: target_profile.profile.name.clone(),
        profile_source: target_profile.source.clone(),
        profile_schema_version: target_profile.profile.schema_version,
        installed_by_version: env!("CARGO_PKG_VERSION").to_string(),
        project: crate::config::ProjectConfig {
            name: target_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("project")
                .to_string(),
            root_markers: target_profile.profile.root_markers.clone(),
        },
        docs: crate::config::DocsConfig {
            enabled: target_profile.profile.docs_enabled,
            required: target_profile.profile.required_docs.clone(),
        },
        rules: crate::config::RulesConfig {
            required_files: target_profile.profile.required_files.clone(),
            forbidden_dirs: target_profile.profile.forbidden_dirs.clone(),
            rule_packs: crate::config::RulePackSelectionConfig {
                enabled: target_profile.profile.rule_packs.default_enabled.clone(),
            },
            task_references: target_profile.profile.task_references.clone(),
            link_requirements: target_profile.profile.link_requirements.clone(),
            evidence_requirements: target_profile.profile.evidence_requirements.clone(),
            forbidden_patterns: target_profile.profile.forbidden_patterns.clone(),
        },
        ci: crate::config::CiConfig {
            provider: target_ci.to_string(),
            workflow_path: target_profile.workflow_path_for_provider(target_ci),
        },
        engines: crate::config::EnginesConfig {
            semgrep: target_profile.profile.semgrep.clone(),
            conftest: target_profile.profile.conftest.clone(),
        },
    };
    target_profile.apply_rule_packs(&mut config)?;
    Ok(config)
}

fn read_profile_lock(repo_root: &Path) -> Result<ProfileLock> {
    let lock_path = repo_root.join(".guardrails/profile.lock");
    if !lock_path.exists() {
        return Ok(ProfileLock {
            version: 2,
            profile: String::new(),
            profile_source: String::new(),
            profile_schema_version: 0,
            config_version: 0,
            installed_by_version: String::new(),
            managed_paths: Vec::new(),
        });
    }

    ProfileLock::load(&lock_path)
        .with_context(|| format!("failed to parse {}", lock_path.display()))
}

fn collect_stale_paths(
    repo_root: &Path,
    installed_managed_paths: &[String],
    target_managed_paths: &[crate::profile_lock::ManagedPathEntry],
) -> Vec<String> {
    let target_managed_paths = target_managed_paths
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();

    let mut stale_paths = Vec::new();

    for path in installed_managed_paths {
        if target_managed_paths.contains(&path.as_str()) {
            continue;
        }
        if repo_root.join(path).exists() {
            stale_paths.push(path.clone());
        }
    }

    stale_paths.sort();
    stale_paths.dedup();
    stale_paths
}

fn collect_review_stale_paths(
    stale_paths: &[String],
    removable_stale_paths: &[String],
    preserved_stale_paths: &[String],
) -> Vec<String> {
    stale_paths
        .iter()
        .filter(|path| {
            !removable_stale_paths.contains(path) && !preserved_stale_paths.contains(path)
        })
        .cloned()
        .collect()
}

fn remove_stale_paths(repo_root: &Path, stale_paths: &[String]) -> Result<()> {
    for relative_path in stale_paths {
        let path = repo_root.join(relative_path);
        fs::remove_file(&path)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .with_context(|| format!("failed to remove stale managed file {}", path.display()))?;
    }

    Ok(())
}

fn build_change(field: &'static str, current: &str, target: &str) -> UpgradeChange {
    UpgradeChange {
        field,
        current: current.to_string(),
        target: target.to_string(),
        changed: current != target,
    }
}

#[derive(Debug, Serialize)]
struct UpgradeSummary {
    schema_version: u32,
    repo_root: String,
    current: UpgradeState,
    target: UpgradeState,
    changes: Vec<UpgradeChange>,
    stale_paths: Vec<String>,
    removable_stale_paths: Vec<String>,
    preserved_stale_paths: Vec<String>,
    review_stale_paths: Vec<String>,
    planned_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UpgradeState {
    profile: String,
    profile_source: String,
    profile_schema_version: u32,
    installed_by_version: String,
    ci_provider: String,
}

#[derive(Debug, Serialize)]
struct UpgradeChange {
    field: &'static str,
    current: String,
    target: String,
    changed: bool,
}
