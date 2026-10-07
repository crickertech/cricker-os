#!/usr/bin/env python3
"""Where CI's merge-group time goes, day by day, and a tracking issue for a job near its budget.

    python3 helpers/ci_job_times.py update [--since YYYY-MM-DD]   # the daily record
    python3 helpers/ci_job_times.py sync [--dry-run]               # open, update or close issues
    python3 helpers/ci_job_times.py --selftest

Milestone 808 (every gate accounts for its time), under §254 (a gate prints what each item cost, and
a job near its budget warns rather than fails). Both run in `.github/workflows/metrics.yml` once a
day, as `nife-smelter[bot]`.

Name: provisional, minted 2026-10-07 (UTC) by milestone 808's lane, as are the CSV's name, the
`near-budget` label and the chart `ci-jobs.svg`. A python helper under `helpers/`, outside
`script/names`' scope, so this paragraph is its provenance. Refused: `job_budget_watch`, which
names one of its two jobs and is one letter from `helpers/job-budget.py`.

# WHY

Milestone 721 (each merge-group CI job has a 20-minute budget) gave every `ci.yml` job one number
and a gate on it. On 2026-10-06 the gate fired on `cpu-matrix` at 20.1 to 21.0 minutes, and the
answer took a lane reading 31 runs out of the Actions API by hand: no step had regressed, the suite
had grown about 0.1 minutes per model per day for two weeks, and every run in that window showed
it. Nothing read CI time over weeks. This does, in two halves.

# `update`: the daily record (§254, Fork 4)

For each UTC day, every completed `merge_group` run of `ci.yml` and `verify.yml` (`RECORDED`), read
through the jobs API, which already holds each job's and each step's start and end, so no job
had to change to provide them. One row per day, workflow, job and step, appended to `CSV`:

    date,workflow,job,variant,kind,name,runs,median_seconds,max_seconds

`job` is the workflow's job key, the name `.github/ci-job-budgets` uses. `variant` is what a matrix
filled into the display name (`rv64 sifive-u54 rva22s64`), empty otherwise. `kind` is `job` (the
whole job; `name` is empty) or `step` (`name` is the step's). Milestone 807 (the kernel suite reports
what each test cost)'s per-test record is an artifact per run; folding it in here is a later kind.
`script/metrics` reads the `job` rows into the weekly chart `ci-jobs.svg`.

# `sync`: the tracking issue (§254, Fork 2)

For each job, the newest `HISTORY` merge-group runs that ran it. When the newest is at or past
`WARN_FRACTION` of the job's budget and no issue is open for the job, it opens one, labeled `LABEL`.
While one is open it rewrites the issue's body on every pass with the recent runs, so the same issue
is updated rather than a second opened. It closes the issue when the newest `CLOSE_AFTER` runs are
all under the line. While an issue is open, it appends one row a week to week-notes.csv, which
`script/metrics` draws as a bullet under `ci-jobs.svg` (milestone 623 (bullet under the chart
explains a cliff)).

**Damping: one run opens, three runs close.** §254's Open section left to this lane whether the line
reads one run or a median. Opening on one run keeps the warning as early as the ruling wants; it is
a warning, it fails nothing, and the wall time of a job sitting at 84% one day and 86% the next is a
job that deserves a look. Closing on one run would flap: the same job on the same commit varies by
about 30% across hosted runners (helpers/verify_times.py measured 455 to 801 seconds for one crate on
2026-10-06). Three in a row under the line is the hysteresis.

The issue reaches a session through `needs-maintainer` (the `budget` cause in
helpers/needs-maintainer.jq), as calef ruled on 2026-10-06 (UTC).

# BUGS

- `sync` runs once a day, so an issue opens up to a day after the run that crossed. The drift it
  exists for is measured in weeks.
- A matrix job's runs are judged by their slowest variant, since that is the pole that meets the
  budget; the issue names it.
- Runner speed is not controlled. Every number is wall time on a shared hosted machine.
- Runs are read from the API on the day, and GitHub keeps run history for 90 days, so a missed day
  can be backfilled with `--since` only within that window.
"""

import csv
import datetime
import importlib.util
import io
import json
import os
import re
import statistics
import subprocess
import sys
import time

REPO = "nifeos/nife"
CSV = "notes/project-metrics/ci-job-times.csv"
COLUMNS = ["date", "workflow", "job", "variant", "kind", "name", "runs", "median_seconds", "max_seconds"]
WEEK_NOTES = "notes/project-metrics/week-notes.csv"
CHART = "ci-jobs.svg"
# The issue's label. helpers/needs-maintainer.jq's `nm_budget_label` and helpers/merge-drain.sh's
# `NM_BUDGET_LABEL` hold the same string; helpers/needs-maintainer-selftest.sh checks all three.
LABEL = "near-budget"
HISTORY = 10
CLOSE_AFTER = 3
DEFAULT_WINDOW_DAYS = 2
MARKER = "<!-- ci-job-times:job=%s -->"
# The workflows the daily record reads. Wider than `helpers/job-budget.py`'s WORKFLOWS on purpose:
# `verify.yml` is outside milestone 721's budget until its medians exist (that milestone's first
# BUGS entry), and this record is where they come from. `sync` reads only the budgeted ones, since a
# job with no budget has no line to cross.
RECORDED = [".github/workflows/ci.yml", ".github/workflows/verify.yml"]


def _load_budget_module():
    """helpers/job-budget.py, whose file name has a hyphen and so cannot be imported by name. The
    budget, the warning line and the job parser come from there, so the two cannot disagree."""
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("job_budget", os.path.join(here, "job-budget.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


jb = _load_budget_module()
WARN_FRACTION = jb.WARN_FRACTION


# ---------------------------------------------------------------------------------------------
# Display names to job keys.

def display_patterns(workflow_text):
    """[(job key, compiled regex over the display name)] for a workflow file.

    A job's display name is its `name:`, or its key when it has none. A matrix job's name carries
    `${{ matrix.x }}`, which GitHub fills per variant; that part becomes a capture group, and what it
    matched is the row's `variant`."""
    out = []
    for key, body in jb.jobs_of(workflow_text).items():
        name = key
        for line in body:
            m = re.match(r"^    name:\s*(.+?)\s*$", line)
            if m:
                name = m.group(1).strip("'\"")
                break
        parts = re.split(r"\$\{\{.*?\}\}", name)
        pattern = "(.*)".join(re.escape(p) for p in parts)
        out.append((key, re.compile("^%s$" % pattern)))
    return out


def job_key(display, patterns):
    """(key, variant) for a display name, or (None, "") for a job this file does not have."""
    for key, rx in patterns:
        m = rx.match(display)
        if m:
            return key, " ".join(g for g in m.groups() if g)
    return None, ""


# ---------------------------------------------------------------------------------------------
# Durations, pure over the API's objects.

def _ts(s):
    return datetime.datetime.fromisoformat(s.replace("Z", "+00:00"))


def seconds(obj):
    """Wall seconds of a job or a step, or None when it did not run (skipped, cancelled, or never
    started)."""
    if obj.get("conclusion") not in ("success", "failure"):
        return None
    if not obj.get("started_at") or not obj.get("completed_at"):
        return None
    return max(0.0, (_ts(obj["completed_at"]) - _ts(obj["started_at"])).total_seconds())


def run_durations(jobs, patterns):
    """{(key, variant): (job seconds, {step name: seconds})} for one run's jobs."""
    out = {}
    for job in jobs:
        s = seconds(job)
        if s is None:
            continue
        key, variant = job_key(job["name"], patterns)
        if key is None:
            continue
        steps = {}
        for st in job.get("steps") or []:
            t = seconds(st)
            if t is not None:
                steps[st["name"]] = steps.get(st["name"], 0.0) + t
        out[(key, variant)] = (s, steps)
    return out


def day_rows(date, workflow, per_run):
    """The CSV rows for one day: medians and maxima over the day's runs, per job and per step."""
    jobs, steps = {}, {}
    for durations in per_run:
        for (key, variant), (s, st) in durations.items():
            jobs.setdefault((key, variant), []).append(s)
            for name, t in st.items():
                steps.setdefault((key, variant, name), []).append(t)
    rows = []
    for (key, variant), v in sorted(jobs.items()):
        rows.append(_row(date, workflow, key, variant, "job", "", v))
    for (key, variant, name), v in sorted(steps.items()):
        rows.append(_row(date, workflow, key, variant, "step", name, v))
    return rows


def _row(date, workflow, key, variant, kind, name, values):
    return {"date": date, "workflow": workflow, "job": key, "variant": variant, "kind": kind,
            "name": name, "runs": len(values), "median_seconds": "%.0f" % statistics.median(values),
            "max_seconds": "%.0f" % max(values)}


def to_csv(rows):
    buf = io.StringIO()
    w = csv.DictWriter(buf, fieldnames=COLUMNS, lineterminator="\n")
    w.writeheader()
    for r in rows:
        w.writerow({c: r.get(c, "") for c in COLUMNS})
    return buf.getvalue()


def from_csv(text):
    return list(csv.DictReader(io.StringIO(text)))


def merge_days(existing, fresh):
    """Every row of a day `fresh` covers is replaced by `fresh`'s; other days are kept."""
    days = {(r["date"], r["workflow"]) for r in fresh}
    kept = [r for r in existing if (r["date"], r["workflow"]) not in days]
    return sorted(kept + list(fresh), key=lambda r: (r["date"], r["workflow"], r["job"], r["variant"],
                                                     r["kind"] != "job", r["name"]))


# ---------------------------------------------------------------------------------------------
# The tracking issue's decision, pure over runs and issues.

def history(per_run):
    """{key: [(run id, created, seconds, variant), ...]} newest first, one entry per run: a matrix
    job's slowest variant, since that is the pole the budget binds."""
    out = {}
    for run_id, created, durations in per_run:
        best = {}
        for (key, variant), (s, _steps) in durations.items():
            if key not in best or s > best[key][0]:
                best[key] = (s, variant)
        for key, (s, variant) in best.items():
            out.setdefault(key, []).append((run_id, created, s, variant))
    for key in out:
        out[key].sort(key=lambda e: e[1], reverse=True)
        out[key] = out[key][:HISTORY]
    return out


def decide(hist, open_issues, budgets):
    """The actions for one pass: [(action, job, detail)].

    `open_issues` is {job: issue number}. Actions are `open` (no issue, newest run at or past the
    line), `update` (an issue is open and stays open), and `close` (an issue is open and the newest
    CLOSE_AFTER runs are all under the line)."""
    actions = []
    for key in sorted(set(hist) | set(open_issues)):
        runs = hist.get(key, [])
        budget = jb.budget_for(key, budgets) * 60.0
        line = budget * WARN_FRACTION
        over = [s >= line for (_r, _c, s, _v) in runs]
        issue = open_issues.get(key)
        if issue is None:
            if over and over[0]:
                actions.append(("open", key, runs[0][2] / budget))
        elif len(over) >= CLOSE_AFTER and not any(over[:CLOSE_AFTER]):
            actions.append(("close", key, issue))
        else:
            actions.append(("update", key, issue))
    return actions


def issue_title(key):
    return "CI job `%s` is near its budget" % key


def issue_body(key, runs, budgets, slowest_steps):
    budget = jb.budget_for(key, budgets)
    lines = [
        MARKER % key,
        "Opened by `helpers/ci_job_times.py sync` (milestone 808 (every gate accounts for its time), "
        "§254 (a gate prints what each item cost)). The job `%s` ran past %d%% of its %d-minute budget "
        "in `.github/ci-job-budgets` on a merge-group run. Nothing has failed: the hard limit is the "
        "budget itself. This issue is rewritten on each daily pass, and closes itself when %d runs in "
        "a row are back under the line."
        % (key, round(WARN_FRACTION * 100), budget, CLOSE_AFTER),
        "",
        "The ways out: make something faster, split the job, or raise the budget with a reason line in "
        "`.github/ci-job-budgets`, which moves this line with it.",
        "",
        "| run | started (UTC) | minutes | share of budget | variant |",
        "|---|---|---:|---:|---|",
    ]
    for run_id, created, s, variant in runs:
        lines.append("| [%s](https://github.com/%s/actions/runs/%s) | %s | %.1f | %.0f%% | %s |"
                     % (run_id, REPO, run_id, created[:16].replace("T", " "), s / 60.0,
                        100.0 * s / (budget * 60.0), variant))
    if slowest_steps:
        lines += ["", "The newest run's slowest steps:", "", "| step | minutes |", "|---|---:|"]
        lines += ["| %s | %.1f |" % (n, t / 60.0) for n, t in slowest_steps]
    lines += ["", "A job that runs the kernel suite also uploads its per-test record as the run's "
              "`time-record-*` artifact (milestone 807 (the kernel suite reports what each test cost))."]
    return "\n".join(lines) + "\n"


def week_of(date):
    y, w, _ = date.isocalendar()
    return "%dW%02d" % (y, w)


def week_note_row(week, key, share, issue):
    """The week-notes.csv line for an open budget issue (milestone 623's bullet)."""
    note = "CI job %s ran at %.0f%% of its budget on its newest merge-group run; tracking issue #%s" % (
        key, share * 100, issue)
    buf = io.StringIO()
    csv.writer(buf, lineterminator="\n").writerow([week, CHART, note])
    return buf.getvalue()


def has_week_note(text, week, issue):
    for row in csv.reader(io.StringIO(text)):
        if len(row) == 3 and row[0] == week and row[1] == CHART and row[2].endswith("#%s" % issue):
            return True
    return False


# ---------------------------------------------------------------------------------------------
# GitHub.

def gh_json(path, tries=3):
    for attempt in range(tries):
        done = subprocess.run(["gh", "api", path], capture_output=True, text=True)
        if not done.returncode:
            return json.loads(done.stdout)
        if attempt + 1 < tries:
            time.sleep(5 * (attempt + 1))
    raise RuntimeError("gh api %s: %s" % (path, done.stderr.strip()))


def gh(*args):
    done = subprocess.run(["gh"] + list(args), capture_output=True, text=True)
    if done.returncode:
        raise RuntimeError("gh %s: %s" % (" ".join(args[:3]), done.stderr.strip()))
    return done.stdout


def merge_group_runs(workflow, created=None, limit=None):
    """Completed merge-group runs of `workflow`, newest first, not cancelled."""
    out, page = [], 1
    q = "event=merge_group&status=completed&per_page=100"
    if created:
        q += "&created=%s" % created
    while True:
        data = gh_json("repos/%s/actions/workflows/%s/runs?%s&page=%d"
                       % (REPO, os.path.basename(workflow), q, page))
        out += [r for r in data["workflow_runs"] if r.get("conclusion") != "cancelled"]
        if len(data["workflow_runs"]) < 100 or (limit and len(out) >= limit):
            return out[:limit] if limit else out
        page += 1


def jobs_of(run_id):
    jobs, page = [], 1
    while True:
        data = gh_json("repos/%s/actions/runs/%d/jobs?per_page=100&page=%d" % (REPO, run_id, page))
        jobs += data["jobs"]
        if len(data["jobs"]) < 100:
            return jobs
        page += 1


def ratchet():
    try:
        with open(jb.RATCHET) as f:
            return jb.parse_ratchet(f.read())[0]
    except FileNotFoundError:
        return {}


def arg_value(argv, flag, default):
    return argv[argv.index(flag) + 1] if flag in argv and argv.index(flag) + 1 < len(argv) else default


def update(argv):
    today = datetime.datetime.now(datetime.timezone.utc).date()
    since = datetime.date.fromisoformat(
        arg_value(argv, "--since", (today - datetime.timedelta(days=DEFAULT_WINDOW_DAYS)).isoformat()))
    for workflow in RECORDED:
        patterns = display_patterns(open(workflow).read())
        name = os.path.basename(workflow)
        day = since
        while day <= today:
            per_run = [run_durations(jobs_of(r["id"]), patterns)
                       for r in merge_group_runs(workflow, created=day.isoformat())]
            fresh = day_rows(day.isoformat(), name, per_run)
            existing = from_csv(open(CSV).read()) if os.path.exists(CSV) else []
            # Written a day at a time, so a failure part way keeps every day before it.
            with open(CSV, "w") as f:
                f.write(to_csv(merge_days(existing, fresh)))
            print("ci_job_times: %s %s: %d run(s)" % (name, day, len(per_run)), file=sys.stderr)
            day += datetime.timedelta(days=1)
    return 0


def open_budget_issues():
    """{job: issue number} for the open issues carrying LABEL and this script's marker."""
    data = gh_json("repos/%s/issues?labels=%s&state=open&per_page=100" % (REPO, LABEL))
    out = {}
    for issue in data:
        m = re.search(r"<!-- ci-job-times:job=([A-Za-z0-9_-]+) -->", issue.get("body") or "")
        if m and "pull_request" not in issue:
            out[m.group(1)] = issue["number"]
    return out


def sync(argv):
    dry = "--dry-run" in argv
    budgets = ratchet()
    per_run, steps_of_run = [], {}
    for workflow in jb.WORKFLOWS:
        patterns = display_patterns(open(workflow).read())
        for r in merge_group_runs(workflow, limit=HISTORY):
            d = run_durations(jobs_of(r["id"]), patterns)
            per_run.append((r["id"], r["created_at"], d))
            steps_of_run[r["id"]] = d
    hist = history(per_run)
    shares = sorted(((runs[0][2] / (jb.budget_for(k, budgets) * 60.0), k) for k, runs in hist.items() if runs),
                    reverse=True)
    for share, key in shares[:5]:
        print("ci_job_times: %s's newest merge-group run took %.0f%% of its budget" % (key, share * 100))
    issues = open_budget_issues()
    week = week_of(datetime.datetime.now(datetime.timezone.utc).date())
    notes = open(WEEK_NOTES).read() if os.path.exists(WEEK_NOTES) else ""
    for action, key, detail in decide(hist, issues, budgets):
        runs = hist.get(key, [])
        slowest = []
        if runs:
            newest = runs[0]
            st = steps_of_run.get(newest[0], {}).get((key, newest[3]), (0, {}))[1]
            slowest = sorted(st.items(), key=lambda kv: -kv[1])[:5]
        body = issue_body(key, runs, budgets, slowest)
        share = runs[0][2] / (jb.budget_for(key, budgets) * 60.0) if runs else 0.0
        if dry:
            print("ci_job_times: (dry run) would %s for %s (%s)" % (action, key, detail))
            continue
        if action == "open":
            url = gh("issue", "create", "--repo", REPO, "--title", issue_title(key), "--label", LABEL,
                     "--body", body).strip()
            number = url.rsplit("/", 1)[-1]
            print("ci_job_times: OPENED #%s for %s at %.0f%%" % (number, key, share * 100))
        elif action == "update":
            number = str(detail)
            gh("issue", "edit", number, "--repo", REPO, "--body", body)
            print("ci_job_times: UPDATED #%s for %s at %.0f%%" % (number, key, share * 100))
        else:
            gh("issue", "close", str(detail), "--repo", REPO, "--comment",
               "Closed by `helpers/ci_job_times.py sync`: the newest %d merge-group runs of `%s` were all "
               "under %d%% of its budget." % (CLOSE_AFTER, key, round(WARN_FRACTION * 100)))
            print("ci_job_times: CLOSED #%s for %s" % (detail, key))
            continue
        if not has_week_note(notes, week, number):
            row = week_note_row(week, key, share, number)
            with open(WEEK_NOTES, "a") as f:
                f.write(row)
            notes += row
    return 0


# ---------------------------------------------------------------------------------------------

FIXTURE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ci-job-times-fixtures",
                       "run-37648588582.json")

WORKFLOW_FIXTURE = """\
name: CI
jobs:
  test:
    name: test (host crates and the kernel suite under QEMU)
    timeout-minutes: 25
  cpu-matrix-shard:
    name: cpu matrix shard (${{ matrix.models }})
    timeout-minutes: 25
  rustfmt:
    timeout-minutes: 10
"""


def selftest():
    ok = True

    def expect(label, got, want):
        nonlocal ok
        if got != want:
            ok = False
            print("FAIL %s:\n  got  %r\n  want %r" % (label, got, want), file=sys.stderr)

    pats = display_patterns(WORKFLOW_FIXTURE)
    expect("plain name", job_key("test (host crates and the kernel suite under QEMU)", pats), ("test", ""))
    expect("matrix name", job_key("cpu matrix shard (rv64 sifive-u54 rva22s64)", pats),
           ("cpu-matrix-shard", "rv64 sifive-u54 rva22s64"))
    expect("key as name", job_key("rustfmt", pats), ("rustfmt", ""))
    expect("unknown", job_key("draft gate", pats), (None, ""))

    # The recorded run: a real merge-group run's jobs, trimmed to the fields read.
    with open(FIXTURE) as f:
        recorded = json.load(f)
    d = run_durations(recorded["jobs"], pats)
    expect("recorded keys", sorted(d), [("cpu-matrix-shard", "rv64 sifive-u54 rva22s64"),
                                        ("cpu-matrix-shard", "rva23s64 thead-c906"), ("rustfmt", ""),
                                        ("test", "")])
    expect("recorded test seconds", d[("test", "")][0], 777.0)
    expect("the suite's own step is timed", round(d[("test", "")][1]["Run script/ci-build test"]) > 0, True)
    expect("the draft gate is no budgeted job", any(k[0] == "draft gate" for k in d), False)
    rows = day_rows("2026-10-07", "ci.yml", [d, d])
    expect("one job row per variant, then steps",
           [(r["job"], r["variant"], r["kind"]) for r in rows][:4],
           [("cpu-matrix-shard", "rv64 sifive-u54 rva22s64", "job"),
            ("cpu-matrix-shard", "rva23s64 thead-c906", "job"), ("rustfmt", "", "job"), ("test", "", "job")])
    expect("csv round trip", from_csv(to_csv(rows)), [{k: str(v) for k, v in r.items()} for r in rows])
    merged = merge_days(from_csv(to_csv(rows)), [dict(rows[0], date="2026-10-08")])
    expect("a new day is added, an old one kept", sorted({r["date"] for r in merged}),
           ["2026-10-07", "2026-10-08"])

    # The three transitions (exit criterion 5), driven by the recorded run with its test job's
    # duration rewritten. The budget is 20 minutes, the line 17.
    def runs_at(minutes_newest_first):
        out = []
        for i, m in enumerate(minutes_newest_first):
            job = dict(next(j for j in recorded["jobs"] if j["name"].startswith("test ")))
            t0 = _ts(job["started_at"])
            job["completed_at"] = (t0 + datetime.timedelta(minutes=m)).strftime("%Y-%m-%dT%H:%M:%SZ")
            out.append((1000 - i, "2026-10-%02dT12:00:00Z" % (20 - i), run_durations([job], pats)))
        return history(out)

    expect("quiet below the line opens nothing", decide(runs_at([16.9, 16, 15]), {}, {}), [])
    expect("crossing opens", [a[:2] for a in decide(runs_at([17.2, 16, 15]), {}, {})], [("open", "test")])
    expect("a later crossing updates the same issue",
           decide(runs_at([18, 17.5, 16]), {"test": 77}, {}), [("update", "test", 77)])
    expect("one run under does not close", decide(runs_at([16, 18, 17.5]), {"test": 77}, {}),
           [("update", "test", 77)])
    expect("three runs under close", decide(runs_at([16, 15, 16.5, 18]), {"test": 77}, {}),
           [("close", "test", 77)])
    expect("a raised budget moves the line", decide(runs_at([18, 16, 15]), {}, {"test": 24}), [])

    body = issue_body("test", runs_at([18, 16])["test"], {}, [("Run script/ci-build test", 600.0)])
    expect("the body carries the marker the next pass finds", body.startswith(MARKER % "test"), True)
    expect("the body names the share", "| 90% |" in body, True)

    row = week_note_row("2026W41", "test", 0.9, 77)
    expect("week note", row, '2026W41,ci-jobs.svg,CI job test ran at 90% of its budget on its newest '
                             'merge-group run; tracking issue #77\n')
    expect("week note found", has_week_note("week,chart,note\n" + row, "2026W41", "77"), True)
    expect("week note per week", has_week_note("week,chart,note\n" + row, "2026W42", "77"), False)

    # The label agrees with the two other places that spell it.
    here = os.path.dirname(os.path.abspath(__file__))
    for path, needle in (("needs-maintainer.jq", 'def nm_budget_label: "%s";' % LABEL),
                         ("merge-drain.sh", 'NM_BUDGET_LABEL="%s"' % LABEL)):
        with open(os.path.join(here, path)) as f:
            expect("%s spells the label %s" % (path, LABEL), needle in f.read(), True)

    print("ci_job_times selftest: %s" % ("ok" if ok else "FAILED"))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ["--selftest"]:
        return selftest()
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd == "update":
        return update(argv[2:])
    if cmd == "sync":
        return sync(argv[2:])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
