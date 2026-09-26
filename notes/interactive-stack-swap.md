# Swapping the interactive stack

*A proposal from the lane for milestone 23 (a capability-routed component OS with live
replacement), 2026-09-26. Status: **PROPOSED**, waiting on an architect for two contract changes.
Nothing here is built. The block's last `Outstanding` line says `line_editor`, `display_terminal` and
`compositor` are not swapped by `swapper` and that "what is missing is a swap role for them, not a
harness". Checked against the tree, that is not quite what is missing, and the three are three
different problems.*

## What the tree says, checked

| component | who builds it on a real boot | who could swap it | state it holds |
|---|---|---|---|
| `line_editor` | `system_initializer`, out of its own budget (`crates/system_initializer/src/lib.rs`, item 3) | `system_initializer`: it holds every object it routed | the edit line, the kill buffer, eight lines of history, up to four queued lines, raw or cooked mode, the interrupt count: a little over 4 KiB |
| `display_terminal` | the kernel, before the progenitor exists (`kernel/src/user.rs`, `boot_graphical_terminal`) | nobody: no userspace process holds its endowment or its supervision endpoint | the character grid and scrollback, several hundred KiB (`video_terminal::Vt`) |
| `compositor` | no boot builds it. Only the tests of milestone 33 (a compositor: one screen, mutually distrusting clients) spawn it (`kernel/src/user/compositor_service.rs`, `#[cfg_attr(not(test), ...)]`); the graphical boot is option A of milestone 177 (wire the graphical terminal stack into the real interactive boot), "no compositor in this path" | nobody outside a test | nothing it could not rebuild: each client's surface is a frame the client owns |

So the block's 2026-09-03 correction was itself half right. All three do run under the kernel test
harness. But milestone 177 wired two of them into a boot path, not three, and one of those two was
built by the kernel for a capability-table budget reason, which puts it outside every supervisor.

None of the three has a `component_plan` declaration. Their endowments are literals in their
builders, which is the defect milestone 23's manifest work removed from `swapper` and nowhere else.
The ELF-note manifest work (milestone 597 (a program carries its manifest in an ELF note),
PR #1338) is changing where a manifest
lives; writing these three declarations should follow it rather than race it.

## line_editor: two contract questions, then ordinary work

This is the one to do first. Its supervisor exists, its state nearly fits a page, and §209 (state
handoff is an opaque blob over a granted frame, and it is optional) is now built (notes/state-handoff.md).
Two things stand in the way, and both are changes to the terminal contract, which eleven programs
speak (`swish`, `rmle`, `mdr`, `window`, `input`, `keyboard_driver`, `console`, `display_terminal`,
`compositor`, `terminal_sink_caretaker` and `line_editor` itself). That makes them wire-format
decisions, and so an architect's.

Fork 1: a quiesce request. The swap's drain step rides the served endpoint so its FIFO does the
waiting, per §41 (the endpoint is the broker, and a device is revoked by taking it back). The terminal
contract has no such opcode. The options: an additive `OP_QUIESCE` in `line_editor::proto`, which
changes no existing opcode and which only a supervisor sends; or a second, supervisor-only control
endpoint, which loses the FIFO property and makes the drain racy. Recommend the additive opcode.
It is what `swap_protocol` already does.

Fork 2: what a parked reader is told. At an idle prompt `swish` is always parked in
`OP_READLINE`, and its one-shot reply capability lives in `line_editor`'s capability table, where no
one can move it (notes/hung-component.md, question 3). So the incumbent must answer it before it
quiesces. Three answers:

- `FLAG_INTERRUPTED`, which exists (`line_editor` already fails a parked read this way on `^C`). No
  contract change, but `swish` would redraw its prompt as if the person had pressed `^C`, and it lies
  about why.
- A new flag, say `FLAG_RETRY`, meaning "ask again". `swish` re-issues the same `OP_READLINE`;
  the replacement, having absorbed the edit line, redraws it; the person sees nothing. One bit in the
  reply word and one branch in each reader.
- Defer the swap until no read is parked. At an idle prompt that is never.

Recommend `FLAG_RETRY`, because it is the only one that makes the swap invisible, which is the
milestone's whole claim. The flag name is provisional.

Then the state. With history, the blob is a little over one page, which is the one-page limit in
`component_plan`'s `BUGS`. Carrying the edit line, the queue, the mode and the interrupt count fits;
history does not. Either drop history across a swap (recorded as a limit) or give `Handoff` a page
count first. Recommend the latter, because `redoxfs_server` needs it too.

## display_terminal: blocked on where it is built

No supervisor can swap what no supervisor built. `display_terminal` is spawned by the kernel because
a virtio-gpu driver needed eleven capability-table slots, one `PageFrame` per DMA page, and the
progenitor did not have them. That premise looks stale. §102 (a Frame names a run of pages) was
decided on 2026-08-20, a week before the comment was written, and `MAP_INTO` already maps a whole run
in one call (`kernel/src/syscall.rs`, the `MAP_INTO` arm's §102 comment: "a run-capable frame maps
the whole run in this one MAP_INTO call"). So the slot count falls to one per DMA *run*. **What is not
checked is whether the kernel mints the gpu's DMA pages as one contiguous run**, and whether the
driver's `virtio` registration accepts one. If both hold, moving the graphical stack's construction
into `system_initializer` is ordinary work and the swap follows it.

Its state is also far past a page. A replacement that starts with an empty grid over a surface that
still shows the old text would look right until the first scroll, then lose the scrollback. That is
probably acceptable (declare no handoff, record the loss), and it is an architect's taste call rather
than an engineering one.

## compositor: not before it has a boot path

It is the easiest of the three on paper: no state worth moving, and every surface a client owns is a
capability its supervisor routed, so a replacement wired from the same declaration gets the same
surfaces. But no boot runs it. Swapping it under the milestone 33 test harness alone would measure
the harness, which is what the block's original sentence warned against.

## Proposed milestones

Provisional titles; the integrator mints the numbers. Each is a file under `design/roadmap/proposals/`:
`swap-line-editor-live-under-system-initializer.md` and
`build-the-graphical-terminal-stack-in-userspace.md`, since promoted to milestone 600 (the
graphical terminal stack is built in userspace).

1. Swap `line_editor` live under `system_initializer`. Blocked on forks 1 and 2 above, and best
   sequenced after the ELF-note manifest work and after `Handoff` grows a page count.
2. Build the graphical terminal stack in userspace now that a frame names a run. Starts by
   answering the one open question above (are the gpu's DMA pages one run). It unblocks swapping
   `display_terminal`, removes a kernel-side spawn that exists for a budget reason that may have
   expired, and corrects `boot_graphical_terminal`'s doc comment either way.

The compositor gets no milestone until it has a boot path; that is recorded in the block.

## BUGS

The state sizes are read from the code, not measured. `line_editor`'s "a little over 4 KiB"
adds up `LINE_MAX`, `HIST` and `QUEUE` from `crates/line_editor/src/lib.rs` and
`components/src/line_editor.rs`; `display_terminal`'s is `video_terminal::Vt`'s own doc comment.

## See also

- notes/state-handoff.md, notes/live-replacement.md, notes/terminal-contract.md
- design/roadmap/177-graphical-interactive-boot.md for why the graphical stack is built kernel-side
