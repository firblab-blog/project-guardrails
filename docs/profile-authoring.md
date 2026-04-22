# Profile Authoring

This document defines the V0 custom-profile contract that `project-guardrails`
actually ships today.

The goal is to let a project bring its own repo doctrine, enforcement
expectations, starter docs, and support assets without widening the Rust CLI.

## Boundary Model

Keep these three layers distinct:

- bootstrap utility logic
  - generic install, upgrade, config writing, lockfile ownership, diagnostics,
    managed-block refresh, and repo-local command behavior
- profile configuration
  - declarative statements about what a repo should contain and what mechanical
    checks should run
- templates, assets, and rule files
  - the actual doctrine, wording, CI definitions, Semgrep policies, Conftest
    policies, helper docs, and other repo-local content

If a change is specific to one team's workflow, one repository shape, or one
set of operating rules, it usually belongs in a profile or in profile-owned
files, not in the generic runtime.

The built-in public set is intentionally small:

- `minimal` for the neutral default baseline
- `docs-driven` for the neutral baseline plus a required decision log
- `guardrails` for the opt-in FirbLab-style doctrine profile

If you need a different doctrine, prefer another profile over more
project-specific branches in the CLI.

## Supported Layout

A custom profile may be provided either as a directory or as a direct path to
`profile.toml`.

Recommended layout:

```text
my-profile/
  profile.toml
  templates/
    AGENTS.md
    docs/project/implementation-tracker.md
    docs/project/handoff-template.md
    .github/workflows/guardrails.yml
    .gitlab-ci.guardrails.yml
  assets/
    docs/project/change-rubric.md
    policy/guardrails/
      checks.rego
    semgrep/
      repo-rules.yml
```

`templates/` and `assets/` are both optional.

A worked public example set lives in
[`docs/custom-profile-example.md`](custom-profile-example.md)
with concrete profile directories under
[`examples/profiles/`](../examples/profiles).

## What `profile.toml` Can Declare

`profile.toml` is required.

It defines profile metadata, install expectations, enforcement defaults, and
profile-owned freshness/context behavior.

### Core Install Fields

These fields shape the installed repo-local baseline:

- `schema_version`
  - currently `1`
- `name`
  - profile name written into `.guardrails/guardrails.toml`
- `description`
  - human-facing summary used by init/upgrade output
- `default_ci`
  - default CI provider when `init` runs without `--ci`
  - must currently be `github`, `gitlab`, or `none`
- `root_markers`
  - optional repo-root markers written into `.guardrails/guardrails.toml`
  - defaults to `[".git"]` if omitted
- `docs_enabled`
  - whether the profile expects required docs to exist
- `required_docs`
  - repo-relative docs expected by `doctor` and `check`
  - if a matching template exists, `init` installs that template
- `required_files`
  - repo-relative files expected by `doctor` and `check`
  - if a matching template exists, `init` installs that template
- `forbidden_dirs`
  - repo-relative directories that must not exist
- `includes_handoff`
  - whether the profile uses the standard handoff-template flow
  - when `true`, `init` ensures `docs/project/handoff-template.md` exists even
    if the rest of the profile is otherwise small
- `[workflow_paths]`
  - optional per-provider CI destination overrides

Example:

```toml
schema_version = 1
name = "team-ops"
description = "Example profile with repo-local release discipline."
default_ci = "gitlab"
root_markers = [".git"]
docs_enabled = true
required_docs = [
  "docs/project/implementation-tracker.md",
  "docs/project/handoff-template.md",
  "docs/project/release-checklist.md",
]
required_files = [
  "README.md",
  "AGENTS.md",
  ".guardrails/guardrails.toml",
  ".pre-commit-config.yaml",
]
forbidden_dirs = []
includes_handoff = true

[workflow_paths]
github = ".github/workflows/guardrails.yml"
gitlab = ".gitlab-ci.guardrails.yml"
```

### Enforcement Fields Written Into Repo Config

These profile fields are copied into `.guardrails/guardrails.toml` during
`init` and `upgrade`.

They define mechanical repo-local checks rather than doctrine text:

- `[task_references]`
  - current field: `required`
  - when `true`, `commit-msg-check` requires at least one task reference for a
    commit that has staged changes
  - referenced tasks must exist under `.guardrails/state/tasks/` and be
    `approved` or `in_progress`
- `[[link_requirements]]`
  - fields: `changed_paths`, `required_docs`, optional `message`
  - checked by `pre-commit`
  - if any staged path matches one of `changed_paths`, at least one staged path
    must also match one of `required_docs`
  - this is a path-to-doc coupling rule, not proof that the doc update is good
- `[[forbidden_patterns]]`
  - fields: `pattern`, optional `message`
  - checked by `pre-commit`
  - the regex is matched against added staged diff lines
  - this is best for explainable "don't commit this text" rules, not deep code
    understanding
- `[semgrep]`
  - external-engine settings for `guardrails check`
- `[conftest]`
  - external-engine settings for `guardrails check`

Example:

```toml
[task_references]
required = true

[[link_requirements]]
changed_paths = ["src/"]
required_docs = [
  "docs/project/implementation-tracker.md",
  "docs/project/release-checklist.md",
]
message = "changes under src/ must update the tracker or release checklist in the same commit"

[[forbidden_patterns]]
pattern = "REMOVE_BEFORE_MERGE"
message = "remove temporary placeholders before commit"

[semgrep]
enabled = true
binary = "semgrep"
config_paths = ["semgrep/repo-rules.yml"]
extra_args = []

[conftest]
enabled = true
binary = "conftest"
policy_paths = ["policy/guardrails"]
extra_args = []
```

Use `changed_paths`, `required_docs`, and external rule paths that match the
consumer repo's actual layout.
The runtime does not assume `src/`, `policy/`, or `semgrep/` globally; those
are profile-owned examples.

### Profile-Owned Freshness And Context Fields

These fields stay in the profile and are not copied into
`.guardrails/guardrails.toml`:

- `[[starter_content]]`
  - fields: `path`, `markers`, `threshold`
  - defines placeholder-content detection for a specific repo-relative file
  - a file is considered starter content when at least `threshold` markers are
    still present
  - `doctor` and `check` use this for required docs and for text required files
    that the runtime validates directly, such as `AGENTS.md` and `README.md`
  - `pre-commit` also checks staged files whose path matches a declared starter
    rule
- `[[managed_blocks]]`
  - fields: `path`, `id`, `generator`, `placement`
  - declares a tool-managed region inside a text file
  - only the declared block is regenerated; the rest of the file remains
    repo-owned
  - `doctor` and `check` can report missing, invalid, or stale managed blocks

Example:

```toml
[[starter_content]]
path = "AGENTS.md"
markers = [
  "Describe the product or system this repository owns.",
  "keep the release checklist current as work lands",
  "leave handoff notes that another contributor can continue without guesswork",
]
threshold = 2

[[starter_content]]
path = "docs/project/release-checklist.md"
markers = [
  "what is intentionally in this release",
  "tests or checks that must pass",
  "who still needs to review",
]
threshold = 2

[[managed_blocks]]
path = "AGENTS.md"
id = "repo-context"
generator = "repo_context_v1"
placement = "after_first_heading"

[[managed_blocks]]
path = "docs/project/implementation-tracker.md"
id = "task-sync"
generator = "tracker_sync_v1"
placement = "after_first_heading"
```

## Managed Blocks

Managed blocks are the main profile-layer extension surface for "refresh part of
this file without taking over the whole file."

They are text-only and use explicit comment markers:

```html
<!-- guardrails:managed start id=repo-context generator=repo_context_v1 -->
...tool-managed content...
<!-- guardrails:managed end id=repo-context -->
```

### Supported Placements

Current supported placements are:

- `after_first_heading`
  - insert after the first Markdown level-1 heading (`# ...`)
  - if no such heading exists, the block is prepended instead
- `prepend`
  - insert at the top of the file

### Supported Generators

Current built-in managed-block generators are:

- `repo_context_v1`
  - intended for files such as `AGENTS.md`
  - renders required context paths plus a short snapshot of active tasks and
    open handoffs
- `tracker_sync_v1`
  - intended for files such as `docs/project/implementation-tracker.md`
  - renders a snapshot of active tasks plus current handoff status

If you declare an unknown generator, rendering fails.
Profile authoring should therefore stick to the generator names the runtime
actually supports.

### How To Use Them Safely

- keep human-written guidance outside the managed block
- use a stable `id` per block within a file
- do not hand-edit the start/end markers into mismatched forms
- expect `init` and `upgrade` to insert or refresh only the declared block
- expect `doctor` and `check` to report:
  - missing managed blocks
  - invalid block markup
  - stale block content when the rendered snapshot no longer matches repo state

Managed blocks strengthen freshness and visibility.
They do not prove that a human or LLM read, understood, or complied with the
surrounding guidance.

## Templates

`templates/` is optional.

When `init` installs files, it looks for a matching template in this order:

1. the selected profile's `templates/`
2. the shared built-in templates shipped with `project-guardrails`

That lets a custom profile override only the files it cares about while still
reusing the generic shared templates.

Use templates for:

- `AGENTS.md` wording
- repo-specific tracker or handoff scaffolding
- required docs the profile wants to seed
- CI workflow templates when the defaults are not enough

The built-in `guardrails` profile uses the same template mechanism.
Its doctrine stays in profile-owned template content rather than special-case
generic runtime behavior.

## Assets

`assets/` is optional.

If present, its contents are copied into the target repo during bootstrap.
Asset copying is byte-safe, so profiles may ship text, binary, or non-UTF-8
support files.

Use assets for:

- Semgrep rule files referenced by `[semgrep]`
- Conftest/Rego policy directories referenced by `[conftest]`
- extra support docs
- helper config files that should live in the consumer repo

If a file is primarily doctrine text, prefer a template.
If it is support material that should be copied as-is, prefer an asset.

## What Belongs Where

Use this rule of thumb when deciding where to put new behavior.

### Bootstrap Utility Logic

Put behavior in the generic runtime only when it must stay cross-profile and
portable, such as:

- loading profiles
- writing `.guardrails/guardrails.toml`
- maintaining `.guardrails/profile.lock`
- installing templates and assets
- refreshing declared managed blocks
- running generic diagnostics and commit-time entrypoints

### Profile Configuration

Put behavior in `profile.toml` when it is a declarative repo expectation, such
as:

- which docs and files must exist
- default CI destination
- whether task references are required
- which path changes require companion docs
- which diff-line patterns are forbidden
- which files still count as starter content
- which managed blocks should be refreshed

### Templates, Assets, And Rule Files

Put behavior in profile-owned files when it is the actual content or doctrine,
such as:

- collaboration instructions in `AGENTS.md`
- seeded tracker, handoff, and release-checklist wording
- Semgrep rule definitions
- Conftest policies
- helper docs and review rubrics

If a behavior can be expressed by profile metadata, a template file, or a
copied asset, prefer that over adding more hardcoded CLI logic.

## Worked References

See:

- [`docs/custom-profile-example.md`](custom-profile-example.md)
- [`examples/profiles/team-ops/profile.toml`](../examples/profiles/team-ops/profile.toml)
- [`examples/profiles/team-ops/templates`](../examples/profiles/team-ops/templates)
- [`examples/profiles/team-ops/assets`](../examples/profiles/team-ops/assets)

The `team-ops` example intentionally shows a richer custom profile surface:

- custom required docs and files
- task-reference enforcement
- link requirements
- forbidden diff patterns
- starter-content detection
- managed blocks layered into human-edited docs
- copied support assets
