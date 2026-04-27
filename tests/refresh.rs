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
fn refresh_reports_unchanged_declared_blocks() {
    let temp = TempDir::new().expect("temp dir");
    let repo = initialized_repo(temp.path(), "unchanged");

    let json = run_json_success(&["refresh", "--target", repo_str(&repo), "--format", "json"]);

    assert_eq!(json["ok"], true);
    assert_eq!(json["check"], false);
    assert_eq!(json["changed"], false);
    assert_eq!(
        json["changed_paths"]
            .as_array()
            .expect("changed paths")
            .len(),
        0
    );
    assert!(
        block_statuses(&json)
            .iter()
            .all(|status| status == "unchanged")
    );
}

#[test]
fn refresh_check_reports_stale_blocks_without_writing() {
    let temp = TempDir::new().expect("temp dir");
    let repo = initialized_repo(temp.path(), "check-stale");
    write_task_file(
        &repo.join(".guardrails/state/tasks/0001-refresh.md"),
        1,
        "refresh",
    );

    let before_agents = read_to_string(repo.join("AGENTS.md"));
    let before_tracker = read_to_string(repo.join("docs/project/implementation-tracker.md"));

    let check = run_json_failure(&[
        "refresh",
        "--target",
        repo_str(&repo),
        "--check",
        "--format",
        "json",
    ]);

    assert_eq!(check["ok"], false);
    assert_eq!(check["check"], true);
    assert_eq!(check["changed"], true);
    assert_eq!(block_statuses(&check), vec!["would_change", "would_change"]);
    assert_eq!(read_to_string(repo.join("AGENTS.md")), before_agents);
    assert_eq!(
        read_to_string(repo.join("docs/project/implementation-tracker.md")),
        before_tracker
    );

    let applied = run_json_success(&["refresh", "--target", repo_str(&repo), "--format", "json"]);
    assert_eq!(applied["ok"], true);
    assert_eq!(applied["changed"], true);
    assert_eq!(block_statuses(&applied), vec!["changed", "changed"]);
    assert!(read_to_string(repo.join("AGENTS.md")).contains("Refresh"));
}

#[test]
fn refresh_updates_guardrails_adapter_blocks_idempotently() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("guardrails-adapter-refresh");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_success(&[
        "init",
        "--target",
        repo_str(&repo),
        "--profile",
        "guardrails",
        "--ci",
        "none",
    ]);
    write_task_file(
        &repo.join(".guardrails/state/tasks/0001-claude.md"),
        1,
        "claude",
    );

    let before_claude = read_to_string(repo.join("CLAUDE.md"));
    let cursor_path = repo.join(".cursor/rules/project-guardrails.mdc");
    let mut cursor_with_local_notes = read_to_string(&cursor_path);
    cursor_with_local_notes.push_str("\nLocal Cursor note outside the managed block.\n");
    fs::write(&cursor_path, &cursor_with_local_notes).expect("cursor local note");

    let check = run_json_failure(&[
        "refresh",
        "--target",
        repo_str(&repo),
        "--check",
        "--format",
        "json",
    ]);

    assert_eq!(check["changed"], true);
    assert_eq!(
        check["changed_paths"],
        serde_json::json!([
            ".cursor/rules/project-guardrails.mdc",
            "AGENTS.md",
            "CLAUDE.md",
            "docs/project/implementation-tracker.md"
        ])
    );
    assert_eq!(
        block_statuses(&check),
        vec![
            "would_change",
            "would_change",
            "would_change",
            "would_change"
        ]
    );
    assert_eq!(read_to_string(repo.join("CLAUDE.md")), before_claude);
    assert_eq!(read_to_string(&cursor_path), cursor_with_local_notes);

    let applied = run_json_success(&["refresh", "--target", repo_str(&repo), "--format", "json"]);
    assert_eq!(applied["changed"], true);
    assert_eq!(
        block_statuses(&applied),
        vec!["changed", "changed", "changed", "changed"]
    );

    let refreshed_claude = read_to_string(repo.join("CLAUDE.md"));
    assert!(refreshed_claude.contains("`0001` Claude"));
    assert!(refreshed_claude.contains("<!-- guardrails:managed start id=adapter-context"));
    let refreshed_cursor = read_to_string(&cursor_path);
    assert!(refreshed_cursor.contains("`0001` Claude"));
    assert!(refreshed_cursor.contains("<!-- guardrails:managed start id=adapter-context"));
    assert!(refreshed_cursor.contains("Local Cursor note outside the managed block."));

    let idempotent = run_json_success(&[
        "refresh",
        "--target",
        repo_str(&repo),
        "--check",
        "--format",
        "json",
    ]);
    assert_eq!(idempotent["changed"], false);
    assert_eq!(
        block_statuses(&idempotent),
        vec!["unchanged", "unchanged", "unchanged", "unchanged"]
    );
    assert_eq!(read_to_string(repo.join("CLAUDE.md")), refreshed_claude);
    assert_eq!(read_to_string(&cursor_path), refreshed_cursor);
}

#[test]
fn refresh_inserts_missing_declared_block_into_existing_file() {
    let temp = TempDir::new().expect("temp dir");
    let repo = initialized_repo(temp.path(), "missing-block");

    fs::write(
        repo.join("AGENTS.md"),
        "# AGENTS.md\n\nHuman-owned intro stays here.\n\n## Human Section\n\nKeep this prose.\n",
    )
    .expect("agents");

    let json = run_json_success(&["refresh", "--target", repo_str(&repo), "--format", "json"]);

    assert_eq!(json["ok"], true);
    assert_eq!(json["changed"], true);
    assert_eq!(json["changed_paths"], serde_json::json!(["AGENTS.md"]));
    let agents = read_to_string(repo.join("AGENTS.md"));
    assert!(agents.contains("<!-- guardrails:managed start id=repo-context"));
    assert!(agents.contains("Human-owned intro stays here."));
    assert!(agents.contains("## Human Section\n\nKeep this prose."));
}

#[test]
fn refresh_reports_missing_target_file_without_creating_it() {
    let temp = TempDir::new().expect("temp dir");
    let repo = initialized_repo(temp.path(), "missing-file");
    fs::remove_file(repo.join("AGENTS.md")).expect("remove agents");

    let json = run_json_failure(&["refresh", "--target", repo_str(&repo), "--format", "json"]);

    assert_eq!(json["ok"], false);
    assert_eq!(json["changed"], false);
    assert_eq!(json["blocks"][0]["status"], "missing_file");
    assert!(has_diagnostic(&json, "managed_block_file_missing"));
    assert!(!repo.join("AGENTS.md").exists());
}

#[test]
fn refresh_reports_invalid_managed_block_markup() {
    let temp = TempDir::new().expect("temp dir");
    let repo = initialized_repo(temp.path(), "invalid-marker");
    fs::write(
        repo.join("AGENTS.md"),
        "# AGENTS.md\n\n<!-- guardrails:managed start id=repo-context generator=repo_context_v1 -->\nopen block\n",
    )
    .expect("invalid agents");

    let json = run_json_failure(&["refresh", "--target", repo_str(&repo), "--format", "json"]);

    assert_eq!(json["ok"], false);
    assert_eq!(json["blocks"][0]["status"], "invalid");
    assert!(has_diagnostic(&json, "managed_block_invalid"));
}

#[test]
fn refresh_reports_unknown_generators() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("unknown-generator");
    copy_dir(&fixture_root().join("bare-repo"), &repo);
    let profile_dir = write_custom_profile(temp.path(), "custom-refresh", "repo_context_v1");

    run_success(&[
        "init",
        "--target",
        repo_str(&repo),
        "--profile",
        "custom-refresh",
        "--profile-path",
        repo_str(&profile_dir),
        "--ci",
        "none",
    ]);
    fs::write(
        profile_dir.join("profile.toml"),
        custom_profile_toml("custom-refresh", "unknown_v1"),
    )
    .expect("rewrite profile");

    let json = run_json_failure(&["refresh", "--target", repo_str(&repo), "--format", "json"]);

    assert_eq!(json["ok"], false);
    assert_eq!(json["blocks"][0]["status"], "error");
    assert!(has_diagnostic(&json, "managed_block_generator_error"));
}

fn initialized_repo(temp: &Path, name: &str) -> PathBuf {
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
    fs::canonicalize(repo).expect("canonical repo")
}

fn write_custom_profile(root: &Path, name: &str, generator: &str) -> PathBuf {
    let profile_dir = root.join(format!("{name}-profile"));
    fs::create_dir_all(&profile_dir).expect("profile dir");
    fs::write(
        profile_dir.join("profile.toml"),
        custom_profile_toml(name, generator),
    )
    .expect("profile");
    profile_dir
}

fn custom_profile_toml(name: &str, generator: &str) -> String {
    format!(
        "schema_version = 1\nname = \"{name}\"\ndescription = \"Custom refresh profile\"\ndefault_ci = \"none\"\ndocs_enabled = false\nrequired_docs = []\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = false\n\n[[managed_blocks]]\npath = \"AGENTS.md\"\nid = \"repo-context\"\ngenerator = \"{generator}\"\nplacement = \"after_first_heading\"\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n"
    )
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

fn block_statuses(json: &Value) -> Vec<String> {
    json["blocks"]
        .as_array()
        .expect("blocks")
        .iter()
        .map(|block| block["status"].as_str().expect("status").to_string())
        .collect()
}

fn has_diagnostic(json: &Value, code: &str) -> bool {
    json["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .any(|diagnostic| diagnostic["code"] == code)
}

fn read_to_string(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("read file")
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

fn repo_str(path: &Path) -> &str {
    path.to_str().expect("path")
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
