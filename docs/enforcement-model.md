# Enforcement Model

`project-guardrails` installs repo-local guardrails, but it does not promise
full semantic enforcement of how a team works.

The honest V0 model is:

- some guardrails are instructional
- some guardrails are enforceable
- the bootstrap utility should only enforce signals that stay portable,
  explainable, and machine-checkable

That keeps the CLI small and keeps project-specific doctrine in profiles and
installed repo-local files instead of compiled-in runtime logic.

## Instructional Guardrails

Instructional guardrails are the files and conventions that tell humans and
LLMs how a repo wants work to happen.

Examples:

- `AGENTS.md`
- `docs/project/implementation-tracker.md`
- `docs/project/handoff-template.md`
- profile-owned docs and examples

These files matter even when the tool cannot prove that a contributor followed
them perfectly.

The tool's role is to install them, keep ownership legible, and surface obvious
signals when the repo never moved past stock starter content.

## Enforceable Guardrails

Enforceable guardrails are the checks the bootstrap utility can apply locally
and in CI without pretending to understand the whole project.

Current V0 enforceable signals include:

- repo-root markers configured in `.guardrails/guardrails.toml`
- required files existing
- required docs existing
- required text docs not being empty
- stock starter content still being present in key installed docs such as
  `AGENTS.md`, the implementation tracker, and the handoff template
- forbidden directories configured by the selected profile
- expected CI workflow files existing
- enabled external engines being runnable and pointed at configured paths

These are intentionally narrow checks.
They answer "is the installed contract present and plausibly owned by this
repo?" rather than "is every document semantically complete?"

## What `doctor` And `check` Mean

`doctor` is the preflight contract.
It reports missing or clearly invalid local prerequisites, including required
docs, starter-content placeholders, configured CI files, and external engine
misconfiguration.

`check` is the enforceable local validation path.
It fails on the same machine-checkable guardrails and then runs any enabled
external engines.

In generated CI wiring, these commands are the intended public enforcement
surface.

## Placeholder Detection

Placeholder detection is deliberately conservative.

The tool only flags stock starter content when it sees explicit markers from
the installed templates, such as unreplaced "describe this repository" or
"replace this line" guidance.

The goal is not to score writing quality.
The goal is to catch the common case where a repo copied in the starter docs
but never turned them into repo-specific instructions.

## Non-Goals

This model does not promise:

- semantic validation of architecture decisions
- proof that humans or LLMs actually read the instructions
- deep content scoring for project docs
- a broad policy engine embedded in the CLI

If a team needs stronger or more opinionated checks, that logic should usually
live in profile-owned files, external engines, or future profile-level
extensions rather than widening the bootstrap runtime by default.
