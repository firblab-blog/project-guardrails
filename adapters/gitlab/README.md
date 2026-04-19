# GitLab Adapter

This directory holds GitLab-oriented workflow examples and future installable
integration examples.

V0 ships a minimal workflow template at
`templates/gitlab/.gitlab-ci.guardrails.yml`.

A richer adaptation example lives at
`examples/gitlab/.gitlab-ci.source-of-truth.example.yml` with supporting notes
in `examples/gitlab/README.md`.
That example shows how a consumer repo can keep GitLab as its private
source-of-truth home while layering additional repo-owned stages such as
quality, security, release, and outbound mirroring.

These files are not a separate runtime surface.
They are public examples and starter material for profile authors.
