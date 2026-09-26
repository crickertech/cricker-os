---
status: AMENDED
raised: 2026-08-25
decided: 2026-08-25
ratified_by: calef
---

# 126. A process holding two directory capabilities gets a real, single, moving `cwd`

*Amended 2026-09-26 (UTC) by §234 (the prompt shows one tree, and other trees are mounted at names
in it), provisional until the merge queue lands it. At the prompt, an absolute path no longer starts
with a tree's label: the person sees one root, and other trees appear at mount names in it. The
single moving cwd below stands.*

calef, 2026-08-25, in conversation, closing one of [milestone
154](../roadmap/154-multi-directory-namespace.md)'s own three "still open" items. Raised as a
direct question: "Let's do a real cwd. Isn't that a better user experience?" Answered yes, with
the boundary behavior settled below.

## The question

[Milestone 154](../roadmap/154-multi-directory-namespace.md) built `grant_plan::nav::TwoRoots`: a
process holding two labeled directory capabilities can resolve `/a/...` against the first and
`/b/...` against the second, and `/a/../b` is refused because there is nothing above `a`'s own
root to pop. That mechanism is deliberately stateless: every lookup is absolute and labeled, and a
bare relative name (no leading `/`) is refused outright, because "a two-grant holder has two roots
and no reason to prefer either" (`TwoRoots::resolve`'s own doc comment) is exactly the shadowing
question [milestone 47](../roadmap/47-navigation-and-naming.md)'s four open questions leave
undecided.

The milestone's own text left "wiring a second grant into the real interactive boot" as calef's
call, "a boot-time decision... policy rather than mechanism." Underneath that sits a sharper,
narrower question that does not require deciding 47's harder open questions (unions, shadowing
across many sources, enumeration, whether `$PATH` survives as a string) at all: **does a two-grant
shell have a real, single, moving current directory, the way a one-grant shell already does?**

## The decision: yes, a real cwd, refuse at either tree's own root

State: a pair `(which: A | B, pos: Cwd)` in place of the single `Cwd` a one-grant `Holdings`
carries today. `pos` is the existing single-tree position type, unchanged.

Resolution:
- A bare relative name resolves against `pos` inside whichever tree `which` currently names.
  Identical to today's one-grant behavior, parameterized by which tree the process is standing in.
- An absolute path (`/a/...` or `/b/...`) resolves the same way `TwoRoots` already does today:
  picks the tree by label, resolves the rest from that tree's own root. This is also how a process
  moves between trees: `cd /b/somewhere` while standing in `a` works with no new verb.

**The boundary: `..` at either tree's own root refuses**, the same `Refused::AtYourRoot` a
one-grant `Cwd::apply` already gives at its own root, applied per-tree rather than newly invented.
Two alternatives were priced before deciding this one:

- Silently clamp instead of refusing (stay put, no error), matching real Unix shells: `/..`
  resolves to `/` itself on every mainstream system, because the on-disk directory-entry format
  requires the root's `..` entry to point somewhere and self-reference is the only sane value.
  That representational necessity does not exist in nife: `Cwd` is a synthetic position tracker in
  a capability-routed namespace, not a walk over on-disk `..` pointers, so refusing costs exactly
  the same one-branch check that clamping would; nothing about nife's implementation makes silence
  the cheaper or more natural choice the way it was for early Unix. And the event means something
  more security-salient here: hitting your root in Unix means "the top of the whole visible
  filesystem," where nothing is hidden by stopping silently; hitting your root in nife means "the
  edge of a specific, narrow capability grant," and silently absorbing that could mask a caller's
  wrong belief about how much it actually holds. `nav.rs`'s own existing doc comment already states
  the principle this extends: "the dangerous refusal is the one that answers." Declined.
- **Hop to the other tree's root on `..`.** The one place this would start behaving like a real
  union of the two trees rather than two disjoint labeled ones, and it buys nothing a process
  cannot already do with one absolute-path `cd`. No real prior art found for this shape either
  (checked against Plan 9's `bind` and standard bind-mount unions, neither of which special-cases
  `..` this way). Declined.

Refuse, unchanged from the one-grant case, applied per-tree, wins: it is nife's own existing
answer to the same question at smaller scope, it is no more code than the alternatives, and the
capability-boundary reasoning that motivated the one-grant refusal in the first place applies
identically to two.

Starting position: the first-listed grant's own root (`which = A`), matching the existing
"slot 0 is always the first grant" precedent [milestone 154](../roadmap/154-multi-directory-namespace.md)
already established for cspace ordering. No new precedent needed.

## What this does not decide

[Milestone 47](../roadmap/47-navigation-and-naming.md)'s four open questions (unions and shadowing
across more than two labeled sources, enumeration, the compile-time-set-to-runtime-lookup gap,
whether `$PATH` survives as a string) remain exactly as open as they were. This decision is
narrower than any of them: two disjoint, individually-labeled trees, one position at a time, no
name ever meaning two different things. `grant_plan::Holdings`'s extension to carry this pair, and
`plan_stage`/`redirect_target`'s and `caps`'s use of it, are implementation, not decided here.

## What it unblocks

The `(which, pos)` design gives whoever builds the rest of [milestone
154](../roadmap/154-multi-directory-namespace.md)'s "still open" list (extending `caps`'s display,
the shell-to-init spawn-protocol encoding) a real state shape to extend `Holdings` with, rather
than an open question to re-raise.
