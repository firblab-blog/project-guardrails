# project-guardrails

Portable repo-local guardrails for projects that want a reviewable operating
baseline, not just lint rules.

`project-guardrails` is a Rust-first bootstrap utility that installs
repo-local guardrails into an existing repository. The CLI stays small.
Project-specific opinions live in profiles, templates, and the files copied
into the target repo.

For this repository, the public GitHub-facing surface is intentionally:

- `crates.io` for Cargo-based installs
- GitHub Releases for prebuilt archive downloads
- source checkout for local inspection and developer installs
- public docs, templates, profiles, examples, and tests

Mirror/export policy stays in repo-owned docs and scripts.
It is not a runtime feature of the CLI.
For maintainers, the GitHub public repo is expected to be produced by an
allowlisted export from the private GitLab source of truth, not by a CLI
publish command.

## Start Here

The canonical first-time path is:

1. install the CLI from `crates.io`
2. run `init`
3. edit the installed repo-local files
4. run `doctor` and `check`
5. wire the same checks into CI
6. use `upgrade` later to refresh the installed baseline

If you are adopting `project-guardrails` for the first time, follow
[docs/quick-start.md](docs/quick-start.md).
That is the one primary onboarding path.

## Install Contract For v0.1

The official convenience install path for this phase is the `crates.io`
package.

Install it with:

```bash
cargo install project-guardrails --locked
```

The public fallback path is the tagged GitHub release archive for your
platform.
Use that when you do not want a Cargo-based install, then verify the archive
against `SHA256SUMS` before extracting it.

Cargo from a local checkout remains the developer path when:

- you want to inspect or build the tagged source yourself
- you are developing on `project-guardrails`

`project-guardrails` does not currently promise:

- Homebrew distribution
- curl-to-install scripts
- signed release artifacts
- a hosted service
- a plugin system
- an agent runtime

See [docs/install.md](docs/install.md) for the full install contract,
the recommended `cargo install` path, and release-archive verification
details.

## Quick Install And First Run

The recommended install path is:

```bash
cargo install project-guardrails --locked
project-guardrails --version
project-guardrails --help
```

If you prefer not to install from `crates.io`, use the matching tagged GitHub
release archive for your platform and verify it against `SHA256SUMS` before
extracting it.
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
  The smallest cross-language starting point and the expected first run.
- `--profile docs-driven`
  Choose this only when you want the `minimal` baseline plus a required
  `docs/project/decision-log.md`.
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
- `.guardrails/guardrails.toml`

If you are starting from a nearly empty repository, create or confirm
`README.md` before running `doctor` or `check`.
The default baseline treats a real top-level README as required.

Then run:

```bash
project-guardrails doctor --target .
project-guardrails check --target .
```

If your selected profile includes handoff support, also run:

```bash
project-guardrails handoff --target .
```

Then review the generated CI file and make sure your runner can actually invoke
`project-guardrails`. The generated CI baseline does not provision the binary
for you in `v0.1`.

When you want to refresh the installed baseline later, preview the change with:

```bash
project-guardrails upgrade --target . --profile minimal --ci github --plan
```

The full end-to-end walkthrough lives in
[docs/quick-start.md](docs/quick-start.md).

## Built-In Profiles

The built-in public profile set is intentionally small:

- `minimal`
  Smallest public cross-language baseline. Installs local config, `AGENTS.md`,
  tracker and handoff docs, and optional CI wiring.
- `docs-driven`
  `minimal` plus a required decision log for teams that want stronger doc
  discipline.

If you need a different doctrine, the expected path is a custom profile rather
than widening the CLI with project-specific behavior.

## What The Tool Owns

Depending on profile and CI choice, `init` and `upgrade` can materialize:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `AGENTS.md`
- project docs under `docs/project/`
- GitHub or GitLab guardrails workflow wiring
- optional profile-owned assets

The installer records tool-managed files in `.guardrails/profile.lock`.
That lockfile is how `upgrade --plan` reports stale managed files and which
ones are removable versus review-only.

In practice:

- docs, `AGENTS.md`, copied assets, and local config are editable repo files
  even when the tool tracks them
- stale CI workflow wiring may be auto-removed when you switch providers
- everything else stays conservative and review-first by default

See [docs/install-ownership.md](docs/install-ownership.md) for the current
ownership rules.

## Public Distribution Trust Model

The public trust story is intentionally modest:

- pushed `v*` tags publish the matching crate version to `crates.io`
- tagged GitHub releases publish the platform archives listed in
  [docs/install.md](docs/install.md)
- the same GitHub release publishes `SHA256SUMS`
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
- install, upgrade, status, doctor, check, and handoff workflows

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
- [docs/ci-provisioning.md](docs/ci-provisioning.md)
- [docs/gitlab-ci.md](docs/gitlab-ci.md)
- [docs/install-ownership.md](docs/install-ownership.md)
- [docs/minimal-init-snapshot.md](docs/minimal-init-snapshot.md)
- [docs/profile-authoring.md](docs/profile-authoring.md)
- [docs/custom-profile-example.md](docs/custom-profile-example.md)
- [docs/release-validation.md](docs/release-validation.md)
