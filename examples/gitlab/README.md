# GitLab Examples

This directory now has one canonical public adoption example and one richer
repo-maintainer example.

## Start With This File

[`examples/gitlab/.gitlab-ci.example.yml`](.gitlab-ci.example.yml) is the
canonical public GitLab recipe.

It shows the shortest supported pattern:

- keep the generated `.gitlab-ci.guardrails.yml` include
- provision a pinned `project-guardrails` release binary in `.pre`
- verify it against `SHA256SUMS`
- expose it to later jobs through `PATH` and artifacts

That example uses the real public repo URL:
`firblab-blog/project-guardrails`.

The matching guide is
[`docs/gitlab-ci.md`](../../docs/gitlab-ci.md).

## Richer Example

[`examples/gitlab/.gitlab-ci.source-of-truth.example.yml`](.gitlab-ci.source-of-truth.example.yml)
is intentionally broader.

It shows one repo-maintainer pattern for this repository family where GitLab is
the private source of truth and later stages handle quality, security, release,
and optional mirroring.

Treat that file as an adaptation example, not the first file a new public user
should copy.

## What Stays Minimal

The generated template in
`templates/gitlab/.gitlab-ci.guardrails.yml` stays intentionally small:

- verify `project-guardrails` is installed
- run `project-guardrails doctor --target .`
- run `project-guardrails check --target .`

That boundary is deliberate.
Provisioning and broader pipeline design stay repo-owned.
