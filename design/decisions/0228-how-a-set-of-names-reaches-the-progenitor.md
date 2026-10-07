---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 228. How a set of matched names reaches the progenitor: in a page the shell fills, copied and checked

Raised 2026-09-26 by the maintainer, from milestone 47 (navigation and naming)'s lane
`milestone/47-navigation`, which wrote the options up in
[notes/a-set-grant-at-the-prompt.md](../../notes/a-set-grant-at-the-prompt.md). The options, the
costs and the lane's reasoning are there and are not copied here. *(Section number provisional until
the merge queue lands it.)*

## The ruling

calef, 2026-09-26 (UTC): *"2b".* He first asked for `rm *.txt` over 30 files to work. He then ruled
that swish gets an allocator: fallible, capped, and with no word splitting as a tested rule. He
chose among the options assuming that allocator, which is why 2b is not in the table below: it was
put to him in conversation once the allocator removed the stack bound.

What 2b means:

- The set travels in a page the shell fills, option 3's carrier. The progenitor copies it, checks
  the encoding, and maps its own read-only copy for the set caretaker, so the shell's later writes
  reach nothing. A new `spawnproto` bit announces the page; its name is provisional.
- A page holds a few hundred names, and a set may span pages. The building lane proposes the limit
  and states its memory cost.
- Over the limit, the shell refuses, names the limit and suggests `xargs`. It never truncates a set.
- `grant::MAX_NAME` (16 bytes today) rises. The building lane proposes the value, with Unix's
  `NAME_MAX` of 255 as the reference.
- The preview is the grant. The set is resolved when the command is planned, as the nameset code
  already requires, so what `caps` shows is exactly what the caretaker enforces.

### Refused, with reasons

- 1 (leave it). `rm *.txt` would never work, and calef asked for it to.
- 2 (data words alone). It cannot carry a realistic set. Its eight-name limit came from the shell's
  stack, and the allocator removes that bound, so 2 would keep a limit whose cause is gone.
- 4 (batches of one). It was argued only on effort, and it does not fix a plain `rm *.txt`.
- 5 (a pattern grant the caretaker resolves), raised in the same conversation. The preview could
  differ from the grant, since the directory can change between the two, and it is a new kind of
  grant. It stays open for the day a program needs a live pattern rather than a fixed set.

### Prior art

All three are from memory and were not re-read for this ruling:

- Unix `execve` copies the argument strings into the new process, up to `ARG_MAX`. A larger set
  fails with `E2BIG`, and `xargs` exists to split one. 2b takes the copy and the loud refusal,
  and points at `xargs` the same way.
- Fuchsia's `processargs` protocol sends a new process its arguments and handles in a bootstrap
  message on a channel, which the receiver owns once sent.
- CloudABI's `argdata` passes structured arguments, including file descriptors, as one encoded
  buffer rather than as flat strings.

### What it depends on

- A frame per filesystem client channel: the lane's proposal
  `a-frame-per-filesystem-client-channel`, which PR #1358 promotes to a milestone (provisionally
  numbered 599), and whose design §230 (badged endpoint capabilities) rules. A set caretaker at the prompt must not
  share the file service's staging frame with another client.
- The swish allocator calef ruled the same day (`milestone/47-swish-allocator`, in flight). A set of
  a few hundred names does not live on the shell's stack.

The options as they were raised follow, unchanged apart from this section.

## What was decided

How the shell asks the progenitor to build a set caretaker, so that a pattern matching two or more
names can be granted at a real prompt. It is a change to `spawnproto`, which the shell and the
progenitor both read, so the section was raised with options only.

## The premise, checked 2026-09-26

It is wider than the record said. The record said `xargs <program>` stops after its first batch.
In fact any pattern that matches more than one name is refused before anything spawns, `rm *.txt`
over two files included: `components/src/swish.rs` answers "a set of names is delivered by a
nameset caretaker, and the progenitor builds the subtree one". `DIR_BIT` in
`crates/grant_plan/src/spawnproto.rs` carries two words of directory and name, and no bit announces
a set. The caretaker itself exists and is proven (`components/src/fs_nameset_caretaker.rs`), but only
a kernel test wires it. It needs up to 137 bytes of encoded set (`filesystem_protocol::nameset::BYTES`)
in a read-only frame.

## The options

The note's table has the detail. In one line each:

1. Leave it. A pattern designates one name or is refused, and `xargs <program>` never runs a batch.
2. The set as data words (provisional `SET_BIT`). Up to six more `SEND`s of three words carry the
   encoded set; the progenitor checks the encoding, writes it into a frame it retypes, and maps it
   read-only for the caretaker. One transient capability slot.
3. The set as a frame the shell owns, §219 (how the shell names an installed program to the
   spawner) option D's shape. The shell keeps write access, so the progenitor has to copy before
   mapping, which makes it option 2 with a page in the middle.
4. Batches of one, no wire change: `xargs` plans one name per spawn through the subtree caretaker
   that already works. It does nothing for a plain `rm *.txt`, and the note says plainly that it is
   recommended only as effort.

The lane recommended option 2, because the carrier is data the progenitor checks rather than a
page someone else can still write. 2b keeps that property by copying the page before anything
reads it.

## The seven questions

1. What else was considered. All four are in the note, each with the place it fails.
2. What this tree does in the analogous case. `DIR_BIT` announces extra `SEND`s carrying data, and
   milestone 154 (a process that holds two directory capabilities) already grew `spawnproto` the
   same way with `DIR2_BIT`. Option 2 is the third use of that shape, not a new mechanism.
3. Prior art, from memory and not re-read: Unix `execve` copies the expanded argument strings into
   the new process, so the caller cannot change them afterwards. That is option 2's property, and
   option 3's race is the one Unix avoids by copying.
4. Is the premise true. Yes, and wider, as above.
5. Cost. Three costs are unmeasured, and the note lists them: whether the directory-granted spawn
   path's capability-table peak stays under the measured 23 of 24
   (`kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED`), the progenitor's pages for carrying the set
   caretaker's ELF, and the frame build itself. `script/swish-check` fails loudly on the first.
6. Reversibility. Irreversible in AGENTS.md's sense: two programs agree on the bit. Who has acted on
   it: nobody yet, because nothing has been built.
7. Equal cost. Option 4 is the only one chosen for effort, and it says so. Between 2 and 3 at equal
   cost, 2 still wins, because 3 must copy anyway.

## Why the frame per channel is a prerequisite

The file service shares one read-write staging frame with every client. A set caretaker built at
the prompt, beside a `>` caretaker in the same pipeline, makes the check-then-use window that
`notes/shared-page-audit.md` called unreachable a live one. The lane filed that work as a proposal,
which PR #1358 promotes. Option 4 would not have needed it.

## What this unblocks

The set grant at the prompt, buildable in milestone 47 (navigation and naming) once its two
dependencies land, and milestone 109 (`xargs`: batching a grant too large to hand over) past its
first batch.
