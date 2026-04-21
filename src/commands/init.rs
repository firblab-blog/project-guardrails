use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    cli::{CLI_DISPLAY_NAME, CiProvider, InitArgs},
    config::{
        CiConfig, DocsConfig, EnginesConfig, GuardrailsConfig, ProjectConfig, RulesConfig,
        write_config,
    },
    profile::ResolvedProfile,
    profile_lock::{ManagedPathEntry, ProfileLock, default_managed_path_entry},
};

pub fn run(args: InitArgs) -> Result<()> {
    let target = normalize_target(&args.target)?;
    let resolved_profile = ResolvedProfile::load(&args.profile, args.profile_path.as_deref())?;
    let profile = &resolved_profile.profile;
    let ci = resolve_ci_provider(args.ci.as_ref(), &profile.default_ci)?;
    let ci_provider = ci.as_str().to_string();

    let config = GuardrailsConfig {
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

    let config_path = target.join(".guardrails/guardrails.toml");
    if config_path.exists() && !args.force {
        bail!(
            "{} already exists; rerun with --force to overwrite",
            config_path.display()
        );
    }

    write_config(&config_path, &config)?;
    let managed_paths = collect_managed_paths(&resolved_profile, &config, profile)?;
    ProfileLock::new(&resolved_profile, &config, managed_paths)
        .write(&target.join(".guardrails/profile.lock"))?;

    copy_profile_template(
        &resolved_profile,
        "AGENTS.md",
        &target.join("AGENTS.md"),
        args.force,
    )?;
    ensure_required_docs(
        &target,
        &resolved_profile,
        &profile.required_docs,
        args.force,
    )?;

    if profile.includes_handoff && !target.join("docs/project/handoff-template.md").exists() {
        copy_profile_template(
            &resolved_profile,
            "docs/project/handoff-template.md",
            &target.join("docs/project/handoff-template.md"),
            args.force,
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
            copy_file(&template, &destination, args.force)?;
        } else if let Some(template) = resolved_profile.ci_template_content(&ci_provider) {
            write_template_content(template, &destination, args.force)?;
        }
    }

    copy_profile_assets(&resolved_profile, &target, args.force)?;

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
    match profile_name {
        "minimal" => {
            "smallest cross-language starting point with local config, AGENTS, tracker, handoff, and optional CI wiring"
        }
        "docs-driven" => {
            "minimal plus a required decision log for teams that want stronger doc discipline"
        }
        _ => fallback_description,
    }
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

    suggestions
}

fn profile_choice_guidance(profile_name: &str) -> String {
    match profile_name {
        "minimal" => {
            "default starting point; switch to `docs-driven` only if you want a required decision log".to_string()
        }
        "docs-driven" => {
            "use this when you want the minimal baseline plus a required decision log".to_string()
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
        "AGENTS.md".to_string(),
    ]);

    for path in &profile.required_docs {
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
    ];

    if profile.includes_handoff {
        commands.push(format!(
            "{CLI_DISPLAY_NAME} handoff --target {}",
            target.display()
        ));
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
        ManagedPathEntry::review("AGENTS.md"),
    ];

    managed_paths.extend(
        profile_data
            .required_docs
            .iter()
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

    managed_paths.extend(profile_asset_paths(profile)?);
    managed_paths.sort_by(|left, right| left.path.cmp(&right.path));
    managed_paths.dedup_by(|left, right| left.path == right.path);
    Ok(managed_paths)
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

fn copy_profile_template(
    profile: &ResolvedProfile,
    template_relative: &str,
    destination: &Path,
    force: bool,
) -> Result<()> {
    for source in profile.template_candidates(template_relative) {
        if source.exists() {
            return copy_file(&source, destination, force);
        }
    }

    if let Some(template) = profile.template_content(template_relative) {
        return write_template_content(template, destination, force);
    }

    bail!("missing template for {template_relative}")
}

fn ensure_required_docs(
    target: &Path,
    profile: &ResolvedProfile,
    required_docs: &[String],
    force: bool,
) -> Result<()> {
    for relative in required_docs {
        let destination = target.join(relative);
        if destination.exists() && !force {
            continue;
        }

        ensure_parent_dir(&destination)?;
        let mut copied = false;
        for template in profile.template_candidates(relative) {
            if template.exists() {
                copy_file(&template, &destination, force)?;
                copied = true;
                break;
            }
        }

        if copied {
            continue;
        }

        if let Some(template) = profile.template_content(relative) {
            write_template_content(template, &destination, force)?;
            continue;
        }

        fs::write(
            &destination,
            format!(
                "# {}\n\nReplace this placeholder with repo-specific content.\n",
                relative
                    .split('/')
                    .next_back()
                    .unwrap_or("Document")
                    .replace(".md", "")
                    .replace('-', " ")
            ),
        )
        .with_context(|| format!("failed to write {}", destination.display()))?;
    }

    Ok(())
}

fn copy_file(source: &Path, destination: &Path, force: bool) -> Result<()> {
    if destination.exists() && !force {
        return Ok(());
    }

    ensure_parent_dir(destination)?;
    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to copy {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn write_template_content(content: &str, destination: &Path, force: bool) -> Result<()> {
    if destination.exists() && !force {
        return Ok(());
    }

    ensure_parent_dir(destination)?;
    fs::write(destination, content)
        .with_context(|| format!("failed to write {}", destination.display()))?;
    Ok(())
}

fn copy_profile_assets(profile: &ResolvedProfile, target: &Path, force: bool) -> Result<()> {
    let Some(source_root) = profile.profile_assets_dir() else {
        return Ok(());
    };
    if !source_root.exists() {
        return Ok(());
    }

    copy_dir_contents(&source_root, target, force)
}

fn copy_dir_contents(source_root: &Path, destination_root: &Path, force: bool) -> Result<()> {
    for entry in fs::read_dir(source_root)
        .with_context(|| format!("failed to read {}", source_root.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", source_root.display()))?;
        let source = entry.path();
        let destination = destination_root.join(entry.file_name());

        if source.is_dir() {
            fs::create_dir_all(&destination)
                .with_context(|| format!("failed to create {}", destination.display()))?;
            copy_dir_contents(&source, &destination, force)?;
        } else {
            ensure_parent_dir(&destination)?;
            copy_file(&source, &destination, force)?;
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
            semgrep: SemgrepEngineConfig::default(),
            conftest: ConftestEngineConfig::default(),
        }
    }

    #[test]
    fn built_in_profile_summary_prefers_short_built_in_copy() {
        assert_eq!(
            built_in_profile_summary("minimal", "fallback"),
            "smallest cross-language starting point with local config, AGENTS, tracker, handoff, and optional CI wiring"
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
}
