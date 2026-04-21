# Repo Layout

V0 layout:

```text
project-guardrails/
  Cargo.toml
  src/
  profiles/
    minimal/
    docs-driven/
  templates/
    shared/
    github/
    gitlab/
  adapters/
    github/
    gitlab/
    pre-commit/
  rules/
    semgrep/
    opa/
  fixtures/
    bare-repo/
    rust-repo/
    monorepo/
    repo-with-spaces/
  tests/
  docs/
```

## Why This Layout

- `src/`
  - the bootstrap utility and validation commands
- `profiles/`
  - declarative repo guardrail bundles
  - optional profile-local templates and assets for custom installs
- `templates/`
  - repo-local files to materialize into consumer repos
- `adapters/`
  - CI and tool integration examples
  - V0 status: informational examples and placeholders, not a plugin system
- `rules/`
  - optional reusable enforcement assets
  - V0 status: example assets and future profile inputs, not globally enabled built-ins
- `fixtures/`
  - portability test inputs
- `tests/`
  - fixture-backed integration tests for bootstrap and checks
- `docs/`
  - design and usage documentation
