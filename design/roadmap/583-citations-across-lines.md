---
status: BUILT
raised: 2026-09-24
built: 2026-09-24
---
# 583. `script/citations` could not see a citation a line break split, or a lettered milestone

Built, 2026-09-24. The number is **provisional**: the integrator mints it at merge. `main`'s highest was 580 when this branch was cut, with 581 and 582 claimed by open pull requests.

## The defect, which is the worst shape a gate has

`script/citations` matched a citation only when `milestone N` and its `(` sat on one physical line. Reflow a paragraph until the wrap falls between the two and the citation leaves the gate's view. Nothing goes red. The tree's confidence in the gate does not move while its coverage drops, which is worse than no gate at all: a gate that fails loudly is working, and this one stopped working and stayed green.

Three lanes paid for it in one week. The lane putting frontmatter on every decision block needed 106 glosses across 69 files, because dropping a `**Status:` prefix made each file's first prose line read as added. The `design/fatal-risks/README.md` condensation lane was bitten twice by wraps that split a citation of milestone 326 (nobody has been assigned to turn a mutation score upward) and one of §19 (architectural parity is a tenet), and ended up adding a rejoin pass to its own pipeline to work around the scanner. Each encounter cost a round trip through CI to diagnose, because the failure reads as unrelated.

The second defect is the same shape one level earlier. The candidate pre-filter was a `git grep` for `[Mm]ilestone [0-9]+,? \(`, with no letter, so a line reading `Milestone 16b (IOMMU-backed driver isolation)` matched nothing and its file was never opened. `design/decisions/26-fault-endpoint.md` was in that hole for as long as it existed, its gloss of milestone 16 unchecked, and no report in the tree could say so.

This is rung two of `AGENTS.md`'s ladder failing quietly. Both defects were found by lanes, and neither was fixed by the lane that found it.

## What changed

**The gap between a number and its `(` is whitespace, at most one newline of it.** Written as two alternatives rather than `\s+`, because the newline has to be counted rather than swallowed: `\s*` in this file once ate the line breaks it was meant to preserve and every reported line number drifted by the number of comment lines above it. The gloss itself already tolerated one newline; only the gap did not.

The pre-filter is looser than `CITE` in two ways, and both were the defect. `git grep` is line-based and `CITE` is not, so the pattern now also accepts a line that *ends* in a citation, which is what a wrapped one looks like. And the letter is in the pattern. The candidate list grows from 649 files to 853 and the scan still takes about three seconds, which is the argument for keeping a pre-filter loose: it selects files to read, and `CITE` decides.

A lettered record keeps its own title. `design/roadmap/20a-name-the-seams.md` is a block of its own, so a gloss of milestone 20a (name the seams) is read against that H1 rather than against milestone 20's, which is different work. Letters with no file of their own, such as milestone 19 (run a real workload)'s addenda 19a through 19f, resolve against the parent block, which is where those sub-parts are described.

A closing emphasis marker is part of the gap. This tree does not bold a citation by wrapping the gloss with it. It writes the number bold and the gloss plain:

```
**milestone 41** (dead code: triage the suppressions)
```

which puts the `**` between the number and the `(` where nothing was allowed to be. Thirteen sites across ten files were invisible for that reason alone. It was found by this block failing the ratchet it had just fixed, which is the selftest's argument made by accident.

**A backticked path is a path.** This surfaced only because the line-break fix let three of them be seen at all. The tree spells a path in backticks nearly everywhere, `script/lint` has a gate that assumes so, and a path citation written the tree's normal way was being judged as a gloss and failed against a title it was never quoting.

**`script/citations --selftest`**, sixteen fixtures, wired into `script/lint` ahead of `--check` the way `script/fatal-risks --selftest` is. Twelve exercise the scanner and four the path classifier. Three are near-misses that must stay quiet: a blank line between the number and the `(` is two paragraphs, a parenthesis further down the page belongs to its own sentence, and `bmilestone 5` is a word. The pre-filter is a named constant so the selftest reads the same string the gate runs rather than a second copy that can drift. Reverting the line-break fix or the pre-filter's letter turns the selftest red; both were checked.

## What it exposed

Measured on this branch against `main`, counting glossed citations the scanner can see:

| fix | citations newly visible | of those, ungrounded |
|---|---|---|
| line break | 51 | 12 |
| lettered milestone | 2 | 1 |
| emphasis marker | 36 | 8 |
| all three together | 91 | 21 |

Twenty-one is small enough to fix rather than ratchet, so `--check` stays a hard failure and no new tolerance was added. The count decided that; a large number would have wanted a ratchet on the new total instead, and saying so with a number is the difference between a decision and a guess.

Eighteen sites needed a correction, three were correct path citations the backtick fix now recognizes, and one of the eighteen is the known `26-fault-endpoint.md` case. Two are worth naming because they are the defect this gate exists for rather than a stale phrase:

- `notes/security.md` said SMP landed at **milestone 41 (dead code: triage the suppressions)**, which is not what that block is. Its own parenthetical named DECISIONS §11 (SMP: per-CPU run queues, message-based migration), which is what actually rules on it, so the sentence now cites the decision and drops the milestone number rather than guessing at a replacement.
- `design/roadmap/443-*.md` glossed milestone 365 (`xtask/src/main.rs` with no module structure) as "`xtask/src/main.rs` is 10,700 lines" against a title reading 6,785. The file grew and the gloss was never a title; it now names the structure rather than a number that keeps moving.

## BUGS

- The per-file gloss key ignores the letter. A file that glosses `milestone 19` answers the ratchet for `milestone 19d` as well. That is deliberate and matches the existing per-number rule, but it means a sub-part can ride on its parent's gloss. Only 20a has a record of its own today, so the exposure is small and the fix is a letter-aware key across `script/roadmap` too.
- A citation split across *two* line breaks is still invisible. The one-newline cap is what stops a stray `(` on line 40 pairing with a `)` on line 900, and nothing in the tree wraps that way today.
- The pre-filter now selects any file with a line ending in a citation. That is 204 files the scan did not open before. It costs time rather than correctness, and it is the reason to keep the selftest: if somebody later tightens the pattern for speed, the fixtures are what notice.

## Follow-on

- **Recorded.** The letter-insensitive gloss key, the two-newline limit, and the wider pre-filter are in this block's `BUGS` section, beside the gate they constrain.

## Index row

`script/citations` stopped seeing a citation whenever a paragraph reflowed the line break between the number and its `(`, and never saw a lettered milestone at all, because the candidate pre-filter's pattern had no letter in it. Both failures were silent: the gate stayed green while its coverage dropped, which cost three lanes a CI round trip each in one week and left `design/decisions/26-fault-endpoint.md` unscanned for its whole life. The scanner now reads across one line break, the pre-filter accepts a line that ends in a citation and a lettered number, a closing `**` no longer hides the `(` behind it, a lettered record with a file of its own is glossed against that file, and a backticked path is recognized as a path. Ninety-one citations became visible, twenty-one of them ungrounded and all twenty-one fixed. `script/citations --selftest` runs in `script/lint` ahead of `--check`, because a green check looks the same whether the scanner works or has quietly stopped seeing a shape.
