---
status: NOT-STARTED
raised: 2026-09-17
promoted_from: a-cadence-job-whose-healthy-state-is-red
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 422. A cadence job whose healthy state is red, and nothing that notices it repeating

Promoted from the proposal `a-cadence-job-whose-healthy-state-is-red`,
filed 2026-09-17 by milestone 311, which was minted to fix a path and found that the path was the
smaller half. *(Number provisional until the merge queue lands it.)*

It reads GitHub's run history, the same source `script/cadence-check` already reads.

**Premise re-checked 2026-09-19 and still true, and the overdue run is now five weeks long.**
`helpers/trunk-health.sh` still reports nothing about a repeated failure, and `script/cadence-check`
still asks only when a workflow last succeeded. `script/audits --due` still names `documentation`,
whose last sweep is 2026-08-17 and whose milestone trigger now reads +122 against a threshold of 10.
The security sibling is the one thing that moved: it was audited on 2026-09-17 and is no longer due,
so `documentation` is now the only standing example.

## In brief

`.github/workflows/audit-cadence.yml` reports "an audit is due" by **failing**. That is deliberate
and correct: its own header argues, at length, that an audit coming due is information about the tree
rather than a defect in a commit, which is why it is a scheduled workflow instead of a gate.

The consequence nobody priced is that **red is now ambiguous**. These two rows look identical in the
Actions tab:

```
2026-09-07  audits: 7 on record, DUE: documentation, security   exit 1     <- working perfectly
2026-09-14  FileNotFoundError: .../user/Cargo.toml              exit 1     <- broken for four days
```

And these four look identical to each other, which is the part that actually cost something:

```
2026-08-17  audits: 5 on record, DUE: documentation, security   exit 1
2026-08-24  audits: 7 on record, DUE: documentation, security   exit 1
2026-08-31  audits: 7 on record, DUE: documentation, security   exit 1
2026-09-07  audits: 7 on record, DUE: documentation, security   exit 1
```

**An audit was overdue every week for a month, the mechanism said so on schedule every time, and no
audit was run.** Milestone 92 built this tripwire to stop auditing depending on somebody remembering
to ask. The tripwire fired four times and auditing still depended on somebody remembering.

## Why `script/cadence-check` does not cover it

Milestone 238's check asks **when a workflow last succeeded**, and reports one whose last success is
more than fifteen days old. That is the right question for a job whose healthy state is green, and it
is why the mutation and undefined-behaviour workflows were caught.

For `audit-cadence` it is the wrong question in both directions. A job that is *supposed* to be red
has no green to be stale against, so a firing tripwire is indistinguishable from a dead one; and a
job that has **never** succeeded is reported forever on a condition it can never clear, which
generates exactly the standing alarm that teaches a reader to stop looking. It caught milestone 311's
path bug by coincidence: this workflow had never succeeded, so it tripped the never-succeeded arm,
for a reason unrelated to the defect.

## What it would report

The proposal is to report **a repeat, not a colour**: a scheduled workflow whose last N runs all
failed *with the same final line*. Sketch, for whoever builds it:

- Group the last six scheduled runs per workflow by the final line of their failing step.
- Report a workflow where the newest group has three or more runs in it, with the line and the span
  of dates.
- A workflow whose failures are all *different* is churning, and is a separate and less urgent
  finding; say so rather than folding it in.

**The delivery is `helpers/trunk-health.sh`, not a new scheduled workflow**, for the reason
`script/cadence-check`'s header already states and this proposal must not undo: a scheduled workflow
that watches scheduled workflows dies the way its subjects die.

## The open question, which is what makes this a proposal rather than a lane brief

**Who is the report for, and what closes it?** A repeat report on `audit-cadence` says "run the
audit", and running an audit is a lane and a day, not a fix somebody applies between tasks. Reported
to a maintainer session it becomes rung four again, a message somebody has to read; reported into a
pull request there is no pull request to hang it on. AGENTS.md's answer for work waiting on a person
is the `needs-architect` label and a `## What I need from you` comment, and neither has a home when
the finding belongs to no branch.

That is an architect's call, not a lane's, and it is the same question the steward's own "it
reported and never acted" failure of 2026-08-04 raised without settling.

## What this does not propose

**Not making the job green when an audit is due.** A cadence checker that cannot report "overdue" is
decoration, and the workflow's own header and `design/audit-reports/README.md`'s `BUGS` both already
say that closing this by editing the index is the one thing that makes the mechanism a lie. The
ambiguity is the cost of a correct design, and the fix is a second signal rather than a quieter
first one.

## Index row

`.github/workflows/audit-cadence.yml` reports that an audit is due by failing, which is deliberate
and correct, and the consequence nobody priced is that red is ambiguous: a firing tripwire and a
`FileNotFoundError` are the same row in the Actions tab, and so are four consecutive weeks of the
same true report. An audit was overdue every week for a month, the mechanism said so on schedule
every time, and no audit ran, which is milestone 92's tripwire firing four times into a process that
still depended on somebody remembering. `script/cadence-check` cannot cover it, because a job whose
healthy state is red has no green to be stale against and a job that never succeeded is reported for
ever. The proposal is to report a repeat rather than a colour, from `helpers/trunk-health.sh` rather
than a scheduled workflow that would die the way its subjects die. What makes it a block rather than
a brief is the open question of who the report is for, since running an audit is a lane and a day
and there is no pull request to hang the finding on.
