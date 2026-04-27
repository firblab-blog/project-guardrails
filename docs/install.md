# Install

This document defines the public `v0.1` install contract for
`project-guardrails`.

## Start Here

For this phase, the recommended public install path is:

```bash
brew install firblab-blog/tap/project-guardrails
```

If you already use Cargo, the secondary install path is:

```bash
cargo install project-guardrails --locked
```

If you do not want a Rust toolchain on the machine, use the tagged GitHub
release archive that matches your platform, verify it against `SHA256SUMS`,
extract it, put the binary on `PATH`, then confirm the binary.

Installing from a local checkout remains the developer path.
Use a local checkout when:

- you want to build directly from the tagged source yourself
- you are developing on `project-guardrails`

What to do next after install:

1. switch to the repository you want to bootstrap
2. run `project-guardrails init --target . --profile minimal --ci github`
3. follow [docs/quick-start.md](quick-start.md) for the rest of the first-run
   flow

## What v0.1 Publicly Promises

`project-guardrails` currently documents and supports:

- installing from Homebrew with `brew install firblab-blog/tap/project-guardrails`
- installing from `crates.io` with `cargo install project-guardrails --locked`
- downloading a prebuilt binary from a tagged GitHub release for the published
  platform matrix
- publishing `SHA256SUMS` alongside those release archives
- verifying the downloaded archive against `SHA256SUMS` before extraction
- installing from a local checkout with `cargo install --path .` as a
  developer path
- running directly from a checkout with `cargo run -- ...` as a developer or
  advanced path

`project-guardrails` does not currently document or promise:

- curl-to-install scripts
- hosted services
- signed release artifacts
- plugin or orchestration systems

That narrower contract keeps the docs aligned with the current codebase and
current trust model.

## Recommended Path: Homebrew

For machines that already use Homebrew:

```bash
brew install firblab-blog/tap/project-guardrails
project-guardrails --version
```

Use the `crates.io` path below when you already use Cargo but do not want the
Homebrew route.

## Secondary Path: `crates.io`

For machines that already have Cargo available:

```bash
cargo install project-guardrails --locked
project-guardrails --version
```

Use the archive path below when you do not want Homebrew or Cargo on the
target machine or when you want to install from the exact published release
asset set.

## Fallback Path: Tagged Release Archives

Choose the tagged GitHub release archive that matches your platform, then
verify it against `SHA256SUMS` before extraction.

## Release Assets

Current documented release targets:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

For a tag `vVERSION`, the expected release assets are:

- `project-guardrails-vVERSION-x86_64-unknown-linux-gnu.tar.gz`
- `project-guardrails-vVERSION-x86_64-apple-darwin.tar.gz`
- `project-guardrails-vVERSION-aarch64-apple-darwin.tar.gz`
- `project-guardrails-vVERSION-x86_64-pc-windows-msvc.zip`
- `SHA256SUMS`

If a tagged release does not include an asset for your platform, use the
`crates.io` path when Cargo is available. Otherwise, use the developer path
from a checkout instead.

## Verify Before You Extract

Before extracting a prebuilt archive, always download both:

- the archive for your platform
- `SHA256SUMS` from the same tagged release

If the checksum does not match, do not extract or run the archive.
Download it again from the tagged release or build from source from the tagged
commit.

## Copy-Paste Release Install Snippets

Replace `VERSION` below with the tagged release you want to install, such as
`0.2.0`.

The commands below assume the public GitHub repository is:
`firblab-blog/project-guardrails`.

### macOS

This snippet auto-selects Intel vs Apple silicon.

```bash
VERSION="VERSION"
REPO="firblab-blog/project-guardrails"
ARCH="$(uname -m)"
if [ "$ARCH" = "arm64" ]; then
  TARGET="aarch64-apple-darwin"
else
  TARGET="x86_64-apple-darwin"
fi
ARCHIVE="project-guardrails-v${VERSION}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
curl -LO "$BASE_URL/$ARCHIVE"
curl -LO "$BASE_URL/SHA256SUMS"
grep " ${ARCHIVE}\$" SHA256SUMS > SHA256SUMS.current
shasum -a 256 -c SHA256SUMS.current
tar -xzf "$ARCHIVE"
mkdir -p "$HOME/.local/bin"
install -m 0755 project-guardrails "$HOME/.local/bin/project-guardrails"
project-guardrails --version
```

Make sure `$HOME/.local/bin` is on your `PATH`.

### Linux

Current release binaries document only `x86_64-unknown-linux-gnu`.

```bash
VERSION="VERSION"
REPO="firblab-blog/project-guardrails"
TARGET="x86_64-unknown-linux-gnu"
ARCHIVE="project-guardrails-v${VERSION}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
curl -LO "$BASE_URL/$ARCHIVE"
curl -LO "$BASE_URL/SHA256SUMS"
grep " ${ARCHIVE}\$" SHA256SUMS > SHA256SUMS.current
sha256sum -c SHA256SUMS.current
tar -xzf "$ARCHIVE"
install -m 0755 project-guardrails "$HOME/.local/bin/project-guardrails"
project-guardrails --version
```

Make sure `$HOME/.local/bin` is on your `PATH`.

### Windows PowerShell

```powershell
$Version = "VERSION"
$Repo = "firblab-blog/project-guardrails"
$Target = "x86_64-pc-windows-msvc"
$Archive = "project-guardrails-v$Version-$Target.zip"
$BaseUrl = "https://github.com/$Repo/releases/download/v$Version"
Invoke-WebRequest -Uri "$BaseUrl/$Archive" -OutFile $Archive
Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS" -OutFile "SHA256SUMS"
$Pattern = [regex]::Escape($Archive) + '$'
$Expected = ((Select-String $Pattern .\SHA256SUMS).Line -split '\s+')[0]
$Actual = (Get-FileHash .\$Archive -Algorithm SHA256).Hash.ToLower()
if ($Actual -ne $Expected.ToLower()) { throw "checksum mismatch" }
Expand-Archive -Path .\$Archive -DestinationPath .\project-guardrails-install -Force
New-Item -ItemType Directory -Force -Path "$HOME\bin" | Out-Null
Copy-Item .\project-guardrails-install\project-guardrails.exe "$HOME\bin\project-guardrails.exe" -Force
& "$HOME\bin\project-guardrails.exe" --version
```

Make sure the directory you choose, such as `$HOME\bin`, is on your `PATH`.

## Developer Path: Build From A Checkout

If you want to inspect or build from source, install from a checkout of this
repository:

```bash
cargo install --path .
project-guardrails --version
project-guardrails --help
```

Expected top-level commands in `v0.1`:

- `init`
- `upgrade`
- `status`
- `doctor`
- `check`
- `handoff`

If the binary is not found after install, make sure your Cargo bin directory is
on `PATH`.

## First Run After Install

Inside the repository you want to bootstrap:

```bash
project-guardrails init --target . --profile minimal --ci github
project-guardrails doctor --target .
project-guardrails check --target .
```

If your repo is GitLab-first, use `--ci gitlab` instead.

If the repository is nearly empty, create or confirm `README.md` before
running `doctor` or `check`.

After `init`, edit the installed starter files so they become your repo's real
instructions and workflow docs.

For the complete first-run sequence, see [docs/quick-start.md](quick-start.md).

## CI Provisioning Contract

The generated CI templates for `v0.1` do not install `project-guardrails` for
you.

They assume the runner already has:

- `project-guardrails` on `PATH`
- any prerequisites needed by your chosen provisioning method

That is the smallest honest portable story this repo can support today.

For public CI adoption, the canonical documented recipes use a pinned GitHub
release binary plus `SHA256SUMS` verification.
That keeps the default CI path short and avoids requiring Rust and Cargo just
to run guardrails.

Choose the provisioning method like this:

- release binary install
  The default public CI choice when your runner platform matches a published
  release asset.
- preinstalled runner image
  Prefer this on self-hosted runners or custom images you already control.
- Cargo install fallback
  Use this when no release asset exists for your runner platform, or when your
  CI image already includes Rust and Cargo for other jobs.

Reference examples:

- GitHub Actions: [docs/ci-provisioning.md](ci-provisioning.md)
- GitLab CI: [docs/gitlab-ci.md](gitlab-ci.md)

## Advanced Alternative: Run From A Checkout

If you do not want to install the binary into your normal bin directory, you
can run directly from a checkout:

```bash
cargo run -- init --target . --profile minimal --ci github
cargo run -- doctor --target .
cargo run -- check --target .
```

This is mainly useful for local development on `project-guardrails` itself.
For public users, the primary documented path is the tagged GitHub release
archive.

## Public Distribution Trust Story

The current public trust model is intentionally modest and explicit:

- a pushed `v*` tag publishes release archives and `SHA256SUMS`
- the same GitLab tag pipeline publishes the crate to `crates.io`
- the same GitLab tag pipeline updates the public Homebrew tap formula
- that workflow publishes a `SHA256SUMS` manifest alongside the archives
- users should verify any downloaded archive against `SHA256SUMS` before
  extracting it

`project-guardrails` does not currently promise:

- signed release artifacts
- Sigstore, cosign, or other external attestation systems
- package-manager provenance

If you need stronger assurance than the current package-and-checksum model,
inspect the tagged source and build from source with Cargo.

Maintainers validating a tagged release should use
[docs/release-validation.md](release-validation.md).
