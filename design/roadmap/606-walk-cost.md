---
status: PARTIAL
raised: 2026-09-26
milestone_dependencies: 121
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 606. A directory walk costs what it does on Linux

calef, 2026-09-26 (UTC), on the walk figures of milestone 121 (`ripgrep` on nife: enumeration as
a capability): *"20x slower is unacceptable."* Raised
the same day for lane `milestone/606-walk-cost`. *(Number and title provisional: the integrator
mints the number at merge, and the title is a draft until an architect names it.)*

## Index row

A recursive `std::fs` walk on nife was about 68x slower than Linux on the same machine. The FS
server now keeps what it read, so a warm walk never reaches the device and runs at 1.9x Linux.
The rest waits on a wire change.

## The goal

Parity with Linux on the same hardware tier for the walk `ripgrep`'s `ignore` crate performs: list
every directory, read every file. The instrument is milestone 121's (`walk_pricing` over
`filesystem_protocol::fixture::walk`), through the grant a confined `rg pattern src/` will hold.

## What is built

- The measurement. `fs_walk` in `script/bench --real --release --smp` (the release kernel, HVF)
  runs `std_exerciser`'s priced walk and prints it split by `std::fs` call.
  `bench/host/run_linux_walk.sh` runs the same walker as PID 1 of an Alpine kernel on the same
  `virt,accel=hvf` machine. `redoxfs_server/examples/walk_replay.rs` replays the walk's requests
  against the server core and counts device reads and server time.
- The fix inside the contract. The FS server remembers listings, names and small files until a
  mutating verb forgets them (`redoxfs_server/src/memo.rs`). Its block cache is 256 slots,
  allocated as they fill, without the header ring. `OPEN` and `CLOSE` skip the engine's
  transaction when the memo answered, and the `std` PAL copies its shared page with `memcpy`.
- The gates. `redoxfs_server/tests/walk_cost.rs` asserts a warm walk reads nothing from the
  device. Host tests in `redoxfs_server` prove every namespace change, write and truncate is seen
  through the memo. The kernel suite's walk test asserts the new `walk split` line.

Warm walk, release kernel, HVF, 2026-09-26: 27.0 to 29.5 ms before, 0.76 to 0.82 ms after;
Linux 0.41 to 0.42 ms. [notes/walk-cost.md](../../notes/walk-cost.md) has the split, the method
and the caveats.

## PROPOSED: the wire changes that would close the rest

The remaining time is 1,259 requests at about 0.63 us each, two IPC round trips apiece. Reducing
either needs a change two programs agree on, so it is calef's call. Each option was prototyped and
measured; the note has the table and the terms.

- A. `OPEN` and `OPENDIR` take a relative path, resolved per component on the server. 0.60 ms.
- B. `OPEN`'s reply carries the size, so a whole-file read skips `FSTAT`. With A, 0.53 ms.
- D. The server narrows a subtree itself and the caretaker hop goes. With A and B, 0.35 ms,
  under Linux. A kernel question: a receive that says which capability a request came through.
- E, not built. A 64 KiB channel for `std`, which `std_runtime_protocol` fixes at one page.

Recommendation: A first. Nothing built waits on the answer. The milestone moves to BUILT when a
chosen option lands and the walk is at parity, or when calef rules parity out.

## Follow-on

- **Outstanding.** The decision above, and building whichever options it picks. Checked
  2026-09-26: no `design/decisions/` section covers a multi-component name, a size in `OPEN`'s
  reply, or server-side narrowing.
- **Recorded.** A cold walk on both sides, with Linux on ext4 over virtio rather than tmpfs, is a
  limitation in `notes/walk-cost.md`.

## BUGS

- `fs_walk` waits forever on a `std_exerciser` that aborts, because an aborting program never
  sends its end-of-stream marker (the kernel suite's `drain_sink` records the same gap).
- Under load the host sometimes runs the guest on an efficiency core, and every figure in the run
  moves about 1.55x at once. The ranges quoted are the fast mode, from three boots.
