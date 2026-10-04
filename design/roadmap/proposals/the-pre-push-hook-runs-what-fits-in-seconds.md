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
out, #1559's hung, and lanes took to `--no-verify`. This proposal narrows milestone 630 (a
merge-queue ejection is caught before the queue, and recovered after it)'s hook. That milestone
widened it to all of `script/lint` the same morning.

## Measured on patagonia, 2026-10-03

| hook step | seconds |
|---|---|
| `script/fmt --check` | 1.9 |
| `script/lint --clippy`, warm | 33.6, 38.9 |
| `script/lint`, warm (load 11.6 to 19.3) | 70.3, 110.7, 119.4 |
| `script/lint`, cold target directory (a lane's first push) | 201.9 |

In the 70.3 s run, 33 cargo invocations took 37.0 s and the other 51 checks took 33.3 s. No check
took more than 3.7 s. The agent harness's Bash tool gives up at 120 s (read from that tool's
description).

## What each hook caught: CI lint failures on pull requests, by the hook the pushed tree carried

A run is put in a period by whether its head commit contains the hook change, because the hook a
push runs is the one in the tree being pushed. The counts cover every `pull_request` CI run from
09-19 to 10-03 in which the `clippy` job ran, which is the job that runs `script/lint`. The other
1,159 runs skipped that job, and cancelled runs are left out. All 43 failures were at the lint step.
Reading each log found a lint finding in every one, and no 503s, timeouts or cancellations.

| hook in the pushed tree | runs | lint failures | rate |
|---|---|---|---|
| fmt only, 09-19 to 09-25 | 511 | 29 | 5.7% |
| fmt only, 09-26 to 09-30 | 250 | 9 | 3.6% |
| fmt and clippy (`639681c37`, on `main` 09-29 20:38) | 34 | 2 | 5.9% |
| plus the prose gates and baselines (10-01 02:51 and 04:25) | 40 | 1 | 2.5% |
| all of lint (`41ef1d4e0`, on `main` 10-03 03:46) | 111 | 2 | 1.8% |

On 10-03 the full hook refused 11 pushes in Claude Code transcripts. 5 of those were real findings:
glosses 3 times, the prose ratchet once and clippy once. 6 were false. In 5 of them a claim push was
refused because "branch names milestone N, which has no roadmap block", which is the protocol's own
first push. The sixth was the GIT_DIR selftest defect, fixed the same day. That makes about 7 lint
failures in 116 pushes without the hook, about 6%, in line with the fmt-only weeks. So the full hook
does work. It also blocks claims, and it costs 70 to 202 s on every push.

Of the 38 fmt-only failures, 36 came from checks that read text: citations 13, the prose ratchet 9,
the icount baselines 6, and 8 others. 2 came from clippy.

## The change

The hook runs `script/fmt --check`, then lint's text checks (every check that is not a cargo
invocation, through a new `script/lint --no-cargo`), then the `--ready-branch` question. It skips
all three on a push whose commits change no files, which is a claim. Clippy stays in CI. That would
have caught 36 of the 38 fmt-only failures and 4 of today's 5 real catches. It costs about 33 s
under load, estimated from the 70.3 s run. That estimate has to be measured once the mode exists.

## The fork, answered

1. **Alternatives.**
   - *fmt only*, this proposal's first draft: refused, because the data above says the hook catches
     about 4 findings per 100 pushes.
   - *All of lint*: 70 to 202 s, past the timeout when cold, and it refuses claims.
   - *Clippy only*: 34 to 39 s, and it would have caught 2 of the 38.
   - *A hand-picked list*: refused, because the hook's header records that a list lagged every
     incident. "Not cargo" is a rule rather than a list.
2. **In the tree.** `script/lint --clippy` and `--baselines` are existing modes, and this adds a
   third.
3. **Prior art, recalled.** pre-commit setups commonly run the fast linters locally and leave the
   compiler-backed ones to CI.
4. **Premise, revised.** The first draft read "lint failed in 2 of 312 PR runs today" as the hook
   catching nothing. That figure was measured with the full hook in place, and it is the rate an
   effective hook produces. The fair comparison is the table above. A drop to 1.8% from 3.6% to
   5.7% is small in count (2 failures in 111 runs) but matches the 5 catches in the transcripts.
   Confounders: models and briefs changed on 09-26, which already moved the fmt-only rate from 5.7%
   to 3.6%. Lanes per day ran from 15 to 102 branches with CI runs, and 95 on 10-03. The middle
   periods have under 50 runs each. No z.ai transcript is searched.
5. **Cost.** A lint flag and a 10-line hook change. A miss costs one CI round trip of 2 to 5.5
   minutes of the clippy job plus a lane resume. A hit costs about 33 s, and an empty push about 2.
6. **Reversible.** Yes.
7. **Same cost?** Yes.

## BUGS

- The 33 s figure is an estimate from one run on a loaded laptop.
- A clippy finding now waits for CI. Clippy was 2 of the 38 fmt-only failures and 1 of the 5 real
  catches on 10-03.
