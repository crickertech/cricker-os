---
status: NOT-STARTED
raised: 2026-10-03
promoted_from: section-230-records-that-a-plain-send-carries-its-badge
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 711. §230 records that a plain `SEND` carries its capability's badge

Promoted from `design/roadmap/proposals/section-230-records-that-a-plain-send-carries-its-badge.md` on 2026-10-03 (UTC). The number 711 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by the 2026-10-03 security audit
(`design/audit-reports/2026-10-03-eight-constants-and-thirteen-components.md`, finding 1). It is a
record owed, not code: the integrator adds an amendment to §230 (badged endpoint capabilities) at
merge, the way §-sections are minted, and this file is retired when the amendment lands.

## What changed on the wire, and where it is written down

Milestone 613 (a system log service: the in-memory half) changed what `RECV` returns in `x3` for an
ordinary message. Before it, `x3` was always `0` for a plain `SEND` and the badge rode only on
`CALL` and `SEND_CAP` (§230's table). Since commit `4feaf0dc2` (2026-10-02), `wide` in
`kernel/src/sched.rs` writes the badge of the capability the sender invoked into word 3, and
`abi::rendezvous::RECV`'s doc says so. calef ruled it on pull request #1494, and the block of
milestone 634 (a plain SEND received by RECV_CAP never hands the receiver a sender-chosen slot)
and the risk 7 record both cite "calef's ruling on #1494" as the fix for the second escape
that audit confirmed (a bound client's plain `SEND` arriving as badge `0`, which `subtree_scope`
reads as root).

So the semantics of a syscall method changed, every program is now written against the new
meaning, and the only records are a PR comment, two roadmap blocks and a doc comment in
`crates/abi`. CLAUDE.md's rule for the syscall surface is that each method's semantics are recorded
in `design/decisions/`, not just in code, and §230 is where a reader of the badge contract looks.
`grep -n '613\|plain SEND' design/decisions/230-*.md design/decisions/242-*.md` finds nothing.

## What the amendment should say

One dated paragraph under §230, in the shape the correction in §74 (audits run on change, not on
the calendar) took:

- The badge is delivered on every receive, not only `CALL` and `SEND_CAP`: `RECV` and `RECV_CAP`
  both return it in `x3`, `0` when the capability was unbadged.
- It is the kernel's word on every path: the syscall layer reads it from the capability in the
  caller's table, never from a register the sender filled (`kernel/src/syscall.rs`, the `SEND`
  arm, `sched::ipc_send_badged`).
- The reason: §242 (a system log) stamps every record from the writer's badge, and a byte-sink
  writer speaks a plain `SEND`; without this the badge a spawner minted was dropped on the one path
  it was minted for. The second reason is the confinement one from milestone 634's block: a bound
  filesystem client that `SEND`s rather than `CALL`s on its badged capability arrived as badge `0`,
  which `crates/subtree_scope` reads as an open binding.
- A death message is told apart by its first word, as before; nothing in the tree reads `x3`
  before checking `x0` (the 2026-10-03 audit read every `recv_fault` caller).

## What it costs

A paragraph. The code, the tests
(`a_plain_send_arrives_with_its_capabilitys_badge_on_recv_and_recv_cap`) and the abi doc are
already on `main`. What is missing is the record a future reader of §230 would act on.

## What is blocked until it lands

Nothing in code. What is at risk is the next lane that reads §230's table, sees `SEND` without a
badge, and writes a server that treats a non-zero `x3` on `RECV` as a fault message or a
corruption, as the pre-613 contract allowed.

## Index row

Milestone 613 changed what `RECV` returns in `x3` for a plain `SEND`, and the only records were a pull request comment, two roadmap blocks and a doc comment. Owed: an amendment to DECISIONS §230 (badged endpoint capabilities) recording it.
