# Change Safety

This file was seeded by the built-in `guardrails` profile.
Edit it when the repository needs sharper or narrower safety guidance.

## Goals

- make risky changes obvious before they spread
- keep changes small enough to validate and review
- leave enough evidence that the next contributor can continue safely

## Practices

- prefer the smallest change that proves the next step
- update repo-local docs when behavior, ownership, or workflow expectations
  change
- avoid speculative widening when the approved slice is narrower
- call out what was not validated instead of implying confidence you do not
  have
- preserve existing user-owned content unless replacing it is clearly part of
  the approved work

## When To Pause

Pause and re-scope before continuing when:

- the change starts depending on a new doctrine that should live in a profile
- the repo needs a new invariant or a new non-goal to stay aligned
- validation gaps are large enough that another contributor could misread the
  current state
