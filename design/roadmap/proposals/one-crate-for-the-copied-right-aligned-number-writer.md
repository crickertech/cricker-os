---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# One crate for the right-aligned number writer three programs each copy

Found on 2026-10-05 (UTC) by the lane `lane/326-survivors` at base `269c1d48c`, while triaging
mutation survivors. Title and slug are provisional.

## What is duplicated

`write_right(v: u64, width: usize, out: &mut dyn FnMut(&[u8]))` appears, byte for byte apart from a
doc comment, in `crates/free/src/lib.rs`, `crates/vmstat/src/lib.rs` and `crates/slabtop/src/lib.rs`.
It prints a decimal `u64` padded on the left to `width`, never cutting a number wider than its
column. `crates/free` also carries a signed sibling, `write_signed`.

The cost showed up as mutation survivors: the same mutant (`buf.len() - i` to `buf.len() / i`) lived
in all three because none had a test of a number wider than its column. The lane added the same
test three times (`a_number_wider_than_its_column_is_printed_whole`). A fourth program that prints a
table will copy it again.

## What is proposed

One crate holds the writer and its tests, and the three programs depend on it. This is the shape
`CLAUDE.md` rule 7 already asks for ("anything two binaries must agree on is a crate"), though here
the agreement is behaviour rather than a wire format, so it is the weaker case for the rule.

**The name of the crate is calef's call** (an architect names crates), so this proposal does not
choose one. The work is small: move the function, add the dependency to three manifests, delete
three copies and two of the three tests.

## Questions for an architect

1. Whether a crate this small is wanted, or whether three copies and three tests are the cheaper
   shape. Would we still choose the crate if both cost the same? The lane thinks yes, since the bug
   surface is one function; the recommendation is not about effort.
2. The crate's name, and whether `write_signed` and the `u64` writer share it.

Reuse: write. No crate in the tree or on crates.io is adopted here; the function is twelve lines and
a dependency for it would be a decision under §46 (thin primitives or whole subsystems) that this
does not justify. The existing copies are the prior art.
