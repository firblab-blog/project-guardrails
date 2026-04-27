# Repo Shaping

This doctrine profile favors repositories that explain themselves by inspection.
Use repo-local files to make intent, ownership, and workflow visible.

## Shape The Repo For Inspection

- keep important workflow expectations in committed files
- prefer declarative configuration over branching hidden in the installer
- keep the CLI small enough that profiles remain the main extension surface
- separate bootstrap behavior, profile behavior, and installed repo content

## Choose The Smallest Durable Home

When adding a new rule or behavior, ask in order:

1. should this be a repo-local doc or rule file?
2. should this be profile metadata, a profile template, or a copied asset?
3. only if neither fits cleanly, should this become generic CLI behavior?

## Result

The goal is a repository that can be understood by reading:

- `AGENTS.md`
- the current tracker and decision log
- the selected profile content
- the CI and rule files that enforce the intended workflow
