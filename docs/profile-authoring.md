# Profile Authoring

This document defines the V0 contract for custom profiles.

## Goal

A profile lets a project bring its own repo doctrine without changing the Rust
CLI.

Profiles should carry project-specific opinions.
The CLI should stay small and portable.

## Supported Layout

A custom profile may be provided either as a directory or a direct path to
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
    ...
```

A worked public example set lives in
[`docs/custom-profile-example.md`](custom-profile-example.md)
with concrete profile directories under
[`examples/profiles/`](../examples/profiles).

## Required File

`profile.toml` is required.

It defines the profile metadata and install expectations:

- `schema_version`
- `name`
- `description`
- `default_ci`
- optional `root_markers`
- `docs_enabled`
- `required_docs`
- `required_files`
- `forbidden_dirs`
- `includes_handoff`
- optional `[workflow_paths]`
- optional Semgrep and Conftest settings

## Templates

`templates/` is optional.

When `init` installs files, it looks for a matching template in this order:

1. the selected profile's `templates/`
2. the shared built-in templates shipped with `project-guardrails`

That lets a custom profile override only the files it cares about while still
reusing the generic shared templates.

Shared templates are intentionally public-facing and mixed-workflow-friendly.
They should be usable by teams working primarily with humans, primarily with
LLMs, or with both.

If a team wants different collaboration wording, prefer overriding template
files in a custom profile rather than adding collaboration modes to the CLI.

## Assets

`assets/` is optional.

If present, its contents are copied into the target repo during bootstrap.
This is the V0 mechanism for shipping profile-owned support files such as:

- rule files
- CI support files
- helper docs
- other repo-local scaffolding

Asset copying is byte-safe.
Profiles may therefore ship binary or non-UTF-8 support files when needed.

## Portability Rules

Custom profiles should follow the same portability rules as the built-in ones:

- avoid assuming one programming language unless the user is opting into that
  explicitly
- avoid assuming one repository layout unless it is part of the profile's
  deliberate contract
- keep project-specific opinions in the profile rather than widening the Rust
  CLI
- prefer copied repo-local files over compiled-in branching when either would
  work

## CI Defaults

`default_ci` must currently be one of:

- `github`
- `gitlab`
- `none`

If `guardrails init` is run without `--ci`, the installer uses the selected
profile's `default_ci`.

Profiles may also declare optional CI destination paths:

```toml
[workflow_paths]
github = ".github/workflows/guardrails.yml"
gitlab = ".gitlab-ci.guardrails.yml"
```

If omitted, the built-in defaults are used.

## Root Detection Defaults

Profiles may optionally override the repo-root markers written into
`.guardrails/guardrails.toml`:

```toml
root_markers = [".git", ".hg"]
```

If omitted, V0 defaults to `[".git"]`.

## Design Guideline

If a behavior can be expressed by:

- profile metadata
- a template file
- a copied asset

prefer that over adding more hardcoded CLI logic.
