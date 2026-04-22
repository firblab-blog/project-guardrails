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
fn init_materializes_expected_files_for_builtin_profile() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("bare");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    assert!(repo.join(".guardrails/guardrails.toml").exists());
    assert!(repo.join(".guardrails/profile.lock").exists());
    assert!(repo.join("AGENTS.md").exists());
    assert!(repo.join("docs/project/implementation-tracker.md").exists());
    assert!(repo.join("docs/project/handoff-template.md").exists());
    assert!(repo.join(".pre-commit-config.yaml").exists());
    assert!(repo.join(".github/workflows/guardrails.yml").exists());

    let pre_commit_config =
        fs::read_to_string(repo.join(".pre-commit-config.yaml")).expect("pre-commit config");
    assert!(pre_commit_config.contains("project-guardrails pre-commit --target ."));
    assert!(pre_commit_config.contains("project-guardrails commit-msg-check --target ."));

    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    let profile_lock: toml::Value = toml::from_str(&profile_lock).expect("structured profile lock");
    let managed_paths = profile_lock["managed_paths"]
        .as_array()
        .expect("managed_paths array");
    assert!(managed_paths.iter().any(|entry| {
        entry["path"].as_str() == Some(".guardrails/guardrails.toml")
            && entry["stale_action"].as_str() == Some("review")
    }));
    assert!(managed_paths.iter().any(|entry| {
        entry["path"].as_str() == Some("AGENTS.md")
            && entry["stale_action"].as_str() == Some("review")
    }));
    assert!(managed_paths.iter().any(|entry| {
        entry["path"].as_str() == Some("docs/project/implementation-tracker.md")
            && entry["stale_action"].as_str() == Some("review")
    }));
    assert!(managed_paths.iter().any(|entry| {
        entry["path"].as_str() == Some(".pre-commit-config.yaml")
            && entry["stale_action"].as_str() == Some("review")
    }));
    assert!(managed_paths.iter().any(|entry| {
        entry["path"].as_str() == Some(".github/workflows/guardrails.yml")
            && entry["stale_action"].as_str() == Some("remove")
    }));
}

#[test]
fn init_github_ci_template_runs_version_doctor_and_check_for_default_path() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("github-ci-contract");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    let workflow =
        fs::read_to_string(repo.join(".github/workflows/guardrails.yml")).expect("workflow");
    assert!(workflow.contains("name: guardrails"));
    assert!(workflow.contains("project-guardrails --version"));
    assert!(workflow.contains("project-guardrails doctor --target ."));
    assert!(workflow.contains("project-guardrails check --target ."));
    assert!(workflow.contains("does not install"));
    assert!(!repo.join(".gitlab-ci.guardrails.yml").exists());
}

#[test]
fn init_gitlab_ci_template_runs_version_doctor_and_check_for_default_path() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("gitlab-ci-contract");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "gitlab",
    ]);

    let workflow = fs::read_to_string(repo.join(".gitlab-ci.guardrails.yml")).expect("workflow");
    assert!(workflow.contains("stages:"));
    assert!(workflow.contains("guardrails:check:"));
    assert!(workflow.contains("project-guardrails --version"));
    assert!(workflow.contains("project-guardrails doctor --target ."));
    assert!(workflow.contains("project-guardrails check --target ."));
    assert!(workflow.contains("does not install"));
    assert!(!repo.join(".github/workflows/guardrails.yml").exists());
}

#[test]
fn init_uses_profile_default_ci_when_ci_is_omitted() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("default-ci");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("default-ci-profile");
    fs::create_dir_all(profile_dir.join("templates")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"default-ci\"\ndescription = \"Profile default CI test\"\ndefault_ci = \"gitlab\"\ndocs_enabled = false\nrequired_docs = []\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = false\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("templates/AGENTS.md"),
        "# Default CI AGENTS\n",
    )
    .expect("agents");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "default-ci",
        "--profile-path",
        profile_dir.to_str().unwrap(),
    ]);

    let config = fs::read_to_string(repo.join(".guardrails/guardrails.toml")).expect("config");
    assert!(config.contains("provider = \"gitlab\""));
    assert!(repo.join(".gitlab-ci.guardrails.yml").exists());
}

#[test]
fn init_defaults_to_minimal_when_profile_is_omitted() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("default-profile");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&["init", "--target", repo.to_str().unwrap()]);

    let config = fs::read_to_string(repo.join(".guardrails/guardrails.toml")).expect("config");
    assert!(config.contains("profile = \"minimal\""));
    assert!(config.contains("profile_source = \"built-in:minimal\""));
    assert!(repo.join("docs/project/implementation-tracker.md").exists());
    assert!(repo.join("docs/project/handoff-template.md").exists());
    assert!(!repo.join("docs/project/decision-log.md").exists());
    assert!(!repo.join("docs/best-practices").exists());
}

#[test]
fn profiles_list_surfaces_builtin_profiles_in_text_and_json() {
    let text_output = run_guardrails_capture(&["profiles", "list"]);
    let stdout = String::from_utf8_lossy(&text_output.stdout);

    assert!(stdout.contains("Built-in profiles"));
    assert!(stdout.contains("minimal (default)"));
    assert!(stdout.contains("docs-driven"));
    assert!(stdout.contains("guardrails (opt-in)"));
    assert!(stdout.contains("Use `project-guardrails init --profile <name>`"));

    let json_output = run_guardrails_capture(&["profiles", "list", "--format", "json"]);
    let stdout = String::from_utf8_lossy(&json_output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid profiles json");
    assert_eq!(json["schema_version"], 1);
    assert!(
        json["profiles"]
            .as_array()
            .expect("profiles array")
            .iter()
            .any(|item| {
                item["name"] == "minimal"
                    && item["is_default"] == true
                    && item["is_opt_in"] == false
            })
    );
    assert!(
        json["profiles"]
            .as_array()
            .expect("profiles array")
            .iter()
            .any(|item| {
                item["name"] == "guardrails"
                    && item["is_default"] == false
                    && item["is_opt_in"] == true
            })
    );
}

#[test]
fn init_guardrails_profile_installs_seeded_doctrine_docs() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("guardrails-profile");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "guardrails",
        "--ci",
        "github",
    ]);

    let config = fs::read_to_string(repo.join(".guardrails/guardrails.toml")).expect("config");
    assert!(config.contains("profile = \"guardrails\""));
    assert!(config.contains("profile_source = \"built-in:guardrails\""));
    assert!(repo.join("docs/project/decision-log.md").exists());
    assert!(
        repo.join("docs/project/implementation-invariants.md")
            .exists()
    );
    assert!(repo.join("docs/best-practices/change-safety.md").exists());
    assert!(
        repo.join("docs/best-practices/ci-and-enforcement.md")
            .exists()
    );
    assert!(
        repo.join("docs/best-practices/docs-and-handoffs.md")
            .exists()
    );
    assert!(repo.join("docs/best-practices/repo-shaping.md").exists());

    let agents = fs::read_to_string(repo.join("AGENTS.md")).expect("agents");
    assert!(agents.contains("This repository uses the built-in `guardrails` profile."));
    assert!(agents.contains("Work In Layers"));
    assert!(
        agents.contains("do not make the opinionated doctrine profile the default bootstrap path")
    );

    let tracker =
        fs::read_to_string(repo.join("docs/project/implementation-tracker.md")).expect("tracker");
    assert!(tracker.contains("keep the bootstrap utility portable and Rust-first"));

    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    assert!(profile_lock.contains("path = \"docs/project/decision-log.md\""));
    assert!(profile_lock.contains("path = \"docs/project/implementation-invariants.md\""));
    assert!(profile_lock.contains("path = \"docs/best-practices/change-safety.md\""));
}

#[test]
fn init_help_mentions_default_profile_and_builtin_summaries() {
    let output = run_guardrails_capture(&["init", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("`minimal` is the default neutral baseline"));
    assert!(stdout.contains("Built-in profiles:"));
    assert!(stdout.contains("minimal      Neutral cross-language baseline"));
    assert!(stdout.contains("docs-driven  Neutral baseline plus a required decision log."));
    assert!(stdout.contains("guardrails   Opt-in FirbLab-style doctrine profile"));
    assert!(stdout.contains("project-guardrails profiles list"));
    assert!(stdout.contains("project-guardrails init --target . --profile minimal --ci github"));
}

#[test]
fn init_dry_run_separates_edit_first_run_next_and_planned_files() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("dry-run");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let output = run_guardrails_capture(&[
        "init",
        "--dry-run",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("edit_first:"));
    assert!(stdout.contains("run_next:"));
    assert!(stdout.contains("planned_files:"));
    assert!(stdout.contains("profile_choice: default neutral baseline"));
    assert!(stdout.contains("ci_choice: writes a GitHub Actions guardrails workflow"));
    assert!(stdout.contains("tool_managed:"));
    assert!(stdout.contains("README.md: create or confirm the repo overview"));
    assert!(stdout.contains("AGENTS.md: set the repo-specific instructions"));
    assert!(stdout.contains("docs/project/handoff-template.md: replace starter handoff guidance"));
    assert!(stdout.contains("project-guardrails doctor --target"));
    assert!(stdout.contains("project-guardrails handoff list --target"));
    assert!(!stdout.contains("project-guardrails handoff --target"));
}

#[test]
fn init_success_output_recommends_decision_log_for_docs_driven_profile() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("docs-driven-success");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let output = run_guardrails_capture(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--ci",
        "github",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout
            .contains("docs/project/decision-log.md: record the first important project decisions")
    );
    assert!(stdout.contains("Created:"));
    assert!(stdout.contains("Tool-managed:"));
    assert!(stdout.contains(
        "Why this profile: use this when you want the neutral baseline plus a required decision log"
    ));
    assert!(stdout.contains("CI choice: writes a GitHub Actions guardrails workflow"));
    assert!(
        stdout.contains(
            ".guardrails/profile.lock records tool-managed paths and stale-file behavior"
        )
    );
    assert!(stdout.contains("project-guardrails handoff list --target "));
    assert!(!stdout.contains("project-guardrails handoff --target "));
}

#[test]
fn init_success_output_lists_created_files_and_ownership_expectations() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("guided-success");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let output = run_guardrails_capture(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "gitlab",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Created:"));
    assert!(stdout.contains(".guardrails/guardrails.toml"));
    assert!(stdout.contains(".guardrails/profile.lock"));
    assert!(stdout.contains("docs/project/handoff-template.md"));
    assert!(stdout.contains(".gitlab-ci.guardrails.yml"));
    assert!(stdout.contains("Tool-managed:"));
    assert!(stdout.contains("Review-only by default: docs, AGENTS.md, config, and copied assets stay editable in your repo"));
    assert!(stdout.contains("CI file: the generated CI workflow is tool-managed and may be auto-removed later if you switch CI providers"));
    assert!(stdout.contains("Edit these first:"));
    assert!(stdout.contains("README.md"));
    assert!(stdout.contains("docs/project/handoff-template.md"));
    assert!(stdout.contains("Run these next:"));
    assert!(stdout.contains("project-guardrails handoff list --target "));
    assert!(!stdout.contains("project-guardrails handoff --target "));
}

#[test]
fn init_success_output_skips_handoff_command_when_profile_disables_it() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("no-handoff");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("no-handoff-profile");
    fs::create_dir_all(profile_dir.join("templates")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"no-handoff\"\ndescription = \"Profile without handoff\"\ndefault_ci = \"none\"\ndocs_enabled = false\nrequired_docs = []\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = false\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("templates/AGENTS.md"),
        "# No Handoff AGENTS\n",
    )
    .expect("agents");

    let output = run_guardrails_capture(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "no-handoff",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!stdout.contains("project-guardrails handoff --target"));
}

#[test]
fn init_uses_profile_owned_root_markers_and_workflow_destination() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("profile-owned-defaults");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("profile-owned-defaults-profile");
    fs::create_dir_all(profile_dir.join("templates/.github/workflows/ci")).expect("profile ci");
    fs::create_dir_all(profile_dir.join("templates")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"profile-owned-defaults\"\ndescription = \"Profile-owned defaults test\"\ndefault_ci = \"github\"\nroot_markers = [\".git\", \".jj\"]\ndocs_enabled = false\nrequired_docs = []\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = false\n\n[workflow_paths]\ngithub = \".github/workflows/ci/guardrails.yml\"\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("templates/AGENTS.md"),
        "# Profile Defaults AGENTS\n",
    )
    .expect("agents");
    fs::write(
        profile_dir.join("templates/.github/workflows/guardrails.yml"),
        "name: profile-owned-defaults\n",
    )
    .expect("workflow template");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "profile-owned-defaults",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "github",
    ]);

    let config = fs::read_to_string(repo.join(".guardrails/guardrails.toml")).expect("config");
    let config: toml::Value = toml::from_str(&config).expect("valid config toml");
    assert_eq!(
        config["project"]["root_markers"]
            .as_array()
            .expect("root_markers array")
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>(),
        vec![".git", ".jj"]
    );
    assert_eq!(
        config["ci"]["workflow_path"].as_str(),
        Some(".github/workflows/ci/guardrails.yml")
    );
    assert!(repo.join(".github/workflows/ci/guardrails.yml").exists());

    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    assert!(profile_lock.contains("path = \".github/workflows/ci/guardrails.yml\""));
}

#[cfg(unix)]
#[test]
fn init_supports_custom_profile_paths_and_external_engines() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let tools_dir = temp.path().join("tools");
    fs::create_dir_all(&tools_dir).expect("tools dir");

    let semgrep_log = temp.path().join("semgrep.log");
    let conftest_log = temp.path().join("conftest.log");
    let semgrep_bin = tools_dir.join("fake-semgrep");
    let conftest_bin = tools_dir.join("fake-conftest");
    write_script(
        &semgrep_bin,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo semgrep-test; exit 0; fi\nprintf '%s\\n' \"$@\" > \"{}\"\nexit 0\n",
            semgrep_log.display()
        ),
    );
    write_script(
        &conftest_bin,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo conftest-test; exit 0; fi\nprintf '%s\\n' \"$@\" > \"{}\"\nexit 0\n",
            conftest_log.display()
        ),
    );

    let profile_dir = temp.path().join("custom-profile");
    fs::create_dir_all(profile_dir.join("templates/docs/project")).expect("profile templates");
    fs::create_dir_all(profile_dir.join("templates/.github/workflows")).expect("profile ci");
    fs::create_dir_all(profile_dir.join("assets/rules/semgrep")).expect("profile assets");
    fs::write(
        profile_dir.join("profile.toml"),
        format!(
            "schema_version = 7\nname = \"custom-docs\"\ndescription = \"Custom profile path test\"\ndefault_ci = \"github\"\ndocs_enabled = true\nrequired_docs = [\"docs/project/implementation-tracker.md\", \"docs/project/custom-playbook.md\"]\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = [\"server\"]\nincludes_handoff = true\n\n[semgrep]\nenabled = true\nbinary = \"{}\"\nconfig_paths = [\"rules/semgrep/custom.yml\"]\nextra_args = [\"--error\"]\n\n[conftest]\nenabled = true\nbinary = \"{}\"\npolicy_paths = [\"policy/opa\"]\nextra_args = [\"--no-color\"]\n",
            semgrep_bin.display(),
            conftest_bin.display()
        ),
    )
    .expect("profile");
    fs::write(profile_dir.join("templates/AGENTS.md"), "# Custom AGENTS\n").expect("agents");
    fs::write(
        profile_dir.join("templates/.github/workflows/guardrails.yml"),
        "name: custom-guardrails\n",
    )
    .expect("github workflow");
    fs::write(
        profile_dir.join("templates/docs/project/custom-playbook.md"),
        "# Custom Playbook\n",
    )
    .expect("playbook");
    fs::write(
        profile_dir.join("assets/rules/semgrep/custom.yml"),
        "rules: []\n",
    )
    .expect("semgrep config");
    fs::create_dir_all(repo.join("policy/opa")).expect("opa policy");
    fs::write(repo.join("policy/opa/example.rego"), "package example\n").expect("opa policy");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "ignored-when-profile-path-is-set",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "github",
    ]);

    assert_eq!(
        fs::read_to_string(repo.join("AGENTS.md")).expect("agents contents"),
        "# Custom AGENTS\n"
    );
    assert!(repo.join("docs/project/custom-playbook.md").exists());
    assert_eq!(
        fs::read_to_string(repo.join(".github/workflows/guardrails.yml")).expect("workflow"),
        "name: custom-guardrails\n"
    );
    assert!(repo.join("rules/semgrep/custom.yml").exists());

    customize_installed_docs(&repo);
    fs::write(
        repo.join("docs/project/custom-playbook.md"),
        "# Custom Playbook\n\n## Usage\n\n- Follow the repo-specific flow.\n",
    )
    .expect("custom playbook");

    run_guardrails(&["check", "--target", repo.to_str().unwrap()]);

    let semgrep_output = fs::read_to_string(&semgrep_log).expect("semgrep log");
    assert!(semgrep_output.contains("scan"));
    assert!(semgrep_output.contains("--config"));
    assert!(semgrep_output.contains("--error"));

    let conftest_output = fs::read_to_string(&conftest_log).expect("conftest log");
    assert!(conftest_output.contains("test"));
    assert!(conftest_output.contains("--policy"));
    assert!(conftest_output.contains("--no-color"));

    let config = fs::read_to_string(repo.join(".guardrails/guardrails.toml")).expect("config");
    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    assert!(config.contains("profile_source = \"custom:"));
    assert!(config.contains("profile_schema_version = 7"));
    assert!(config.contains("installed_by_version = \"0.1.16\""));
    assert!(profile_lock.contains("path = \"rules/semgrep/custom.yml\""));
}

#[test]
fn init_copies_binary_profile_assets_without_utf8_assumptions() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("binary-assets");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("binary-assets-profile");
    fs::create_dir_all(profile_dir.join("assets/support")).expect("profile assets");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"binary-assets\"\ndescription = \"Binary asset copy test\"\ndefault_ci = \"none\"\ndocs_enabled = false\nrequired_docs = []\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = false\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("assets/support/logo.bin"),
        [0_u8, 159_u8, 255_u8, 13_u8, 10_u8],
    )
    .expect("binary asset");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "binary-assets",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    assert_eq!(
        fs::read(profile_dir.join("assets/support/logo.bin")).expect("source bytes"),
        fs::read(repo.join("support/logo.bin")).expect("copied bytes")
    );
}

#[test]
fn init_handles_repo_paths_with_spaces() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("repo with spaces");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--ci",
        "gitlab",
    ]);

    assert!(repo.join(".guardrails/guardrails.toml").exists());
    assert!(repo.join(".gitlab-ci.guardrails.yml").exists());
    assert!(repo.join("docs/project/decision-log.md").exists());
}

#[test]
fn init_preserves_existing_repo_files_and_reports_kept_existing_paths() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("existing-repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    fs::create_dir_all(repo.join("docs/project")).expect("project docs");
    fs::create_dir_all(repo.join(".github/workflows")).expect("workflow dir");
    fs::write(
        repo.join("README.md"),
        "# Existing Repo\n\nThis README already belongs to the repository.\n",
    )
    .expect("readme");
    fs::write(
        repo.join("AGENTS.md"),
        "# Existing AGENTS\n\nThis repo already has repo-owned collaboration guidance.\n",
    )
    .expect("agents");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Existing Tracker\n\n- keep the current tracker content\n",
    )
    .expect("tracker");
    fs::write(
        repo.join("docs/project/handoff-template.md"),
        "# Existing Handoff\n\n- keep the current handoff format\n",
    )
    .expect("handoff");
    fs::write(
        repo.join(".github/workflows/guardrails.yml"),
        "name: existing-guardrails-workflow\n",
    )
    .expect("workflow");

    let output = run_guardrails_capture(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        fs::read_to_string(repo.join("README.md")).expect("readme contents"),
        "# Existing Repo\n\nThis README already belongs to the repository.\n"
    );
    let agents = fs::read_to_string(repo.join("AGENTS.md")).expect("agents contents");
    assert!(agents.starts_with("# Existing AGENTS"));
    assert!(agents.contains("This repo already has repo-owned collaboration guidance."));
    assert!(agents.contains("<!-- guardrails:managed start id=repo-context"));
    let tracker =
        fs::read_to_string(repo.join("docs/project/implementation-tracker.md")).expect("tracker");
    assert!(tracker.starts_with("# Existing Tracker"));
    assert!(tracker.contains("- keep the current tracker content"));
    assert!(tracker.contains("<!-- guardrails:managed start id=task-sync"));
    assert_eq!(
        fs::read_to_string(repo.join("docs/project/handoff-template.md"))
            .expect("handoff contents"),
        "# Existing Handoff\n\n- keep the current handoff format\n"
    );
    assert_eq!(
        fs::read_to_string(repo.join(".github/workflows/guardrails.yml"))
            .expect("workflow contents"),
        "name: existing-guardrails-workflow\n"
    );

    assert!(stdout.contains("Kept existing:"));
    assert!(stdout.contains("/README.md"));
    assert!(stdout.contains("/AGENTS.md"));
    assert!(stdout.contains("/docs/project/implementation-tracker.md"));
    assert!(stdout.contains("/docs/project/handoff-template.md"));
    assert!(stdout.contains("/.github/workflows/guardrails.yml"));
    assert!(repo.join(".guardrails/guardrails.toml").exists());
    assert!(repo.join(".guardrails/profile.lock").exists());
}

#[test]
fn init_reports_existing_repo_files_that_are_adopted_without_overwrite() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("existing-readme-repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    fs::write(
        repo.join("README.md"),
        "# Existing README\n\nThis repo already had a README before guardrails init.\n",
    )
    .expect("readme");

    let output = run_guardrails_capture(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Created:"));
    assert!(stdout.contains("/AGENTS.md"));
    assert!(stdout.contains("/docs/project/implementation-tracker.md"));
    assert!(stdout.contains("Kept existing:"));
    assert!(stdout.contains("/README.md"));
}

#[test]
fn compiled_binary_does_not_embed_built_in_asset_source_paths() {
    let binary = fs::read(binary_path()).expect("compiled binary bytes");
    let binary_text = String::from_utf8_lossy(&binary);
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let forbidden_paths = [
        format!("{repo_root}/profiles/minimal/profile.toml"),
        format!("{repo_root}/profiles/docs-driven/profile.toml"),
        format!("{repo_root}/profiles/guardrails/profile.toml"),
        format!("{repo_root}/profiles/guardrails/templates/AGENTS.md"),
        format!("{repo_root}/profiles/guardrails/templates/docs/project/implementation-tracker.md"),
        format!("{repo_root}/profiles/guardrails/templates/docs/project/decision-log.md"),
        format!(
            "{repo_root}/profiles/guardrails/templates/docs/project/implementation-invariants.md"
        ),
        format!("{repo_root}/profiles/guardrails/templates/docs/best-practices/change-safety.md"),
        format!("{repo_root}/templates/shared/AGENTS.md"),
        format!("{repo_root}/templates/shared/docs/project/implementation-tracker.md"),
        format!("{repo_root}/templates/shared/docs/project/handoff-template.md"),
        format!("{repo_root}/templates/shared/docs/project/decision-log.md"),
        format!("{repo_root}/templates/shared/docs/project/implementation-invariants.md"),
        format!("{repo_root}/templates/github/.github/workflows/guardrails.yml"),
        format!("{repo_root}/templates/gitlab/.gitlab-ci.guardrails.yml"),
    ];

    for forbidden_path in forbidden_paths {
        assert!(
            !binary_text.contains(&forbidden_path),
            "compiled binary unexpectedly embedded built-in asset source path: {forbidden_path}"
        );
    }
}

#[test]
fn check_fails_when_forbidden_dir_is_present() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("forbidden-dir");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("forbidden-dir-profile");
    fs::create_dir_all(profile_dir.join("templates/docs/project")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"forbidden-dir\"\ndescription = \"Custom forbidden dir test\"\ndefault_ci = \"gitlab\"\ndocs_enabled = true\nrequired_docs = [\"docs/project/implementation-tracker.md\", \"docs/project/handoff-template.md\"]\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = [\"server\"]\nincludes_handoff = true\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "forbidden-dir",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "gitlab",
    ]);

    fs::create_dir_all(repo.join("server")).expect("forbidden dir");

    let output = run_guardrails_expect_failure(&["check", "--target", repo.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("forbidden directory present: server"));
}

#[test]
fn upgrade_plan_reports_current_and_target_state() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("upgrade-repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    let output = run_guardrails_capture(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--ci",
        "gitlab",
        "--plan",
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Guardrails upgrade plan"));
    assert!(stdout.contains("current.profile=minimal"));
    assert!(stdout.contains("target.profile=docs-driven"));
    assert!(stdout.contains("change.ci_provider=github -> gitlab"));
    assert!(stdout.contains("removable_stale_paths:"));
    assert!(stdout.contains(".github/workflows/guardrails.yml"));
}

#[test]
fn upgrade_apply_rewrites_repo_to_new_profile() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("apply-repo");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    run_guardrails(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--ci",
        "gitlab",
        "--apply",
    ]);

    let config = fs::read_to_string(repo.join(".guardrails/guardrails.toml")).expect("config");
    assert!(config.contains("profile = \"docs-driven\""));
    assert!(config.contains("provider = \"gitlab\""));
    assert!(repo.join("docs/project/decision-log.md").exists());
    assert!(repo.join(".gitlab-ci.guardrails.yml").exists());
    assert!(!repo.join(".github/workflows/guardrails.yml").exists());

    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    assert!(profile_lock.contains("path = \"docs/project/decision-log.md\""));
    assert!(!profile_lock.contains("path = \".github/workflows/guardrails.yml\""));
}

#[test]
fn init_force_refreshes_existing_unedited_managed_non_block_template() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("init-force-refresh");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = write_non_block_refresh_profile(temp.path(), "init-force-refresh");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "init-force-refresh",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    write_non_block_template(
        &profile_dir,
        "# Custom Playbook v2\n\n- refreshed by init --force\n",
    );

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "init-force-refresh",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
        "--force",
    ]);

    assert_eq!(
        fs::read_to_string(repo.join("docs/project/custom-playbook.md")).expect("playbook"),
        "# Custom Playbook v2\n\n- refreshed by init --force\n"
    );
}

#[test]
fn init_force_preserves_user_edited_managed_non_block_template() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("init-force-preserve-edit");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = write_non_block_refresh_profile(temp.path(), "init-force-preserve-edit");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "init-force-preserve-edit",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    fs::write(
        repo.join("docs/project/custom-playbook.md"),
        "# Repo Custom Playbook\n\n- user-owned edits should stay put\n",
    )
    .expect("edited playbook");
    write_non_block_template(
        &profile_dir,
        "# Custom Playbook v2\n\n- refreshed by init --force\n",
    );

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "init-force-preserve-edit",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
        "--force",
    ]);

    assert_eq!(
        fs::read_to_string(repo.join("docs/project/custom-playbook.md")).expect("playbook"),
        "# Repo Custom Playbook\n\n- user-owned edits should stay put\n"
    );
}

#[test]
fn upgrade_apply_refreshes_existing_unedited_managed_non_block_template() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("upgrade-apply-refresh");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = write_non_block_refresh_profile(temp.path(), "upgrade-apply-refresh");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "upgrade-apply-refresh",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    write_non_block_template(
        &profile_dir,
        "# Custom Playbook v2\n\n- refreshed by upgrade --apply\n",
    );

    run_guardrails(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "upgrade-apply-refresh",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
        "--apply",
    ]);

    assert_eq!(
        fs::read_to_string(repo.join("docs/project/custom-playbook.md")).expect("playbook"),
        "# Custom Playbook v2\n\n- refreshed by upgrade --apply\n"
    );
}

#[test]
fn upgrade_apply_preserves_user_edited_managed_non_block_template_across_repeated_reapply() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("upgrade-apply-preserve-edit");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = write_non_block_refresh_profile(temp.path(), "upgrade-apply-preserve-edit");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "upgrade-apply-preserve-edit",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
    ]);

    fs::write(
        repo.join("docs/project/custom-playbook.md"),
        "# Repo Custom Playbook\n\n- keep the repo edits through repeated reapply\n",
    )
    .expect("edited playbook");
    write_non_block_template(
        &profile_dir,
        "# Custom Playbook v2\n\n- refreshed by upgrade --apply when unedited\n",
    );

    run_guardrails(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "upgrade-apply-preserve-edit",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
        "--apply",
    ]);

    assert_eq!(
        fs::read_to_string(repo.join("docs/project/custom-playbook.md")).expect("playbook"),
        "# Repo Custom Playbook\n\n- keep the repo edits through repeated reapply\n"
    );

    write_non_block_template(
        &profile_dir,
        "# Custom Playbook v3\n\n- still should not overwrite repo edits\n",
    );

    run_guardrails(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "upgrade-apply-preserve-edit",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "none",
        "--apply",
    ]);

    assert_eq!(
        fs::read_to_string(repo.join("docs/project/custom-playbook.md")).expect("playbook"),
        "# Repo Custom Playbook\n\n- keep the repo edits through repeated reapply\n"
    );
}

#[test]
fn upgrade_apply_keeps_review_only_stale_files_for_manual_follow_up() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("review-stale-apply");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("review-stale-apply-profile");
    fs::create_dir_all(profile_dir.join("templates/docs/project")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"review-stale-apply\"\ndescription = \"Review stale docs apply test\"\ndefault_ci = \"github\"\ndocs_enabled = true\nrequired_docs = [\"docs/project/implementation-tracker.md\", \"docs/project/handoff-template.md\", \"docs/project/custom-playbook.md\"]\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = true\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("templates/docs/project/custom-playbook.md"),
        "# Custom Playbook\n",
    )
    .expect("playbook");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "review-stale-apply",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "github",
    ]);

    run_guardrails(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--apply",
    ]);

    assert!(repo.join("docs/project/custom-playbook.md").exists());
    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    assert!(!profile_lock.contains("path = \"docs/project/custom-playbook.md\""));
}

#[test]
fn upgrade_plan_surfaces_stale_managed_docs_for_review() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("review-stale-docs");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("review-stale-docs-profile");
    fs::create_dir_all(profile_dir.join("templates/docs/project")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"review-stale-docs\"\ndescription = \"Review stale docs test\"\ndefault_ci = \"github\"\ndocs_enabled = true\nrequired_docs = [\"docs/project/implementation-tracker.md\", \"docs/project/handoff-template.md\", \"docs/project/custom-playbook.md\"]\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = true\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("templates/docs/project/custom-playbook.md"),
        "# Custom Playbook\n",
    )
    .expect("playbook");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "review-stale-docs",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "github",
    ]);

    let output = run_guardrails_capture(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--plan",
        "--format",
        "json",
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid upgrade plan json");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["target"]["profile"], "minimal");
    assert!(
        json["review_stale_paths"]
            .as_array()
            .expect("review_stale_paths array")
            .iter()
            .any(|value| value == "docs/project/custom-playbook.md")
    );
    assert!(
        !json["removable_stale_paths"]
            .as_array()
            .expect("removable_stale_paths array")
            .iter()
            .any(|value| value == "docs/project/custom-playbook.md")
    );
}

#[test]
fn upgrade_plan_text_output_surfaces_review_stale_paths() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("review-stale-docs-text");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("review-stale-docs-text-profile");
    fs::create_dir_all(profile_dir.join("templates/docs/project")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"review-stale-docs-text\"\ndescription = \"Review stale docs text test\"\ndefault_ci = \"github\"\ndocs_enabled = true\nrequired_docs = [\"docs/project/implementation-tracker.md\", \"docs/project/handoff-template.md\", \"docs/project/custom-playbook.md\"]\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = true\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("templates/docs/project/custom-playbook.md"),
        "# Custom Playbook\n",
    )
    .expect("playbook");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "review-stale-docs-text",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "github",
    ]);

    let output = run_guardrails_capture(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--plan",
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stale_paths:"));
    assert!(stdout.contains("removable_stale_paths=none"));
    assert!(stdout.contains("review_stale_paths:"));
    assert!(stdout.contains("docs/project/custom-playbook.md"));
}

#[test]
fn doctor_json_output_reports_failures_structurally() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("doctor-json");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--ci",
        "github",
    ]);

    customize_installed_docs(&repo);
    fs::write(
        repo.join("docs/project/decision-log.md"),
        "# Decision Log\n\n## 2026-04-18\n\n- decision: keep the bootstrap utility small\n- rationale: preserve portability\n- consequences: profile templates carry project-specific policy\n",
    )
    .expect("decision log");

    fs::remove_file(repo.join("docs/project/decision-log.md")).expect("remove decision log");

    let output = run_guardrails_expect_failure(&[
        "doctor",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid doctor json");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["ok"], false);
    assert_eq!(json["profile"], "docs-driven");
    assert!(
        json["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|item| item["code"] == "required_doc_missing")
    );
    assert!(
        json["statuses"]
            .as_array()
            .expect("statuses array")
            .iter()
            .any(|item| item["relative_path"] == "docs/project/decision-log.md")
    );
}

#[test]
fn check_json_output_reports_success_structurally() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("check-json");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    customize_installed_docs(&repo);
    refresh_managed_blocks(&repo, "minimal", "github");
    fs::create_dir_all(repo.join(".git")).expect("git dir");

    let output = run_guardrails_capture(&[
        "check",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid check json");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["ok"], true);
    assert_eq!(
        json["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .len(),
        0
    );
}

#[test]
fn quick_start_minimal_flow_succeeds_after_customizing_installed_docs() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("quick-start-minimal");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    customize_installed_docs(&repo);
    refresh_managed_blocks(&repo, "minimal", "github");
    fs::create_dir_all(repo.join(".git")).expect("git dir");

    let doctor_output = run_guardrails_capture(&["doctor", "--target", repo.to_str().unwrap()]);
    let doctor_stdout = String::from_utf8_lossy(&doctor_output.stdout);
    assert!(doctor_stdout.contains("Guardrails doctor"));
    assert!(doctor_stdout.contains("Doctor checks passed."));

    let check_output = run_guardrails_capture(&["check", "--target", repo.to_str().unwrap()]);
    let check_stdout = String::from_utf8_lossy(&check_output.stdout);
    assert!(check_stdout.contains("All configured local checks passed."));

    let handoff_output = run_guardrails_capture(&["handoff", "--target", repo.to_str().unwrap()]);
    let handoff_stdout = String::from_utf8_lossy(&handoff_output.stdout);
    assert!(handoff_stdout.contains("# Handoff Template"));
    assert!(handoff_stdout.contains("customized starter docs for this repo"));
    assert!(handoff_stdout.contains("run project-guardrails doctor"));
}

#[test]
fn quick_start_gitlab_flow_succeeds_after_customizing_installed_docs() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("quick-start-gitlab");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "gitlab",
    ]);

    customize_installed_docs(&repo);
    refresh_managed_blocks(&repo, "minimal", "gitlab");
    fs::create_dir_all(repo.join(".git")).expect("git dir");

    let doctor_output = run_guardrails_capture(&["doctor", "--target", repo.to_str().unwrap()]);
    let doctor_stdout = String::from_utf8_lossy(&doctor_output.stdout);
    assert!(doctor_stdout.contains("Guardrails doctor"));
    assert!(doctor_stdout.contains("ci_workflow:.gitlab-ci.guardrails.yml=ok"));
    assert!(doctor_stdout.contains("Doctor checks passed."));

    let check_output = run_guardrails_capture(&["check", "--target", repo.to_str().unwrap()]);
    let check_stdout = String::from_utf8_lossy(&check_output.stdout);
    assert!(check_stdout.contains("All configured local checks passed."));

    let handoff_output = run_guardrails_capture(&["handoff", "--target", repo.to_str().unwrap()]);
    let handoff_stdout = String::from_utf8_lossy(&handoff_output.stdout);
    assert!(handoff_stdout.contains("# Handoff Template"));
    assert!(handoff_stdout.contains("customized starter docs for this repo"));

    let upgrade_output = run_guardrails_capture(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "gitlab",
        "--plan",
    ]);
    let upgrade_stdout = String::from_utf8_lossy(&upgrade_output.stdout);
    assert!(upgrade_stdout.contains("target.ci_provider=gitlab"));
    assert!(upgrade_stdout.contains("stale_paths=none"));
}

#[test]
fn status_json_output_reports_repo_state_structurally() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("status-json");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    let output = run_guardrails_capture(&[
        "status",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid status json");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["profile"], "minimal");
    assert_eq!(json["profile_source"], "built-in:minimal");
    assert_eq!(json["ci_provider"], "github");
    assert!(
        json["required_files"]
            .as_array()
            .expect("required_files array")
            .iter()
            .any(|value| value == "AGENTS.md")
    );
}

#[test]
fn upgrade_plan_json_output_reports_changes_structurally() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("upgrade-json");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    let output = run_guardrails_capture(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--ci",
        "gitlab",
        "--plan",
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid upgrade json");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["current"]["profile"], "minimal");
    assert_eq!(json["target"]["profile"], "docs-driven");
    assert!(
        json["changes"]
            .as_array()
            .expect("changes array")
            .iter()
            .any(|item| item["field"] == "ci_provider" && item["changed"] == true)
    );
    assert!(
        json["stale_paths"]
            .as_array()
            .expect("stale_paths array")
            .iter()
            .any(|value| value == ".github/workflows/guardrails.yml")
    );
    assert!(
        json["removable_stale_paths"]
            .as_array()
            .expect("removable_stale_paths array")
            .iter()
            .any(|value| value == ".github/workflows/guardrails.yml")
    );
    assert_eq!(
        json["review_stale_paths"]
            .as_array()
            .expect("review_stale_paths array")
            .len(),
        0
    );
}

#[test]
fn check_fails_when_unedited_starter_docs_are_present() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("starter-docs");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    let output = run_guardrails_expect_failure(&[
        "check",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid check json");
    let diagnostics = json["diagnostics"].as_array().expect("diagnostics array");

    assert!(diagnostics.iter().any(|item| {
        item["code"] == "required_file_starter_content"
            && item["message"]
                .as_str()
                .expect("message")
                .contains("AGENTS.md")
    }));
    assert!(diagnostics.iter().any(|item| {
        item["code"] == "required_doc_starter_content"
            && item["message"]
                .as_str()
                .expect("message")
                .contains("docs/project/implementation-tracker.md")
    }));
    assert!(diagnostics.iter().any(|item| {
        item["code"] == "required_doc_starter_content"
            && item["message"]
                .as_str()
                .expect("message")
                .contains("docs/project/handoff-template.md")
    }));
}

#[test]
fn pre_commit_fails_for_missing_linked_doc_and_forbidden_pattern() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("pre-commit-rules");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    init_git_repo(&repo);
    append_to_guardrails_config(
        &repo,
        "\n[[rules.link_requirements]]\nchanged_paths = [\"src/\"]\nrequired_docs = [\"docs/project/implementation-tracker.md\"]\nmessage = \"changes under src/ must update the implementation tracker in the same commit\"\n\n[[rules.forbidden_patterns]]\npattern = \"console\\\\.log\"\nmessage = \"remove debug logging before commit\"\n",
    );

    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(
        repo.join("src/lib.rs"),
        "pub fn demo() {\n    console.log(\"debug\");\n}\n",
    )
    .expect("write source");
    git(repo.as_path(), &["add", "src/lib.rs"]);

    let output = run_guardrails_expect_failure(&["pre-commit", "--target", repo.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[commit_link_requirement_missing]"));
    assert!(stderr.contains("[commit_forbidden_pattern]"));
}

#[test]
fn pre_commit_passes_when_required_doc_is_staged_with_clean_diff() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("pre-commit-pass");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    init_git_repo(&repo);
    append_to_guardrails_config(
        &repo,
        "\n[[rules.link_requirements]]\nchanged_paths = [\"src/\"]\nrequired_docs = [\"docs/project/implementation-tracker.md\"]\n",
    );

    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(
        repo.join("src/lib.rs"),
        "pub fn demo() -> &'static str {\n    \"ok\"\n}\n",
    )
    .expect("write source");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n## Current Approved Focus\n\n- document why src/ changed in this slice\n",
    )
    .expect("write tracker");

    git(
        repo.as_path(),
        &[
            "add",
            "src/lib.rs",
            "docs/project/implementation-tracker.md",
        ],
    );

    run_guardrails(&["pre-commit", "--target", repo.to_str().unwrap()]);
}

#[test]
fn commit_msg_check_requires_task_reference_for_staged_changes() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("commit-msg-missing-task");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    init_git_repo(&repo);
    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(repo.join("src/lib.rs"), "pub fn demo() {}\n").expect("write source");
    git(repo.as_path(), &["add", "src/lib.rs"]);

    let message_file = repo.join("COMMIT_EDITMSG");
    fs::write(&message_file, "feat: add demo command\n").expect("write message");

    let output = run_guardrails_expect_failure(&[
        "commit-msg-check",
        "--target",
        repo.to_str().unwrap(),
        message_file.to_str().unwrap(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[commit_task_reference_missing]"));
}

#[test]
fn commit_msg_check_accepts_reference_to_active_task() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("commit-msg-active-task");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "minimal",
        "--ci",
        "github",
    ]);

    init_git_repo(&repo);
    run_guardrails(&[
        "tasks",
        "new",
        "--target",
        repo.to_str().unwrap(),
        "--slug",
        "active-slice",
        "--owner",
        "codex",
    ]);

    fs::create_dir_all(repo.join("src")).expect("src dir");
    fs::write(repo.join("src/lib.rs"), "pub fn demo() {}\n").expect("write source");
    git(repo.as_path(), &["add", "src/lib.rs"]);

    let message_file = repo.join("COMMIT_EDITMSG");
    fs::write(&message_file, "[task:0001] feat: add demo command\n").expect("write message");

    run_guardrails(&[
        "commit-msg-check",
        "--target",
        repo.to_str().unwrap(),
        message_file.to_str().unwrap(),
    ]);
}

#[test]
fn check_fails_when_required_doc_is_missing() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("missing-doc");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--ci",
        "github",
    ]);

    customize_installed_docs(&repo);
    fs::write(
        repo.join("docs/project/decision-log.md"),
        "# Decision Log\n\n## 2026-04-18\n\n- decision: require a decision log\n- rationale: keep durable context reviewable\n- consequences: contributors update this file for major choices\n",
    )
    .expect("decision log");

    fs::remove_file(repo.join("docs/project/decision-log.md")).expect("remove decision log");

    let output = run_guardrails_expect_failure(&["doctor", "--target", repo.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required doc: docs/project/decision-log.md"));
}

#[test]
fn doctor_fails_when_semgrep_binary_is_missing() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("missing-semgrep");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    let profile_dir = temp.path().join("missing-semgrep-profile");
    fs::create_dir_all(profile_dir.join("assets/rules/semgrep")).expect("profile assets");
    fs::write(
        profile_dir.join("profile.toml"),
        "schema_version = 1\nname = \"missing-semgrep\"\ndescription = \"Missing semgrep binary test\"\ndefault_ci = \"github\"\ndocs_enabled = false\nrequired_docs = []\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = false\n\n[semgrep]\nenabled = true\nbinary = \"definitely-not-a-real-semgrep-binary\"\nconfig_paths = [\"rules/semgrep/custom.yml\"]\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n",
    )
    .expect("profile");
    fs::write(
        profile_dir.join("assets/rules/semgrep/custom.yml"),
        "rules: []\n",
    )
    .expect("rule");

    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "missing-semgrep",
        "--profile-path",
        profile_dir.to_str().unwrap(),
        "--ci",
        "github",
    ]);

    let output = run_guardrails_expect_failure(&["doctor", "--target", repo.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to execute definitely-not-a-real-semgrep-binary"));
}

fn run_guardrails(args: &[&str]) {
    let status = Command::new(binary_path())
        .args(args)
        .status()
        .expect("guardrails command should run");
    assert!(status.success(), "guardrails command failed: {args:?}");
}

fn run_guardrails_capture(args: &[&str]) -> Output {
    let output = Command::new(binary_path())
        .args(args)
        .output()
        .expect("guardrails command should run");
    assert!(
        output.status.success(),
        "guardrails command failed: {args:?}"
    );
    output
}

fn run_guardrails_expect_failure(args: &[&str]) -> Output {
    let output = Command::new(binary_path())
        .args(args)
        .output()
        .expect("guardrails command should run");
    assert!(
        !output.status.success(),
        "guardrails command unexpectedly succeeded: {args:?}"
    );
    output
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn init_git_repo(repo: &Path) {
    git(repo, &["init"]);
    git(repo, &["config", "user.name", "Test User"]);
    git(repo, &["config", "user.email", "test@example.com"]);
}

fn append_to_guardrails_config(repo: &Path, suffix: &str) {
    let path = repo.join(".guardrails/guardrails.toml");
    let mut config = fs::read_to_string(&path).expect("read config");
    if suffix.contains("[[rules.link_requirements]]") {
        config = config.replace("link_requirements = []\n", "");
    }
    if suffix.contains("[[rules.forbidden_patterns]]") {
        config = config.replace("forbidden_patterns = []\n", "");
    }
    config.push_str(suffix);
    fs::write(path, config).expect("write config");
}

fn write_non_block_refresh_profile(root: &Path, profile_name: &str) -> PathBuf {
    let profile_dir = root.join(format!("{profile_name}-profile"));
    fs::create_dir_all(profile_dir.join("templates/docs/project")).expect("profile templates");
    fs::write(
        profile_dir.join("profile.toml"),
        format!(
            "schema_version = 1\nname = \"{profile_name}\"\ndescription = \"Managed non-block refresh test\"\ndefault_ci = \"none\"\ndocs_enabled = true\nrequired_docs = [\"docs/project/custom-playbook.md\"]\nrequired_files = [\"README.md\", \"AGENTS.md\", \".guardrails/guardrails.toml\"]\nforbidden_dirs = []\nincludes_handoff = false\n\n[semgrep]\nenabled = false\nbinary = \"semgrep\"\nconfig_paths = []\nextra_args = []\n\n[conftest]\nenabled = false\nbinary = \"conftest\"\npolicy_paths = []\nextra_args = []\n"
        ),
    )
    .expect("profile");
    write_non_block_template(&profile_dir, "# Custom Playbook v1\n\n- initial baseline\n");
    profile_dir
}

fn write_non_block_template(profile_dir: &Path, contents: &str) {
    fs::write(
        profile_dir.join("templates/docs/project/custom-playbook.md"),
        contents,
    )
    .expect("playbook template");
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

#[cfg(unix)]
fn write_script(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, contents).expect("write script");
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod script");
}

fn customize_installed_docs(repo: &Path) {
    fs::write(
        repo.join("AGENTS.md"),
        "# AGENTS.md\n\n## Repo Purpose\n\nThis repository packages a portable bootstrap utility for repo-local guardrails.\n\n## Guardrails\n\n- keep bootstrap behavior small and portable\n- keep project-specific policy in copied profile assets\n- read docs/project/implementation-tracker.md before widening scope\n\n## Workflow\n\n- update the tracker when the approved slice changes\n- leave a handoff-quality summary after substantial work\n- keep docs and code aligned\n",
    )
    .expect("custom agents");
    fs::write(
        repo.join("docs/project/implementation-tracker.md"),
        "# Implementation Tracker\n\n## Current Approved Focus\n\n- keep placeholder detection conservative and reviewable\n\n## Current Approved Next Steps\n\n1. validate starter-doc diagnostics in doctor and check\n\n## Current Explicit Non-Goals\n\n- do not build semantic content scoring\n\n## Phase Status\n\n- Phase 0: validation in progress\n\n## Recently Validated\n\n- required docs exist and contain repo-owned content\n\n## Open Questions\n\n- none for this focused slice\n",
    )
    .expect("custom tracker");

    let handoff_path = repo.join("docs/project/handoff-template.md");
    if handoff_path.exists() {
        fs::write(
            handoff_path,
            "# Handoff Template\n\n## Current Result\n\n- customized starter docs for this repo\n- placeholder checks now have repo-specific content to read\n- broader semantic enforcement remains intentionally unimplemented\n\n## Docs Updated\n\n- AGENTS.md\n- docs/project/implementation-tracker.md\n- docs/project/handoff-template.md\n\n## Verification\n\n- run project-guardrails doctor\n- run project-guardrails check\n\n## Next Valid Steps\n\n1. extend coverage only when a machine-checkable signal is obvious\n\n## Remaining Non-Goals\n\n- do not widen this into a semantic rules engine\n",
        )
        .expect("custom handoff");
    }
}

fn refresh_managed_blocks(repo: &Path, profile: &str, ci: &str) {
    run_guardrails(&[
        "init",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        profile,
        "--ci",
        ci,
        "--force",
    ]);
}
