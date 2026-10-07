---
status: NOT-STARTED
raised: 2026-09-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 381. The UEFI loader's firmware half is proved by one boot and nothing else

Filed 2026-09-04 as an unnumbered proposal by the
`maintainer/uefi-loader-mutants` lane, from that crate's own mutation score; numbered 2026-09-19 by
milestone 433's drain of the proposal pile. Premise re-read against the tree on 2026-09-19 and
still true: `uefi_loader/src/main.rs` is still 790 lines, `.cargo/mutants.toml` still names it in
the exclusion list with the `required-features = ["uefi"]` reason beside it, and `load`,
`say_conflict`, `copy_trampoline`, `find_screen`, `find_rsdp` and the four `say_*` formatters are
all still in that one file with nothing on the host executing a line of them.
*(Number provisional until the merge queue lands it.)*

Lifting pure logic out of a binary into the library beside it is the move this crate
already made once, so nothing here is owed to calef; what the work owes is a measurement first, and
it is allowed to come back saying no. Found by the `maintainer/uefi-loader-mutants` lane while making
that crate's published score honest.

What would prove it worked: `cargo mutants -p uefi_loader` reaches `say_conflict`'s memory-map
walk and whatever else is lifted, scoring it against the same bar as `handoff` and `image`, and
`cargo xtask uefi-boot` still boots xenon under OVMF.

## What is there

`uefi_loader/src/main.rs` is 790 lines and 154 of the crate's 189 mutants. It sits behind
`required-features = ["uefi"]`, so no host test compiles a line of it; it is now excluded from
mutation for that reason, with a gate deriving the exclusion. Excluding it makes the *number* honest.
It does not make the file proved, and the only thing that does is `cargo xtask uefi-boot` under OVMF
on `script/test`'s own leg: one boot, pass or fail, on the code path that runs on xenon before
anything else does and where a fault has no console and no debugger.

The mutants by function, which is the shape of the question rather than a worklist:

| function | mutants | what it does |
|---|---|---|
| `load` | 66 | the boot sequence: allocate, place, exit boot services |
| `say_conflict` | 28 | walks the memory map to name what is sitting in the kernel's range |
| `say_decimal`, `say_hex`, `say`, `say_span` | 31 | firmware-console number formatting |
| `copy_trampoline` | 12 | places the AP trampoline |
| `find_screen` | 9 | milestone 243's framebuffer discovery |
| `find_rsdp` | 5 | walks the configuration table |
| `efi_main` | 2 | the entry point |

## Why it is worth a milestone and not a shrug

This crate has already made the argument once and won it. `lib.rs`'s own header says the pure
half exists because *"a structure layout proved only by booting is proved by nothing that runs in
milliseconds"*, and that half scores 100% of viable mutants. The functions in the table are the same
argument not yet applied: `say_conflict`'s walk is arithmetic over descriptors, the four `say_*`
formatters are pure string work over a byte buffer, and `find_screen` and `find_rsdp` are table
searches. None of them calls firmware except to print.

The counter-argument is real and is why this is a proposal rather than a milestone. `load` is 66 of
the 154 and is genuinely a firmware call sequence; lifting the third of it that is arithmetic may buy
less than the seam costs, which is exactly the trade milestone 244 measured for `system_initializer`
and then declined. The honest version of this work measures that split first, with
`cargo mutants --list -p uefi_loader` against a candidate seam, and is allowed to come back saying
no.

## What would make it worth doing anyway

`say_conflict` is the one to look at first regardless of the verdict on the rest. It is the sentence a
person standing at a machine that will not boot reads to find out **what is occupying the kernel's
physical range**, so a wrong answer there is not a dead boot, it is a dead boot that lies about why.
It has 28 mutants, its inputs are descriptors, and nothing has ever executed it on the host.

## BUGS

- **Lifting code to be mutable is a way to raise a score without improving anything**, and this
  proposal is exposed to that more than most. The check is the one AGENTS.md already applies to
  elegance: would the seam be worth having if the score were not published? For `say_conflict` and
  the formatters, plausibly yes. For `load`, probably not.
- **The OVMF leg is the only thing that catches a regression in what stays behind**, and it is a
  single boot rather than a suite. Nothing here changes that.

## Index row

`uefi_loader/src/main.rs` is 790 lines and 154 of the crate's 189 mutants, and it sits behind
`required-features = ["uefi"]`, so no host test compiles a line of it. Excluding it from mutation
made the published score honest and did nothing for the file: what proves it is one OVMF boot under
`cargo xtask uefi-boot`, pass or fail, on the code path that runs on xenon before anything else does
and where a fault has no console and no debugger. The crate has already won this argument once, for
its library half, which scores 100% of viable mutants because the pure structure work was lifted out
of the binary. `say_conflict` is the piece worth lifting first whatever the verdict on the rest: it
is arithmetic over memory-map descriptors, it has 28 mutants, and it is the sentence a person at a
machine that will not boot reads to find out what is occupying the kernel's physical range, so a
wrong answer there is a dead boot that lies about why. The work is required to measure the split
before taking it, with `cargo mutants --list -p uefi_loader` against a candidate seam, and is
allowed to come back saying no for `load`, which is 66 of the 154 and is genuinely a firmware call
sequence.
