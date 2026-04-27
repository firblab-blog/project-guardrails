use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;

use crate::{
    config::{
        EvidenceRequirementConfig, ForbiddenPatternConfig, GuardrailsConfig, LinkRequirementConfig,
    },
    diagnostics::{Diagnostic, DiagnosticReport},
    profile::ResolvedProfile,
    state::tasks::{self, TaskStatus},
};

#[derive(Debug, Clone)]
struct AddedLine {
    path: String,
    line: String,
}

#[derive(Debug, Clone)]
struct StagedChange {
    path: String,
    status: char,
}

pub fn collect_pre_commit_diagnostics(
    repo_root: &Path,
    config: &GuardrailsConfig,
) -> Result<(Vec<String>, DiagnosticReport)> {
    let staged_changes = staged_changes(repo_root)?;
    let staged_paths = staged_changes
        .iter()
        .map(|change| change.path.clone())
        .collect::<Vec<_>>();
    let added_lines = staged_added_lines(repo_root)?;
    let mut report = DiagnosticReport::default();

    report.extend(
        collect_forbidden_dir_diagnostics(&staged_paths, &config.rules.forbidden_dirs)
            .diagnostics()
            .to_vec(),
    );
    report.extend(
        collect_link_requirement_diagnostics(&staged_paths, &config.rules.link_requirements)
            .diagnostics()
            .to_vec(),
    );
    report.extend(
        collect_evidence_requirement_diagnostics(
            &staged_changes,
            &added_lines,
            &config.rules.evidence_requirements,
        )
        .diagnostics()
        .to_vec(),
    );
    report.extend(
        collect_forbidden_pattern_diagnostics(&added_lines, &config.rules.forbidden_patterns)
            .diagnostics()
            .to_vec(),
    );
    report.extend(
        collect_starter_content_diagnostics(repo_root, &staged_paths, config)
            .diagnostics()
            .to_vec(),
    );

    Ok((staged_paths, report))
}

pub fn collect_commit_msg_diagnostics(
    repo_root: &Path,
    config: &GuardrailsConfig,
    message: &str,
) -> Result<(Vec<String>, DiagnosticReport)> {
    let staged_paths = staged_paths(repo_root)?;
    let task_ids = extract_task_ids(message)?
        .into_iter()
        .map(|id| format!("{id:04}"))
        .collect::<Vec<_>>();
    let mut report = DiagnosticReport::default();

    if config.rules.task_references.required && !staged_paths.is_empty() && task_ids.is_empty() {
        report.push(Diagnostic::new(
            "commit_task_reference_missing",
            "commit message must reference at least one active task, for example `[task:0001]` or `refs #0001`",
        ));
    }

    if task_ids.is_empty() {
        return Ok((task_ids, report));
    }

    let collection = tasks::load_collection(repo_root)?;
    report.extend(collection.diagnostics.diagnostics().to_vec());

    let mut tasks_by_id = BTreeMap::new();
    for task in collection.tasks {
        tasks_by_id.insert(task.frontmatter.id, task);
    }

    for task_id in extract_task_ids(message)? {
        let Some(task) = tasks_by_id.get(&task_id) else {
            report.push(Diagnostic::new(
                "commit_task_reference_unknown",
                format!(
                    "commit message references task {:04}, but no matching task record exists under `.guardrails/state/tasks/`",
                    task_id
                ),
            ));
            continue;
        };

        if !matches!(
            task.frontmatter.status,
            TaskStatus::Approved | TaskStatus::InProgress
        ) {
            report.push(Diagnostic::new(
                "commit_task_reference_inactive",
                format!(
                    "commit message references task {:04}, but it is `{}` instead of `approved` or `in_progress`",
                    task_id,
                    task.frontmatter.status.as_str()
                ),
            ));
        }
    }

    Ok((staged_paths, report))
}

fn collect_evidence_requirement_diagnostics(
    staged_changes: &[StagedChange],
    added_lines: &[AddedLine],
    requirements: &[EvidenceRequirementConfig],
) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();

    for requirement in requirements {
        if !evidence_requirement_triggered(staged_changes, requirement) {
            continue;
        }

        if evidence_requirement_satisfied(staged_changes, added_lines, requirement, &mut report) {
            continue;
        }

        let message = if requirement.message.trim().is_empty() {
            default_evidence_requirement_message(requirement)
        } else {
            requirement.message.clone()
        };
        report.push(Diagnostic::new(
            "commit_evidence_requirement_missing",
            message,
        ));
    }

    report
}

fn evidence_requirement_triggered(
    staged_changes: &[StagedChange],
    requirement: &EvidenceRequirementConfig,
) -> bool {
    let changed_match = !requirement.changed_paths.is_empty()
        && staged_changes.iter().any(|change| {
            requirement
                .changed_paths
                .iter()
                .any(|spec| path_matches_spec(&change.path, spec))
        });
    let deleted_match = !requirement.deleted_paths.is_empty()
        && staged_changes
            .iter()
            .filter(|change| change.status == 'D')
            .any(|change| {
                requirement
                    .deleted_paths
                    .iter()
                    .any(|spec| path_matches_spec(&change.path, spec))
            });

    changed_match || deleted_match
}

fn evidence_requirement_satisfied(
    staged_changes: &[StagedChange],
    added_lines: &[AddedLine],
    requirement: &EvidenceRequirementConfig,
    report: &mut DiagnosticReport,
) -> bool {
    let has_replacement_path = !requirement.replacement_paths.is_empty()
        && staged_changes
            .iter()
            .filter(|change| change.status != 'D')
            .any(|change| {
                requirement
                    .replacement_paths
                    .iter()
                    .any(|spec| path_matches_spec(&change.path, spec))
            });
    if has_replacement_path {
        return true;
    }

    if requirement.evidence_paths.is_empty() {
        return false;
    }

    let has_staged_evidence_path = staged_changes
        .iter()
        .filter(|change| change.status != 'D')
        .any(|change| {
            requirement
                .evidence_paths
                .iter()
                .any(|spec| path_matches_spec(&change.path, spec))
        });
    if !has_staged_evidence_path {
        return false;
    }

    if requirement.evidence_patterns.is_empty() {
        return true;
    }

    let mut regexes = Vec::new();
    for pattern in &requirement.evidence_patterns {
        match Regex::new(pattern) {
            Ok(regex) => regexes.push(regex),
            Err(error) => {
                report.push(Diagnostic::new(
                    "evidence_pattern_invalid",
                    format!("invalid evidence pattern `{}`: {}", pattern, error),
                ));
            }
        }
    }
    if regexes.is_empty() {
        return false;
    }

    added_lines.iter().any(|added| {
        requirement
            .evidence_paths
            .iter()
            .any(|spec| path_matches_spec(&added.path, spec))
            && regexes.iter().any(|regex| regex.is_match(&added.line))
    })
}

fn default_evidence_requirement_message(requirement: &EvidenceRequirementConfig) -> String {
    let label = if requirement.name.trim().is_empty() {
        "staged changes".to_string()
    } else {
        format!("staged changes for `{}`", requirement.name)
    };
    let mut accepted = Vec::new();
    if !requirement.evidence_paths.is_empty() {
        accepted.push(format!(
            "repo-local evidence under [{}]",
            requirement.evidence_paths.join(", ")
        ));
    }
    if !requirement.replacement_paths.is_empty() {
        accepted.push(format!(
            "replacement paths under [{}]",
            requirement.replacement_paths.join(", ")
        ));
    }

    if accepted.is_empty() {
        format!("{label} require configured repo-local evidence")
    } else {
        format!("{label} require {}", accepted.join(" or "))
    }
}

fn collect_forbidden_dir_diagnostics(
    staged_paths: &[String],
    forbidden_dirs: &[String],
) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();

    for staged_path in staged_paths {
        for forbidden in forbidden_dirs {
            if path_matches_spec(staged_path, forbidden) {
                report.push(Diagnostic::new(
                    "commit_forbidden_dir_change",
                    format!(
                        "staged change `{}` is under forbidden directory `{}`",
                        staged_path, forbidden
                    ),
                ));
            }
        }
    }

    report
}

fn collect_link_requirement_diagnostics(
    staged_paths: &[String],
    requirements: &[LinkRequirementConfig],
) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();

    for requirement in requirements {
        if requirement.changed_paths.is_empty() || requirement.required_docs.is_empty() {
            continue;
        }

        let changed = staged_paths.iter().any(|path| {
            requirement
                .changed_paths
                .iter()
                .any(|spec| path_matches_spec(path, spec))
        });
        if !changed {
            continue;
        }

        let has_companion_doc = staged_paths.iter().any(|path| {
            requirement
                .required_docs
                .iter()
                .any(|required| path_matches_spec(path, required))
        });

        if !has_companion_doc {
            let message = if requirement.message.trim().is_empty() {
                format!(
                    "staged changes under [{}] require one of [{}] in the same commit",
                    requirement.changed_paths.join(", "),
                    requirement.required_docs.join(", "),
                )
            } else {
                requirement.message.clone()
            };
            report.push(Diagnostic::new("commit_link_requirement_missing", message));
        }
    }

    report
}

fn collect_forbidden_pattern_diagnostics(
    added_lines: &[AddedLine],
    patterns: &[ForbiddenPatternConfig],
) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();

    for pattern in patterns {
        if pattern.pattern.trim().is_empty() {
            continue;
        }

        let regex = match Regex::new(&pattern.pattern) {
            Ok(regex) => regex,
            Err(error) => {
                report.push(Diagnostic::new(
                    "forbidden_pattern_invalid",
                    format!("invalid forbidden pattern `{}`: {}", pattern.pattern, error),
                ));
                continue;
            }
        };

        for added in added_lines {
            if regex.is_match(&added.line) {
                let detail = if pattern.message.trim().is_empty() {
                    format!(
                        "staged addition in `{}` matched forbidden pattern `{}`",
                        added.path, pattern.pattern
                    )
                } else {
                    format!(
                        "staged addition in `{}` matched forbidden pattern `{}`: {}",
                        added.path, pattern.pattern, pattern.message
                    )
                };
                report.push(Diagnostic::new("commit_forbidden_pattern", detail));
            }
        }
    }

    report
}

fn collect_starter_content_diagnostics(
    repo_root: &Path,
    staged_paths: &[String],
    config: &GuardrailsConfig,
) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();
    let resolved_profile = match ResolvedProfile::load_from_config(config) {
        Ok(profile) => profile,
        Err(_) => return report,
    };

    for staged_path in staged_paths {
        let Some(rule) = resolved_profile.starter_content_rule(staged_path) else {
            continue;
        };
        let path = repo_root.join(staged_path);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        let matched_markers = rule
            .markers
            .iter()
            .filter(|marker| contents.contains(marker.as_str()))
            .count();
        if matched_markers >= rule.threshold {
            report.push(Diagnostic::new(
                "commit_starter_content",
                format!(
                    "`{}` still contains stock starter content; replace the placeholder guidance before committing it",
                    staged_path
                ),
            ));
        }
    }

    report
}

fn staged_paths(repo_root: &Path) -> Result<Vec<String>> {
    Ok(staged_changes(repo_root)?
        .into_iter()
        .map(|change| change.path)
        .collect())
}

fn staged_changes(repo_root: &Path) -> Result<Vec<StagedChange>> {
    if !repo_root.join(".git").exists() {
        bail!(
            "pre-commit enforcement requires a git repository at {}",
            repo_root.display()
        );
    }

    let output = git_output(
        repo_root,
        &["diff", "--cached", "--name-status", "--diff-filter=ACMRD"],
    )?;
    Ok(output
        .lines()
        .filter_map(parse_staged_change_line)
        .collect())
}

fn parse_staged_change_line(line: &str) -> Option<StagedChange> {
    let mut parts = line.split('\t');
    let status = parts.next()?.chars().next()?;
    let path = parts.next_back()?.trim();
    if path.is_empty() {
        return None;
    }

    Some(StagedChange {
        path: path.to_string(),
        status,
    })
}

fn staged_added_lines(repo_root: &Path) -> Result<Vec<AddedLine>> {
    let output = git_output(
        repo_root,
        &[
            "diff",
            "--cached",
            "--unified=0",
            "--no-color",
            "--diff-filter=ACMR",
        ],
    )?;
    let mut current_path = None::<String>;
    let mut added_lines = Vec::new();

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(path.to_string());
            continue;
        }

        if let Some(path) = current_path
            .as_ref()
            .filter(|_| line.starts_with('+') && !line.starts_with("+++"))
        {
            added_lines.push(AddedLine {
                path: path.clone(),
                line: line.trim_start_matches('+').to_string(),
            });
        }
    }

    Ok(added_lines)
}

fn git_output(repo_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .with_context(|| {
            format!(
                "failed to run git {} in {}",
                args.join(" "),
                repo_root.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow!(
            "git {} failed in {}: {}",
            args.join(" "),
            repo_root.display(),
            stderr
        ));
    }

    String::from_utf8(output.stdout).context("git output was not valid UTF-8")
}

fn extract_task_ids(message: &str) -> Result<BTreeSet<u32>> {
    let regex = Regex::new(
        r"(?ix)
        \[(?:gr|task):(?P<bracket>\d{1,4})\]
        |
        \b(?:task|tasks|ref|refs|close|closes|fix|fixes)\s*(?:\#|:|-)?\s*(?P<plain>\d{1,4})\b
        ",
    )?;

    let mut ids = BTreeSet::new();
    for captures in regex.captures_iter(message) {
        let value = captures
            .name("bracket")
            .or_else(|| captures.name("plain"))
            .map(|capture| capture.as_str())
            .unwrap_or_default();
        if let Ok(id) = value.parse::<u32>() {
            ids.insert(id);
        }
    }
    Ok(ids)
}

fn path_matches_spec(path: &str, spec: &str) -> bool {
    let normalized_path = normalize_path_spec(path);
    let normalized_spec = normalize_path_spec(spec);
    if normalized_spec.is_empty() {
        return false;
    }

    normalized_path == normalized_spec
        || normalized_path.starts_with(&(normalized_spec.clone() + "/"))
        || (spec.ends_with('/') && normalized_path.starts_with(&(normalized_spec + "/")))
}

fn normalize_path_spec(value: &str) -> String {
    value
        .split('#')
        .next()
        .unwrap_or(value)
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{extract_task_ids, parse_staged_change_line, path_matches_spec};

    #[test]
    fn extract_task_ids_supports_bracket_and_refs_styles() {
        let ids = extract_task_ids("[task:0001] refs #42 closes 7").expect("ids");
        assert!(ids.contains(&1));
        assert!(ids.contains(&42));
        assert!(ids.contains(&7));
    }

    #[test]
    fn path_matching_accepts_exact_paths_and_prefixes() {
        assert!(path_matches_spec("src/lib.rs", "src/"));
        assert!(path_matches_spec(
            "docs/project/implementation-tracker.md",
            "docs/project/implementation-tracker.md"
        ));
        assert!(!path_matches_spec("tests/task_state.rs", "src/"));
    }

    #[test]
    fn staged_change_parser_uses_new_path_for_renames() {
        let change = parse_staged_change_line("R100\told_test.rs\ttests/new_test.rs")
            .expect("renamed change");
        assert_eq!(change.status, 'R');
        assert_eq!(change.path, "tests/new_test.rs");
    }
}
