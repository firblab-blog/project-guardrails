# Minimal Init Snapshot

This example captures the documented default bootstrap path from a consumer
repo's point of view.

Command used:

```bash
project-guardrails init --target . --profile minimal --ci github
```

Snapshot location:

- [`examples/minimal-init-snapshot/`](../examples/minimal-init-snapshot)

Included files:

- `README.md`
- `AGENTS.md`
- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- `.github/workflows/guardrails.yml`

The snapshot is intentionally a fresh post-`init` state.
It shows the default files a new consumer repo receives before replacing the
starter content with repo-specific instructions and project docs.
