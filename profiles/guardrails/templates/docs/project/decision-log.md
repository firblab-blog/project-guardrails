# Decision Log

Record durable decisions here so future contributors can understand why the
current workflow exists.

## 2026-04-22 Keep Doctrine Opt-In

- decision: keep the built-in `guardrails` doctrine profile opt-in and preserve
  `minimal` as the neutral default
- rationale: `project-guardrails` is a portable bootstrap utility first, not a
  single mandatory worldview
- consequences: opinionated workflow guidance belongs in profiles, templates,
  and installed docs instead of generic bootstrap logic

## Entry Format

For each new decision, capture:

- date
- decision
- rationale
- consequences
- follow-up docs or rules that must stay aligned
