use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;

use crate::{
    config::{ForbiddenPatternConfig, GuardrailsConfig, LinkRequirementConfig},
    diagnostics::{Diagnostic, DiagnosticReport},
    profile::ResolvedProfile,
    state::tasks::{self, TaskStatus},
};

#[derive(Debug, Clone)]
struct AddedLine {
    path: String,
    line: String,
}

pub fn collect_pre_commit_diagnostics(
    repo_root: &Path,
    config: &GuardrailsConfig,
) -> Result<(Vec<String>, DiagnosticReport)> {
    let staged_paths = staged_paths(repo_root)?;
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
    if !repo_root.join(".git").exists() {
        bail!(
            "pre-commit enforcement requires a git repository at {}",
            repo_root.display()
        );
    }

    let output = git_output(
        repo_root,
        &["diff", "--cached", "--name-only", "--diff-filter=ACMR"],
    )?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
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
    use super::{extract_task_ids, path_matches_spec};

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
}
