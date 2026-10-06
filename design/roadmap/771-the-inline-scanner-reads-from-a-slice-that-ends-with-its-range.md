---
status: NOT-STARTED
promoted_from: the-inline-scanner-reads-from-a-slice-that-ends-with-its-range
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 771. The inline scanner reads from a slice that ends with its range

Raised by the lane for milestone 637 (triage the crates the 2026-09-21 mutation census measured for the first time), from the `documentation` triage in `notes/mutation-testing/census-2026-10-03-triage.md`.

## What is owed

`Renderer::inline` in `crates/documentation/src/render.rs` scans a byte range of a shared 2,048-byte line buffer. Its one-byte lookaheads (`i + 1 < end`, `rb + 1 < end`, `open < end`) are guarded by the range's end, but the slice they index is the whole buffer, so a read one byte too far lands on whatever the previous line left there. Eight mutants of those guards survive because the next check absorbs each over-read. The code is right today, and a test cannot say so.

Hand `inline` a slice that stops at the range's end, `&self.line[..end]`, and index only that. An over-read then panics instead of reading a stale byte. That makes the whole class unrepresentable rather than harmless, and it is the top rung of the ladder in `CLAUDE.md`.

## What it makes unrepresentable

A lookahead past the range. Of the 33 equivalent survivors in `documentation`, eight are this shape and would stop being equivalent: they would panic and a test would kill them.

## Risk and cost

The renderer has 71 integration tests and a corpus check that renders every page in the repository, so a regression shows in milliseconds. The risk is a branch that today relies on reading `line[end]` on purpose. The sweep found none, and the corpus check would catch one. The change touches every branch of `inline` (about 90 lines) and `closer`. Estimated at one lane, a few hours. Cost to performance: none expected, since a slice bound is a compare the compiler already pays. Measure it with the `doc` render of the largest page before and after.

## Done when

`inline` and `closer` index only a slice that ends with the range, the eight survivors are killed or gone, and the corpus check and 71 tests pass.

## Index row

`Renderer::inline` indexes a shared line buffer that outlives the range it scans, so an off-by-one read returns stale bytes instead of failing. Handing it a slice that ends with the range makes the over-read panic. That turns eight equivalent mutation survivors in `documentation` into killable ones.
