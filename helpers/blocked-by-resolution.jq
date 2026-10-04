# helpers/blocked-by-resolution.jq: given a paused draft's own `unblocked` label and the STATE of
# every pull request its `Blocked-by:` line names, decide what the drain should do.
#
# Consumed by helpers/merge-drain.sh's unblocked_drafts, spliced the same way
# helpers/queue-eligible.jq is: jq -c "$(cat helpers/blocked-by-resolution.jq)"'resolution'. The
# split keeps the decision testable without a `gh` call: helpers/blocked-by-resolution-selftest.sh
# checks it against fixtures without touching GitHub, and script/lint runs it.
#
# Input: {"has_label": bool, "blockers": [{"number": N, "state": "MERGED"|"CLOSED"|"OPEN"}, ...]}
#
# `has_label` short-circuits everything else: a draft already carrying `unblocked` is left alone
# here (the comment marker is still the load-bearing dedupe).
#
# Output, one of four shapes:
#   {"status": "already-labelled"}
#   {"status": "waiting"}                                    -- at least one blocker still open
#   {"status": "closed-unmerged", "numbers": [...]}           -- none open, at least one closed unmerged
#   {"status": "unblocked", "numbers": [...]}                 -- every blocker merged
#
# `closed-unmerged` is checked before `unblocked` on purpose: a blocker that closed without
# merging is the anomaly worth a person's attention even when every other blocker did merge, so
# one closed-unmerged blocker in the list is enough to report it that way rather than folding it
# into a plain "unblocked".
def resolution:
  if .has_label then
    {status: "already-labelled"}
  elif ([.blockers[] | select(.state != "MERGED" and .state != "CLOSED")] | length) > 0 then
    {status: "waiting"}
  elif ([.blockers[] | select(.state == "CLOSED")] | length) > 0 then
    {status: "closed-unmerged", numbers: [.blockers[] | select(.state == "CLOSED") | .number]}
  else
    {status: "unblocked", numbers: [.blockers[] | select(.state == "MERGED") | .number]}
  end;
