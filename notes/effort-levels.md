# What `claude --effort` buys, measured against a real repository query

**Provisional name.** Written 2026-09-23. `claude --effort <level>` has existed the whole time this
project has (`claude --help` lists it) and no lane, rented or otherwise, has ever passed it.
`helpers/open-lane.sh` now takes `OPEN_LANE_EFFORT`; this note is the measurement that set its
default, run before trusting a lever nobody had turned.

## What the CLI accepts

`claude --help` lists exactly five levels: `low`, `medium`, `high`, `xhigh`, `max`. Confirmed by
running each; none was rejected.

## The task

A read-only, checkable, real query against this repository: *in `design/roadmap/`, at the top
level only (not `proposals/`), find every milestone file whose Gate line names `DECISION`, as a
markdown table of milestone and slug, plus a total count.* Ground truth was computed once with
`grep` and never touched again, so scoring a run means comparing its output against a fixed file,
not against a second LLM call.

**The first wording was ambiguous, and that ambiguity is itself a finding (see below).** It asked
for lines beginning exactly `**Gate: DECISION.**`, meant to exclude compound gates like
`**Gate: MILESTONE 75, DECISION.**`. It did not say what to do with the other kind of compound,
`**Gate: DECISION, MILESTONE 23.**`, where `DECISION` is first. Ground truth (67 milestones) counts
those; the literal wording does not (63). A second, disambiguated wording spelled out both cases.
Both wordings were run at all five levels, twice each, for 20 runs total. Every run was a fresh
`claude` process (`--output-format json`, `--allowedTools "Read,Grep,Glob"`,
`--permission-prompts none`, `--no-session-persistence`), scored by parsing its own final table and
`Total: N` line against the fixed ground truth.

## Result 1: effort did not move correctness, in either direction

**Ambiguous wording, 10 runs:** all five levels, both runs each, produced the identical answer (63,
the same four milestones missing: 39, 48, 347, 493). Not close-but-different: byte-for-byte the same
list. `max` did not do a more careful reading than `low`; it did the identical reading, slower.

**Disambiguated wording, 10 runs:** all five levels, both runs each, produced the exactly correct
answer (67, full list, matching ground truth). `low` was right both times. `max` was also right both
times and bought nothing beyond that.

So on this task, over 20 runs, correctness was **100% determined by the wording of the prompt and
0% determined by the effort level.** A bad prompt gets the same wrong answer from every effort
level; a good prompt gets the same right answer from every effort level. That is a genuine null
result, not an absence of a result: effort is not the lever that fixes an underspecified brief, and
raising it is not a substitute for writing the brief correctly. One run (`xhigh`, ambiguous wording,
run 2) said so itself, unprompted, in its own output:

> I matched only lines that start with `**Gate: DECISION.**`. That leaves out the four compound
> gates (39, 48, 347, 493) and the historical mentions of `Gate: DECISION` in the text of other
> blocks.

That is a correct diagnosis of the ambiguity, delivered alongside the same literal-reading answer
every other level gave. Higher effort produced a clearer account of the choice it made, not a
different choice.

## Result 2: effort level is a real dial on time, turns and (noisily) cost

Wall-clock and turn count scaled cleanly with level, on both wordings:

| level  | avg wall-clock | avg turns | avg output tokens |
|---|---|---|---|
| low    | ~13s  | 2.0 | ~1,600 |
| medium | ~23s  | 3.2 | ~2,200 |
| high   | ~25s  | 4.0 | ~2,500 |
| xhigh  | ~32s  | 4.8 | ~2,900 |
| max    | ~68s  | 8.0 | ~7,000 |

(Averaged across both wordings, 4 runs per level. The 20 JSON results this table summarizes were
not checked in; see the BUGS section below.) `max` runs roughly 5x longer and takes roughly 4x the
turns of `low`, for an identical answer.

**Cost is real but noisy, and the noise has an honest cause: prompt caching.** Each `claude
--effort <level> -p ...` invocation loaded this repository's full `AGENTS.md`/skills/system-prompt
context fresh (about 40-43K tokens), unless a prior run in the same benchmark had written that
exact content to Anthropic's server-side cache within its TTL, in which case the second run paid
the much cheaper cache-read rate instead of the cache-write rate. Runs launched back-to-back
against the same content therefore cost 3-7x less than the first run of a batch, purely from
ordering, not from effort level. With that caveat stated plainly: even comparing worst-case
(cache-miss) runs, `low` ($0.37-0.38) to `max` ($0.68-0.74) is roughly a 2x spread, growing to
5-10x once a `max` run's much larger output (thinking and verification tokens) is included.

**This cost is the CLI's own `total_cost_usd`, at list-price rates against a subscription
session, not a bill.** These runs went through `claude`'s ordinary session auth (this lane had no
`ANTHROPIC_API_KEY` and could not use `--bare`, and had no OpenRouter credentials to route through
the open-lane gateway), so no real dollar amount moved and no OpenRouter credit balance
was checked. `total_cost_usd` is Claude Code's own notional accounting of what the same usage would
cost at API list price (`"costBasis":"list"` in the raw JSON); it is the only cost number this lane
could get, and it is reported as exactly that, not as a charge.

## What was not measured, and why it matters

**The effort flag was never tested against the backend `helpers/open-lane.sh` actually drives.**
That script runs `claude --bare --effort <level>` with `ANTHROPIC_BASE_URL` pointed at a LiteLLM
gateway translating to an open-weight model over OpenRouter (`notes/open-model-lanes.md`). This
lane had no `OPENROUTER_API_KEY` and no running gateway to test against, so every run above went
straight to Claude. The gateway's configuration sets `drop_params: true`, which strips request
fields the upstream model does not recognise; whether an open-weight model's `/chat/completions`
translation preserves anything `--effort` sends, or silently drops it, is unmeasured. The default
chosen below is evidence-based for Claude and a guess, clearly labelled as one, for the rented
backend.

**One task shape, at one size.** This measured a bounded read-only grep-and-tabulate query over
about 600 files. It says nothing about whether effort matters for a task with a larger search space,
a multi-step edit, or a genuinely ambiguous judgement call rather than an ambiguous instruction. The
finding is scoped to what was run, not generalized past it.

## What this sets as the default, and why

**`OPEN_LANE_EFFORT` defaults to `low`.** Not because low is assumed safe, but because on the one
task measured, no effort level bought more correctness, so the correct move is to buy less time and
less spend for the identical outcome. This is the finding stated in the brief's own terms: a real
null result, reported as one, rather than a manufactured recommendation. If a future task shape
*does* show effort moving correctness, this default should change with that evidence, not before
it.

**This should be re-measured through the actual gateway once a lane can reach it on cordoba**
(`notes/open-model-lanes.md`). That is the gap that matters most: this
note answers "does `--effort` change what Claude does," which it does (time, turns, tokens) without
changing correctness on this task; it does not yet answer "does `--effort` reach the open-weight
model at all."

## BUGS

- **No raw run data is checked in.** The 20 JSON transcripts and the scoring scripts live in a
  scratch directory outside this repository and are gone once that scratch is cleared. Anyone
  wanting to re-verify has to re-run the benchmark, not re-read a saved transcript. This is a lower
  rung than the tree's own convention prefers; a lane with more time budget should save the raw JSON
  under `notes/` or a `bench/` directory rather than a summary table.
- **Two runs per level per wording is a small sample.** It is enough to see that the ambiguous
  wording's answer was not noisy (it was identical, not merely similar, across all 10 runs), which
  is the strongest kind of evidence a small sample can give. It is not enough to bound a confidence
  interval on the cost or wall-clock numbers, which is why this note reports ranges and says "noisy"
  rather than a mean plus error bars.
- **Only Claude, never the rented backend.** Stated above; repeated here because it is the most
  consequential gap and the easiest one to lose track of.
