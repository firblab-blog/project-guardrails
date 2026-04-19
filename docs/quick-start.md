# Quick Start

This is the canonical first-time adoption path for `project-guardrails`.

Follow this sequence:

1. install the CLI from the tagged GitHub release for your platform
2. run `init`
3. edit the installed files
4. run `doctor` and `check`
5. wire the same commands into CI
6. use `upgrade` later when you want to refresh the baseline

If you want the smallest public cross-language starting point, use the built-in
`minimal` profile shown below.

Keep the first-run decision simple:

- choose `--profile minimal` unless you already know you want a required
  decision log
- choose `--profile docs-driven` only when that stronger doc baseline is
  intentional
- choose `--ci github` for GitHub Actions, `--ci gitlab` for GitLab CI, or
  `--ci none` if you want to wire CI later

## 1. Install The CLI

The recommended public install path is the tagged GitHub release archive for
your platform.
The copy-paste archive steps live in [docs/install.md](install.md).

If `project-guardrails` is published on `crates.io`, you can also install and
verify it with:

```bash
cargo install project-guardrails --locked
project-guardrails --version
project-guardrails --help
```

For a pinned Cargo install, use:

```bash
cargo install project-guardrails --locked --version VERSION
```

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

What `init` installs:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
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

## 4. Validate The Repo

Run:

```bash
project-guardrails doctor --target .
project-guardrails check --target .
```

If your selected profile includes handoff support, also run:

```bash
project-guardrails handoff --target .
```

Use this mental model:

- `doctor` checks that the expected repo-local files and configured tools exist
- `check` runs the configured local rules and repo checks
- `handoff` prints the repo-local handoff template for contributor workflows

If `doctor` fails, fix the missing file or prerequisite first.
If `check` fails, update your repo-local configuration or project files until
the baseline passes.

## 5. Add CI

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
If no release asset exists for your runner platform, use the documented Cargo
install fallback instead.

## 6. Upgrade Later

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

Stay on `minimal` if you want the smallest public baseline.

Move to `docs-driven` if you want:

- everything from `minimal`
- a required `docs/project/decision-log.md`

Use a custom profile if your team needs project-specific files, templates, CI
paths, or extra required docs.
That keeps the CLI portable while moving doctrine into the profile layer where
it belongs.
