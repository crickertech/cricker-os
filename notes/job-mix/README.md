# The job mix: the appendices to notes/job-mix.md

[`notes/job-mix.md`](../job-mix.md) is the page to read: the instrument, the bench procedure and the
Results table. The parent is over its word budget, so a bench evening's full reading lives here and
the parent's Results row points to it.

- [`radon-2026-10-04.md`](radon-2026-10-04.md): five boots of the seven-job instrument on radon,
  read in step 6's order and judged against step 7.
- [`null-syscall-under-load.md`](null-syscall-under-load.md): why that evening's `null_syscall`
  nearly doubled from one busy core to four (the reaper freed kernel stacks under `IPC_TABLES`),
  the fix, and the radon procedure that will size it.
- [`null-syscall-first-radon-run.md`](null-syscall-first-radon-run.md): what TCG could not say about
  that fix, and the procedure radon sized it by, moved out of the note above for §212 (a prose budget).
- [`null-syscall-off-radon.md`](null-syscall-off-radon.md): the rest was the global lock on every
  capability lookup, measured under TCG and on four Apple cores under HVF (2026-10-05).

*Name: provisional, minted 2026-10-04 (UTC) by the `lane/radon-jobmix-2026-10-04` lane, for the
directory and every stem in it; `lane/null-syscall-under-load` added two stems on 2026-10-05. Naming is an architect's; `script/names --unratified` lists each
stem.*
