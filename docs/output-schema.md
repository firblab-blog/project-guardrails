# Output Schema

This document defines the initial machine-readable output contract for
`project-guardrails`.

Supported commands:

- `project-guardrails profiles list --format json`
- `project-guardrails pre-work --format json`
- `project-guardrails brief --format json`
- `project-guardrails resume --format json`
- `project-guardrails timeline --format json`
- `project-guardrails adapters list --format json`
- `project-guardrails status --format json`
- `project-guardrails status --for-llm`
- `project-guardrails doctor --format json`
- `project-guardrails check --format json`
- `project-guardrails refresh --format json`
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

`project-guardrails mcp serve` is documented separately in
[`docs/mcp.md`](mcp.md). MCP tool responses use `structuredContent` plus a JSON
text fallback, but the server command itself is a stdio protocol endpoint
rather than a `--format json` CLI output surface.

## Stability Intent

These JSON shapes are intended to be stable within the current major version of
the bootstrap utility.

`schema_version` is the machine contract version for these payloads. It is not
the package version and should change only when consumers need a clear signal
that the contract has intentionally shifted.

Fields may be added over time. Existing fields should not be removed casually
once other tools begin consuming them. Changing the type or meaning of an
existing field requires schema review and corresponding test updates.

Diagnostics use `code` as the automation-facing identifier. The `message` field
is human-facing text and may change for clarity without replacing `code` as the
stable matching surface.

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
  "installed_by_version": "0.2.0",
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
      "installed_by_version": "0.2.0"
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

## Brief

`project-guardrails brief --target . --format json` emits a read-only
paste-friendly new-session starter. It reuses the same `LlmRepoSummary` shape
as `status --for-llm` and adds compact counts plus recommended next commands.

`brief` does not write `.guardrails/state/runs/` records and does not run
`refresh`.

Example shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "generated_at": "2026-04-25T00:00:00Z",
  "summary": {
    "repo_root": "/path/to/repo",
    "generated_at": "2026-04-25T00:00:00Z",
    "profile": {
      "name": "minimal",
      "source": "built-in:minimal",
      "schema_version": 1,
      "installed_by_version": "0.2.0"
    },
    "repo": {
      "config_version": 1,
      "docs_enabled": true,
      "ci_provider": "none",
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
    "required_reading": [],
    "active_tasks": [],
    "recent_handoffs": [],
    "latest_handoff": null,
    "doctor": {
      "ok": true,
      "diagnostics": []
    }
  },
  "brief": {
    "active_task_count": 0,
    "recent_handoff_count": 0,
    "diagnostic_count": 0,
    "recommended_commands": [
      "project-guardrails tasks list --target .",
      "project-guardrails pre-work --target ."
    ]
  }
}
```

## Resume

`project-guardrails resume --target . --format json` emits a read-only
continuation payload for returning to existing work. It is centered on the
latest durable handoff, the active tasks linked from that handoff, current
doctor diagnostics, and one conservative recommended next step.

`resume` does not write `.guardrails/state/runs/` records, does not mutate task
or handoff records, does not run `refresh`, and does not update managed blocks.

Example shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "generated_at": "2026-04-25T00:00:00Z",
  "latest_handoff": {
    "id": "0001",
    "slug": "continuation-slice",
    "title": "Continuation Slice",
    "status": "open",
    "created": "2026-04-25T00:00:00Z",
    "updated": "2026-04-25T00:00:00Z",
    "task_ids": ["0006"],
    "template_path": "docs/project/handoff-template.md",
    "path": ".guardrails/state/handoffs/0001-continuation-slice.md",
    "body_path": ".guardrails/state/handoffs/0001-continuation-slice.md",
    "body_excerpt": "# Continuation Slice\nTask `0006` is ready to resume."
  },
  "linked_active_tasks": [
    {
      "id": "0006",
      "slug": "resume-core",
      "title": "Resume core",
      "status": "in_progress",
      "owner": "codex",
      "priority": "p1",
      "updated": "2026-04-25T00:00:00Z",
      "path": ".guardrails/state/tasks/0006-resume-core.md"
    }
  ],
  "git": {
    "available": true,
    "status": "available",
    "handoff_timestamp": "2026-04-25T00:00:00Z",
    "baseline_commit": "abc123def456",
    "changed_since_handoff": [
      "src/commands/resume.rs",
      "tests/task_state.rs"
    ],
    "staged_paths": [],
    "unstaged_paths": [
      "src/commands/resume.rs"
    ],
    "untracked_paths": [],
    "diagnostics": []
  },
  "doctor": {
    "ok": true,
    "diagnostics": []
  },
  "next_step": {
    "command": "project-guardrails tasks get 0006 --target .",
    "reason": "latest open handoff .guardrails/state/handoffs/0001-continuation-slice.md links active task 0006"
  }
}
```

`git` is optional continuity context collected from the local `git` binary. It
is an aid for returning to work, not proof of intent, validation, or semantic
completion. The field is always present in `resume --format json`; when Git is
unavailable or insufficient it reports a fallback status and diagnostics
instead of failing the baseline resume command.

`git.status` is one of:

- `available`: Git is usable and a baseline commit at or before the latest
  handoff timestamp was found.
- `no_handoff`: Git is usable, but no latest handoff exists.
- `unavailable`: the target is not a usable Git checkout or the `git` binary
  could not be run.
- `insufficient_baseline`: Git is usable, but the handoff timestamp or history
  was not enough to choose a baseline commit.

`changed_since_handoff` is a conservative path list from the selected baseline
commit to the current working tree plus currently untracked paths. The staged,
unstaged, and untracked arrays report current working tree categories.

When no handoff is available, `latest_handoff` is `null` and
`linked_active_tasks` is empty. The `next_step.command` is selected
deterministically:

- stale managed blocks, stale required docs, task/tracker sync issues, or a
  stale handoff recommend `project-guardrails refresh --target . --check`
- an open latest handoff with linked active tasks recommends
  `project-guardrails tasks get <id> --target .`
- active tasks without a handoff, or with no active task linked from the latest
  handoff, recommend `project-guardrails tasks list --target .`
- a clean doctor report with no handoff-linked active task recommends
  `project-guardrails pre-work --target .`
- other diagnostics recommend `project-guardrails check --target .`

## Timeline

`project-guardrails timeline --target . --format json` emits a read-only
timeline over existing repo-local guardrails state. It reads task records,
handoff records, and existing pre-work run JSON files under `.guardrails/state/`.

The timeline is not a complete audit log. It does not inspect Git history, CI
history, file diffs, or changed files, and it does not run `refresh` or create
new run records.

Task and handoff events use their frontmatter `created` timestamp and, when
different, their `updated` timestamp. Pre-work run events use the stored
`summary.generated_at` value from `.guardrails/state/runs/pre-work-*.json`;
when that value is missing, the command falls back to the timestamp prefix in
the run id.

Events are sorted newest first.

Example shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "generated_at": "2026-04-25T00:00:00Z",
  "events": [
    {
      "timestamp": "2026-04-25T00:00:00Z",
      "kind": "handoff",
      "action": "updated",
      "id": "0001",
      "title": "Slice 1",
      "status": "open",
      "path": ".guardrails/state/handoffs/0001-slice-1.md",
      "task_ids": ["0001"]
    },
    {
      "timestamp": "2026-04-24T20:00:00Z",
      "kind": "pre_work_run",
      "action": "recorded",
      "id": "20260424T200000Z-1a2b3c4d",
      "title": "Pre-work run",
      "path": ".guardrails/state/runs/pre-work-20260424T200000Z-1a2b3c4d.json"
    }
  ],
  "diagnostics": []
}
```

## Adapters List

`project-guardrails adapters list --target . --format json` reports
profile-declared adapter targets without installing or refreshing adapter
files.

Adapters are profile-owned managed targets. The command is read-only: it loads
the repo-local config, resolves the installed profile, checks whether each
declared target path exists, and reports managed-block declarations for that
path. It does not create adapter files and there is no `adapters install`
flow.

Example shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "profile": "adapter-fixture",
  "profile_source": "custom:/path/to/profile/profile.toml",
  "adapters": [
    {
      "kind": "claude",
      "name": "Claude Code",
      "path": ".claude/CLAUDE.md",
      "source_profile": "custom:/path/to/profile/profile.toml",
      "exists": true,
      "managed_blocks": [
        {
          "id": "adapter-context",
          "generator": "repo_context_v1",
          "placement": "after_first_heading",
          "exists": true
        }
      ]
    }
  ]
}
```

When a profile declares no adapter targets, `adapters` is an empty array.

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
      "installed_by_version": "0.2.0"
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

## Refresh

`project-guardrails refresh --format json` reports the active profile's
declared managed blocks and whether each block was already current, changed, or
could not be refreshed.

`project-guardrails refresh --check --format json` uses the same shape, but
does not write files and exits non-zero when any declared block would change.

Example shape:

```json
{
  "schema_version": 1,
  "ok": false,
  "repo_root": "/path/to/repo",
  "check": true,
  "changed": true,
  "changed_paths": ["AGENTS.md"],
  "blocks": [
    {
      "path": "AGENTS.md",
      "id": "repo-context",
      "generator": "repo_context_v1",
      "status": "would_change"
    }
  ],
  "diagnostics": []
}
```

Current block status values are:

- `unchanged`
- `changed`
- `would_change`
- `missing_file`
- `invalid`
- `error`

Missing target files, invalid managed-block markup, and generator failures are
reported as diagnostics. Refresh only rewrites declared managed-block regions in
existing files; it does not recreate whole missing files.

## Doctor

Example shape:

```json
{
  "schema_version": 1,
  "ok": false,
  "repo_root": "/path/to/repo",
  "profile": "docs-driven",
  "profile_source": "built-in:docs-driven",
  "installed_by_version": "0.2.0",
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
    "installed_by_version": "0.2.0",
    "ci_provider": "github"
  },
  "target": {
    "profile": "docs-driven",
    "profile_source": "built-in:docs-driven",
    "profile_schema_version": 1,
    "installed_by_version": "0.2.0",
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

`handoff new --from-git --format json` uses the same JSON shape. The generated
`handoff.body` is a draft based on observable Git working tree state and must
not be treated as proof of intent, validation, or semantic completion. Without
`--from-git`, `handoff new` continues to use the configured handoff template.

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
