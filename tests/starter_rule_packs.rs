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
fn guardrails_profile_enables_shipped_starter_rule_packs() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("guardrails-packs");
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

    let config = read_toml(repo.join(".guardrails/guardrails.toml"));
    let enabled = config["rules"]["rule_packs"]["enabled"]
        .as_array()
        .expect("enabled rule packs")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        enabled,
        vec!["llm-common-mistakes", "docs-freshness", "secret-safety"]
    );

    let forbidden_patterns = config["rules"]["forbidden_patterns"]
        .as_array()
        .expect("forbidden patterns");
    assert!(forbidden_patterns.len() >= 9);
    assert!(forbidden_patterns.iter().any(|entry| {
        entry["message"]
            .as_str()
            .is_some_and(|message| message.contains("llm-common-mistakes"))
    }));
    assert!(forbidden_patterns.iter().any(|entry| {
        entry["message"]
            .as_str()
            .is_some_and(|message| message.contains("secret-safety"))
    }));

    let link_requirements = config["rules"]["link_requirements"]
        .as_array()
        .expect("link requirements");
    assert!(link_requirements.iter().any(|entry| {
        entry["message"]
            .as_str()
            .is_some_and(|message| message.contains("docs-freshness"))
    }));

    let evidence_requirements = config["rules"]["evidence_requirements"]
        .as_array()
        .expect("evidence requirements");
    assert!(
        evidence_requirements
            .iter()
            .any(|entry| { entry["name"].as_str() == Some("source-change-context") })
    );
    assert!(
        evidence_requirements
            .iter()
            .any(|entry| { entry["name"].as_str() == Some("public-api-decision") })
    );
    assert!(
        evidence_requirements
            .iter()
            .any(|entry| { entry["name"].as_str() == Some("dependency-rationale") })
    );
    assert!(
        evidence_requirements
            .iter()
            .any(|entry| { entry["name"].as_str() == Some("infra-validation") })
    );
    assert!(
        evidence_requirements
            .iter()
            .any(|entry| { entry["name"].as_str() == Some("deleted-test-evidence") })
    );
}

#[test]
fn guardrails_starter_packs_block_common_ai_drift_patterns() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "guardrails-drift", "guardrails");

    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(
        repo.join("src/lib.rs"),
        r#"pub fn generated_draft() {
    // As an AI language model, I would implement this.
    // TODO: implement this later.
    // REMOVE_BEFORE_MERGE
    console.log("debug");
    // lorem ipsum placeholder implementation
    let _aws = "AKIA1234567890ABCDEF";
    let _private_key = "-----BEGIN OPENSSH PRIVATE KEY-----";
    let _token = "ghp_abcdefghijklmnopqrstuvwxyz1234567890";
    let password = "correcthorsebattery";
}
"#,
    )
    .expect("source");
    git(&repo, &["add", "src/lib.rs"]);

    let json = run_json_failure(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);

    assert_has_diagnostic(&json, "commit_link_requirement_missing", "docs-freshness");
    assert_has_diagnostic(&json, "commit_forbidden_pattern", "AI boilerplate");
    assert_has_diagnostic(
        &json,
        "commit_forbidden_pattern",
        "unfinished placeholder TODOs",
    );
    assert_has_diagnostic(
        &json,
        "commit_forbidden_pattern",
        "temporary commit blockers",
    );
    assert_has_diagnostic(
        &json,
        "commit_forbidden_pattern",
        "browser debug statements",
    );
    assert_has_diagnostic(&json, "commit_forbidden_pattern", "generated filler");
    assert_has_diagnostic(&json, "commit_forbidden_pattern", "AWS access key");
    assert_has_diagnostic(&json, "commit_forbidden_pattern", "private key material");
    assert_has_diagnostic(&json, "commit_forbidden_pattern", "GitHub token");
    assert_has_diagnostic(
        &json,
        "commit_forbidden_pattern",
        "inline secret assignment",
    );
}

#[test]
fn docs_freshness_pack_accepts_source_changes_with_companion_project_doc() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "guardrails-docs-freshness", "guardrails");

    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(
        repo.join("src/internal.rs"),
        "pub fn changed() -> bool { true }\n",
    )
    .expect("source");
    git(&repo, &["add", "src/internal.rs"]);

    let missing_doc = run_json_failure(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_has_diagnostic(
        &missing_doc,
        "commit_link_requirement_missing",
        "docs-freshness",
    );

    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- staged src/internal.rs with its tracker note.\n",
    )
    .expect("tracker");
    git(&repo, &["add", "docs/project/implementation-tracker.md"]);

    let ok = run_json_success(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(ok["ok"], true);
    assert_eq!(ok["diagnostics"].as_array().expect("diagnostics").len(), 0);
}

#[test]
fn docs_freshness_pack_requires_decision_log_for_public_api_changes() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "guardrails-api-freshness", "guardrails");

    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(
        repo.join("src/lib.rs"),
        "pub fn public_api_changed() -> bool { true }\n",
    )
    .expect("source");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- staged src/lib.rs with tracker context.\n",
    )
    .expect("tracker");
    git(
        &repo,
        &[
            "add",
            "src/lib.rs",
            "docs/project/implementation-tracker.md",
        ],
    );

    let missing_decision = run_json_failure(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_has_diagnostic(
        &missing_decision,
        "commit_evidence_requirement_missing",
        "decision-log",
    );

    fs::write(
        repo.join("docs/project/decision-log.md"),
        "# Decision Log\n\n- Public API change accepted for this fixture.\n",
    )
    .expect("decision log");
    git(&repo, &["add", "docs/project/decision-log.md"]);

    let ok = run_json_success(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(ok["ok"], true);
}

#[test]
fn docs_freshness_pack_requires_dependency_rationale_evidence() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "guardrails-dependency-freshness", "guardrails");

    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("cargo manifest");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- Cargo.toml changed.\n",
    )
    .expect("tracker");
    git(
        &repo,
        &[
            "add",
            "Cargo.toml",
            "docs/project/implementation-tracker.md",
        ],
    );

    let missing_rationale = run_json_failure(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_has_diagnostic(
        &missing_rationale,
        "commit_evidence_requirement_missing",
        "dependency manifest",
    );

    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- Rationale: Cargo.toml pins the fixture package metadata for tests.\n",
    )
    .expect("tracker with rationale");
    git(&repo, &["add", "docs/project/implementation-tracker.md"]);

    let ok = run_json_success(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(ok["ok"], true);
}

#[test]
fn docs_freshness_pack_requires_infra_validation_or_rollback_evidence() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "guardrails-infra-freshness", "guardrails");

    fs::create_dir_all(repo.join("infra")).expect("infra dir");
    fs::write(
        repo.join("infra/main.tf"),
        "resource \"null_resource\" \"fixture\" {}\n",
    )
    .expect("infra");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- Infra changed.\n",
    )
    .expect("tracker");
    git(
        &repo,
        &[
            "add",
            "infra/main.tf",
            "docs/project/implementation-tracker.md",
        ],
    );

    let missing_validation = run_json_failure(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_has_diagnostic(
        &missing_validation,
        "commit_evidence_requirement_missing",
        "infra changes",
    );

    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- Validation: terraform fixture syntax was reviewed. Rollback: remove infra/main.tf.\n",
    )
    .expect("tracker with validation");
    git(&repo, &["add", "docs/project/implementation-tracker.md"]);

    let ok = run_json_success(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(ok["ok"], true);
}

#[test]
fn docs_freshness_pack_requires_deleted_test_evidence_or_replacement() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(
        temp.path(),
        "guardrails-deleted-test-freshness",
        "guardrails",
    );

    fs::create_dir_all(repo.join("tests")).expect("tests dir");
    fs::write(
        repo.join("tests/old_behavior.rs"),
        "#[test]\nfn old_behavior() {}\n",
    )
    .expect("old test");
    git(&repo, &["add", "tests/old_behavior.rs"]);
    git(&repo, &["commit", "-m", "Add old test"]);

    git(&repo, &["rm", "tests/old_behavior.rs"]);
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- Test file changed.\n",
    )
    .expect("tracker");
    git(&repo, &["add", "docs/project/implementation-tracker.md"]);

    let missing_evidence = run_json_failure(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_has_diagnostic(
        &missing_evidence,
        "commit_evidence_requirement_missing",
        "deleted tests",
    );

    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- Deleted test was obsolete; replacement test coverage now lives in tests/new_behavior.rs.\n",
    )
    .expect("tracker with deleted test evidence");
    git(&repo, &["add", "docs/project/implementation-tracker.md"]);

    let ok_with_notes = run_json_success(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(ok_with_notes["ok"], true);

    git(&repo, &["reset", "--hard", "HEAD"]);
    git(&repo, &["rm", "tests/old_behavior.rs"]);
    fs::create_dir_all(repo.join("tests")).expect("tests dir");
    fs::write(
        repo.join("tests/new_behavior.rs"),
        "#[test]\nfn new_behavior() {}\n",
    )
    .expect("replacement test");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n- Test coverage moved with the replacement path.\n",
    )
    .expect("tracker for replacement");
    git(
        &repo,
        &[
            "add",
            "tests/new_behavior.rs",
            "docs/project/implementation-tracker.md",
        ],
    );

    let ok_with_replacement = run_json_success(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(ok_with_replacement["ok"], true);
}

#[test]
fn minimal_profile_remains_neutral_without_starter_rule_packs() {
    let temp = TempDir::new().expect("temp dir");
    let repo = ready_repo(temp.path(), "minimal-neutral", "minimal");

    let config = read_toml(repo.join(".guardrails/guardrails.toml"));
    assert_eq!(
        config["rules"]["rule_packs"]["enabled"]
            .as_array()
            .expect("enabled rule packs")
            .len(),
        0
    );
    assert_eq!(
        config["rules"]["evidence_requirements"]
            .as_array()
            .expect("evidence requirements")
            .len(),
        0
    );

    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(
        repo.join("src/lib.rs"),
        "pub fn sample() { /* As an AI language model */ let _key = \"AKIA1234567890ABCDEF\"; }\n",
    )
    .expect("source");
    git(&repo, &["add", "src/lib.rs"]);

    let ok = run_json_success(&[
        "pre-commit",
        "--target",
        repo_str(&repo),
        "--format",
        "json",
    ]);
    assert_eq!(ok["ok"], true);
}

fn ready_repo(temp: &Path, name: &str, profile: &str) -> PathBuf {
    let repo = temp.join(name);
    copy_dir(&fixture_root().join("bare-repo"), &repo);
    run_success(&[
        "init",
        "--target",
        repo_str(&repo),
        "--profile",
        profile,
        "--ci",
        "none",
    ]);
    init_git_repo(&repo);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "Initial fixture"]);
    fs::canonicalize(repo).expect("canonical repo")
}

fn init_git_repo(repo: &Path) {
    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "tests@example.com"]);
    run_git(repo, &["config", "user.name", "Guardrails Tests"]);
}

fn git(repo: &Path, args: &[&str]) {
    run_git(repo, args);
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_success(args: &[&str]) {
    let output = run_guardrails(args);
    assert!(
        output.status.success(),
        "project-guardrails {} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_json_success(args: &[&str]) -> Value {
    let output = run_guardrails(args);
    assert!(
        output.status.success(),
        "project-guardrails {} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid success json")
}

fn run_json_failure(args: &[&str]) -> Value {
    let output = run_guardrails(args);
    assert!(
        !output.status.success(),
        "project-guardrails {} unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid failure json")
}

fn run_guardrails(args: &[&str]) -> Output {
    Command::new(binary_path())
        .args(args)
        .output()
        .expect("run guardrails")
}

fn assert_has_diagnostic(json: &Value, code: &str, message_part: &str) {
    let diagnostics = json["diagnostics"].as_array().expect("diagnostics");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["code"].as_str() == Some(code)
                && diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(message_part))
        }),
        "missing diagnostic code={code} message_part={message_part}; diagnostics={diagnostics:#?}"
    );
}

fn read_toml(path: impl AsRef<Path>) -> toml::Value {
    let raw = fs::read_to_string(path).expect("read toml");
    toml::from_str(&raw).expect("parse toml")
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create destination dir");
    for entry in fs::read_dir(source).expect("read source dir") {
        let entry = entry.expect("dir entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy file");
        }
    }
}

fn repo_str(path: &Path) -> &str {
    path.to_str().expect("utf-8 path")
}
