# CI And Enforcement

This profile treats enforcement as layered and reviewable.
Keep the primary source of truth in repo-owned files before adding more runtime
logic.

## Principles

- start with repo-local docs and explicit workflow expectations
- run `project-guardrails doctor` and `project-guardrails check` locally before
  wiring the same commands into CI
- prefer repo-tracked CI definitions and rules over hidden machine-local setup
- keep enforcement understandable by reading the repository

## Practical Guidance

- generated CI files are a baseline, not the full policy story
- when CI setup becomes project-specific, express it in profile templates or
  repo-owned workflow files
- treat local and CI validation as complements: local checks catch drift early,
  CI keeps the shared branch honest
- document any external tool requirement in repo-local docs before making it a
  hard dependency
