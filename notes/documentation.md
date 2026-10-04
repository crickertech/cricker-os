# The manual: documentation as a system service

*(Milestone 40. Markdown authored, rendered for display rather than shown raw, searchable, and
installed by the package that owns it. The pure logic is `crates/documentation`; the program is
`components/src/mdr.rs`; the store is built by `cargo xtask manual`.)*

The project's own argument is written in markdown: 328 files, three megabytes, `design/decisions/`
and a hundred notes. A nife that serves them, on itself, through a viewer that can name
nothing is a better demonstration of the component story than another synthetic test, and it costs
the documentation nothing because it already exists.

Three words carry design weight here, and each turned out to be a capability question rather than a
feature question.

## Rendered

`crates/documentation` is a streaming renderer: bytes in, styled terminal bytes out, no allocator, no
document held anywhere. That shape is not an optimisation, it is what the program's capabilities
already are. `doc` receives its input as `byte_sink_protocol` messages of sixteen bytes each and writes its
output the same way, so a renderer that needed the whole document would need somewhere to put it,
and somewhere to put it is a memory grant the program can otherwise do without. The test
`framing_does_not_matter` pins it: one byte at a time and the whole document at once produce
identical output.

It handles ATX headings, wrapped paragraphs, fenced code, block quotes, nested lists, tables with
computed column widths, thematic breaks, and the inline set (`**strong**`, `*emphasis*`,
`` `code` ``, `~~strike~~`, links and images).

A link and an image both show their destination, because a terminal cannot follow one or draw
the other, so the path is the only thing a reader can act on. An image used to render as
`[image: alt text]` with the path dropped, which named a picture the reader had no way to find.
That was invisible until 2026-09-02, when notes/project-metrics.md became the first page under
`notes/` to carry an image and `every_character_survives` failed on the missing bytes. The test is a
subsequence check over the source, so a dropped destination is exactly what it exists to catch, and
the first page that could trip it did.

### The roadmap said take `pulldown-cmark`. Two facts overrule it.

It is not `no_std`. Version 0.13's `lib.rs` carries no `#![no_std]` and `parse.rs` uses
`std::collections::HashMap`. Taking it means either a permanent fork of somebody else's parser or a
std program.

And a std program on this system cannot be this program. The nife PAL's `Stdin::read` returns
`Ok(0)` unconditionally (`patches/std-nife/overlay/std/src/sys/stdio/nife.rs`) and there is no
argv. A std viewer could neither read a keypress to page nor be told which page to show. The
roadmap's premise that "milestone 27's std is what makes this buildable at all" is true of the
parser and false of the program.

What the roadmap could not weigh is the third fact: the corpus is closed and in-tree. A renderer
for this repository's own markdown does not need CommonMark conformance; it needs the constructs
these files use, and unlike conformance that is checkable directly.
`every_character_survives` does exactly that: every letter and digit of every note, decision and
roadmap page reaches the rendered output, in order. It found three real defects while it was being
written (an escaped pipe read as a column boundary, a table tail dropped when the buffer filled, a
doubled blank line), which is three more than a conformance suite over invented documents would have
found here.

DECISIONS §46 rule 1 then settles it: this is on the verification path, so we write it. The crate is
`no_std`, zero-dependency, allocation-free on the guest path, and reachable by Kani.

### Two rendering decisions worth knowing

`_` is never emphasis. CommonMark reads `__rust_alloc` as an opened strong span. This repository
writes `snake_case` identifiers in running prose constantly (`filesystem_protocol`, `line_editor`, `c_seam`),
so honouring the spec here would misrender far more than it would style. Only `*` and `` open
emphasis, and a closer must not be preceded by a space.

A code span is consumed before anything else looks at the line. There are 11,281 of them in the
corpus, many full of the exact characters the other rules hunt for. If that ordering were wrong,
`*ptr` inside backticks would open emphasis and eat the rest of the paragraph.

### The bug the corpus test could not see, and the honest filter that kept it findable

A fenced code block opened inside a block quote never closed, from phase 1 until 2026-08-18. The
closing test ran against the raw line, so this:

> A transcript quoted from somewhere else, with its own fence inside the quote:
>
> ```text
> $ doc glob.md | wc
>   1 4 26
> ```
>
> and prose after it, which used to render as code.

left the renderer in code mode to the end of the document. One quoted transcript misrendered every
line after it.

`every_character_survives` could not find it, and the reason is worth keeping: verbatim output
loses no characters, so a page rendered entirely as code still passes a subsequence check. What the
test *did* see was the opening fence's info string going missing, because its filter excluded a
fence by looking for backticks after `trim_start` and a quoted fence does not start with one. That
is the renderer being right about one line inside a page it was then ruining.

The filter was left wrong on purpose, with a `BUGS` comment saying why: widening it would have
silenced one false failure by hiding three hundred misrendered lines, and nobody had answered
whether the renderer kept its quote state across a nested fence. It did not. So the answer came
first and the filter second, which is the order that entry existed to enforce.

And the corpus test still cannot guard it, which was measured rather than assumed. Reverting the
fix leaves this page ruined from the block above onward, and `every_character_survives` passes:
verbatim output loses no characters, and `Renderer::has_unclosed_fence` (added here, and the
strongest thing the corpus check can assert) misses it too, because a bare closing fence three
sections later matches the stuck one and lets the renderer out. A unit test is the guard. The lesson
generalises past this bug: a subsequence check proves nothing was dropped and nothing about what
was ruined, so a renderer wants both kinds of test and this one had only the first.

## Installed

A doc bundle is a package's pages plus its index shard, installed as a unit. `doc/<bundle>/` in
the filesystem image, with `doc/bundles` listing the names, built by `cargo xtask manual` from the
`DOC_BUNDLES` table in `xtask/src/manual.rs` and imported into the RedoxFS image by `mkredoxfs`.

The table names paths that already exist rather than copying notes into crate directories, and that
is deliberate: a second copy of a note is a copy that can drift, and in-tree documentation earns
its keep by there being one. A bundle that lists a page which has moved fails the build.

### What a doc-holding capability designates, which is nothing

The roadmap proposed that "the viewer holds a directory capability to the doc store". It should not,
and does not.

`doc`'s manifest is byte-identical to `wc`'s: `InputSpec::Required`, `OutputSpec::Bytes`, and
`Forbidden` for argument, memory, file and directory. Its capability table holds two endpoints. `doc glob.md`
is the shell resolving that name against the directory capability *it* holds and streaming the
bytes in; nothing in the program names a file, a directory or the filesystem, and there is no
message it can send to find out what it is reading.

That matters more here than it did for `wc`, because a documentation viewer is precisely the program
a reader would expect to go and fetch things. A `doc` that opened the page it renders would be a
`doc` that could open any page. `doc glob.md`, `doc < glob.md` and `something | doc` are one
behaviour with three sources, and the program cannot tell them apart.

So there is no ambient authority to arrive by accident, because there is no authority at all. The
concentration is in the shell, where it already was.

## Searchable

There is no directory iteration in this system, and that is a feature rather than a gap.
`readdir` refuses in the std PAL and the §27 file contract has no such verb, and the capability
model argues against adding one: *enumeration is authority*, and a viewer that can list a directory
can discover what it was not given. So "what pages exist" is not discoverable at runtime. It is
computed on the host at image time and shipped, which is what Unix's `mandb` does for a different
reason (scanning was slow).

### The writer and the reader must agree on what a word is, and did not

`documentation::index::normalize` folds a query by dropping every byte that is not a letter or a digit, so
a reader who types `line_editor` looks up `lineeditor`. `documentation::index::tokens` split the *text* on
that same byte, so the builder only ever wrote `line` and `editor`. The term the query asks for
was one no page could ever have. In a repository whose prose is full of `snake_case` identifiers,
that is most of what anybody would search for: `apropos fs_proto` and `apropos grant_plan` both
answered "nothing says that" while dozens of pages said exactly that.

The test that should have caught it asserted the property in its own first comment, *"the builder's
tokeniser and the reader's query normaliser must agree, byte for byte, on what a term is"*, and
then checked a word with no underscore in it. Same shape as the fence: a claim in prose, a weaker
thing checked.

The fix is in the writer rather than the reader, and that is the choice worth recording.
`line_editor` now yields three terms, `line`, `editor` and `lineeditor`, so `apropos editor`
keeps working and `apropos line_editor` starts. Narrowing `normalize` to stop at the underscore
would have been smaller and would have made `apropos line_editor` silently search for `line`, which
is a worse answer than no answer.

Only the underscore joins. It is unambiguous in this tree and the hyphen is not: joining across
`-` would manufacture a term out of every hyphenated phrase in the corpus, and `notes/glob-grant.md`
is a filename rather than a word. The renderer narrowed emphasis on exactly this reasoning and
records it in its own `BUGS`.

### The layout is designed for a reader that holds one page

A client of the file contract shares exactly one 4 KiB frame with the FS server, and a shell that
had to buffer a whole index would need a memory grant to search. So every section of the index
starts on a page boundary and every record divides 4096 evenly, which together mean a reader with
one page in hand never sees half a record. A lookup is then a binary search over *pages*: each
probe reads one page and compares the term it starts with, and the last page is searched in memory
for free.

```text
  page 0            header
  page_off          page records, 128 bytes each, 32 per page
  term_off          term records, 32 bytes each, 128 per page, sorted by term
  post_off          postings, 4 bytes each, 1024 per page
```

### The guest searches with `apropos`, and it is a builtin

Phase 2's other half, and the decision in it is *where the search runs* rather than how.

A builtin, not a program, which is exactly the argument `ls` already carries in this shell: a
listing program would have to hold the power to read everything it lists. Search is an enumeration,
so a searching program would have to be handed a capability to the whole documentation store in
order to read every shard in it. That is a new principal holding more than the answer needs, for a
command that moves no authority whatsoever. The shell already holds enumeration over what it can
see, so `apropos` is the shell reading a file it could already read.

What comes back is names, never capabilities. A result is a store location a person can type:

```text
$ apropos capability
    32  doc/swish/pipes.md            Pipes and redirection: `>`, `<` and `|` are one
    11  doc/kernel/ipc-naming.md      Who does IPC name?
$ wc doc/kernel/ipc-naming.md
  163 1556 9503
```

(Those three numbers move whenever that note is edited. What does not move is that the name the
search printed resolved, which is the claim.)

The second line is where a capability moves, and it moves because a person typed a name. So search
cannot widen what its caller could already reach, and `doc notes/ipc-naming.md` granting exactly one
readable file survives having a search in front of it. A search *program* would have moved the
authority one line earlier and silently.

The split follows the tree's usual one. The reading is `documentation::index::search`, and it is the
single point at which the writer and the reader are proved to agree: `cargo xtask manual capability`
on the host and `apropos capability` at the prompt call that same function, over the same bytes,
through the same one-page-at-a-time `Pages`. The rendering is `swish::write_apropos`, host-tested
with the rest of what the prompt says. What is left in `components/src/swish.rs` is four filesystem
requests and a 4 KiB page buffer.

### And the same index, pointed at the repository

`script/apropos <word>` is the guest's builtin with a checkout underneath it instead of a
filesystem image. It is milestone 117 (the stranger test)'s finding rather than a convenience. Three stranger runs
have measured what a newcomer cannot reach by following this tree while doing ordinary work, and it
is a list rather than an impression: `notes/net.md`, `notes/capabilities.md`, any
`design/decisions/` file, and `crates/abi/src/lib.rs`, which is four syscall numbers and the whole
design on one screen. None of them is hidden. Nothing a person would type led to them.

```text
$ script/apropos capability
512 pages, 5049711 bytes of documentation in this repository

searching for: capability

     46  notes/capabilities.md                             Capabilities, and why the kernel has no `open()`
     43  notes/README.md                                   Concept notes
     37  design/roadmap/47-navigation-and-naming.md        47. Navigation and naming: `cd`, `pwd`, `ls`, `m
     33  notes/std.md                                      Rust `std` on the native ABI
     32  notes/pipes.md                                    Pipes and redirection: `>`, `<` and `|` are one

  16 of 302 pages, strongest first
```

Every number in that block moves whenever anything in the tree is edited, this page included, and
that is the same property the store's own table has: the documentation is the data. What does
not move is which page came first.

Three things about it are deliberate.

It is the same code, `documentation::index::build` and `documentation::index::search`, one shard per part of
the tree and the same merge across shards the shell does with one 4 KiB page. Not a second
implementation, so a defect in the layout shows up in both places and a fix lands in both. What
differs is what a result *names*: a guest result names `doc/<bundle>/<page>`, because that is what a
shell there can designate, and this one names a path in this repository, because that is what a
person with a checkout opens. Both come out of the same `Found`; the store location and the origin
are two fields it already carried.

Crate and program module headers are pages. That is the half that makes `crates/abi/src/lib.rs`
findable at all, and no markdown page was ever going to do it: the document a reader wants about the
ABI *is* that file's header. A `//!` block is markdown already, so it indexes with no conversion and
no copy, and the result names the source file, which is the thing to open. A header shorter than a
paragraph is skipped, because indexing it would put noise in front of the pages that answer.

**There is no cache, and a run is about a second over five megabytes. A cache that can be stale is
worse here than a second of work, for the same reason `script/catch-up` and `script/names` are
derived views: a maintained one rots and nothing says so.

### The store's own layout is a thing two programs agree on

`doc/bundles` lists what is installed, one name per line; `doc/<bundle>/index` is a shard;
`doc/<bundle>/<page>.md` are the pages. Those three names are `documentation::index::STORE_DIR`,
`MANIFEST` and `SHARD`, in the crate both sides depend on, because the host writes them and the
guest opens them (AGENTS.md rule 7). The manifest is a file rather than a directory listing,
which is this whole milestone in one constant.

### What it costs, measured

`cargo xtask manual` over the current bundles:

| bundle | pages | terms | postings | markdown | index | probes |
|---|---|---|---|---|---|---|
| `manual` | 1 | 1222 | 1222 | 34011 | 57344 | 5 |
| `swish` | 2 | 1820 | 2143 | 91736 | 81920 | 5 |
| `kernel` | 3 | 2461 | 3369 | 101022 | 106496 | 6 |
| `glob` | 1 | 882 | 882 | 20019 | 40960 | 4 |

**246,788 bytes of markdown produce 286,720 bytes of index**, which is 1.16x, and that is the number
worth arguing with rather than the pleasant ones. (It was 1.56x when phase 1 measured it and 1.24x
in the middle, and the improvement is not an optimisation: the notes it indexes grew, and page
alignment's fixed floor is a smaller share of a bigger bundle. It went the other way on 2026-08-18,
from 1.14x, and that one *is* a cost: underscore-joined terms are a third term for every
`snake_case` identifier in the prose, which is what makes those identifiers findable.) Two things pay for it. A term record stores its
term inline in 24 bytes so a probe is one page read rather than two, which is most of the bulk.
And page alignment puts a four-page floor (16 KiB) under every bundle however small, so a bundle of
one short page still costs 16 KiB to index. Ranking by length (2026-08-22) cost four more bytes
per page record, spent out of the six the layout already held spare: with four pages in the
shipped store that is sixteen bytes, immeasurable against the four-page floor, which is why the
ratio above barely moved.

The `manual` row indexes this page, so editing it moves its own numbers. Rerun
`cargo xtask manual` for current ones; the ratio is what is stable.

The number that justifies the layout is the last column: a lookup is at most five page reads,
20 KiB of IO, with no allocation and a 4 KiB working set.

## EXAMPLES

Build the store and search it from the host, with the same reader the guest runs:

```text
$ cargo xtask manual capability
documentation store: target/redoxfs-tree/doc

  bundle     pages   terms postings  markdown    index probes
  manual         1    1222     1222     34011    57344      5
  swish          2    1820     2143     91736    81920      5
  kernel         3    2461     3369    101022   106496      6
  glob           1     882      882     20019    40960      4

  246788 bytes of markdown, 286720 bytes of index

search: capability
    46  doc/kernel/capabilities.md    Capabilities, and why the kernel has no `open()`  notes/capabilities.md
    11  doc/kernel/ipc-naming.md      Who does IPC name?                              notes/ipc-naming.md
    18  doc/manual/manual.md          The manual: documentation as a system service   notes/documentation.md
     3  doc/swish/line-discipline.md  The line discipline as a userspace component    notes/line-discipline.md
     8  doc/glob/glob.md              The glob matcher                                notes/glob.md
    32  doc/swish/pipes.md            Pipes and redirection: `>`, `<` and `|` are one   notes/pipes.md
```

The host prints a fourth column the prompt does not: the page's path in the source tree, which is
provenance rather than something to open. The store location beside it is computed by the searcher
from the shard it opened, so no byte of the index carries it and the two cannot disagree.

**The order is no longer sorted by that first column**, and that is the ranking increment showing
itself rather than a bug in the example: `notes/pipes.md` has the second-highest raw count (32) but
the lowest density, because it is the longest page in the `swish` bundle, and `notes/ipc-naming.md`
outranks it with a third of the mentions because it is short. `Found::count` still prints the raw
number honestly; only the order it appears in changed.

Search the same store from the prompt, where the answer is what a person acts on:

```text
$ apropos capability
    46  doc/kernel/capabilities.md    Capabilities, and why the kernel has no `open()`
    11  doc/kernel/ipc-naming.md      Who does IPC name?
    18  doc/manual/manual.md          The manual: documentation as a system service
     3  doc/swish/line-discipline.md  The line discipline as a userspace component
     8  doc/glob/glob.md              The glob matcher
    32  doc/swish/pipes.md            Pipes and redirection: `>`, `<` and `|` are one
$ caps wc doc/kernel/ipc-naming.md
  wc would grant the new process, and nothing else:
    cap 0  endpoint  result   report its answer back
    output   this shell's result endpoint (it reads the bytes and prints them)
    input    ipc-naming.md  (this shell reads it and streams it in; the program
             holds an endpoint, not a file)
    arg    (none)
  reading the command is reading its whole authority.
```

Those two lines together are the milestone: the first names pages and grants nothing, and the second
grants exactly the one page a person named out of what the first said. `apropos` itself has no
`caps` preview to print, because there is nothing to preview.

Render a page at the prompt, with no pipe or redirection needed to make it run
(DECISIONS §106 (take the `terminal_sink_caretaker` narrowing), 2026-08-22). `gate.txt` is `script/swish-check`'s own fixture, two lines of
`hello world` written earlier in that script; the renderer re-flows them to one paragraph:

```text
$ doc gate.txt
  hello world hello world
```

And prove the viewer is an ordinary pipeline stage rather than something that reached for a file:

```text
$ echo # The terminal contract | doc
THE TERMINAL CONTRACT
$ doc gate.md | wc
1 3 22
$ doc
doc: reads an input stream: name a file, redirect with '<', or pipe into it
```

## BUGS

- **`doc <page>` on its own used to be refused; DECISIONS §106 (2026-08-22) closed it and it now
  renders.** It deadlocked when phase 1 shipped, was a refusal that named the fix from milestone
  50's chain check onward, and this entry described that refusal until this line. The mechanism
  is exactly what it named as the fix: an unredirected tail stage's primary output now defaults to
  `terminal_sink_caretaker`, the terminal's own sink adapter, the same one a declared second stream
  already reached by default under DECISIONS §67. `grant_plan::check_chain` still refuses a
  redirected shape that has nowhere for the shell to wait but itself (`doc <page> > out.txt`); it
  is only the unredirected, screen-bound case this decision narrows:

  ```text
  $ doc gate.txt
    hello world hello world
  ```

  The constraint underneath is the kernel's rather than the shell's, and it is worth reading before
  reaching for a scheduling fix. A process has one wait point: `SEND` blocks until a receiver
  takes the message, `RECEIVE` blocks until one arrives, and there is no select and no timed wait. So a
  shell feeding a chain cannot be receiving from it, and no interleaving schedule fixes it:
  alternating one send with one receive deadlocks whenever the stage reads twice before it writes,
  and the other way round deadlocks whenever it writes twice before it reads. The shell cannot know
  which, because the whole point of the sink contract is that neither end knows anything about the
  other. What changed is not the schedule; it is that the tail's reader is no longer the shell at
  all, so the shell was never the thing the chain needed to wait behind.
- **The caretaker-hop display race is a known, accepted interim, not a bug this decision missed.**
  A page's last line and the shell's next `$ ` prompt can interleave under contention: kernel
  exit-delivery (DECISIONS §26) tells the shell a child is dead-until-reaped, but
  `terminal_sink_caretaker`'s own trailing `CALL` to `line_editor` is a second, independent call
  with no ordering primitive against the shell's next prompt. This is the cost this section used to
  flag before the decision was made ("a stage that outlives its command can paint a prompt nobody
  typed"), named precisely now that the mechanism is built: a display glitch, not a confinement or
  correctness failure, because no capability changes hands and no byte reaches the wrong reader.
  Tracked at milestone 151 (notification objects), DECISIONS §101's kernel build, which lets the
  shell `WAIT` on "the caretaker's queue for this client has drained" instead of racing it. See
  notes/tail-output-narrowing.md.
- **A screen-narrowed child is invisible to a concurrent `ps`/`pgrep` for its short life.** Its
  DECISIONS §26 fault target is a fresh endpoint this shell minted, not the progenitor's `deaths` domain
  channel, because the shell needs to `RECEIVE` its own child's exit directly rather than race
  `job_undertaker` for the same message. Its memory still returns to the progenitor's job pool when this
  shell reaps it (§26: a corpse's region returns to its *builder*, not its supervisor), so nothing
  leaks; it simply does not appear in a domain survey while it runs. Given `doc`'s render is a
  handful of milliseconds, this is unlikely to matter in practice and is recorded here rather than
  fixed because nothing asked for it.
- **`doc <page> | wc` and `doc <page> > out.txt` deliver the file now.** Both
  answered `0 0 0` when phase 1 measured them, because a pipeline's head was wired off the `Line`,
  which carries no `<`, so the planned input operand was dropped and the stage counted an empty
  stream. That was fixed in `components/src/swish.rs` (the head's input comes off the plan now) and the
  fix is pinned at the real prompt on both architectures:

  ```text
  $ wc gate.txt
    2 4 24
  $ doc gate.txt | wc
    1 4 26
  ```

  Two source lines are one paragraph re-flowed to one output line, and the two extra bytes are the
  body indent, so the counts are what separate a rendered page from silence. `MAX_TEXT_CHUNKS = 32`
  was the other half of this entry and is also gone: the shell drains `MAX_OUTPUT_CHUNKS = 4096`
  messages, which is 64 KiB rather than 512 bytes.
- **No pager, and the reason is authority rather than effort.** Paging needs a keypress; a keypress
  needs `line_editor::proto::OPERATION_READLINE`; and that opcode rides on the terminal endpoint whose read
  side *is* the keyboard. The spawn protocol has no way to hand a child the right to read one line
  without handing it the terminal, which is the exact thing `terminal_sink_caretaker` exists to
  prevent. So a long page scrolls off. The fix is a decision about the spawn protocol, and it is the
  most interesting thing this milestone found.
- **`doc` emits plain text even at a terminal.** The renderer can colour, and the shell has no way
  to tell a stage "you end at the terminal", for the same reason the sink contract is a good
  contract: a writer cannot tell what is underneath it. Unix answers this with `isatty`, which is a
  sniff; the honest answer here is a wiring bit the spawn protocol does not carry yet.
- **A search keeps sixteen results and counts the rest**, saying "16 of 43 pages, strongest first"
  when it dropped any. Sixteen is the reader's number rather than the index's: a search answer is
  read at a prompt before deciding what to open, and one of this system's two terminals is sixteen
  rows tall. A term nearly every page mentions therefore answers with the sixteen that mention it
  most, which for `capability` is a fair answer and for `the` would not be.
- **A hyphenated name is not one term, and a path is not searchable.** Only the underscore joins,
  so `apropos line-discipline` searches `line` and then `discipline` separately (as two queries;
  this verb takes one word), and `apropos notes/glob.md` folds to `notesglobmd`, which no page
  says. A search is for words, and the location a result prints is what you hand to `doc`.
- ~~Ranking is occurrence count and nothing else.~~ It divides by document length now
  (2026-08-22): the page record grew a `tokens` field, spent out of the six bytes it already held
  spare, and `documentation::index::Ranked::offer` ranks by `count / tokens` (fixed-point, one division)
  rather than by raw `count`. A short page where a term is dense now outranks a long page where it
  is only mentioned in passing, even with a smaller raw count. `Found::count` still reports the
  raw occurrence count in the printed answer; only the order changed, and `apropos capability`'s
  own answer over this store did not reorder, because the strongest page for that term is also the
  densest one. `index::VERSION` moved to 2 for the format change; every shard is a build artifact
  regenerated by `cargo xtask manual`, so there was nothing to migrate.
- **A search answer is up to 86 columns wide** when a long location meets a long title, so it wraps
  on the 80-column serial console and wraps hard on the 32-column graphical one. The location is
  never truncated to fit, deliberately: it is the name the reader is meant to type, and a wrapped
  line beats an unusable one.
- **A negative example cannot be written into this page.** This note is in the `manual` bundle, so
  every word here is a word the store then says, and writing `apropos <nonsense>` with its answer
  would make that answer wrong on the next build. The boot gate holds the negative control instead,
  with a word chosen to appear in no bundled page. That is funny and it is also the honest shape of
  a system that documents itself: the documentation is data.
- **A shard whose version is not the reader's is refused, not migrated.** Every shard in the tree is
  a build artifact regenerated by `cargo xtask manual`, so the format and its reader ship together
  and there is nothing yet to migrate. The day a shard arrives from somewhere the build did not
  produce, that is the decision to revisit.
- **The shell reads at most 256 bytes of the bundle manifest**, which is ten times what the shipped
  store needs and is a quarter of a page in a program with one stack page. A manifest that fills the
  buffer says so and searches what it read, because a silent "no page says that" about bundles
  nobody looked in is the one failure a search must not have.
- **`apropos` searches from the root of what the shell holds, not from the cwd.** The store is
  installed at that root and a `cd` does not move the manual. A shell granted a *subtree* that does
  not contain `doc/` therefore cannot search at all, and says so with the filesystem's own errno.
- **The index is 1.18x the markdown it indexes**, per the table above, and it was 1.56x when
  phase 1 measured it. The floor is what moves it: page alignment costs every bundle 16 KiB
  however small, so the ratio improves as the bundles grow rather than because anything got better.
- **A source line longer than `documentation::LINE_MAX` (2048) loses its tail.** The longest line in
  this repository is 1928 bytes <!--count:longest-markdown-line-->, so the corpus fits; a document
  from elsewhere may not, and `Renderer::is_truncated` reports it while `doc` does not print it. The
  number carries a marker because it drifted: these three places said 1835 for as long as the two
  gated ones said 1841, which is the margin this milestone is measured against going stale in the
  prose that explains it.
- **Table cells are truncated to their column width**, so a wide table on an 80-column terminal
  loses text. This is a formatting choice, not a parsing failure, and the corpus test runs at 4000
  columns to keep the two apart. A table too large for the renderer's buffers spills into a second
  aligned chunk rather than losing rows; this repository's largest table is 117 rows.
- **Setext headings and reference links are not recognised**, and no HTML is interpreted. There is
  one reference link in the corpus and no setext heading; `---` on its own line is a thematic break
  here 64 times, so reading it as a heading would misrender all of them to catch none.

## Where this goes next

The guest-side `apropos` that used to head this list is built (phase 2, above), and it went to the
builtin the entry predicted. The spawn-protocol decision that used to head this list is also built
(DECISIONS §106, 2026-08-22): `doc <page>` renders at the prompt with no `| wc` in front of it, the
caretaker-hop race it flagged before the decision was made is a named, accepted interim rather than
a surprise, and both are recorded in this note's `BUGS` section now rather than here. The ranking
item that used to be third here is built too (2026-08-22, same day): see this note's `BUGS` section
for what changed and what did not. What is left, in the order it pays off:

1. **Colour and the pager, which are the same decision's other two thirds.** DECISIONS §106 took
   the narrowing for a tail stage's *primary* output; it did not extend the same bit to "tell this
   stage it ends at a real screen" (colour, the honest replacement for `isatty`) or to granting one
   line of *input* without granting the keyboard (the pager). Both still want the wiring bit this
   entry originally scoped for all three; §106 built the narrowest slice that unblocks `doc <page>`.
   This is a spawn-protocol decision, the same shape as §106 itself, so it is not this lane's call
   to make unilaterally.
2. **The store as something a package installs**, rather than a table in `xtask`. `DOC_BUNDLES` is
   the shape milestone 40 asked for minus a package manager, and milestone 39 is where the manifest,
   the hash and the version it should hang off already live. Blocked on milestone 39 itself, whose
   own Gate is `DECISION, MILESTONE 23` with no decision taken.

Phase 3 of the roadmap (a graphical viewer as a compositor client) is untouched and still wants
milestone 33's rungs, which are themselves blocked (notes/frames.md's `Frame`-sizing fork, milestone
29's status).

## Prior art

`man` plus `apropos` plus `mandb` for the split between format, index and pager, which is this
architecture minus the troff. Dash and Zeal *docsets* for the bundle shape. `cargo doc`'s HTML as
the road not taken: it would need a browser engine, which is a mountain with no thesis behind it.
