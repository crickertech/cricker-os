# helpers/ci-failing.jq: which REQUIRED checks are failing on a pull request's head commit.
#
# Provisional name, minted with helpers/ci-failing.sh (see that file and notes/merge-queue.md).
#
# Input: an array of check-run objects for one commit, as `gh api repos/R/commits/SHA/check-runs
# --paginate --slurp` returns them once the pages are flattened (each has `id`, `name`,
# `conclusion`, `status`, `html_url`). Argument `$required`: the array of required check names.
#
# Output: one object per failing required check, `{name, url}`, sorted by name.
#
# **A check name is failing only if its most recent run failed.** Runs of one name accumulate on a
# commit (a rerun, a cancelled duplicate, a rerun after a flake), and a failure superseded by a
# later success is not a failure; #1592 had stale failures sitting beside newer successes. "Most
# recent" is the highest check-run id, because ids only grow and `started_at` is null on a run that
# has not started. A newer run that is queued, in progress or cancelled therefore hides an older
# failure, deliberately: a rerun is underway, or a newer push owns the result, and either way the
# old verdict is no longer the head's. Only `failure` and `timed_out` count, the same pair
# notes/merge-queue.md says to trust over `statusCheckRollup`.
#
# **The `architect hold` check is ignored by name.** It goes red on purpose while `needs-architect`
# is on a pull request, which is a person's queue and not a failing build; flagging it would label
# every pull request waiting on an architect.
#
# A required check with no run at all is not failing; it is missing, which is a different fact and
# is `ready status` and the queue's business.

def failing($required):
  group_by(.name)
  | map(sort_by(.id) | last)
  | map(select(.name as $n | $required | index($n)))
  | map(select(.name | startswith("architect hold") | not))
  | map(select(.conclusion == "failure" or .conclusion == "timed_out"))
  | map({name: .name, url: .html_url})
  | sort_by(.name);
