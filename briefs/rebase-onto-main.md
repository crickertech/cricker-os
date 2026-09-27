Rebase a branch onto `origin/main` and resolve its conflicts. You are in the branch's git worktree.
Do the work; do not ask questions.

    git fetch origin
    git rebase origin/main

## The conflicts you will hit, and exactly how each is resolved

These are known. Do not improvise; resolve them this way. Case 8 was added on 2026-09-26. Cases 4 to 7 were added on 2026-09-24
after a batch of seven conflicted pull requests found that the first three covered almost none of
what maintainer branches actually collide on.

### 1. `design/roadmap/README.md` shows as `DU` (deleted by us)

That file was deliberately retired
from the repository. The resolution is to accept the deletion:

    git rm -f design/roadmap/README.md
    git rebase --continue

It may recur on several commits in a row. Each time, same answer.

### 2. A file under `bench/` conflicts

The file is `baseline-aarch64.txt`, `baseline-riscv64.txt`, or `baseline-x86_64.txt`. These are
benchmark floors. Take `origin/main`'s version, never this
branch's, and never hand-merge the numbers: a baseline is a measurement of one specific binary, so
a merged file describes a binary nobody ever built.

    git checkout origin/main -- bench/
    git add bench/
    git rebase --continue

### 3. `notes/benchmarks.md` or `notes/benchmarks/*.md` conflicts

Both sides are *additions* of
different dated entries or appendix rows; neither replaces the other. Keep both, one after the other, and delete only the `<<<<<<<`,
`=======` and `>>>>>>>` marker lines.

### 4. Any additive index file conflicts

`notes/README.md`, `design/decisions/README.md`, and
anything else that is one entry per thing. Both sides are adding different entries at the same spot.
Keep both, delete only the marker lines, and preserve the file's own ordering:
`design/decisions/README.md` is numeric, `notes/README.md` is not, so there keep an order that reads
naturally.

If two rows claim the same decision section number, the row already on `origin/main` keeps the
number and this branch's row moves to the lowest free number above the highest on `main`. Rename
the file with `git mv`, change its H1 to match, and add one sentence to its status paragraph saying
it was renumbered, with the date, because a concurrently merged lane had taken the old number.

### 5. A heading was renamed on `main` while this branch inserted prose next to it

Git flags the
adjacency and it reads like a disputed heading. Take `origin/main`'s heading line, keep this
branch's added paragraphs, and hand-merge nothing.

This case has a mandatory check before you use it. Run `git log` and confirm the branch never
edited that heading itself. If it did, the two sides genuinely disagree and you stop. The case was
established on `notes/merge-queue.md` by verifying through `4924fa49c^` and `271cca0b5`, and that
verification is the only thing separating this rule from a guess.

### 6. `AGENTS.md` conflicts, and both sides are additions in different sections

Keep both.
If the two sides edit the same sentence or the same rule, stop. A rule's meaning in dispute is
an architect's call, not a lane's.

### 7. Both sides edit one line of a shell script under `helpers/`, touching different tokens

Combine them. The worked example, from `helpers/merge-drain.sh` on 2026-09-24:

- `origin/main`: `echo "$ME: dequeued #$num ($HELD_LABEL arrived after it was enqueued): $title"`
- the branch: `echo "merge-drain: dequeued #$num ($why arrived after it was enqueued): $title"`
- merged: `echo "$ME: dequeued #$num ($why arrived after it was enqueued): $title"`

`main` changed the prefix, the branch changed the label variable, so taking both loses neither.

Two mandatory conditions. The edits must touch different tokens; if both sides changed the
same token they disagree about what the line should say, and you stop. And every variable in your
merged line must be in scope on the merged result, verified by grepping for its definition on both
sides rather than assumed. `script/lint` does not execute these scripts, so a merged line naming a
variable only one side defines is a runtime bug that no gate here catches.

And rule 7 has a residue clause. When `main` makes a tree-wide mechanical change to a script (a
prefix, a helper call) and this branch *adds* a new line of the old shape, that line does not
conflict and merges in inconsistent. After resolving, grep the file for the old shape, not just
for conflict markers. On 2026-09-24 a branch added `echo "merge-drain: ARMED ..."` while `main` had
converted every other literal to `$ME:`, which would have left the script printing a mix of tagged
and untagged lines and defeated the tagging for the one event that branch existed to add.

Why `helpers/` needs its own case. `helpers/merge-drain.sh` has become what
`system_tests/src/user/tests.rs` already was: the one file every branch in a subsystem has to edit. When
`main` gained instance tagging for the watchers (`INSTANCE`, `ME`), every in-flight watcher branch
collided with it at once.

### 8. A block under `design/roadmap/` conflicts, and `main` put it in frontmatter

On 2026-09-26
milestone 596 (the roadmap blocks get frontmatter too) moved every block's `**Status:`,
`**Gate:` and `**Built:**` lines into frontmatter in one mechanical commit, so a branch that edited
its block before then conflicts at the top of the file. Take this branch's side of the block,
whole, then convert it; never hand-merge the frontmatter:

    git checkout --theirs -- design/roadmap/<N>-<slug>.md
    script/roadmap --migrate design/roadmap/<N>-<slug>.md
    git add design/roadmap/<N>-<slug>.md
    git rebase --continue

During a rebase `--theirs` is the commit being replayed, which is this branch's. The migrator is
deterministic, so the result is what `main` would hold plus this branch's edits, provided the
switch is the only change `main` made to that block. Check first with `git log --oneline
<merge-base>..origin/main -- <file>`: if it lists anything besides the switch, merge that prose by
hand below the frontmatter, or stop. A block this branch
*added* in the old form does not conflict, and `script/roadmap --check` fails on it afterwards with
the same command. If the migrator refuses a block (a `HARDWARE` gate or a `SUPERSEDED` status it has
no hand-read row for), write the frontmatter by hand from `notes/roadmap.md`'s table, and say so in
your report.

## What is not permitted, and stops

A rename on one side and an edit or a second rename on the other. `main` edited a file this branch moved, or the
reverse. Do not hand-resolve it, do not re-run the rename, and do not move files to make it fit.
Abort, and report how many such conflicts there were and which files. For a tree-wide mechanical
rename the answer is usually to redo the rename on top of `main` rather than to reconcile two
histories, and that is a decision rather than a rebase.

The tell for rename-versus-rename is a conflict marker carrying two paths:
`<<<<<<< HEAD:script/swish-check` against `>>>>>>> :script/shell-check`. And the prose in such a
branch is usually worse than mechanical: on 2026-09-24 one branch rewrote the watcher sections to say
`helpers/` while `main` had rewritten the same sections to move the watchers into scheduled Actions
workflows, so the two sides disagreed about facts rather than about a path.

A CSV in `notes/project-metrics/`. This is the `bench/` rule one level over: do not hand-merge
measurements. Both sides typically carry the same week measured at two different trees, so taking
either side discards the other's data. The resolution is to take `main`'s side and run
`script/metrics --update`, once, which is a measurement re-run rather than a rebase step. Abort and
say so.

Milestone 581 (one metrics file per measure) made this rarer than it was. Until 2026-09-23 the
directory held one 58-column `weekly.csv`, every metrics branch added columns to it, and two
branches adding disjoint measures conflicted with each other and with `main` over a header line
neither had read. A measure now lives in its own file, so a new measure is a new file and a new file
cannot conflict; what remains is two branches measuring the same week of the same measure.

A delete-versus-add hunk, even when one side is "just prose". Before treating a deletion as
housekeeping, check whether the region being deleted gained a new `##` section on `main`.
`git diff origin/main...HEAD -- <file>` will not show it; the hunk's `HEAD` side will. Silently
deleting a section somebody added is the exact failure this brief exists to prevent.

Anything that is none of the seven cases above. Run `git rebase --abort`, leave the branch
exactly as you found it, and say in your final message which file conflicted and quote what the two
sides were. Do not guess. A wrong resolution here is worse than no resolution, because it looks like
housekeeping and ships silently.

## After the rebase completes

Run these and fix what they report:

    script/lint
    script/roadmap --check
    script/citations --ratchet

Two failures are common after a rebase and both are yours to fix:

- A milestone number collision (`milestone N is claimed by two files`). The file *this branch*
  introduced moves to a free number; the one already on `origin/main` keeps it. Pick the lowest free
  number above 557. Rename with `git mv`, change the H1's number to match, and add a sentence to
  the status paragraph saying it was renumbered, with the date, because a concurrently merged lane
  had taken the old number.
- A compile error about a function taking more arguments than were supplied. Some function grew
  a parameter on `main` while this branch sat. Find another call site of that function on `main` and
  pass what it passes.

`script/citations --ratchet` reads the committed state, so commit before running it.

Those three are the whole of your gating, and that is deliberate. Do not run `script/test` or
`script/verify`. They are the heavy half, they belong in CI (`briefs/gate-in-ci.md`), and a
rebase that also gated would hold this machine's memory for an hour to prove something a runner
proves for free.

Do not push unless your brief says to. Do not enqueue, enable auto-merge, or merge. A rebase
re-presents the whole diff to `architect-label.yml` and `coe-architect-label.yml`, so
`needs-architect` may reappear; that is expected and not yours to remove. If an architect has
already ruled on this pull request's surface and the maintainer applied `architect-ruled`,
`architect-label.yml` skips re-adding it instead; `coe-architect-label.yml` never re-adds it in the
first place (its own detection asks what THIS push added, not what the whole diff still contains).
