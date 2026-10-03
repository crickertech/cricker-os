---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: 613
decision_dependencies: 175
machine_requirements: none
specific_machine: none
needs_person: no
---
# 342. The kernel and the `console` server drive one UART from two address spaces, and nothing arbitrates

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 230's block; numbered 2026-09-19 by milestone 433. **Premise re-checked 2026-09-19 and
it holds.** §149 (may the kernel answer on an endpoint) decided how a program *reaches* a console
server and was itself dissolved by §121 (what a device capability is when the device has no page)'s
reopening, which is a different question. `script/swish-check`'s
own `BUGS` still describes the interleaving as a live defect in the system rather than in the script,
and milestone 243 (a machine with no serial port has no way to say anything)'s `BUGS` still records
that it "has its own home", which is this block.

**Corrected 2026-09-27**: this block's earlier text said "nothing in `design/decisions/` answers
it." That was true on 2026-09-19 and stopped being true the same day. Milestone 435 (forty-five
milestones are gated on a decision nobody wrote down) raised §175 (where the kernel's own output
goes once userspace owns the console) as this question's actual home, status PROPOSED still.
`decision_dependencies` above pointed at `unwritten` regardless; it now points at `175`.

## Priced, 2026-09-27 (PRICING lane, calef's approval)

[`notes/kernel-console-arbitration-pricing.md`](../../notes/kernel-console-arbitration-pricing.md)
answers §175's two open items: the hardware fact per board, and what a panic costs under each
option. It prices a fifth option calef asked for (release builds go quiet, seL4's shape), and
confirms the interleaving with a captured, byte-exact CI transcript already in-tree
(`xtask/src/swish_check.rs`'s `SHREDDED_CI_TRANSCRIPT`), rather than assuming it.

Highlights: a second port is free under QEMU on all three architectures, and confirmed absent,
unconfirmed, or flatly impossible on the three real boards respectively, worst on xenon where there
is no second serial connector at all. The kernel's collision surface is larger than §175 measured:
two routine gauges, not just fault reports, and one of the two (`capability slots:`) has no
equivalent filter to the one already built for the other gauge after it caused this exact failure
once. The release-only option does not touch CI at all, since CI never builds release.

No option is recommended, per §175's own reasoning and AGENTS.md's rule to give options rather than
a recommendation on an irreversible fork. Still no work on the fork itself: this remains NOT-STARTED
pending calef's ruling on §175.

Where the kernel's own output goes once userspace owns the console is a design
fork rather than a bug to fix, and it is an architect's. The options are genuinely different systems, not
variations on one, and the choice binds every architecture and every future console consumer.

## Decided, 2026-09-27

calef ruled §175: **B, with a panic escape and a fallback**, the same shape as pull request #1423's
system log proposal (`design/decisions/242-a-system-log.md`, now landed and DECIDED). See §175's
"The ruling" for the full record and why A and E lost and C and D are superseded.
`decision_dependencies` stays `175`; the citation now resolves to a DECIDED section instead of a
PROPOSED one.

The ruling names a customer for the kernel's ring: the log service is milestone 613 (a system log
service: the in-memory half), which drains the ring and forwards whole lines to the console. 613
was built on 2026-10-02 (pull request #1494), and this milestone is claimed by lane
`milestone/342-kernel-console-arbitration` (draft pull request #1498) from 2026-10-03 (UTC).

## The drain is a syscall-surface fork, proposed 2026-10-03

§242 (a system log) left how the service reads the kernel's ring to the building lane: a read
method on a new object, or a read-only frame plus a notification. Both change the surface §10 (the
capability-based microkernel process model) governs, so the lane proposes and stops.
[`notes/kernel-ring-drain.md`](../../notes/kernel-ring-drain.md) answers the seven questions and
recommends the frame, which needs no new object type and no new method, keeps the property that no
user pointer crosses the syscall boundary, and matches the machine statistics page's shape. Nothing
past the proposal is built until it is ruled.

**In brief.** Once the `console` server owns the console, two address spaces are writing to the same
UART with no arbitration between them. The kernel writes directly, because a kernel that cannot
print during a fault is a kernel nobody can debug, and the server writes on behalf of userspace. The
streams interleave at byte granularity. Deciding this means saying where kernel output goes: a
second port, a buffer the server drains, a claim the server takes and the kernel respects except in
a panic, or something else.

## Why this matters

It corrupts every bench session on argon, radon and xenon. A serial log is the only thing those
three machines can say, and milestone 216 built a tool whose whole contract is recognising a boot
sequence in that stream. Interleaved bytes break that contract in the least visible way available:
the log is present, it looks like output, and the line the tool is matching on has a kernel message
spliced through the middle of it. A gate that reads a board reads this.

It is also already load-bearing somewhere it cannot be fixed. Milestone 243's `BUGS` points at a
home for this question that does not exist, which is the tell AGENTS.md names for being on too low a
rung: a fact that lives only in a citation to nothing.

## What a lane can do before the ruling

The fork deserves its options priced rather than argued, and none of that needs a decision first.
What the kernel actually writes after userspace takes the console, and when, is a measurable list
rather than an opinion. Whether the boards have a second usable port is a hardware fact per board.
What a buffered path would cost during a panic, which is the case that matters most and the case a
buffer serves worst, is the constraint that probably decides it.

## Where it came from

Milestone 230 (`script/shell-check` is red on `main`) named it while fixing something else: *"Decide
where the kernel's own output goes once userspace owns the console. Today the kernel and the
`console` server drive the same UART from two address spaces with nothing arbitrating, so the
streams interleave at byte granularity. It corrupts every bench session on argon, radon and xenon,
and 243's BUGS points at a home that does not exist."*

## Index row

Once the `console` server owns the console, two address spaces write to the same UART with no
arbitration between them: the kernel writes directly, because a kernel that cannot print during a
fault is a kernel nobody can debug, and the server writes on behalf of userspace, and the streams
interleave at byte granularity. It corrupts every bench session on argon, radon and xenon, where a
serial log is the only thing those machines can say and milestone 216 built a tool whose whole
contract is recognising a boot sequence in that stream; interleaved bytes break that contract in the
least visible way available, because the log is present, it looks like output, and the line being
matched has a kernel message spliced through the middle of it. Deciding it means saying where kernel
output goes: a second port, a buffer the server drains, a claim the server takes and the kernel
respects except in a panic, or something else. Those are genuinely different systems, and the choice
binds every architecture and every future console consumer. A lane can price the options before the
ruling: what the kernel writes after userspace takes the console is a measurable list, whether each
board has a second usable port is a hardware fact, and what a buffered path costs during a panic is
the constraint that probably decides it.
