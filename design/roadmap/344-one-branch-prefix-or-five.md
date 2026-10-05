---
status: BUILT
raised: 2026-08-18
built: 2026-08-18
---
# 344. The branch-prefix taxonomy is a gate enforcing a convention with one consumer

Built 2026-08-18, sixteen days before the proposal that asks for it was filed on
2026-09-03 by the milestone 247 sweep, from milestone 130's block; numbered 2026-09-19 by milestone
433, which checked the premise and found it already answered. calef asked what the taxonomy was for,
got the same answer this proposal reaches (nothing consumes it except the check), and retired it.
**The evidence, read in the tree on 2026-09-19.** `script/lint` check 4 is now titled "The one branch
prefix that carries a mechanism, guarded against near-misses": its `case` accepts `milestone/[0-9]*-*`
and everything that claims no milestone at all, refusing only a name that *looks* like a milestone
claim and would therefore skip check 4b in silence. `design/naming.md`'s "Branches" section carries
the vocabulary as a convention and states it plainly: *"This is a convention and not a gate, since
2026-08-18."* Both records carry the same four false rejections the proposal cites (`roadmap/`,
`gh-readonly-queue/*`, `dependabot/*`, `claude/*`) and the same conclusion, that a check which only
ever rejects valid work is measuring the wrong thing.

**One thing did not follow the decision**, and it is recorded rather than fixed here because
`design/decisions/` is the integrator's: §77 still describes the enforced allowlist as current.
Its `Status` is `DECIDED` on the 2026-08-16 vocabulary question, its prose says `script/lint` check 4
"accepts `milestone/`, `fix/`, `bench/`, `audit/`, `integration/`, `finalize/` and `feature/`, and
rejects everything else", and its "Practical impact until answered" paragraph describes a gate that
no longer exists. The proposal predicted exactly this (*"documented by a decision file that assumes a
premise nobody has rechecked"*), and it is the one live cost left.

**The proposal as filed on 2026-09-03**, kept because the argument is what the decision was made
on and because §77 still needs it. Everything below is written in the tense of that day and was
already out of date when it was written.

**In brief.** `script/lint` checks a branch name against a taxonomy of prefixes. Exactly one of
them, `milestone/N-`, is read by anything: §90's roadmap-block check parses the number out of it. A
grep found no other consumer. So the question is whether to retire the taxonomy down to that one
prefix and let every other branch be named freely, or to keep the list as a convention people are
expected to follow with a gate as its only enforcement.

## Why this matters

A gate that enforces a convention nothing consumes teaches contributors that the gates are
arbitrary, and this tree already has a scar from that exact check: AGENTS.md cites the
branch-prefix check as its worked example of a rung-two mechanism being *wrong about the tree*,
because it rejected the repository's second-commonest prefix, and it failed every group build in
the merge queue's first day by rejecting GitHub's own synthetic branch names. A check with one
consumer and a history of false rejections is a poor trade.

The argument the other way is real and is why this is a decision rather than a cleanup. A shared
vocabulary for branch names is worth something to a reader scanning `git ls-remote --heads`, which
AGENTS.md makes a required step before briefing a lane, and it is the only ledger one session has
of another session's work. Retiring the list does not delete that value; it removes the enforcement
and leaves the convention as prose, which this tree's own ladder says is rung four.

Either answer is fine and cheap. What is not fine is the current state, where the list is enforced,
partly unread, and documented by a decision file that assumes a premise nobody has rechecked.

## Where it came from

Milestone 130's Follow-on: *"Decide whether to retire the branch-prefix taxonomy down to
`milestone/N-`, the one prefix §90's roadmap-block check actually reads. A grep found nothing else
consumes it, so the rest is a gate enforcing a convention with no consumer.
`design/decisions/77-branch-prefixes.md` answers which prefixes belong on the list and assumes it
stays, so retiring it is an architect's call."*

## What answering it needs

Not a lane. The lookups are done: one grep established the single consumer, and the decision file
already exists to be amended. What is owed is a `**Status: PROPOSED.**` entry under
`design/decisions/` framing the two options with the false-rejection history attached, which is a
short piece of writing rather than an investigation.

## Follow-on

- **Done.** The retirement itself, on 2026-08-18 by calef. `script/lint` check 4 accepts
  `milestone/[0-9]*-*` and anything that claims no milestone; the taxonomy moved to
  `design/naming.md`'s "Branches" section as a convention, with the four false rejections and the
  reason a branch name rather than a label carries the one surviving mechanism.
- **Recorded.** `design/decisions/77-branch-prefixes.md` still describes `script/lint` check 4 as an
  allowlist of seven prefixes that rejects everything else, and its "Practical impact until
  answered" paragraph describes a gate that no longer exists. A reader meeting §77 first is told the
  wrong thing about what the tree does. Amending a decision is the integrator's, so this block
  records it where the promotion found it rather than editing that file.

## Index row

A gate enforcing a convention nothing consumes teaches contributors that the gates are arbitrary,
and this one had a scar: AGENTS.md cites the branch-prefix check as its worked example of a rung-two
mechanism being wrong about the tree, because it rejected the repository's second-commonest prefix
and then failed every group build in the merge queue's first day by rejecting GitHub's own synthetic
branch names. Milestone 130's block asked whether to retire the taxonomy down to `milestone/N-`, the
one prefix a mechanism reads. calef asked the same question on 2026-08-17 and answered it on
2026-08-18, before the proposal was written: the taxonomy is a convention in `design/naming.md` and
`script/lint` refuses only a milestone-lookalike, which is the one real hole the allowlist was
plugging, since `milestone-126-pgrep` with a hyphen would sail past check 4b. The residue is that
`design/decisions/77-branch-prefixes.md` still describes the allowlist as current, which is the
premise the proposal said nobody had rechecked.
