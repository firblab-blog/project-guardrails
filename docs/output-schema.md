# Output Schema

This document defines the initial machine-readable output contract for
`project-guardrails`.

Supported commands:

- `project-guardrails profiles list --format json`
- `project-guardrails pre-work --format json`
- `project-guardrails status --format json`
- `project-guardrails status --for-llm`
- `project-guardrails doctor --format json`
- `project-guardrails check --format json`
- `project-guardrails pre-commit --format json`
- `project-guardrails commit-msg-check <message-file> --format json`
- `project-guardrails upgrade --plan --format json`
- `project-guardrails tasks list --format json`
- `project-guardrails tasks get <id> --format json`
- `project-guardrails tasks new ... --format json`
- `project-guardrails tasks claim <id> ... --format json`
- `project-guardrails tasks update <id> ... --format json`
- `project-guardrails tasks close <id> ... --format json`
- `project-guardrails tasks lint --format json`
- `project-guardrails handoff list --format json`
- `project-guardrails handoff new ... --format json`
- `project-guardrails handoff close <id> --format json`

The default remains `--format text`.

This document does not treat bare `project-guardrails handoff` compatibility
output or `project-guardrails upgrade --apply --format json` as supported JSON
contracts.

## Stability Intent

These JSON shapes are intended to be stable within the current major version of
the bootstrap utility.

Fields may grow over time, but existing fields should not be removed or changed
casually once other tools begin consuming them.

Commands that can fail validation still emit their JSON payload on stdout before
returning a non-zero exit status.

## Profiles List

`project-guardrails profiles list --format json` reports the built-in profile
catalog.

Example shape:

```json
{
  "schema_version": 1,
  "profiles": [
    {
      "name": "minimal",
      "summary": "Neutral cross-language baseline with local config, AGENTS, tracker, handoff, and optional CI wiring.",
      "description": "Smallest public cross-language guardrails profile with local config, AGENTS, tracker, handoff, and optional CI wiring.",
      "is_default": true,
      "is_opt_in": false
    },
    {
      "name": "guardrails",
      "summary": "Opt-in FirbLab-style doctrine profile with seeded AGENTS, tracker, decision log, handoff, and curated best-practice docs.",
      "description": "Opt-in FirbLab-style doctrine profile with seeded AGENTS guidance, project docs, and curated best-practice references.",
      "is_default": false,
      "is_opt_in": true
    }
  ]
}
```

## Status

Example shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "profile": "minimal",
  "profile_source": "built-in:minimal",
  "profile_schema_version": 1,
  "installed_by_version": "0.1.16",
  "docs_enabled": true,
  "ci_provider": "github",
  "required_files": [
    "README.md",
    "AGENTS.md",
    ".guardrails/guardrails.toml",
    ".pre-commit-config.yaml"
  ],
  "forbidden_dirs": [],
  "semgrep_enabled": false,
  "conftest_enabled": false
}
```

## Status For LLM

`project-guardrails status --for-llm` emits the richer machine-readable
repo summary intended for agents.

Example shape:

```json
{
  "schema_version": 1,
  "summary": {
    "repo_root": "/path/to/repo",
    "generated_at": "2026-04-22T16:50:00Z",
    "profile": {
      "name": "minimal",
      "source": "built-in:minimal",
      "schema_version": 1,
      "installed_by_version": "0.1.16"
    },
    "repo": {
      "config_version": 1,
      "docs_enabled": true,
      "ci_provider": "github",
      "required_docs": [
        "docs/project/implementation-tracker.md",
        "docs/project/handoff-template.md"
      ],
      "required_files": [
        "README.md",
        "AGENTS.md",
        ".guardrails/guardrails.toml",
        ".pre-commit-config.yaml"
      ],
      "forbidden_dirs": [],
      "semgrep_enabled": false,
      "conftest_enabled": false,
      "task_references_required": true
    },
    "required_reading": [
      {
        "topic": "repo_intent",
        "path": "AGENTS.md",
        "exists": true
      },
      {
        "topic": "approved_focus",
        "path": "docs/project/implementation-tracker.md",
        "exists": true
      },
      {
        "topic": "handoff_template",
        "path": "docs/project/handoff-template.md",
        "exists": true
      },
      {
        "topic": "non_goals",
        "path": "AGENTS.md",
        "exists": true
      }
    ],
    "active_tasks": [],
    "recent_handoffs": [],
    "latest_handoff": null,
    "doctor": {
      "ok": false,
      "diagnostics": [
        {
          "code": "required_file_starter_content",
          "message": "AGENTS.md still contains stock starter content; replace the placeholder guidance with repo-specific content"
        }
      ]
    }
  }
}
```

`required_reading` is a machine-readable reading list, not proof that an
agent understood the repo.

## Pre-Work

`project-guardrails pre-work --format json` emits the same repo summary plus a
durable run record and stores that JSON under `.guardrails/state/runs/`.

Example shape:

```json
{
  "schema_version": 1,
  "run_id": "20260422T165500Z-1a2b3c4d",
  "run_path": ".guardrails/state/runs/pre-work-20260422T165500Z-1a2b3c4d.json",
  "summary": {
    "repo_root": "/path/to/repo",
    "generated_at": "2026-04-22T16:55:00Z",
    "profile": {
      "name": "minimal",
      "source": "built-in:minimal",
      "schema_version": 1,
      "installed_by_version": "0.1.16"
    },
    "repo": {
      "config_version": 1,
      "docs_enabled": true,
      "ci_provider": "github",
      "required_docs": [
        "docs/project/implementation-tracker.md",
        "docs/project/handoff-template.md"
      ],
      "required_files": [
        "README.md",
        "AGENTS.md",
        ".guardrails/guardrails.toml",
        ".pre-commit-config.yaml"
      ],
      "forbidden_dirs": [],
      "semgrep_enabled": false,
      "conftest_enabled": false,
      "task_references_required": true
    },
    "required_reading": [
      {
        "topic": "repo_intent",
        "path": "AGENTS.md",
        "exists": true
      },
      {
        "topic": "approved_focus",
        "path": "docs/project/implementation-tracker.md",
        "exists": true
      },
      {
        "topic": "handoff_template",
        "path": "docs/project/handoff-template.md",
        "exists": true
      },
      {
        "topic": "non_goals",
        "path": "AGENTS.md",
        "exists": true
      },
      {
        "topic": "active_task",
        "path": ".guardrails/state/tasks/0001-ingest-pipeline.md",
        "exists": true
      },
      {
        "topic": "recent_handoff",
        "path": ".guardrails/state/handoffs/0001-slice-1.md",
        "exists": true
      }
    ],
    "active_tasks": [
      {
        "id": "0001",
        "slug": "ingest-pipeline",
        "title": "Ingest Pipeline",
        "status": "in_progress",
        "owner": "codex",
        "priority": "p1",
        "updated": "2026-04-22T16:20:00Z",
        "path": ".guardrails/state/tasks/0001-ingest-pipeline.md"
      }
    ],
    "recent_handoffs": [
      {
        "id": "0001",
        "slug": "slice-1",
        "title": "Slice 1",
        "status": "open",
        "created": "2026-04-22T16:45:00Z",
        "updated": "2026-04-22T16:45:00Z",
        "task_ids": ["0001"],
        "template_path": "docs/project/handoff-template.md",
        "path": ".guardrails/state/handoffs/0001-slice-1.md"
      }
    ],
    "latest_handoff": {
      "id": "0001",
      "slug": "slice-1",
      "title": "Slice 1",
      "status": "open",
      "created": "2026-04-22T16:45:00Z",
      "updated": "2026-04-22T16:45:00Z",
      "task_ids": ["0001"],
      "template_path": "docs/project/handoff-template.md",
      "path": ".guardrails/state/handoffs/0001-slice-1.md"
    },
    "doctor": {
      "ok": true,
      "diagnostics": []
    }
  }
}
```

The file written to `run_path` is the same JSON payload emitted on stdout.

## Doctor

Example shape:

```json
{
  "schema_version": 1,
  "ok": false,
  "repo_root": "/path/to/repo",
  "profile": "docs-driven",
  "profile_source": "built-in:docs-driven",
  "installed_by_version": "0.1.16",
  "semgrep_engine": "disabled",
  "conftest_engine": "disabled",
  "statuses": [
    {
      "label": "required_doc",
      "relative_path": "docs/project/decision-log.md",
      "status": "missing"
    }
  ],
  "diagnostics": [
    {
      "code": "required_doc_missing",
      "message": "missing required doc: docs/project/decision-log.md"
    }
  ]
}
```

## Check

Example shape:

```json
{
  "schema_version": 1,
  "ok": true,
  "repo_root": "/path/to/repo",
  "diagnostics": []
}
```

When `ok` is `false`, `diagnostics` contains the structured failures.

## Pre-Commit

`project-guardrails pre-commit --format json` reports the staged paths it
evaluated and any enforcement diagnostics.

Example shape:

```json
{
  "schema_version": 1,
  "ok": false,
  "repo_root": "/path/to/repo",
  "staged_paths": [
    "src/lib.rs",
    "docs/project/implementation-tracker.md"
  ],
  "diagnostics": [
    {
      "code": "commit_link_requirement_missing",
      "message": "staged changes under [src/**] require one of [docs/project/implementation-tracker.md] in the same commit"
    }
  ]
}
```

When `ok` is `true`, `diagnostics` is empty.

## Commit Message Check

`project-guardrails commit-msg-check <message-file> --format json` reports the
normalized task ids extracted from the commit message and any enforcement
diagnostics.

Example shape:

```json
{
  "schema_version": 1,
  "ok": true,
  "repo_root": "/path/to/repo",
  "task_ids": ["0001"],
  "diagnostics": []
}
```

When `ok` is `false`, `diagnostics` contains the structured failures.

## Upgrade Plan

Example shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "current": {
    "profile": "minimal",
    "profile_source": "built-in:minimal",
    "profile_schema_version": 1,
    "installed_by_version": "0.1.16",
    "ci_provider": "github"
  },
  "target": {
    "profile": "docs-driven",
    "profile_source": "built-in:docs-driven",
    "profile_schema_version": 1,
    "installed_by_version": "0.1.16",
    "ci_provider": "gitlab"
  },
  "changes": [
    {
      "field": "ci_provider",
      "current": "github",
      "target": "gitlab",
      "changed": true
    }
  ],
  "stale_paths": [
    ".github/workflows/guardrails.yml"
  ],
  "removable_stale_paths": [
    ".github/workflows/guardrails.yml"
  ],
  "preserved_stale_paths": [],
  "review_stale_paths": [],
  "planned_actions": [
    "reapply the selected profile against the target repo"
  ]
}
```

`preserved_stale_paths` is reserved for managed paths whose lockfile
`stale_action = "preserve"`.

## Tasks

`tasks list --format json` returns task summaries:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "tasks": [
    {
      "id": "0001",
      "slug": "ingest-pipeline",
      "title": "Ingest Pipeline",
      "status": "in_progress",
      "owner": "codex",
      "priority": "p1",
      "updated": "2026-04-22T16:20:00Z",
      "path": ".guardrails/state/tasks/0001-ingest-pipeline.md"
    }
  ]
}
```

Summary entries currently include:

- `id`
- `slug`
- `title`
- `status`
- `owner`
- `priority`
- `updated`
- `path`

Task status values emitted today are:

- `proposed`
- `approved`
- `in_progress`
- `blocked`
- `done`
- `dropped`

`tasks get <id> --format json` returns the full task record.
The JSON contract for `tasks new`, `tasks claim`, `tasks update`, and
`tasks close` is the same full-task shape for the resulting task.

Example full-task shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "task": {
    "path": ".guardrails/state/tasks/0001-ingest-pipeline.md",
    "id": "0001",
    "slug": "ingest-pipeline",
    "title": "Ingest Pipeline",
    "status": "in_progress",
    "owner": "codex",
    "priority": "p1",
    "created": "2026-04-22T14:10:00Z",
    "updated": "2026-04-22T16:20:00Z",
    "refs": {
      "tracker": ["docs/project/implementation-tracker.md"]
    },
    "commits": [],
    "body": "# Ingest Pipeline\n..."
  }
}
```

Within `task.refs`, the stable buckets are `tracker`, `code`, and `docs`.
Empty buckets may be omitted.

`tasks lint --format json` reports whether the repo-local task state is valid.

Example shape:

```json
{
  "schema_version": 1,
  "ok": false,
  "repo_root": "/path/to/repo",
  "diagnostics": [
    {
      "code": "task_duplicate_id",
      "message": "task 0001 is defined more than once: .guardrails/state/tasks/0001-first.md, .guardrails/state/tasks/0001-second.md"
    }
  ]
}
```

## Handoffs

Example `handoff list --format json` shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "handoffs": [
    {
      "id": "0001",
      "slug": "slice-1",
      "title": "Slice 1",
      "status": "open",
      "created": "2026-04-22T16:45:00Z",
      "task_ids": ["0001"],
      "updated": "2026-04-22T16:45:00Z",
      "template_path": "docs/project/handoff-template.md",
      "path": ".guardrails/state/handoffs/0001-slice-1.md"
    }
  ]
}
```

Example `handoff new --format json` and `handoff close --format json` shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "handoff": {
    "id": "0001",
    "slug": "slice-1",
    "title": "Slice 1",
    "status": "open",
    "created": "2026-04-22T16:45:00Z",
    "updated": "2026-04-22T16:45:00Z",
    "task_ids": ["0001"],
    "template_path": "docs/project/handoff-template.md",
    "path": ".guardrails/state/handoffs/0001-slice-1.md",
    "body": "# Handoff Template\n..."
  }
}
```

## Diagnostics

Every machine-readable response now includes:

- `schema_version`
  - top-level JSON contract version for downstream consumers

Consumers should validate `schema_version` before depending on any field shape.

Shared diagnostics contain:

- `code`
  - stable identifier for the issue class
- `message`
  - human-readable explanation

The `message` is for humans.
The `code` is the better field to automate against.
