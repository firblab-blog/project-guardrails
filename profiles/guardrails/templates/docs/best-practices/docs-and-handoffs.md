# Docs And Handoffs

Good repo-local docs are part of the operating system for human and LLM
contributors.
They should reduce ambiguity, not just exist on disk.

## Documentation Rules

- keep `AGENTS.md` focused on durable workflow expectations
- keep the implementation tracker aligned with the current approved slice
- use the decision log for durable choices, not transient notes
- update docs in the same change when the behavior they describe changes

## Handoff Rules

- summarize what actually changed, not what you intended to change
- list real validation and missing validation separately
- point the next contributor at the narrowest valid next step
- restate any non-goals or risks that should still constrain the work

## Smells

Refresh the docs before continuing when:

- the tracker still points at work that is already done or no longer approved
- a decision changed but only lives in chat, memory, or commit history
- the handoff cannot tell another contributor where to restart safely
