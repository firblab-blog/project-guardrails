# Implementation Tracker

This tracker is seeded by the built-in `guardrails` profile.
Keep it current enough that a human or LLM can recover the approved slice,
current state, and next safe move without rereading the whole repo.

## Current Approved Focus

- keep the bootstrap utility portable and Rust-first
- move project doctrine into profile content, templates, and rules
- make the installed repo-local guidance specific enough to prevent drift

## Current Approved Next Steps

1. record the active slice before widening code or docs
2. update profile-facing docs in the same change when behavior moves
3. capture validation results and remaining gaps before handoff

## Current Explicit Non-Goals

- making one opinionated doctrine mandatory for every repository
- adding hosted service behavior or remote profile registry behavior
- hiding important workflow state outside the repository

## Phase Status

- bootstrap utility: active and intentionally small
- doctrine profile: opt-in and reviewable
- repo-local docs: expected to stay current with the approved slice

## Recently Validated

- built-in profiles can install a neutral baseline or an opt-in doctrine
- profile-owned docs remain reviewable repo files after bootstrap
- `doctor` and `check` remain the main mechanical validation entrypoints

## Open Questions

- which future checks belong in repo-local rules instead of the CLI
- which best-practice docs should stay generic versus repo-specific
- how later task and freshness work should build on this baseline
