use std::{collections::BTreeSet, fs};

use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{OutputFormat, TargetArgs},
    config::GuardrailsConfig,
    managed_block::{ManagedBlockPlacement, parse_managed_blocks},
    output::JSON_SCHEMA_VERSION,
    profile::{AdapterTargetConfig, ManagedBlockConfig, ResolvedProfile},
};

pub fn list(args: TargetArgs) -> Result<()> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(&args.target)?;
    let profile = ResolvedProfile::load_from_config(&config)?;
    let adapters = profile
        .profile
        .adapter_targets
        .iter()
        .map(|target| adapter_target_output(&repo_root, &profile, target))
        .collect::<Vec<_>>();

    let output = AdaptersListOutput {
        schema_version: JSON_SCHEMA_VERSION,
        repo_root: repo_root.display().to_string(),
        profile: config.profile,
        profile_source: config.profile_source,
        adapters,
    };

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_adapters_list(&output);
    }

    Ok(())
}

fn adapter_target_output(
    repo_root: &std::path::Path,
    profile: &ResolvedProfile,
    target: &AdapterTargetConfig,
) -> AdapterTargetOutput {
    let path = repo_root.join(&target.path);
    let declared_blocks = profile.managed_blocks_for_path(&target.path);
    let existing_block_ids = existing_block_ids(&path);

    AdapterTargetOutput {
        kind: target.kind.clone(),
        name: target.name.clone(),
        path: target.path.clone(),
        source_profile: profile.source.clone(),
        exists: path.exists(),
        managed_blocks: declared_blocks
            .into_iter()
            .map(|block| adapter_block_output(block, &existing_block_ids))
            .collect(),
    }
}

fn existing_block_ids(path: &std::path::Path) -> BTreeSet<String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let Ok(blocks) = parse_managed_blocks(&contents) else {
        return BTreeSet::new();
    };

    blocks.into_iter().map(|block| block.id).collect()
}

fn adapter_block_output(
    block: ManagedBlockConfig,
    existing_block_ids: &BTreeSet<String>,
) -> AdapterManagedBlockOutput {
    AdapterManagedBlockOutput {
        id: block.id.clone(),
        generator: block.generator,
        placement: block.placement,
        exists: existing_block_ids.contains(&block.id),
    }
}

fn print_adapters_list(output: &AdaptersListOutput) {
    println!("Guardrails adapters");
    println!("repo_root={}", output.repo_root);
    println!("profile={}", output.profile);
    println!("profile_source={}", output.profile_source);

    if output.adapters.is_empty() {
        println!("adapters=none");
        return;
    }

    println!("adapters:");
    for adapter in &output.adapters {
        println!(
            "  - {} name={} path={} exists={} source_profile={}",
            adapter.kind, adapter.name, adapter.path, adapter.exists, adapter.source_profile
        );
        if adapter.managed_blocks.is_empty() {
            println!("    managed_blocks=none");
        } else {
            println!("    managed_blocks:");
            for block in &adapter.managed_blocks {
                println!(
                    "      - id={} generator={} placement={} exists={}",
                    block.id,
                    block.generator,
                    placement_label(block.placement),
                    block.exists
                );
            }
        }
    }
}

fn placement_label(placement: ManagedBlockPlacement) -> &'static str {
    match placement {
        ManagedBlockPlacement::Prepend => "prepend",
        ManagedBlockPlacement::AfterFirstHeading => "after_first_heading",
    }
}

#[derive(Debug, Serialize)]
struct AdaptersListOutput {
    schema_version: u32,
    repo_root: String,
    profile: String,
    profile_source: String,
    adapters: Vec<AdapterTargetOutput>,
}

#[derive(Debug, Serialize)]
struct AdapterTargetOutput {
    kind: String,
    name: String,
    path: String,
    source_profile: String,
    exists: bool,
    managed_blocks: Vec<AdapterManagedBlockOutput>,
}

#[derive(Debug, Serialize)]
struct AdapterManagedBlockOutput {
    id: String,
    generator: String,
    placement: ManagedBlockPlacement,
    exists: bool,
}
