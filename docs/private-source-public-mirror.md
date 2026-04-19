# Private Source, Public Mirror

This document records the repo-owned operating model for maintaining
`project-guardrails` privately in GitLab while publishing a smaller public
GitHub surface.

This is maintainer guidance for repository owners.
It is not a `project-guardrails` runtime feature.
The CLI does not host, sync, or manage a cross-forge mirror.

## Surface Decision

For `project-guardrails`, GitHub is intentionally both:

- the public source-and-docs surface
- the public release surface for tagged archive downloads

The private GitLab repository remains the source of truth for maintainer
operations such as merge requests, internal planning, private CI composition,
and mirror/export automation.

That means the public export should contain the product surface an outside user
needs to install, evaluate, build, test, and adopt the tool without relying on
private GitLab context.

It should not contain:

- private-source CI orchestration
- mirror/export control files used only by maintainers
- internal prompts that assume private GitLab context or undisclosed
  maintainer workflow

## Chosen Pattern For This Repository

`project-guardrails` should follow the same broad operating model as
`firblab`:

- private GitLab repository as the source of truth
- repo-owned CI job prepares the public output
- public GitHub repository receives the reviewed exported result

But for `project-guardrails`, the implementation should be simpler than
`firblab`.

`firblab` needs a sanitization pipeline because its public output strips or
rewrites infrastructure-specific material before publishing, and it uses an
intermediate public repo as part of that flow.

`project-guardrails` does not currently show the same need.
The current repo surface is better served by an explicit allowlist export:

- no text rewriting by default
- no intermediate public repository
- direct push from private GitLab CI to the public GitHub repository

That is the recommended default until real evidence appears that exported
content needs rewriting rather than simple exclusion.

Revisit that decision only if one of these becomes true:

- public-facing files need tokenized replacement of private hostnames, URLs, or
  organization-specific wording
- the public repo needs a materially different file layout than the source repo
- release or docs publishing needs a second reviewed staging repo before GitHub

## The Model

Use two repositories with different purposes:

- a private GitLab repository as the source of truth
- a public GitHub repository as a filtered or barebones mirror

In this model, the private GitLab repository is where the team does normal
day-to-day work:

- planning and implementation
- internal review and discussion
- CI wiring and runner setup
- project-specific templates, rules, and docs
- any automation that should stay inside the organization

The public GitHub repository exists to publish the parts you want outside
consumers to see, clone, or build from.

`project-guardrails` still plays the same narrow role in both places:
it bootstraps repo-local files and validates the repo-local contract.
It does not become a control plane for source hosting.

## Why Teams Use This Split

Teams often want to keep internal operating detail private while still
publishing a clean public codebase or release surface.

Common reasons include:

- internal issue tracking and roadmap documents should not be public
- private CI credentials, runner details, and deployment steps should not be
  exported
- internal-only profiles, templates, or rules may contain project doctrine
  that is not intended for public consumers
- the public repo may need fewer files so outside contributors get a smaller,
  easier-to-read baseline

That does not make `project-guardrails` a platform.
It only means the repo owner can use the tool inside a broader repository
publishing workflow.

## Source Of Truth

Treat the private GitLab repository as authoritative for:

- default branch history
- merge requests and approvals
- protected branch rules
- internal docs under `docs/`
- project-specific `.guardrails/` config and profile choices
- CI provisioning details
- private support scripts and automation

If a public mirror exists, it should be derived from this private source, not
edited independently and then merged back by hand.

That keeps the bootstrap story understandable:
`project-guardrails` installs and checks files in one repo, and the repository
owner decides what subset is exported elsewhere.

## What The Public Mirror Usually Contains

A public GitHub mirror is often intentionally smaller than the private source.
For this repository, the exported GitHub surface should include:

- the Rust source and Cargo metadata
- the public `README`
- public docs that explain install, bootstrap, or release usage
- public built-in profiles and templates
- sanitized examples and fixtures
- tests and fixtures needed to validate the public checkout
- release workflow metadata that is safe to expose

The public mirror can be a near-full mirror or a barebones export.
The important part is that the exported shape is a repo-owner choice, not a
new mode in the bootstrap utility.

## How This Fits The Install Contract

This private-home and public-mirror model does not change the public install
contract for `project-guardrails`.

The supported `v0.1` story remains:

- install the CLI from `crates.io` as the official convenience path
- use tagged GitHub release archives plus `SHA256SUMS` verification as the
  public fallback path
- keep source checkout as the developer path
- keep CI provisioning repo-owned

If a team uses GitLab as the private source of truth and GitHub as the public
mirror or release surface, that is still compatible with the same narrow
contract.
GitHub can host the public mirror and tagged release assets without turning
the CLI into a forge-integrated platform.

The public mirror can still carry narrow maintainer docs when they improve
release trust or explain the public product boundary, as long as they are
self-contained and do not depend on private-source context to make sense.

See [`docs/install.md`](install.md)
for the install contract and
[`docs/gitlab-ci.md`](gitlab-ci.md)
for the GitLab-first CI pattern.

## What Can Stay Private

The following kinds of files or automation often stay only in the private
GitLab source repository:

- internal roadmap, incident, or planning documents
- handoff notes that contain internal team structure or private links
- org-specific `AGENTS.md` wording that assumes internal tools or policies
- private custom profiles under `profiles/` that are not part of the public
  product
- internal templates or assets used only for company repos
- CI jobs that depend on self-hosted runners, private registries, or secrets
- release promotion scripts that push to internal package or artifact systems
- mirror/export manifests and helper scripts used only to prepare the public
  snapshot

Those private files can still be bootstrapped or validated inside the private
repo if that is how the team uses `project-guardrails`.
Keeping them private does not require any new runtime feature.

## When Mirroring Should Happen

Mirroring should happen after the private source repository reaches a state you
are willing to publish.

Practical patterns include:

- after changes land on the protected default branch in GitLab
- after an internal release branch is cut and reviewed
- after docs or examples are sanitized for public consumption
- at tagged releases when the public repo is mainly a release surface

Avoid treating the public mirror as a second live source of truth.
If both repositories accept unrelated direct edits, ownership becomes unclear
and the bootstrap/install story gets harder to reason about.

## Filtering Approaches

There is no single required mirroring mechanism.
A team can choose any repo-owned approach that matches its risk tolerance.

Common patterns are:

1. Mirror the whole repository when private-only material already lives
   elsewhere.
2. Export a filtered branch or snapshot that excludes private docs, scripts,
   and CI files.
3. Maintain a barebones public repo that only receives selected source,
   profiles, templates, and release files.

In every case, keep the separation explicit:

- private repo: full operating source
- public repo: reviewed exported subset

This repository keeps one concrete repo-owned snapshot-export helper in the
private source tree with an allowlist manifest.
That helper is intentionally outside the Rust CLI.
It is maintainer automation for one source-of-truth workflow, not bootstrap
utility behavior.

The helper is designed to be driven by CI variables such as:

- `PUBLIC_MIRROR_URL`
- `PUBLIC_MIRROR_BRANCH`
- `PUBLIC_MIRROR_MANIFEST`
- `PUBLIC_MIRROR_GIT_NAME`
- `PUBLIC_MIRROR_GIT_EMAIL`
- `PUBLIC_MIRROR_PUSH_TAGS`

For compatibility with the existing firblab GitHub-mirror pattern, this
repository's GitLab CI can also derive the push URL from:

- `GITHUB_MIRROR_TOKEN`
- `GITHUB_MIRROR_REPO`

That keeps the private/public publishing policy in repo-owned automation and
CI configuration instead of expanding the Rust runtime.

## Recommended Publishing Topology

For `project-guardrails`, prefer this topology:

1. private GitLab repository remains the only source-of-truth working repo
2. GitLab CI runs tests, release checks, and the allowlisted mirror export
3. the mirror job force-pushes the exported snapshot directly to
   `firblab-blog/project-guardrails` on GitHub
4. the public GitHub repository serves both as:
   - the public source/docs repo
   - the public GitHub Releases repo

Do not add an intermediate public GitLab repo unless the repo later needs
sanitization or multi-step review that the current allowlist flow cannot
provide.

## How This Differs From `firblab`

Use the `firblab` sanitization flow as a reference for ownership boundaries and
CI responsibility, not as the exact topology to copy.

`firblab` uses repo-owned sanitization config and CI to prepare a different
public surface with exclusions and replacements.
That is appropriate for infrastructure content with private operational detail.

`project-guardrails` should keep the same ownership boundary while choosing the
smaller mechanism:

- `firblab`: sanitize, rewrite, and publish through a public-repo staging flow
- `project-guardrails`: allowlist export and push directly to the public GitHub
  repo

That keeps this repository aligned with the reference pattern without adding
complexity it does not currently need.

## Creating The Missing GitHub Public Repo

When the team is ready to stand up the real public repo:

1. Create an empty GitHub repository named `project-guardrails` under the
   intended public owner, such as `firblab-blog`.
2. Set the default branch to `main`.
3. Do not add a README, license, or starter workflow in GitHub UI if you want
   the first export push to define the repo contents cleanly.
4. Create a GitHub token or GitHub App credential that can push commits to that
   repository.
5. Store the push credential in GitLab CI as masked protected variables.
   You can either:
   - set `PUBLIC_MIRROR_URL` directly, or
   - use the existing firblab-style `GITHUB_MIRROR_TOKEN` plus
     `GITHUB_MIRROR_REPO`
6. Run a preview export first from the private repo to inspect the exact public
   tree before enabling pushes.
7. Enable the mirror job on the default branch, then decide whether tag pushes
   should also publish mirrored tags.

Example `PUBLIC_MIRROR_URL` shape for HTTPS push:

```text
https://<github-user-or-app>:<github-token>@github.com/firblab-blog/project-guardrails.git
```

Recommended GitLab CI variables:

- `PUBLIC_MIRROR_URL`
  Authenticated HTTPS remote for the public GitHub repo.
  Optional when using `GITHUB_MIRROR_TOKEN` plus `GITHUB_MIRROR_REPO`.
- `GITHUB_MIRROR_TOKEN`
  Existing firblab-style GitHub credential for CI push jobs.
- `GITHUB_MIRROR_REPO`
  Base HTTPS GitHub repo URL, for example
  `https://github.com/firblab-blog/project-guardrails`.
- `PUBLIC_MIRROR_BRANCH`
  Usually `main`.
- `PUBLIC_MIRROR_GIT_NAME`
  Commit author name for mirror snapshots.
- `PUBLIC_MIRROR_GIT_EMAIL`
  Commit author email for mirror snapshots.
- `PUBLIC_MIRROR_PUSH_TAGS`
  Set to `1` only when you want GitLab release tags mirrored to GitHub too.

## Previewing The Export Before First Push

Use the helper locally or in CI to inspect the exact export without pushing:

```bash
PUBLIC_MIRROR_EXPORT_DIR=/tmp/project-guardrails-public \
PUBLIC_MIRROR_SKIP_PUSH=1 \
bash scripts/mirror-public.sh
```

That should produce a repo-shaped directory containing only the allowlisted
public surface.
Review it as if it were the real GitHub repo:

- README and install docs should make sense without private GitLab context
- examples, templates, profiles, fixtures, and tests should still be complete
- no maintainer-only planning or mirror-control files should be present

## Wiring The Existing GitLab Pipeline

This repository already has a repo-owned mirror stage in
[`/.gitlab-ci.yml`](../.gitlab-ci.yml).
The current `mirror:public` job is the right integration point for the real
public repo.

That means the missing setup work is mostly operational:

- create the GitHub repo
- provide either `PUBLIC_MIRROR_URL` or `GITHUB_MIRROR_TOKEN` plus
  `GITHUB_MIRROR_REPO`
- optionally set `PUBLIC_MIRROR_BRANCH`
- decide whether `PUBLIC_MIRROR_PUSH_TAGS=1` is desired

No Rust CLI change is needed.
No separate sanitization pipeline is required unless future content proves that
allowlisting is insufficient.

## Suggested First Real Rollout

The lowest-risk rollout is:

1. run the helper in preview mode and inspect the exported tree locally
2. create the empty GitHub repo
3. enable `PUBLIC_MIRROR_URL` only for the protected default branch
4. let `mirror:public` publish the default branch snapshot first
5. inspect the GitHub repo as an outside user
6. only then decide whether release tags should also be mirrored

That keeps the first rollout reversible and makes it easy to separate
"public source/docs repo is correct" from "public tag mirroring is correct."

## A Concrete Operating Example

One workable model looks like this:

1. The team develops in a private GitLab repository.
2. `project-guardrails init` or `upgrade` manages the repo-local baseline in
   that private repository.
3. Internal-only docs, private-source CI wiring, and export scripts remain in
   the private repository.
4. A repo-owned release or publish job prepares a reviewed public subset.
5. That subset is pushed to a public GitHub repository after review or at tag
   time.

Example private-only paths:

- `docs/project/internal-release-checklist.md`
- `scripts/publish-public-mirror.sh`
- `.gitlab-ci.yml`
- `.guardrails/internal.profile.toml`

Example public paths:

- `src/`
- `Cargo.toml`
- `README.md`
- `docs/install.md`
- `docs/quick-start.md`
- `profiles/minimal/`

The point is not the exact file list.
The point is that the repository owner can define a reviewable export boundary
without asking `project-guardrails` to become a hosted sync service.

For this repository, the snapshot-export boundary is maintained in repo-owned
mirror automation.
If the public surface should grow or shrink, update that repo-owned manifest
and docs rather than widening the CLI.

## How To Keep It Bootstrap-Tool-Focused

To stay within the intended product boundary:

- keep mirror policy in docs, scripts, and repo automation
- keep project-specific export rules in private repo-owned assets
- keep `project-guardrails` responsible only for bootstrap, install ownership,
  and repo-local validation
- avoid proposing a `sync`, `publish`, or hosted mirror command in the CLI

If a team needs special docs, templates, or checks for the private source
repository, prefer expressing that through profile contents and repo-local
files rather than widening the Rust runtime.

## What This Does Not Mean

Using GitLab privately and GitHub publicly does not make
`project-guardrails` into:

- a hosted platform
- a forge bridge
- a mirror orchestration service
- a deployment control plane

It remains a portable bootstrap utility.
The mirroring workflow around it belongs to the repository owner.

## Related Docs

- [`docs/install.md`](install.md)
- [`docs/gitlab-ci.md`](gitlab-ci.md)
- [`examples/gitlab/README.md`](../examples/gitlab/README.md)
