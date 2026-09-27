---
status: NOT-STARTED
raised: 2026-09-17
promoted_from: a-block-roster-that-can-name-an-nvme-disk
milestone_dependencies: none
decision_dependencies: 193
machine_requirements: none
specific_machine: none
needs_person: no
---
# 421. The block roster cannot name an NVMe disk, and the reason it could not has just gone away

Promoted from the proposal `a-block-roster-that-can-name-an-nvme-disk`,
filed 2026-09-17 by the milestone 261 lane (the EL0 NVMe server), which closed the question this was
waiting on and deliberately did not take the work, because the work is a wire shape and that is the
expensive category. *(Number provisional until the merge queue lands it.)*

calef, 2026-09-19, correcting the token this block carried on promotion. The
hardware is not the constraint and never was: QEMU's NVMe is attached on every leg of all three
runners already (`NIFE_NVME`), and the surveyor's two clients run there today. What stops a lane is
§193 (what a block-roster entry calls an NVMe disk, and whether it carries more than virtio does), written up on 2026-09-19 when calef asked
whether this gate had a decision behind it and the answer was no: the ask lived only in this block's
own *What is needed from calef* section, one rung above a chat message. It is the expensive category
rather than a preference:
**a transport kind is a wire value two programs read**, so shipping the wrong spelling cannot be
un-shipped, and whether an NVMe entry carries a namespace id or a controller identity is the same
kind of question. §86 routed both to him rather than to a lane, and milestone 261's own lane closed
the question this block was waiting on and then **deliberately did not take the work**, for exactly
this reason.

**The token was `NONE` from 2026-09-17 to 2026-09-19**, which is recorded rather than quietly fixed
because it is the failure mode the gate vocabulary exists to prevent. `NONE` put the block on
`script/roadmap --ready`, the list a lane picks work from, while its own last section said a decision
was owed; a lane taking it would have reached the wire shape and either stopped or guessed. Milestone
433's slice 4 found it while promoting the file, flagged it, and correctly did not change a gate,
because changing one was not in that pass's method.

**Premise re-checked 2026-09-19 and still true.** `crates/block_roster` still encodes exactly two
transport kinds, `TRANSPORT_MMIO` and `TRANSPORT_PCI`, in the four bytes at offset 4 of an entry,
and `transport_name` still matches those two alone. There is no NVMe kind and no entry the surveyor
could fill in from
`kernel/src/user/non_volatile_memory_express_service.rs`.

## What the work is

`block_roster` is the read-only listing a process holds when it may know *what block devices exist*
without holding any of them (milestone 57, `notes/block-devices.md`). It has a transport kind for
virtio and none for NVMe, so `disk_surveyor` cannot list the machine's NVMe disk at all: the one
device in this tree whose driver is now a confined process is the one the roster cannot see.

The work is a transport kind, the roster entry that carries it, and the wiring that fills it in from
`kernel/src/user/non_volatile_memory_express_service.rs` the way the virtio disks are filled in from theirs.

## Why now, and why not before

[DECISIONS §86](../decisions/86-el0-nvme-driver.md) listed this under "what is blocked until it is
answered", with the reason stated precisely: *"small, but its wire shape depends on who owns the
controller."* That dependency was real. A roster entry for a kernel-resident driver and one for a
confined EL0 server are different entries, because what a holder of the roster can then *ask for* is
different: with the driver in the kernel there is no endpoint to name, and with the driver at EL0
there is.

**Milestone 261 answered it.** A process owns the controller's data plane, it serves
`filesystem_protocol::blk` on a request endpoint, and that endpoint is exactly the shape the roster's
virtio entries already point at. So the wire shape is decidable now and was not before.

## Why milestone 261's lane did not take it

Two reasons, and the second is the one that decides.

It is off that milestone's scope, which is the driver leaving the kernel. And a roster entry is
**something two programs agree on**, which AGENTS.md's *move fast on what can be undone* tenet puts
in the irreversible category along with names, dependencies and the syscall surface. §86 names it
directly as "the genuinely expensive thing" in that section, above the capability question. A lane
that invented a transport-kind encoding on its way past would be committing the tree to it.

## What it would settle

Whether the surveyor's authority split survives a second kind of disk. The claim milestone 57 makes
is that listing what exists and holding one of them are different powers; today that claim has only
been exercised against devices of one kind, behind one transport, all of them virtio. An NVMe entry
is the first test of whether the roster's shape is about *block devices* or about *virtio*.

## What is needed from calef

The transport kind's spelling, since it is a wire value two programs read, and whether an NVMe entry
carries anything a virtio one does not (a namespace id, a controller identity). Both are the kind of
question §86 already routed to him rather than to a lane.

## Index row

`block_roster` is the read-only listing a process holds when it may know what block devices exist
without holding any of them, and it has a transport kind for virtio and none for NVMe, so
`disk_surveyor` cannot list the machine's NVMe disk: the one device whose driver is a confined
process is the one the roster cannot see. §86 listed this as blocked on who owns the controller,
because a roster entry for a kernel-resident driver and one for a confined EL0 server differ in what
a holder may then ask for, and milestone 261 answered it by putting the data plane in a process that
serves `filesystem_protocol::blk` on a request endpoint, which is the shape the virtio entries
already point at. What it settles is whether the roster's shape is about block devices or about
virtio, since milestone 57's claim that listing and holding are different powers has only ever been
exercised against one transport. The transport kind's spelling is a wire value two programs read and
is an architect's, along with whether an NVMe entry carries a namespace id or a controller identity.
