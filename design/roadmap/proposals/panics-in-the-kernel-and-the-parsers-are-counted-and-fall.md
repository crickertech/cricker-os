---
status: PROPOSED
raised: 2026-10-09
milestone_dependencies: none
decision_dependencies: 61
machine_requirements: none
specific_machine: none
needs_person: no
---
# Panics in the kernel and in the parsers of untrusted bytes are counted, and the count falls

Raised by a maintainer session on 2026-10-09 (UTC), one of five lint proposals asked for together.
Its numbers are in [notes/lint-census-2026-10-09.md](../../../notes/lint-census-2026-10-09.md),
which says how each was taken. This one also specifies the lint ratchet the other four reuse.

Reuse: clippy's own `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`
and `indexing_slicing`, configured rather than written. The ratchet copies
`helpers/file_length_ratchet.py`'s rules, from milestone 841 (a ratchet on Rust file length). No
dependency is added.

## The rulings, 2026-10-09 (UTC)

calef ruled on two of this proposal's questions on 2026-10-09 (UTC).

- Open question 1, the untrusted-input scope: all 32 crates the note lists. Device drivers count as
  untrusted, so a device is a less-trusted party here, IOMMU or not.
- The thresholds: panic counts are frozen per file at today's value and may only go down. There is
  no zero-by-date target, so done criteria 2 and 3 below were rewritten to match.

## The problem

A kernel panic halts the machine (`kernel/src/panic.rs` ends in `arch::halt`). So a panic that a
confined process can reach with a syscall argument is a denial of service against every other
process. The same holds one level up for a server that panics on a client's bytes. Milestone 779
(fuzz the surface a confined process can reach) found one: `AddressSpace::LIST` followed a
caller's cursor and took a kernel data abort. Nothing counts this class today.

Measured over non-test code (the note's scope: no `#[cfg(test)]` items, no `tests/`, no
`vendor/`, no benches, no host tooling):

- The kernel carries 459 `unwrap_used` and `expect_used` hits, 44 `panic`, 3 `unreachable`, 1
  `unimplemented` and 0 `todo`. The request's grep said 814 and 77; the gap is test code.
- 251 of the 418 `expect_used` are in the boot-time service builders,
  `kernel/src/user/*_service.rs`. A sample of 25 kernel sites read as boot construction or a stated
  invariant, and none sat on a syscall argument path. Most of the kernel's count is a panic that is the right answer.
- The kernel has 282 `indexing_slicing` hits.
- The 32 crates the note reads as parsing less-trusted bytes carry 24 panic hits between them, and
  719 `indexing_slicing` hits. Indexing is where their exposure is.

Of those 32 crates, 20 have no target under `fuzz/`:

- Disk and image formats: `file_allocation_table`, `manifest_note`, `boot_slot`.
- Network: `network_time_protocol`, `socket_protocol`, `name_resolution_protocol`.
- Device-supplied bytes: `machine_discovery` (ACPI tables), `pci`, `usb`,
  `extensible_host_controller_interface`, `e1000e`, `designware_ethernet`,
  `designware_mobile_storage`, `non_volatile_memory_express`, `virtio`.
- Peer-process contracts: `graphics_protocol` (reached only through `compositor`'s target),
  `login_protocol`, `credential_protocol`, `supervision_protocol`, `swap_protocol`.

The 12 with a target are `redoxfs_server`, `nifefs`, `globally_unique_identifier_partition_table`,
`package_archive`, `elf`, `domain_name_system`, `http_response`, `device_tree_blob`,
`filesystem_protocol`, `system_log_protocol`, `byte_sink_protocol` and `compositor`.

## The mechanism, and its rung

The highest rung that fits is two. Rung one, making the panic unrepresentable, is what a
`get()`-and-`?` rewrite does at each site, and the ratchet is what drives those rewrites without a
sweep.

The lints cannot go in `[workspace.lints]` today. §61 (a lint is adopted on evidence from this
tree) holds that `script/lint` runs `-D warnings`, so adding a lint commits to fixing every
violation first. `undocumented_unsafe_blocks` went in that way, after all 205 sites were fixed;
it was never a count-down ratchet. The count-down shapes in this tree are the unsafe census
ceilings of milestone 134 (the register of measures), the prose ratchet of milestone 586 (a prose
ratchet in lint) and the file-length ratchet of milestone 841.

So the lint ratchet is a separate pass, in the file-length ratchet's shape:

1. `script/lint` runs clippy once more per configuration it already lints, with the ratcheted lints
   at `-W` and `--message-format=json`. The census's five passes took 14 seconds with dependencies
   built.
2. A helper (name provisional, minted by the lane) counts hits per file per lint. A site compiled
   for several architectures counts once, so a parity gap cannot hide a hit.
3. A committed baseline TSV holds one row per file and lint over zero. It follows
   `helpers/file_length_ratchet.py`'s rules: a row is a ceiling that only `--bank` lowers, a file
   may not grow against the merge base, no row may be added, and a renamed file moves its row.
4. The scope is the baseline's, not the lint table's. A crate outside the scope is not counted, so
   widening it is a reviewed edit to one list.

A per-file baseline was chosen over one tree-wide counted-claims ceiling, because a single number
lets a lane trade a removed panic in one file for a new one in another, and its failure cannot name
the file.

## Scope

The kernel crate, and the 32 crates the note lists, unless an architect amends the list. For
each, the four panic lints and `indexing_slicing`. `arithmetic_side_effects` is left to the cast
proposal: release builds keep overflow checks (calef, 2026-10-04, `notes/overflow-checks.md`), so
an overflow is a panic and the two proposals share that boundary.

The boot builders are in scope and counted, not exempted. Their `expect`s are a choice the
baseline records. Converting them is not this milestone's work.

## Done when

1. The lint ratchet runs in `script/lint`, with a selftest that runs first, as the file-length
   ratchet's does.
2. The baseline freezes every file in scope at its count on 2026-10-09 (UTC), and a row may only
   go down. No row carries a target of zero by a date (the thresholds ruling above). The syscall
   paths and the network and peer-process crates fall as their sites are rewritten.
3. Each of the 20 crates without a fuzz target has one, or a `BUGS` entry in its own crate saying
   why fuzzing it would not find anything.
4. The baseline is banked at the end, so every row reads as the tree does.

## Open questions for an architect

1. Ruled 2026-10-09 (UTC): all 32 crates, with device drivers counted as untrusted. The question
   was which crates count as parsing untrusted input, given that the note's 32 include
   device-supplied bytes (an NVMe completion, an ACPI table).
2. What counts as syscall-reachable in the kernel. A file list is cheap and goes stale; a module
   boundary (`syscall.rs` and what it calls) is cleaner and needs a split first. With no zero
   target, this now decides only which rows a reader watches, not what blocks done. The cast
   proposal's ruling names 16 files for its own scope.
3. Is a boot-time `expect` acceptable in the end state? The recommendation is yes, at a ceiling the
   baseline records, since a misconfigured image failing at boot is the right failure.
4. Does `indexing_slicing` belong in the kernel at all? 89 of its 282 kernel hits are in `arch/`,
   mostly fixed-size table walks the compiler can bound.
5. One baseline file for every lint ratchet, or one per proposal? One file is the recommendation;
   four proposals reuse the pass.

## Where it sits in the ranking

The customer path is vacant, so the tie breaks toward the fatal risks. This proposal serves risk 7
(the confinement claim is false). Risk 7's entry already records a denial of service as a
finding, and a reachable kernel panic is one. It also serves risk 2 (the proofs prove trivia), since
Kani proves panic-freedom only where a harness reaches. It ranks first of the five.
