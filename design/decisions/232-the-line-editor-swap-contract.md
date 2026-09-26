---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 232. The `line_editor` swap contract: a quiesce opcode on the served endpoint, and a retry flag

*Section number provisional until the merge queue lands it, for the reason §231 (a swap's warning to
a dependent is advisory) gives. The file name and both new identifiers are provisional too.*

Raised 2026-09-26 by the lane for milestone 23 (a capability-routed component OS with live replacement)
([block](../roadmap/23-component-os-live-replacement.md)). The lane had built state handoff and then
checked why the interactive stack is still not swapped. The two forks, the eleven programs that speak
the terminal contract and the state sizes are in two files. Both are on branch
`milestone/23-line-editor-swap`, stacked on #1342, and neither is on `main` yet:
[`notes/interactive-stack-swap.md`](https://github.com/crickertech/nife/blob/milestone/23-line-editor-swap/notes/interactive-stack-swap.md)
and
[`design/roadmap/proposals/swap-line-editor-live-under-system-initializer.md`](https://github.com/crickertech/nife/blob/milestone/23-line-editor-swap/design/roadmap/proposals/swap-line-editor-live-under-system-initializer.md).

## The ruling

calef, 2026-09-26 (UTC): *"1a and 2b."* Recorded by the maintainer at 18:22Z the same day.

Fork 1, answer 1a. An additive `OP_QUIESCE` in `line_editor::proto`. Only a supervisor sends it.
It rides the served endpoint, so the endpoint's FIFO does the waiting. Every request queued before
it is answered first, per §41 (the endpoint is the broker, and a device is revoked by taking
it back). No existing opcode changes. It is the shape `swap_protocol` already has.

Fork 2, answer 2b. A new reply flag, `FLAG_RETRY` (name provisional), meaning "ask again". The
incumbent answers a parked reader with it before it quiesces. The reader re-issues the same request,
the replacement has absorbed the edit line and redraws it, and the person sees nothing.

- For `OP_READLINE` it is a bit in the reply word, beside `FLAG_INTERRUPTED`.
- For `OP_READRAW` the reply is `r0 = 0` bytes, with the flag in `r1`.

## The interaction with §227

§227 (the shell edits its own line), open as #1361, rules option D: the shell owns its own line and
reads the terminal in raw mode. So at an idle prompt `swish` is parked in `OP_READRAW`, not
`OP_READLINE`, and the `OP_READRAW` form above is the one the shell meets. `OP_READLINE` keeps
its readers, since other line-mode clients of `line_editor` remain. The swap test's witness is
therefore a line-mode client as well as `swish`.

## Refused

- 1b, a separate control endpoint only a supervisor holds. It loses the FIFO property: a quiesce
  on another endpoint can overtake a request already queued on the served one, so the drain is racy.
- 2a, answering the parked read with `FLAG_INTERRUPTED`. No contract change, but it misstates why:
  the shell would redraw its prompt as if the person had pressed `^C`.
- 2c, deferring the swap until no read is parked. At an idle prompt a read is always parked, so the
  swap would never happen.

## What follows, and what it does not decide

The swap also needs the handoff to carry more than one page, because `line_editor`'s state with
history is a little over 4 KiB. That is §233 (`MemoryRegion::RETYPE` takes a page count), not this section.
The `component_plan` declaration for `line_editor` waits on the ELF-note manifest work (#1338),
so it is written once where a manifest finally lives.

Both answers are wire changes to a contract eleven programs speak, which is why they were an
architect's call. Both are additive: no existing opcode or flag changes its meaning.
