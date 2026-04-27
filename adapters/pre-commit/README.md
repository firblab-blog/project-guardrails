# Pre-Commit Adapter

`project-guardrails` now treats repo-tracked `pre-commit` configuration as the
primary shared commit-enforcement surface.

The intended contract is:

1. `init` writes `.pre-commit-config.yaml` into the target repo
2. that file is reviewed and versioned with the repo
3. each clone runs the local adapter step:

```bash
pre-commit install --hook-type pre-commit --hook-type commit-msg
```

The adapter then shells out to:

- `project-guardrails pre-commit`
- `project-guardrails commit-msg-check`

This keeps the enforcement behavior readable in the repository while still
using the developer's local Git hook installation as a thin execution bridge.
