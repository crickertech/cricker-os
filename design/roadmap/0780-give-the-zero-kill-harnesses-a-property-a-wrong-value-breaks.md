---
status: NOT-STARTED
promoted_from: give-the-zero-kill-harnesses-a-property-a-wrong-value-breaks
raised: 2026-10-04
milestone_dependencies: 741
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 780. Give the zero-kill harnesses a property a wrong value breaks

Raised by the risk-2 reach study (`lane/kani-reach`, milestone 741 (does a standing proof notice a
regression)) on 2026-10-04 (UTC). Title and slug are drafts.

Six standing harnesses reach mutants and kill none of them (`notes/kani-reach-2026-10-04.md`):
`device_tree_blob`'s `be32_is_total` and `be64_is_total`, `elf`'s
`check_segment_bounds_never_panics` and `note_extent_never_panics`, `machine_discovery`'s
`a_multi_letter_extension_is_never_read_as_a_privilege_letter` (0 of 133 in transitive reach), and
`pci`'s `ecam_offset_stays_inside_the_window`. Four of them prove panic-freedom. cargo-mutants
makes functions return wrong values, not panic, so no regression of the measured kind can turn
these red.

## Done when

Each of the six has one of these:

- an assertion a wrong value breaks, with a falsification record showing a mutant it now kills;
- a merge into a sibling harness that already kills those mutants; or
- a sentence beside it saying that panic-freedom alone is the claim, and why that is worth a
  harness.

Start with the machine_discovery harness. Its name claims a value property (a letter is never
misread), so 0 of 13 direct mutants is the most surprising result of the six.

## Index row

Six standing Kani harnesses reach mutants and kill none, four because they prove only panic-freedom, which a wrong return value cannot break. Each needs a stronger assertion, a merge into a sibling, or a recorded reason that panic-freedom alone is the claim.
