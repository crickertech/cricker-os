---
status: SUPERSEDED
raised: 2026-09-13
superseded_by: 405
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 399. The six `init` roles in `hello`, which milestone 266's title says should not exist

Superseded 2026-09-19, by milestone 405, which was filed one day after this one by a
lane holding the whole inventory and which takes the same six constants as part of a larger change.
Filed 2026-09-13 as an unnumbered proposal by the lane that finished milestone 266's prose sweep;
numbered and disposed of in one act on 2026-09-19 by milestone 433's drain of the proposal pile,
which is how a proposal reaches a disposition at all. **The premise was checked before it was
closed, and it is still true**: `fixtures/src/hello.rs` declares all six constants and
`kernel/src/user/tests.rs` declares all six again, so the defect this names is live. What makes it
`SUPERSEDED` rather than `NOT-STARTED` is milestone 405's own argument, which this block cannot
answer: after that split the six parents are programs with their own names and these constants do
not exist, so renaming them now spends a naming decision, which is an architect's scarcest resource,
on an interim state. 405 quotes his deferral in the same breath (*"If there is anything left then we
can consider a name for what remains"*). The refusals below are the durable half and are cited from
405, because they refuse names that work will reach for. *(Number provisional until the merge queue
lands it.)*

Nothing gates this because nothing should start it: the work is milestone 405's, and
this block is the record of what was found and what was refused. The gate was `DECISION` while it
was open, on the ground that calef names constants, and that is still true of the names 405 will
reach for.

## The claim in one line

Milestone 266 is titled *"One progenitor, on all three architectures, and `init` stops being a role."*
It is still a role, six times, in `fixtures/src/hello.rs`, and six more times as duplicate constants
in `kernel/src/user/tests.rs`.

## What they actually are, traced rather than assumed

| constant | role | what the role does |
|---|---|---|
| `INIT` | 20 | parses an ELF out of the initrd and builds a child of the same binary |
| `INIT_DEV` | 23 | the same, and delegates the PL011 device capability to the child (19d.2) |
| `INIT_CONSOLE` | 24 | builds the real console server, wires a channel, delegates the UART (19d.2b) |
| `INIT_IRQ` | 25 | builds an interrupt-driven child and delegates an interrupt capability (19d.2b) |
| `INIT_LEAST_AUTHORITY_DEMO` | 28 | builds a demo, passes an argument through `START`, reads the answer (19e) |
| `INIT_COREMARK` | 29 | builds the CoreMark workload and reads the CRC it computed (19e) |

**None of them is the first process.** The role that meant "boot the system" was `INIT_BOOT_ROLE`
(27), and that is exactly the one milestone 266 moved out to `components/src/progenitor.rs`. What is
left is the 19d/19e catalogue, in which `hello` plays the **parent**: it parses, builds, endows,
delegates and collects a report, which is the demonstration that userspace and not the kernel
composes the system.

**They are live.** `kernel/src/user/tests.rs` drives all six through `spawn_hello`, each from its
own duplicate constant (`INIT_ROLE = 20`, `INIT_DEV_ROLE = 23`, `INIT_CONSOLE_ROLE = 24`,
`INIT_IRQ_ROLE = 25`, `INIT_LEAST_AUTHORITY_DEMO_ROLE = 28`, `INIT_COREMARK_ROLE = 29`), and each test
asserts on the word the child reports. Nothing here is dead wiring, and deleting any of it would be a
different proposal.

## The recommendation

`PARENT`, `PARENT_DEV`, `PARENT_CONSOLE`, `PARENT_IRQ`, `PARENT_LEAST_AUTHORITY_DEMO`,
`PARENT_COREMARK`, with the functions renamed to match (`fn parent`, `fn parent_dev`, ...) and the
six duplicates in `kernel/src/user/tests.rs` following.

It is a noun, it is what the role is (the binary being the other end of a build), and it is what the
tests already assert about: *"a child the parent built reported the agreed word."*

## The refusals, which are the valuable half

- **`PROGENITOR_*` is refused: it would be false.** The progenitor is one program under one archive
  entry, and `hello` entered at role 20 is neither. Replacing one wrong name with another wrong name
  is worse than leaving it, because the second one looks decided.
- **`LOADER_*` is refused.** It names one of three things the roles do. `INIT_IRQ` delegates an
  interrupt and `INIT_DEV` hands over a device; the loading is the least interesting part of both.
- **`BUILDER_*` is refused.** `builder` is a program in this tree with its own argument
  (`components/src/builder.rs`), and a role constant sharing that word would put two things behind one
  name. That is the refusal that cost `system_builder` a crate name twice, on 2026-08-01 and
  2026-08-04, and it is recorded in `crates/system_initializer`'s own header.
- **`SPAWNER_*` is refused** for the same reason one level over: `spawner` is milestone 22's
  construction sub-server.
- **Leaving them is a real option**, and it costs nothing mechanical. What it costs is that milestone
  266's own title is false of this file, and that a newcomer meets `INIT => init(dma_phys)` a few
  hundred lines from `PROGENITOR_ENTRY` with no way to tell that the two words name different ideas.

## What it would cost

Small and compiler-checked, which is why this is a decision rather than a project. Six constants and
six functions in `fixtures/src/hello.rs`, six constants in `kernel/src/user/tests.rs`, the
`hello.rs` prose bound to them (about 50 occurrences, deliberately left matching the constants by the
lane that swept everything else), and about 40 more in `kernel/src/user/tests.rs`'s doc comments.
Nothing crosses a wire: the role numbers do not move, so the archive, the manifest and the
measurement table are untouched.

## BUGS

- **The role numbers stay where they are, and they are the thing that is actually fragile.** 20, 23,
  24, 25, 28 and 29 are written down twice, once in `fixtures/src/hello.rs` and once in
  `kernel/src/user/tests.rs`, and nothing gates the two against each other. A rename does not fix
  that and would be a good moment to notice it.
- **This proposal does not argue that six demo roles should exist at all.** Whether the 19d/19e
  catalogue still earns its keep now that the boot role has left it is a separate question, and a
  bigger one.

## Index row

Milestone 266 is titled *"One progenitor, on all three architectures, and `init` stops being a
role"*, and it is still a role six times in `fixtures/src/hello.rs` and six more as duplicate
constants in `kernel/src/user/tests.rs`. Traced rather than assumed, none of the six is the first
process: the role that meant "boot the system" was `INIT_BOOT_ROLE`, which is exactly the one 266
moved out to `components/src/progenitor.rs`, and what is left is the 19d/19e catalogue in which
`hello` plays the parent that parses, builds, endows, delegates and collects a report. The
recommendation was `PARENT_*`, a noun, what the role is, and what the tests already assert about.
The refusals are the durable half: `PROGENITOR_*` would be false, since the progenitor is one
program under one archive entry and `hello` entered at role 20 is neither, and replacing a wrong
name with a wrong name is worse because the second one looks decided; `LOADER_*` names one of three
things the roles do; `BUILDER_*` and `SPAWNER_*` each collide with a program that already exists
here, which is the refusal that cost `system_builder` a crate name twice. It is `SUPERSEDED` by
milestone 405 rather than open, because 405 turns these six parents into programs with their own
names and the constants stop existing, so the rename would spend a naming decision on an interim;
405 carries the refusals forward and picks up the one thing this block found that it did not, which
is that the six role numbers live in two files with nothing gating them against each other.
