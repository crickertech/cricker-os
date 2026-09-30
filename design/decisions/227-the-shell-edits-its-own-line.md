---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 227. How Tab reaches the shell: the shell edits its own line, and the terminal wire does not change

Raised 2026-09-26 by the maintainer, from milestone 47 (navigation and naming)'s block, where the
proposal has sat unchanged since 2026-08-26 with no section for calef to answer. *(Section number
provisional until the merge queue lands it.)*

## The ruling

calef, 2026-09-26 (UTC): *"D".* The shell turns raw mode on (`OP_RAWMODE` and `OP_READRAW`, from
milestone 169 (`kilo`, the smallest real text editor)) and runs `LineDisc` in its own process, the
way bash and readline do. No new wire message.

He chose it after seeing what each option means for a person at the prompt. D is the one that
gives a prompt like fish's: suggestions as you type, syntax colouring, a live `^R` history search,
and completion that knows a program's arguments from its manifest. What it costs that person is
recorded rather than hidden:

- Editing keys may differ between the shell and other programs that read a line through the
  terminal, since the shell no longer uses the terminal's editor.
- A stalled shell stops echoing, because the shell now does the echo.
- `^C` at the prompt becomes the shell's own path under §24 (interrupting the foreground process),
  a byte it reads, rather than the terminal's `FLAG_INTERRUPTED`.

D partly reverses §21 (the terminal is a userspace component) for the shell. §21 put line editing
in the terminal so that a program never sees a keystroke, and the shell is the program it most had
in mind. That was weighed and accepted: every other client keeps §21's terminal editor, and the
terminal's protocol is unchanged, so the reversal is one client's choice rather than a change to
the contract.

### Refused, with reasons

- A (leave it). Refused because it leaves the prompt without completion or any of what D makes
  possible, and nothing about A gets cheaper by waiting.
- B (a resume opcode). Refused because it spends an opcode two programs agree on forever to buy
  Tab alone. Suggestions as you type and colouring would each want another round trip on the same
  wire, so B is the first of several messages rather than the last.
- C (a resume bit). Refused for B's reason, and it spends reserved bits instead of an opcode, which
  is the same cost in a place that is harder to see.

### Owed by the building lane

Two costs in the table below were unmeasured when this was ruled. The lane that builds D owes both
as measurements in its block. One is the shell's binary size with `LineDisc` linked in, against
today's. The other is the per-keystroke IPC cost of `OP_READRAW` (at most 8 bytes a reply) against today's
one `OP_READLINE` reply per line.

The options as they were put to calef follow, unchanged.

## What was decided

Whether Tab completion exists at the prompt, and if it does, what carries it between the terminal
component and the shell. Every option but D is a change to `line_editor::proto`, which the
terminal, the shell and the kernel-side tests all read, so the section was raised with options and
no recommendation.

The full design, with its four pieces costed, is milestone 47's section "Completion: a concrete
primitive, priced and not built" in
[design/roadmap/47-navigation-and-naming.md](../roadmap/47-navigation-and-naming.md). It is not
repeated here.

## The premise, checked 2026-09-26

Tab is ignored, and the reason is structural. `crates/line_editor/src/lib.rs` says so in its module
doc, and `LineDisc::feed` drops the byte into its catch-all. The shell's only read is a blocking
`CALL` on `OP_READLINE`, which replies once, when Enter ends the line. Completion needs a reply in
the middle of a line, and then a way back into that same line. The terminal cannot answer Tab
itself, because the candidates are the shell's authority (its directory capability and its program
names), and the terminal holds none of it.

One fact the 2026-08-26 write-up did not weigh: milestone 169 (`kilo`) landed `OP_RAWMODE` and
`OP_READRAW` the same day. A client can now take keystrokes raw. That makes a fourth option real,
and it needs no new wire message.

## The options

| | What carries Tab | New wire message | Cost |
|---|---|---|---|
| A. Leave it | Nothing. Tab stays ignored | None | Zero. Nobody has been unable to use this shell for want of Tab |
| B. A completion round trip with a resume opcode | `Event::Tab` in the engine; `OP_READLINE` replies early with a new flag (provisional `FLAG_COMPLETE`) and the partial line. The shell answers with a new opcode (provisional `OP_READLINE_RESUME`) carrying the edited buffer and cursor | Two: a reply flag shaped like `FLAG_EOF`, and a request with no precedent, "continue an exchange" rather than "start one" | One crate, one component, one client. The block sizes it at one lane |
| C. The same round trip, resumed by a bit | As B, but the resume is a bit on `OP_READLINE` itself, in the request word's reserved bits 55:32 | The same flag, plus a meaning for bits that are reserved as zero today | As B. It spends reserved bits instead of an opcode number |
| D. The shell edits its own line | The shell turns raw mode on and runs `LineDisc` in-process, the way bash and readline do on Unix | None. `OP_RAWMODE` and `OP_READRAW` exist | Unmeasured: the shell's size with the engine linked in. Known by shape: an IPC round trip per keystroke burst (at most 8 bytes per `OP_READRAW` reply) instead of one per line. And `^C` arrives as a byte, so the shell's interrupt path (§24 (interrupting the foreground process)) changes |

B and C differ only in how the resume is spelled on the wire. D differs in where line editing lives.

## The seven questions

1. What else was considered. A fifth option, the terminal completing on its own, is Plan 9's shape
   (below). It was refused in the block because the terminal would have to hold the session's
   directory capability on its behalf, which is a per-session delegation into a server that holds
   no capabilities today. That reason stands.
2. What this tree does in the analogous case. `FLAG_EOF` and `FLAG_INTERRUPTED` are the reply shape
   B and C copy. Nothing in `line_editor::proto` resumes an exchange, which is why the resume is the
   decision. D is what milestone 169 already does for `kilo`.
3. Prior art, read 2026-09-26. Plan 9's `complete(2)` takes a directory and a prefix, and `rio` and
   `acme` call it on `^F` or Insert: the window system completes, from its own view of the
   namespace. From memory, not re-read: GNU readline runs inside the shell's process with the tty
   in non-canonical mode, which is D's shape.
4. Is the premise true. Yes, checked above, with the one correction that raw mode now exists.
5. Cost. B and C are sized in the block by piece; none of it is measured, because nothing is
   built. D's per-keystroke IPC and binary size are unmeasured, and a lane taking D should measure
   both before building.
6. Reversibility. B and C are the irreversible category: an opcode or a reserved bit, once two
   programs agree on it, is not un-shipped. D adds no wire, so it is cheap to undo in code, but it
   reverses part of §21 (the terminal is a userspace component) for the shell, which is the one
   program §21 had in mind.
7. Equal cost. Not applicable when raised, since there was no recommendation. Where the block leaned toward B,
   its reason was blast radius (one crate, one component, one client) and not effort.

## The question that decided it

Does line editing stay in the terminal for the shell, as §21 put it? calef's answer was no: the
shell may own its own line, so D, and no wire at all.

## What this unblocks

Tab completion in milestone 47 (navigation and naming), which can now be built with no
architect's call left in it.
