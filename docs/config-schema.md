# Config Schema

The repo-local configuration lives at `.guardrails/guardrails.toml`.

Install ownership metadata lives in `.guardrails/profile.lock`.

A concrete example also ships in:

- `templates/shared/.guardrails/guardrails.toml.example`

V0 schema:

```toml
version = 1
profile = "minimal"
profile_source = "built-in:minimal"
profile_schema_version = 1
installed_by_version = "0.2.0"

[project]
name = "my-project"
root_markers = [".git"]

[docs]
enabled = true
required = [
  "docs/project/implementation-tracker.md",
  "docs/project/handoff-template.md",
]

[rules]
required_files = [
  "README.md",
  "AGENTS.md",
  ".guardrails/guardrails.toml",
  ".pre-commit-config.yaml",
]
forbidden_dirs = [
  "server",
  "controllers",
]
link_requirements = []
evidence_requirements = []
forbidden_patterns = []

[rules.rule_packs]
enabled = []

[rules.task_references]
required = true

[[rules.link_requirements]]
changed_paths = ["src/"]
required_docs = ["docs/project/implementation-tracker.md"]
message = "changes under src/ must update the implementation tracker in the same commit"

[[rules.evidence_requirements]]
name = "dependency-rationale"
changed_paths = ["Cargo.toml", "Cargo.lock"]
evidence_paths = [
  "docs/project/implementation-tracker.md",
  ".guardrails/state/tasks/",
]
evidence_patterns = ["(?i)\\b(rationale|because|reason|why|dependency)\\b"]
message = "dependency manifest changes must stage rationale evidence in task notes or project docs"

[[rules.evidence_requirements]]
name = "deleted-test-evidence"
deleted_paths = ["tests/"]
evidence_paths = [
  "docs/project/implementation-tracker.md",
  ".guardrails/state/tasks/",
]
evidence_patterns = ["(?i)\\b(deleted test|replacement test|coverage|obsolete)\\b"]
replacement_paths = ["tests/"]
message = "deleted tests must stage replacement tests or task/tracker evidence"

[[rules.forbidden_patterns]]
pattern = "console\\.log"
message = "remove debug logging before commit"

[ci]
provider = "github"
workflow_path = ".github/workflows/guardrails.yml"

[engines.semgrep]
enabled = false
binary = "semgrep"
config_paths = []
extra_args = []

[engines.conftest]
enabled = false
binary = "conftest"
policy_paths = []
extra_args = []
```

## Field Notes

- `version`
  - schema version for the repo-local config
- `profile`
  - the selected built-in profile or later custom profile
- `project.name`
  - display name used by status and diagnostics
- `profile_source`
  - where the selected profile came from, such as a built-in profile or custom path
- `profile_schema_version`
  - schema version declared by the selected profile
- `installed_by_version`
  - version of `project-guardrails` that wrote the current config
- `project.root_markers`
  - files or directories used when confirming repo root
  - now sourced from profile metadata when the profile declares them
- `docs.enabled`
  - whether docs are expected by the profile
- `docs.required`
  - docs that should exist for `doctor` and future checks
- `rules.required_files`
  - file existence checks
- `rules.forbidden_dirs`
  - directories that should not exist
- `rules.rule_packs.enabled`
  - profile-declared rule pack names to enable for this repo
  - enabled packs expand into the existing config surfaces: required docs,
    required files, link requirements, forbidden patterns, Semgrep config
    paths, and Conftest policy paths
  - packs are not a new evaluator; after expansion, existing commands consume
    the same fields they already understand
- `rules.task_references.required`
  - when `true`, `commit-msg-check` requires a task reference for staged
    changes and validates that referenced tasks are active
- `rules.link_requirements`
  - diff-based path-to-doc rules checked by `pre-commit`
  - when a staged path matches `changed_paths`, at least one staged doc in
    `required_docs` must be part of the same commit
- `rules.evidence_requirements`
  - diff-based path-to-evidence rules checked by `pre-commit`
  - when a staged path matches `changed_paths`, or a staged deletion matches
    `deleted_paths`, the commit must also stage configured repo-local evidence
  - `evidence_paths` lists docs, task files, or other repo-local files that can
    carry the evidence
  - `evidence_patterns` is optional; when present, at least one added line in a
    staged evidence path must match one of the regexes
  - `replacement_paths` is optional; when present, a staged non-deleted path
    matching one of those specs also satisfies the rule
  - path matching uses exact paths or directory prefixes, not glob semantics
  - this is a proxy signal over staged files and added evidence lines, not
    proof that the rationale, decision, rollback, or replacement is complete
- `rules.forbidden_patterns`
  - regex patterns checked against added staged diff lines by `pre-commit`
  - keep these checks mechanical and explainable
- `ci.provider`
  - `github`, `gitlab`, or `none`
  - defaults to the selected profile's `default_ci` when `init` is run without
    an explicit `--ci`
- `ci.workflow_path`
  - the expected installed workflow file
  - now sourced from profile metadata when the profile declares it
- `engines.semgrep`
  - optional Semgrep execution settings for `guardrails check`
- `engines.conftest`
  - optional Conftest execution settings for `guardrails check`

## Rule Pack Selection

Rule packs are selected in repo-local config by name:

```toml
[rules.rule_packs]
enabled = ["llm-common-mistakes", "docs-freshness", "secret-safety"]
```

The active profile must declare each enabled pack. If a repo enables an
unknown pack, commands fail while loading the config so the mismatch is visible
instead of silently weakening enforcement.

Profiles may also default-enable packs during `init` or `upgrade`. Those
defaults are written into `.guardrails/guardrails.toml` as ordinary config, so
the repo can later review, remove, or add pack names explicitly.

Rule packs may contribute required docs, required files, link requirements,
evidence requirements, forbidden patterns, Semgrep config paths, and Conftest
policy paths. Pack expansion is still declarative; it appends to the same
config fields that commands already read.

The built-in `guardrails` profile declares and default-enables
`llm-common-mistakes`, `docs-freshness`, and `secret-safety`. The built-in
`minimal` profile keeps `enabled = []` unless a future profile or custom config
declares packs explicitly.

The built-in `docs-freshness` pack now contributes both broad path-to-doc link
requirements and narrower evidence requirements for source context, public API
decisions, dependency rationales, infra rollback or validation notes, and
deleted-test notes or replacement tests. Those checks are opt-in through the
profile/rule-pack mechanism; the neutral `minimal` profile does not enable
them by default.

## Design Intent

The config should stay declarative.

When a repo wants different opinions, it should usually change the config or
choose a different profile instead of widening the CLI.

Custom profiles may also bring repo-local files through:

- `templates/` for generated docs and workflow templates
- `assets/` for rule files or other support artifacts copied into the target repo

The accompanying `.guardrails/profile.lock` records the selected profile
metadata plus structured managed-path entries for files the installer considers
tool-managed.

Example V0.1 lockfile:

```toml
version = 1
profile = "minimal"
profile_source = "built-in:minimal"
profile_schema_version = 1
config_version = 1
installed_by_version = "0.2.0"

[[managed_paths]]
path = ".guardrails/guardrails.toml"
stale_action = "review"

[[managed_paths]]
path = ".github/workflows/guardrails.yml"
stale_action = "remove"
```

Current intent:

- config, docs, `AGENTS.md`, and copied assets are review-only when stale
- repo-tracked enforcement config such as `.pre-commit-config.yaml` is also
  review-only when stale
- generated CI workflow files are the only built-in auto-removable stale files

See [`docs/install-ownership.md`](install-ownership.md)
for the ownership contract.
