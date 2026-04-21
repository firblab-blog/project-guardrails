# Install Ownership

This document defines what `project-guardrails` considers tool-managed after
`init` or `upgrade`.

The short first-run version is:

- `init` creates a repo-local starting baseline
- `.guardrails/profile.lock` is the ownership ledger for that baseline
- teams are expected to edit the installed docs and instructions
- `upgrade` stays conservative and only auto-removes a narrow set of stale CI
  files

## Ownership Model

The installer writes repo-local files and records them in
`.guardrails/profile.lock`.

That lockfile is now structured TOML with one `[[managed_paths]]` entry per
tool-managed file.

Each entry records:

- `path`
  - the repo-relative file path the installer manages
- `stale_action`
  - what `upgrade` should do later if that path is no longer part of the
    selected profile or CI configuration

Current `stale_action` values:

- `review`
  - surface the path in `review_stale_paths`
  - never auto-delete it during `upgrade --apply`
- `remove`
  - allow `upgrade --apply` to remove the stale file automatically

## What The Tool Owns

The tool owns files it installed from:

- shared templates
- profile-local templates
- profile `assets/`
- generated config/lock metadata
- generated CI workflow files

Typical examples:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `AGENTS.md`
- required project docs such as tracker or handoff files
- selected CI workflow wiring
- profile-owned support assets copied from `assets/`

These files are still repo-local and editable by the consuming project.
`project-guardrails` is not a hosted controller.
Ownership here means the installer knows it created the starting file and can
reason about it later.

That is why the `init` handoff now points people to:

- what was created
- what the tool owns
- what they should edit first
- what command to run next

## What The Tool Only Validates

`doctor` and `check` validate expectations from the active config/profile, such
as:

- required docs existing
- required files existing
- forbidden directories not existing
- optional Semgrep/Conftest setup being runnable when enabled

Those validations do not make every matching file auto-removable.

## Current Deletion Contract

For V0.1, `upgrade --apply` only auto-removes stale files whose
`stale_action = "remove"`.

Today that is intentionally narrow:

- generated GitHub and GitLab guardrails workflow files when they become stale

Everything else remains review-only by default, including:

- docs
- `AGENTS.md`
- copied profile assets
- other repo-local files a team may have edited after install

That keeps upgrades conservative and explainable while the public contract is
still maturing.

## Editing Expectations

Teams are expected to edit installed docs and instructions after bootstrap.

If a tool-managed file is still part of the active profile:

- `init --force` may rewrite it
- `upgrade --apply` may rewrite it

If a tool-managed file is review-only and still active, that does not make it
"untouchable."
It means the file remains editable in the repo, but the installer may refresh
the baseline copy later when you explicitly reapply it.

If a file becomes stale:

- `upgrade --plan` should surface it
- `upgrade --apply` should only auto-delete it when the lockfile marks it as
  `remove`

The intended workflow is:

1. run `project-guardrails upgrade --target . --plan`
2. inspect `stale_paths`, `removable_stale_paths`, and `review_stale_paths`
3. apply the upgrade
4. review the resulting Git diff before committing
