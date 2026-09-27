---
status: DECIDED
raised: 2026-09-27
decided: 2026-09-27
ratified_by: calef
---

# 237. Subtree grants may skip the caretaker, per filesystem

*Section number provisional until the merge queue lands it.*

Raised on pull request #1387, milestone 606 (a directory walk costs what it does on Linux), as
option D. It is a third wire-and-architecture change, alongside the A and B wire changes of §236
(`OPEN`/`OPENDIR` take a relative path, and `OPEN`'s reply carries the size). D narrows a subtree
grant on the server itself rather than through a per-client caretaker process.

Measured at 0.35 ms, against the caretaker's 0.52-0.53 ms (with A and B) and Linux's 0.41-0.42 ms.
D alone would put nife ahead of Linux on this walk. calef ruled D, option 2, on per-filesystem
terms, at 2026-09-27T07:17Z.

## The ruling

- Caretakers stay the default. Every filesystem runs behind one unless it qualifies for the fast
  path below.
- D is a fast path a filesystem server earns, not a replacement for caretakers. It requires the
  server to be memory-safe. It must also resolve every path through one shared scope crate, written
  once and proven with Kani, that owns `..`, symlinks, links and mount crossings. A filesystem
  supplies lookups and never makes the containment decision itself.
- **Scope: subtree grants only.** Name-set (glob) grants keep their caretaker.
- **The progenitor chooses per mount**, minting a badge (via `BIND`/`UNBIND`) or building a
  caretaker, from what the filesystem's package declares.

calef's framing, in his words: *"We are going to have many more file systems than RedoxFS. Some of
them large. File systems are often sources of vulnerabilities."* That is why the fast path is
conditioned on a proven scope crate rather than granted to every filesystem. A caretaker is 111 and
170 lines of code, where a filesystem server can be over a thousand lines plus tens of thousands
more in a vendored library. A handle-scoping bug under D does not widen the blast radius, and the
reason is that the scope crate, not the filesystem, decides containment.

## What D gives up, and what it doesn't

- What D gives up is the argument of §27 (the filesystem service is a capability-shaped contract
  over a component we did not write): that confinement is a property of the client's capability
  space today. Under D, containment becomes a branch inside the server instead, checked by the
  shared scope crate rather than enforced by a separate process boundary.
- Revocation becomes `UNBIND` (the server stops honoring the badge), where a caretaker's revocation
  ends the caretaker process instead. No kernel method pulls a capability back out of a client
  under either design.
- Name-set grants could move to the same shape later. calef's ruling holds that decision back and
  restricts D to subtrees, the smaller step.

## Built

`BIND` and `UNBIND` (`filesystem_protocol::fs`, provisional numbers 64 and 65) are built in #1408
(open at the time of this section). They sit outside the verb table, so no caretaker forwards
them, and a bound caller is refused them. #1408 also adds `crates/subtree_scope` (name provisional:
`walk`, `landing`, `admit`, `Bindings`), proven by four Kani harnesses. It wires `redoxfs_server` in
as the first eligible server.

## Reversibility

Nothing outside this tree has acted on `BIND`/`UNBIND`'s wire numbers yet, and the scope crate is
new and unshipped, so the fast path itself is still cheap to revisit. What is expensive to revisit
is once a filesystem's package declares it: a filesystem that starts under D and is later found to
need a caretaker after all means moving its clients off the badge shape. That touches every one of
its grants.
