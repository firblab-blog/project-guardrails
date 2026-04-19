# Release Validation

This document is the maintainer checklist for validating one tagged public
GitHub release end to end.

It is intentionally narrow:

- confirm the release workflow published the documented `crates.io` package and
  GitHub release assets
- confirm the checksum manifest matches those assets
- confirm the release page, README, and install docs all describe the same
  `crates.io`-first install story
- confirm the public trust language stays honest about what is and is not
  provided

## Expected Release Assets

For a tag `vVERSION`, the workflow should publish crate version `VERSION` to
`crates.io`.

The same GitHub release should publish exactly these assets:

- `project-guardrails-vVERSION-x86_64-unknown-linux-gnu.tar.gz`
- `project-guardrails-vVERSION-x86_64-apple-darwin.tar.gz`
- `project-guardrails-vVERSION-aarch64-apple-darwin.tar.gz`
- `project-guardrails-vVERSION-x86_64-pc-windows-msvc.zip`
- `SHA256SUMS`

`SHA256SUMS` should contain one line for each archive above.

## Release Validation Checklist

After pushing a `v*` tag and letting
[release.yml](../.github/workflows/release.yml)
finish:

1. Open the GitHub release page for that tag.
2. Confirm `crates.io` lists `project-guardrails` version `VERSION`.
3. Confirm the four platform archives and `SHA256SUMS` are attached.
4. Confirm the asset names match [docs/install.md](install.md) exactly.
5. Confirm the release page recommends
   `cargo install project-guardrails --locked --version VERSION`.
6. Confirm the release page tells users to verify downloads against
   `SHA256SUMS` before extraction.
7. Confirm the release page does not claim signing, provenance, or other trust
   features that the repo does not actually provide.
8. Download each archive plus `SHA256SUMS`.
9. Verify each download against `SHA256SUMS`.
10. Extract each archive on the intended platform and run
   `project-guardrails --version`.
11. Confirm `README.md`, `docs/install.md`, and `docs/quick-start.md` all
    describe `crates.io` as the recommended convenience install path, with
    tagged release archives as fallback.
12. Record the validation result in the release notes, a tracked issue, or a
    release follow-up note.

## Verification Commands

On macOS, verify the single archive you downloaded:

```bash
grep ' project-guardrails-vVERSION-x86_64-apple-darwin.tar.gz$' SHA256SUMS > SHA256SUMS.current
shasum -a 256 -c SHA256SUMS.current
```

On Linux, verify the single archive you downloaded:

```bash
grep ' project-guardrails-vVERSION-x86_64-unknown-linux-gnu.tar.gz$' SHA256SUMS > SHA256SUMS.current
sha256sum -c SHA256SUMS.current
```

On Windows PowerShell:

```powershell
$Pattern = [regex]::Escape('project-guardrails-vVERSION-x86_64-pc-windows-msvc.zip') + '$'
$Expected = ((Select-String $Pattern .\SHA256SUMS).Line -split '\s+')[0]
$Actual = (Get-FileHash .\project-guardrails-vVERSION-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash.ToLower()
if ($Actual -ne $Expected.ToLower()) { throw "checksum mismatch" }
```

After checksum verification, smoke-test the extracted binary:

```bash
project-guardrails --version
```

## Current Public Trust Contract

The release trust contract for `v0.1` is:

- GitHub Actions publishes crate version `VERSION` to `crates.io` from a pushed
  `vVERSION` tag
- the same workflow builds the GitHub release archives from that tag
- the same workflow publishes `SHA256SUMS`
- users verify any archive they downloaded against `SHA256SUMS`

The release trust contract for `v0.1` does not include:

- signed release artifacts
- Sigstore or cosign attestations
- package-manager provenance

If release notes, docs, or workflow behavior drift from that contract, treat it
as a release issue.

## What Still Requires A Real Tagged Release

This repository can prepare the workflow and docs locally, but only a real
public tag can prove:

- the crate really publishes on `crates.io` with the expected metadata
- the GitHub Actions workflow uploads all expected assets on the live release
  page
- the generated release body matches the install contract and trust language
- the published archives can be downloaded and verified by an external user
- the final asset names and checksum manifest are correct in the real release
