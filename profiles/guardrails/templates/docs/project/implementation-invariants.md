# Implementation Invariants

These constraints should remain true even as the repo evolves.

- the bootstrap utility stays generic and portable across repository layouts
- the neutral baseline remains the default built-in profile
- FirbLab-style doctrine remains opt-in and profile-owned
- repo-local docs and rules are the primary enforcement surface
- reviewable files in the repo are preferred over hidden install-time magic
- when behavior and doctrine diverge, update the docs or narrow the code
