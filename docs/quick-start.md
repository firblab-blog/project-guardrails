# Quick Start

This is the canonical first-time adoption path for `project-guardrails`.

Follow this sequence:

1. install the CLI with Homebrew, `crates.io`, or the tagged GitHub release
   archive for your platform
2. run `init`
3. edit the installed files and create a repo-local task for approved work
4. start sessions with `brief` or `resume`
5. run `refresh`, `doctor`, and `check`
6. enable the repo-tracked commit checks locally and wire the same baseline into CI
7. use durable handoffs, adapter targets, MCP, and `upgrade` as needed

If you want the neutral public cross-language starting point, use the built-in
`minimal` profile shown below.

Keep the first-run decision simple:

- choose `--profile minimal` unless you already know you want a stricter
  built-in doctrine
- choose `--profile docs-driven` only when that stronger doc baseline is
  intentional
- choose `--profile guardrails` only when you want the opt-in FirbLab-style
  doctrine profile with seeded `docs/best-practices/` content
- choose `--ci github` for GitHub Actions, `--ci gitlab` for GitLab CI, or
  `--ci none` if you want to wire CI later

To discover the built-in set from the CLI, run:

```bash
project-guardrails profiles list
```

## 1. Install The CLI

The recommended public install path is Homebrew:

```bash
brew install firblab-blog/tap/project-guardrails
project-guardrails --version
```

Rust/Cargo users can install from `crates.io`:

```bash
cargo install project-guardrails --locked
project-guardrails --version
```

The fallback public path is the tagged GitHub release archive for your
platform. The copy-paste archive steps live in [docs/install.md](install.md).

If you want to build from source, use the developer path from a checkout:

```bash
cargo install --path .
project-guardrails --version
```

If one of those paths works, move to the repository you want to bootstrap.

## 2. Run `init`

Inside the repository you want to bootstrap, run:

```bash
project-guardrails init --target . --profile minimal --ci github
```

Use `--ci gitlab` instead if your repo is GitLab-first:

```bash
project-guardrails init --target . --profile minimal --ci gitlab
```

If you omit `--ci`, the selected profile's default CI is used.
For the current built-in profiles, that default is `github`.

For the current built-in profiles, `init` installs:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- `.pre-commit-config.yaml`
- one guardrails CI file for your selected provider

What the `init` success output should tell you right away:

- what was created
- what the tool still tracks as managed
- what you should edit first
- what command to run next

Stay in the target repo after `init`.
The basic next step should be visible directly in the CLI output.

## 3. Edit The Installed Files

Start with these files:

- `README.md`
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- `.guardrails/guardrails.toml`

Treat the installed content as a starting point, not finished truth.
The tool bootstraps a baseline; your repository owns the day-to-day content
after that.

If you are bootstrapping a nearly empty repository, create or confirm
`README.md` now before running `doctor` or `check`.
The default baseline requires a real top-level README.

Ownership works like this:

- `.guardrails/profile.lock` records which installed paths are tool-managed
- docs, `AGENTS.md`, config, and copied assets stay editable and review-first
  even when tracked
- stale CI workflow wiring is the main thing `upgrade --apply` may remove
  automatically when it no longer matches the selected CI provider

Also review the generated CI file before you wire runners:

- `.github/workflows/guardrails.yml`
- `.gitlab-ci.guardrails.yml`

Only one of those exists, depending on `--ci`.

If you want to see a fresh post-`init` example before editing, see
[docs/minimal-init-snapshot.md](minimal-init-snapshot.md).

For substantial work, create and claim a durable repo-local task:

```bash
project-guardrails tasks new --target . --slug first-slice --title "First slice"
project-guardrails tasks claim 1 --target . --owner your-name
```

Use `tasks list`, `tasks get`, `tasks update`, and `tasks close` to keep the
approved work state under `.guardrails/state/tasks/`.

## 4. Start From Current Context

Use `brief` when a new session needs a compact repo-local starting point:

```bash
project-guardrails brief --target .
```

Use `resume` when returning to existing work:

```bash
project-guardrails resume --target .
```

`brief` and `resume` are read-only. They surface required reading, active task
state, recent handoffs, diagnostics, and recommended next commands, but they do
not create run records or refresh files.

When you want a durable run record for auditability, use:

```bash
project-guardrails pre-work --target .
```

That writes the emitted structured repo summary under
`.guardrails/state/runs/`.

## 5. Validate The Repo

Run:

```bash
project-guardrails refresh --target . --check
project-guardrails doctor --target .
project-guardrails check --target .
```

Then enable the repo-tracked local hook adapter:

```bash
pre-commit install --hook-type pre-commit --hook-type commit-msg
```

Use this mental model:

- `refresh --check` verifies declared managed-block freshness without writing
- `doctor` checks that the expected repo-local files and configured tools exist
- `check` runs the configured local rules and repo checks
- `brief` and `resume` are the read-only session starters
- `pre-work` captures a persistent structured repo summary when you need one

That local install step is just the adapter.
The primary reviewable policy lives in the repo-tracked
`.pre-commit-config.yaml`, which runs:

- `project-guardrails pre-commit` at `pre-commit` time
- `project-guardrails commit-msg-check` at `commit-msg` time

For the current built-in profiles, `commit-msg-check` expects staged work to
reference at least one active task, for example `[task:0001]` or `refs #0001`.

If `doctor` fails, fix the missing file or prerequisite first.
If `check` fails, update your repo-local configuration or project files until
the baseline passes.

When work needs a durable repo-local handoff record, use:

```bash
project-guardrails handoff new --target . --slug first-slice --task 1 --title "First slice"
project-guardrails handoff list --target .
project-guardrails handoff close 1 --target .
```

Bare `project-guardrails handoff --target .` only prints the current handoff
template for compatibility.

If managed context has drifted because task or handoff state changed, repair
only the declared managed blocks with:

```bash
project-guardrails refresh --target .
```

`refresh` does not reapply whole templates, install adapter files, or rewrite
repo-local config.

## 6. Add CI

The generated CI file is a baseline, not a complete pipeline.
For `v0.1`, it does not install `project-guardrails` for you.

The practical CI sequence is:

1. make `project-guardrails` available on the runner
2. run `project-guardrails doctor --target .`
3. run `project-guardrails check --target .`

Review the generated file in your repo:

- `.github/workflows/guardrails.yml`
- `.gitlab-ci.guardrails.yml`

Only one of those will exist, depending on `--ci`.

Use the matching reference doc when you are ready to wire the runner setup:

- GitHub Actions:
  [docs/ci-provisioning.md](ci-provisioning.md)
- GitLab CI:
  [docs/gitlab-ci.md](gitlab-ci.md)

Use those docs differently depending on the provider:

- GitHub Actions:
  replace the generated workflow with the canonical repo-owned GitHub Actions
  recipe that downloads the pinned release binary from
  `firblab-blog/project-guardrails`, verifies `SHA256SUMS`, then runs
  `doctor` and `check`
- GitLab CI:
  keep the generated `.gitlab-ci.guardrails.yml` file, and add a repo-owned
  root `.gitlab-ci.yml` that provisions the pinned release binary in `.pre`
  and includes the generated file as shown in `docs/gitlab-ci.md`

If your runners already have `project-guardrails` on `PATH`, you can skip the
download step and keep the rest of the recipe.
If no release asset exists for your runner platform, use a preinstalled runner
image or another repo-owned provisioning path that places the binary on `PATH`.

## 7. Use Adapters And MCP Where They Fit

Profile-declared adapter targets make host-specific instruction files visible
without turning them into a separate install subsystem:

```bash
project-guardrails adapters list --target .
```

Adapter targets are ordinary repo-local files and managed blocks. A profile may
ship files such as `CLAUDE.md` or Cursor `.mdc` rules through its normal
templates, but `adapters list` itself is read-only and there is no
`adapters install` command.

For MCP-capable local clients, configure the client to launch:

```bash
project-guardrails mcp serve --target /absolute/path/to/repo
```

The MCP server is repo-scoped and stdio-only. It exposes the same typed
operations used by the CLI for brief/resume/status, tasks, handoffs, refresh,
doctor, check, and timeline. It is not a hosted service, remote registry, or
agent host.

## 8. Upgrade Later

Preview changes before applying them:

```bash
project-guardrails upgrade --target . --profile minimal --ci github --plan
```

Apply when ready:

```bash
project-guardrails upgrade --target . --profile minimal --ci github --apply
```

If your repo uses GitLab, switch the CI flag to `gitlab`.

You can also change profiles during upgrade.
For example, moving from `minimal` to `docs-driven` adds a required decision
log:

```bash
project-guardrails upgrade --target . --profile docs-driven --ci github --plan
```

Review the resulting diff before committing.
`project-guardrails` keeps the CLI small and relies on repo-local files plus
Git review for the final decision.

## When To Reach For More

Stay on `minimal` if you want the smallest neutral public baseline.

Move to `docs-driven` if you want:

- everything from `minimal`
- a required `docs/project/decision-log.md`

Move to `guardrails` if you want:

- an opt-in FirbLab-style operating doctrine
- seeded `AGENTS.md`, tracker, decision log, and implementation invariants
- curated `docs/best-practices/` docs installed with the profile

Use a custom profile if your team needs project-specific files, templates, CI
paths, or extra required docs.
That keeps the CLI portable while moving doctrine into the profile layer where
it belongs.
