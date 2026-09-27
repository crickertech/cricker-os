# How stable the interface is

*Milestone 610 (the interface's stability is measured weekly), its number provisional. calef,
2026-09-27 (UTC): "What I want are good metrics for capturing the stability of our interface." The
numbers are on [the metrics page](project-metrics.md#interface-stability), per crate in
[the appendix](project-metrics/interface-stability.md), and produced by
`helpers/interface_stability.py`. The note's name, the program's, both CSVs' and every column's are
provisional.*

## What the interface is

What a program that is not the kernel is written against, and what would break it if it changed.
Four parts, checked against `main` at `06f2caa66` (2026-09-26):

- The contract crates, 28 of them. That is the contracts division of
  `notes/packages-and-divisions.md` (on #1389's branch, not yet on `main`): `abi`, the fifteen
  `*_protocol` crates that are contracts (not `network_time_protocol`, a service), and eleven named
  formats. `manifest_note` is added: it was born on 2026-09-26, after that census, and is exactly
  what the division names.
- The syscall surface, which is every public constant in `abi`: 4 syscall numbers (`SYS_*`), 3
  object types (`objtype::*`), and 51 method numbers and argument or result encodings in the object
  modules. `abi` is also a contract crate, so these are a lens on the first part, not an addition.
- The wire formats. A contract crate's public constant named `*VERSION*`, `*MAGIC*` or `*REVISION*`
  is a format identity. Twelve today, in ten crates: `boot_slot` (two), `clock_protocol`,
  `compositor` (two), `counter_frequency_protocol`, `current_cpu_protocol`, `environment_protocol`,
  `globally_unique_identifier_partition_table`, `manifest_note`, `nifefs` and `package_archive`.
- The std ABI, which is every crate `cargo xtask std-src` copies into the standard library's
  platform layer. It is read from `xtask/src/farm.rs`, so the list cannot drift from the copy:
  `abi`, seven protocols, `std_runtime_protocol` and `user_mode_heap`. The last is the runtime
  division's, which is why the interface is the union and not the contracts alone.

Together, 29 crates and 2,430 public items.

## What each column counts

One row per ISO week in `notes/project-metrics/interface-stability.csv`. Each week's first-parent
commit (the same commit `script/metrics` picks) is compared with the previous week's.

| column | counts |
|---|---|
| `interface_breaking` | public items removed or changed, plus one per crate renamed |
| `interface_additions` | public items added, a new crate's every item included |
| `interface_std_abi_breaking`, `_additions` | the same, over the std ABI crates only |
| `interface_syscall_changed` | `abi` constants removed or renumbered with no successor |
| `interface_syscall_renamed` | `abi` constants renamed with the number kept |
| `interface_syscall_added` | `abi` constants added |
| `interface_format_bumps` | a format identity's value changed |
| `interface_format_amendments` | dated entries in a format's own amendment ledger |
| `interface_contract_commits`, `_crossing`, `_cochange_pct` | co-change, below |
| `interface_cochange_pct_excluding_mass` | the same without mass mechanical commits |
| `interface_crates_unmeasured` | crates rustdoc could not document on either side |

An item is anything a consumer names. That is a function, a constant or a type, and each struct
field, enum variant, trait item, inherent method, and trait a type implements. "Changed" means its
signature, type, field order, `repr`, discriminant or constant value moved. A parameter's name and a
constant's spelling (`1 << 2` for `4`) are not part of it. A crate rename is one break, not one per
item, because it is one decision and every consumer fixes it with one line.

**How it is read.** rustdoc's JSON output, on the pinned nightly, for `aarch64-unknown-none-softfloat`,
over a `git archive` of each week's tree in `target/`. Nothing is checked out. The toolchain is
always HEAD's, so every week's JSON has one format. A constant rustdoc cannot evaluate (it writes
`_` for `*b"CRKR0002"`) is read from the source its span points at, which is how the one format bump
in the history was found. Old crate names follow git's rename detection to today's name, preferring
the renamed `src/lib.rs` over the renamed `Cargo.toml`, because git paired `uheap`'s manifest with
`line_editor`'s in the 2026-08 naming sweep.

**The syscall rename rule.** `endpoint::SEND` becoming `rendezvous::SEND` at the same value breaks
every caller's source and no binary. A removed `abi` constant is a rename when exactly one added
constant has its value and keeps either its leaf name or its module; a module another constant
already moved unambiguously resolves the rest. Anything less certain stays a change.

**Co-change** reuses the method of the measurement behind #1389's 76%, as the maintainer forwarded
it on 2026-09-26: merges excluded, documentation excluded from both sides, and a commit crosses when
it touches any non-documentation path outside the contract directories. A commit is mass mechanical
when it touches more than 40 files, spans more than 8 top-level areas, or its subject reads like a
rename or a migration. Over #1389's 60-day window with today's directory names this reads 78.6% of
224 commits, against that measurement's 78.8% of 226. The weekly series also counts commits to a
contract's older directory names, which raises the window to 378 commits and moves the share to
78.3%.

**Past cells are carried.** `--update` writes the current week and any week the CSV lacks, and
`script/metrics` never recomputes these cells. A week's old source that compiles on today's nightly
may not compile on next month's, and a past number must not change for that reason. `--backfill`
restates everything, in about ten seconds on the dev machine.

## The amendment ledger

An in-place change to a released format version cannot be derived: the layout may move with no
public item changing, and a public item may change with no byte on the wire moving. So it is
recorded at the format, AGENTS.md's rung 3, in the crate's own module documentation:

```text
//! # Amendments
//!
//! - 2026-09-27: version 1's field order changed in place, because ...
```

One list item per amendment, opening with its UTC date. The heading and the shape are this lane's,
provisional. No crate carries one yet. `manifest_note`'s version 1, amended in place on 2026-09-27
on a branch not yet merged, is the first that should.

## First reading, and the trend

On 2026-09-26 (UTC), 2026W39 so far: 19 breaking changes and 365 additions. Over the last four weeks
(2026W36 to 2026W39): 121 breaking, 449 additions, 30 of them in the std ABI, 2 syscall numbers
changed, no format bumps, no recorded amendments, and 71% of 151 contract commits crossing another
division.

| weeks | breaking | additions | syscall renumbered or removed, renamed | format bumps | co-change, excl. mass |
|---|---:|---:|---|---:|---:|
| W29 to W30 | 0 | 134 | 0, 0 | 0 | 100%, 89% |
| W31 to W33 | 38 | 1,545 | 0, 0 | 1 | 68% to 82% |
| W34 to W35 | 82 | 436 | 2, 25 | 0 | 89% to 92% |
| W36 to W39 | 121 | 449 | 2, 0 | 0 | 52% to 71% |

Growth came first and breakage followed it: the naming sweeps of 2026W34 and 2026W35 (`endpoint`
to `rendezvous`, `untyped` to `memory_region`, the `_proto` crates) are most of the syscall
renames, and 2026W38's fourteen crate renames are most of that week. The only format bump in the
history is `crickerfs`' magic going from `CRKR0001` to `CRKR0002` on 2026-08-01. `grant_plan`,
`credential_protocol`, `globally_unique_identifier_partition_table` and `filesystem_protocol` carry
most of the recent breaks. Co-change excluding mass commits has fallen from the high 80s to between
52% and 71%.

## What "stable" would mean

A threshold proposal only. The threshold is calef's call.

Over a trailing four weeks: no syscall number renumbered or removed, no format changed in place
without a bump, and at most four breaking changes to the contract crates in all, about one a week.
Separately, a contract crate is settled once it has gone four weeks without a breaking change,
which the appendix already sorts by. The co-change share is not part of it: it measures how often a
contract moves with a consumer, which is coupling and growth, not breakage. §151 (the goal of the
repository split is independent release and third-party programs) wants it low before a split, and
50% excluding mass commits is the obvious line, where a contract changes alone more often than not. By this proposal the interface is not stable today, and no four-week window
in its history has been.

## Why rustdoc JSON and not a semver tool

rustdoc's JSON is on the pinned nightly and adds no dependency. `cargo-semver-checks` and
`cargo-public-api` both read the same JSON. What they add is a catalogue of rules for whether a
change needs a new major version (from memory, not verified here). This series asks a narrower
question, whether the public surface moved, so neither is needed. Both also pin a rustdoc JSON
format version, which would tie the tool's version to the nightly pin. That argues against taking
either, and taking one is a §46 (thin primitives or whole subsystems) decision for calef.

## BUGS

- A semantic change behind an unchanged signature is invisible: a function that now returns a
  different value, a message whose meaning moved, a method whose rights check changed.
- A wire layout can change with no public item changing (an encoder's field order in a function
  body), and only the ledger sees that, if someone writes it. Nothing checks that a ledger entry
  exists.
- Only public items rustdoc documents are read. `#[doc(hidden)]` items, anything behind a
  `cfg(target_arch)` for riscv64 or x86_64, and auto traits (`Send`, `Sync`, `Unpin`) lost through a
  field change are not seen. Blanket impls are skipped on purpose.
- A rename inside a crate reads as one removal and one addition. That is honest for a consumer's
  source but doubles the event, and this week's list shows the predicate rule's `plausible` to
  `is_plausible` sweep that way.
- The syscall rename rule overcounts on purpose: `CSPACE_SLOTS` becoming `CAPABILITY_TABLE_SLOTS`
  in 2026W35 had no unique successor, so it counts as removed.
- A macro's fingerprint is its source text, so an edit to its body is a change even when its
  expansion is not.
- The first week counts every item as an addition, because there is no week before it.
- Membership is today's list applied to every past week, like every restated series on the metrics
  page. A contract crate deleted before today (`multicast_dns_protocol`, `smb_proto`) was never a
  member, so neither its life nor its deletion is counted.
- The first-half co-change finding in the maintainer's measurement (87.8% against 76.5%) used
  today's directory names only; this series' early weeks include older names and read differently.
- `--update` recomputes only the current week and missing weeks. A ledger entry dated in a past week
  reaches the series only on a `--backfill`.
