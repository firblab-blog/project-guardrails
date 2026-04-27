# project-guardrails

Stop starting every AI coding session from amnesia.

`project-guardrails` gives AI-assisted projects a repo-local operating loop:
session briefings, resumable handoffs, durable tasks, managed context blocks,
adapter files, rule-pack-backed checks, and a local MCP access layer. It is
for teams that want the working memory and enforcement surface to live in the
repository, where humans can review it.

`project-guardrails` is a Rust-first bootstrap utility that installs
repo-local guardrails into an existing repository. The CLI stays small.
Project-specific opinions live in profiles, templates, and the files copied
into the target repo.

It does not prove that an LLM read or understood the repo. It makes the right
context easier to find, refresh, inspect, and validate through portable proxy
signals.

For this repository, the public GitHub-facing surface is intentionally:

- GitHub Releases for prebuilt archive downloads
- source checkout for local inspection and developer builds
- public docs, templates, profiles, examples, and tests

Mirror/export policy stays in repo-owned docs and scripts.
It is not a runtime feature of the CLI.
For maintainers, the GitHub public repo is expected to be produced by an
allowlisted export from the private GitLab source of truth, not by a CLI
publish command.

## Current Status

For `v0.1`, the supported public install paths are:

- Homebrew for the default one-command install path
- `crates.io` for Rust/Cargo users
- tagged GitHub release archives as the fallback path for everyone else

Today, that means:

- `brew install firblab-blog/tap/project-guardrails`
- `cargo install project-guardrails --locked`
- or download the release archive for your platform from GitHub Releases,
  verify it against `SHA256SUMS`, extract the binary, and place it on your
  `PATH`

`project-guardrails` does not currently provide:

- curl-to-install scripts
- signed release artifacts

If you want the complete copy-paste install commands and asset names, use
[docs/install.md](docs/install.md).

If you are adopting `project-guardrails` for the first time, the canonical
flow is:

1. install the CLI with Homebrew, `crates.io`, or the matching tagged GitHub
   release for your platform
2. run `init`
3. edit the installed repo-local files, keep declared managed blocks intact,
   and create repo-local task state for approved work with `tasks new`
4. use `brief` or `resume` to start sessions from current repo-local context
5. run `refresh`, `doctor`, and `check` as the mechanical validation loop
6. use `handoff new` during work, enable the repo-tracked commit checks
   locally, and wire the same baseline into CI
7. use adapter files or the MCP server when your local AI client supports them,
   and use `upgrade` later to refresh the installed baseline

The end-to-end walkthrough lives in [docs/quick-start.md](docs/quick-start.md).
MCP-capable local clients can use the stdio server documented in
[docs/mcp.md](docs/mcp.md).

The intended install order is:

1. Homebrew
2. `crates.io`
3. GitHub release archives

## Install

Today, the easiest install path is:

```bash
brew install firblab-blog/tap/project-guardrails
```

The secondary install path for Rust users is:

```bash
cargo install project-guardrails --locked
```

The fallback install contract is the tagged GitHub release archive.
That means the public GitHub release needs to contain:

- `project-guardrails-vVERSION-x86_64-unknown-linux-gnu.tar.gz`
- `project-guardrails-vVERSION-x86_64-apple-darwin.tar.gz`
- `project-guardrails-vVERSION-aarch64-apple-darwin.tar.gz`
- `project-guardrails-vVERSION-x86_64-pc-windows-msvc.zip`
- `SHA256SUMS`

If a release does not contain an archive for your platform, use the
`crates.io` path if Cargo is available.
Otherwise, the remaining path is a developer build from a source checkout.

Developer/source builds are for:

- maintainers working on `project-guardrails`
- users who want to inspect and build the tagged source directly

See [docs/install.md](docs/install.md) for the full install contract,
copy-paste release install steps, and checksum verification.

## Bootstrap And Configure

The recommended install path order today is:

- if Homebrew is available: `brew install firblab-blog/tap/project-guardrails`
- if you already use Cargo: `cargo install project-guardrails --locked`
- if you do not want Homebrew or Cargo on the machine: use the matching tagged
  GitHub release archive for your platform

The full copy-paste archive instructions live in [docs/install.md](docs/install.md).

If you are developing on `project-guardrails` itself, or want to build from a
local checkout, use:

```bash
cargo install --path .
```

Next, switch to the repository you want to bootstrap and run:

```bash
project-guardrails init --target . --profile minimal --ci github
```

Use this default unless you have a clear reason not to:

- `--profile minimal`
  The neutral cross-language starting point and the expected first run.
- `--profile docs-driven`
  Choose this only when you want the `minimal` baseline plus a required
  `docs/project/decision-log.md`.
- `--profile guardrails`
  Opt-in FirbLab-style doctrine profile with seeded operating guidance,
  implementation invariants, and curated `docs/best-practices/` docs.
- `--ci github`
  Writes a GitHub Actions guardrails workflow.
- `--ci gitlab`
  Writes a GitLab guardrails include file.
- `--ci none`
  Skips CI file generation so you can bootstrap locally first.

If you use GitLab instead of GitHub, change only the CI flag:

```bash
project-guardrails init --target . --profile minimal --ci gitlab
```

After `init`, the CLI handoff tells you four things in order:

- what it created
- what stays tool-managed
- what to edit first
- what command to run next

The same first-run sequence is:

- `README.md`
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- `.pre-commit-config.yaml`
- `.guardrails/guardrails.toml`

If you are starting from a nearly empty repository, create or confirm
`README.md` before running `doctor` or `check`.
The default baseline treats a real top-level README as required.

Then run:

```bash
project-guardrails brief --target .
project-guardrails refresh --target . --check
project-guardrails doctor --target .
project-guardrails check --target .
```

`brief` is the read-only new-session entrypoint. `resume` is the read-only
continuation entrypoint centered on the latest durable handoff and active task
context. `pre-work` remains available when you want to persist a structured
`LlmRepoSummary` run under `.guardrails/state/runs/` for repo-local
auditability.

To enable the repo-tracked commit checks that `init` now installs, run:

```bash
pre-commit install --hook-type pre-commit --hook-type commit-msg
```

That local install step is just the adapter.
Keep `--hook-type commit-msg` in place: that is what activates
`project-guardrails commit-msg-check` in addition to the staged-file checks.
The primary reviewable policy lives in the repo-tracked
`.pre-commit-config.yaml`.

For the current built-in profiles, the repo-tracked enforcement surface can
catch:

- forbidden directory changes via `forbidden_dirs`
- missing companion docs via `link_requirements`
- staged forbidden patterns via `forbidden_patterns`
- stock placeholder text via starter-content checks
- commit messages that fail to reference an active task

For durable repo-local workflow state, use:

```bash
project-guardrails brief --target .
project-guardrails resume --target .
project-guardrails tasks list --target .
project-guardrails handoff list --target .
project-guardrails refresh --target . --check
project-guardrails adapters list --target .
```

Use `tasks new|claim|update|close|lint` to manage approved work under
`.guardrails/state/tasks/`.
Use `handoff new|list|close` to record contributor handoffs under
`.guardrails/state/handoffs/`.
Use `refresh` to update declared managed blocks after task or handoff state
changes. Use `adapters list` to inspect profile-declared host adapter targets;
there is no separate adapter install command.
Bare `project-guardrails handoff --target .` only prints the current template
for compatibility; it is not the main recommended path.

For MCP-capable clients, `project-guardrails mcp serve --target .` exposes the
same repo-scoped operations over local stdio transport. It is an access layer
over the CLI's typed operations, not a hosted service or agent runtime.

For the current built-in profiles, the local `commit-msg` hook also runs
`project-guardrails commit-msg-check`, which expects staged work to reference
at least one active task, for example `[task:0001]` or `refs #0001`.

Then review the generated CI file and make sure your runner can actually
invoke `project-guardrails`. The generated CI baseline does not provision the
binary for you in `v0.1`.

Supported automation-facing commands also support `--format json`.
That JSON surface carries `schema_version = 1` today and is the intended
machine-readable contract for CI integrations, downstream tools, and agent
workflows. See [docs/output-schema.md](docs/output-schema.md) for the exact
supported JSON command list.

## Managed Blocks And Freshness

Some installed files now include declared managed blocks rather than being
treated as all-or-nothing generated text. The built-in profiles currently use
two block generators:

- `repo_context_v1` for the managed repo-context block in `AGENTS.md`
- `tracker_sync_v1` for the managed task-sync block in
  `docs/project/implementation-tracker.md`

Those blocks keep required context, active tasks, and handoff state visible in
repo-local docs without claiming to prove that a human or LLM actually read
them.

Edit around those blocks rather than deleting them wholesale. If you remove or
drift a declared block, `doctor` and `check` can emit diagnostics such as
`managed_block_missing` or `managed_block_stale`, and a conservative recovery
path is to rerun `init --force` or `upgrade --apply` before continuing edits.

`refresh` is the normal lightweight repair path for stale declared blocks:

```bash
project-guardrails refresh --target .
project-guardrails refresh --target . --check
```

It refreshes only declared block regions in existing files. It does not reapply
whole templates, create missing adapter files, or prove the generated context
was read.

## Modify And Maintain

Once the baseline is installed, the normal maintenance loop is:

- edit the installed repo-local files so they become your real project docs and instructions
- create or claim a task before substantial work
- start sessions with `brief` or `resume`
- refresh managed blocks after task and handoff state changes
- rerun `doctor` and `check`
- review CI wiring when your runner setup changes
- record durable handoffs when work should be resumable
- use `upgrade --plan` before applying a newer baseline

Preview a later refresh with:

```bash
project-guardrails upgrade --target . --profile minimal --ci github --plan
```

## Built-In Profiles

The built-in public profile set is intentionally small:

- `minimal`
  Neutral public cross-language baseline. Installs local config, `AGENTS.md`,
  tracker and handoff docs, and optional CI wiring. This remains the default.
- `docs-driven`
  `minimal` plus a required decision log for teams that want stronger doc
  discipline.
- `guardrails`
  Opt-in FirbLab-style doctrine profile. Adds seeded `AGENTS.md`, decision log,
  implementation invariants, and curated `docs/best-practices/` docs while
  keeping the CLI itself generic.

Discover the built-ins from the CLI with:

```bash
project-guardrails profiles list
```

If you need a different doctrine, the expected path is a custom profile rather
than widening the CLI with project-specific behavior.

## What The Tool Owns

Depending on profile and CI choice, `init` and `upgrade` can materialize:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `.guardrails/state/`
- `.pre-commit-config.yaml`
- `AGENTS.md`
- project docs under `docs/project/`
- profile docs such as `docs/best-practices/`
- GitHub or GitLab guardrails workflow wiring
- optional profile-owned assets

The installer records tool-managed files in `.guardrails/profile.lock`.
That lockfile is how `upgrade --plan` reports stale managed files and which
ones are removable versus review-only.

In practice:

- `.guardrails/state/` is durable repo-local workflow state and is preserved
  across reapply flows
- `.pre-commit-config.yaml` is repo-tracked policy, not hidden clone-local
  hook logic
- docs, `AGENTS.md`, copied assets, and local config are editable repo files
  even when the tool tracks them
- declared managed blocks inside editable files are refreshed in place instead
  of requiring whole-file ownership
- stale CI workflow wiring may be auto-removed when you switch providers
- everything else stays conservative and review-first by default

See [docs/install-ownership.md](docs/install-ownership.md) for the current
ownership rules.

## Public Distribution Trust Model

The public trust story is intentionally modest:

- pushed `v*` tags in the private GitLab source repo trigger the release
  pipeline
- that GitLab release pipeline publishes the platform archives and
  `SHA256SUMS` to the matching GitHub release
- that same GitLab release pipeline updates the public Homebrew tap formula
  from the published release artifacts
- users verify the downloaded archive against `SHA256SUMS` before extracting
  or running it

`project-guardrails` does not currently promise signing, Sigstore provenance,
or package-manager attestations.
If you need stronger assurance than the current package-and-checksum model,
inspect the tagged source and build from source with Cargo.

## Scope

`v0.1` is intentionally narrow:

- small Rust CLI
- built-in and custom profiles
- repo-local config and copied assets
- install, upgrade, status, doctor, check, pre-work, tasks, handoff,
  brief, resume, timeline, refresh, adapters, pre-commit, commit-msg-check,
  and local MCP workflows
- repo-tracked enforcement and durable repo-local state
- machine-readable JSON output with explicit schema versioning

It is intentionally not:

- a package manager
- a hosted control plane
- an agent host
- a remote profile registry
- a general plugin marketplace

## More Docs

- [docs/quick-start.md](docs/quick-start.md)
- [docs/install.md](docs/install.md)
- [docs/crates-io.md](docs/crates-io.md)
- [docs/homebrew.md](docs/homebrew.md)
- [docs/ci-provisioning.md](docs/ci-provisioning.md)
- [docs/gitlab-ci.md](docs/gitlab-ci.md)
- [docs/install-ownership.md](docs/install-ownership.md)
- [docs/llm-operating-contract.md](docs/llm-operating-contract.md)
- [docs/task-model.md](docs/task-model.md)
- [docs/enforcement-model.md](docs/enforcement-model.md)
- [docs/output-schema.md](docs/output-schema.md)
- [docs/minimal-init-snapshot.md](docs/minimal-init-snapshot.md)
- [docs/profile-authoring.md](docs/profile-authoring.md)
- [docs/custom-profile-example.md](docs/custom-profile-example.md)
- [docs/release-validation.md](docs/release-validation.md)
