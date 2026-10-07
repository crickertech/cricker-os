---
status: BUILT
raised: 2026-08-31
built: 2026-08-31
---
# 203. Nothing will ever tell us RedoxFS moved

The detection half shipped 2026-08-31: `script/vendor-watch` and the monthly
`.github/workflows/vendor-watch.yml`, with the report landing in `vendor/upstream-status.md` and the
update procedure in `vendor/README.md`'s "Bumping a pin" section. The fork below was settled the way
this block recommended, cheap first: the workflow raises the pin and lets `script/vendor-verify` go
red, and does not try to re-apply the divergence patch. The first run found something, which was
not the expected outcome: 0.9.1 is still the newest published version, and upstream git is 37
non-merge commits ahead of the pinned sha, including `fix: do not hardcode # of sectors per block`.
So the case this milestone was written for (a fix sitting on master long before it is published) was
live on the day the watch was built. Acting on it is a bump lane's job, not this one's. Minted
2026-08-31 by calef: *"a regular process to check if RedoxFS has
changed and if it does then initiate work to update our version and our divergences."* *(Number
provisional until the merge queue lands it.)*

In brief. The vendored engine is pinned at `redoxfs` 0.9.1 and no mechanism in this tree will
ever report that a newer version exists.

- `script/vendor-verify` proves the pin is what we *say* it is, byte-identical to the published
  tarball plus the divergence patch. It never asks what upstream has published since.
- Dependabot cannot see it. `vendor/redoxfs` is deliberately its own workspace, kept out of the
  nife workspace so the `-D warnings` clippy gate and `cargo fmt` never touch upstream code, so it is
  outside the graph dependabot watches. `.github/dependabot.yml` does not mention it.
- DECISIONS §81 (a dependency stays upgradable) governs exactly this failure and covers
  only the Cargo graph. `vendor/` is §81's blind spot, and the same argument applies: an upgrade
  nothing surfaces is an upgrade nobody takes.

## Why the gap costs more here than for an ordinary dependency

`vendor/README.md` records **five divergences**, and says of three of them that they are *"re-applied
forever, can conflict on a pin bump."* So the cost of a bump grows with the gap, and nothing is
counting the gap. That is the shape where waiting is silently expensive.

The narrow case that actually matters: a correctness fix upstream, in the engine holding backups.
That is the one where not knowing is not merely untidy.

## What it has to do

1. Detect, on both pins. `vendor/redoxfs.pin` records a crates.io version and an upstream git
   sha, and both have to be compared. The published version alone is the smaller question and it
   misses the case this milestone exists for: releases are months apart, so a correctness fix can sit
   in upstream git long before it is published. The check belongs beside `script/vendor-verify`,
   which already owns the pin.
2. **Report what changed, not that it changed.** A version number tells nobody whether to act.
   Carrying the changelog entries or commit subjects is what separates *upstream fixed a data-loss
   bug* from *upstream bumped a dev-dependency*, and that difference is the whole value of the
   prompt.
3. Monthly, and idempotent. Measured 2026-08-31: releases are irregular and clustered, 0.8.5 and
   0.8.6 landing the same day and 0.9.0 to 0.9.1 spanning five months. Weekly is noise; monthly's
   worst-case lag is shorter than the gaps. Clustering is also why the check must know what it has
   already reported: **two releases in a week must not produce two pull requests.**
4. **Run on a schedule that does not depend on anyone's laptop.** A GitHub Actions cron rather than
   the `launchd` watchers on patagonia, which is the one place this differs usefully from
   `notes/merge-queue.md`'s pair: calef accepted that a sleeping patagonia means nobody is watching,
   and that gap is fine for a merge queue measured in hours and wrong for a check measured in months.
5. Produce something a person acts on, not a log line nobody reads.
6. Carry the update procedure, so the person acting is not reconstructing it: bump the pin,
   re-apply the divergences, regenerate with `script/vendor-verify --write-patch`, extend
   `vendor/README.md`'s exhaustive list, and run the suite plus milestone 37's crash injector, since
   the store's safety claim is the thing a bump risks.

## Copy `toolchain-bump.yml`, which already does this for the other pin

This is a GitHub Action, and the tree has a working one to copy.
`.github/workflows/toolchain-bump.yml` watches the Rust nightly pin on a cron, and its own header
states the rule that answers the idempotence question above:

> One PR, updated in place on a fixed branch, rather than one per day.
>
> -- .github/workflows/toolchain-bump.yml

The same header goes on to say it is silent when there is nothing to do.

It force-pushes a fixed branch and opens or updates a single pull request. Pull request #587, which
merged 2026-08-31, is what that looks like in practice. `toolchain-drift.yml` is the sibling that
builds against the newest nightly without proposing anything, and `audit-cadence.yml` and
`stranger-cadence.yml` are two more scheduled reporters, so the shape is established four times over.

So step 5 produces a pull request on a fixed branch, updated in place. That is not a fork; it is
the tree's existing answer, and choosing anything else would need a reason.

What remains genuinely open is how much the pull request does. The cheap version bumps the pin
and lets `script/vendor-verify` fail, which makes the upgrade a visible object with a red check on
it. The expensive version also re-applies the divergence patch where it applies cleanly, so the pull
request arrives saying which of the five carried over and which conflicted, which is what dependabot
does for an ordinary crate.

Recommend the cheap version first. Automating the patch rebase before anyone has performed one by
hand is guessing at the shape of a job nobody has done; the first real bump is what would tell us
whether it is mechanical.

## A bump is allowed to fail, and that is a result rather than a defect

The block above reads as though a bump always succeeds. It may not. `vendor/README.md` says three of
the five divergences are re-applied forever and can conflict, and if upstream restructures the code
they touch, **they may not be re-applicable at all.**

The honest outcome then is a decision rather than a fix: take the new version and rewrite the
divergences, stay on 0.9.1 and record why, or fork permanently and stop pretending the pin tracks
upstream. A lane that meets this should stop and write the fork up rather than forcing a patch
through, because which of the three is right is not a lane's call.

We are current as of 2026-08-31, with 0.9.1 published 2026-07-01 and still the maximum. So the
first run of this check should report nothing, which is the cheapest possible way to verify the
mechanism works before it ever has to be right about something.

## BUGS

- It reports; it cannot decide. A newer version is not automatically a better one, and this tree
  pinned deliberately. The output is a prompt, and a prompt nobody acts on is the same silence with
  more steps. **This must not quietly become an agent that acts on its own**: calef declined that
  shape for the merge watchers, and the precedent here is `notes/merge-queue.md`'s, where the watcher
  reports and a maintainer session acts.
- **One consumer.** RedoxFS is the only vendored engine, so this generalizes to `vendor/` on paper
  and is really about one directory. Writing it as a general mechanism would be abstraction ahead of
  requirement.
- Watching git is noisier than watching releases, which is the cost of promoting it into scope:
  upstream commits land continuously and most of them will not matter, so the "report what changed"
  half is what keeps the git side from becoming the log line nobody reads.
- A bump is where the divergences get re-litigated, and divergence 3 in particular is one
  `vendor/README.md` says is kept rather than reverted only because reverting it is an architect's
  call. Any bump lane will meet that.

## Follow-on

- **Recorded.** It reports and cannot decide. A newer version is not automatically a better one,
  this tree pinned deliberately, and the output is a prompt a maintainer session acts on. It must
  not quietly become an agent that acts on its own, the shape calef declined for the merge watchers.
  `design/roadmap/0203-vendored-engine-upgrades.md`.
- **Recorded.** One consumer. RedoxFS is the only vendored engine, so this generalizes to `vendor/`
  on paper and is really about one directory; writing it as a general mechanism would be abstraction
  ahead of requirement. `design/roadmap/0203-vendored-engine-upgrades.md`.
- **Recorded.** Watching upstream git is noisier than watching releases, which is the cost of
  putting it in scope: most upstream commits will not matter, and the "report what changed" half is
  what keeps that side from becoming the log line nobody reads.
  `design/roadmap/0203-vendored-engine-upgrades.md`.
- **Recorded.** A bump is where the five divergences get re-litigated, and divergence 3 is kept
  rather than reverted only because reverting it is an architect's call. Three of the five are
  re-applied forever and can conflict, and if upstream restructures the code they touch they may not
  be re-applicable at all, at which point the honest outcome is a decision (rewrite the divergences,
  stay on the pin and record why, or fork permanently) rather than a forced patch.
  `vendor/README.md`.
- **Refused.** The expensive half of the workflow, which would re-apply the divergence patch and
  report which of the five carried over. Automating a rebase before anyone has performed one by hand
  is guessing at the shape of a job nobody has done; the cheap version raises the pin and lets
  `script/vendor-verify` go red, which makes the upgrade a visible object.
  This refusal is milestone 469 (design/roadmap/0469-automating-the-vendored-engine-rebase.md), which
  carries it with the condition that would change it.
- **Milestone 346.** Perform the first real RedoxFS
  pin bump. The watch's first run found upstream 37 non-merge commits ahead of the pinned sha,
  including `fix: do not hardcode # of sectors per block`, with 0.9.1 still the newest published
  version. Bumping means re-applying five divergences, regenerating with `script/vendor-verify
  --write-patch`, and re-running the suite plus milestone 37's crash injector. Nobody owns it, and
  this is the engine holding backups.

## Index row

calef, 2026-08-31: a regular process to check whether RedoxFS changed, and to initiate the update
of our version and our divergences when it does. `script/vendor-verify` proves the pin is what we
say it is and never asks what upstream published since; dependabot cannot see `vendor/redoxfs`
because it is deliberately its own workspace. `vendor/` is §81's blind spot, and the cost of
the gap grows with it: `vendor/README.md` records five divergences, three of them re-applied
forever and able to conflict on a bump. The narrow case that matters is a correctness fix upstream
in the engine holding backups.
