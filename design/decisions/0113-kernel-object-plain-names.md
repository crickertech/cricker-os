---
status: AMENDED
raised: 2026-08-23
decided: 2026-08-23
ratified_by: calef
---

# 113. Eleven kernel object and identifier names move from contraction or borrowed jargon to the plain, standard term

(the 2026-09-13 amendment below ends the crate-name exemption.)

**Decided 2026-08-23 (calef), after repeatedly having to ask what `Aspace`, `Endpoint`,
`Untyped`, and `Tcb` meant in the course of ordinary conversation about this tree: *"I have
repeatedly had to ask what these terms mean because they're often used without context. Thus
they're clearly not working."*

## The question

Five kernel object type names, plus six identifiers or pointers derived from them, were checked one
at a time
against whether a reader needs to
already know a specific external system (seL4, in every case) to recognize them on sight. The
check that mattered was not "is this a real term of art somewhere" but the plainer one calef named
directly: does the architect, working in this tree daily, still have to ask what it means. Prior
art turned out not to answer that question, because seL4, Mach, Windows and QNX disagree with each
other on several of these, so no single borrowed word is broadly recognized either.

## The decision

Eleven renames, all of the same shape: replace a contraction or a borrowed short word with the plain,
standard English compound the field already uses for the concept, so a reader who knows the concept
recognizes it immediately and a reader who doesn't gets a running start from the words themselves.

| Was | Becomes | Kind of fix |
|---|---|---|
| `Aspace` | `AddressSpace` | contraction spelled out |
| `Untyped` | `MemoryRegion` | seL4's word replaced with the tree's own working name, disambiguated |
| `Endpoint` | `Rendezvous` | seL4's word replaced with the property that actually matters |
| `Frame` | `PageFrame` | bare word disambiguated with the standard OS qualifier |
| `Tcb` | `ThreadControlBlock` | acronym spelled out |
| `EpId` | `RendezvousId` | follows `Endpoint`'s rename; names what it identifies |
| `Tid` | `ThreadId` | acronym spelled out, follows `Tcb`'s rename |
| `TcbPtr` | `ThreadControlBlockPointer` | follows `Tcb`'s rename; `Ptr` spelled out too |
| `TidSet` | `ThreadIdSet` | follows `Tid`'s rename |
| `EpFail` | `RendezvousFailure` | follows `Endpoint`'s rename; `Fail` spelled out too |
| `FreeVas` | `FreeAddressSpace` | `Vas` ("virtual address space") is `Aspace`'s own concept under a second contraction |

## Why each one, with the in-tree evidence that decided it

`EpId` -> `RendezvousId`, `Tid` -> `ThreadId`, `TcbPtr` -> `ThreadControlBlockPointer`,
`TidSet` -> `ThreadIdSet`, `EpFail` -> `RendezvousFailure`. All five are companions of a
renamed object rather than independent decisions: they identify, point at, collect, or report on
an `Endpoint`/`Rendezvous` or a `Tcb`/`ThreadControlBlock`. Leaving a companion abbreviated after
spelling out the object it names would just move the same problem one field over, and the same
test applies to the suffix as to the root: `Ptr` and `Fail` have no more external constraint than
`Tcb` did, so they are spelled out too (`Pointer`, `Failure`), not left as a shorter compromise.

**`FreeVas` -> `FreeAddressSpace`.** `Vas` is "virtual address space," `Aspace`'s own concept
reached a second time through a second, unrelated contraction (`kernel/src/thread.rs`'s
`FREE_STACK_VAS` / `struct FreeVas`). Once `Aspace` becomes `AddressSpace`, a reader meeting `Vas`
two files over would have to learn that it names the same thing under a different abbreviation;
folding it into the same rename removes a second name for one concept rather than leaving one.

**`Aspace` -> `AddressSpace`.** No rationale for the contraction exists anywhere in the tree
(checked: commit history, `notes/`, `DECISIONS.md`). It is also not seL4's term: seL4 calls the
equivalent object `VSpace`, so `Aspace` is neither the full English word nor borrowed vocabulary,
just an unexplained local abbreviation. Its own sibling variants in `kernel/src/cap.rs`'s `Object`

**`Aspace` -> `AddressSpace`.** No rationale for the contraction exists anywhere in the tree
(checked: commit history, `notes/`, `DECISIONS.md`). It is also not seL4's term: seL4 calls the
equivalent object `VSpace`, so `Aspace` is neither the full English word nor borrowed vocabulary,
just an unexplained local abbreviation. Its own sibling variants in `kernel/src/cap.rs`'s `Object`
enum (`Endpoint`, `Untyped`, `Frame`, `Reply`) are unabbreviated; `Aspace` was the outlier before
this decision touched any of them.

`Untyped` -> `MemoryRegion`. `Untyped` is exactly seL4's word (`seL4_Untyped_Retype`,
confirmed against seL4's own docs), so a reader with seL4 background recognizes it and nobody else
has anywhere to start, because Linux, Mach and Windows expose no equivalent concept under any name.
Meanwhile this tree's own implementation never actually says "untyped" once you're inside it:
`kernel/src/untyped.rs`'s table is `REGIONS`, its cap is `MAX_REGIONS`, its accessor is
`region_bounds()`, every parameter is `region: u64`, there is a whole crate
(`crates/regions`, whose own doc comment calls itself "the untyped-region accounting"), and five or
more `notes/` files describe "the region model." Contributors independently reached for "region"
the moment they weren't being careful about the official name, which is stronger evidence of what
actually reads naturally here than any borrowed vocabulary. Bare `Region` was considered and
rejected: `crates/compositor` already uses "region" for a damaged screen rectangle
("keeping a region list," "the damaged region is window 1's"), a real in-tree collision.
`MemoryRegion` keeps the word the tree already converged on and removes the collision.

`Endpoint` -> `Rendezvous`. `notes/ipc-naming.md`'s own "Family resemblance" section records
that Mach calls this object a `port`, QNX calls it a `channel`, and seL4 calls it `Endpoint` --
three influential microkernels, three different words, so `Endpoint` privileges a reader with
seL4 background over one from Mach or Windows (whose ALPC ports descend from Mach's vocabulary)
with nothing to recommend it over the alternatives except which kernel got asked first. `Channel`
was considered and rejected on a concrete collision: Go's `chan` is probably the most common
thing "channel" means to a working developer today, and Go channels can be buffered, which
directly contradicts the strict rendezvous guarantee (`notes/ipc-naming.md`: "the only client
parked in `RECV(reply)`... is the one it just served") that is the entire reason this object's
semantics matter. `Port` was rejected for colliding with this tree's own hardware and networking
vocabulary. `Rendezvous` names the actual guarantee -- both sides must be present, nothing is
buffered -- rather than any kernel's incidental word for it, has real standing outside this tree
(Ada's concurrency model calls its synchronous meet-both-sides-required primitive a rendezvous),
and it is already the word this tree's own prose reaches for independently:
`crates/compositor/src/lib.rs:705` says, of its own IPC, "is a rendezvous, so a compositor that
narrated each frame would block."

`Frame` -> `PageFrame`. `Frame` is generic on its own and collides in-tree the same way bare
`Region` did: `crates/compositor` uses "frame" throughout for a rendered screen update ("per
frame," "the frame's damage," "a compositor that narrated each frame"), which is an unrelated
concept from the kernel's `Frame` object (one physical page, DECISIONS §102). `PageFrame` is not
invented vocabulary -- "page frame" is the standard OS term for a physical page in a virtual
memory system, used across the field regardless of kernel lineage -- so the fix is the same shape
as `Aspace` and `Tcb`: stop truncating the standard term.

`Tcb` -> `ThreadControlBlock`. The clean case: "Thread Control Block" is standard OS
terminology taught in essentially every operating systems course, so a reader who already knows
the acronym loses nothing recognizing the spelled-out form, and a reader who doesn't gets three
ordinary English words with real content ("thread," "control," "block") instead of three letters
with none. Unlike `elf`, `pci`, `dtb`, `gpt`, `ipc`, `paging`, `glob` and `asid` -- the crate-naming
rules' list of abbreviations kept because they are external wire-format or spec names this tree
cannot rename without becoming incompatible with what the rest of the world calls them -- nothing
external constrains what this tree calls its own internal thread object. There is no format
called "TCB" anywhere outside an OS textbook's prose.

## The general principle, stated so it applies to names not yet found

**A borrowed abbreviation is not accessible just because it is correct.** Prior art (seL4's
vocabulary, in every case here) proves a name is defensible, not that it works, and "works" is
measured by whether a reader needs to ask, not by whether the word has a pedigree. The distinction
that separates a name worth keeping abbreviated from one that is not: is the abbreviation forced
by something external the tree must interoperate with (`elf`, `pci`, `dtb`, `gpt`, `asid` all name
a wire format or hardware spec verbatim, and renaming them would just be wrong), or is it a purely
internal convenience this tree chose and could equally have spelled out. `Tcb`, `Aspace`,
`Untyped`, `Frame` and `Endpoint` were all the second kind.

## The sweep's first pass: `kernel/src`, checked and left alone

A systematic sweep of every top-level `struct`/`enum` name in `kernel/src`, prioritized over
`crates/` per calef's direction (2026-08-23), turned up two groups that were checked against the
same external-constraint test and are **not** part of this decision:

- Real hardware and protocol names, exempt like `pci`/`dtb`/`elf`: `Gic` (ARM's own name for
  the Generic Interrupt Controller), `Iommu` (the industry name for the hardware feature, not a
  local coinage), `Ns16550` and `Pl011` (literal chip and ARM PrimeCell part numbers), `Nvme` (the
  protocol's actual name), `Smmu` (ARM's own name for its IOMMU), `PciNvmeDevice`,
  `PciVirtioDevice`, `VirtioMmioDevice`. Renaming any of these would make them wrong, not clearer.
- **Left as borderline, not decided here**: `IrqSafeMutex`/`IrqSafeGuard` (IRQ) and `RamMap`
  (RAM). Both acronyms are arguably as universally known as any full English phrase would be,
  closer to `CPU` than to `Tcb`; flagged rather than renamed pending calef's read.

`crates/` has not yet been swept; that is separate, ongoing work.

## What this does not decide

The mechanical rename itself: each touches a real, measured surface (`Endpoint` alone is 81
occurrences across 23 `.rs` files; the others are unmeasured but comparable, being equally
foundational kernel object types) and is left to whoever executes it, tracked as its own piece of
work per name. This decision settles the eleven names, not the rename lane(s) that carry them out.

The `crates/` half of the systematic sweep, and the `IrqSafeMutex`/`RamMap` borderline call above,
are separate, ongoing work; findings land as their own entries once checked, not folded into this
one.

## What it unblocks

Eleven renames the tree can now execute without re-litigating the name each time: `Aspace` ->
`AddressSpace`, `Untyped` -> `MemoryRegion`, `Endpoint` -> `Rendezvous`, `Frame` -> `PageFrame`,
`Tcb` -> `ThreadControlBlock`, `EpId` -> `RendezvousId`, `Tid` -> `ThreadId`, `TcbPtr` ->
`ThreadControlBlockPointer`, `TidSet` -> `ThreadIdSet`, `EpFail` -> `RendezvousFailure`, `FreeVas`
-> `FreeAddressSpace`.

## Amended 2026-08-25: a twelfth name, checked ad hoc rather than by the promised `crates/` sweep

`CSpace` -> `CapabilityTable`. `capability::CSpace<Object, CSPACE_SLOTS>` (`crates/capability`,
aliased as `kernel/src/cap.rs`'s `CSpace`) sits in `crates/`, which "The sweep's first pass" section
above says explicitly has "not yet been swept." This name did not wait for that sweep: the architect
raised it directly, it was checked against this decision's own test on the spot, and he ratified it
before the systematic pass reached it. That is the honest shape of what happened -- one name decided
out of band, not a finding from a completed `crates/` sweep -- and it is recorded as such rather than
folded silently into "the eleven" above, whose count and table stay exactly as they were decided on
2026-08-23.

Why it passes the same test. `CSpace` is seL4's own contraction (a tree of `CNode`s, in seL4's
own vocabulary; this tree uses a flat sixteen-slot array instead, `crates/capability/src/lib.rs`'s
own module doc explains the divergence), so it fails the same question every one of the eleven
above failed: nothing external -- no wire format, no hardware spec -- constrains what this tree
calls its own capability table. Notably, `CSpace` was not among the four names calef's original
complaint named (`Aspace`, `Endpoint`, `Untyped`, `Tcb`) -- it had not come up yet, not because it
was judged and kept.

How the replacement was picked, and why it changed once. calef's first answer, in conversation,
was "Rename it to CapabilitySpace" -- the direct spelled-out form, the same shape as `Aspace` ->
`AddressSpace`. Before building it, the maintainer ran the same evidentiary check this decision's
own `Untyped` -> `MemoryRegion` entry used: what does this tree's own prose already reach for.
`crates/capability/src/lib.rs`'s own module doc opens with "A thread's capability table," not
"space," before it ever justifies the flat-array implementation. A grep across the tree found
"capability table" used independently 17 times, "capability space" 7 times, and a third candidate,
"capability array" (closer to the concrete Rust type, `[Option<Cap<O>>; N]`), 0 times anywhere.
Given that, calef reconsidered and picked `CapabilityTable`: "Use CapabilityTable, redirect the two
lanes." Recorded here rather than only as `CapabilitySpace`, because the reconsideration is real
history and this decision's own culture keeps a correction on the record instead of quietly
overwriting the first answer (see `AGENTS.md`'s "correct yourself loudly").

**The cost this one carries that the original eleven did not.** Those eleven were scoped to the
identifier surface; this tree's own descriptive prose about "an address space" or "a thread" was
deliberately left alone. `cspace`, lowercase, is different: it is not a generic English phrase
with an ambiguous descriptive use elsewhere, it is shorthand for this specific object everywhere
it appears, including in this project's own governance document. `cspace` (case-insensitive)
appears in roughly 74 markdown files across this tree, and once in `AGENTS.md` itself (the
Steward section: "the sixteen-slot cspace"). Whoever merges the build lane's rename needs to
update that line by hand -- a developer lane may not edit `AGENTS.md` under this project's own
rules, so that one file is a maintainer follow-up, not part of the lane's own diff.

**What this does not decide.** The mechanical rename itself, same as the original eleven: a
separate lane carries it out, touching `crates/capability`, `kernel/src/cap.rs`'s alias, and
prose throughout the tree. The rest of the `crates/` sweep remains separate, ongoing, and
unstarted by this amendment.


## Amended 2026-09-13: the crate-name exemption is ended, and this decision already contained the argument

calef, working the unratified worklist, ruled that all six exempt acronym crates expand. The
exemption this decision codified at "Unlike `elf`, `pci`, `dtb`, `gpt`, `ipc`, `paging`, `glob` and
`asid`" no longer holds for the acronyms in it.

This decision stated the winning argument and then applied it to only half its list. `Tcb` was
expanded here because *"a reader who already knows the acronym loses nothing recognizing the
spelled-out form, and a reader who doesn't gets three ordinary English words with real content
instead of three letters with none."* That is precisely the asymmetry calef named on 2026-09-05 as
the general acronym test, and nothing in it is special to `Tcb`. Every name in the exempt list
satisfies it too.

What held the exemption up was a claim that turns out to be false. The sentence justifying it
says these are *"names this tree cannot rename without becoming incompatible with what the rest of
the world calls them."* A crate name is a directory and a Cargo package, not a wire format. Renaming
`crates/pci` does not change one byte this tree exchanges with anything: the spec's own identifiers
live in the code, in constants and register names, where they are untouched and must stay. Checked
rather than assumed. The interoperability the exemption protects was never at risk from the
directory.

### The six, as ruled

| Today | Becomes | Expansion |
|---|---|---|
| `pci` | `peripheral_component_interconnect` | Peripheral Component Interconnect |
| `dtb` | `device_tree` | the blob is the encoding, not the subject |
| `gpt` | `guid_partition_table` | GUID Partition Table |
| `asid` | `address_space_identifier` | Address Space Identifier |
| `ipc` | `rendezvous` | follows this decision's own `Endpoint` -> `Rendezvous` |
| `elf` | `executable_format` | Executable and Linkable Format, minus a half this crate refuses |

`ipc` is the one that closes a loop rather than opening one. This decision renamed the kernel
object `Endpoint` to `Rendezvous` and left the crate alone because of the exemption. With the
exemption gone the crate follows the type, which is what the original rename wanted and could not
have.

**`gpt` carries a second argument that does not depend on the acronym test at all.** In 2026 the
name reads as something else entirely to any reader arriving, which is milestone 265's complaint
about `proto` and prototype in different clothes.

**`elf` is the one that does not take its standard's full name, and the reason is in the crate's own
first line.** It is *"an ELF64 loader's front half: parse, validate, and hand back the segments to
map"*, and it does no linking at all: no dynamic linking, no relocations, no interpreter, no PIE,
each refused deliberately because *"we would rather say no in eleven lines than maybe in a
thousand."* So "Executable and Linkable Format" would name a capability this crate exists to decline,
which is worse than an acronym. calef ruled **`executable_format`** on 2026-09-13.

It also matters less here than elsewhere. `asid` is the vocabulary a reader uses to discuss the
thing; `elf`'s spec identifiers live in the code (`Elf<'a>`, `e_machine`, `ET_EXEC`, `p_type`) and
stay whatever the directory is called. The name carries less weight when the spec's own words are
still on the page beneath it.

`paging` and `glob` are unaffected. They were in the exempt list and are not acronyms, so no
expansion test reaches them. The list conflated two kinds of name and only one kind is ruled here.

### Not performed here, and that is deliberate

611 file-references across the six, and `ipc` is load-bearing. `design/naming.md` already says the
acronym sweep is its own milestone for that reason, and
`design/roadmap/0388-an-acronym-sweep-the-tree-can-do-at-once.md` exists because answering one
name at a time is the failure mode. This amendment answers the *question* for all six at once, which
is what that proposal asked for; the sweep performs them. Doing it here would also collide with
milestone 265, which is already renaming 349 files.
