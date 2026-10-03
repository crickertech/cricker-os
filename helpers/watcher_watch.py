#!/usr/bin/env python3
"""Is each merge watcher running, and does somebody who can act know when it is not?

    helpers/watcher_watch.py check [--quiet]   # one row per watcher; exit 1 if any is stopped
    helpers/watcher_watch.py sync              # check, then open or close one issue per watcher
    helpers/watcher_watch.py --selftest

Milestone 723 (a stopped merge watcher is reported within three of its own intervals), provisional
number. Name provisional.

# WHY

The merge drain sat `disabled_manually` from about 21:40 UTC on 2026-09-30 to the morning of
2026-10-03. `script/cadence-check` called a workflow dead only when its last success was 15 days
old, so on 10-03 it still called the drain live, 62 hours after its last run. The drain is what
lands green work, so a stopped drain is a stopped merge rate (notes/coes/2026-10-03-the-merge-rate.md,
on PR #1513's branch until it lands).

# THE RULE

A merge watcher is one of `WATCHERS`. It is reported when either holds:

1. Its workflow state is anything but `active` (`disabled_manually`, `disabled_inactivity`, ...).
   This is immediate and exact, and it is the whole of the 2026-09-30 incident.
2. No run of any kind, schedule or otherwise, has started within `silence_minutes(name, cron)`:
   three of its cron intervals, but never less than `MEASURED_FLOOR_MIN` for that watcher.

# THE FLOOR, AND WHY THE RULE IS NOT SIMPLY THREE INTERVALS

calef ruled three intervals (15 minutes for a `*/5` cron) on 2026-10-03. Measured the same day over
the last runs the API still held, GitHub does not deliver that cron:

| watcher | runs | median gap | p90 gap | longest gap |
|---|---|---|---|---|
| merge-drain (99 gaps, mostly `workflow_run` after CI) | 100 | 2 min | 9 min | 140 min |
| trunk-health (50 gaps, all `schedule`) | 51 | 281 min | 404 min | 499 min |

`schedule:` is a floor (merge-drain.yml's BUGS says so). Three intervals would call trunk-health
silent on every run it has, and the drain on every idle stretch with no CI to trigger it, so
the report would open and close an issue all day and teach its reader to skip it. The floors sit
above the longest gap measured. They are the one number here that a reader should expect to change:
when GitHub delivers the cron, delete the floor and the rule is exactly calef's. calef ruled the
floors stay, 2026-10-03 UTC.

# WHO REPORTS

A stopped `trunk-health` cannot report itself, so the drain's pass runs `sync`, and trunk-health's
pass runs it too. Each run opens an issue for any stopped watcher that has none open, and closes the
issue of any watcher that is running again. A running watcher is healthy by construction, so a
watcher closes its own issue the moment it next runs, which is the recovery signal.

The issue is opened as `nife-smelter[bot]` when its App token may create issues, and as
`github-actions[bot]` otherwise (`WATCHER_FALLBACK_TOKEN`). The log says which.

# BUGS

- The silence floors are measured on nine days of history for trunk-health and nine hours for the drain, and GitHub
  throttles per repository load, so a floor can be wrong in either direction. A real stop longer than
  the floor is caught; one shorter is not, except by the state check.
- Both watchers can stop together, and then nothing reports. The state check is a report, not a
  heartbeat, and a third party watching both is outside this repository.
- `sync` writes an issue only on a transition (stopped with none open, running with one open). A
  watcher that stays stopped is one issue, not one per run, on purpose, and the same property means
  nobody is re-told.
- It reads each watcher's last run, not its last success. A watcher that runs and fails every time
  reads as running; `trunk-health` already fails its own run when main is red.
"""

import datetime
import json
import os
import re
import subprocess
import sys

REPO = "nifeos/nife"
WORKFLOWS = ".github/workflows"
WATCHERS = ["merge-drain", "trunk-health"]
SILENT_INTERVALS = 3

# Longest gap between runs measured on 2026-10-03 (see the table in the docstring), rounded up
# with room: the drain's 140 minutes becomes 180 and trunk-health's 499 becomes 720. A floor, never
# a ceiling, so a watcher with a shorter real interval still gets three of them.
MEASURED_FLOOR_MIN = {"merge-drain": 180, "trunk-health": 720}

TITLE_PREFIX = "merge watcher stopped: "


def cron_interval_minutes(cron):
    """The nominal minutes between two firings of a five-field cron, or None if unrecognised.

    Covers the shapes this tree writes: `*/N` minutes, a fixed minute every hour or every N hours, a
    fixed time daily, weekly or monthly. Anything fancier returns None, and the caller then falls
    back to the floor alone rather than guessing."""
    f = cron.split()
    if len(f) != 5:
        return None
    minute, hour, dom, month, dow = f
    step = re.fullmatch(r"\*/(\d+)", minute)
    if step and hour == dom == month == dow == "*":
        return int(step.group(1))
    if minute == "*" and hour == dom == month == dow == "*":
        return 1
    if minute.isdigit():
        hstep = re.fullmatch(r"\*/(\d+)", hour)
        if hour == "*" and dom == month == dow == "*":
            return 60
        if hstep and dom == month == dow == "*":
            return 60 * int(hstep.group(1))
        if hour.isdigit() and month == "*":
            if dom == "*" and dow == "*":
                return 24 * 60
            if dom == "*" and dow.isdigit():
                return 7 * 24 * 60
            if dom.isdigit() and dow == "*":
                return 30 * 24 * 60
    return None


def silence_minutes(name, cron):
    """How long without any run before a watcher is called silent."""
    interval = cron_interval_minutes(cron) if cron else None
    floor = MEASURED_FLOOR_MIN.get(name, 0)
    return max(floor, SILENT_INTERVALS * interval) if interval else floor


def parse_time(s):
    return datetime.datetime.fromisoformat(s.replace("Z", "+00:00"))


def verdict(name, state, cron, last_run, now):
    """`(ok, text)` for one watcher. `last_run` is an ISO timestamp or None; `now` a datetime."""
    if state != "active":
        return False, "DEAD: workflow state is %s, so it will not run until someone enables it" % state
    limit = silence_minutes(name, cron)
    if last_run is None:
        return False, "DEAD: no run on record (silence limit %d min)" % limit
    ago = (now - parse_time(last_run)).total_seconds() / 60.0
    if limit and ago > limit:
        return False, "DEAD: no run for %s, past its %d-minute silence limit" % (human(ago), limit)
    return True, "live (last run %s ago)" % human(ago)


def human(minutes):
    if minutes >= 120:
        return "%.0f hours" % (minutes / 60.0)
    return "%.0f min" % minutes


def plan(rows, open_titles):
    """Issue actions: `[(action, name, text)]`, action `open` or `close`. Pure.

    `rows` is `[(name, ok, text)]`; `open_titles` is the set of open issue titles. An issue opens
    only when a stopped watcher has none, and closes only when a running one has one."""
    acts = []
    for name, ok, text in rows:
        title = TITLE_PREFIX + name
        if not ok and title not in open_titles:
            acts.append(("open", name, text))
        elif ok and title in open_titles:
            acts.append(("close", name, text))
    return acts


def issue_body(name, text):
    return (
        "`%s` is not running: %s.\n\n"
        "The merge drain is what lands green pull requests, and trunk-health is what says main is red. "
        "While either is stopped, work waits unattended.\n\n"
        "To restart it: `gh workflow enable %s.yml` if it is disabled, or open its latest run if it "
        "is failing. This issue closes itself the next time the watcher runs.\n\n"
        "Opened by `helpers/watcher_watch.py`, milestone 723 (a stopped merge watcher is reported within "
        "three of its own intervals), provisional. The rule and its measured silence floor are in that file's header."
        % (name, text.replace("DEAD: ", ""), name)
    )


def gh(args, token=None):
    env = dict(os.environ)
    if token:
        env["GH_TOKEN"] = token
    return subprocess.run(["gh"] + args, capture_output=True, text=True, env=env)


def read_cron(name):
    try:
        with open(os.path.join(WORKFLOWS, name + ".yml")) as f:
            m = re.search(r'cron:\s*"([^"]+)"', f.read())
    except FileNotFoundError:
        return None
    return m.group(1) if m else None


def collect(now):
    """`[(name, ok, text, last_run)]`, or None when the API could not be asked."""
    read = os.environ.get("WATCHER_READ_TOKEN") or None
    out = []
    for name in WATCHERS:
        w = gh(["api", "repos/%s/actions/workflows/%s.yml" % (REPO, name), "--jq", ".state"], read)
        r = gh(["api", "repos/%s/actions/workflows/%s.yml/runs?per_page=1" % (REPO, name),
                "--jq", ".workflow_runs[0].created_at // empty"], read)
        if w.returncode or r.returncode:
            print("watcher_watch: could not read %s: %s" % (name, (w.stderr or r.stderr).strip()),
                  file=sys.stderr)
            return None
        last = r.stdout.strip() or None
        ok, text = verdict(name, w.stdout.strip(), read_cron(name), last, now)
        out.append((name, ok, text, last))
    return out


def check(quiet):
    rows = collect(datetime.datetime.now(datetime.timezone.utc))
    if rows is None:
        return 2
    bad = 0
    for name, ok, text, last in rows:
        if ok and quiet:
            continue
        print("%-32s %-24s %s" % (name, last or "never", text))
        bad += not ok
    return 1 if bad else 0


def sync():
    rows = collect(datetime.datetime.now(datetime.timezone.utc))
    if rows is None:
        return 2
    listed = gh(["issue", "list", "--repo", REPO, "--state", "open", "--limit", "100",
                 "--json", "number,title"])
    if listed.returncode:
        print("watcher_watch: could not list issues: %s" % listed.stderr.strip(), file=sys.stderr)
        return 2
    issues = {i["title"]: i["number"] for i in json.loads(listed.stdout)}
    write = os.environ.get("GH_TOKEN") or None
    fallback = os.environ.get("WATCHER_FALLBACK_TOKEN") or None
    status = 0
    for action, name, text in plan([(n, ok, t) for n, ok, t, _ in rows], set(issues)):
        title = TITLE_PREFIX + name
        if action == "open":
            args = ["issue", "create", "--repo", REPO, "--title", title, "--body", issue_body(name, text)]
        else:
            args = ["issue", "close", str(issues[title]), "--repo", REPO,
                    "--comment", "%s is running again: %s." % (name, text)]
        for who, token in (("nife-smelter", write), ("fallback token", fallback)):
            if token is None and who != "nife-smelter":
                continue
            done = gh(args, token)
            if done.returncode == 0:
                print("watcher_watch: %s issue for %s as %s: %s" % (action, name, who, done.stdout.strip()))
                break
            print("watcher_watch: %s as %s failed: %s" % (action, who, done.stderr.strip()), file=sys.stderr)
        else:
            status = 2
    return status


def selftest():
    ok = True

    def expect(label, got, want):
        nonlocal ok
        if got != want:
            ok = False
            print("FAIL %s: got %r want %r" % (label, got, want), file=sys.stderr)

    # Intervals from the cron shapes this tree writes.
    for cron, want in [("*/5 * * * *", 5), ("0 8 * * 1", 10080), ("0 6 * * *", 1440),
                       ("0 * * * *", 60), ("0 */6 * * *", 360), ("0 9 1 * *", 43200),
                       ("*/15 * * * *", 15), ("5-10 * * * *", None), ("nonsense", None)]:
        expect("cron " + cron, cron_interval_minutes(cron), want)

    # The rule is calef's three intervals, raised to the measured floor.
    expect("drain limit", silence_minutes("merge-drain", "*/5 * * * *"), 180)
    expect("unlisted watcher gets three intervals", silence_minutes("other", "*/5 * * * *"), 15)
    expect("unparsable cron falls back to the floor", silence_minutes("trunk-health", "weird"), 720)

    now = parse_time("2026-10-03T12:00:00Z")
    # The 2026-09-30 incident: a disabled workflow is reported with no wait at all, however recent
    # its last run was.
    expect("disabled is dead at once",
           verdict("merge-drain", "disabled_manually", "*/5 * * * *", "2026-10-03T11:59:00Z", now)[0], False)
    expect("62 hours silent is dead",
           verdict("merge-drain", "active", "*/5 * * * *", "2026-10-01T00:00:00Z", now)[0], False)
    expect("a run 20 minutes ago is live for the drain (the floor)",
           verdict("merge-drain", "active", "*/5 * * * *", "2026-10-03T11:40:00Z", now)[0], True)
    expect("trunk-health at its longest measured gap is live",
           verdict("trunk-health", "active", "*/5 * * * *", "2026-10-03T03:41:00Z", now)[0], True)
    expect("trunk-health past its floor is dead",
           verdict("trunk-health", "active", "*/5 * * * *", "2026-10-02T23:00:00Z", now)[0], False)
    expect("never ran is dead", verdict("merge-drain", "active", "*/5 * * * *", None, now)[0], False)
    expect("dead text starts DEAD, which trunk-health.yml greps for",
           verdict("merge-drain", "disabled_manually", None, None, now)[1].startswith("DEAD"), True)

    # One issue per stop, closed on recovery, and silent otherwise.
    t = TITLE_PREFIX
    rows = [("merge-drain", False, "DEAD: x"), ("trunk-health", True, "live")]
    expect("opens for a stopped watcher with no issue", plan(rows, set()), [("open", "merge-drain", "DEAD: x")])
    expect("does not re-open", plan(rows, {t + "merge-drain"}), [])
    expect("closes a running watcher's issue",
           plan(rows, {t + "merge-drain", t + "trunk-health"}), [("close", "trunk-health", "live")])
    expect("an unrelated issue is ignored", plan(rows, {"something else"}), [("open", "merge-drain", "DEAD: x")])

    print("watcher_watch selftest: %s" % ("ok" if ok else "FAILED"))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ["--selftest"]:
        return selftest()
    if len(argv) >= 2 and argv[1] == "check":
        return check("--quiet" in argv[2:])
    if argv[1:] == ["sync"]:
        return sync()
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
