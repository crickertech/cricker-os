#!/usr/bin/env python3
"""A workflow says where somebody watched it fail, or says that nobody has.

    helpers/observed_failing.py                 # script/lint: every workflow carries a record
    helpers/observed_failing.py --list          # each workflow and what its record says
    helpers/observed_failing.py --verify-runs   # by hand, needs `gh`: each cited run exists and
                                                # belongs to the workflow that cites it
    helpers/observed_failing.py --selftest

Milestone 640 (a gate is not evidence until somebody has watched it fail), provisional number. The
helper's name, the `Observed failing:` spelling and the cutoff below are provisional too. See
notes/observed-failing.md for the rule, the backfill, and EXAMPLES.

# WHY

A new gate ships against a tree where its defect is absent, so its first result is green, and green
is also what it reports if it cannot fire at all. `falsifications.yml` replayed nothing for three
weeks and reported success; `coe-architect-label.yml` applied no label and reported pass. Neither
had been run once before it was trusted (notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md).
DECISIONS §134 already makes a Kani harness carry the patch that turns it red. This is the same
demand made of a workflow, in a weaker form on purpose: a cited run, not a replayable patch.

# THE RULE

Every file in `.github/workflows/` carries at least one full-line, column-zero comment of one of
these shapes:

    # Observed failing: 2026-10-04, run 37226530740: what went red, or what fired, and why that is
    #   the thing this workflow exists to catch.
    # Observed failing: 2026-10-04, by script: what was broken on purpose, what was run against it,
    #   and why a real run could not be staged.
    # Observed failing: never. Why not, and what would stage it.

The first is the standard. "Failing" means the arm that carries the workflow's claim went off: a
red job for a gate, and for a workflow green by design (a labeller, a sweep that reports survivors
in its summary) the label applied or the finding reported.

**`never` is accepted only for a workflow this tree already had when the rule landed**, and that is
derived from git rather than written in the record: the file's earliest commit, following renames,
must be on or before `CUTOFF`. A new workflow therefore cannot ship with `never`; it ships with a
run. That is the rung-two half. A clone too shallow to answer is refused rather than guessed at.

A file with both an observation and a `never` contradicts itself and fails.

# BUGS

- **A cited run is not checked by the lint.** `script/lint` has no network, so a typo'd or invented
  run id passes it. `--verify-runs` checks each one against the API (it exists, and its workflow
  path is the file citing it) and was run over the whole backfill when this landed; nothing runs it
  on a schedule.
- **One record per file, not per job.** `ci.yml` has more than twenty jobs and its record is about
  one of them. A job added to an existing workflow, or a failure arm added to an existing step,
  inherits the file's record and carries no claim of its own. The milestone asks for the second;
  nothing here can tell a new arm from an edited line.
- **The cutoff is a date, not a list.** A workflow added on a branch cut before `CUTOFF` and merged
  after it would still be allowed `never`. Its window closes as those branches land.
- **Whether the cited run failed for the right reason is a reading, not a check.** `--verify-runs`
  shows the conclusion; a red run that died on infrastructure (`ci-failing.yml`'s first run, on a
  `jq` argument limit) looks the same as one that caught its defect. The record's prose is where
  that difference is stated.
"""

import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone

WORKFLOW_DIR = ".github/workflows"
# The day milestone 640 was asked for (calef, 2026-10-04 UTC). Inclusive: a workflow first
# committed on this day may still say `never`. Provisional, like the rest of the milestone.
CUTOFF = "2026-10-04"
CUTOFF_END = datetime(2026, 10, 5, tzinfo=timezone.utc).timestamp()

RECORD = re.compile(r"^#\s*Observed failing:\s*(.*)$")
OBSERVED_RUN = re.compile(r"^(\d{4}-\d{2}-\d{2}), run (\d+)\b(.*)$")
OBSERVED_SCRIPT = re.compile(r"^(\d{4}-\d{2}-\d{2}), by script:\s*(\S.*)$")
NEVER = re.compile(r"^never\.\s*(\S.*)?$")


def parse(text):
    """Return (observations, nevers, errors) for one workflow's text.

    An observation is (date, run_id or None); a never is its reason.
    """
    observed, never, errors = [], [], []
    for n, line in enumerate(text.splitlines(), 1):
        m = RECORD.match(line)
        if not m:
            continue
        body = m.group(1).strip()
        r = OBSERVED_RUN.match(body)
        s = OBSERVED_SCRIPT.match(body)
        v = NEVER.match(body)
        if r or s:
            date = (r or s).group(1)
            try:
                datetime.strptime(date, "%Y-%m-%d")
            except ValueError:
                errors.append(f"line {n}: '{date}' is not a date (YYYY-MM-DD, UTC)")
                continue
            if r and not r.group(3).strip(" :"):
                errors.append(f"line {n}: a run with no account of what it showed")
                continue
            observed.append((date, r.group(2) if r else None))
        elif v:
            if not v.group(1):
                errors.append(f"line {n}: `never.` with no reason after it")
                continue
            never.append(v.group(1))
        else:
            errors.append(
                f"line {n}: unreadable record; expected `<YYYY-MM-DD>, run <id>: ...`, "
                "`<YYYY-MM-DD>, by script: ...` or `never. <reason>`"
            )
    return observed, never, errors


def judge(name, text, first_commit_epoch):
    """Return a list of failure messages for one workflow. Empty means it passes.

    `first_commit_epoch` is the file's earliest commit time, or None if it has none.
    """
    observed, never, errors = parse(text)
    out = [f"{name}: {e}" for e in errors]
    if not (observed or never or errors):
        out.append(
            f"{name}: no `# Observed failing:` record. Cite a run where this workflow went red (or "
            "fired) for the reason it exists; see notes/observed-failing.md."
        )
    if observed and never:
        out.append(f"{name}: says both that it was watched failing and that it never was")
    if never and not observed:
        if first_commit_epoch is None or first_commit_epoch >= CUTOFF_END:
            out.append(
                f"{name}: `never` is only for a workflow the tree had by {CUTOFF}; this one is "
                "newer, so it ships with a run that watched it fail"
            )
    return out


def first_commit_epoch(path):
    out = subprocess.run(
        ["git", "log", "--follow", "--format=%ct", "--", path],
        capture_output=True, text=True, check=True,
    ).stdout.split()
    return int(out[-1]) if out else None


def workflows():
    names = sorted(f for f in os.listdir(WORKFLOW_DIR) if f.endswith((".yml", ".yaml")))
    return [os.path.join(WORKFLOW_DIR, f) for f in names]


def lint():
    shallow = subprocess.run(
        ["git", "rev-parse", "--is-shallow-repository"], capture_output=True, text=True, check=True
    ).stdout.strip()
    if shallow == "true":
        print(
            "observed-failing: this clone is shallow, so a workflow's age cannot be read and `never` "
            "cannot be judged. Fetch full history (CI: `fetch-depth: 0`).",
            file=sys.stderr,
        )
        return 1
    paths = workflows()
    if not paths:
        # Zero is loud: a rename of the directory must not turn this into a pass over nothing.
        print(f"observed-failing: no workflows under {WORKFLOW_DIR}; refusing to pass over nothing",
              file=sys.stderr)
        return 1
    failures, seen, nevers = [], 0, 0
    for p in paths:
        with open(p) as f:
            text = f.read()
        observed, never, _ = parse(text)
        seen += bool(observed)
        nevers += bool(never and not observed)
        failures += judge(p, text, first_commit_epoch(p))
    for msg in failures:
        print(msg, file=sys.stderr)
    print(f"observed-failing: {len(paths)} workflows, {seen} watched failing, "
          f"{nevers} never (each older than {CUTOFF})")
    return 1 if failures else 0


def list_records():
    for p in workflows():
        with open(p) as f:
            observed, never, errors = parse(f.read())
        if observed:
            what = "; ".join(f"{d} run {r}" if r else f"{d} by script" for d, r in observed)
        elif never:
            what = "never"
        else:
            what = "NO RECORD" if not errors else "UNREADABLE"
        print(f"{os.path.basename(p):32} {what}")
    return 0


def verify_runs():
    repo = os.environ.get("GITHUB_REPOSITORY", "nifeos/nife")
    bad = 0
    for p in workflows():
        with open(p) as f:
            observed, _, _ = parse(f.read())
        for date, run in observed:
            if run is None:
                continue
            r = subprocess.run(["gh", "api", f"repos/{repo}/actions/runs/{run}"],
                               capture_output=True, text=True)
            if r.returncode != 0:
                print(f"{p}: run {run} not found", file=sys.stderr)
                bad += 1
                continue
            j = json.loads(r.stdout)
            ok = j.get("path") == p and j.get("created_at", "").startswith(date)
            bad += not ok
            print(f"{'ok ' if ok else 'BAD'} {os.path.basename(p):32} run {run} "
                  f"{j.get('created_at', '')[:10]} {j.get('event')} {j.get('conclusion')} "
                  f"({j.get('path')})")
    return 1 if bad else 0


def selftest():
    old, new = 1_700_000_000, CUTOFF_END + 1
    run = "# Observed failing: 2026-10-04, run 123: went red on a stale record.\n"
    cases = [
        # (name, text, first commit, should pass)
        ("cited run", "name: x\n" + run, new, True),
        ("by script", "# Observed failing: 2026-10-04, by script: broke X, ran Y.\n", new, True),
        ("old never", "# Observed failing: never. Nobody has; a stub gh would stage it.\n", old, True),
        ("no record", "name: x\non: push\n", old, False),
        ("new never", "# Observed failing: never. Not yet.\n", new, False),
        ("uncommitted never", "# Observed failing: never. Not yet.\n", None, False),
        ("never, no reason", "# Observed failing: never.\n", old, False),
        ("run, no account", "# Observed failing: 2026-10-04, run 123\n", new, False),
        ("bad date", "# Observed failing: 2026-13-40, run 1: x\n", new, False),
        ("garbled", "# Observed failing: yes, it did\n", new, False),
        ("both", run + "# Observed failing: never. x\n", old, False),
        ("indented is not a record", "jobs:\n  # Observed failing: never. x\n", old, False),
    ]
    wrong = [name for name, text, epoch, ok in cases if (not judge(name, text, epoch)) != ok]
    if wrong:
        print(f"observed-failing selftest: wrong verdict on {wrong}", file=sys.stderr)
        return 1
    print(f"observed-failing selftest: {len(cases)} fixtures, each judged as expected")
    return 0


def main(argv):
    os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
    mode = argv[1] if len(argv) > 1 else ""
    if mode == "--selftest":
        return selftest()
    if mode == "--list":
        return list_records()
    if mode == "--verify-runs":
        return verify_runs()
    if mode == "":
        return lint()
    print(f"observed-failing: unknown argument {mode}; see the docstring", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
