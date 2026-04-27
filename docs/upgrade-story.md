# Upgrade Story

This document defines the first explicit upgrade contract for
`project-guardrails`.

## Goals

Upgrades should be:

- understandable from repo-local files
- safe to review before applying
- explicit about profile source and schema version
- able to evolve built-in and custom profiles without silent drift

## Current Version Markers

V0 now writes upgrade-relevant metadata to:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`

Relevant fields include:

- `version`
- `profile`
- `profile_source`
- `profile_schema_version`
- `installed_by_version`
- structured `[[managed_paths]]` entries for tool-managed installed files
- per-path `stale_action` markers that distinguish review-only files from
  auto-removable ones
- preserved state paths such as `.guardrails/state/`

## V0 Upgrade Contract

For V0, the upgrade flow is intentionally conservative:

1. inspect current config and profile lock
2. choose whether to stay on the same profile or move to a different one
3. run `guardrails upgrade --plan` with the intended built-in or custom profile
4. run `guardrails upgrade --apply` with the same intended profile, or rerun
   `guardrails init --force` manually
5. review the resulting repo-local file changes before committing
6. rerun `guardrails doctor` and `guardrails check`

`doctor` is the intended early warning step for:

- missing generated docs
- missing required files
- missing CI workflow files
- missing or broken Semgrep/Conftest configuration
- stale declared managed blocks that can be repaired with `refresh`

Use `project-guardrails refresh --target .` before considering an upgrade when
the only problem is stale generated context inside profile-declared managed
blocks. Refresh does not reapply whole-file templates, copy assets, rewrite CI,
or update `.guardrails/guardrails.toml`.

## Why Reapply Is The V0 Upgrade Mechanism

V0 does not yet carry a complex built-in migration path.

Instead, it relies on:

- declarative config
- profile source metadata
- the same underlying bootstrap path used by `init`
- limited removal of stale managed CI workflow files when the selected provider changes
- structured install ownership recorded in `.guardrails/profile.lock`
- Git review of file-level changes

That keeps upgrades auditable while the product is still proving its portable
bootstrap and packaging model.

## Current Reconciliation Rules

For V0.1:

- generated stale CI workflow files may be auto-removed
- stale docs and copied assets are surfaced for manual review instead
- durable state paths marked `preserve` stay in place across reapply flows
- `upgrade --apply` refreshes declared managed blocks for files that are still
  managed by the selected target profile
- `upgrade --apply` refreshes whole-file managed content only when the current
  file still matches the last installed baseline hash
- if a whole-file managed path has repo-owned edits since the last install,
  `upgrade --apply` leaves it in place rather than overwriting it blindly

That contract is intentionally conservative.
See `docs/install-ownership.md` for the detailed ownership rules.

## Future Direction

Later versions may add:

- richer profile diff output
- config migrations between schema versions
- warnings when built-in profiles have changed materially since install

Those should build on the metadata already written in V0 rather than replacing
it with opaque state.
