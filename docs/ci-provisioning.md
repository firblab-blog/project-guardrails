# GitHub Actions CI

This document publishes the one canonical GitHub Actions recipe for
`project-guardrails` in `v0.1`.

For public GitHub-hosted runners, the default recommendation is:

- download the pinned release binary from the public GitHub release
- verify it against `SHA256SUMS`
- put it on `PATH`
- run `project-guardrails doctor --target .`
- run `project-guardrails check --target .`

That keeps the happy path short and avoids requiring Rust or Cargo in CI just
to run the guardrails checks.

The generated `.github/workflows/guardrails.yml` file remains the minimal
tool-managed baseline.
For GitHub Actions, the canonical public recipe below is the repo-owned
workflow you should copy into your repository when the runner does not already
have `project-guardrails` installed.

## Canonical Recipe

Copy this into `.github/workflows/guardrails.yml`, then change only the pinned
version when you adopt a newer release.

The same file also lives at
[`examples/github/guardrails.example.yml`](../examples/github/guardrails.example.yml).

```yaml
name: guardrails

on:
  pull_request:
  push:

permissions:
  contents: read

env:
  PROJECT_GUARDRAILS_VERSION: "0.1.7"
  PROJECT_GUARDRAILS_REPO: "firblab-blog/project-guardrails"

jobs:
  guardrails:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install pinned project-guardrails release binary
        shell: bash
        run: |
          set -euo pipefail
          target="x86_64-unknown-linux-gnu"
          version="${PROJECT_GUARDRAILS_VERSION}"
          repo="${PROJECT_GUARDRAILS_REPO}"
          archive="project-guardrails-v${version}-${target}.tar.gz"
          base_url="https://github.com/${repo}/releases/download/v${version}"

          mkdir -p "$HOME/.local/bin"
          curl -fsSLO "${base_url}/${archive}"
          curl -fsSLO "${base_url}/SHA256SUMS"
          grep " ${archive}\$" SHA256SUMS > SHA256SUMS.current
          sha256sum -c SHA256SUMS.current
          tar -xzf "${archive}"
          install -m 0755 project-guardrails "$HOME/.local/bin/project-guardrails"
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"

      - name: Verify installed CLI
        run: project-guardrails --version

      - name: Validate guardrails contract with doctor
        run: project-guardrails doctor --target .

      - name: Enforce configured checks
        run: project-guardrails check --target .
```

This recipe intentionally assumes `ubuntu-latest` so the asset name stays
stable and copy-pasteable.
If your workflow runs on another OS or architecture, keep the same pattern and
swap only the release asset name to the matching platform from
[`docs/install.md`](install.md).

## When To Use Each Provisioning Option

Use release binary install when:

- you are on GitHub-hosted Linux runners
- you want the smallest public recipe with no Rust toolchain setup
- you want the workflow to match the public GitHub Releases contract exactly

Use a repo-owned provisioning path instead when:

- you control self-hosted runners or a custom image
- you want faster builds and fewer network calls
- your image already places `project-guardrails` on `PATH`

Use a preinstalled runner image when:

- you already control the runner image or bootstrap layer
- no published release asset exists for your runner platform
- you want `project-guardrails` available on `PATH` before the job starts

## Relationship To `init --ci github`

When you run:

```bash
project-guardrails init --target . --profile minimal --ci github
```

the tool writes a minimal `.github/workflows/guardrails.yml` that only checks a
runner where `project-guardrails` is already available.

GitHub Actions does not have the same local-include pattern GitLab uses, so the
practical public move is:

1. run `init --ci github`
2. replace the generated workflow with the canonical repo-owned workflow above
3. keep the same pinned version policy during future upgrades

If you later run `project-guardrails upgrade --apply`, review the workflow diff
carefully because your repo-owned provisioning logic may need to be re-applied.

## Why This Is The Canonical GitHub Pattern

This is the narrowest honest public GitHub Actions story for `v0.1` because it:

- uses the real public repo URL
- uses the published tagged-release assets that the repo already ships
- verifies downloads against `SHA256SUMS`
- avoids making Rust toolchain setup part of the default CI adoption path
- keeps alternative provisioning strategies explicit but secondary
