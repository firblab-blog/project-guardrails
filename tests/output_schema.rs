use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::{Map, Value, json};
use tempfile::TempDir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_project-guardrails"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

#[test]
fn stable_json_contracts_match_shape_goldens() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "json-contracts");
    init_git_repo(&repo);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "Initial fixture"]);
    stage_readme_change(&repo);

    let message_file = repo.join("COMMIT_MSG");
    fs::write(&message_file, "Document JSON contracts\n").expect("message file");

    let profiles = run_json_success(&["profiles", "list", "--format", "json"]);
    assert_eq!(
        normalized_json(&profiles, &repo),
        json!({
            "schema_version": 1,
            "profiles": [
                {
                    "name": "minimal",
                    "summary": "Neutral cross-language baseline with local config, AGENTS, tracker, handoff, and optional CI wiring.",
                    "description": "Smallest neutral built-in profile for teams that want a portable starting point.",
                    "is_default": true,
                    "is_opt_in": false
                },
                {
                    "name": "docs-driven",
                    "summary": "Neutral baseline plus a required decision log for teams that want stronger documentation discipline.",
                    "description": "Use this when you want the minimal baseline and a required docs/project/decision-log.md.",
                    "is_default": false,
                    "is_opt_in": false
                },
                {
                    "name": "guardrails",
                    "summary": "Opt-in FirbLab-style doctrine profile with seeded AGENTS, tracker, decision log, handoff, and curated best-practice docs.",
                    "description": "Opinionated built-in profile for teams that want seeded operating doctrine without making it the default bootstrap path.",
                    "is_default": false,
                    "is_opt_in": true
                }
            ]
        })
    );

    assert_contract_shape(
        "profiles list --format json",
        &profiles,
        json!({
            "schema_version": "number",
            "profiles": [{
                "name": "string",
                "summary": "string",
                "description": "string",
                "is_default": "bool",
                "is_opt_in": "bool"
            }]
        }),
    );

    assert_contract_shape(
        "status --format json",
        &run_json_success(&["status", "--target", repo_str(&repo), "--format", "json"]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "profile": "string",
            "profile_source": "string",
            "profile_schema_version": "number",
            "installed_by_version": "string",
            "docs_enabled": "bool",
            "ci_provider": "string",
            "required_files": ["string"],
            "forbidden_dirs": [],
            "semgrep_enabled": "bool",
            "conftest_enabled": "bool"
        }),
    );

    assert_contract_shape(
        "adapters list --format json empty",
        &run_json_success(&[
            "adapters",
            "list",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "profile": "string",
            "profile_source": "string",
            "adapters": []
        }),
    );

    assert_contract_shape(
        "doctor --format json",
        &run_json_success(&["doctor", "--target", repo_str(&repo), "--format", "json"]),
        json!({
            "schema_version": "number",
            "ok": "bool",
            "repo_root": "string",
            "profile": "string",
            "profile_source": "string",
            "installed_by_version": "string",
            "semgrep_engine": "string",
            "conftest_engine": "string",
            "statuses": [{
                "label": "string",
                "relative_path": "string",
                "status": "string"
            }],
            "diagnostics": []
        }),
    );

    assert_contract_shape(
        "check --format json",
        &run_json_success(&["check", "--target", repo_str(&repo), "--format", "json"]),
        json!({
            "schema_version": "number",
            "ok": "bool",
            "repo_root": "string",
            "diagnostics": []
        }),
    );

    assert_contract_shape(
        "refresh --format json",
        &run_json_success(&["refresh", "--target", repo_str(&repo), "--format", "json"]),
        json!({
            "schema_version": "number",
            "ok": "bool",
            "repo_root": "string",
            "check": "bool",
            "changed": "bool",
            "changed_paths": [],
            "blocks": [{
                "path": "string",
                "id": "string",
                "generator": "string",
                "status": "string"
            }],
            "diagnostics": []
        }),
    );

    assert_contract_shape(
        "pre-commit --format json",
        &run_json_success(&[
            "pre-commit",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "ok": "bool",
            "repo_root": "string",
            "staged_paths": ["string"],
            "diagnostics": []
        }),
    );

    let commit_msg_failure = run_json_failure(&[
        "commit-msg-check",
        message_file.to_str().expect("message path"),
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(
        normalized_json(&commit_msg_failure, &repo),
        json!({
            "schema_version": 1,
            "ok": false,
            "repo_root": "<repo>",
            "task_ids": [],
            "diagnostics": [
                {
                    "code": "commit_task_reference_missing",
                    "message": "commit message must reference at least one active task, for example `[task:0001]` or `refs #0001`"
                }
            ]
        })
    );
    assert_contract_shape(
        "commit-msg-check <message-file> --format json",
        &commit_msg_failure,
        json!({
            "schema_version": "number",
            "ok": "bool",
            "repo_root": "string",
            "task_ids": [],
            "diagnostics": [{
                "code": "string",
                "message": "string"
            }]
        }),
    );

    assert_contract_shape(
        "upgrade --plan --format json",
        &run_json_success(&[
            "upgrade",
            "--target",
            repo_str(&repo),
            "--plan",
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "current": {
                "profile": "string",
                "profile_source": "string",
                "profile_schema_version": "number",
                "installed_by_version": "string",
                "ci_provider": "string"
            },
            "target": {
                "profile": "string",
                "profile_source": "string",
                "profile_schema_version": "number",
                "installed_by_version": "string",
                "ci_provider": "string"
            },
            "changes": [{
                "field": "string",
                "current": "string",
                "target": "string",
                "changed": "bool"
            }],
            "stale_paths": [],
            "removable_stale_paths": [],
            "preserved_stale_paths": [],
            "review_stale_paths": [],
            "planned_actions": ["string"]
        }),
    );

    assert_contract_shape(
        "tasks list --format json empty",
        &run_json_success(&[
            "tasks",
            "list",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "tasks": []
        }),
    );

    let task_new = run_json_success(&[
        "tasks",
        "new",
        "--target",
        repo_str(&repo),
        "--slug",
        "json-contracts",
        "--priority",
        "p1",
        "--format",
        "json",
    ]);
    assert_task_output_shape("tasks new --format json", &task_new);

    assert_task_output_shape(
        "tasks claim --format json",
        &run_json_success(&[
            "tasks",
            "claim",
            "1",
            "--target",
            repo_str(&repo),
            "--owner",
            "codex",
            "--format",
            "json",
        ]),
    );

    assert_task_output_shape(
        "tasks update --format json",
        &run_json_success(&[
            "tasks",
            "update",
            "1",
            "--target",
            repo_str(&repo),
            "--status",
            "blocked",
            "--format",
            "json",
        ]),
    );

    run_json_success(&[
        "tasks",
        "claim",
        "1",
        "--target",
        repo_str(&repo),
        "--owner",
        "codex",
        "--format",
        "json",
    ]);

    assert_contract_shape(
        "tasks list --format json populated",
        &run_json_success(&[
            "tasks",
            "list",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "tasks": [{
                "id": "string",
                "slug": "string",
                "title": "string",
                "status": "string",
                "owner": "string",
                "priority": "string",
                "updated": "string",
                "path": "string"
            }]
        }),
    );

    assert_contract_shape(
        "handoff list --format json empty",
        &run_json_success(&[
            "handoff",
            "list",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "handoffs": []
        }),
    );

    assert_handoff_output_shape(
        "handoff new --format json",
        &run_json_success(&[
            "handoff",
            "new",
            "--target",
            repo_str(&repo),
            "--slug",
            "json-contracts",
            "--task",
            "1",
            "--format",
            "json",
        ]),
    );

    assert_contract_shape(
        "handoff list --format json populated",
        &run_json_success(&[
            "handoff",
            "list",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "handoffs": [{
                "id": "string",
                "slug": "string",
                "title": "string",
                "status": "string",
                "created": "string",
                "updated": "string",
                "task_ids": ["string"],
                "template_path": "string",
                "path": "string"
            }]
        }),
    );

    let resume = run_json_success(&["resume", "--target", repo_str(&repo), "--format", "json"]);
    assert_eq!(resume["latest_handoff"]["slug"], "json-contracts");
    assert_eq!(resume["linked_active_tasks"][0]["slug"], "json-contracts");
    assert_eq!(
        resume["next_step"]["command"],
        "project-guardrails refresh --target . --check"
    );
    assert_contract_shape(
        "resume --format json",
        &resume,
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "generated_at": "string",
            "latest_handoff": {
                "id": "string",
                "slug": "string",
                "title": "string",
                "status": "string",
                "created": "string",
                "updated": "string",
                "task_ids": ["string"],
                "template_path": "string",
                "path": "string",
                "body_path": "string",
                "body_excerpt": "string"
            },
            "linked_active_tasks": [{
                "id": "string",
                "slug": "string",
                "title": "string",
                "status": "string",
                "owner": "string",
                "priority": "string",
                "updated": "string",
                "path": "string"
            }],
            "git": {
                "available": "bool",
                "status": "string",
                "handoff_timestamp": "string",
                "baseline_commit": "string",
                "changed_since_handoff": ["string"],
                "staged_paths": ["string"],
                "unstaged_paths": [],
                "untracked_paths": ["string"],
                "diagnostics": []
            },
            "doctor": {
                "ok": "bool",
                "diagnostics": [{
                    "code": "string",
                    "message": "string"
                }]
            },
            "next_step": {
                "command": "string",
                "reason": "string"
            }
        }),
    );

    assert_contract_shape(
        "status --for-llm",
        &run_json_success(&["status", "--target", repo_str(&repo), "--for-llm"]),
        llm_status_shape(),
    );

    assert_contract_shape(
        "brief --format json",
        &run_json_success(&["brief", "--target", repo_str(&repo), "--format", "json"]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "generated_at": "string",
            "summary": llm_summary_shape(),
            "brief": {
                "active_task_count": "number",
                "recent_handoff_count": "number",
                "diagnostic_count": "number",
                "recommended_commands": ["string"]
            }
        }),
    );

    assert_contract_shape(
        "pre-work --format json",
        &run_json_success(&["pre-work", "--target", repo_str(&repo), "--format", "json"]),
        json!({
            "schema_version": "number",
            "run_id": "string",
            "run_path": "string",
            "summary": llm_summary_shape()
        }),
    );

    assert_contract_shape(
        "timeline --format json",
        &run_json_success(&["timeline", "--target", repo_str(&repo), "--format", "json"]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "generated_at": "string",
            "events": [{
                "timestamp": "string",
                "kind": "string",
                "action": "string",
                "id": "string",
                "title": "string",
                "status": "string",
                "path": "string",
                "task_ids": ["string"]
            }],
            "diagnostics": []
        }),
    );

    assert_handoff_output_shape(
        "handoff close --format json",
        &run_json_success(&[
            "handoff",
            "close",
            "1",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
    );

    assert_task_output_shape(
        "tasks close --format json",
        &run_json_success(&[
            "tasks",
            "close",
            "1",
            "--target",
            repo_str(&repo),
            "--commit",
            "abc123def",
            "--format",
            "json",
        ]),
    );

    assert_task_output_shape(
        "tasks get --format json",
        &run_json_success(&[
            "tasks",
            "get",
            "1",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
    );

    assert_contract_shape(
        "tasks lint --format json",
        &run_json_success(&[
            "tasks",
            "lint",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "ok": "bool",
            "repo_root": "string",
            "diagnostics": []
        }),
    );
}

#[test]
fn validation_failures_emit_json_before_nonzero_exit() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "invalid-json-contract");

    write_invalid_duplicate_tasks(&repo);
    let lint_failure = run_json_failure(&[
        "tasks",
        "lint",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_contract_shape(
        "tasks lint --format json failure",
        &lint_failure,
        json!({
            "schema_version": "number",
            "ok": "bool",
            "repo_root": "string",
            "diagnostics": [{
                "code": "string",
                "message": "string"
            }]
        }),
    );
    assert_eq!(lint_failure["ok"], false);
    assert!(
        lint_failure["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|item| item["code"] == "task_duplicate_id")
    );
}

#[test]
fn adapters_list_populated_json_contract_matches_shape() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("adapter-json-contract");
    copy_dir(&fixture_root().join("bare-repo"), &repo);
    let profile_dir = write_adapter_contract_profile(temp.path());

    run_success(&[
        "init",
        "--target",
        repo_str(&repo),
        "--profile",
        "adapter-json-contract",
        "--profile-path",
        repo_str(&profile_dir),
        "--ci",
        "none",
    ]);

    assert_contract_shape(
        "adapters list --format json populated",
        &run_json_success(&[
            "adapters",
            "list",
            "--target",
            repo_str(&repo),
            "--format",
            "json",
        ]),
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "profile": "string",
            "profile_source": "string",
            "adapters": [{
                "kind": "string",
                "name": "string",
                "path": "string",
                "source_profile": "string",
                "exists": "bool",
                "managed_blocks": [{
                    "id": "string",
                    "generator": "string",
                    "placement": "string",
                    "exists": "bool"
                }]
            }]
        }),
    );
}

fn assert_task_output_shape(label: &str, value: &Value) {
    let owner_shape = if value["task"]["owner"].is_null() {
        "null"
    } else {
        "string"
    };
    let priority_shape = if value["task"]["priority"].is_null() {
        "null"
    } else {
        "string"
    };
    let commits_shape = if value["task"]["commits"]
        .as_array()
        .expect("commits array")
        .is_empty()
    {
        json!([])
    } else {
        json!(["string"])
    };

    assert_contract_shape(
        label,
        value,
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "task": {
                "path": "string",
                "id": "string",
                "slug": "string",
                "title": "string",
                "status": "string",
                "owner": owner_shape,
                "priority": priority_shape,
                "created": "string",
                "updated": "string",
                "refs": {
                    "tracker": ["string"]
                },
                "commits": commits_shape,
                "body": "string"
            }
        }),
    );
}

fn assert_handoff_output_shape(label: &str, value: &Value) {
    assert_contract_shape(
        label,
        value,
        json!({
            "schema_version": "number",
            "repo_root": "string",
            "handoff": {
                "id": "string",
                "slug": "string",
                "title": "string",
                "status": "string",
                "created": "string",
                "updated": "string",
                "task_ids": ["string"],
                "template_path": "string",
                "path": "string",
                "body": "string"
            }
        }),
    );
}

fn llm_status_shape() -> Value {
    json!({
        "schema_version": "number",
        "summary": llm_summary_shape()
    })
}

fn llm_summary_shape() -> Value {
    json!({
        "repo_root": "string",
        "generated_at": "string",
        "profile": {
            "name": "string",
            "source": "string",
            "schema_version": "number",
            "installed_by_version": "string"
        },
        "repo": {
            "config_version": "number",
            "docs_enabled": "bool",
            "ci_provider": "string",
            "required_docs": ["string"],
            "required_files": ["string"],
            "forbidden_dirs": [],
            "semgrep_enabled": "bool",
            "conftest_enabled": "bool",
            "task_references_required": "bool"
        },
        "required_reading": [{
            "topic": "string",
            "path": "string",
            "exists": "bool"
        }],
        "active_tasks": [{
            "id": "string",
            "slug": "string",
            "title": "string",
            "status": "string",
            "owner": "string",
            "priority": "string",
            "updated": "string",
            "path": "string"
        }],
        "recent_handoffs": [{
            "id": "string",
            "slug": "string",
            "title": "string",
            "status": "string",
            "created": "string",
            "updated": "string",
            "task_ids": ["string"],
            "template_path": "string",
            "path": "string"
        }],
        "latest_handoff": {
            "id": "string",
            "slug": "string",
            "title": "string",
            "status": "string",
            "created": "string",
            "updated": "string",
            "task_ids": ["string"],
            "template_path": "string",
            "path": "string"
        },
        "doctor": {
            "ok": "bool",
            "diagnostics": [{
                "code": "string",
                "message": "string"
            }]
        }
    })
}

fn assert_contract_shape(label: &str, value: &Value, expected: Value) {
    assert_eq!(contract_shape(value), expected, "{label}");
}

fn contract_shape(value: &Value) -> Value {
    match value {
        Value::Null => json!("null"),
        Value::Bool(_) => json!("bool"),
        Value::Number(_) => json!("number"),
        Value::String(_) => json!("string"),
        Value::Array(items) => {
            if items.is_empty() {
                json!([])
            } else {
                json!([merge_shapes(items.iter().map(contract_shape).collect())])
            }
        }
        Value::Object(map) => {
            let mut shaped = Map::new();
            for (key, value) in map {
                shaped.insert(key.clone(), contract_shape(value));
            }
            Value::Object(shaped)
        }
    }
}

fn merge_shapes(shapes: Vec<Value>) -> Value {
    let mut iter = shapes.into_iter();
    let Some(first) = iter.next() else {
        return json!([]);
    };
    iter.fold(first, merge_two_shapes)
}

fn merge_two_shapes(left: Value, right: Value) -> Value {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let mut keys = BTreeSet::new();
            keys.extend(left.keys().cloned());
            keys.extend(right.keys().cloned());

            let mut merged = Map::new();
            for key in keys {
                match (left.get(&key), right.get(&key)) {
                    (Some(left_value), Some(right_value)) => {
                        merged.insert(
                            key,
                            merge_two_shapes(left_value.clone(), right_value.clone()),
                        );
                    }
                    (Some(value), None) | (None, Some(value)) => {
                        merged.insert(key, value.clone());
                    }
                    (None, None) => unreachable!("key comes from one side"),
                }
            }
            Value::Object(merged)
        }
        (Value::Array(left), Value::Array(right)) if left.is_empty() && right.is_empty() => {
            json!([])
        }
        (Value::Array(left), Value::Array(right)) if left.len() == 1 && right.len() == 1 => {
            json!([merge_two_shapes(left[0].clone(), right[0].clone())])
        }
        (left, right) if left == right => left,
        (Value::String(left), Value::String(right)) => {
            let mut parts = vec![left, right];
            parts.sort();
            parts.dedup();
            json!(parts.join("|"))
        }
        (left, right) => panic!("incompatible JSON shapes: {left:?} vs {right:?}"),
    }
}

fn normalized_json(value: &Value, repo: &Path) -> Value {
    normalize_value(value, repo, None)
}

fn normalize_value(value: &Value, repo: &Path, key: Option<&str>) -> Value {
    match value {
        Value::String(text) => {
            if Some(text.as_str()) == repo.to_str() {
                return json!("<repo>");
            }
            if matches!(
                key,
                Some("installed_by_version" | "created" | "updated" | "generated_at")
            ) {
                return json!(format!("<{}>", key.expect("key")));
            }
            if key == Some("run_id") {
                return json!("<run_id>");
            }
            if key == Some("run_path") {
                return json!(".guardrails/state/runs/pre-work-<run_id>.json");
            }
            json!(text)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| normalize_value(item, repo, None))
                .collect(),
        ),
        Value::Object(map) => {
            let mut normalized = Map::new();
            for (key, value) in map {
                normalized.insert(key.clone(), normalize_value(value, repo, Some(key)));
            }
            Value::Object(normalized)
        }
        other => other.clone(),
    }
}

fn ready_repo(temp: &Path, name: &str) -> PathBuf {
    let repo = temp.join(name);
    copy_dir(&fixture_root().join("bare-repo"), &repo);
    run_success(&[
        "init",
        "--target",
        repo_str(&repo),
        "--profile",
        "minimal",
        "--ci",
        "none",
    ]);
    customize_installed_docs(&repo);
    run_success(&[
        "init",
        "--target",
        repo_str(&repo),
        "--profile",
        "minimal",
        "--ci",
        "none",
        "--force",
    ]);
    fs::canonicalize(repo).expect("canonical repo")
}

fn write_adapter_contract_profile(root: &Path) -> PathBuf {
    let profile_dir = root.join("adapter-json-contract-profile");
    fs::create_dir_all(profile_dir.join("templates")).expect("profile templates");
    fs::write(profile_dir.join("templates/AGENTS.md"), "# Adapter JSON\n")
        .expect("agents template");
    fs::write(
        profile_dir.join("profile.toml"),
        r#"schema_version = 1
name = "adapter-json-contract"
description = "Adapter JSON contract profile"
default_ci = "none"
docs_enabled = false
required_docs = []
required_files = ["README.md", "AGENTS.md", ".guardrails/guardrails.toml"]
forbidden_dirs = []
includes_handoff = false

[[adapter_targets]]
path = ".claude/CLAUDE.md"
kind = "claude"
name = "Claude Code fixture"

[[managed_blocks]]
path = ".claude/CLAUDE.md"
id = "adapter-context"
generator = "repo_context_v1"
placement = "after_first_heading"

[semgrep]
enabled = false
binary = "semgrep"
config_paths = []
extra_args = []

[conftest]
enabled = false
binary = "conftest"
policy_paths = []
extra_args = []
"#,
    )
    .expect("profile");

    profile_dir
}

fn customize_installed_docs(repo: &Path) {
    fs::write(
        repo.join("AGENTS.md"),
        "# AGENTS.md\n\n## Repo Purpose\n\nThis fixture exercises stable JSON output contracts.\n\n## Guardrails\n\n- keep output fields stable for automation consumers\n- review schema changes before changing field types\n",
    )
    .expect("custom agents");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n## Current Approved Focus\n\n- task 0001 json-contracts keeps JSON contract coverage stable\n\n## Current Approved Next Steps\n\n1. run the output schema harness\n\n## Current Explicit Non-Goals\n\n- do not add future commands in this fixture\n",
    )
    .expect("custom tracker");
    fs::write(
        repo.join("docs/project/handoff-template.md"),
        "# Handoff Template\n\n## Current Result\n\n- JSON contract fixture is ready for handoff records\n\n## Verification\n\n- run project-guardrails check\n",
    )
    .expect("custom handoff");
}

fn write_invalid_duplicate_tasks(repo: &Path) {
    write_task_file(
        &repo.join(".guardrails/state/tasks/0001-first.md"),
        1,
        "first",
    );
    write_task_file(
        &repo.join(".guardrails/state/tasks/0001-second.md"),
        1,
        "second",
    );
}

fn write_task_file(path: &Path, id: u32, slug: &str) {
    fs::write(
        path,
        format!(
            "+++\nid = {id}\nslug = \"{slug}\"\ntitle = \"{}\"\nstatus = \"approved\"\ncreated = \"2026-04-22T00:00:00Z\"\nupdated = \"2026-04-22T00:00:00Z\"\n\n[refs]\ntracker = [\"docs/project/implementation-tracker.md\"]\n+++\n\n# {}\n",
            title_from_slug(slug),
            title_from_slug(slug)
        ),
    )
    .expect("task file");
}

fn title_from_slug(slug: &str) -> String {
    slug.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn init_git_repo(repo: &Path) {
    git(repo, &["init"]);
    git(repo, &["config", "user.email", "codex@example.invalid"]);
    git(repo, &["config", "user.name", "Codex"]);
}

fn stage_readme_change(repo: &Path) {
    fs::write(
        repo.join("README.md"),
        "# JSON Contracts Fixture\n\nThis README is staged for enforcement JSON checks.\n",
    )
    .expect("readme");
    git(repo, &["add", "README.md"]);
}

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_json_success(args: &[&str]) -> Value {
    parse_json(run_success(args))
}

fn run_json_failure(args: &[&str]) -> Value {
    let output = run_capture(args);
    assert!(
        !output.status.success(),
        "command unexpectedly passed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_json(output)
}

fn run_success(args: &[&str]) -> Output {
    let output = run_capture(args);
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn run_capture(args: &[&str]) -> Output {
    Command::new(binary_path())
        .args(args)
        .output()
        .expect("run guardrails")
}

fn parse_json(output: Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not valid json: {error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn repo_str(repo: &Path) -> &str {
    repo.to_str().expect("repo path")
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination dir");

    for entry in fs::read_dir(source).expect("read fixture dir") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_dir(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy file");
        }
    }
}
