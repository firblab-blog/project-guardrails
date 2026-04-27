use std::{
    collections::BTreeSet,
    io,
    path::Path,
    process::{Command, Output},
};

use chrono::DateTime;
use serde::Serialize;

use crate::state::handoffs::HandoffSummary;

#[derive(Debug, Clone, Serialize)]
pub struct GitContinuity {
    pub available: bool,
    pub status: GitContinuityStatus,
    pub handoff_timestamp: Option<String>,
    pub baseline_commit: Option<String>,
    pub changed_since_handoff: Vec<String>,
    pub staged_paths: Vec<String>,
    pub unstaged_paths: Vec<String>,
    pub untracked_paths: Vec<String>,
    pub diagnostics: Vec<GitContinuityDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitContinuityStatus {
    Available,
    NoHandoff,
    Unavailable,
    InsufficientBaseline,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitContinuityDiagnostic {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitWorkingTreeState {
    pub staged_paths: Vec<String>,
    pub unstaged_paths: Vec<String>,
    pub untracked_paths: Vec<String>,
}

pub fn continuity_since_handoff(
    repo_root: &Path,
    latest_handoff: Option<&HandoffSummary>,
) -> GitContinuity {
    let mut diagnostics = Vec::new();

    if let Err(diagnostic) = ensure_git_repo(repo_root) {
        diagnostics.push(diagnostic);
        return unavailable(diagnostics);
    }

    let working_tree = match working_tree_state(repo_root) {
        Ok(state) => state,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            return unavailable(diagnostics);
        }
    };

    let Some(handoff) = latest_handoff else {
        return GitContinuity {
            available: true,
            status: GitContinuityStatus::NoHandoff,
            handoff_timestamp: None,
            baseline_commit: None,
            changed_since_handoff: Vec::new(),
            staged_paths: working_tree.staged_paths,
            unstaged_paths: working_tree.unstaged_paths,
            untracked_paths: working_tree.untracked_paths,
            diagnostics: vec![GitContinuityDiagnostic {
                code: "git_handoff_missing",
                message:
                    "no latest handoff is available, so Git continuity has no handoff baseline"
                        .to_string(),
            }],
        };
    };

    let timestamp = handoff.updated.clone();
    if DateTime::parse_from_rfc3339(&timestamp).is_err() {
        diagnostics.push(GitContinuityDiagnostic {
            code: "git_handoff_timestamp_invalid",
            message: format!(
                "latest handoff {} has an unusable updated timestamp `{}`",
                handoff.path, timestamp
            ),
        });
        return insufficient(Some(timestamp), None, working_tree, diagnostics);
    }

    let Some(baseline_commit) = baseline_commit_before(repo_root, &timestamp, &mut diagnostics)
    else {
        return insufficient(Some(timestamp), None, working_tree, diagnostics);
    };

    let mut changed = match diff_paths_from_baseline(repo_root, &baseline_commit) {
        Ok(paths) => paths,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            Vec::new()
        }
    };
    changed.extend(working_tree.untracked_paths.iter().cloned());
    changed.sort();
    changed.dedup();

    let status = if diagnostics.is_empty() {
        GitContinuityStatus::Available
    } else {
        GitContinuityStatus::InsufficientBaseline
    };

    GitContinuity {
        available: true,
        status,
        handoff_timestamp: Some(timestamp),
        baseline_commit: Some(baseline_commit),
        changed_since_handoff: changed,
        staged_paths: working_tree.staged_paths,
        unstaged_paths: working_tree.unstaged_paths,
        untracked_paths: working_tree.untracked_paths,
        diagnostics,
    }
}

pub fn draft_handoff_body_from_git(repo_root: &Path) -> String {
    let mut body = String::from(
        "# Git State Draft\n\nThis draft is based on observable Git state only. It is not proof of intent, validation, or semantic completion.\n",
    );

    if let Err(diagnostic) = ensure_git_repo(repo_root) {
        body.push_str("\n## Git State\n\n");
        body.push_str(&format!(
            "- Git state unavailable: {} ({})\n",
            diagnostic.message, diagnostic.code
        ));
        body.push_str("\n## Next Valid Steps\n\n1. Inspect the repository manually and replace this draft with a human-checked handoff.\n");
        return body;
    }

    let head = short_head(repo_root).unwrap_or_else(|diagnostic| {
        format!("unavailable: {} ({})", diagnostic.message, diagnostic.code)
    });
    let working_tree = working_tree_state(repo_root).unwrap_or_default();
    let changed_paths = combined_working_tree_paths(&working_tree);

    body.push_str("\n## Observed Git State\n\n");
    body.push_str(&format!("- HEAD: {head}\n"));
    if changed_paths.is_empty() {
        body.push_str("- Changed files observed: none\n");
    } else {
        body.push_str("- Changed files observed:\n");
        for path in &changed_paths {
            body.push_str(&format!("  - `{path}`\n"));
        }
    }

    body.push_str("\n## Working Tree Categories\n\n");
    append_path_section(&mut body, "Staged", &working_tree.staged_paths);
    append_path_section(&mut body, "Unstaged", &working_tree.unstaged_paths);
    append_path_section(&mut body, "Untracked", &working_tree.untracked_paths);

    body.push_str("\n## Validation\n\n- Not inferred from Git state; replace this line with commands actually run.\n");
    body.push_str("\n## Next Valid Steps\n\n1. Review the observed paths and replace this draft with the actual result, validation, and remaining work.\n");
    body
}

fn unavailable(diagnostics: Vec<GitContinuityDiagnostic>) -> GitContinuity {
    GitContinuity {
        available: false,
        status: GitContinuityStatus::Unavailable,
        handoff_timestamp: None,
        baseline_commit: None,
        changed_since_handoff: Vec::new(),
        staged_paths: Vec::new(),
        unstaged_paths: Vec::new(),
        untracked_paths: Vec::new(),
        diagnostics,
    }
}

fn insufficient(
    handoff_timestamp: Option<String>,
    baseline_commit: Option<String>,
    working_tree: GitWorkingTreeState,
    diagnostics: Vec<GitContinuityDiagnostic>,
) -> GitContinuity {
    GitContinuity {
        available: true,
        status: GitContinuityStatus::InsufficientBaseline,
        handoff_timestamp,
        baseline_commit,
        changed_since_handoff: Vec::new(),
        staged_paths: working_tree.staged_paths,
        unstaged_paths: working_tree.unstaged_paths,
        untracked_paths: working_tree.untracked_paths,
        diagnostics,
    }
}

fn ensure_git_repo(repo_root: &Path) -> Result<(), GitContinuityDiagnostic> {
    let output = run_git(repo_root, &["rev-parse", "--show-toplevel"])?;
    if output.status.success() {
        return Ok(());
    }

    Err(GitContinuityDiagnostic {
        code: "git_repo_unavailable",
        message: "target is not inside a usable Git repository".to_string(),
    })
}

fn baseline_commit_before(
    repo_root: &Path,
    timestamp: &str,
    diagnostics: &mut Vec<GitContinuityDiagnostic>,
) -> Option<String> {
    let before = format!("--before={timestamp}");
    let output = match run_git(repo_root, &["rev-list", "-1", before.as_str(), "HEAD"]) {
        Ok(output) => output,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            return None;
        }
    };

    if !output.status.success() {
        diagnostics.push(GitContinuityDiagnostic {
            code: "git_baseline_unavailable",
            message: "could not inspect Git history for a handoff baseline".to_string(),
        });
        return None;
    }

    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if commit.is_empty() {
        diagnostics.push(GitContinuityDiagnostic {
            code: "git_baseline_missing",
            message: format!("no commit was found at or before handoff timestamp `{timestamp}`"),
        });
        return None;
    }

    Some(commit)
}

fn diff_paths_from_baseline(
    repo_root: &Path,
    baseline_commit: &str,
) -> Result<Vec<String>, GitContinuityDiagnostic> {
    let output = run_git(
        repo_root,
        &["diff", "--name-only", "-z", baseline_commit, "--"],
    )?;
    if !output.status.success() {
        return Err(GitContinuityDiagnostic {
            code: "git_diff_unavailable",
            message: format!("could not diff working tree against baseline {baseline_commit}"),
        });
    }

    Ok(split_nul_paths(&output.stdout))
}

fn working_tree_state(repo_root: &Path) -> Result<GitWorkingTreeState, GitContinuityDiagnostic> {
    let output = run_git(
        repo_root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    if !output.status.success() {
        return Err(GitContinuityDiagnostic {
            code: "git_status_unavailable",
            message: "could not inspect Git working tree status".to_string(),
        });
    }

    Ok(parse_porcelain_status_z(&output.stdout))
}

fn short_head(repo_root: &Path) -> Result<String, GitContinuityDiagnostic> {
    let output = run_git(repo_root, &["rev-parse", "--short", "HEAD"])?;
    if !output.status.success() {
        return Err(GitContinuityDiagnostic {
            code: "git_head_unavailable",
            message: "could not inspect Git HEAD".to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git(repo_root: &Path, args: &[&str]) -> Result<Output, GitContinuityDiagnostic> {
    Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|error| git_spawn_diagnostic(error, args))
}

fn git_spawn_diagnostic(error: io::Error, args: &[&str]) -> GitContinuityDiagnostic {
    if error.kind() == io::ErrorKind::NotFound {
        return GitContinuityDiagnostic {
            code: "git_binary_missing",
            message: "git binary was not found on PATH".to_string(),
        };
    }

    GitContinuityDiagnostic {
        code: "git_command_failed",
        message: format!("failed to run git {}: {error}", args.join(" ")),
    }
}

fn parse_porcelain_status_z(raw: &[u8]) -> GitWorkingTreeState {
    let mut staged_paths = BTreeSet::new();
    let mut unstaged_paths = BTreeSet::new();
    let mut untracked_paths = BTreeSet::new();

    let mut entries = raw
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let x = entry[0] as char;
        let y = entry[1] as char;
        let path = String::from_utf8_lossy(&entry[3..]).to_string();

        if x == '?' && y == '?' {
            untracked_paths.insert(path);
            continue;
        }

        if x != ' ' && x != '!' {
            staged_paths.insert(path.clone());
        }
        if y != ' ' && y != '!' {
            unstaged_paths.insert(path);
        }

        if matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C') {
            let _ = entries.next();
        }
    }

    GitWorkingTreeState {
        staged_paths: staged_paths.into_iter().collect(),
        unstaged_paths: unstaged_paths.into_iter().collect(),
        untracked_paths: untracked_paths.into_iter().collect(),
    }
}

fn split_nul_paths(raw: &[u8]) -> Vec<String> {
    let mut paths = raw
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| String::from_utf8_lossy(entry).to_string())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn combined_working_tree_paths(working_tree: &GitWorkingTreeState) -> Vec<String> {
    let mut paths = BTreeSet::new();
    paths.extend(working_tree.staged_paths.iter().cloned());
    paths.extend(working_tree.unstaged_paths.iter().cloned());
    paths.extend(working_tree.untracked_paths.iter().cloned());
    paths.into_iter().collect()
}

fn append_path_section(body: &mut String, label: &str, paths: &[String]) {
    if paths.is_empty() {
        body.push_str(&format!("- {label}: none\n"));
        return;
    }

    body.push_str(&format!("- {label}:\n"));
    for path in paths {
        body.push_str(&format!("  - `{path}`\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_status_categories() {
        let raw = b"M  staged.md\0 M unstaged.md\0?? notes/new.md\0AM both.md\0";
        let state = parse_porcelain_status_z(raw);

        assert_eq!(state.staged_paths, vec!["both.md", "staged.md"]);
        assert_eq!(state.unstaged_paths, vec!["both.md", "unstaged.md"]);
        assert_eq!(state.untracked_paths, vec!["notes/new.md"]);
    }

    #[test]
    fn parses_porcelain_rename_without_treating_old_path_as_entry() {
        let raw = b"R  new-name.md\0old-name.md\0";
        let state = parse_porcelain_status_z(raw);

        assert_eq!(state.staged_paths, vec!["new-name.md"]);
        assert!(state.unstaged_paths.is_empty());
        assert!(state.untracked_paths.is_empty());
    }

    #[test]
    fn combines_working_tree_paths_in_stable_order() {
        let state = GitWorkingTreeState {
            staged_paths: vec!["b.md".to_string()],
            unstaged_paths: vec!["a.md".to_string(), "b.md".to_string()],
            untracked_paths: vec!["notes.md".to_string()],
        };

        assert_eq!(
            combined_working_tree_paths(&state),
            vec!["a.md", "b.md", "notes.md"]
        );
    }
}
