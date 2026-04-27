# LLM Operating Contract

This document defines the V0.1 repo-local operating contract for LLM-assisted
work in repositories bootstrapped by `project-guardrails`.

The goal is not perfect compliance.
The goal is to make correct behavior more likely, more visible, and more
reviewable through a small set of durable repo-local files and checks.

`project-guardrails` does not provide a hosted controller, agent runtime, or
host-specific LLM integration in V0.1.
It installs repo-local assets, durable workflow state, adapter targets, a local
MCP access layer, and small CI wiring that teams can read, edit, and enforce
locally.

## Core Promise

The V0.1 promise is limited:

- put the intended operating context inside the repository
- make key human/LLM-facing files easy to find
- keep active tasks, handoffs, and session-start context in durable repo-local
  state
- refresh declared managed blocks so adapter and guidance files can carry
  current task/handoff context
- validate machine-checkable proxy signals through `doctor` and `check`
- expose the same loop to MCP-capable local clients over stdio
- keep the contract public, portable, and editable

The tool does not promise:

- perfect LLM instruction-following
- uniform behavior across editors, agents, or chat hosts
- semantic understanding of every repo policy
- automatic prevention of all drift or bad decisions

## The 4-Layer Model

Think about LLM collaboration as four layers, ordered from most durable to
least durable.

### 1. Human-Approved Repo Intent

This is the project's actual intent as maintained by humans.

Typical examples:

- the repository purpose
- approved scope
- explicit non-goals
- current next steps
- validated decisions recorded during work

This layer matters most.
If a temporary instruction, tool integration, or generated prompt conflicts
with the repo's approved written intent, the repo's approved intent should win.

### 2. Authoritative Repo-Local Guidance Files

These are the main durable files that humans and LLMs should read before or
during substantial work.

For the shared templates in V0.1, the authoritative collaboration set is:

1. `AGENTS.md`
2. `docs/project/implementation-tracker.md`
3. `docs/project/handoff-template.md`

Their roles are intentionally simple:

- `AGENTS.md` states the durable collaboration contract
- the implementation tracker states the currently approved slice of work
- the handoff template captures what changed, what was validated, and what
  remains

These files are repo-local assets, not remote policy.
Teams are expected to edit them after bootstrap so they reflect the actual
repository.

### 3. Machine-Checkable Enforcement Proxies

Some parts of the contract can be checked mechanically even when full semantic
compliance cannot.

In V0.1, the main enforcement path is:

```bash
project-guardrails refresh --target . --check
project-guardrails doctor --target .
project-guardrails check --target .
```

That contract is intentionally centered on managed-block freshness plus
`doctor` and `check`:

- `refresh --check` reports stale declared managed blocks without writing
- `doctor` validates preconditions such as required files and runnable external
  setup
- `check` executes configured checks once the repo passes basic preflight

These commands can verify useful proxy signals, such as:

- required files exist
- configured checks run successfully
- starter templates have been replaced when portable detection exists

They cannot prove that:

- an LLM actually read the right files
- a contributor fully understood the instructions
- every handoff was complete or high quality
- every repo-specific policy was followed semantically

That limitation is intentional.
The tool should enforce what is clearly machine-checkable and avoid pretending
that proxies are guarantees.

### 4. Host-Specific Delivery Context

This is the least durable layer.
It includes whatever editor, chat host, agent wrapper, or local workflow is
used to expose repo-local guidance to a contributor or LLM.

Examples:

- a terminal-based coding agent
- an IDE extension
- a chat tool that injects selected repo files into context
- human copy/paste workflows

`project-guardrails` does not standardize this layer in V0.1.
Different hosts may load context differently, truncate it, ignore it, or apply
their own prompting rules.

That is why the contract is anchored in repo-local files and lightweight CI
checks rather than in host-specific integration.

## Read-Only Session Starters

Use `project-guardrails brief --target .` when a human or LLM needs a compact
paste-friendly starting point for a new session.

`brief` reports the current profile, required reading, active tasks, latest
handoff, diagnostics, and suggested next commands. It is broad by design: use
it when the session needs to orient itself to the repository before choosing a
specific continuation path. It is intentionally read-only: it does not create a
pre-work run, does not run `refresh`, and does not mutate tasks, handoffs,
config, lockfiles, or docs.

Use `project-guardrails resume --target .` when a human or LLM is returning to
existing work and needs the strongest continuation cue. `resume` is narrower
than `brief`: it centers the latest durable handoff, the active task context
linked from that handoff, current doctor diagnostics, and one conservative next
step. It is also read-only: it does not create a pre-work run, does not run
`refresh`, and does not mutate tasks, handoffs, config, lockfiles, managed
blocks, or docs.

When Git data is available, `resume` also reports optional Git continuity
context: a conservative path list from the latest handoff baseline to the
current working tree, plus current staged, unstaged, and untracked paths. This
context is deliberately humble. It can help a returning contributor notice what
changed locally, but it cannot prove why the work changed, whether tests ran,
or whether the handoff captured every important detail. Outside Git, without a
usable `git` binary, with malformed handoff timestamps, or when no baseline
commit can be selected, `resume` reports fallback diagnostics and continues to
return the normal handoff/task/doctor continuation payload.

Use `project-guardrails timeline --target .` when a session needs a compact
newest-first view of existing repo-local task, handoff, and pre-work run state.
It is also read-only and should be treated as a continuity aid, not as full
Git, CI, or audit history.

Use `project-guardrails pre-work --target .` when the session should write a
durable run record under `.guardrails/state/runs/`. `status --for-llm` remains
available for callers that want only the canonical machine-readable repo
summary.

Use `project-guardrails handoff new --target . --slug <slug> --from-git` when a
handoff draft should start from observable Git working tree state. The generated
body is only a draft. It says that Git observations are not proof of intent,
validation, or semantic completion, and the contributor should replace the
draft placeholders with the actual result, validation commands, and remaining
work. Plain `handoff new` remains template-based and does not require Git.

## The Shipped Continuity Loop

The practical V0.1 loop is:

1. `brief` for a new session, or `resume` for a returning session
2. `tasks list|get|new|claim|update|close|lint` for approved work state
3. `handoff new|list|close` for durable continuation notes
4. `refresh --check` before validation, and `refresh` when declared managed
   blocks need repair
5. `doctor` and `check` for machine-checkable local validation
6. `adapters list` to inspect profile-declared host instruction targets
7. `mcp serve` when an MCP-capable local client should call the same typed
   operations without screen scraping

This loop is repo-local. The durable state lives under `.guardrails/state/`,
the configured contract lives in `.guardrails/guardrails.toml` and installed
docs, and the enforcement surface remains proxy-based.

Adapter targets are profile-declared managed targets such as `CLAUDE.md` or
Cursor `.mdc` files. They can carry managed blocks refreshed by
`project-guardrails refresh`, but they are still normal repo files. There is no
separate `adapters install` command.

The MCP server is a local stdio access layer over the same typed operations
used by CLI commands. It is repo-scoped by its launch `--target`, does not
listen on the network, and does not host or orchestrate agents.

## Authoritative Files In Shared Templates

The shared template set should mark authoritative human/LLM-facing files
explicitly.

For V0.1, those files should:

- say they are authoritative repo-local guidance
- tell contributors what to read first
- stay short and durable
- avoid claiming that LLM compliance is guaranteed

This keeps the contract legible even when the installer is absent later.

## CI Contract

Public CI templates should stay small and centered on the real contract.

For V0.1, that means:

- run `doctor` first
- run `check` second
- avoid host-specific LLM integration
- avoid implying that CI can verify every collaboration rule semantically

The CI role is to back the repo-local contract with meaningful checks, not to
become an agent orchestration layer.

## Writing Guidance For Profiles And Templates

When adding human/LLM-facing template content:

- keep durable policy in repo-local files
- keep project-specific opinions in profiles and installed assets
- prefer explicit "read this first" language over long prose
- separate mandatory rules from helpful guidance
- describe enforcement honestly

Good wording:

- "Read these files before substantial work."
- "This file is authoritative repo-local guidance."
- "CI validates some proxy signals through `doctor` and `check`."

Avoid wording like:

- "LLMs will follow this contract."
- "These checks guarantee compliance."
- "The host will always load the right files."

## Honest Public Framing

The public message for V0.1 should be:

`project-guardrails` helps teams make LLM-oriented collaboration more explicit
and more enforceable in portable, repo-local ways.

It does not eliminate review.
It does not eliminate contributor judgment.
It does not guarantee that every LLM, tool, or human collaborator will behave
correctly.
