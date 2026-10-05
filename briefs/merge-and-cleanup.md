**Provisional name.** Clean up after a lane's pull request merges. You hold the maintainer or the
steward role, a pull request you were watching has just landed, and the lane that produced it is
finished. Do the work; do not ask questions.

Where this came from. The three clauses below lived in `AGENTS.md` (the merge-checklist line,
the prune-the-worktree paragraph, and the `nife-dev` relink), moved here on 2026-09-23 by the
extraction that milestone 579 (which of the constitution must be carried, and which is a brief)
proposed. They fire at one event and are performed by one role, which is the whole test that moved
them. The constitution keeps a pointer and keeps the one clause that is genuinely ambient, which is
that every lane takes `nife-dev` and nobody should try to stop it.

Why the mechanics belong in a file rather than in prose nobody re-reads. The failure this
prevents is the one failure in this system that destroys work rather than delaying it, and it has
happened at two scales. Eight finished worktrees accumulated before anyone looked, one of them
holding 3.3 GB. Then on 2026-07-31 the volume hit zero bytes free with 42 worktrees holding
78 GB, two lanes died mid-work, and those lanes could not even run `pgrep` to check for leaked
emulators, because every tool must create an output file before it runs. The warning signs had been
noted hours earlier, not acted on, and four more lanes were launched on top of them.

## Before you remove anything: is the lane actually finished

If a lane is blocked rather than merged, commit and push its work before removing anything. A
snapshot on the remote cannot be lost by a cleanup; an uncommitted worktree can. This clause is
carried verbatim from the constitution because it is the one that stops a cleanup from becoming a
loss.

    git -C <worktree> status --short

Anything printed is uncommitted work. Commit it and push the branch before continuing, even if the
commit message is `checkpoint` and the branch will never be merged. Do not prune a dirty worktree to
save a step.

## The four steps

### 1. Every piece of identified work in the lane's report has a home

The lane's final report names work it found and did not do. Each such item is in one of exactly two
shapes, and "worth doing someday" is neither:

- a proposed milestone, provisional, whose number you mint at merge like every other global name;
  or
- a recorded limitation in the `BUGS` section beside the feature a reader meets.

A finding with no home holds the merge. If the pull request has already landed and you find a
homeless item in the report, file it now rather than deciding it was minor: a lane report is read
once, by one person, on the day it is written, which is exactly why this check is a step in a
checklist and not a habit.

### 2. The branch

Nothing to do. `delete_branch_on_merge` is `true` on this repository, so GitHub deletes the remote
branch itself. Prune your own stale remote-tracking refs when they accumulate:

    git fetch --prune origin

The local branch in the worktree goes away with the worktree in step 3.

### 3. Prune the worktree, in the same breath

    git worktree remove <worktree>
    git branch -d <branch>
    git worktree prune

**Deleting the branch does not remove the ~2 GB of `target/` behind it.** That is the whole reason
this step is spelled out: a branch deletion looks like cleanup and reclaims nothing. Measured on this
machine on 2026-09-23, ten lane worktrees held between 1.0 GB and 4.8 GB of `target/` each.

`git worktree remove` refuses a worktree with modifications, which is the safety net behind the
check at the top of this brief and the reason not to reach for `--force`. If it refuses, go back and
find out whose work you are about to delete.

### 4. Relink `nife-dev` from the main checkout

`nife-dev` is one `rustup` symlink for the whole user account, not one per worktree, so it means
whichever worktree ran `xtask std-src` last. Every lane that gates takes it, unavoidably, because
`script/test` calls `std_src()` transitively and a fresh worktree always has a cold farm. That is
expected and is not a lane misbehaving. The integrator's duty is to put it back:

    cd "$(git rev-parse --path-format=absolute --git-common-dir)/.."
    rustup toolchain link nife-dev "$(pwd)/target/nife-farm"

The `cd` lands in the main checkout from any worktree on any host. In a cloud session where nothing
gated locally, `rustup toolchain list | grep nife-dev` prints nothing, and there is nothing to relink.

Run it from the main checkout, never from a lane worktree, and run it after pruning rather than
before, since pruning a worktree the link points into leaves it dangling.
`notes/std.md` has the mechanism, the 2026-08-18 cross-contamination that prompted the rule, and why
relinking loudly still does not make concurrent lanes safe.

## EXAMPLES

Checking what the machine is actually holding, before and instead of guessing (real output, this
machine, 2026-09-23):

    $ git worktree list
    /Users/calef/projects/nife                       34d7d89f0 [main]
    /Users/calef/projects/nife-worktrees/a3          e6958bc5e [maintainer/installing-is-granting]
    /Users/calef/projects/nife-worktrees/charter     7fc963909 [maintainer/a-brief-is-not-about-the-reader]
    ...
    /Users/calef/projects/nife-worktrees/m315        10c669ecd [milestone/315-port-revocation-two-core]

    $ du -sh ~/projects/nife-worktrees/*/target
    1.0G    /Users/calef/projects/nife-worktrees/a3/target
    1.1G    /Users/calef/projects/nife-worktrees/charter/target
    ...
    4.8G    /Users/calef/projects/nife-worktrees/m315/target

    $ df -h /
    /dev/disk3s1s1   460Gi    12Gi   115Gi    10%   /

A clean removal of a merged lane:

    $ git -C ~/projects/nife-worktrees/m315 status --short
    $ git worktree remove ~/projects/nife-worktrees/m315
    $ git branch -d milestone/315-port-revocation-two-core
    Deleted branch milestone/315-port-revocation-two-core (was 10c669ecd).
    $ git worktree prune

And the relink, confirmed by reading it back:

    $ rustup toolchain list -v | grep nife-dev
    nife-dev /Users/calef/projects/nife/target/nife-farm

If that path names a worktree under `nife-worktrees/` rather than the main checkout, the link is
pointing at a lane and step 4 has not been done (or has been undone by a lane that gated since).

## Stop, do not improvise

Three things are outside this brief. Stop and hand them back rather than guessing:

- **A worktree with uncommitted changes whose lane you cannot identify.** Commit and push it on its
  own branch and say so. Do not remove it.
- **`git worktree remove` failing for any reason other than modifications**, such as a locked
  worktree or a path that no longer exists.
- **A pull request that merged with its report naming work you cannot classify** as either a proposed
  milestone or a `BUGS` entry. That classification is judgment; say what you found and let the
  maintainer place it.

## BUGS

- Nothing here fires on its own. This is rung three of the constitution's ladder, a written
  record at the thing itself, and it only helps a reader who opens it. The two failures it records
  both happened to people who knew the rule. The steward's interval check for lane worktrees at risk
  is the nearest thing to a mechanism, and it watches for uncommitted work rather than for disk.
- There is no gate on disk headroom, so nothing warns before a write fails. `df -h /` is a manual
  step this brief cannot make anyone run, and the failure arrives as an unrelated command dying with
  `No space left on device`.
- Step 1 is unmechanisable on purpose. No check can tell an intention from an observation in
  prose; a lint that tried would carry `git grep -w TODO`'s 82% false-positive rate wearing a
  different hat. So the homeless-finding check is a human reading a report, and it has already failed
  once at scale: milestone 94 (the untracked-work sweep, and the convention that ends the category)
  left its own inventory in a pull request body for twelve days, by which point the item-level list
  was gone and had to be re-derived (`notes/untracked-work-sweep.md`).
- **The `nife-dev` relink is racy and this brief does not fix that.** Another lane can gate and take
  the link between step 4 and the next time anyone looks. `notes/std.md` records why relinking loudly
  does not make concurrent lanes safe; there is no version of this step that stays true.
- The example paths are this machine's. The main checkout is `/Users/calef/projects/nife` and
  lanes live under `~/projects/nife-worktrees/`; nothing here is portable to another machine, and
  nothing checks that.
