# AGENTS.md

<!-- guardrails:managed start id=repo-context generator=repo_context_v1 -->
## Managed Repo Context

This block is tool-managed. It strengthens proxy enforcement and freshness signals, but it does not prove that a human or LLM read or understood the repository.

### Required Context Paths
- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`

### Active Tasks
- no active repo-local tasks are recorded under `.guardrails/state/tasks/`

### Open Handoffs
- `0001` Self-adoption bootstrap handoff (1 task link(s), updated 2026-04-25T02:32:10Z)
<!-- guardrails:managed end id=repo-context -->

## Repo Purpose

`project-guardrails` is a Rust-first reusable tool for bootstrapping and
maintaining repo-local operating guardrails.

The product is not a single profile or doctrine.
The product is the portable bootstrap utility plus configurable profiles and
repo-local installed assets.

## Core Boundaries

Always distinguish:

1. bootstrap utility behavior
2. profile behavior
3. installed repo-local assets

Important rules:

- do not hardcode the `cognitive-control-plane` worldview into the bootstrap
  utility
- do not assume one repository layout
- do not assume one CI provider
- do not assume one programming language stack for consumer repos
- keep project-specific opinions in profiles, templates, and rules

## V0 Guardrails

For the current V0:

- keep the implementation Rust-first
- keep the CLI small and legible
- prefer declarative config over compiled-in branching
- prefer filesystem portability over clever install-time magic
- make the bootstrap flow understandable by reading the repo

## Current Non-Goals

- hosted service behavior
- agent orchestration
- remote profile registry
- plugin marketplace
- large-scale rule evaluation framework
- npm/npx distribution

## Implementation Posture

Before widening the implementation, check:

1. is this bootstrap utility behavior or profile behavior?
2. does this belong in the CLI, or in generated repo-local assets?
3. does the change preserve portability across different project layouts?
4. can the same result be expressed declaratively in `.guardrails/guardrails.toml`?

If the answer is unclear, preserve the smaller runtime and move the opinion into
a profile.
