# helpers/missing-check.jq: a required status check that never reported on a head whose workflow
# runs have all finished. Lane/drain-missing-check, 2026-10-07 UTC; the name is provisional.
#
# #1814 on 2026-10-07: CI run 37654234928 finished `failure` with all 20 of its jobs green, and the
# required "cpu matrix (riscv64 across QEMU CPU models)" job was never created. Nothing was red and
# nothing was running, so the armed pull request sat BLOCKED until calef reported it (#1816 and
# #1812 were the same during a GitHub incident, with the Kani jobs absent). A check that does not
# exist is not a failing check, so helpers/ci-failing.sh cannot see it.
#
# Input, one object (helpers/merge-drain.sh `missing_check_scan` builds it; this file must not
# call `gh`):
#   required  the contexts of the `main` ruleset's required_status_checks rule
#   present   the names of the check runs and commit statuses that exist at the head
#   runs      the workflow runs at the head: { id, name, status, conclusion, run_attempt, event }
#   owners    { "<context>": "<name of the workflow that has a job of that name>" }
#
# Output: nothing, when nothing is absent or a pull-request run is still going (a check not yet
# created is not a missing one), else
#   { absent: [context...], rerun: [{ id, name }...], tried: bool }
# `rerun` is the newest run of each absent check's workflow, when it is at `run_attempt` 1: the run
# is the record that a rerun was tried, as in helpers/cancelled-duplicate.jq, so the same head is
# never rerun twice. `tried` is true when nothing is left to rerun (every owner's newest run was
# already rerun, or no workflow or run owns the check), which is when the drain says so out loud.

def missing_checks:
  (.runs | map(select(.event == "pull_request" or .event == "pull_request_target"))) as $prs
  | select(($prs | map(select(.status != "completed")) | length) == 0)
  | .owners as $owners
  | .runs as $runs
  | (.present) as $present
  | [ .required[] | select(. as $c | $present | index($c) | not) ] as $absent
  | select($absent | length > 0)
  | [ $absent[] | $owners[.] as $w | select($w != null)
      | [ $runs[] | select(.name == $w and (.event == "pull_request" or .event == "pull_request_target")) ]
      | sort_by(.id) | last | select(. != null) ] as $newest
  | ($newest | map(select((.run_attempt // 1) == 1) | { id, name }) | unique_by(.id)) as $rerun
  | { absent: $absent, rerun: $rerun, tried: ($rerun | length == 0) };
