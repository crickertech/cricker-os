---
status: BUILT
raised: 2026-09-21
built: 2026-09-21
---
# 522. A boundary drawn by dependency, not by path, is what finds the 117 blocks a path split misses

*(Number provisional until the merge queue lands it.)*

**The contribution is the boundary, not the count.** `script/metrics`'s `unsafe_outside_arch` (824,
density 77 per 10,000, `script/lint`'s ceiling 88) answers "how much of the tree's Rust is unsafe
outside `kernel/src/arch/`", and nothing in that number distinguishes code nothing confines from
code the kernel's own MMU-plus-capability-table mechanism confines like any other program. A hand
computation (`notes/trusted-base.md`, the `maintainer/redleaf-comparison` lane, now published in a
comparison against Tock and RedLeaf) drew that boundary at `kernel/src/**` versus everything else
and got 577 kernel blocks against 561 elsewhere. **That boundary is a path prefix, and a path prefix
is the wrong instrument**: sixteen `crates/` members, `paging` and `dma_validator` and
`inter_process_communication` among them, were lifted out of `kernel/src` on purpose so Kani could
reach them, and every one of them ships only in the kernel binary. A path-only split cannot see that
they never leave the trusted base just because their directory changed; it counted their 117 unsafe
blocks as if they were userspace's.

**The boundary this milestone draws instead is a real dependency edge**, read once from `cargo
metadata --format-version 1` at the repository root: a crate is kernel-trusted if the `kernel`
package reaches it over an edge that is not exclusively `dev`, userspace-confined if only
`components` or `fixtures` (the two packages holding every EL0 program, milestone 175 (split
`user/`: `components/` for services, `fixtures/` for test and benchmark programs)) reach it that
way, and a fifth population, `shared`, when both do. That is mechanical and checkable in a way a
hand-drawn line is not, and it is what catches the 117.

## The four categories, and the four densities

- **kernel, 694 blocks, 48,724 code lines, density 142 per 10,000.** `kernel/src/**` (arch and not)
  plus sixteen `crates/` members reachable only from the `kernel` package: `address_space_identifier`,
  `capability`, `cpu_set`, `dma_validator`, `firmware_configuration`, `generational_table`,
  `inter_process_communication`, `intrusive_fifo`, `jh7110_clock_and_reset`,
  `memory_corruption_canary_gate`, `memory_regions`, `page_frames`, `paging`, `pci`,
  `thread_wake_handshake`, `work_steal_slot`.
- **userspace, 382 blocks, 31,639 code lines, density 120 per 10,000.** `components/`, `fixtures/`,
  sixteen `crates/` members reachable only from them, and five packages that are each their own
  cargo workspace and so are invisible to `cargo metadata` run at the repository root at all:
  `redoxfs_server` (its `el0` build; its `hosttest` build never ships), `std_exerciser`,
  `entropy_backend`, `cryptography_exerciser`, `cryptography_provider`. Every one of them is, by its
  own header, a program or library that runs on nife at EL0 and never as kernel code.
  `cargo metadata`'s own resolve graph does not include them; they were found by reading which
  directories declare their own `[workspace]` rather than joining the root one, and confirmed from
  their own provenance comments.
- **shared, 19 blocks, 5 of 36 crates reachable from both sides.** Not folded into either side. Two
  of the five, `environment_protocol` and `clock_protocol`, were read closely enough to confirm this
  is not a hedge: the kernel builds a page with the crate's `unsafe fn new`/`from_raw_parts`
  constructor and a userspace `std` program reads it back through the identical accessor, so the
  same unsafe source genuinely executes with kernel privilege in one binary and under confinement in
  the other.
- **boot chain, 37 blocks.** `uefi_loader` and the one crate only it reaches (`sealed_pair`). It
  runs once, before the kernel starts, with the full privilege of the pre-OS environment, to decide
  which kernel image gets control, and its memory is reclaimed before the kernel's isolation
  boundary exists to enforce anything on it. A chain-of-trust question, not a runtime-isolation one.

## What was found and deliberately not folded in

**Three `crates/` members are host tooling `unsafe_census`'s own `HOST_ONLY` exclusion does not
catch**: `board_console`, `portable_executable`, `stick_maker`. Each says so in its own header
(`stick_maker`'s: *"A host program, not a nife program. It runs on macOS, Linux and Windows and
never on nife"*); each is read only by `xtask` or invoked directly, never by a nife binary. They sit
under `crates/`, not one of `HOST_ONLY`'s path prefixes (`bench/host/`, `xtask/`, `tools/`, `fuzz/`,
`helpers/`, `patches/`), so the existing census counts their 6 unsafe blocks as if they ran on nife.

**This was found and is reported here, and `unsafe_census()`/`unsafe_outside_arch`/`script/lint`'s
ceiling are left exactly as they were.** The 824 and 77 that `script/lint` still gates on mean
precisely what they meant before this milestone. Widening `HOST_ONLY` to also exclude these three
crates is a real, small correction (it would move the published 824 to 818 and the density from 77
to roughly 81), but it is a decision about an existing gate's inputs, not a byproduct of drawing a
new one, and it is an architect's to make.

## The history, and why a gap in it is not a bug

`script/metrics --backfill` restated this split across all ten prior weeks. **Eight of them carry a
nonzero `unsafe_trust_unclassified`** (57 to 188 blocks): this tree spells its crate names out
(design/naming.md), and around forty crates did not always have today's spelling
(`crates/ipc` became `inter_process_communication`, `crates/dtb` became `device_tree_blob`, and so
on), so the classification table, built from today's names, cannot see them under their old ones.

**This was not reconstructed from git's own rename detection, and that refusal is itself a finding.**
`git log --diff-filter=R --summary` offers candidate renames, and at least one it offers is wrong:
`crates/canary_gate` pairs with `crates/work_steal_slot` at 96% on a `Cargo.toml`-only match, while
`canary_gate`'s own `src/lib.rs` correctly pairs with `memory_corruption_canary_gate` at 97%. A
hand-verified alias table for around forty old names, several of which (`mdns_proto`, `smb_proto`,
`ntlm`) name protocols since removed outright with no current bucket to map to at all, is real work
this milestone chose not to rush past a false pairing. **A future reader who sees the gap in the
series and assumes it is a bug should read this instead**: it is a recorded limitation, not an
oversight, and it is exact from 2026W38 (2026-09-20) onward.

## The ceiling question, answered as a recommendation and not a change

`script/lint`'s `<!--count-at-most:unsafe-density-outside-arch-->` (`notes/unsafe-obligations.md`,
currently 88 against a mixed density of 77) is a ceiling over the population this milestone shows is
two populations. **If a ceiling is held against the split, kernel density (142 per 10,000) is the
number it belongs on**, because it is the population a mixed number's blind spot actually lives in:
unsafe code nothing confines. Userspace density (120) matters less for the same reason a bug there
is a bug in one confined program rather than in the base. **This is a recommendation. Changing
`script/lint`'s 88, or setting any ceiling on the new columns, is an architect's, not this
milestone's**; a ceiling started cold, with no history of it moving deliberately the way 88 was
cinched down six times (milestone 134 (the register of measures: every number this kernel owes
itself)), and that history is worth having before a number is chosen.

## What this means for the published RedLeaf/Tock comparison

`notes/trusted-base.md` states 577 as the kernel's unsafe count, in a comparison against Tock and
RedLeaf's own published trusted-base figures. That number is drawn at the `kernel/src/**` path
boundary and is short by the 117 blocks the sixteen kernel-only `crates/` members carry, exactly
the risk that note's own `BUGS` section names (moving code out of `kernel/src` can shrink a line
count without shrinking what anyone has to trust) without having checked whether it had happened to
its own figure. The maintainer is correcting that note separately, citing this milestone for the
number; this milestone does not touch it.

## BUGS

- **The crate classification table is hand-maintained, not derived at every run.** It was read once
  from `cargo metadata` on 2026-09-20 and is applied to every week alike, the same restatement trade
  `script/metrics`'s `MILESTONE_STATUSES`/`NAME_STATUSES` tables already make. A crate renamed,
  split or removed since will misclassify or, if the table has genuinely never seen its name at all,
  report `unsafe_trust_unclassified` rather than guess. See `helpers/rust_source.py`'s own comment.
- **A `shared` crate's unsafe is not attributed to a call site.** Whether a given block in one of the
  five `shared` crates that carry `unsafe` is reached by the kernel's production logic, by a
  userspace `std` program, or only by the kernel's own `#[cfg(test)]` test-oracle modules (several
  of which never ship at all) is a per-function read this milestone did not perform.
  `cargo`'s own `unused_dependencies` warnings during `script/test` are concrete, partial evidence
  that some of this is real: `byte_sink_protocol`, `calendar`, `capability_witness_protocol`,
  `coremark`, `counter_frequency_protocol`, `grant_plan`, `job_mix`, `network_time_protocol`,
  `pgrep`, `pmap` and `soak_page` all showed as unused kernel dependencies for at least one
  target/feature combination in one run, which is consistent with (but does not prove) test-only use.
- **`notes/register-of-measures.md` says the live ceiling is 94; the actual gate
  (`notes/unsafe-obligations.md`) is 88.** Found while reading the register to place this split
  beside it; a note wanted the correction pointed out, not made, since the historical ratchet
  narrative needs care this milestone did not budget for.

## Follow-on

- **Recorded.** Rebuilding the old-crate-name alias table by hand, verified file-by-file rather
  than trusted from `git log --summary` alone, would close the `unsafe_trust_unclassified` gap in
  2026W29 through 2026W37; not attempted here, and the reason is in this block's own `BUGS` above.
- **Recorded.** Per-call-site attribution inside the five `shared` crates that carry `unsafe`, to
  separate kernel production use from kernel test-oracle-only use from userspace use, is future
  work this milestone did not perform; `cargo`'s own `unused_dependencies` warnings, named in this
  block's own `BUGS` above, are partial evidence some of it is real.
- **Recorded.** `notes/register-of-measures.md` says the live ceiling is 94 against the actual 88
  in `notes/unsafe-obligations.md`, a drift this milestone found and did not correct; see this
  block's own `BUGS` above.
- **Recorded.** Whether to widen `unsafe_census()`'s `HOST_ONLY` to also exclude `board_console`,
  `portable_executable` and `stick_maker` (found above; it would move the published 824/77 by 6
  blocks) is not made here.
- **Recorded.** Whether any ceiling should be held against the split, and if so which density, is
  recorded above as a recommendation (kernel density, 142 per 10,000) rather than decided here.

## Index row

`script/metrics`'s `unsafe_outside_arch` mixed unconfined kernel code with confined userspace code
into one density. This milestone draws the boundary as a real `cargo metadata` dependency edge
rather than a path prefix, which is what finds the 117 blocks of sixteen kernel-only `crates/`
members (`paging`, `dma_validator`, `inter_process_communication` among them, lifted out of
`kernel/src` on purpose for Kani) that a path-only split counts as userspace's. The split, backfilled
across history with its gaps stated rather than guessed at, is kernel 694 blocks at density 142 per
10,000, userspace 382 at 120, shared 19, boot chain 37; `script/lint`'s ceiling is left untouched,
with a recommendation, not a change, that a future one belongs on the kernel density.
