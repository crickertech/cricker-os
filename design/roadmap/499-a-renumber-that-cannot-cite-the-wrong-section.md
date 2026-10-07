---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: a-renumber-that-cannot-cite-the-wrong-section
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 499. A renumber that cannot cite the wrong section

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-renumber-that-cannot-cite-the-wrong-section`, filed 2026-09-19, on calef's instruction
of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own,
unedited except for this paragraph: the argument is its author's and promotion is not the moment to
improve it. Filed by the integrator after the third `design/decisions/` number collision between two
sessions in ninety minutes, on the evening calef ruled that the interleaving stays
([§194 (sessions interleave rather than serialize)](../decisions/0194-sessions-interleave-rather-than-serialize.md)).

It is a mode on a script that already exists, over files already in the tree.

In brief. When two sessions mint from the same range and one lands first, the other's whole run
shifts. That happened four times on 2026-09-19 in two and a half hours: §156, then §157, then §158,
then §159, each landing moving one branch's run up by one, from §156-§189 to §160-§194. The shift itself is mechanical. The hazard is the
citations, and it is specific: a `§` rewritten by number can be silently wrong and still pass
every gate in the tree, because the section it now names exists. `script/decisions --check` verifies
that §174 resolves. Nothing can tell it the sentence meant the section that used to be §173.

## What the tool does, because it was written before it was proposed

The third renumber was done by a script rather than by hand, and it is the shape to land:

1. Rename descending, so no rename clobbers the next one.
2. Bump each file's own `# N.` heading with its file, in the same pass, so a heading can never
   disagree with its name.
3. Rewrite citations keyed on the filename, never on the number. `157-what-a-subshell-copies.md`
   is unique across a collision; `§157` is not.
4. Move a `§N` sigil only on a line that already names the file it belongs to.
5. Print every remaining bare sigil in the affected range for a person to read. This is the step
   that earns the tool. It is an admission that the last few cannot be decided mechanically, and it
   turns them from a silent rewrite into a short list.

Steps 1 to 4 are the mechanism; step 5 is the honest floor under it.

## Where it should live

`script/decisions`, as a `--renumber-from <n>` mode, because that script already owns the index, the
numbering check and the gap check, and whoever is renumbering is already running it. The version
used on 2026-09-19, for the third shift and the fourth, lives in a session scratchpad, which is rung four and is why this exists.

## BUGS

- **Step 5 cannot be gated.** Telling an intended citation from an unintended one is reading prose
  for intent, which this tree already priced at `git grep -w TODO`'s 82% false-positive rate. The
  tool lists; a person decides.
- **It does not prevent the collision**, and is not meant to. §194 rules that the interleaving is
  the condition being modeled rather than a fault to design out.

## Index row

When two sessions mint from the same range and one lands first, the other's whole run shifts.
