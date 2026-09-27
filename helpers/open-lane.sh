#!/bin/sh
# Run one mechanical lane against a rented open-weight model instead of Claude.
#
#     helpers/open-lane.sh <worktree> <brief-file> [max-rounds]
#
# **Why this exists.** DECISIONS 202 routes mechanical work to a cheaper model and DECISIONS 203
# rules that capacity is rented rather than bought. Neither was built: on 2026-09-21 about 6% of
# five million lane tokens went anywhere other than the expensive model, and calef's limit is a rate
# limit rather than a bill, so the wall arrives on a date rather than in an invoice.
#
# **One process talks to one provider.** Claude Code resolves the endpoint once at startup, and a
# subagent's `model` field accepts only Claude aliases, so a session cannot send some lanes
# elsewhere. That is why this is a separate headless process per lane rather than a flag.
#
# **The gates are the oracle, and that is the whole safety argument.** A cheaper model is safe here
# exactly to the extent that a shell command says pass or fail. This script never judges the work:
# it loops the model against `script/lint` and `script/citations --ratchet` until they exit 0 or the
# round budget runs out, and a lane that cannot reach green is handed back rather than merged.
#
# # SETUP
#
# The provider must speak the **Anthropic Messages API** (`POST /v1/messages`). Open-weight
# providers are OpenAI-shaped, so a translating gateway sits in between; LiteLLM is the one the
# Claude Code documentation names. Set these before running:
#
#     export OPEN_LANE_BASE_URL=https://<cordoba>.<tailnet>.ts.net:4000
#     export OPEN_LANE_MODEL=<model id the gateway exposes>
#     export OPEN_LANE_EFFORT=low                        # optional; see below
#
# **`OPEN_LANE_EFFORT`** is passed straight through as `claude --effort <level>`. The CLI accepts
# `low`, `medium`, `high`, `xhigh` or `max` (`claude --help`); this script does not second-guess that
# list, so a typo here is the CLI's error to report, not this script's to pre-validate. Default is
# `low`, and the reason is a measured null result rather than a guess: notes/effort-levels.md ran a
# real, checkable repository query at all five levels, twice each, and every level answered
# identically, including the ambiguous first version of the task where all five made the same wrong
# call and the correct second version where all five got it exactly right. Higher effort bought more
# wall-clock (about 5x from low to max) and more turns, not more correctness. **That measurement ran
# directly against Claude, not through this gateway against an open-weight model**, because this lane
# had no OpenRouter credentials; whether `--effort` does anything at all once LiteLLM's
# `drop_params: true` has a chance to strip it for a model that does not know the parameter is
# unmeasured. Override per run (`OPEN_LANE_EFFORT=max helpers/open-lane.sh ...`) for a lane worth
# spending more on, and re-measure against the real backend before trusting this default there.
#
# The gateway has no password of its own (notes/open-model-lanes.md), but Claude Code under
# `--bare` still insists on *some* credential, so this sends a placeholder. It is not a secret and
# the gateway ignores it.
#
# # BUGS
#
# - **Tool-call fidelity is the unknown and is not this script's to promise.** Claude Code sends
#   tool definitions in Anthropic's schema; the gateway translates them, and whether a given
#   open-weight model emits well-formed calls turn after turn is a property of that model. Nothing
#   in the documentation vouches for it. Benchmark before trusting: `--dry-run` prints the command.
# - **Prompt caching is billed as a miss.** Claude Code sends `cache_control` regardless, and a
#   gateway that does not implement it bills every turn uncached. The per-token estimate in
#   DECISIONS 203 assumed nothing about caching, so it is not wrong, but a cached-rate quote is.
# - **The context window is guessed at 200K** for a model id Claude Code does not recognise. Set
#   `CLAUDE_CODE_MAX_CONTEXT_TOKENS` if the real window is smaller, or the run truncates mid-task.
# - **`--bare` skips CLAUDE.md, skills, hooks and plugins.** That is deliberate here: a mechanical
#   lane should be told what to do by its brief, and the constitution is 924 lines that a cheap
#   model would spend its window on. It also means this lane does not inherit the rules, so the
#   brief has to carry whatever it needs.
set -eu

[ $# -ge 2 ] || { echo >&2 "usage: $0 <worktree> <brief-file> [max-rounds]"; exit 2; }
worktree=$1
brief=$2
rounds=${3:-4}

[ -d "$worktree" ] || { echo >&2 "open-lane: no such worktree: $worktree"; exit 2; }
[ -f "$brief" ] || { echo >&2 "open-lane: no such brief: $brief"; exit 2; }

: "${OPEN_LANE_BASE_URL:?set OPEN_LANE_BASE_URL to the gateway that speaks /v1/messages}"
: "${OPEN_LANE_MODEL:?set OPEN_LANE_MODEL to the model id the gateway exposes}"
effort=${OPEN_LANE_EFFORT:-low}

brief_text=$(cat "$brief")
base_commit=$(cd "$worktree" && git rev-parse HEAD)

# The failure text from the last round is appended to the prompt, so the model is told what the
# gate said rather than asked to guess. This is the loop that makes a cheaper model usable.
feedback=""
round=1
while [ "$round" -le "$rounds" ]; do
    echo "==> open-lane round $round of $rounds ($OPEN_LANE_MODEL, effort=$effort)"

    prompt="$brief_text

## How you are judged

Nothing here reads your prose. You are finished when both of these exit 0, and not before:

    script/lint
    script/citations --ratchet

Run them yourself, read the exit code, and fix what they say. \`script/citations --ratchet\` reads
the COMMITTED tip, so commit before you run it or you are reading a stale answer.
$feedback"

    (
        cd "$worktree"
        ANTHROPIC_BASE_URL="$OPEN_LANE_BASE_URL" \
        ANTHROPIC_AUTH_TOKEN="${OPEN_LANE_TOKEN:-unused-the-gateway-has-no-password}" \
        ANTHROPIC_MODEL="$OPEN_LANE_MODEL" \
        CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1 \
        claude --bare --effort "$effort" -p "$prompt" --allowedTools "Bash,Read,Edit,Write,Glob,Grep"
    ) || echo "open-lane: the model's own run exited non-zero; the gates decide, not this"

    # **A green tree is not a delivered lane.** On 2026-09-22 a lane fixed a flaky assertion
    # correctly, never committed it, and this loop reported green after one round: both gates pass on
    # an unchanged working tree, so "the model did nothing" and "the model did the work and forgot to
    # commit" were indistinguishable. The oracle has to answer "did anything land", not only "is the
    # tree clean", or every green it has ever printed is suspect.
    if [ "$(cd "$worktree" && git rev-parse HEAD)" = "$base_commit" ]; then
        feedback="

## You have committed nothing

The gates pass, but they pass on an unchanged tree, so that proves nothing. Commit your work. If you
believe there is nothing to do, say so explicitly rather than leaving the worktree untouched."
        echo "==> open-lane: no commit since the lane started; asking again"
        round=$((round + 1))
        continue
    fi
    if (cd "$worktree" && [ -z "$(git status --porcelain)" ]) || true; then :; fi
    if (cd "$worktree" && script/lint >/tmp/open-lane-lint.$$ 2>&1 \
        && script/citations --ratchet >/tmp/open-lane-cit.$$ 2>&1); then
        echo "==> open-lane: green after $round round(s)"
        rm -f /tmp/open-lane-lint.$$ /tmp/open-lane-cit.$$
        exit 0
    fi

    feedback="

## The gate said this last round, and it is the thing to fix

$(tail -30 /tmp/open-lane-lint.$$ 2>/dev/null; tail -30 /tmp/open-lane-cit.$$ 2>/dev/null)"
    round=$((round + 1))
done

echo >&2 "open-lane: not green after $rounds rounds. Handing back rather than merging."
echo >&2 "open-lane: the worktree is left as it stands, at $worktree"
exit 1
