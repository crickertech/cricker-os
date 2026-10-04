---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: riscv64 silicon with four or more harts
specific_machine: none
needs_person: yes
---
# Capability lookup off the global lock

Raised 2026-10-04 (UTC) by `lane/null-syscall-contention`, from fatal risk 4's open finding: why the
cheapest syscall nearly doubles in cost when four cores are busy. Title, slug and every name below
are drafts. `needs_person` is yes only because the result is taken at radon's bench.

## The question

Every syscall that names a capability starts in `sched::current_cap`, and `current_cap` takes
`IPC_TABLES`, the kernel's one global lock, to read the calling thread's own capability table.
Should each thread's capability table sit behind its own lock instead, so that the lookup stops
competing with every other core's IPC and scheduling?

## The numbers behind it

[`notes/job-mix/null-syscall-under-load.md`](../../../notes/job-mix/null-syscall-under-load.md) has
both radon evenings. In short, at four tasks on radon (2026-10-04, five boots):

- Freeing kernel stacks under `IPC_TABLES` was half of the null syscall's growth and is fixed. The
  growth from one task to four fell from 94 ticks a job to 48, and `jpm_median` rose 9.4%.
- What is left is contention for the lock itself. `IPC_TABLES` is 95% of all lock wait.
  41% of `current_cap` calls find it held (0.015% at one task, 51% at 32), and they wait 3.70 ticks
  on average when they do.
- `current_cap` is 28% of the `IPC_TABLES` wait. The other 72% is the lock's other acquirers, the hot
  rows of [`notes/ipc-tables-lock-inventory.md`](../../../notes/ipc-tables-lock-inventory.md):
  `ipc_call`, `ipc_receive_cap` and `ipc_reply`, `schedule()` (twice per switch), `finish_switch`,
  and `take_ipc_aborted` after each IPC. Each of those waits slightly longer on average (3.80 ticks).
  TCG's hold-time table names the same holders, with `start_thread_control_block`.

## What this would and would not buy

**It would take the lock off the null syscall entirely.** A refused `invoke` would take only its
own thread's lock, which no other core contends for in the job mix, so the job's growth from one
task to four should fall to roughly `compute`'s preemption share (about 6 ticks). That is the
prediction to check, and it is the whole of the per-crossing cost risk 4 still carries.

**It would also take every IPC invoke's first acquisition off the global lock.** `CALL`, `REPLY`
and `RECEIVE_CAP` each enter through `current_cap` before taking `IPC_TABLES` again for the IPC
itself, so each IPC syscall makes two global acquisitions today and would make one. That shortens
the queue the other 72% wait in, though by how much is a measurement, not a derivation.

**It would not move the IPC and scheduler rows.** The thread table and the endpoints stay under
`IPC_TABLES`. Splitting those is milestone 17 (the multikernel-leaning scheduler), which the
inventory gates on hardware with more harts than radon has.

## What it costs, and where the hard part is

- The inventory already calls this the free win, and says why. A capability table has exactly one
  owner thread, and it shares the global lock only because it is stored in the thread table.
- **Writers from other cores.** `ipc_send_cap` and `grant_at` insert into a receiver's table, and the
  revocation sweeps (`delete_page_frame_caps_where`, `delete_device_frame_caps_from_others`) walk
  every table. Each must take the per-table lock under `IPC_TABLES`, at a rank below it. The sweeps
  then pay one acquisition per table, which is the trade the inventory said to price first.
- **Reaching the table without `IPC_TABLES`.** The current thread cannot be reaped while it runs, so
  its page-resident TCB is stable for the length of its own syscall, and a per-core pointer to it
  makes the lookup lock-free of the thread table. That claim wants a loom model or a written proof
  before it is relied on (milestone 80 (loom) is the method).
- **The IPC fastpath footprint gate** is at +4.4% of its 5% band on x86_64 `ipc_send_receive`. A
  second lock on the IPC path has to fit, or the path has to drop the global acquisition it no
  longer needs.
- **Collision surface:** `capability-tables-sized-per-process.md` and the 64-slot capability table
  lane touch the same structure. This should follow them, not race them.

## How to know it worked

The same two payloads as the 2026-10-04 evening: three boots of the plain job-mix image and two of
`--extra-features lock_wait`, against `bench/radon-2026-10-04-fix/`. Success is `null_syscall` at
four tasks within about 10 ticks of `compute`'s share (roughly 105, against 147 today), with
`current_cap` gone from the `job-mix-lock:` lines and the `IPC_TABLES` contended count falling.
