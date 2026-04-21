# GitLab CI

This document publishes the one canonical GitLab CI recipe for
`project-guardrails` in `v0.1`.

For public GitLab adoption, the default recommendation is:

- keep the generated `.gitlab-ci.guardrails.yml` include minimal
- add one repo-owned `.gitlab-ci.yml`
- download the pinned release binary in a `.pre` job
- verify it against `SHA256SUMS`
- expose the binary to later jobs through artifacts and `PATH`
- include the generated guardrails file

That keeps the tool-managed file portable while making the repo-owned entry
point fully copy-pasteable.

## What `init --ci gitlab` Generates

When you run:

```bash
project-guardrails init --target . --profile minimal --ci gitlab
```

the tool installs `.gitlab-ci.guardrails.yml`.

That generated file is intentionally small.
It runs:

```bash
project-guardrails --version
project-guardrails doctor --target .
project-guardrails check --target .
```

It does not install the CLI for you.
The canonical repo-owned `.gitlab-ci.yml` below is the missing provisioning
layer public users should copy.

## Canonical Recipe

Keep the generated `.gitlab-ci.guardrails.yml` file, and add this root
`.gitlab-ci.yml`.
Change only the pinned version when you adopt a newer release.

The same file also lives at
[`examples/gitlab/.gitlab-ci.example.yml`](../examples/gitlab/.gitlab-ci.example.yml).

```yaml
default:
  image: debian:bookworm-slim
  before_script:
    - export PATH="$CI_PROJECT_DIR/.project-guardrails-root/bin:$PATH"

variables:
  PROJECT_GUARDRAILS_VERSION: "0.1.12"
  PROJECT_GUARDRAILS_REPO: "firblab-blog/project-guardrails"

include:
  - local: .gitlab-ci.guardrails.yml

guardrails:provision:
  stage: .pre
  script:
    - apt-get update -qq
    - apt-get install -yqq --no-install-recommends ca-certificates curl tar gzip grep coreutils
    - export TARGET="x86_64-unknown-linux-gnu"
    - export VERSION="$PROJECT_GUARDRAILS_VERSION"
    - export REPO="$PROJECT_GUARDRAILS_REPO"
    - export ARCHIVE="project-guardrails-v${VERSION}-${TARGET}.tar.gz"
    - export BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
    - mkdir -p "$CI_PROJECT_DIR/.project-guardrails-root/bin"
    - curl -fsSLo "$ARCHIVE" "${BASE_URL}/${ARCHIVE}"
    - curl -fsSLo SHA256SUMS "${BASE_URL}/SHA256SUMS"
    - grep " ${ARCHIVE}\$" SHA256SUMS > SHA256SUMS.current
    - sha256sum -c SHA256SUMS.current
    - tar -xzf "$ARCHIVE"
    - install -m 0755 project-guardrails "$CI_PROJECT_DIR/.project-guardrails-root/bin/project-guardrails"
    - '"$CI_PROJECT_DIR/.project-guardrails-root/bin/project-guardrails" --version'
  artifacts:
    paths:
      - .project-guardrails-root/
    expire_in: 1 day
```

This recipe intentionally assumes a Linux x86_64 Docker runner so the asset
name stays stable and the snippet stays short.
If your GitLab runners use another platform, keep the same pattern and swap
only the release asset name to the matching platform from
[`docs/install.md`](install.md).

## When To Use Each Provisioning Option

Use release binary install when:

- you want the shortest public GitLab recipe
- your runner platform matches a published release asset
- you want to avoid provisioning Rust and Cargo in GitLab just for guardrails

Use a repo-owned provisioning path instead when:

- you control the runner image
- you want the fastest steady-state pipeline
- your image already places `project-guardrails` on `PATH`

Use a preinstalled runner image when:

- no published release asset exists for your runner platform
- you already control the runner image or bootstrap layer
- you want `project-guardrails` available on `PATH` before the pipeline starts

## Why This Is The Canonical GitLab Pattern

This is the narrowest honest public GitLab story for `v0.1` because it:

- keeps the generated `.gitlab-ci.guardrails.yml` file minimal and tool-managed
- makes the root `.gitlab-ci.yml` the only repo-owned provisioning layer
- uses the real public repo URL
- uses the published tagged-release assets the repo already ships
- verifies downloads against `SHA256SUMS`
- keeps preinstalled images and Cargo explicit as alternatives instead of
  competing defaults

## Related Examples

- canonical adoption example:
  [`examples/gitlab/.gitlab-ci.example.yml`](../examples/gitlab/.gitlab-ci.example.yml)
- richer repo-maintainer example:
  [`examples/gitlab/.gitlab-ci.source-of-truth.example.yml`](../examples/gitlab/.gitlab-ci.source-of-truth.example.yml)
