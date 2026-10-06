---
status: NOT-STARTED
promoted_from: a-swish-check-line-has-a-falsification-path
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: 134
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 775. A swish-check line has a falsification path

<!-- writing-standards: exception. Granted 2026-10-06 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised 2026-10-04 (UTC) by milestone 742 (every test is falsified as routine). Title and
slug are drafts. **This is an architect's call**: it is the spelling of §134's patch path.

**The fork.** §134 (a harness carries a machine-replayable falsification record) names a patch `<package>/falsifications/<module.path>.<fn>.patch`, keyed on a
function. Milestone 742 made a swish-check record replayable, and the record sits above
`swish_check_boot`, the function that types every line. So exactly one swish-check record can exist:
`xtask/falsifications/swish_check.swish_check_boot.patch`, which today falsifies the
`installed/unvouched` line (row 31 of notes/confinement-claims.md). A second line's record has no
path to take.

**Options.**

1. A line slug after the function: `swish_check.swish_check_boot.installed-unvouched.patch`, with
   the block above the line's entry in `SWISH_CHECK_SCRIPT` rather than above the function.
   Recommended. It keeps one record per claim, which is what the path convention exists for, and the
   `Line:` field (provisional) becomes the slug's source of truth rather than a second copy.
2. Several hunks in the one existing patch, one per line, replayed together. Refused: a single red
   would no longer say which line caught which defect, the failure 742's `Line:` check exists to
   prevent.
3. A separate directory, `xtask/falsifications/swish-check/<slug>.patch`. Works, and breaks the one
   rule every other record obeys (path derived from the code's own name), so `--check` would need a
   branch.

**What is blocked.** Only a second swish-check record. The first is swept today.

**Cost.** Each swish-check record costs one leg's build and boots in the weekly sweep, about 2 to 6
minutes on a hosted runner (aarch64 is the cheap leg). Nothing per pull request.

## Index row

The falsification-record path convention is keyed on a function, but one swish-check function types every line, so only one swish-check line can have a record. The block states the fork and its options for an architect to rule on; nothing is decided here.
