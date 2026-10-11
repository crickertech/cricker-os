---
status: PROPOSED
raised: 2026-10-11
milestone_dependencies: 586
decision_dependencies: 212, 213
machine_requirements: none
specific_machine: none
needs_person: no
---
# The ten longest notes become a page and its appendices

Raised 2026-10-11 (UTC) by a maintainer session, as the brief for one developer lane. It applies
[§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md) to the ten
longest over-cap documents a developer may edit. Each is split into a main page under 3,000 words
and appendices beside it. Nothing is cut except true duplication. Claim it with
`script/claim <branch> --debt-paydown`: this is compliance with a written standard
([notes/debt-paydown.md](../../../notes/debt-paydown.md), class 4).

Reuse: `helpers/prose_ratchet.py` measures, gates and banks; §212's siting and orphan rule already
exist; `notes/soak/`, `notes/visionfive2/`, `notes/merge-queue/` and `notes/footprint-perturbation/`
are appendix directories already in place. No code is written.

## The measurement

Taken 2026-10-11 (UTC) at `b98a74f` with the gate's own `measured()`, main body only. 1,711
documents are in scope and 140 are over 3,000 words. Together they sit 310,859 words above the cap.
The plain `measure()` reads 149 and 311,636, because it counts roadmap field tokens the gate treats
as syntax. The baseline, `design/prose-baseline.tsv`, holds 148 rows on words.

The ten longest by words are not ten a developer can take. Four are routed elsewhere (below), and
the next four notes by words replace them. Work them in this order, which is §212's rank of words
times citing files (`git grep -l` on the basename, the file itself excluded):

| order | document | words | above cap | citing files |
|---|---|---|---|---|
| 1 | `notes/visionfive2.md` | 12,805 | 9,805 | 115 |
| 2 | `notes/soak.md` | 13,076 | 10,076 | 81 |
| 3 | `notes/verification.md` | 9,888 | 6,888 | 76 |
| 4 | `notes/confinement-claims.md` | 8,967 | 5,967 | 75 |
| 5 | `notes/fs-server.md` | 10,358 | 7,358 | 56 |
| 6 | `notes/merge-queue.md` | 9,278 | 6,278 | 47 |
| 7 | `notes/x86-uefi-boot.md` | 8,072 | 5,072 | 52 |
| 8 | `notes/smb.md` | 10,296 | 7,296 | 32 |
| 9 | `notes/glyphs.md` | 8,368 | 5,368 | 28 |
| 10 | `notes/footprint-perturbation.md` | 8,082 | 5,082 | 27 |

The ten hold 69,190 words above the cap, 22% of the tree's excess. `notes/unsafe-obligations.md`
(8,139) was skipped for the backfill because open pull request #1892 edits it.

## The four that stay out, and where each goes

- `design/roadmap/0047-navigation-and-naming.md` (16,257) and
  `design/roadmap/0139-drive-down-unsafe.md` (14,093). calef ruled on 2026-09-25 (UTC) that
  neither block is split, and each carries its marked exception. The gate already excuses them, so
  they are not debt. Leave them alone.
- `design/decisions/0139-cycle-counter-authority.md` (9,481, 17 citing files). A developer never
  edits `design/decisions/`. It is a DECIDED section calef ratified, and its text is what other
  files quote, so a split moves words he signed and breaks glosses that point at them. It goes to
  calef as open question 1 below. If he says split, the maintainer does it on its own branch. He
  said split on 2026-10-11 (UTC), and question 1 records where it was done.
- `design/roadmap/0161-x86-64-kernel-port.md` (9,655, 8 citing files). It is another milestone's
  block, which a developer may not edit. Roadmap blocks as a class stay under the cap, so it owes a
  split, but its rank is low (77,240 words times readers against 218,214 for the lowest note
  here). The maintainer splits it in its own change when a slot is free. No lane is cut for it.

## How a document is split

The completeness rule comes first, and no gate checks it. A reader must be able to read the main
page and act on it without opening an appendix. An appendix verifies or challenges the main page;
it never carries the argument. Keep in the main page what the document is for, the procedure or
claim a reader came for, and every conclusion. Move out what supports a conclusion: run tables,
dated evening logs, per-board transcripts, superseded designs and their refusals, long worked
examples. Each moved block leaves behind a sentence stating what it found, plus a link.

Siting is §212's default. The appendices to `notes/X.md` go in `notes/X/<topic>.md`. Each appendix
is under 3,000 words and linked from `notes/X.md` or `notes/X/README.md`. The ratchet's orphan check
fails an unlinked one. Where the directory exists, extend it and its README. Where it does not,
create `notes/X/README.md` with a one-line purpose and a `Name:` paragraph, which `script/names`
requires of every documentation directory (§75 (directories under `design/` and `notes/` carry
provenance in their own README)).

Every directory and file name minted here is provisional, and calef ratifies it. Write each one into
the README's `Name:` paragraph, or a `` `stem` Name: `` paragraph for a stem, saying so with the
date and branch. `notes/soak/README.md` is the model. Name an appendix for its content, not its
position: `bring-up-log.md`, not `appendix-a.md`.

Move, do not cut. Every fact in the old document lands in the new main page or an appendix. The one
exception is true duplication: a passage that restates what another file already says may be
replaced by a sentence and a link to that file. Say which passages went that way in the commit
message.

## What the gates will do to you

Read these before the first move. Each one is a failure a careless split hits.

1. A new appendix is a new document, absent from the baseline, so it meets every limit outright: a
   median sentence of 20 words, no sentence over 40, at most 4 bold spans per 1,000 words. The
   parents carry sentences of 67 to 122 words today. A long sentence must be split as it moves.
2. Bold density is judged on any document a change touches. A parent cut to 3,000 words may hold
   at most 12 bold spans. `notes/visionfive2.md` has 51 today, so most bold goes.
3. `script/lint`'s link check strips `#anchor` before resolving, so a link to a heading that moved
   still passes and silently lands on the wrong page. No inbound anchor links exist today. Before
   each commit, run `git grep -n 'X.md#'`, read each hit (a relative link names only the basename, so
`the-confinement-claims.md#` matches too), and repoint any that land on your page.
4. `script/citations --ratchet` checks that a quoted gloss appears in its target's body. Text moved
   out of a parent breaks every gloss quoting it. Fix the citing file to point at the appendix. It
   reads the committed tip, so commit first ([briefs/gate-a-lane.md](../../../briefs/gate-a-lane.md)).
5. The first `--bank` rewrites far more than your rows. The committed baseline is out of the order
   `--bank` writes, and about 86 rows would fall or drop out, shrinks nobody banked since at least 2026-10-07. So the
   lane's first commit is a bank of the tree as it stands, with no prose change. Later banks then
   touch one row each.

`design/prose-baseline.tsv` is under `design/`, which a developer does not edit. This brief grants
the exception for that one file: it is written by `--bank` and never by hand.

## Commits

One commit for the opening bank, then one commit per document. Each document's commit holds the
new main page, its appendices and README, every inbound link it repointed, and
`python3 helpers/prose_ratchet.py --bank` run after the move. The message says what moved where and
names any passage replaced as duplication. On a rebase that conflicts in the baseline, take `main`'s
file and run `--bank` again; never merge its rows by hand
([briefs/rebase-onto-main.md](../../../briefs/rebase-onto-main.md), the baseline rule).

Before starting each document, check that no open pull request edits it
(`gh api 'repos/nifeos/nife/pulls?state=open'`, then each pull request's files). If one does, skip
it, record why in this block, and take it last. `notes/confinement-claims.md` is the likeliest,
since each outsider pass adds to it.

## Done when

1. `script/lint`, `script/roadmap --check`, `script/fmt` and `script/citations --ratchet` pass.
2. Each of the ten main pages is under 3,000 words by
   `python3 helpers/prose_ratchet.py --report`, or carries a marked `prose-budget` exception with a
   date and a reason. A developer may propose one but not grant it; it waits for calef.
3. Every appendix is under every limit and linked, and the baseline rows for the ten are lowered or
   gone.
4. This block records, per document, the words before and after and what moved.
   Result: lane `lane/ten-longest-notes`, PR #1918, put all ten main pages under 3,000 words and
   removed all ten baseline rows. Words before and after: visionfive2 12,805→~2,903, soak
   13,076→~2,577, verification 9,888→~2,303, confinement-claims 8,967→~2,048, fs-server
   10,358→~2,597. Then merge-queue 9,278→~2,458, x86-uefi-boot 8,072→~2,325, smb 10,296→~1,639,
   glyphs 8,368→~2,981, footprint-perturbation 8,082→~2,752. What moved, per document, is in
   #1918's body.
5. The report names each provisional directory and file name for calef's queue.

## Open questions for an architect

1. Resolved: split, by the maintainer (calef, 2026-10-11 UTC). The question was whether
   `design/decisions/0139-cycle-counter-authority.md` is split or gets a marked exception like
   milestones 47 and 139. The recommendation was to split it: 17 files cite it, and a decision is
   read in order to decide, the case §212 wrote the cap for. The maintainer did the split on
   `maintainer/split-decision-0139`.
2. The appendix names, as for every split. The lane ships them provisional and keeps going.

## Lane shape

One lane, not two. All ten are under `notes/`, so there is no `design/roadmap/` half for a second
lane to take; the four `design/` files are not developer work. Two lanes on disjoint notes would
still share `design/prose-baseline.tsv`, and every bank in each would conflict with the other.
That is cheap to resolve but it lands on the merge queue, which is the bottleneck. The maintainer
skill sets lane count by collision surface, and here the surface is one file every commit touches.
If the lane is slow, cut a second one after the first merges, on the next ten.

## Where it sits in the ranking

Off the customer path, like all §212 work. It ranks by reading cost: their citing-file counts sum
to 589, and each arrival today loads 8,000 to 13,000 words to find one fact.

## Index row

The ten longest notes a developer may edit, split under §212's 3,000-word cap into a main page a
reader can act on and appendices that hold the evidence, with the shrink banked.
