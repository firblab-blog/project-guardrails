use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;
use tempfile::TempDir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_project-guardrails"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

#[test]
fn adapters_list_reports_empty_targets_for_neutral_profile() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("minimal-adapters");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "none",
    ]);

    let output = run_guardrails_capture(&[
        "adapters",
        "list",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid adapters json");

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["profile"], "minimal");
    assert_eq!(json["profile_source"], "built-in:minimal");
    assert!(json["adapters"].as_array().expect("adapters").is_empty());
}

#[test]
fn adapters_list_reports_builtin_guardrails_adapter_targets() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("guardrails-adapters");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "guardrails",
        "--ci",
        "none",
    ]);

    assert!(repo.join("CLAUDE.md").exists());
    assert!(repo.join(".cursor/rules/project-guardrails.mdc").exists());

    let output = run_guardrails_capture(&[
        "adapters",
        "list",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid adapters json");
    let adapters = json["adapters"].as_array().expect("adapters");
    assert_eq!(adapters.len(), 2);

    let claude = find_adapter(adapters, "CLAUDE.md");
    assert_adapter(
        claude,
        "claude",
        "Claude Code",
        "CLAUDE.md",
        "built-in:guardrails",
    );

    let cursor = find_adapter(adapters, ".cursor/rules/project-guardrails.mdc");
    assert_adapter(
        cursor,
        "cursor",
        "Cursor",
        ".cursor/rules/project-guardrails.mdc",
        "built-in:guardrails",
    );
}

fn find_adapter<'a>(adapters: &'a [Value], path: &str) -> &'a Value {
    adapters
        .iter()
        .find(|adapter| adapter["path"] == path)
        .unwrap_or_else(|| panic!("missing adapter for {path}"))
}

fn assert_adapter(adapter: &Value, kind: &str, name: &str, path: &str, source_profile: &str) {
    assert_eq!(adapter["kind"], kind);
    assert_eq!(adapter["name"], name);
    assert_eq!(adapter["path"], path);
    assert_eq!(adapter["source_profile"], source_profile);
    assert_eq!(adapter["exists"], true);
    assert_eq!(adapter["managed_blocks"][0]["id"], "adapter-context");
    assert_eq!(adapter["managed_blocks"][0]["generator"], "repo_context_v1");
    assert_eq!(
        adapter["managed_blocks"][0]["placement"],
        "after_first_heading"
    );
    assert_eq!(adapter["managed_blocks"][0]["exists"], true);
}

#[test]
fn adapters_list_reports_profile_declared_targets_without_creating_files() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("adapter-profile-repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);
    let profile_dir = write_adapter_profile(temp.path());

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "adapter-fixture",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    let adapter_path = repo.join(".claude/CLAUDE.md");
    assert!(!adapter_path.exists());

    let output = run_guardrails_capture(&[
        "adapters",
        "list",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid adapters json");
    let adapter = &json["adapters"].as_array().expect("adapters")[0];

    assert_eq!(adapter["kind"], "claude");
    assert_eq!(adapter["name"], "Claude Code fixture");
    assert_eq!(adapter["path"], ".claude/CLAUDE.md");
    assert!(
        adapter["source_profile"]
            .as_str()
            .expect("source profile")
            .starts_with("custom:")
    );
    assert_eq!(adapter["exists"], false);
    assert_eq!(adapter["managed_blocks"][0]["id"], "adapter-context");
    assert_eq!(adapter["managed_blocks"][0]["generator"], "repo_context_v1");
    assert_eq!(
        adapter["managed_blocks"][0]["placement"],
        "after_first_heading"
    );
    assert_eq!(adapter["managed_blocks"][0]["exists"], false);
    assert!(!adapter_path.exists());
}

#[test]
fn adapters_list_reports_existing_target_and_managed_block_metadata() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("adapter-existing-repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);
    let profile_dir = write_adapter_profile(temp.path());

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "adapter-fixture",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    let adapter_path = repo.join(".claude/CLAUDE.md");
    fs::create_dir_all(adapter_path.parent().expect("adapter dir")).expect("adapter dir");
    fs::write(
        &adapter_path,
        "# Claude Fixture\n\n<!-- guardrails:managed start id=adapter-context generator=repo_context_v1 -->\nmanaged fixture\n<!-- guardrails:managed end id=adapter-context -->\n",
    )
    .expect("adapter fixture");

    let output = run_guardrails_capture(&[
        "adapters",
        "list",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid adapters json");
    let adapter = &json["adapters"].as_array().expect("adapters")[0];

    assert_eq!(adapter["exists"], true);
    assert_eq!(adapter["managed_blocks"][0]["exists"], true);
}

#[test]
fn init_records_adapter_targets_as_review_managed_paths_without_materializing_them() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("adapter-lock-repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);
    let profile_dir = write_adapter_profile(temp.path());

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "adapter-fixture",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    let profile_lock: toml::Value = toml::from_str(&profile_lock).expect("profile lock toml");
    let managed_paths = profile_lock["managed_paths"]
        .as_array()
        .expect("managed paths");

    assert!(managed_paths.iter().any(|entry| {
        entry["path"].as_str() == Some(".claude/CLAUDE.md")
            && entry["stale_action"].as_str() == Some("review")
    }));
    assert!(!repo.join(".claude/CLAUDE.md").exists());
}

fn write_adapter_profile(root: &Path) -> PathBuf {
    let profile_dir = root.join("adapter-fixture-profile");
    fs::create_dir_all(profile_dir.join("templates")).expect("profile templates");
    fs::write(
        profile_dir.join("templates/AGENTS.md"),
        "# Adapter Fixture\n",
    )
    .expect("agents template");
    fs::write(
        profile_dir.join("profile.toml"),
        r#"schema_version = 1
name = "adapter-fixture"
description = "Adapter target fixture"
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

fn run_guardrails(args: &[&str]) {
    let output = run_guardrails_capture(args);
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_guardrails_capture(args: &[&str]) -> Output {
    Command::new(binary_path())
        .args(args)
        .output()
        .expect("run project-guardrails")
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
