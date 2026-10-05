# What a session carries, and why delegation is about context

`AGENTS.md` never says the word context, and there is no note about it. That is exactly the
shape the ladder already names: *"a fact that exists only at a call site or in a report, with no
artifact anyone can read"* is the tell that you are on too low a rung. A maintainer session's own
token consumption, and what to do about it, has been living only in one session's head. This is
that fact, written down.

## The measurement, 2026-09-22

One maintainer session spent roughly 233 million tokens. The twenty lanes it dispatched that
day spent about 5 million between them (the same day's count behind
`notes/open-model-lanes.md`). The session is roughly 98% of the total; the lanes it briefed,
gated and merged are roughly 2%.

The dominant term is carrying and re-reading a large context every turn, not reasoning or
output. A maintainer session accumulates `AGENTS.md`, every file it has read, every gate's output,
every lane report, and every prior turn of its own conversation, and all of that rides along on the
*next* turn too, because that is how a context window works. A lane starts cold, does one thing, and
returns a report. Nothing it read outlives it.

## The consequence, and it inverts the obvious framing

The instinct is to think delegation is worth it because lanes are cheaper or **faster**: a
smaller model, run in parallel, doing the same work for less. That is a real effect and it is not
the first-order one here.

The first-order value is that a lane can read 547 files and return four paragraphs. Read those
547 files in the maintainer session directly, and all 547 are now in that session's context for the
rest of its life: every one of them gets carried and re-read on every subsequent turn, whether or
not it is still relevant. A lane that reads the same 547 files and hands back a conclusion pays for
that reading once, in a context that then closes.

Get this the right way round, because it changes which tasks are worth delegating. A task is worth
delegating when it would drag a lot of text into the session, even if it is small and even if the
session could do it faster itself. Cost-per-token and wall-clock are real, and they are the second
column, not the first.

## Why lowering `--effort` is not the answer

calef, 2026-09-23: *"I don't want to adjust the main lane effort. I want you to delegate effectively
and take other steps to manage your context."*

`--effort` tunes how much a turn thinks and how much it writes. It does not touch what gets read.
The 233M-token session was not spending its budget on long reasoning chains or verbose replies; it
was spending it on input, carried forward every turn. Turning effort down spends quality on the
wrong side of the ledger: it makes each turn's answer worse without shrinking the thing that is
actually large. There is a real second-order effect, since a shorter turn accumulates less context
per turn than a longer one, but it is second-order, and it is unmeasured here.

## The practices, each earned by a specific failure on 2026-09-22/23

- **Never pull a CI log into the session.** `gh run view --log` on a whole job is thousands of lines
  to find one assertion, and every one of those lines is now in the session's context permanently.
  `briefs/triage-a-failing-check.md` exists for exactly this: it runs in a lane, reads past the
  cleanup-step noise, and returns the one line that matters.
- Ask a gate for its exit code, not its output. `script/lint >/dev/null 2>&1; echo $?`, and read
  the actual output only when the code is non-zero. A gate that passes has nothing worth carrying.
- Delegate conflict resolution. A rebase run through `briefs/rebase-onto-main.md` was measured
  at $0.055, correct first time, no redo (`notes/open-model-lanes.md` has the run). The
  maintainer session never saw the diff that produced the conflict, only the report that it was
  resolved.
- **Spot-check a lane's work; do not re-derive it.** Read one sample of what a lane changed, not the
  whole task again. Re-deriving the answer to check it defeats the point of having delegated it: the
  session ends up carrying everything the lane carried, plus the lane's report on top.
- Any question that needs several files opened goes to a subagent. Not a size threshold, a
  shape one: if answering means reading around the tree to assemble a picture, that assembly belongs
  in a context that will close, not in the one that has to keep going.
- Keep pull request bodies and commit messages proportionate. They are composed in the
  session's own context, so a long one is written once and then sits in that context a second time,
  reread on the next turn along with everything else. Say what a reader needs, not everything the
  session knows.

## BUGS

- One session, one day. The 233M/5M figures are a single measured session, not a controlled
  comparison across sessions or across days. Nothing here rules out that day being unusually
  context-heavy for reasons specific to it (a `design/roadmap/` sweep, several stalled pull
  requests read in full).
- No per-turn breakdown exists. The claim that the dominant term is carried context rather than
  reasoning or output is an inference from what the session was doing (reading files, rebriefing
  lanes, rereading gate output) rather than a token-by-token accounting. Anthropic's own usage
  reporting was not instrumented for this session; the 233M figure is a session-level total.
- **The `--effort` second-order claim is asserted, not measured.** Nobody has run the same
  maintainer workload at two effort settings and diffed the resulting context growth.
- This note names practices, not a mechanism that enforces them. Nothing gates "did this turn
  pull a CI log directly" or "was this pull request body proportionate." It is rung four on
  `AGENTS.md`'s own ladder (a written record, not a gate), which is honest about being the floor
  rather than the fix.
