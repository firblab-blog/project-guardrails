# AGENTS.md

This is an authoritative repo-local collaboration file.
Humans and LLMs should read this file before substantial work.

## First Bootstrap Pass

If this repo was just bootstrapped with `project-guardrails init`, do this
before running `doctor` or `check`:

1. inspect the current repo before rewriting anything
2. read `.guardrails/guardrails.toml`
3. keep existing repo-owned files when they already contain real content
4. replace starter placeholder guidance with repo-specific content
5. prefer updating existing docs over deleting and recreating them

On an existing repo:

- merge thoughtfully into the current `README.md`, `AGENTS.md`, and project docs
- do not overwrite user-owned files casually
- treat generated files as starter scaffolding that must be reconciled with the
  real repository

## Repo Purpose

Describe what this repository exists to do.

Keep this short, durable, repo-specific, and easy for the next human or LLM
collaborator to understand quickly.

## Read This First

Before substantial work, read:

1. `AGENTS.md`
2. `docs/project/implementation-tracker.md`
3. `docs/project/handoff-template.md`

These files are the main human/LLM-facing operating context inside the repo.
They improve consistency, but they do not guarantee perfect contributor or LLM
compliance.

## Guardrails

- state the approved implementation center
- state the main non-goals
- state what contributors should read before substantial work
- state what should never be widened casually
- call out which files are tool-managed vs repo-owned when that matters

## Workflow

- update the tracker when status changes or the approved slice changes
- leave a handoff-quality summary another contributor can continue from
- keep repo docs and code in sync
- prefer small, reviewable doc updates that explain the current repo reality
