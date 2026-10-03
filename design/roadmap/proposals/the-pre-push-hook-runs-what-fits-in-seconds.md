---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The pre-push hook runs what fits in seconds

Raised by the lane that wrote the
[2026-10-03 queue correction](../../../notes/coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md),
failure 2. That day #1556's claim push sat in the hook for the whole lane, #1560's first push timed
out, #1559's hung, and lanes took to `--no-verify`. This proposal partly reverses milestone 630 (a
merge-queue ejection is caught before the queue, and recovered after it), which widened the hook
to all of `script/lint` the same morning.

## Measured on patagonia, 2026-10-03

| hook step | seconds |
|---|---|
| `script/fmt --check` | 1.9 |
| `script/lint --clippy`, warm | 33.6, 38.9 |
| `script/lint`, warm (load 11.6 to 19.3) | 70.3, 110.7, 119.4 |
| `script/lint`, cold target directory (a lane's first push) | 201.9 |

No single check is the problem. Of 84 checks the largest is spelling, at 3.7 s. 33 cargo
invocations take 37.0 s, and the other 51 checks take 33.3 s. The agent harness's Bash tool gives
up after 120 s by default (read from that tool's description), so a cold push is guaranteed to hit
it and a warm push on a busy machine sometimes does.

## The change

The hook keeps `script/fmt --check` and the `--ready-branch` question, and drops `script/lint`. The
full lint stays where it already runs: CI's `clippy` job on every push, which the drain reads before
it arms anything. Rung 2 stays the authority, and the courtesy shrinks to what fits in a few
seconds.

## The fork, answered

1. **Alternatives.** *Keep all of lint*: 70 to 202 s on every push, and it caught none of today's
   39 lint failures in the queue, because those were all union failures (see 4). *Skip lint only on
   an empty claim push*: that fixes #1556, but not #1559 or #1560, which had content. *Lint the
   merge with `origin/main`*: the same cost again, and `main` keeps moving after the push.
   *A hand-picked fast subset*: the hook's own header says that list lagged every incident, and that
   is why milestone 630 dropped it.
2. **In the tree.** The hook's header calls itself "a courtesy to the queue rather than a rule", and
   `--no-verify` is documented as legitimate.
3. **Prior art, recalled.** pre-commit and lefthook setups commonly run formatters locally and leave
   the slow linters to CI.
4. **Premise.** The hook lints the branch on its own. On pull requests, lint failed in 2 of 312 CI
   runs today, at 2 to 5.5 minutes of the clippy job each. Every lint failure in a group (31 on
   promotion, 2 on the prose ratchet, 6 on IN-PROGRESS) existed only in the union.
5. **Cost.** A 20-line deletion. Each lint miss costs a lane one CI round trip of about 5 minutes,
   instead of every push costing 1 to 3 minutes.
6. **Reversible.** Yes. It is one file, read at push time.
7. **Same cost?** Yes. This is about lane wall time and timeouts, not about effort.

## BUGS

- A lane that never looks at its CI will now learn about a lint failure later. Lanes end with
  `WAITING <run_id>` (CLAUDE.md), so the run gets read.
