# AGENTS.md

This repository uses the built-in `guardrails` profile.
Treat this file as the repo-local operating contract for substantial work.

## Work In Layers

Always distinguish three things before you change anything:

1. bootstrap utility behavior in the `project-guardrails` CLI
2. profile behavior shipped by a selected profile
3. installed repo-local assets and docs that the target repo owns after bootstrap

If a change can live in profile templates, profile docs, or repo-local rules,
prefer that over widening generic Rust behavior.

## Read Before Substantial Work

Read these files before code changes that affect behavior, process, or
enforcement:

1. `AGENTS.md`
2. `docs/project/implementation-tracker.md`
3. `docs/project/decision-log.md`
4. `docs/project/implementation-invariants.md`
5. `docs/best-practices/repo-shaping.md`
6. `docs/best-practices/change-safety.md`
7. `docs/best-practices/docs-and-handoffs.md`
8. `docs/best-practices/ci-and-enforcement.md`

If the current task is narrow, keep the reading narrow too, but do not skip the
tracker, decision log, and the best-practice document most relevant to the
change.

## Operating Doctrine

- keep the bootstrap utility portable, reviewable, and small
- keep FirbLab-style doctrine in profile content, not hardcoded runtime branches
- preserve neutral built-in behavior for users who stay on `minimal`
- prefer explicit repo-owned docs over hidden workflow assumptions
- make validation and handoffs specific enough that another contributor can
  continue without guesswork

## Change Rules

- do not make the opinionated doctrine profile the default bootstrap path
- do not assume one repository layout, one CI provider, or one language stack
- do not widen the CLI when a profile template, required doc, or copied rule
  can express the same policy
- update repo-facing docs in the same change when workflow expectations shift
- prefer small, reviewable increments with clear validation notes

## Working Loop

1. Confirm the approved slice in `docs/project/implementation-tracker.md`.
2. Check the decision log and invariants before changing behavior.
3. Make the smallest change that keeps bootstrap logic generic.
4. Update docs or profile content when the doctrine changed.
5. Run the relevant validation and record what was and was not checked.
6. Leave a handoff-quality summary in `docs/project/handoff-template.md` when
   the change is substantial or incomplete.

## Done Looks Like

A change is ready when:

- the code change matches the approved slice
- profile-facing docs still describe the actual repo behavior
- validation was run or the gap is called out explicitly
- the next contributor can tell what changed, what remains, and what must not
  be widened casually
