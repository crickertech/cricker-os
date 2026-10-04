---
status: BUILT
raised: 2026-10-04
built: 2026-10-04
promoted_from: the-noteless-launch-that-prints-nothing
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 748. The noteless launch that prints nothing

Raised by the lane that recorded the signature in
[the swish-check flake note](../../notes/swish-check-flake.md); chased by `lane/noteless-flake-chase`
(2026-10-04 UTC). The number 748 is provisional until the queue lands it. *(Title and slug are
drafts; `without_redraws` and `NIFE_SWISH_ENTER_PAUSE_MS` are provisional names.)*

## The problem

Twice in four hours on 2026-10-03 and 2026-10-04 (UTC), the aarch64 leg of swish-check saw
`packages/noteless/0.1.0/noteless` answer an empty string. One of the two evicted #1573 (docs only)
from the merge queue.

## What it found

**The program answered; the gate misread the transcript.** In both failing runs the line was
typed once and echoed twice, with a kernel gauge between the copies and the program's sentence
after the second. A gauge that reaches the console mid-line waits for the line end. Here the system
log service's 250 ms flush fell between the echo of the last typed character and the echo of
Enter. The console then wrote the gauge on its own line and redrew the partial line beneath it, as
milestone 342 (the kernel and the `console` server drive one UART from two address spaces)
designed. The gate filters gauges out, so it read `$ line` twice and took the first copy for a
command that printed nothing. The system behaved correctly and the harness was wrong.

Both failures were the first line after the reboot, the line a fresh gauge follows. The earlier
record put one after `package rollback`, which the transcript does not show. The signature
appeared only after 342 landed.

## What was built

- `without_redraws` in `xtask/src/swish_check.rs` takes out an exact redraw: a `$ ` line ending
  where a gauge was removed and followed by the same line. A prefix redraw (a flush mid-typing) is
  left alone, since every reader already handled it. The one shape it could mistake is a silent
  line typed twice in a row, and a test keeps every script free of it.
- A missing wanted phrase that is in the transcript before the next line's echo now says so: the
  program wrote it and the gate cut the answer short. That is the self-describing failure this
  block asked for; the cause turned out to be neither a launch that never ran nor one that ran
  silently.
- `NIFE_SWISH_ENTER_PAUSE_MS` types like a person, pausing before Enter, which opens the window on
  every line.

## Evidence

- Red to green: a unit test feeds run 37167978481's bytes through the gate's own filter and parser.
  Before the fix it answered `""`, as CI did.
- Widened window, aarch64 under QEMU on patagonia, pause 400 ms: without the fix 4 of 4 boots failed
  (3 empty answers each, on whichever lines a gauge followed); with it 0 of 3, reboot segments
  included. Rows in `notes/swish-check-flake-2026w40.md`.

## Follow-on

- **Recorded.** The flush deadline is armed by the first forwarded line, not the queued one, so a
  redraw can come well inside 250 ms; cosmetic, in `components/src/system_log.rs`'s BUGS.
- **Recorded.** Only gauges are filtered. Another kernel line flushed above a redraw stays in and
  fails the line that owns it, loudly. See `without_redraws`'s doc in `xtask/src/swish_check.rs`.

## Index row

The last flake evicting merge-queue entries after milestone 342 was the gate misreading a console
redraw as a second prompt, not a silent program. The harness now takes the redraw out, says when an
answer was cut short rather than unwritten, and can open the window on demand.

## The work

- Reproduce on aarch64 under `helpers/qemu-bounded.sh` and count occurrences with a denominator.
- Make the failure self-describing.
- Decide the remediation with that data in hand, and record it in the flake note.

## Done when

The note names the mechanism with a measured rate, and the swish-check failure line distinguishes a
launch that never ran from one that ran silently.
