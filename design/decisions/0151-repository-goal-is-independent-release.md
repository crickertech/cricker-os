---
status: DECIDED
raised: 2026-09-15
decided: 2026-09-15
ratified_by: calef
---

# 151. The goal of the repository split is independent release and third-party programs

calef, 2026-09-15, in conversation, thinking through the eventual multi-repository
structure: *"Independent releases is the goal and third party programs is the goal."*

## What this settles

Milestone 39 (repository structure for a loosely-coupled OS) recorded four options and took no
decision, on the ground that nothing external needed one yet. Its options B, C and D were three ways
of reaching, or deferring, a decoupled structure; the open question underneath them was what the
structure is for. This decides that:

- Independent release. Each shippable piece can be versioned and released on its own cadence,
  behind an interface that does not change under its consumers. This is the property that decides the
  grain of the split (see BUGS): a thing earns separation when it can ship on its own schedule, not
  because it is a separate package.
- Third-party programs. The structure is the one an outside author writes a program against
  without this project in the loop, which is milestone 39's option C described as *"what an ecosystem
  with third-party components looks like"* and milestone 198's *"no third party sees nife until there
  is a package manager and a trivial install."*

So the endpoint is the decoupled ecosystem, not the monorepo held forever. Milestone 39's own
recommendation is consistent with this: get there through option B (workspaces inside one repository)
and let the distribution be a manifest repository (`basalt`, reserved by milestone 120), reaching the
split when the seams exist rather than by directory surgery up front.

## What this does not settle, and it is deliberately separate

The order. *When* the split happens, against what preconditions, is not decided here and should be
ruled on its own. The candidate sequencing, from the records rather than invented: the wire protocols
carry real version numbers and have stopped churning weekly (milestone 39, §14), and a package format
exists so `basalt` has something to assemble (milestone 198). Until then the single-command,
all-architecture `script/test` is the credibility mechanism milestone 39 says a premature split would
quietly break, so the strain has to justify the trade. That trade is a separate ruling.

## The model is a Linux distribution, not a BSD (calef, 2026-09-15)

A BSD ships a base system: kernel plus core userland as one cohesive tree, developed and released
as a unit, with ports layered on top. There is a privileged base, and it *is* the OS. A Linux
distribution has no base tree: the kernel is one upstream project on its own cadence, every userland
piece is a separately-versioned upstream, and the distribution is an integrator that packages those
independent pieces together with glue and a package manager, owning none of them.

nife is the second. Three consequences that are the point of naming it rather than a stylistic note:

- The kernel is one independently-released component, not a privileged base. From the
  distribution's view it is packaged like any other piece. This is why independent release (above) is
  the property that matters and not "the base tree stays coherent": there is no base tree to keep
  coherent.
- `basalt` is the distribution, not a base manifest. It packages the kernel, the components, and
  the system glue, the way Debian assembles Linux plus a userland it did not write. It is an
  integrator and an assembly step, which is exactly milestone 39's packaging observation (the program
  manifest plus measured-boot hashing are three quarters of a package format already). It is **not**
  a pinned manifest over a base system that a BSD-shaped reading of milestone 39's "distribution as a
  manifest repo" recommendation would suggest.
- Third-party programs are first-class, not ports on top of a base. A program an outside author
  writes is the same kind of thing as one this project wrote; the distribution packages both. That is
  the Linux-distribution posture and it is what "third-party programs is the goal" means structurally.

**What this does not change** is the near-term path. Develop in one repository now (milestone 39's
option B) and reach the split when the seams exist; the *target model* being a distribution rather
than a base system does not force the split earlier. It corrects the mental model of the endpoint,
and with it the maintainer's earlier BSD-leaning comparison, not the sequencing.

## What is unblocked

- Milestone 39 can move from RECORDED to a direction: its option C is the stated destination,
  reached as B. It no longer waits on "what is the structure for."
- The wire-protocol crates gain a reason to carry real versions, which milestone 39's packaging
  observation names as the thing that lets components evolve independently at all. Independent release
  is not possible while every crate is `version = "0.1.0"`.
- `basalt` (milestone 120's reserved distribution repository) has its job named: it assembles
  released pieces, and it can begin as a manifest repository that names what the distribution contains
  before it holds any code.

## BUGS

- **This decides the goal, not the grain.** "One repository per package" was calef's opening framing
  in the same conversation; the maintainer argued for splitting on release cadence instead, since a
  piece earns a repository when it can ship independently and almost nothing can yet while the
  protocols move. This section records that independent release is the *test* for the grain; which
  pieces pass it is measured when the split is scheduled, not asserted here.
- The integration gate is the real cost and is not solved by this decision. A multi-repository
  world has to keep proving the whole system on every supported architecture, and milestone 39 records
  that a naive split makes that proof "live somewhere awkward or quietly stop running." Deciding the
  goal does not build that gate.
