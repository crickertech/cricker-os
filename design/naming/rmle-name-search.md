# The `rmle` name search

An appendix to [design/naming.md](../naming.md), holding one naming-history record. Stem
provisional, minted 2026-10-09 by milestone 860's lane; naming is calef's.

`rmle` (Rust Multi-Line Editor, `components/src/rmle.rs`) was ratified 2026-08-27 (calef),
milestone 169 (the kilo editor). It was chosen over `kilo` so two things are not named `kilo`
once a real C port of `kilo.c` through the seam of milestone 181 (the foreign-language seam)
exists. It is a Rust reimplementation of `kilo`'s spirit and scope, not a port, and the
distinction from `line_editor` (a single-line editor) is worth keeping visible on sight.

The search that backed the choice moved here from the program's Name block on 2026-10-09 (UTC),
by milestone 860 (comments state the constraint as it is now): a Name block states the constraint
as it is now. A refused name with its reason is the record. Each candidate below was checked
against real prior art before settling on the stem, and refused as already-real, same-domain
software:

- every SI-prefix name in the genre: `micro`, `nano`, `milli`, `giga`, `mega`;
- several metalwork-themed alternatives: `oxide`, `chisel`, `crucible`, `anvil`, `alloy`,
  `redit`.
