use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use filetime::{FileTime, set_file_mtime};
use serde_json::Value;
use tempfile::TempDir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_project-guardrails"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

#[test]
fn tasks_cli_and_handoff_flow_manage_repo_local_state() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("task-flow");
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
        "tasks",
        "new",
        "--target",
        repo.to_str().unwrap(),
        "--slug",
        "ingest-pipeline",
        "--priority",
        "p1",
        "--owner",
        "codex",
    ]);

    let task_path = repo.join(".guardrails/state/tasks/0001-ingest-pipeline.md");
    let task_contents = fs::read_to_string(&task_path).expect("task contents");
    assert!(task_contents.contains("status = \"in_progress\""));
    assert!(task_contents.contains("owner = \"codex\""));

    let list_output =
        run_guardrails_capture(&["tasks", "list", "--target", repo.to_str().unwrap()]);
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_stdout.contains("0001 in_progress p1 codex ingest-pipeline"));

    let get_output =
        run_guardrails_capture(&["tasks", "get", "1", "--target", repo.to_str().unwrap()]);
    let get_stdout = String::from_utf8_lossy(&get_output.stdout);
    assert!(get_stdout.contains("title = \"Ingest Pipeline\""));
    assert!(get_stdout.contains("# Ingest Pipeline"));

    run_guardrails(&[
        "tasks",
        "update",
        "1",
        "--target",
        repo.to_str().unwrap(),
        "--status",
        "blocked",
    ]);
    run_guardrails(&[
        "tasks",
        "claim",
        "1",
        "--target",
        repo.to_str().unwrap(),
        "--owner",
        "jordan",
    ]);
    run_guardrails(&[
        "tasks",
        "close",
        "1",
        "--target",
        repo.to_str().unwrap(),
        "--commit",
        "abc123def",
    ]);

    let closed_task = fs::read_to_string(&task_path).expect("closed task");
    assert!(closed_task.contains("status = \"done\""));
    assert!(closed_task.contains("commits = [\"abc123def\"]"));

    run_guardrails(&[
        "handoff",
        "new",
        "--target",
        repo.to_str().unwrap(),
        "--slug",
        "slice-1",
        "--task",
        "1",
    ]);

    let handoff_path = repo.join(".guardrails/state/handoffs/0001-slice-1.md");
    let handoff_contents = fs::read_to_string(&handoff_path).expect("handoff contents");
    assert!(handoff_contents.contains("status = \"open\""));
    assert!(handoff_contents.contains("task_ids = [1]"));
    assert!(handoff_contents.contains("# Handoff Template"));

    let handoff_list =
        run_guardrails_capture(&["handoff", "list", "--target", repo.to_str().unwrap()]);
    let handoff_list_stdout = String::from_utf8_lossy(&handoff_list.stdout);
    assert!(handoff_list_stdout.contains("0001 open 1 slice-1"));

    run_guardrails(&["handoff", "close", "1", "--target", repo.to_str().unwrap()]);

    let closed_handoff = fs::read_to_string(&handoff_path).expect("closed handoff");
    assert!(closed_handoff.contains("status = \"closed\""));
}

#[test]
fn task_lint_flags_duplicate_ids_and_check_surfaces_them() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("task-lint");
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

    write_task_file(
        &repo.join(".guardrails/state/tasks/0001-first.md"),
        1,
        "first",
        "proposed",
    );
    write_task_file(
        &repo.join(".guardrails/state/tasks/0001-second.md"),
        1,
        "second",
        "proposed",
    );

    let lint_output =
        run_guardrails_capture(&["tasks", "lint", "--target", repo.to_str().unwrap()]);
    assert!(!lint_output.status.success());
    let lint_stderr = String::from_utf8_lossy(&lint_output.stderr);
    assert!(lint_stderr.contains("[task_duplicate_id]"));

    let check_output = run_guardrails_capture(&["check", "--target", repo.to_str().unwrap()]);
    assert!(!check_output.status.success());
    let check_stderr = String::from_utf8_lossy(&check_output.stderr);
    assert!(check_stderr.contains("[task_duplicate_id]"));
}

#[test]
fn upgrade_apply_preserves_state_and_records_preserve_ownership() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("upgrade-state");
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

    write_task_file(
        &repo.join(".guardrails/state/tasks/0001-preserved.md"),
        1,
        "preserved",
        "approved",
    );
    fs::write(
        repo.join(".guardrails/state/handoffs/0001-preserved.md"),
        "+++\nid = 1\nslug = \"preserved\"\ntitle = \"Preserved\"\nstatus = \"open\"\ncreated = \"2026-04-22T00:00:00Z\"\nupdated = \"2026-04-22T00:00:00Z\"\ntask_ids = [1]\ntemplate_path = \"docs/project/handoff-template.md\"\n+++\n\n# Preserved\n",
    )
    .expect("handoff");

    run_guardrails(&[
        "upgrade",
        "--target",
        repo.to_str().unwrap(),
        "--profile",
        "docs-driven",
        "--apply",
    ]);

    assert!(
        repo.join(".guardrails/state/tasks/0001-preserved.md")
            .exists()
    );
    assert!(
        repo.join(".guardrails/state/handoffs/0001-preserved.md")
            .exists()
    );

    let profile_lock =
        fs::read_to_string(repo.join(".guardrails/profile.lock")).expect("profile lock");
    assert!(profile_lock.contains("path = \".guardrails/state\""));
    assert!(profile_lock.contains("stale_action = \"preserve\""));
}

#[test]
fn check_surfaces_managed_block_task_tracker_and_handoff_freshness_drift() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("freshness-drift");
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

    run_guardrails(&[
        "tasks",
        "new",
        "--target",
        repo.to_str().unwrap(),
        "--slug",
        "freshness-slice",
        "--owner",
        "codex",
    ]);

    fs::write(
        repo.join(".guardrails/state/handoffs/0001-stale.md"),
        "+++\nid = 1\nslug = \"stale\"\ntitle = \"Stale\"\nstatus = \"open\"\ncreated = \"2026-01-01T00:00:00Z\"\nupdated = \"2026-01-01T00:00:00Z\"\ntask_ids = [1]\ntemplate_path = \"docs/project/handoff-template.md\"\n+++\n\n# Stale\n",
    )
    .expect("handoff");

    let output = run_guardrails_capture(&["check", "--target", repo.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[managed_block_stale]"));
    assert!(stderr.contains("[task_tracker_sync_missing]"));
    assert!(stderr.contains("[handoff_stale]"));
}

#[test]
fn doctor_reports_stale_required_doc_age() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("stale-doc-age");
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
        "# Decision Log\n\n## 2026-04-18\n\n- decision: keep guardrails docs reviewable\n- rationale: improve portability\n- consequences: refresh project docs when behavior changes\n",
    )
    .expect("decision log");
    refresh_managed_blocks(&repo, "docs-driven", "github");
    fs::create_dir_all(repo.join(".git")).expect("git dir");

    let stale_time = FileTime::from_unix_time(1_735_689_600, 0);
    set_file_mtime(repo.join("docs/project/decision-log.md"), stale_time).expect("mtime");

    let output = run_guardrails_capture(&["doctor", "--target", repo.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[required_doc_stale_age]"));
    assert!(stderr.contains("docs/project/decision-log.md"));
}

#[test]
fn status_for_llm_json_reports_empty_repo_state() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("status-for-llm-empty");
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

    let output =
        run_guardrails_capture(&["status", "--target", repo.to_str().unwrap(), "--for-llm"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("valid llm status json");

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["summary"]["profile"]["name"], "minimal");
    assert_eq!(json["summary"]["repo"]["config_version"], 1);
    assert_eq!(json["summary"]["repo"]["ci_provider"], "github");
    assert_eq!(
        json["summary"]["active_tasks"]
            .as_array()
            .expect("active_tasks array")
            .len(),
        0
    );
    assert_eq!(
        json["summary"]["recent_handoffs"]
            .as_array()
            .expect("recent_handoffs array")
            .len(),
        0
    );
    assert!(json["summary"]["latest_handoff"].is_null());
    assert!(
        json["summary"]["required_reading"]
            .as_array()
            .expect("required_reading array")
            .iter()
            .any(|item| {
                item["topic"] == "repo_intent"
                    && item["path"] == "AGENTS.md"
                    && item["exists"] == true
            })
    );
}

#[test]
fn pre_work_and_handoff_json_outputs_capture_populated_repo_state() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("pre-work-json");
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

    run_guardrails(&[
        "tasks",
        "new",
        "--target",
        repo.to_str().unwrap(),
        "--slug",
        "repo-summary",
        "--owner",
        "codex",
    ]);

    let new_handoff = run_guardrails_capture(&[
        "handoff",
        "new",
        "--target",
        repo.to_str().unwrap(),
        "--slug",
        "summary-slice",
        "--task",
        "1",
        "--format",
        "json",
    ]);
    let new_handoff_stdout = String::from_utf8_lossy(&new_handoff.stdout);
    let new_handoff_json: Value =
        serde_json::from_str(&new_handoff_stdout).expect("valid new handoff json");
    assert_eq!(new_handoff_json["handoff"]["id"], "0001");
    assert_eq!(new_handoff_json["handoff"]["status"], "open");
    assert_eq!(
        new_handoff_json["handoff"]["template_path"],
        "docs/project/handoff-template.md"
    );
    assert!(
        new_handoff_json["handoff"]["body"]
            .as_str()
            .expect("handoff body")
            .contains("# Handoff Template")
    );

    let pre_work = run_guardrails_capture(&[
        "pre-work",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let pre_work_stdout = String::from_utf8_lossy(&pre_work.stdout);
    let pre_work_json: Value = serde_json::from_str(&pre_work_stdout).expect("valid pre-work json");

    assert_eq!(pre_work_json["schema_version"], 1);
    assert!(
        pre_work_json["run_id"]
            .as_str()
            .expect("run_id")
            .starts_with("20")
    );
    let run_path = pre_work_json["run_path"].as_str().expect("run path");
    assert!(repo.join(run_path).exists());
    let stored_run = fs::read_to_string(repo.join(run_path)).expect("stored pre-work run");
    let stored_json: Value = serde_json::from_str(&stored_run).expect("stored run json");
    assert_eq!(stored_json, pre_work_json);
    assert_eq!(
        pre_work_json["summary"]["active_tasks"][0]["slug"],
        "repo-summary"
    );
    assert_eq!(
        pre_work_json["summary"]["latest_handoff"]["slug"],
        "summary-slice"
    );
    assert!(
        pre_work_json["summary"]["required_reading"]
            .as_array()
            .expect("required_reading array")
            .iter()
            .any(|item| item["topic"] == "active_task"
                && item["path"] == ".guardrails/state/tasks/0001-repo-summary.md")
    );

    let closed_handoff = run_guardrails_capture(&[
        "handoff",
        "close",
        "1",
        "--target",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let closed_handoff_stdout = String::from_utf8_lossy(&closed_handoff.stdout);
    let closed_handoff_json: Value =
        serde_json::from_str(&closed_handoff_stdout).expect("valid closed handoff json");
    assert_eq!(closed_handoff_json["handoff"]["status"], "closed");
    assert_eq!(closed_handoff_json["handoff"]["task_ids"][0], "0001");
}

fn write_task_file(path: &Path, id: u32, slug: &str, status: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("task dir");
    }
    fs::write(
        path,
        format!(
            "+++\nid = {id}\nslug = \"{slug}\"\ntitle = \"{}\"\nstatus = \"{status}\"\ncreated = \"2026-04-22T00:00:00Z\"\nupdated = \"2026-04-22T00:00:00Z\"\n\n[refs]\ntracker = [\"docs/project/implementation-tracker.md\"]\n+++\n\n# {}\n",
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
    fs::write(
        repo.join("docs/project/handoff-template.md"),
        "# Handoff Template\n\n## Current Result\n\n- customized starter docs for this repo\n- placeholder checks now have repo-specific content to read\n- broader semantic enforcement remains intentionally unimplemented\n\n## Docs Updated\n\n- AGENTS.md\n- docs/project/implementation-tracker.md\n- docs/project/handoff-template.md\n\n## Verification\n\n- run project-guardrails doctor\n- run project-guardrails check\n\n## Next Valid Steps\n\n1. extend coverage only when a machine-checkable signal is obvious\n\n## Remaining Non-Goals\n\n- do not widen this into a semantic rules engine\n",
    )
    .expect("custom handoff");
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

fn run_guardrails(args: &[&str]) -> Output {
    let output = run_guardrails_capture(args);
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn run_guardrails_capture(args: &[&str]) -> Output {
    Command::new(binary_path())
        .args(args)
        .output()
        .expect("run guardrails")
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
