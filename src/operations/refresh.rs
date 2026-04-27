use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    config::GuardrailsConfig,
    diagnostics::Diagnostic,
    managed_block::{
        ParsedManagedBlock, parse_managed_blocks, render_declared_block, sha256_text,
        upsert_managed_block,
    },
    output::JSON_SCHEMA_VERSION,
    profile::{ManagedBlockConfig, ResolvedProfile},
};

pub fn refresh(target: &Path, check: bool) -> Result<RefreshOutput> {
    let (repo_root, config) = GuardrailsConfig::load_from_repo(target)?;
    let profile = ResolvedProfile::load_from_config(&config)?;
    refresh_managed_blocks(&repo_root, &config, &profile, check)
}

pub fn refresh_managed_blocks(
    repo_root: &Path,
    config: &GuardrailsConfig,
    profile: &ResolvedProfile,
    check: bool,
) -> Result<RefreshOutput> {
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    let mut changed_paths = BTreeSet::new();

    for (relative_path, specs) in managed_blocks_by_path(profile) {
        refresh_path(
            repo_root,
            config,
            check,
            &relative_path,
            &specs,
            &mut blocks,
            &mut diagnostics,
            &mut changed_paths,
        )?;
    }

    let changed_paths = changed_paths.into_iter().collect::<Vec<_>>();
    let changed = !changed_paths.is_empty();
    let ok = diagnostics.is_empty() && !(check && changed);

    Ok(RefreshOutput {
        schema_version: JSON_SCHEMA_VERSION,
        ok,
        repo_root: repo_root.display().to_string(),
        check,
        changed,
        changed_paths,
        blocks,
        diagnostics,
    })
}

#[allow(clippy::too_many_arguments)]
fn refresh_path(
    repo_root: &Path,
    config: &GuardrailsConfig,
    check: bool,
    relative_path: &str,
    specs: &[ManagedBlockConfig],
    blocks: &mut Vec<RefreshBlock>,
    diagnostics: &mut Vec<Diagnostic>,
    changed_paths: &mut BTreeSet<String>,
) -> Result<()> {
    let path = repo_root.join(relative_path);
    let Some(contents) = read_refresh_target(&path, relative_path, specs, blocks, diagnostics)
    else {
        return Ok(());
    };
    let Some(parsed) = parse_refresh_target(&contents, relative_path, specs, blocks, diagnostics)
    else {
        return Ok(());
    };

    let mut updated = contents.clone();
    let mut path_changed = false;
    for spec in specs {
        if refresh_declared_block(
            repo_root,
            config,
            check,
            relative_path,
            &parsed,
            spec,
            blocks,
            diagnostics,
            &mut updated,
        )? {
            path_changed = true;
        }
    }

    write_refreshed_target(
        &path,
        check,
        relative_path,
        path_changed,
        updated,
        changed_paths,
    )
}

fn read_refresh_target(
    path: &Path,
    relative_path: &str,
    specs: &[ManagedBlockConfig],
    blocks: &mut Vec<RefreshBlock>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    if !path.exists() {
        push_all_blocks(blocks, specs, RefreshStatus::MissingFile);
        diagnostics.push(Diagnostic::new(
            "managed_block_file_missing",
            format!(
                "{} is missing; refresh only inserts declared blocks into existing files",
                relative_path
            ),
        ));
        return None;
    }

    match fs::read_to_string(path) {
        Ok(contents) => Some(contents),
        Err(error) => {
            push_all_blocks(blocks, specs, RefreshStatus::Error);
            diagnostics.push(Diagnostic::new(
                "managed_block_unreadable",
                format!("failed to read {} for refresh: {}", relative_path, error),
            ));
            None
        }
    }
}

fn parse_refresh_target(
    contents: &str,
    relative_path: &str,
    specs: &[ManagedBlockConfig],
    blocks: &mut Vec<RefreshBlock>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<ParsedManagedBlock>> {
    match parse_managed_blocks(contents) {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            push_all_blocks(blocks, specs, RefreshStatus::Invalid);
            diagnostics.push(Diagnostic::new(
                "managed_block_invalid",
                format!(
                    "{} has invalid managed block markup: {}",
                    relative_path, error
                ),
            ));
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_declared_block(
    repo_root: &Path,
    config: &GuardrailsConfig,
    check: bool,
    relative_path: &str,
    parsed: &[ParsedManagedBlock],
    spec: &ManagedBlockConfig,
    blocks: &mut Vec<RefreshBlock>,
    diagnostics: &mut Vec<Diagnostic>,
    updated: &mut String,
) -> Result<bool> {
    let expected = match render_declared_block(repo_root, config, spec) {
        Ok(expected) => expected,
        Err(error) => {
            blocks.push(refresh_block(spec, RefreshStatus::Error));
            diagnostics.push(Diagnostic::new(
                "managed_block_generator_error",
                format!(
                    "failed to render managed block `{}` for {}: {}",
                    spec.id, relative_path, error
                ),
            ));
            return Ok(false);
        }
    };

    if !managed_block_changed(parsed, spec, &expected.content) {
        blocks.push(refresh_block(spec, RefreshStatus::Unchanged));
        return Ok(false);
    }

    *updated = upsert_managed_block(updated, &expected, spec.placement)
        .with_context(|| format!("failed to update managed block `{}`", spec.id))?;
    blocks.push(refresh_block(
        spec,
        if check {
            RefreshStatus::WouldChange
        } else {
            RefreshStatus::Changed
        },
    ));

    Ok(true)
}

fn managed_block_changed(
    parsed: &[ParsedManagedBlock],
    spec: &ManagedBlockConfig,
    expected_content: &str,
) -> bool {
    parsed
        .iter()
        .find(|block| block.id == spec.id)
        .map(|block| sha256_text(&block.content) != sha256_text(expected_content))
        .unwrap_or(true)
}

fn write_refreshed_target(
    path: &Path,
    check: bool,
    relative_path: &str,
    path_changed: bool,
    updated: String,
    changed_paths: &mut BTreeSet<String>,
) -> Result<()> {
    if !path_changed {
        return Ok(());
    }

    changed_paths.insert(relative_path.to_string());
    if !check {
        fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

fn push_all_blocks(
    blocks: &mut Vec<RefreshBlock>,
    specs: &[ManagedBlockConfig],
    status: RefreshStatus,
) {
    blocks.extend(specs.iter().map(|spec| refresh_block(spec, status)));
}

fn managed_blocks_by_path(profile: &ResolvedProfile) -> BTreeMap<String, Vec<ManagedBlockConfig>> {
    let mut by_path = BTreeMap::new();
    for spec in &profile.profile.managed_blocks {
        by_path
            .entry(spec.path.clone())
            .or_insert_with(Vec::new)
            .push(spec.clone());
    }
    by_path
}

fn refresh_block(spec: &ManagedBlockConfig, status: RefreshStatus) -> RefreshBlock {
    RefreshBlock {
        path: spec.path.clone(),
        id: spec.id.clone(),
        generator: spec.generator.clone(),
        status,
    }
}

#[derive(Debug, Serialize)]
pub struct RefreshOutput {
    pub schema_version: u32,
    pub ok: bool,
    pub repo_root: String,
    pub check: bool,
    pub changed: bool,
    pub changed_paths: Vec<String>,
    pub blocks: Vec<RefreshBlock>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct RefreshBlock {
    pub path: String,
    pub id: String,
    pub generator: String,
    pub status: RefreshStatus,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshStatus {
    Unchanged,
    Changed,
    WouldChange,
    MissingFile,
    Invalid,
    Error,
}

impl RefreshStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Changed => "changed",
            Self::WouldChange => "would_change",
            Self::MissingFile => "missing_file",
            Self::Invalid => "invalid",
            Self::Error => "error",
        }
    }
}
