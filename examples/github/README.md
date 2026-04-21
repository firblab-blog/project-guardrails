# GitHub Examples

[`examples/github/guardrails.example.yml`](guardrails.example.yml) is the
canonical public GitHub Actions recipe for `project-guardrails`.

It shows the shortest supported pattern for GitHub-hosted Linux runners:

- download the pinned release binary from
  `firblab-blog/project-guardrails`
- verify it against `SHA256SUMS`
- put it on `PATH`
- run `project-guardrails doctor --target .`
- run `project-guardrails check --target .`

The matching guide is
[`docs/ci-provisioning.md`](../../docs/ci-provisioning.md).
