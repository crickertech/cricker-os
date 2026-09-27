---
status: BUILT
raised: 2026-09-02
built: 2026-09-03
---
# 236. Three derivations are copied between scripts, and nothing notices when they drift

Minted 2026-09-02 by calef, from milestone 234's (the project's own numbers,
one row per ISO week) lane, which had to copy them to do its job. Built 2026-09-03; what was
actually decided, and what the block deliberately left open, is at the bottom.

**In brief.** Three derivations now exist twice:

- the **`unsafe` census**, in `script/lint` and again in `script/metrics`;
- the **code and comment line split**, likewise;
- the **harness count**, in `script/falsifications` and again in `script/metrics`.

They are copied rather than imported because each lives inside a `#!/bin/sh` script's inline python
heredoc, where nothing can import anything.

**So if a gate changes its definition, the dashboard keeps the old one silently.** The numbers on
`notes/project-metrics.md` would drift away from the numbers the gates enforce, both would look
authoritative, and no check would fire. That is rung four of AGENTS.md's ladder holding together the
page whose entire purpose is measurement.

## Why it is not simply "make it a crate"

The obvious fix is a host crate both callers depend on, and it may well be right, but two things make
it a decision rather than a refactor:

- **These are shell entry points on purpose.** `script/` is the "Scripts to Rule Them All" front door
  and its members are `#!/bin/sh` by convention; turning three of them into consumers of a Rust crate
  changes what a `script/` command is. AGENTS.md's naming table already treats `script/` as its own
  domain.
- **A derivation used by a gate has different requirements from one used by a report.** The gate must
  be fast enough to run on every push and refuse ambiguity; the report must run over eight historical
  checkouts and tolerate a tree that no longer parses. Those may not want the same code, and saying
  so is part of the work.

## What it needs

**One definition per question, or a check that fires when two disagree.** The second is weaker and
much cheaper, and this block does not choose: a test that runs both derivations on the current tree
and fails on a mismatch would catch every drift that matters without moving any code, at the cost of
keeping two implementations honest rather than removing one.

## BUGS

- **Nobody has seen this drift yet.** It is a hazard rather than a defect, which is the argument for
  the cheap answer and against a large refactor.
- **The count may be higher than three.** Those are the three milestone 234's lane needed; nothing has
  swept `script/` for others, and the same shape may exist between gates that never talk to a report.
- **`script/metrics` deliberately tolerates old trees** that the gates would reject, so a shared
  implementation would have to grow that tolerance and might then be looser than a gate should be.

## What was built, 2026-09-03

**Status: BUILT.** Both answers, split by which question each derivation is being asked. The block
declined to choose and was right to: measurement showed the choice is not one decision but three.

**The premise was false, and checking it is what shrank this from a refactor to an afternoon.** The
block says the derivations are copied "because each lives inside a `#!/bin/sh` script's inline python
heredoc, where nothing can import anything." A heredoc with a stable working directory imports
perfectly well, and `script/lint`, `script/metrics` and `script/falsifications` all `cd` to the
repository root before they run python. So the host crate the block priced was never the only way to
get one definition, and the thing that made it a decision (three `script/` commands becoming
consumers of a built artifact) does not arise. `helpers/rust_source.py` is a plain module, nothing
executes it, and no caller's shape changed.

**Two derivations are now one definition.** The comment-and-literal stripper, which is what makes a
code-line count a code-line count and is the shared half of the code and comment line split, and the
whole `unsafe` census with every exclusion and its reason. `script/lint` and `script/metrics` import
both. There is no check on them because there is nothing left to compare.

**The tolerance the block warned about was never in the count.** It is in the file *sourcing*, and
that stayed in each caller: `script/lint` walks `git ls-files` and reads the working tree, and
`script/metrics` streams blobs out of `git cat-file --batch` at eight revisions and checks nothing
out. Sharing the count removed the drift without asking either script to describe a tree it is not
looking at.

**The harness count could not be collapsed, and the reason is a capability difference rather than a
preference.** `script/lint` and `script/falsifications` attribute each harness to a workspace
package out of `cargo metadata`, which is the better derivation and needs a workspace on disk.
`script/metrics` has no workspace on disk at a July revision and its own header records that nothing
there is allowed to build. So that one gets the block's weaker answer, said out loud as the weaker
answer: `script/lint` runs all three and fails on a disagreement.

**The comparison is not a third copy, which is the trap in that answer.** It calls
`_harness_hits()` (this gate's own), `script/falsifications --count` (a mode added for exactly this,
printing two integers rather than the prose `--check` already prints, because a gate that greps a
sentence breaks when somebody improves the sentence), and `rust_source.harness_count` (the function
`script/metrics` itself calls). Nothing in the check knows what a harness looks like. It was proved
to fire by perturbing the shared regex and its exclusion list: `147, 147, 145`, and `script/lint`
exited 1.

**The sweep for a fourth copy, which the block asked for.** Every constant and regex definition in
`script/` and `helpers/` was compared across files. Two more shapes turned up and neither is this
hazard:

- `FNAME = re.compile(r"(\d+)-[a-z0-9][a-z0-9-]*\.md")` in `script/decisions` and `script/journeys`.
  Both are gates on a filename convention, neither reports a number the other must match, and
  nothing downstream restates either. It is a convention spelled twice, not a derivation that can
  drift apart while both look authoritative.
- The QEMU invocation fragments shared by `helpers/qemu-runner-aarch64.sh` and
  `helpers/qemu-runner-riscv64.sh` (about a dozen `-drive`/`-device` lines). Shell rather than
  python, a machine configuration rather than a measurement, and out of this block's scope. Recorded
  here rather than turned into a milestone, because nothing yet says the two runners should be one.

**No committed data moved.** `script/metrics --backfill` was run before and after: all seven
historical rows are byte-identical, and only the current week's row changes, because `HEAD` moved
between the last workflow run and this branch. Two consecutive backfills produce identical bytes, so
the idempotence guarantee holds. The generated CSV and SVGs are deliberately not in this branch's
diff.

## BUGS

- **The harness comparison is rung two, not rung one.** Two implementations are kept honest rather
  than removed, so somebody can still change one definition; they just cannot do it quietly.
- **It compares one number, not a breakdown.** A per-package comparison would fail on a scope
  difference the three derivations deliberately have, so the count that actually reaches a reader is
  what is checked.
- **`helpers/rust_source.py` is a provisional name.** Minted by this lane; names are an architect's.
  Refused: `source_census`, because this tree already spends "census" on the `unsafe` count
  specifically and a module named for it would read as holding only that; `measures`, which collides
  with `notes/register-of-measures.md`, the same collision `script/metrics`' own header refused;
  `common` and `util`, which claim nothing, and DECISIONS §39 says a name is a claim.
- **The module is Python in a tree that is otherwise Rust and shell.** It adds no dependency
  (DECISIONS §46 untouched: it imports `re` and nothing else) and no build step, but it is a third
  language in the tooling, and that was accepted rather than argued for at length because the
  alternative was a `cargo build` in front of three `script/` commands.

## Follow-on

- **Refused.** Folding the QEMU invocation fragments shared by `helpers/qemu-runner-aarch64.sh` and
  `helpers/qemu-runner-riscv64.sh` into one place. They are shell rather than python, a machine
  configuration rather than a measurement, and nothing yet says the two runners should be one.
- **Refused.** Sharing the filename regex between `script/decisions` and `script/journeys`. Both
  gate a filename convention, neither reports a number the other must match, and nothing downstream
  restates either, so it is a convention spelled twice rather than a derivation that can drift apart
  while both look authoritative.
- **Recorded.** `design/roadmap/236-lift-the-copied-derivations.md`: the harness comparison is rung
  two, not rung one. Two implementations are kept honest rather than removed, so somebody can still
  change one definition; they just cannot do it quietly.
- **Recorded.** `design/roadmap/236-lift-the-copied-derivations.md`: the comparison checks one
  number rather than a breakdown, because a per-package comparison would fail on a scope difference
  the three derivations deliberately have.
- **Recorded.** `helpers/rust_source.py` is Python in a tree that is otherwise Rust and shell. It
  adds no dependency and no build step, and the third tooling language was accepted rather than
  argued for, because the alternative was a `cargo build` in front of three `script/` commands.
- **Recorded.** `design/roadmap/236-lift-the-copied-derivations.md` carries the provisional name.
  `helpers/rust_source.py` was minted by the lane, names are an architect's, and what was refused
  and why is written down beside it.

## Index row

two lifted into `helpers/rust_source.py`; the harness count cannot be one definition and `script/lint` compares all three instead
