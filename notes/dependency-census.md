# The dependency census: what 547 milestones actually stood on

Measured 2026-09-22, by four parallel lanes reading every milestone block in
`design/roadmap/` and classifying each reference to another milestone. The data is
`notes/dependency-census/dependencies.tsv`; this note is what it says and what it does not.

## Why it was taken

§207 (the roadmap is a graph, and the block says so in fields a script can walk) proposes that a milestone block declare its
dependencies in fields a script can walk. Before building that, calef asked the question that
decides whether it is worth building: *"I think we have not captured the dependencies well because we
create the dependent tasks after we've cleared the dependencies."*

He was right that the obvious measurement is the wrong one. Counting edges in the ready queue
counts a population selected for having no live dependencies. The finished work is where the graph
is observable, so the census reads all of it.

## What it found

| | |
|---|---|
| References classified | **3,381** (after removing 310 duplicates) |
| Of those, genuine prerequisites | **128** |
| Everything else | context, follow-on, or superseded |
| Dependency edges **declared** in `Gate:` fields | **15** |

The headline is the ratio, not the rate. 3.8% of references are prerequisites, which sounds like
a refutation and is not: 128 real edges against 15 declared ones is an eightfold
under-declaration. The graph exists. It is thin, and it is almost entirely unstated.

## The shape is flat, and that is the useful result

No milestone is depended on by more than five others. The most-depended-on are 25, 55, 64, 74 and
218, all at five; the most dependent blocks carry four prerequisites each.

There is no deep critical path. This roadmap is wide rather than tall, which has a consequence
worth stating plainly: the work is close to embarrassingly parallel, and dependencies are not what
bounds lane count. Files and memory are, exactly as `AGENTS.md` already says. Ranking milestones by
depth in this graph would buy very little, because the depth is not there.

## When the dependency was known

Among prerequisite edges: 77 stated at creation, 50 added later. So roughly 40% of the
dependencies this roadmap records were discovered while doing the work rather than while planning it.

Read that number with care. `git blame` records when a *sentence* was written, not when a
*dependency* was known. Milestone 76 (split the roadmap: an index and one file per milestone) backfilled the first day's history on 2026-08-03 in a
single sweep, so a large block of references date to that day regardless of when the relation was true. The
`discovery` column is therefore a lower bound on planning foresight and an upper bound on nothing.

## BUGS

- **The census can only see dependencies somebody wrote a sentence about.** A milestone that
  genuinely needed another, where nobody said so, is invisible here and always will be. This is the
  limit that matters: the census gives history and pattern, and cannot give a complete graph. It
  is why the forward convention in §207 is worth more than any backfill.
- **Classification was done by a cheap model and its precision is about 80%.** A twenty-row sample
  found sixteen genuine. Every failure had one cause: negation. Sentences reading *"Nothing gated
  this after milestone 107 (the socket contract learns to accept) merged"* and *"Independent of
  milestone 142 (a text display good enough that people use it instead of a GUI)'s type-and-scanout
  work"* (the
  two milestones are cited here only as the text of the misread sentences, not as records this note relies on) were
  filed as dependencies although both say the opposite. Five such rows were found mechanically and
  demoted, and they carry `RECLASSIFIED-NEGATION:` in the evidence column. More certainly remain.
- The four shards disagreed with each other, returning prerequisite rates of 7.0%, 3.3%, 1.9% and
  2.5% on adjacent milestone ranges from an identical brief. Treat any per-shard figure as noise and
  only the pooled number as meaningful.
- `discovery` is `UNKNOWN` for 77 rows in one shard, where `git blame` output could not be parsed
  because of non-standard author name formats.
- Ten milestone files reference nothing at all, and three more could not be dated because
  `--diff-filter=A` finds no creation record for a renamed file. `--follow` would fix the latter and
  the brief did not ask for it.
