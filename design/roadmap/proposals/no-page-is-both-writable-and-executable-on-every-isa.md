---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# No page is both writable and executable, proved on every ISA

Raised 2026-10-03 (UTC) by the lane for milestone 633 (an outside agent attacks the confinement
claim), whose three-ISA sweep found row 11 of `notes/confinement-claims.md` evidenced on one
architecture. §19 (architectural parity is a tenet) makes a capability a gate on every ISA; this is
a gap in the evidence, not in the capability.

## The finding

Row 11, "no page is both writable and executable", is proved by one harness,
`paging::x86_64::verification::no_encoded_leaf_is_both_writable_and_executable`. It takes any
physical address and any of the eight named `Flags` constructors, encodes a leaf through
`Ia32e::leaf_entry`, and asserts that a leaf without `XD` has no `RW`. It carries a replayable
falsification.

The aarch64 and Sv39 encoders carry eight harnesses each, and none states this claim. Neither file
has an assertion that mentions its execute bits together with its write bit. The two encoders say
the same thing in different words, which is why one harness cannot serve three:

- aarch64 (`crates/paging/src/aarch64.rs`): execute is the absence of `PXN` (bit 53) for EL1 and
  of `UXN` (bit 54) for EL0; writable is `AP_RO` (bit 7) clear.
- Sv39 (`crates/paging/src/sv39.rs`): `W` is bit 2 and `X` is bit 3.

Row 22 is not a substitute. Its two kernel tests run on all three ISAs and refuse an ELF that asks
for a writable executable segment, which is the loader layer. They say nothing about whether the
encoder could produce such a leaf from a `Flags` the loader never sends.

## What each ISA needs

One harness per encoder, the shape of the `x86_64` one, stated in raw bits rather than through
`leaf_flags` so a decoder defect cannot hide an encoder defect (the phrasing the `capability`
crate's derive proof records the reason for):

- `paging::aarch64::verification::no_encoded_leaf_is_both_writable_and_executable`: for any
  masked `pa` and any of the eight `Flags`, if `leaf & AP_RO == 0` then `leaf & PXN != 0` and
  `leaf & UXN != 0`.
- `paging::sv39::verification::no_encoded_leaf_is_both_writable_and_executable`: for any masked
  `pa` and any of the eight `Flags`, if `leaf & W != 0` then `leaf & X == 0`.

Each needs a replayable falsification. The `x86_64` patch,
`crates/paging/falsifications/x86_64.verification.no_encoded_leaf_is_both_writable_and_executable.patch`,
is the template: make one code constructor writable and watch the harness go red. Row 11's
"Tested by" column then names three harnesses, and its "Falsified" column three records.

## Cost

An estimate, not a measurement. About twenty lines per harness, two patch files, one row edit in
`notes/confinement-claims.md`. Kani time is seconds: the `x86_64` harness already runs inside
`script/verify`'s paging shard, and these two have the same eight-way enumeration. One lane, well
under a day, touching only `crates/paging` and the notes.

## Recommendation

Build it as a small lane. It is reversible, nobody has acted on the gap, and if both options cost
the same, a parity tenet with one-ISA evidence is still the wrong state to leave.
