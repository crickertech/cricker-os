---
status: PROPOSED
raised: 2026-09-19
---

# 160. What a subshell copies, given that a capability set cannot always be copied

Raised 2026-09-19 by milestone 435 (forty-five milestones are gated on a decision nobody wrote down)'s lane, which found that milestone 52 (subshells without `fork`, and what copying an endowment means) has
carried a `DECISION` gate since it was recorded and names no decision anywhere a reader can open.
The fork itself is calef's and older: the block's own status line says *"calef asked for this to be
captured as a milestone and explicitly asked to design it together."* This file is that ask moved
from a paragraph inside a roadmap block into the place AGENTS.md says an open decision lives.
*(Section number provisional until the merge queue lands it.)*

## What is being decided

What `( commands )` means in a shell with no `fork`. Three sub-questions, in the order they
decide each other:

1. Is the mechanism scoping or isolation, given that Unix conflates them because `fork` was the
   tool it had.
2. If a child is involved, is its endowment derived from the parent's or duplicated from it,
   which is the question with no Unix analogue.
3. What happens when the endowment holds a capability that cannot be copied, which is not a
   corner case: this tree has already proved two such capabilities exist.

## What this tree already does in the analogous case

Milestone 50 answered most of what a subshell is used for, and it is built. Each side of a
pipe, `$( ... )` substitution and `( ... ) &` backgrounding are all spawn-and-grant today
(milestone 50, milestone 48). The residue is `(cd /tmp && make)` and `(umask 077; ...)`, and
`umask` has no subject here because there are no permission bits. So the decision is about
scoping a shell's own mutable state, not about process duplication, and an option priced
against the Unix requirement is priced against the wrong requirement.

The separation this decision would make has been made twice before and was right both times.
milestone 47 separated unlink from revoke in `rm`, and milestone 42's own fork separated what a
filesystem offers from what a caller may assume. Both were the same act: a Unix verb doing two jobs
because one implementation served both.

And the tree already refuses "copy the capability". [§41](0041-endpoint-as-broker.md) (a device is revoked by taking it back) gave
`Frame::REVOKE` take-back semantics on a `DeviceFrame` precisely because a device must never have
two owners, and milestone 23's witness is that the generation never goes backwards. A one-shot
reply capability is consumed by construction. So **"copy the endowment" is not a total function**,
and this is measured rather than feared.

## The options

| | shape | what it costs |
|---|---|---|
| **A** | **Simulate in-process.** The shell saves its own working-directory capability, variables and options, runs the group, restores. | No new mechanism, and correct exactly when the effects are shell-local, which is what `(cd x && y)` is. It cannot undo what a command did to a capability, so it is a lie for the isolation case, and a silent one, which [§42](0042-truthful-filesystem.md) (no silent degradation) forbids. |
| **B** | **A real child shell** with a derived endowment. | Honest isolation. Costs a full spawn for `(cd /tmp && ls)`, and forces question 3 to be answered before anything works. |
| **C** | **Scoped bindings**, `with cwd = /tmp { ... }`, and no subshell at all. | Says what it means rather than reaching for duplication. Covers the scoping use completely and the isolation use not at all. Diverges from Unix spelling, which milestone 47's rule allows only when the divergence earns it. |
| **D** | **Hybrid**: scoping by binding, isolation by an explicit verb. | Two mechanisms, each doing one thing, which is the shape §42 and milestone 47 both converged on. Two syntaxes to learn instead of one. |

Recommendation: D, and derivation rather than duplication inside it. The argument is not that
it is less work; it is more. It is that this tree has twice found a Unix verb doing two jobs and
has twice been right to split them, and that the split is what makes question 3 answerable at all:
a scoping construct never touches a capability, so the non-duplicable case simply does not arise
there, and an explicit isolation verb is the one place that has to answer it.

And derivation beats duplication on machinery that already exists. If a child's capabilities
are derived from the parent's, [§16](0016-object-revocation.md) (reclaim the objects a process built)'s revocation and the derivation tree
already give "destroying the child revokes exactly its copies", and [§40](0040-no-reaper-of-last-resort.md) (a supervisor's death is its subtree's death) makes the cleanup automatic. Duplication would need bookkeeping
for the same result. This is the same shape as §92's caretaker-lifetime answer, where supervision
was both the better option and the smaller one once it was checked.

## Whether the premise is true

Partly, and the part that is false matters. The block's framing is "what replaces `fork`", and
the honest answer is that `fork` has a third use that has nothing to do with shells: Redis
`BGSAVE`, the Android and Chrome zygotes, and PostgreSQL's backend-per-connection all want a copy
of a running address space, frozen or warm. Checked 2026-08-17 and unchanged: there is no
copy-on-write machinery in `kernel/src` or `crates/`, and no address-space-copy method on the ABI.

**That is a separate decision and should not be answered here.** It is a kernel question that would
exist with no shell in the tree, and folding it in would design a shell feature around a mechanism
the shell does not need. The two benchmark claims are both true at once and the pairing is the
honest form: `spawn_el0` at ~7.7 us beats Linux `fork`+`exit` at ~19.7 us (notes/benchmarks.md) at
making a *process*, and this system cannot do the thing those three programs use `fork` for.

## How reversible it is

**The syntax is the expensive half, not the implementation.** Option A is a morning and can be
replaced. Whether `( ... )` keeps its Unix spelling while meaning something materially different is
a name, which AGENTS.md puts in the irreversible column: it lands in a reader's head and in every
script anyone writes. So the spelling deserves the deliberation and the mechanism behind it does
not.

## What is blocked until this is answered

Milestone 52, and nothing else. Its sequencing dependency on milestone 50 is satisfied: 50 is
BUILT, which is what removed most of the requirement and changed what is left.

**Not blocked:** address-space duplication, which wants its own block and its own trigger. The
trigger is stated rather than scheduled: the day a candidate workload needs a consistent snapshot
of its own memory, or per-child warm start badly enough to measure. Redis is the obvious candidate
and is on nobody's list.
