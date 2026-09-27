---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 342. The kernel and the `console` server drive one UART from two address spaces, and nothing arbitrates

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 230's block; numbered 2026-09-19 by milestone 433. **Premise re-checked 2026-09-19 and
it holds.** Nothing in `design/decisions/` answers it: §149 (may the kernel answer on an endpoint)
decided how a program *reaches* a console server and was itself dissolved by §121's reopening, which
is a different question. `script/swish-check`'s own `BUGS` still describes the interleaving as a live
defect in the system rather than in the script, and milestone 243's `BUGS` still records that it "has
its own home", which is this block.

Where the kernel's own output goes once userspace owns the console is a design
fork rather than a bug to fix, and it is an architect's. The options are genuinely different systems, not
variations on one, and the choice binds every architecture and every future console consumer.

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
