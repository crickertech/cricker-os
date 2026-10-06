---
status: BUILT
raised: 2026-07-22
built: 2026-07-22
---
# 13. Capability revocation + untyped reclamation

In brief. Unmap a page from every holder; reclaim a region safely. **Built (frame scope), §13.**

Why it matters. safe teardown, a TCB property

Built (milestone 13), scoped to frame revocation; see DECISIONS §13. The full derivation tree is
deferred, the way the argument earlier in the roadmap predicted: revoke-all-derivatives serves the
reclamation triggers, and subtree granularity waits for a driver. The rest of this block is the
proposal it was built from.

Deliverable. A capability-derivation tree and a recursive `revoke` that unmaps an object from
every holder, so authority can be retracted from a live peer and a page can finally be reclaimed.

Why. The deepest thing left in the capability model, and it unblocks everything about
reclamation. `untyped::destroy` already exists, dead, as a tripwire: today frames are spend-only and
never reused, which is the *only* reason teardown's dangling mappings are safe rather than a
use-after-free.

Prior art. seL4's CDT plus recursive revoke, a first-class kernel object there.

Blocking precondition. design/open-design-ideas.md (revocation) and
notes/capability-lifecycle.md state the invariant this must not break: no reclamation of any kind
until revocation lands. This milestone is that work, and the precondition is why it comes before
14.

## Follow-on

- **Milestone 14.** Reclamation itself, which this milestone was the blocking precondition for.
  Frames were spend-only until revocation existed; 14 removes the kernel heap and retypes objects
  out of untyped, which is the reclamation this block made safe.
- **Refused.** The full seL4-style capability-derivation tree, and with it subtree granularity
  (revoke Bob's copy while keeping Alice's). `design/decisions/13-frame-revocation.md` argues it as
  a considered terminal design rather than a way-station: revoke-all-derivatives is the
  memory-safety-critical half and is exactly what reclamation wants, nothing on the roadmap needs
  subtree revoke, and if one ever does, the unmap side and the revoke-before-reclaim discipline are
  reused unchanged with only the holders index rebuilt as a tree.
  This refusal is milestone 456 (design/roadmap/456-the-capability-derivation-tree.md), which
  carries it with the condition that would change it.

## Index row

safe teardown, a TCB property
