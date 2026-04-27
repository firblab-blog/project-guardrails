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
- declared managed blocks being present, parseable, and current with generated
  task/handoff context
- forbidden directories configured by the selected profile
- expected CI workflow files existing
- enabled external engines being runnable and pointed at configured paths

Rule packs do not add another evaluator to this model. A profile may declare
named packs, and repo-local config may enable those pack names, but expansion
only appends to the enforceable surfaces above: required docs, required files,
diff link requirements, forbidden patterns, Semgrep config paths, and Conftest
policy paths.

The current commit-time surface also includes:

- repo-tracked `.pre-commit-config.yaml` as the primary shared enforcement
  entrypoint
- `project-guardrails pre-commit` for staged diff checks
- `project-guardrails commit-msg-check` for task-reference validation
- diff-based link requirements between changed paths and required docs
- configurable evidence requirements coupling changed or deleted paths to
  staged repo-local evidence files, added evidence lines, or replacement paths
- forbidden-pattern checks against added diff lines
- starter-content hard failures when a staged file still contains stock markers

These are intentionally narrow checks.
They answer "is the installed contract present and plausibly owned by this
repo?" rather than "is every document semantically complete?"

## Shipped Starter Pack Enforcement

The built-in `guardrails` profile ships practical starter packs through profile
configuration. They are opt-in by profile selection; the built-in `minimal`
profile remains neutral and does not enable them by default.

The `llm-common-mistakes` pack contributes forbidden added-line patterns for:

- AI boilerplate such as "as an AI" or "as a language model"
- unfinished placeholder TODOs such as "TODO: implement this later"
- explicit temporary markers such as `REMOVE_BEFORE_MERGE` and `DO_NOT_COMMIT`
- browser debug statements such as `console.log(` or `debugger;`
- filler implementation text such as "lorem ipsum" or "stub implementation"

The `docs-freshness` pack contributes a staged diff link requirement. Changes
under code, test, profile, template, CI, or agent-context paths must stage one
of these repo-local docs in the same commit:

- `docs/project/implementation-tracker.md`
- `docs/project/decision-log.md`
- `docs/project/implementation-invariants.md`

This is a proxy signal. It shows that a companion doc moved with the change; it
does not prove the doc is semantically complete.

The same pack also contributes narrower staged evidence requirements:

- source paths can require implementation-tracker or task evidence
- configured public API or command/config paths can require decision-log
  evidence
- configured dependency manifest paths can require rationale wording in staged
  task notes or project docs
- configured infra paths can require rollback or validation wording in staged
  task notes or project docs
- configured deleted test paths can require replacement test paths or staged
  task/tracker notes explaining the deletion

These checks still only inspect path names, Git staged status, and added lines
in configured evidence files. They do not infer what counts as a public API,
which dependency managers a repo uses, which CI provider owns infra, or whether
the evidence is sufficient for human review. Profiles choose the paths and
wording patterns.

The `secret-safety` pack contributes forbidden added-line patterns for obvious
AWS access keys, GitHub tokens, private-key headers, and inline assignments to
names like `password`, `api_key`, `secret`, or `token`.

Each failure message includes profile-owned remediation guidance pointing back
to repo-local docs, removing draft artifacts, rotating exposed secrets, or using
the repo's approved secret storage. The runtime still only reports the matched
mechanical signal.

## What `doctor` And `check` Mean

`doctor` is the preflight contract.
It reports missing or clearly invalid local prerequisites, including required
docs, starter-content placeholders, configured CI files, and external engine
misconfiguration.

`check` is the enforceable local validation path.
It fails on the same machine-checkable guardrails and then runs any enabled
external engines.

`refresh --check` is the managed-block freshness gate.
It reports declared context blocks that would change without mutating files.
Plain `refresh` repairs those declared blocks in place.

In generated CI wiring, these commands are the intended public enforcement
surface.

For local commit-time enforcement, the intended flow is:

1. keep `.pre-commit-config.yaml` tracked in the repo
2. run `pre-commit install --hook-type pre-commit --hook-type commit-msg`
3. let the installed framework shell out to `project-guardrails pre-commit`
   and `project-guardrails commit-msg-check`

That keeps the primary contract reviewable in the repository instead of hiding
it under `.git/hooks/`.

MCP does not add a separate enforcement model. The local stdio MCP server
exposes the same typed operations as the CLI, so `guardrails.check`,
`guardrails.doctor`, and `guardrails.refresh` report the same proxy signals
through a different access layer.

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
