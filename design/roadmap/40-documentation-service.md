---
status: BUILT
raised: 2026-07-30
built: 2026-08-26
---
# 40. Documentation as a system service: searchable, rendered, and installed by packages

Built, 2026-08-26. Phases 1 and 2 are built, phase 2's own known gap (ranking by raw
occurrence count rather than document length) closed 2026-08-22, and phase 3's caretaker-narrowing
increment is built too (DECISIONS §106, 2026-08-22): `doc <page>` renders at the prompt with no
`| wc` in front of it, which is the milestone's own headline demonstration. Phase 3's other half,
a graphical viewer, is struck rather than built. Traced to its origin 2026-08-26: calef's own
quoted direction below never asked for one; a graphical viewer was the scoping lane's own addition
("the first *application* on the display ladder that anybody would actually use"), carried forward
unquestioned through every later revision of this file, including the one that kept the status at
PARTIAL for it. calef, asked directly once the origin was traced: *"I don't need a graphical
viewer."* Struck below wherever it was named; the status moves to BUILT because the milestone's own
quoted direction, and everything a lane actually decided to build against it, is now complete.

What exists: `crates/manual` (a streaming markdown renderer, the byte layout of a search index, and
the query that reads one, all pure and host-tested in `tests/render.rs`), `components/src/doc.rs`, the
viewer program, which the shell can spawn as `Prog::Doc`, the store the image installs, `apropos` at
the nife prompt, and `script/apropos` over the whole repository. The renderer was written against
`line_editor`'s contract rather than `pulldown-cmark`, which the block's own text anticipated as the
alternative and which the crate's header records.

Phase 2 landed 2026-08-16. The store is built on the host from the `DOC_BUNDLES` table
(`cargo xtask manual`), installed into the filesystem image as `doc/<bundle>/` with `doc/bundles`
naming what is there, and searched from the prompt by **`apropos <word>`**, a shell builtin. The
whole of it is gated by `script/swish-check` on both architectures: `apropos capability` names the
pages, and the line after it grants `wc` exactly one of them. Search produces **names, never
capabilities**, which is why it may be a builtin: a searching *program* would have to be handed the
whole store to read every shard in it, which is more authority than the answer needs. See
notes/manual.md.

2026-08-18: the record was the defect three times over, and all three are fixed.

*In the renderer.* A fenced code block opened inside a block quote **never closed**: the closing
test ran against the raw line, so a quoted closing fence never matched its own opener and every line
to the end of the document rendered as quoted code. It survived because the corpus test cannot see
this class of failure at all (verbatim output loses no characters, and the check is a subsequence),
and because the one symptom it *could* see had been quarantined behind an honest `BUGS` comment
that refused to widen the filter until somebody answered the question. Answering it found the bug.
That refusal is the mechanism working, and it is worth reading before the next one is waved through.

*In the prose.* Three of notes/manual.md's `BUGS` entries described a system that no longer existed:
`doc <page>` was said to deadlock (it is refused, by `grant_plan::check_chain`, with a sentence that
names the fix), `doc <page> | wc` was said to render an empty stream (the head stage's input comes
off the plan now), and `MAX_TEXT_CHUNKS = 32` was said to truncate a page to 512 bytes (it is
`MAX_OUTPUT_CHUNKS = 4096`). Two of the three were closed by other lanes and nobody came back. The
correction is three lines in `script/swish-check` rather than three paragraphs, because this is the
milestone least allowed to describe a system that is not there.

*In the index.* `normalize` folds a query by dropping every non-alphanumeric byte, so a reader who
types `line_editor` looks up `lineeditor`; `tokens` split the *text* on that byte, so the builder
only ever wrote `line` and `editor`, and the term the query asked for was one no page could ever
have. In a repository whose prose is full of `snake_case` identifiers that is most of what anyone
would search for: `apropos fs_proto` answered "nothing says that" while dozens of pages said it. The
test that should have caught it asserted the property in its own first comment and then checked a
word with no underscore in it, which is the fence's shape again. Fixed in the writer, so
`apropos editor` keeps working and `apropos line_editor` starts; narrowing the reader instead would
have made it silently search for `line`, which is worse than no answer.

`script/apropos`, and it is milestone 117's finding rather than a nicety. Three stranger runs
have measured what a newcomer cannot reach by following this tree: `notes/net.md`,
`notes/capabilities.md`, **any** `design/decisions/` file, and `crates/abi/src/lib.rs`, which is
four syscall numbers and the whole design on one screen. None of them is hidden; nothing a person
would type led to them. This milestone already owned the machinery, pointed at the six pages a guest
can hold, so it is now also pointed at the 512 pages a checkout holds: the same
`manual::index::build` and `manual::index::search`, one shard per part of the tree, and a result
that names a repository path rather than a store location. Crate and program **module headers** are
indexed as pages, which is what makes `crates/abi/src/lib.rs` findable at all: the document a reader
wants about the ABI *is* that file's header, and no markdown page was ever going to fix it.

What remained, as of 2026-08-18, was one fork rather than a list. There was still no line a
person could type that rendered a page at the prompt, and the reason was not the shell's
scheduling. A process has one wait point, so a shell that feeds a chain cannot also read from it,
and no interleaving fixes that. What fixes it is somewhere for the viewer's output to go that is
**not the shell**, and the tree already had that thing: `terminal_sink_caretaker`, the terminal's
own sink adapter, where a declared second stream goes by default. Putting it in a tail stage's
*output* slot was a spawn-protocol decision, the same decision the pager and the color bit still
want, and notes/pipes.md had been holding it open since milestone 50: *"a shell that wanted a
program to print straight to the screen rather than through its own result endpoint could hand it
over, and would lose the ability to redirect that program at all."* DECISIONS §106 took that fork
on 2026-08-22, and it is built; see below. Phase 3's other half, the graphical viewer, still waits
on the display ladder as the block says below.

**2026-08-22: the fork is worked through, not just named.** DECISIONS §101 (notification objects),
decided 2026-08-20, ratified the *direction*: the `terminal_sink_caretaker` narrowing is "the right
short-term move" and stays valid as the permanent shape even once the notification object lands,
but it explicitly left milestone 40's own fork undecided: "That is milestone 40's fork, and this
decision does not take it." notes/tail-output-narrowing.md answers CLAUDE.md's six questions
against it: confirms the premise against `grant_plan::check_chain` rather than trusting this
block's own framing, prices the two previously-refused alternatives (a pull-based source, a
buffering stage) against measured numbers already in notes/pipes.md, and finds two things not
previously in the tree. The shell can reuse DECISIONS §26's already-built fault endpoint as its
completion signal instead of reading the child's bytes, and moving output off the shell's own read
loop opens a narrower race than notes/manual.md named, between a child's exit and its own trailing
delivery through the caretaker.

Decided (§106, 2026-08-22): take it. Checking `SINK_BIT`'s existing contract
(`crates/grant_plan/src/spawnproto.rs`) found the fork's own write-up had overpriced it: the child's
output slot is already opaque to the program ("the shell delegates an endpoint and init puts it
where the result endpoint would have gone, so the child writes to a pipe or a file sink without
knowing which"), so no program's manifest changes, only shell-and-init default-routing logic.

Built, the same day. An unredirected tail stage whose program declares
`InputSpec::Required { writes_while_reading: true }` (`doc` is the only one today) is delegated to
`terminal_sink_caretaker` by default; `grant_plan::check_chain` no longer refuses that shape, only
the redirected one (`doc <page> > out.txt`, still DECISIONS §55's shell); and the shell's completion
signal for a narrowed child is DECISIONS §26's kernel exit-delivery, on a fresh endpoint it mints and
delegates, exactly the reuse notes/tail-output-narrowing.md found. Verified at the real prompt, both
architectures (`script/swish-check`, `NIFE_SHOW_TRANSCRIPT=1`):

```text
$ doc gate.txt | wc
  1 4 26
$ doc gate.txt
  hello world hello world
```

The first line is the control this milestone has carried since phase 1 (the barrier `wc` makes the
render countable); the second is the line nobody could type before today, rendering with no `| wc`
in front of it. The caretaker-hop display race notes/tail-output-narrowing.md named is carried as a
documented, accepted interim rather than fixed here; see this block's `BUGS` and milestone 151.
This is also the milestone's own headline demonstration, decided 2026-08-26 to be the whole of
phase 3 rather than half of it (see the Status block above).

2026-08-22, later the same day: ranking divides by length. `crates/manual/src/index.rs`'s own
`BUGS` had named this since phase 2 landed: "ranking is occurrence count and nothing else, so a
long page that mentions a word in passing can outrank a short page about it," priced at "one
division" needing "the page's length, which the layout does not store." That price was accurate.
A page record grew a `tokens` field (four of the six bytes `PAGE_REC` already held spare, no
format growth), `index::VERSION` moved to 2, and `manual::index::Ranked::offer` now ranks by
`count / tokens` (fixed-point, one division) rather than raw `count`. `Found::count` still reports
the raw occurrence count in the printed answer; only the merge order changed, and it is pure crate
logic with no spawn-protocol or syscall-surface change, unlike the two items above it. See
notes/manual.md's `BUGS` and "Where this goes next" for detail and `crates/manual`'s own new test,
`ranking_divides_by_page_length`. This does not move phase 2 status; phase 2 was already BUILT and
this is a quality improvement inside it, named as future work at the time and now closed.

**The status did not move to BUILT that day.** This closed phase 3's caretaker-narrowing increment,
not phase 3 as this file then defined it: a graphical viewer, waiting on the display ladder
(milestone 29's font rendering, milestone 33's compositor), was still named as the phase's other
half. That naming is struck 2026-08-26, traced to its origin and found unrequested; see the
Status block above. Phase 3 is therefore complete as of the caretaker-narrowing increment, and so
is this milestone.

In brief. Markdown authored, **rendered** for display rather than shown raw, searchable locally, and installed by the package that owns it. Reuse `pulldown-cmark` for parsing (CommonMark is a fiddly spec worth taking from someone else) and write the ANSI renderer against `line_editor`'s contract, because `termimad`/`mdcat` sit on `crossterm` and assume a POSIX terminal we do not have. Phase 1 is a terminal viewer and pager, phase 2 a host-built inverted index shipped as a per-package shard, phase 3 a page rendered straight at the prompt. Two constraints found while scoping: `readdir` refuses and the §27 contract has no such verb, so nothing can walk a tree for documents, and font rendering is still milestone 29's remaining increment, so the terminal comes first, and stays: no graphical viewer was ever asked for (struck 2026-08-26, see the Status block)

Why it matters. the OS explains itself, on itself. The project's whole argument is already markdown (DECISIONS, thirty-plus notes, this roadmap), so a capability-confined viewer serving them is a better milestone-23 demonstration than another synthetic test and costs the documentation nothing. The missing `readdir` turns out to be a feature: enumeration is authority, so indexing at package-build time is both the way around the gap and the more honest shape, which is the same answer `apropos` reached for a different reason. And `doc notes/ipc-naming.md` granting exactly one readable file is milestone 31's designation-is-authorization made into something a person uses

calef's direction, 2026-07-30. Markdown as the authored format, rendered for display rather than
shown raw, searchable on the local machine, and installed *by the package that owns it*, so a
component brings its documentation with it.

Why this belongs on a demonstrator's roadmap rather than being a nicety. The project's own
argument is written in markdown: `design/decisions/`, thirty-plus notes, this roadmap. A nife that
serves its own design notes, on itself, through a capability-confined viewer, is a better
demonstration of milestone 23's component story than another synthetic test, and it costs the
documentation nothing because it already exists.

What this section used to say next, struck 2026-08-26: that this would also be "the first
*application* on the display ladder that anybody would actually use," an unattributed sentence never
in calef's own quoted direction above it. It is the traced origin of "phase 3 = graphical viewer,"
which calef confirmed he never asked for once the origin was found; see the Status block. Kept here
rather than deleted, because the sentence that quietly became a requirement is worth a reader seeing
exactly where it entered.

## Two constraints found while scoping, both real

1. There is no directory iteration. `readdir` refuses in the std PAL and the §27 file contract has
   no such verb, so nothing can walk a tree looking for documents. Adding one is a decision, not a
   detail, and the capability model argues against it anyway: enumeration is authority. A viewer
   that can list a directory can discover what it was not given. So the design below indexes at
   *package build time* and ships the index, which sidesteps the missing verb and is the more honest
   shape. Unix reached the same answer for a different reason: `apropos` reads a prebuilt `mandb`
   because scanning was slow.
2. ~~There is no font rendering yet.~~ **There is now** (milestone 29, 2026-07-30): a bitmap
   font, a VT engine, and a display terminal that is a compositor client. ~~A *graphical*
   documentation browser is therefore unblocked in principle~~ **struck 2026-08-26**: nobody asked
   for one, and this milestone's own headline demonstration renders at the prompt instead. See the
   Status block.

## Reuse: take the parser, write the renderer

CommonMark is a fiddly specification with a large conformance suite, and parsing it is exactly the
kind of work worth taking from someone else. Rendering to *our* terminal contract is ours and small.
That split is the reuse judgment, and it is the same one milestone 32 made about RedoxFS.

| Piece | Option | Judgment |
|---|---|---|
| Parse | **`pulldown-cmark`** (pure Rust, CommonMark, event-stream API, few dependencies) | **Take it.** The event stream is the right shape for a renderer that emits ANSI. Milestone 27's `std` is what makes this buildable at all. |
| Parse | `comrak` (GFM: tables, strikethrough, footnotes) | Consider later if GFM tables matter; more dependencies. |
| Render | `termimad`, `mdcat` | **Do not take.** Both sit on `crossterm`, which assumes a POSIX terminal (termios, ioctl). Porting that is more work than emitting ANSI against `line_editor`'s contract, which we own and already speak (§21). |
| Search | `tantivy` | **Too heavy.** It assumes a filesystem and mmap. |
| Search | A host-built inverted index shipped in the package | **Take this shape.** Built by `xtask` where there are no constraints, merged by the viewer across installed packages. |
| UI | `ratatui` | Possible for a pager later; needs a backend against our terminal contract first. |

## Shape

- A doc bundle is part of a package: rendered-source markdown plus a small index shard, installed
  into a documentation store when the component is installed. This is where milestone 39's packaging
  observation pays: manifest, hash, version, and now a doc bundle.
- ~~The viewer holds a directory capability to the doc store and nothing else.~~ **It should
  not, and does not** (phase 1's finding, notes/manual.md): `doc`'s manifest is byte-identical to
  `wc`'s, its cspace holds two endpoints, and the shell is what resolves a page name. A viewer that
  could open the page it renders could open any page. Phase 2 kept that true by making search a
  builtin rather than a program, for the same reason `ls` is one.
- The index is a merge of shards, one per installed package, so installing a component makes its
  documentation searchable without a reindex pass and without any component being able to see
  another's files.
- `apropos <word>` and `doc <page>`, shell verbs. (Written as `doc search` / `doc view` when
  this was scoped; phase 2 split them because `doc` is a program and a builtin sharing its first
  word would make the parse ambiguous.) Milestone 31's grant expression makes this a demonstration
  rather than a convenience: `doc notes/ipc-naming.md` passes exactly one readable file capability,
  a viewer invoked with no argument can read nothing, and `apropos` in front of it passes nothing at
  all.

## Phasing

- Phase 1, the terminal viewer. `pulldown-cmark` to an ANSI renderer over `line_editor`'s contract:
  headings, emphasis, lists, block quotes, code blocks, and a pager. Works on the serial console
  today and inherits the display terminal for free when 29's glyph work lands. Host-tested in
  milliseconds like every other pure-logic piece: markdown in, styled bytes out.
- Phase 2, search. BUILT (2026-08-16). The host-built index, the shard merge, and the search
  verb, which is **`apropos <word>`** rather than the `doc search` proposed above: `doc` is a
  program and a builtin sharing its first word would make the parse ambiguous, and `man`/`apropos`
  is the split this design already borrowed everywhere else. The merge is
  `manual::index::Ranked`, a fixed sixteen-result table with no allocator, so a shell with one
  stack page can hold it. **Ranking by document length, not just raw occurrence count, closed
  2026-08-22**: `manual::index::Page::tokens` and the `score` function above `Ranked::offer`, see
  this page's status block.
- Phase 3. BUILT (DECISIONS §106, 2026-08-22): `doc <page>` renders straight at the prompt,
  with no `| wc` needed to give it a reader that is not the shell. A graphical viewer was named here
  as the phase's other half through every revision of this file until 2026-08-26, when it was traced
  to its origin (an unattributed sentence, not calef's own quoted direction) and struck once calef
  confirmed he never asked for one. See the Status block.

Prior art worth reading: `man` plus `apropos` plus `mandb` for the split between format, index and
pager, which is the architecture this proposes minus the troff. Dash/Zeal *docsets* (a bundle with its
own index) for the packaging shape. `cargo doc`'s HTML output as the road not taken, since HTML would
need a browser engine, which is a mountain with no thesis behind it.

Sequencing. Phase 1 wants milestone 31 phase 2 finished (per-file grants make `doc <file>` the
demonstration it should be) and nothing else; it can precede the packaging work and be wired into it
later. Effort: 1 lane estimated per phase, three phases, landed separately.

## BUGS

- **The caretaker-hop display race is a known, accepted interim, not fixed by §106.** With the
  `terminal_sink_caretaker` narrowing built, a page's last line and the shell's next `$ ` prompt can
  interleave under contention: kernel exit-delivery (DECISIONS §26) tells the shell a child is dead,
  but the caretaker's own trailing `CALL` to `line_editor` is a second, independent call with no
  ordering primitive against the shell's next prompt. A display glitch, not a confinement or
  correctness failure (no capability changes hands, no byte reaches the wrong reader). Not observed
  in either `script/swish-check` leg's transcript (`doc gate.txt`'s render is one short message, so
  the window is narrow), but the argument for it is structural rather than about this one case; see
  notes/tail-output-narrowing.md's own BUGS, which is honest that the race is named and not measured.
  Tracked at milestone 151 (notification objects), DECISIONS §101's kernel build, which lets the
  shell `WAIT` on "the caretaker's queue for this client has drained" instead of racing it.
- **A screen-narrowed child does not appear in a concurrent `ps`/`pgrep`.** Its DECISIONS §26 fault
  target is a fresh endpoint the shell mints for this purpose, not init's `deaths` domain channel
  (the shell needs to `RECV` its own child's exit directly, not race `job_undertaker` for the same
  message), and domain membership is exactly having `deaths` as that target. Its memory still
  returns to init's job pool when the shell reaps it. Recorded rather than fixed: nothing asked for
  a rendering `doc` invocation to be surveyable, and the alternative (giving the shell a second,
  narrower view into `deaths` itself) is a bigger change than this increment needed.

## Follow-on

- **Milestone 151.** The caretaker-hop display race: with the `terminal_sink_caretaker` narrowing
  built, a page's last line and the shell's next `$ ` prompt can interleave, because the caretaker's
  trailing `CALL` to `line_editor` has no ordering primitive against the shell's next prompt. The
  block names 151 (notification objects) as what lets the shell `WAIT` on the caretaker's queue
  draining instead of racing it.
- **Recorded.** `design/roadmap/40-documentation-service.md`'s own BUGS: a screen-narrowed child
  does not appear in a concurrent `ps` or `pgrep`, because its fault target is a fresh endpoint the
  shell mints rather than init's `deaths` domain channel, and domain membership is exactly having
  `deaths` as that target. Its memory still returns to init's job pool when the shell reaps it.
- **Refused.** A graphical viewer. Traced to its origin 2026-08-26 and found to be the scoping
  lane's own unattributed addition rather than calef's direction; asked directly, he said "I don't
  need a graphical viewer." Struck wherever it was named, and the sentence that quietly became a
  requirement is kept in the block so a reader can see where it entered.
- **Refused.** Adding a `readdir` verb so a viewer could walk a tree for documents. Enumeration is
  authority, so a viewer that can list a directory can discover what it was not given. The design
  indexes at package build time and ships the index instead, which sidesteps the missing verb and is
  the shape Unix's prebuilt `mandb` reached for a different reason.
- **Refused.** `comrak` for GFM tables, strikethrough and footnotes. It carries more dependencies
  than the job needs and nothing in the corpus has wanted a GFM table, so the row stays "consider
  later if tables matter" rather than work anyone owes.
- **Refused.** `ratatui` for the pager. It needs a backend written against this tree's terminal
  contract before it can render anything at all, so taking it would buy a widget library and leave
  the actual work undone.
- **Milestone 334.** The other two thirds of
  §106's spawn-protocol narrowing: a bit telling a tail stage
  it ends at a real screen (color, the honest `isatty` replacement), and a way to grant one line of
  input without granting the keyboard (the pager). Both widen a protocol two programs agree on, so
  both are an architect's call; `notes/manual.md`'s "where this goes next" is the only record either
  has.

## Index row

the OS explains itself, on itself. Phases 1 and 2 built: `crates/manual` (a streaming renderer and
an index layout a reader with one 4 KiB page can binary-search), `doc` at the prompt, and `apropos` as a builtin because search is enumeration and a searching *program* would need the
whole store. 2026-08-18 fixed a renderer bug the corpus test structurally could not see (a fence
inside a block quote never closed, so one quoted transcript misrendered every line after it),
fixed an index defect where the builder and the reader disagreed about what a word is (`apropos
fs_proto` could not match a term the builder never wrote), corrected three `BUGS` entries that
described a system which no longer existed, and pointed the same index at the repository as `script/apropos`, which is milestone 117's finding rather than a nicety: three strangers reached
neither `notes/capabilities.md` nor any `design/decisions/` file. The `terminal_sink_caretaker`
narrowing is decided and built (§106, 2026-08-22): `doc <page>` renders at the prompt. An
unredirected tail stage's output now defaults to the same adapter `2>` already uses; `grant_plan::check_chain` no longer refuses that shape, only the redirected one; and the shell's
completion signal is DECISIONS §26's kernel exit-delivery on a fresh endpoint. Verified at the
real prompt, both architectures: `doc gate.txt` prints `hello world hello world` with no `| wc` in
front of it. Milestone 151 tracks the one known cost (the caretaker-hop display race) as a
carried, accepted interim rather than a blocker. A graphical viewer, named as phase 3's other
half through every revision, is struck 2026-08-26: traced to its origin, an unattributed
sentence, and confirmed unrequested.
