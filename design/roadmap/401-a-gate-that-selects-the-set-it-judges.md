---
status: BUILT
raised: 2026-09-14
built: 2026-09-23
---
# 401. A gate that selects the set it judges can pass by checking nothing

Built 2026-09-23. Filed 2026-09-14 as an unnumbered proposal by milestone 265 (`_proto` is a truncation), which
broke one and caught it by hand rather than by anything red; numbered 2026-09-19 by milestone 433 (drain the proposal pile), whose
drain of the proposal pile. *(Number provisional until the merge queue lands it.)*

In brief. Several gates pick the things they judge with a pattern, then judge them. When the
pattern stops matching, the loop body never runs and the gate reports clean. Eight such selectors
now assert that they selected something. The enumeration, the rule that fell out of it, and what
each guarded site now guarantees are in [notes/empty-selectors.md](../../notes/empty-selectors.md).

## The exhibit does not hold, and that is the first finding

This block was written around one instance: `script/lint`'s contract-crate check, which globbed
`crates/*proto` until milestone 265 renamed every one of those crates to `*_protocol`. The block
says the glob then matched nothing, the loop ran zero times, and **the check passed by checking
nothing**. Replayed against the tree at `51fee8248^`, the commit immediately before 265's sweep
landed, that is false.

POSIX `sh` leaves an unmatched glob in place as a literal word. `basename` then yields the string
`*proto`, which does not match the `*_proto` arm of the `case`, so it falls to the failure branch:

```console
$ /bin/sh /tmp/replay/script/lint
lint: contract crate *proto should be spelled *_proto (notes/naming.md)
$ echo $?
1
```

The check would have gone red, loudly, naming a crate that does not exist. That is a bad message
and a genuine defect; it is not a silent pass. The loop's shape saved it, because its default arm is
a failure rather than an acceptance, and nobody noticed that the shape was doing the work.

The correction matters beyond the arithmetic. The tree has been carrying this instance as the
exhibit for a whole class, in this block and in its index row, and a reader who checked it would
have found the class unsupported. The class is real anyway, and the sweep found a worse case than
the one that was filed.

## What was built

One assertion per selector that can go quiet. The rule the enumeration produced is narrower than
"every selector", and the narrowing is what keeps it from being thirty-five assertions: a selector
whose empty result is silent needs the assertion; one whose empty result raises does not. A
shell glob, `git ls-files <pattern>` and Python's `glob`/`rglob` are silent. `os.listdir` and
`iterdir` raise, which is how `script/citations`, `script/decisions`, `script/roadmap`,
`script/names`, `script/journeys` and `script/falsifications` read their record directories.

Guarded, with the full table in the note:

- `script/ci-build`, and this is the one that earned the milestone. `names_in_tier local` matches
  a literal column value in the `checks` table with `awk`. Rename the tier or reformat the separator
  and the loop iterates nothing, every check is skipped, and the script prints `ci-build: all pass`.
  It is the command a developer runs before pushing and the enumeration CI fans out from.
- `script/lint`, seven sites: the CFI scan over `kernel/src/arch/*/*.s`, the README
  script-reference resolve, the module-wide dead-code ratchet over `git ls-files '*.rs'`, the rule 7
  `#[path]` scan (whose greps are all `2>/dev/null`, so a moved source tree reads as clean), the
  icount toolchain stamp over `bench/baseline-*.txt`, the notes-index check over `notes/*.md`, and
  the host-exclusion cross-check that reads `xtask/src/*.rs`.
- The contract-crate check keeps a counter too, in spite of the correction above, because the
  property should not depend on a `case` arm somebody could reorder, and because the message it
  prints now says what actually went wrong.

Already guarded, and left alone: `script/vendor-verify` and `script/vendor-watch` both exit on an
empty `vendor/*.pin`; `script/mutation-census` exits on a run with no outcome artifacts; and
`script/fatal-risks`' `no-history` check fires when a shallow clone makes its own check unable to
fail, on the stated ground that a check which cannot fail is worse than one that is absent. That
comment is this block's argument, written by a gate about itself.

**Judged not to need one:** `script/lint`'s `git grep` finders (the TODO-milestone scan, the
`daemon` scan, the conflict-marker scan) look for bad things rather than selecting a set to judge, so
empty genuinely means clean and an assertion there would be a false alarm by construction. That
distinction, finder against selector, is the thing a lint for this could not make.

## BUGS

- **A non-empty assertion is not a correct selector.** A glob matching three of fifteen crates passes
  every assertion added here and still covers a fifth of its subject. Catching that needs the right
  count, which is the hand-kept list `script/verify` and `script/falsifications` both moved away
  from.
- Nothing gates the convention. A selector added tomorrow gets no assertion unless its author
  reads the note. This is rung three deliberately: a lint would have to tell a selector from a finder
  in someone else's shell, and the two are the same syntax.
- The `--exclude` family is untouched and needs a different mechanism. `cargo` takes an unknown
  `--exclude` silently and `.cargo/mutants.toml`'s exclusion globs are not an error when they match
  nothing. A non-empty test says nothing about an exclusion, because a stale exclusion still covers
  everything else.
- The tell is the durable half and no machine can check it. A gate that passed before your change
  and passes after it, on a change that is exactly what the gate is about, has probably stopped
  looking.

## Follow-on

- **Recorded.** The three limitations this leaves are in the `BUGS` section of
  `notes/empty-selectors.md`, beside the enumeration a reader meets them in: a non-empty assertion
  is not a correct selector, nothing gates the convention for a selector added tomorrow, and the
  `--exclude` family needs a different mechanism because a stale exclusion still covers everything
  else.

## Index row

Several gates pick the things they judge with a pattern and then judge them, so when the pattern
stops matching the loop body never runs and the gate reports clean. **The instance this block was
filed about turned out not to be one**: replayed against the tree at `51fee8248^`, `script/lint`'s
`crates/*proto` glob did not pass silently after milestone 265's rename, because POSIX `sh` leaves
an unmatched glob in place as a literal word and `*proto` falls through the `case` to the failure
arm, so the check went red with a confusing message about a crate that does not exist. The class is
real anyway and the sweep found a worse case than the filed one: `script/ci-build` selects its
checks by matching a literal tier string in its own table with `awk`, so renaming that column left
the command a developer runs before pushing running zero checks and printing "all pass". Eight
selectors now assert that they selected something, seven of them in `script/lint` and one in
`script/ci-build`, and the rule that keeps this from being thirty-five assertions is that a selector
whose empty result is silent (a shell glob, `git ls-files`, Python `glob` and `rglob`) needs the
assertion while one whose empty result raises (`os.listdir`, `iterdir`) does not. Finders are
excluded on purpose, since a `git grep` for bad things means clean when it is empty, and that
distinction is why this is one assertion per site rather than a lint. notes/empty-selectors.md has
the per-site table, what each guarded gate now guarantees, and the `--exclude` family it does not
reach.
