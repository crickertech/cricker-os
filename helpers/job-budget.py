#!/usr/bin/env python3
"""A merge-group CI job has a wall-time budget, and nothing may exceed it quietly.

    helpers/job-budget.py check <job> <started-epoch>   # the last step of every ci.yml job
    helpers/job-budget.py lint                          # script/lint: every job carries the check
    helpers/job-budget.py --selftest

Milestone 721 (each merge-group CI job has a 20-minute budget), provisional
number. calef ruled it 2026-10-03 UTC: "I think 20 minutes is a decent target").

# WHY

Nothing in the tree read CI wall time. The long merge-group job went from 12 to 22 minutes in one
merge on 2026-09-19 and stayed there; when it hit its 30-minute bound the bound was raised to 45, one
line that cost nothing, and on 2026-09-30 the job ran into the new bound 29 times. A budget of 20
minutes would have failed the first 22.3-minute run, about 10.5 days earlier. An absolute number
beat a relative one (1.5 times the trailing median fires, twice does not) because a reader can
check a number.

# THE RULE

- Every `ci.yml` job that checks out the tree opens with a step that records `JOB_STARTED_AT`, and
  closes with an `if: always()` step that runs `check`.
- `check` warns at 75% of the budget (15 of 20 minutes) and fails above it.
- The budget is `BUDGET_MINUTES` unless `.github/ci-job-budgets` raises it for that job. A line there
  is `<job> <minutes> <reason>`, and a line without a reason is refused, so a budget grows only by
  saying why in the diff that grows it.
- `timeout-minutes` is set on every job and is at most the budget plus `MARGIN_MINUTES`, so creep
  ends a job at 25 minutes rather than burning 45 a group. A job with no `timeout-minutes` gets
  GitHub's six hours, which is how the old bound was never a bound.
- A job with no `actions/checkout` has no `helpers/` to run, so it is exempt from the two steps and
  still needs the timeout. No ci.yml job is exempt today: `gate` checks out the tree for the
  prose-only classifier (helpers/prose_only.py) since 2026-10-05.

`lint` is the rung-two mechanism: a new job added without the steps fails `script/lint`, so the
budget does not depend on anyone remembering to give the next job one.

# BUGS

- Only `.github/workflows/ci.yml` is covered. `verify.yml` also runs on `merge_group` with 45 and 60
  minute bounds, but no median for its jobs has been measured, so a 25-minute cap there would be a
  guess. Add it to `WORKFLOWS` once its medians exist.
- The clock starts at the job's first step, so runner provisioning and image setup before it are not
  counted. That is seconds, not minutes, on hosted runners.
- A job that hits `timeout-minutes` is killed before its last step runs, so it reports the timeout
  rather than the budget. At 25 against 20 that is the intended behavior.
"""

import os
import re
import sys
import time

BUDGET_MINUTES = 20
WARN_FRACTION = 0.75
MARGIN_MINUTES = 5
WORKFLOWS = [".github/workflows/ci.yml"]
RATCHET = ".github/ci-job-budgets"

START_MARK = "JOB_STARTED_AT"
CHECK_MARK = "helpers/job-budget.py check"


def parse_ratchet(text):
    """`{job: minutes}` and a list of problems. A raise needs a reason and has to be an increase."""
    budgets, problems = {}, []
    for n, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 2)
        if len(parts) < 3 or not parts[1].isdigit():
            problems.append("%s:%d: want `<job> <minutes> <reason>`, got %r" % (RATCHET, n, line))
            continue
        job, minutes, reason = parts[0], int(parts[1]), parts[2]
        if minutes <= BUDGET_MINUTES:
            problems.append("%s:%d: %s is raised to %d, which is not above the default %d"
                            % (RATCHET, n, job, minutes, BUDGET_MINUTES))
        if not reason.strip():
            problems.append("%s:%d: %s has no reason" % (RATCHET, n, job))
        budgets[job] = minutes
    return budgets, problems


def budget_for(job, ratchet):
    return ratchet.get(job, BUDGET_MINUTES)


def check(job, started, now, ratchet):
    """(exit code, message). `started` and `now` are epoch seconds."""
    minutes = (now - started) / 60.0
    budget = budget_for(job, ratchet)
    warn_at = budget * WARN_FRACTION
    if minutes > budget:
        return 1, ("::error::%s took %.1f minutes against a budget of %d. Make it faster or split it; "
                   "raising the budget is a line with a reason in %s." % (job, minutes, budget, RATCHET))
    if minutes >= warn_at:
        return 0, ("::warning::%s took %.1f minutes, past the %.0f-minute warning for its %d-minute "
                   "budget." % (job, minutes, warn_at, budget))
    return 0, "==> %s took %.1f minutes (budget %d)" % (job, minutes, budget)


def jobs_of(text):
    """`{name: [lines]}` for the jobs under the top-level `jobs:` key, by indentation alone.

    No YAML library: this runs in `script/lint` on a bare checkout, and the file's job keys sit at two
    spaces, which is all that is needed to split them."""
    lines = text.splitlines()
    out, name, in_jobs = {}, None, False
    for line in lines:
        if re.match(r"^jobs:\s*$", line):
            in_jobs = True
            continue
        if in_jobs and re.match(r"^\S", line) and not line.startswith("#"):
            break
        if not in_jobs:
            continue
        m = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if m:
            name = m.group(1)
            out[name] = []
        elif name is not None:
            out[name].append(line)
    return out


def steps_of(body):
    """Each step of a job as its list of lines, or None when the job has no `steps:`."""
    start = next((i for i, l in enumerate(body) if re.match(r"^    steps:\s*$", l)), None)
    if start is None:
        return None
    steps, cur = [], None
    for line in body[start + 1:]:
        if line.strip() and not line.lstrip().startswith("#") and len(line) - len(line.lstrip()) <= 4:
            break
        if re.match(r"^      - ", line):
            cur = [line]
            steps.append(cur)
        elif cur is not None:
            cur.append(line)
    return steps


def timeout_of(body):
    for line in body:
        m = re.match(r"^    timeout-minutes:\s*(\d+)\s*$", line)
        if m:
            return int(m.group(1))
    return None


def lint_workflow(path, text, ratchet):
    problems = []
    for job, body in jobs_of(text).items():
        where = "%s job %s" % (path, job)
        budget = budget_for(job, ratchet)
        timeout = timeout_of(body)
        if timeout is None:
            problems.append("%s: no timeout-minutes; GitHub's default is six hours" % where)
        elif timeout > budget + MARGIN_MINUTES:
            problems.append("%s: timeout-minutes %d is over budget %d plus %d; lower it, or raise the "
                            "budget in %s with a reason" % (where, timeout, budget, MARGIN_MINUTES, RATCHET))
        steps = steps_of(body)
        if steps is None:
            problems.append("%s: no steps" % where)
            continue
        if not any("actions/checkout" in "\n".join(s) for s in steps):
            continue
        first, last = "\n".join(steps[0]), "\n".join(steps[-1])
        if START_MARK not in first or "GITHUB_ENV" not in first:
            problems.append("%s: the first step must record %s" % (where, START_MARK))
        if CHECK_MARK not in last or not re.search(r"^\s+if:\s*always\(\)\s*$", last, re.M):
            problems.append("%s: the last step must be `if: always()` running `%s`" % (where, CHECK_MARK))
    return problems


def lint(root="."):
    ratchet_text = ""
    try:
        with open(os.path.join(root, RATCHET)) as f:
            ratchet_text = f.read()
    except FileNotFoundError:
        pass
    ratchet, problems = parse_ratchet(ratchet_text)
    seen = set()
    for wf in WORKFLOWS:
        with open(os.path.join(root, wf)) as f:
            text = f.read()
        seen |= set(jobs_of(text))
        problems += lint_workflow(wf, text, ratchet)
    problems += ["%s: %s names no job in %s" % (RATCHET, j, ", ".join(WORKFLOWS))
                 for j in ratchet if j not in seen]
    return problems


GOOD = """\
name: CI
jobs:
  quick:
    runs-on: x
    timeout-minutes: 25
    steps:
      - name: clock
        run: echo "JOB_STARTED_AT=$(date +%s)" >> "$GITHUB_ENV"
      - uses: actions/checkout@v5
      - run: make
      # a trailing comment
      - name: budget
        if: always()
        run: python3 helpers/job-budget.py check "${{ github.job }}" "$JOB_STARTED_AT"
  gate:
    runs-on: x
    timeout-minutes: 10
    steps:
      - run: gh api x
"""


def selftest():
    ok = True

    def expect(label, got, want):
        nonlocal ok
        if got != want:
            ok = False
            print("FAIL %s: got %r want %r" % (label, got, want), file=sys.stderr)

    # check(): quiet under 15, warns from 15, fails above 20, and a ratchet moves both lines.
    expect("quiet", (check("j", 0, 14 * 60, {})[0], check("j", 0, 14 * 60, {})[1][:3]), (0, "==>"))
    expect("warn", check("j", 0, 15 * 60, {})[1].startswith("::warning::"), True)
    expect("edge ok", check("j", 0, 20 * 60, {})[0], 0)
    expect("fail", check("j", 0, 20 * 60 + 1, {})[0], 1)
    expect("22.3 minutes fails (the 2026-09-19 run)", check("j", 0, int(22.3 * 60), {})[0], 1)
    expect("ratchet raises", check("j", 0, 22 * 60, {"j": 24})[0], 0)
    expect("ratchet is per job", check("k", 0, 22 * 60, {"j": 24})[0], 1)
    expect("ratchet warn line moves", check("j", 0, 16 * 60, {"j": 24})[1][:3], "==>")

    # The ratchet file refuses a raise with no reason, a non-raise, and junk.
    _, p = parse_ratchet("cpu-matrix 24\n")
    expect("no reason refused", len(p), 1)
    _, p = parse_ratchet("cpu-matrix 20 because\n")
    expect("not a raise refused", len(p), 1)
    b, p = parse_ratchet("# c\n\ncpu-matrix 24 split is owed\n")
    expect("good ratchet", (b, p), ({"cpu-matrix": 24}, []))

    # lint(): the good fixture passes, and each wrong form is named.
    expect("good fixture", lint_workflow("f", GOOD, {}), [])
    bad = GOOD.replace('      - name: clock\n        run: echo "JOB_STARTED_AT=$(date +%s)" >> "$GITHUB_ENV"\n', "")
    expect("missing clock", len(lint_workflow("f", bad, {})), 1)
    bad = GOOD.replace("        if: always()\n", "")
    expect("missing always()", len(lint_workflow("f", bad, {})), 1)
    bad = GOOD.replace("timeout-minutes: 25", "timeout-minutes: 45")
    expect("timeout too long", len(lint_workflow("f", bad, {})), 1)
    expect("ratchet lifts the timeout cap", lint_workflow("f", bad, {"quick": 40}), [])
    bad = GOOD.replace("    timeout-minutes: 25\n", "")
    expect("timeout missing", len(lint_workflow("f", bad, {})), 1)
    expect("gate with no checkout is exempt from the steps", [p for p in lint_workflow("f", GOOD, {}) if "gate" in p], [])

    print("job-budget selftest: %s" % ("ok" if ok else "FAILED"))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ["--selftest"]:
        return selftest()
    if len(argv) == 4 and argv[1] == "check":
        try:
            with open(RATCHET) as f:
                ratchet, problems = parse_ratchet(f.read())
        except FileNotFoundError:
            ratchet, problems = {}, []
        if problems:
            print("\n".join("::error::" + p for p in problems))
            return 1
        if not argv[3].isdigit():
            print("::error::JOB_STARTED_AT is %r; the first step of the job did not record it" % argv[3])
            return 1
        code, msg = check(argv[2], int(argv[3]), time.time(), ratchet)
        print(msg)
        return code
    if argv[1:] == ["lint"]:
        problems = lint()
        for p in problems:
            print("job-budget: " + p, file=sys.stderr)
        return 1 if problems else 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
