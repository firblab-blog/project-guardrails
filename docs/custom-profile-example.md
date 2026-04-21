# Worked Custom Profile Examples

This repo ships two built-in public profiles:

- `minimal`
- `docs-driven`

If a project wants different repo-local doctrine, the intended path is a custom
profile directory, not a Rust fork.

## Example Directories

See:

- [`examples/profiles/README.md`](../examples/profiles/README.md)
- [`examples/profiles/team-ops/profile.toml`](../examples/profiles/team-ops/profile.toml)
- [`examples/profiles/team-ops/templates`](../examples/profiles/team-ops/templates)
- [`examples/profiles/team-ops/assets`](../examples/profiles/team-ops/assets)
- [`examples/profiles/human-only/profile.toml`](../examples/profiles/human-only/profile.toml)
- [`examples/profiles/human-plus-llm/profile.toml`](../examples/profiles/human-plus-llm/profile.toml)

The examples stay language-agnostic.
`team-ops` demonstrates all three extension surfaces:

- profile metadata in `profile.toml`
- template overrides in `templates/`
- additional repo-local support files in `assets/`

The `human-only` and `human-plus-llm` examples are intentionally smaller.
They show that collaboration style can be expressed by profile-owned template
wording without becoming a new built-in runtime mode.

## What It Changes

Compared with the built-in profiles, `team-ops`:

- keeps the standard tracker and handoff flow
- adds a custom required doc: `docs/project/release-checklist.md`
- overrides `AGENTS.md` with team-specific collaboration prompts
- ships an extra support asset at `docs/project/change-rubric.md`
- defaults CI to GitLab without changing Rust code

Compared with the shared templates, `human-only` and `human-plus-llm`:

- keep the standard tracker and handoff flow
- only override `AGENTS.md`
- show different collaboration wording through profile templates instead of new
  CLI behavior

## Try It

From inside a target repo:

```bash
cargo run -- init \
  --target . \
  --profile team-ops \
  --profile-path /path/to/project-guardrails/examples/profiles/team-ops \
  --ci gitlab
```

Expected installed files include:

- `.guardrails/guardrails.toml`
- `.guardrails/profile.lock`
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- `docs/project/release-checklist.md`
- `docs/project/change-rubric.md`
- `.gitlab-ci.guardrails.yml`

## Why This Example Exists

The point is to prove the public extension path:

- add or remove required docs
- replace starter templates
- ship extra repo-local assets
- choose a different default CI
- adjust collaboration wording for different team styles

All of that should happen through profile contents, not through new hardcoded
branches in the CLI.
