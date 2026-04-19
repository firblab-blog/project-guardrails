# Output Schema

This document defines the initial machine-readable output contract for
`project-guardrails`.

Supported commands:

- `guardrails status --format json`
- `guardrails doctor --format json`
- `guardrails check --format json`
- `guardrails upgrade --plan --format json`

The default remains `--format text`.

## Stability Intent

These JSON shapes are intended to be stable within the current major version of
the bootstrap utility.

Fields may grow over time, but existing fields should not be removed or changed
casually once other tools begin consuming them.

## Status

Example shape:

```json
{
  "schema_version": 1,
  "repo_root": "/path/to/repo",
  "profile": "minimal",
  "profile_source": "built-in:minimal",
  "profile_schema_version": 1,
  "installed_by_version": "0.1.6",
  "docs_enabled": true,
  "ci_provider": "github",
  "required_files": ["README.md", "AGENTS.md", ".guardrails/guardrails.toml"],
  "forbidden_dirs": [],
  "semgrep_enabled": false,
  "conftest_enabled": false
}
```

## Doctor

Example shape:

```json
{
  "schema_version": 1,
  "ok": false,
  "repo_root": "/path/to/repo",
  "profile": "docs-driven",
  "profile_source": "built-in:docs-driven",
  "installed_by_version": "0.1.6",
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
    "installed_by_version": "0.1.6",
    "ci_provider": "github"
  },
  "target": {
    "profile": "docs-driven",
    "profile_source": "built-in:docs-driven",
    "profile_schema_version": 1,
    "installed_by_version": "0.1.6",
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
  "review_stale_paths": [],
  "planned_actions": [
    "reapply the selected profile against the target repo"
  ]
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
