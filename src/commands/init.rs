use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    cli::{CLI_DISPLAY_NAME, CiProvider, InitArgs},
    config::{
        CiConfig, DocsConfig, EnginesConfig, GuardrailsConfig, ProjectConfig,
        RulePackSelectionConfig, RulesConfig, write_config,
    },
    managed_block::{parse_managed_blocks, render_declared_block, upsert_managed_block},
    profile::{ResolvedProfile, built_in_profile_info},
    profile_lock::{
        ManagedBlockEntry, ManagedPathEntry, ProfileLock, default_managed_path_entry,
        installed_sha256_for_path,
    },
    state::{STATE_DIR, ensure_state_layout},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FullFileSyncOutcome {
    Written,
    Skipped,
    PreservedEdited,
}

pub fn run(args: InitArgs) -> Result<()> {
    let target = normalize_target(&args.target)?;
    let resolved_profile = ResolvedProfile::load(&args.profile, args.profile_path.as_deref())?;
    let profile = &resolved_profile.profile;
    let ci = resolve_ci_provider(args.ci.as_ref(), &profile.default_ci)?;
    let ci_provider = ci.as_str().to_string();

    let mut config = GuardrailsConfig {
        version: 1,
        profile: profile.name.clone(),
        profile_source: resolved_profile.source.clone(),
        profile_schema_version: profile.schema_version,
        installed_by_version: env!("CARGO_PKG_VERSION").to_string(),
        project: ProjectConfig {
            name: target
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("project")
                .to_string(),
            root_markers: profile.root_markers.clone(),
        },
        docs: DocsConfig {
            enabled: profile.docs_enabled,
            required: profile.required_docs.clone(),
        },
        rules: RulesConfig {
            required_files: profile.required_files.clone(),
            forbidden_dirs: profile.forbidden_dirs.clone(),
            rule_packs: RulePackSelectionConfig {
                enabled: profile.rule_packs.default_enabled.clone(),
            },
            task_references: profile.task_references.clone(),
            link_requirements: profile.link_requirements.clone(),
            evidence_requirements: profile.evidence_requirements.clone(),
            forbidden_patterns: profile.forbidden_patterns.clone(),
        },
        ci: CiConfig {
            provider: ci_provider.clone(),
            workflow_path: resolved_profile.workflow_path_for_provider(&ci_provider),
        },
        engines: EnginesConfig {
            semgrep: profile.semgrep.clone(),
            conftest: profile.conftest.clone(),
        },
    };
    resolved_profile.apply_rule_packs(&mut config)?;

    let reportable_paths = reportable_paths(&config, profile);
    let existing_before = existing_report_paths(&target, &reportable_paths);
    let existing_assets_before = existing_profile_asset_paths(&target, &resolved_profile)?;

    if args.dry_run {
        println!("Bootstrap plan for {}", target.display());
        println!("profile: {}", profile.name);
        println!(
            "profile_summary: {}",
            built_in_profile_summary(&profile.name, &profile.description)
        );
        println!("profile_source: {}", resolved_profile.source);
        println!("description: {}", profile.description);
        println!("profile_default_ci: {}", profile.default_ci);
        println!("ci: {}", ci_provider);
        println!("profile_choice: {}", profile_choice_guidance(&profile.name));
        println!("ci_choice: {}", ci_choice_guidance(&ci_provider));
        println!("tool_managed:");
        for note in ownership_notes(&config, profile, &resolved_profile) {
            println!("  - {}: {}", note.label, note.detail);
        }
        println!("edit_first:");
        for suggestion in recommended_edit_paths(profile) {
            println!("  - {}: {}", suggestion.path, suggestion.reason);
        }
        println!("run_next:");
        for command in recommended_next_commands(&target, profile) {
            println!("  - {}", command);
        }
        println!("planned_files:");
        for path in created_paths(
            &target,
            &config,
            profile,
            &resolved_profile,
            &reportable_paths,
        ) {
            println!("  - {}", path);
        }
        return Ok(());
    }

    ensure_parent_dir(&target.join(".guardrails/guardrails.toml"))?;
    ensure_parent_dir(&target.join("docs/project/implementation-tracker.md"))?;
    ensure_state_layout(&target)?;

    let config_path = target.join(".guardrails/guardrails.toml");
    if config_path.exists() && !args.force {
        bail!(
            "{} already exists; rerun with --force to overwrite",
            config_path.display()
        );
    }

    let previous_profile_lock = load_existing_profile_lock(&target)?;
    let mut preserved_prior_hashes = HashSet::new();

    write_config(&config_path, &config)?;
    {
        let mut template_sync = ProfileTemplateSync {
            repo_root: &target,
            config: &config,
            profile: &resolved_profile,
            force: args.force,
            previous_profile_lock: previous_profile_lock.as_ref(),
            preserved_prior_hashes: &mut preserved_prior_hashes,
        };

        template_sync.sync_profile_template("AGENTS.md", &target.join("AGENTS.md"))?;
        ensure_required_file_templates(&mut template_sync, &config.rules.required_files)?;
        ensure_required_docs(&mut template_sync, &config.docs.required)?;

        if profile.includes_handoff {
            template_sync.sync_profile_template(
                "docs/project/handoff-template.md",
                &target.join("docs/project/handoff-template.md"),
            )?;
        }

        ensure_adapter_target_templates(
            &mut template_sync,
            &profile.adapter_targets,
            &config.rules.required_files,
        )?;
    }

    if let Some(workflow_path) = config.ci.workflow_path.as_ref() {
        let destination = target.join(workflow_path);
        ensure_parent_dir(&destination)?;
        if let Some(template) = resolved_profile
            .ci_template_candidates(&ci_provider)
            .into_iter()
            .find(|path| path.exists())
        {
            record_preserved_prior_hash(
                workflow_path,
                sync_managed_copy(
                    &template,
                    &destination,
                    args.force,
                    managed_path_entry(previous_profile_lock.as_ref(), workflow_path),
                )?,
                &mut preserved_prior_hashes,
            );
        } else if let Some(template) = resolved_profile.ci_template_content(&ci_provider) {
            record_preserved_prior_hash(
                workflow_path,
                sync_managed_text(
                    template,
                    &destination,
                    args.force,
                    managed_path_entry(previous_profile_lock.as_ref(), workflow_path),
                )?,
                &mut preserved_prior_hashes,
            );
        }
    }

    copy_profile_assets(
        &resolved_profile,
        &target,
        args.force,
        previous_profile_lock.as_ref(),
        &mut preserved_prior_hashes,
    )?;
    let managed_paths = hydrate_managed_paths(
        &target,
        &resolved_profile,
        collect_managed_paths(&resolved_profile, &config, profile)?,
        previous_profile_lock.as_ref(),
        &preserved_prior_hashes,
    )?;
    ProfileLock::new(&resolved_profile, &config, managed_paths)
        .write(&target.join(".guardrails/profile.lock"))?;

    let init_report = build_init_report(
        &target,
        &reportable_paths,
        &existing_before,
        &resolved_profile,
        &existing_assets_before,
    )?;

    print_init_handoff(
        &target,
        &config,
        profile,
        &resolved_profile,
        &ci_provider,
        &init_report,
    );

    Ok(())
}

fn built_in_profile_summary<'a>(profile_name: &str, fallback_description: &'a str) -> &'a str {
    built_in_profile_info(profile_name)
        .map(|info| info.summary)
        .unwrap_or(fallback_description)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EditSuggestion {
    path: &'static str,
    reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnershipNote {
    label: &'static str,
    detail: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct InitReport {
    created: Vec<String>,
    kept_existing: Vec<String>,
}

fn recommended_edit_paths(profile: &crate::profile::Profile) -> Vec<EditSuggestion> {
    let mut suggestions = vec![
        EditSuggestion {
            path: "README.md",
            reason: "create or confirm the repo overview because doctor/check require a real README.md",
        },
        EditSuggestion {
            path: "AGENTS.md",
            reason: "set the repo-specific instructions contributors and agents should read first",
        },
        EditSuggestion {
            path: "docs/project/implementation-tracker.md",
            reason: "replace starter tracker content with the real work, milestones, and current status",
        },
        EditSuggestion {
            path: ".guardrails/state/tasks/",
            reason: "create or refine durable repo-local task records for approved work instead of relying on ad-hoc handoff text alone",
        },
    ];

    if profile.includes_handoff {
        suggestions.push(EditSuggestion {
            path: "docs/project/handoff-template.md",
            reason: "replace starter handoff guidance with the repo's actual verification and next-step expectations",
        });
    }

    if profile
        .required_docs
        .iter()
        .any(|path| path == "docs/project/decision-log.md")
    {
        suggestions.push(EditSuggestion {
            path: "docs/project/decision-log.md",
            reason: "record the first important project decisions so the profile's stricter doc flow is real",
        });
    } else {
        suggestions.push(EditSuggestion {
            path: ".guardrails/guardrails.toml",
            reason: "review the installed profile, CI provider, and required repo-local expectations",
        });
    }

    if profile
        .required_docs
        .iter()
        .any(|path| path.starts_with("docs/best-practices/"))
    {
        suggestions.push(EditSuggestion {
            path: "docs/best-practices/",
            reason: "curate the seeded doctrine so it matches the repository's real safety, review, and collaboration expectations",
        });
    }

    suggestions
}

fn profile_choice_guidance(profile_name: &str) -> String {
    match profile_name {
        "minimal" => {
            "default neutral baseline; switch only if you intentionally want stricter repo-local doctrine".to_string()
        }
        "docs-driven" => {
            "use this when you want the neutral baseline plus a required decision log".to_string()
        }
        "guardrails" => {
            "opt-in FirbLab-style doctrine profile; use this when you want seeded operating guidance and curated best-practice docs in the repo".to_string()
        }
        _ => "custom profile supplied by the selected profile source".to_string(),
    }
}

fn ci_choice_guidance(ci_provider: &str) -> String {
    match ci_provider {
        "github" => {
            "writes a GitHub Actions guardrails workflow; choose `gitlab` for GitLab CI or `none` for no CI file".to_string()
        }
        "gitlab" => {
            "writes a GitLab guardrails CI include; choose `github` for GitHub Actions or `none` for no CI file".to_string()
        }
        "none" => {
            "skips CI file generation; use this when you want repo-local checks first and will wire CI later".to_string()
        }
        _ => "uses the CI wiring defined by the selected profile".to_string(),
    }
}

fn ownership_notes(
    config: &GuardrailsConfig,
    profile: &crate::profile::Profile,
    resolved_profile: &ResolvedProfile,
) -> Vec<OwnershipNote> {
    let mut notes = vec![
        OwnershipNote {
            label: "Lockfile",
            detail: ".guardrails/profile.lock records tool-managed paths and stale-file behavior"
                .to_string(),
        },
        OwnershipNote {
            label: "Review-only by default",
            detail:
                "docs, AGENTS.md, config, and copied assets stay editable in your repo and are not auto-removed when they go stale"
                    .to_string(),
        },
        OwnershipNote {
            label: "Durable state",
            detail:
                ".guardrails/state/ is preserved across upgrades so repo-local tasks and handoffs survive reapply flows"
                    .to_string(),
        },
    ];

    if config.ci.workflow_path.is_some() {
        notes.push(OwnershipNote {
            label: "CI file",
            detail:
                "the generated CI workflow is tool-managed and may be auto-removed later if you switch CI providers"
                    .to_string(),
        });
    }

    if resolved_profile.has_profile_assets() {
        notes.push(OwnershipNote {
            label: "Profile assets",
            detail: "copied profile assets are tracked as managed paths, but remain review-only on upgrade"
                .to_string(),
        });
    }

    if profile
        .required_docs
        .iter()
        .any(|path| path == "docs/project/decision-log.md")
    {
        notes.push(OwnershipNote {
            label: "Decision log",
            detail:
                "the decision log is part of this profile's required baseline and stays checked by doctor/check"
                    .to_string(),
        });
    }

    if profile
        .required_docs
        .iter()
        .any(|path| path.starts_with("docs/best-practices/"))
    {
        notes.push(OwnershipNote {
            label: "Doctrine docs",
            detail: "seeded best-practice docs are repo-owned profile content: edit them to match the repository instead of treating them as fixed runtime behavior"
                .to_string(),
        });
    }

    notes
}

fn created_paths(
    target: &Path,
    config: &GuardrailsConfig,
    profile: &crate::profile::Profile,
    resolved_profile: &ResolvedProfile,
    reportable_paths: &[String],
) -> Vec<String> {
    let mut paths = reportable_paths
        .iter()
        .map(|path| target.join(path).display().to_string())
        .collect::<Vec<_>>();

    if resolved_profile.has_profile_assets() {
        paths.push(format!("{} (plus copied profile assets)", target.display()));
    }

    let _ = (config, profile);
    paths.sort();
    paths.dedup();
    paths
}

fn reportable_paths(config: &GuardrailsConfig, profile: &crate::profile::Profile) -> Vec<String> {
    let mut paths = BTreeSet::from([
        ".guardrails/guardrails.toml".to_string(),
        ".guardrails/profile.lock".to_string(),
        STATE_DIR.to_string(),
        "AGENTS.md".to_string(),
    ]);

    for path in &config.docs.required {
        paths.insert(path.clone());
    }

    for path in &config.rules.required_files {
        paths.insert(path.clone());
    }

    if profile.includes_handoff {
        paths.insert("docs/project/handoff-template.md".to_string());
    }

    if let Some(workflow_path) = config.ci.workflow_path.as_ref() {
        paths.insert(workflow_path.clone());
    }

    paths.into_iter().collect()
}

fn existing_report_paths(target: &Path, relative_paths: &[String]) -> BTreeSet<String> {
    relative_paths
        .iter()
        .filter(|path| target.join(path).exists())
        .cloned()
        .collect()
}

fn existing_profile_asset_paths(
    target: &Path,
    profile: &ResolvedProfile,
) -> Result<BTreeSet<String>> {
    Ok(profile_asset_paths(profile)?
        .into_iter()
        .map(|entry| entry.path)
        .filter(|path| target.join(path).exists())
        .collect())
}

fn build_init_report(
    target: &Path,
    reportable_paths: &[String],
    existing_before: &BTreeSet<String>,
    profile: &ResolvedProfile,
    existing_assets_before: &BTreeSet<String>,
) -> Result<InitReport> {
    let existing_after = existing_report_paths(target, reportable_paths);
    let mut created = existing_after
        .difference(existing_before)
        .map(|path| target.join(path).display().to_string())
        .collect::<Vec<_>>();
    let mut kept_existing = existing_before
        .iter()
        .map(|path| target.join(path).display().to_string())
        .collect::<Vec<_>>();

    let asset_paths = profile_asset_paths(profile)?
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    let new_assets_copied = asset_paths
        .iter()
        .any(|path| target.join(path).exists() && !existing_assets_before.contains(path));
    let existing_assets_kept = asset_paths
        .iter()
        .any(|path| existing_assets_before.contains(path));

    if new_assets_copied {
        created.push(format!("{} (plus copied profile assets)", target.display()));
    }

    if existing_assets_kept {
        kept_existing.push(format!(
            "{} (existing profile assets kept)",
            target.display()
        ));
    }

    created.sort();
    created.dedup();
    kept_existing.sort();
    kept_existing.dedup();

    Ok(InitReport {
        created,
        kept_existing,
    })
}

fn recommended_next_commands(target: &Path, profile: &crate::profile::Profile) -> Vec<String> {
    let mut commands = vec![
        format!("{CLI_DISPLAY_NAME} doctor --target {}", target.display()),
        format!("{CLI_DISPLAY_NAME} check --target {}", target.display()),
        format!(
            "{CLI_DISPLAY_NAME} tasks list --target {}",
            target.display()
        ),
    ];

    if profile.includes_handoff {
        commands.push(format!(
            "{CLI_DISPLAY_NAME} handoff list --target {}",
            target.display()
        ));
    }

    if profile
        .required_files
        .iter()
        .any(|path| path == ".pre-commit-config.yaml")
    {
        commands
            .push("pre-commit install --hook-type pre-commit --hook-type commit-msg".to_string());
    }

    commands
}

fn resolve_ci_provider(explicit: Option<&CiProvider>, profile_default: &str) -> Result<CiProvider> {
    if let Some(ci) = explicit {
        return Ok(ci.clone());
    }

    CiProvider::from_str(profile_default)
        .ok_or_else(|| anyhow!("unsupported profile default_ci: {profile_default}"))
}

pub(crate) fn collect_managed_paths(
    profile: &ResolvedProfile,
    config: &GuardrailsConfig,
    profile_data: &crate::profile::Profile,
) -> Result<Vec<ManagedPathEntry>> {
    let mut managed_paths = vec![
        ManagedPathEntry::review(".guardrails/guardrails.toml"),
        ManagedPathEntry::review(".guardrails/profile.lock"),
        ManagedPathEntry::preserve(STATE_DIR),
        ManagedPathEntry::review("AGENTS.md"),
    ];

    managed_paths.extend(
        config
            .docs
            .required
            .iter()
            .map(|path| ManagedPathEntry::review(path.clone())),
    );
    managed_paths.extend(
        config
            .rules
            .required_files
            .iter()
            .filter(|path| is_managed_required_file(profile, path))
            .map(|path| ManagedPathEntry::review(path.clone())),
    );

    if profile_data.includes_handoff {
        let handoff = "docs/project/handoff-template.md";
        if !managed_paths.iter().any(|entry| entry.path == handoff) {
            managed_paths.push(ManagedPathEntry::review(handoff));
        }
    }

    if let Some(workflow_path) = config.ci.workflow_path.as_ref() {
        managed_paths.push(ManagedPathEntry::remove(workflow_path.clone()));
    }

    managed_paths.extend(
        profile_data
            .adapter_targets
            .iter()
            .map(|target| ManagedPathEntry::review(target.path.clone())),
    );
    managed_paths.extend(profile_asset_paths(profile)?);
    managed_paths.sort_by(|left, right| left.path.cmp(&right.path));
    managed_paths.dedup_by(|left, right| left.path == right.path);
    Ok(managed_paths)
}

fn load_existing_profile_lock(repo_root: &Path) -> Result<Option<ProfileLock>> {
    let path = repo_root.join(".guardrails/profile.lock");
    if !path.exists() {
        return Ok(None);
    }

    ProfileLock::load(&path)
        .map(Some)
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn hydrate_managed_paths(
    repo_root: &Path,
    profile: &ResolvedProfile,
    managed_paths: Vec<ManagedPathEntry>,
    previous_profile_lock: Option<&ProfileLock>,
    preserved_prior_hashes: &HashSet<String>,
) -> Result<Vec<ManagedPathEntry>> {
    managed_paths
        .into_iter()
        .map(|entry| {
            hydrate_managed_path(
                repo_root,
                profile,
                entry,
                previous_profile_lock,
                preserved_prior_hashes,
            )
        })
        .collect()
}

fn hydrate_managed_path(
    repo_root: &Path,
    profile: &ResolvedProfile,
    entry: ManagedPathEntry,
    previous_profile_lock: Option<&ProfileLock>,
    preserved_prior_hashes: &HashSet<String>,
) -> Result<ManagedPathEntry> {
    let absolute = repo_root.join(&entry.path);
    let installed_sha256 = if preserved_prior_hashes.contains(&entry.path) {
        managed_path_entry(previous_profile_lock, &entry.path)
            .and_then(|previous| previous.installed_sha256.clone())
    } else if absolute.exists() {
        installed_sha256_for_path(&absolute)?
    } else {
        None
    };
    let managed_blocks = if absolute.exists() {
        managed_block_entries_for_path(&absolute, &entry.path, profile)?
    } else {
        Vec::new()
    };

    Ok(entry
        .with_installed_sha256(installed_sha256)
        .with_managed_blocks(managed_blocks))
}

fn managed_block_entries_for_path(
    absolute_path: &Path,
    relative_path: &str,
    profile: &ResolvedProfile,
) -> Result<Vec<ManagedBlockEntry>> {
    let specs = profile.managed_blocks_for_path(relative_path);
    if specs.is_empty() || absolute_path.is_dir() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(absolute_path)
        .with_context(|| format!("failed to read {}", absolute_path.display()))?;
    let parsed = parse_managed_blocks(&contents)?;

    specs
        .into_iter()
        .map(|spec| {
            let block = parsed
                .iter()
                .find(|block| block.id == spec.id)
                .with_context(|| {
                    format!(
                        "{} is missing managed block `{}`",
                        absolute_path.display(),
                        spec.id
                    )
                })?;
            Ok(ManagedBlockEntry {
                id: spec.id,
                generator: spec.generator,
                content_sha256: crate::managed_block::sha256_text(&block.content),
            })
        })
        .collect()
}

fn profile_asset_paths(profile: &ResolvedProfile) -> Result<Vec<ManagedPathEntry>> {
    let Some(source_root) = profile.profile_assets_dir() else {
        return Ok(Vec::new());
    };
    if !source_root.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    collect_asset_paths_recursive(&source_root, &source_root, &mut paths)?;
    Ok(paths)
}

fn collect_asset_paths_recursive(
    source_root: &Path,
    current_dir: &Path,
    paths: &mut Vec<ManagedPathEntry>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)
        .with_context(|| format!("failed to read {}", current_dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", current_dir.display()))?;
        let source = entry.path();

        if source.is_dir() {
            collect_asset_paths_recursive(source_root, &source, paths)?;
            continue;
        }

        let relative = source
            .strip_prefix(source_root)
            .with_context(|| format!("failed to relativize {}", source.display()))?;
        paths.push(default_managed_path_entry(
            &relative.to_string_lossy().replace('\\', "/"),
        ));
    }

    Ok(())
}

fn print_init_handoff(
    target: &Path,
    config: &GuardrailsConfig,
    profile: &crate::profile::Profile,
    resolved_profile: &ResolvedProfile,
    ci_provider: &str,
    init_report: &InitReport,
) {
    println!("Initialized guardrails in {}", target.display());
    println!(
        "Profile: {} ({})",
        profile.name,
        built_in_profile_summary(&profile.name, &profile.description)
    );
    println!(
        "  Why this profile: {}",
        profile_choice_guidance(&profile.name)
    );
    println!("CI: {}", ci_provider);
    println!("  CI choice: {}", ci_choice_guidance(ci_provider));
    println!("Created:");
    for path in &init_report.created {
        println!("  - {}", path);
    }
    if !init_report.kept_existing.is_empty() {
        println!("Kept existing:");
        for path in &init_report.kept_existing {
            println!("  - {}", path);
        }
    }
    println!("Tool-managed:");
    for note in ownership_notes(config, profile, resolved_profile) {
        println!("  - {}: {}", note.label, note.detail);
    }
    println!("Edit these first:");
    for suggestion in recommended_edit_paths(profile) {
        println!("  - {}: {}", suggestion.path, suggestion.reason);
    }
    println!("Run these next:");
    for command in recommended_next_commands(target, profile) {
        println!("  - {}", command);
    }
}

fn normalize_target(target: &Path) -> Result<PathBuf> {
    if target.exists() {
        fs::canonicalize(target)
            .with_context(|| format!("failed to canonicalize {}", target.display()))
    } else {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
        fs::canonicalize(target)
            .with_context(|| format!("failed to canonicalize {}", target.display()))
    }
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}

struct ProfileTemplateSync<'a, 'b> {
    repo_root: &'a Path,
    config: &'a GuardrailsConfig,
    profile: &'a ResolvedProfile,
    force: bool,
    previous_profile_lock: Option<&'a ProfileLock>,
    preserved_prior_hashes: &'b mut HashSet<String>,
}

impl ProfileTemplateSync<'_, '_> {
    fn sync_profile_template(&mut self, template_relative: &str, destination: &Path) -> Result<()> {
        let managed_blocks = self.profile.managed_blocks_for_path(template_relative);
        if managed_blocks.is_empty() {
            let template = load_template_text(self.profile, template_relative)?;
            record_preserved_prior_hash(
                template_relative,
                sync_managed_text(
                    &template,
                    destination,
                    self.force,
                    managed_path_entry(self.previous_profile_lock, template_relative),
                )?,
                self.preserved_prior_hashes,
            );
            return Ok(());
        }

        let mut contents = if destination.exists() {
            fs::read_to_string(destination)
                .with_context(|| format!("failed to read {}", destination.display()))?
        } else {
            load_template_text(self.profile, template_relative)?
        };

        for spec in managed_blocks {
            let block = render_declared_block(self.repo_root, self.config, &spec)?;
            contents = upsert_managed_block(&contents, &block, spec.placement)?;
        }

        ensure_parent_dir(destination)?;
        fs::write(destination, contents)
            .with_context(|| format!("failed to write {}", destination.display()))?;
        Ok(())
    }
}

fn ensure_required_docs(
    template_sync: &mut ProfileTemplateSync<'_, '_>,
    required_docs: &[String],
) -> Result<()> {
    for relative in required_docs {
        let destination = template_sync.repo_root.join(relative);
        if template_sync
            .profile
            .template_candidates(relative)
            .into_iter()
            .any(|path| path.exists())
            || template_sync.profile.template_content(relative).is_some()
        {
            template_sync.sync_profile_template(relative, &destination)?;
            continue;
        }

        record_preserved_prior_hash(
            relative,
            sync_managed_text(
                &default_placeholder_doc(relative),
                &destination,
                template_sync.force,
                managed_path_entry(template_sync.previous_profile_lock, relative),
            )?,
            template_sync.preserved_prior_hashes,
        );
    }

    Ok(())
}

fn ensure_required_file_templates(
    template_sync: &mut ProfileTemplateSync<'_, '_>,
    required_files: &[String],
) -> Result<()> {
    for relative in required_files {
        if !is_managed_required_file(template_sync.profile, relative) {
            continue;
        }

        let destination = template_sync.repo_root.join(relative);
        template_sync.sync_profile_template(relative, &destination)?;
    }

    Ok(())
}

fn ensure_adapter_target_templates(
    template_sync: &mut ProfileTemplateSync<'_, '_>,
    adapter_targets: &[crate::profile::AdapterTargetConfig],
    required_files: &[String],
) -> Result<()> {
    for target in adapter_targets {
        if required_files.iter().any(|path| path == &target.path) {
            continue;
        }

        if template_sync
            .profile
            .template_candidates(&target.path)
            .into_iter()
            .any(|path| path.exists())
            || template_sync
                .profile
                .template_content(&target.path)
                .is_some()
        {
            let destination = template_sync.repo_root.join(&target.path);
            template_sync.sync_profile_template(&target.path, &destination)?;
        }
    }

    Ok(())
}

fn is_managed_required_file(profile: &ResolvedProfile, relative_path: &str) -> bool {
    if matches!(
        relative_path,
        "README.md" | "AGENTS.md" | ".guardrails/guardrails.toml"
    ) {
        return false;
    }

    profile
        .template_candidates(relative_path)
        .into_iter()
        .any(|path| path.exists())
        || profile.template_content(relative_path).is_some()
}

fn load_template_text(profile: &ResolvedProfile, template_relative: &str) -> Result<String> {
    for source in profile.template_candidates(template_relative) {
        if source.exists() {
            return fs::read_to_string(&source)
                .with_context(|| format!("failed to read {}", source.display()));
        }
    }

    if let Some(template) = profile.template_content(template_relative) {
        return Ok(template.to_string());
    }

    bail!("missing template for {template_relative}")
}

fn default_placeholder_doc(relative_path: &str) -> String {
    format!(
        "# {}\n\nReplace this placeholder with repo-specific content.\n",
        relative_path
            .split('/')
            .next_back()
            .unwrap_or("Document")
            .replace(".md", "")
            .replace('-', " ")
    )
}

fn sync_managed_copy(
    source: &Path,
    destination: &Path,
    force: bool,
    previous_entry: Option<&ManagedPathEntry>,
) -> Result<FullFileSyncOutcome> {
    let contents =
        fs::read(source).with_context(|| format!("failed to read {}", source.display()))?;
    sync_managed_bytes(&contents, destination, force, previous_entry)
}

fn sync_managed_text(
    content: &str,
    destination: &Path,
    force: bool,
    previous_entry: Option<&ManagedPathEntry>,
) -> Result<FullFileSyncOutcome> {
    sync_managed_bytes(content.as_bytes(), destination, force, previous_entry)
}

fn sync_managed_bytes(
    contents: &[u8],
    destination: &Path,
    force: bool,
    previous_entry: Option<&ManagedPathEntry>,
) -> Result<FullFileSyncOutcome> {
    if destination.exists() {
        match full_file_sync_outcome(destination, force, previous_entry)? {
            FullFileSyncOutcome::Written => {}
            outcome => return Ok(outcome),
        }
    }

    ensure_parent_dir(destination)?;
    fs::write(destination, contents)
        .with_context(|| format!("failed to write {}", destination.display()))?;
    Ok(FullFileSyncOutcome::Written)
}

fn full_file_sync_outcome(
    destination: &Path,
    force: bool,
    previous_entry: Option<&ManagedPathEntry>,
) -> Result<FullFileSyncOutcome> {
    if !force {
        return Ok(FullFileSyncOutcome::Skipped);
    }

    let Some(previous_entry) = previous_entry else {
        return Ok(FullFileSyncOutcome::Skipped);
    };
    let Some(installed_sha256) = previous_entry.installed_sha256.as_deref() else {
        return Ok(FullFileSyncOutcome::Skipped);
    };

    let current_sha256 = installed_sha256_for_path(destination)?;
    if current_sha256.as_deref() == Some(installed_sha256) {
        Ok(FullFileSyncOutcome::Written)
    } else {
        Ok(FullFileSyncOutcome::PreservedEdited)
    }
}

fn managed_path_entry<'a>(
    profile_lock: Option<&'a ProfileLock>,
    relative_path: &str,
) -> Option<&'a ManagedPathEntry> {
    profile_lock.and_then(|lock| lock.managed_path_entry(relative_path))
}

fn record_preserved_prior_hash(
    relative_path: &str,
    outcome: FullFileSyncOutcome,
    preserved_prior_hashes: &mut HashSet<String>,
) {
    if outcome == FullFileSyncOutcome::PreservedEdited {
        preserved_prior_hashes.insert(relative_path.to_string());
    }
}

fn copy_profile_assets(
    profile: &ResolvedProfile,
    target: &Path,
    force: bool,
    previous_profile_lock: Option<&ProfileLock>,
    preserved_prior_hashes: &mut HashSet<String>,
) -> Result<()> {
    let Some(source_root) = profile.profile_assets_dir() else {
        return Ok(());
    };
    if !source_root.exists() {
        return Ok(());
    }

    copy_dir_contents(
        &source_root,
        &source_root,
        target,
        force,
        previous_profile_lock,
        preserved_prior_hashes,
    )
}

fn copy_dir_contents(
    source_root: &Path,
    current_dir: &Path,
    destination_root: &Path,
    force: bool,
    previous_profile_lock: Option<&ProfileLock>,
    preserved_prior_hashes: &mut HashSet<String>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)
        .with_context(|| format!("failed to read {}", current_dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", current_dir.display()))?;
        let source = entry.path();

        if source.is_dir() {
            copy_dir_contents(
                source_root,
                &source,
                destination_root,
                force,
                previous_profile_lock,
                preserved_prior_hashes,
            )?;
        } else {
            let relative = source
                .strip_prefix(source_root)
                .with_context(|| format!("failed to relativize {}", source.display()))?;
            let relative = relative.to_string_lossy().replace('\\', "/");
            let destination = destination_root.join(relative.as_str());
            record_preserved_prior_hash(
                &relative,
                sync_managed_copy(
                    &source,
                    &destination,
                    force,
                    managed_path_entry(previous_profile_lock, &relative),
                )?,
                preserved_prior_hashes,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{built_in_profile_summary, recommended_edit_paths};
    use crate::{
        config::{ConftestEngineConfig, SemgrepEngineConfig},
        profile::Profile,
    };

    fn sample_profile(required_docs: &[&str]) -> Profile {
        Profile {
            schema_version: 1,
            name: "sample".to_string(),
            description: "sample profile".to_string(),
            default_ci: "github".to_string(),
            root_markers: vec![".git".to_string()],
            docs_enabled: true,
            required_docs: required_docs
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            required_files: vec!["README.md".to_string()],
            forbidden_dirs: Vec::new(),
            includes_handoff: true,
            workflow_paths: std::collections::BTreeMap::new(),
            task_references: crate::config::TaskReferenceRuleConfig::default(),
            link_requirements: Vec::new(),
            evidence_requirements: Vec::new(),
            forbidden_patterns: Vec::new(),
            starter_content: Vec::new(),
            managed_blocks: Vec::new(),
            adapter_targets: Vec::new(),
            rule_packs: crate::profile::ProfileRulePacksConfig::default(),
            semgrep: SemgrepEngineConfig::default(),
            conftest: ConftestEngineConfig::default(),
        }
    }

    #[test]
    fn built_in_profile_summary_prefers_short_built_in_copy() {
        assert_eq!(
            built_in_profile_summary("minimal", "fallback"),
            "Neutral cross-language baseline with local config, AGENTS, tracker, handoff, and optional CI wiring."
        );
        assert_eq!(
            built_in_profile_summary("guardrails", "fallback"),
            "Opt-in FirbLab-style doctrine profile with seeded AGENTS, tracker, decision log, handoff, and curated best-practice docs."
        );
        assert_eq!(built_in_profile_summary("custom", "fallback"), "fallback");
    }

    #[test]
    fn recommended_edit_paths_switches_third_file_for_docs_driven_profiles() {
        let base = sample_profile(&["docs/project/implementation-tracker.md"]);
        let docs_driven = sample_profile(&[
            "docs/project/implementation-tracker.md",
            "docs/project/decision-log.md",
        ]);

        let base_suggestions = recommended_edit_paths(&base);
        let docs_suggestions = recommended_edit_paths(&docs_driven);

        assert_eq!(
            base_suggestions[2].path,
            "docs/project/implementation-tracker.md"
        );
        assert_eq!(
            base_suggestions.last().unwrap().path,
            ".guardrails/guardrails.toml"
        );
        assert_eq!(
            docs_suggestions[2].path,
            "docs/project/implementation-tracker.md"
        );
        assert_eq!(
            docs_suggestions.last().unwrap().path,
            "docs/project/decision-log.md"
        );
    }

    #[test]
    fn recommended_edit_paths_adds_best_practices_for_doctrine_profiles() {
        let doctrine = sample_profile(&[
            "docs/project/implementation-tracker.md",
            "docs/project/decision-log.md",
            "docs/best-practices/change-safety.md",
        ]);

        let suggestions = recommended_edit_paths(&doctrine);

        assert!(
            suggestions
                .iter()
                .any(|suggestion| suggestion.path == "docs/best-practices/")
        );
    }
}
