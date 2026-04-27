# Bootstrap Flow

This document describes the intended V0 installation flow for a consumer repo.

## 1. Choose A Profile

Start with one of the built-in profiles:

- `minimal`
- `docs-driven`
- `guardrails`

The profile chooses the default opinions and copied repo-local assets.

If a project wants its own profile package, point `init` at a directory or
`profile.toml` file with `--profile-path`.

Custom profiles may include:

- `profile.toml`
- `templates/`
- `assets/`

## 2. Run Init

Example:

```bash
project-guardrails init --target /path/to/repo --profile minimal --ci github
```

If `--ci` is omitted, `init` uses the selected profile's `default_ci`.

Custom profile example:

```bash
project-guardrails init --target /path/to/repo --profile my-profile --profile-path /path/to/profile-dir --ci gitlab
```

Dry-run first if desired:

```bash
project-guardrails init --target /path/to/repo --profile minimal --ci github --dry-run
```

## 3. Review Installed Assets

After bootstrap, review:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- CI workflow file
- any profile-owned files copied from `assets/`

Depending on the selected profile, additional docs or support assets may also be
materialized.

These files are intentionally repo-local so the project remains understandable
and editable even if the installer is not present later.

`project-guardrails` also records install ownership in
`.guardrails/profile.lock`.
That file tells `upgrade` which stale files are review-only and which are safe
to auto-remove.

## 4. Validate

Run:

```bash
project-guardrails brief --target /path/to/repo
project-guardrails resume --target /path/to/repo
project-guardrails refresh --target /path/to/repo --check
project-guardrails doctor --target /path/to/repo
project-guardrails check --target /path/to/repo
project-guardrails handoff list --target /path/to/repo
```

`brief` is the read-only new-session starter. `resume` is the read-only
continuation view centered on the latest durable handoff and linked active
tasks. `refresh --check` reports stale managed blocks without writing files.
`doctor` checks for missing required docs/files and broken external-engine
setup before `check` attempts to execute those engines.

`pre-work` is still available when a session should persist the structured repo
summary under `.guardrails/state/runs/`.

For durable repo-local workflow state, use `handoff new|list|close` under
`.guardrails/state/handoffs/`.
Use `tasks new|claim|update|close|lint` under `.guardrails/state/tasks/` for
approved work state.
Bare `project-guardrails handoff --target /path/to/repo` only prints the installed
handoff template for compatibility.

If another tool needs to consume these results, use `--format json`.
If an MCP-capable local client needs the same operations over stdio, configure
it to run `project-guardrails mcp serve --target /path/to/repo`.

## 5. Customize

Edit `.guardrails/guardrails.toml`, `AGENTS.md`, and any copied project docs to
reflect the target project's actual rules and workflows.

That is the main portability contract:
the installer writes a legible starting point, and the target repo owns the
specific rules from there.

## 6. Upgrade Later

For V0, use `project-guardrails upgrade --plan --target /path/to/repo` first. Then either:

- run `project-guardrails upgrade --apply --target /path/to/repo` for the thin wrapper path
- or rerun `init --force` manually if you want direct control

See `docs/upgrade-story.md` for the explicit contract.
See `docs/install-ownership.md` for the ownership model.
