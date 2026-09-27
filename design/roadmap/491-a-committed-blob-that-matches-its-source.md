---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: a-committed-blob-that-matches-its-source
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 491. Nothing checks that a committed `.dtb` is what its `.dts` compiles to

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-committed-blob-that-matches-its-source`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Found by milestone 326 (nobody has been assigned to turn a)'s `machine_discovery` lane while regenerating a fixture for an unrelated
reason, and filed by the integrator at merge because the lane's report was its only home.

The check is a `script/lint` entry over files already in the tree.

In brief. Twenty-three `.dtb` blobs are committed beside the `.dts` sources they were compiled
from, and nothing compares them. Five of them still said `cricker,` in their root `compatible`
five weeks after the 2026-08-15 rename, because a sweep can `sed` text and cannot `sed` a binary.
No test read that property, so nothing noticed. The instance is closed (326's lane regenerated all
five, commit `38cc4c1`, and fixed six stale `crates/isa/tests/fixtures/` paths in the same headers);
the mechanism is not, and the next tree-wide rename drifts them the same way.

## Why this is worth a check rather than a habit

A committed artifact that nothing regenerates is a fact nobody re-reads, which is the shape this
tree keeps finding: a `BUGS` entry nobody struck when the work landed, a baseline column six weeks
stale being read as last week's census, a roadmap status that was wrong in both records. A blob is
the worst member of that family because no reader can see it. A stale sentence at least shows up
in a diff.

It is also the failure the rename convention cannot reach by construction. `design/naming.md`
describes how a ratified rename is performed, and every step of it operates on text.

## The two shapes, and the weaker one is proposed

Rung two, proposed: a `script/lint` check that compiles each committed `.dts` with `dtc` and
compares the result against the committed `.dtb`, skipping with a printed note where `dtc` is
absent so it cannot fail a machine that has no device-tree compiler. That is the shape every other
check in `script/lint` already takes, and it fires without anybody remembering.

Rung one, refused here rather than ignored: drop the blobs and compile at build time. That makes
the wrong state unrepresentable, which is the ladder's top rung, and it buys a **build dependency on
`dtc`** for every contributor and every CI leg. DECISIONS §46 makes taking a dependency a decision
rather than a convenience, so proposing it here would be claiming a ruling that is an architect's.
If they want the stronger answer, this proposal is where the weaker one's cost is written down.

## BUGS

- A skip is a pass, and this tree has a recorded opinion about that: a check that cannot fail
  where the tool is missing is advisory on exactly those machines. The honest mitigation is that CI
  has `dtc` and a contributor's laptop may not, so the gate is real where it counts and silent where
  it cannot speak. That should be stated in the check's own output rather than left to be inferred.
- `dtc` output is not byte-stable across versions, so a naive comparison could fail on a
  contributor with a different `dtc` than CI. Comparing the decompiled *structure* rather than the
  bytes is the fix, and it is why this is a proposal rather than a one-line patch.
- **It does not generalise to other committed binaries** on its own. `bench/` images, the font
  atlases and the RedoxFS test image have the same property and no source beside them to compare
  against, so this check covers the one case where the comparison is possible rather than the class.

## Index row

Twenty-three `.dtb` blobs are committed beside the `.dts` sources they were compiled from, and
nothing compares them.
