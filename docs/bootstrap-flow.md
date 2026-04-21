# Bootstrap Flow

This document describes the intended V0 installation flow for a consumer repo.

## 1. Choose A Profile

Start with one of the built-in profiles:

- `minimal`
- `docs-driven`

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
cargo run -- init --target /path/to/repo --profile minimal --ci github
```

If `--ci` is omitted, `init` uses the selected profile's `default_ci`.

Custom profile example:

```bash
cargo run -- init --target /path/to/repo --profile my-profile --profile-path /path/to/profile-dir --ci gitlab
```

Dry-run first if desired:

```bash
cargo run -- init --target /path/to/repo --profile minimal --ci github --dry-run
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
cargo run -- doctor --target /path/to/repo
cargo run -- check --target /path/to/repo
cargo run -- handoff --target /path/to/repo
```

`doctor` should be the first validation pass. It checks for missing required
docs/files and broken external-engine setup before `check` attempts to execute
those engines.

`handoff` is intentionally lightweight in V0.1.
It simply prints the repo-local handoff template so contributors can use the
same installed file directly.

If another tool needs to consume these results, use `--format json`.

## 5. Customize

Edit `.guardrails/guardrails.toml`, `AGENTS.md`, and any copied project docs to
reflect the target project's actual rules and workflows.

That is the main portability contract:
the installer writes a legible starting point, and the target repo owns the
specific rules from there.

## 6. Upgrade Later

For V0, use `guardrails upgrade --plan` first. Then either:

- run `guardrails upgrade --apply` for the thin wrapper path
- or rerun `init --force` manually if you want direct control

See `docs/upgrade-story.md` for the explicit contract.
See `docs/install-ownership.md` for the ownership model.
