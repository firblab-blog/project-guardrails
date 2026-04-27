# Architecture

## Purpose

`project-guardrails` exists to make repo-local project guardrails portable.

Many repos want stronger structure than linting provides:

- required docs
- contributor workflow expectations
- architecture guardrails
- CI protection for repo shape and implementation boundaries

The hard part is not inventing those rules once.
The hard part is packaging them as an installable, understandable starter layer
that any repo can adopt without inheriting someone else's filesystem or language
assumptions.

## Core Model

The system has three practical layers:

1. runtime
2. profile
3. installed assets

### Runtime

The runtime is the Rust CLI bootstrap utility.

It owns:

- repo-root detection
- config loading
- profile loading
- bootstrap/install orchestration
- upgrade planning
- local diagnostic and check commands, including preflight validation

It should stay small, portable, and relatively opinion-free.

### Profile

A profile is a named package of installable repo opinions.

It owns:

- required docs
- required files
- forbidden directories
- default CI provider
- whether handoff docs are expected
- external rule-engine defaults such as Semgrep and Conftest policy locations

Profiles may also own:

- Semgrep rule sets
- OPA policies
- custom scripts
- prompt templates
- support docs

### Installed Assets

Installed assets are the files materialized into a consumer repository.

Examples:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- CI workflow snippets

These assets are the main product surface.
They make the guardrails durable inside the consumer repo instead of living only
in the installer.

Profiles may also carry an optional `assets/` tree whose contents are copied
into the target repo during bootstrap. This is the intended V0 path for
shipping rule files, support assets, and other repo-local scaffolding without
hardcoding them into the Rust CLI.

## Portability Rules

Portability depends on four design choices:

1. detect repo roots dynamically instead of hardcoding paths
2. keep project-specific policy in profiles rather than in CLI logic
3. generate repo-local assets that remain understandable and editable without
   the installer
4. test bootstrap behavior against fixture repos in different shapes

## V0 Command Model

V0 commands:

- `guardrails init`
- `guardrails upgrade --plan`
- `guardrails upgrade --apply`
- `guardrails status`
- `guardrails doctor`
- `guardrails check`
- `guardrails handoff new|list|close`

These commands are intentionally small.
The goal is to prove the bootstrap and packaging model first.

Current command-surface decision:

- keep `guardrails handoff` as a top-level command in V0.1
- center the durable workflow on `handoff new|list|close`
- keep bare `guardrails handoff` only as a compatibility path that prints the
  installed template

## Future Expansion

Later phases may add:

- richer file and glob checks
- profile inheritance
- upgrade commands
- pre-commit integration
- test harnesses for fixture repos

Those should follow the same rule:
keep the CLI generic and move opinions outward into profiles and copied files.
