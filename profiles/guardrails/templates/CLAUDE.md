# CLAUDE.md

<!-- guardrails:managed start id=adapter-context generator=repo_context_v1 -->
## Managed Repo Context

This block is tool-managed. It strengthens proxy enforcement and freshness signals, but it does not prove that a human or LLM read or understood the repository.

### Required Context Paths
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`

### Active Tasks
- no active repo-local tasks are recorded under `.guardrails/state/tasks/`

### Open Handoffs
- no open handoffs are recorded under `.guardrails/state/handoffs/`
<!-- guardrails:managed end id=adapter-context -->

## Claude Code Guidance

Treat `AGENTS.md` as the canonical repo guidance. This file is only the
Claude-specific entry point and should stay concise.

Before substantial work, read:

- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- relevant task records under `.guardrails/state/tasks/`
- recent handoffs under `.guardrails/state/handoffs/`

Useful guardrails commands:

- `project-guardrails brief --target .`
- `project-guardrails resume --target .`
- `project-guardrails doctor --target .`
- `project-guardrails check --target .`
- `project-guardrails refresh --target . --check`

Keep host-specific notes here. Put project doctrine, task state, and durable
handoff context in the canonical repo-local files instead of duplicating them.
