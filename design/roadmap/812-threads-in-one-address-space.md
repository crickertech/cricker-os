---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: 16, 19, 105
machine_requirements: none
specific_machine: none
needs_person: no
---
# 812. `std::thread::spawn` runs real threads in one address space

*(Minted 2026-10-07 (UTC) by lane/filesystem-crate-probe from fork 5 of pull request #1803. The
number is provisional until the merge queue lands it; the title and slug are drafts. Not yet
scheduled: calef ruled that it should exist, and that its first customer, read-write ZFS, is not the
current priority.)*

calef reopened DECISIONS §105 (`std::thread::spawn` stays declined, until a customer needs it) on
2026-10-07 (UTC). He refused read-only ZFS, *"Read only isn't interesting to me. Read write is the
point."*, and the only realistic read-write path is OpenZFS's own `libzpool`, which needs threads in
one process. Asked whether that reopens §105: *"Yes. However read-write ZFS just isn't the current
priority."* This block is §105's option A, as `notes/thread-spawn-fork.md` costed it.

## What exists, checked 2026-10-07 (UTC)

- `std::thread::spawn` returns `Unsupported` unconditionally
  (`patches/std-nife/overlay/std/src/sys/thread/nife.rs`).
- All three `targets/*-unknown-nife.json` set `"singlethread": true`, which routes `std` to its
  `no_threads` locks and a single static for thread-local storage.
- `Tcb::CONFIGURE` consumes the address-space capability it binds, and `kernel/src/thread.rs` owns
  that `AddressSpace` outright so the reaper's drop frees it. No two TCBs can share one today.
- `rayon`, `crossbeam-channel`, `tokio` and `ignore` already compile and link against the PAL. They
  fail at the first spawn, at run time.

Reuse: the kernel half is ours to write, since §46 (thin primitives or whole subsystems; we write everything in between) has the kernel written here. Its design
is seL4's, where `TCB_SetSpace` binds a VSpace without consuming it. The `std` half reuses std's
own threaded `sys` code for every platform shape it can, and writes only the nife arm of the PAL.

## The work

Kernel, on all three architectures:

1. An address-space capability can be bound to a TCB without being consumed. This changes what an
   existing method promises, so it is a syscall-surface change and owes a `design/decisions/`
   section ruled by calef before code. Whether the change is a new rights bit or a second
   `CONFIGURE` form is part of that ruling.
2. `AddressSpace` gains multi-holder liveness, the shape `Endpoint` already has under §16 (object
   revocation): teardown waits for the last TCB, not the first.
3. The reap path (`reap_region_objects`) learns "still referenced" beside live and dead.
4. Whether sibling threads share one `CapabilityTable` is the same fork one level down. seL4 answers
   it the same way, and the section in step 1 should rule both together.

`std`, in the overlay:

5. `Thread::new`, `join`, and thread exit, on the kernel's TCB methods.
6. Thread-local storage per thread, set up by the PAL on each architecture's thread pointer
   (`TPIDR_EL0`, `tp`, `FS` base).
7. Real locks: park and unpark on a kernel primitive that blocks one thread and wakes another, so
   `std::sync` stops using its `no_threads` variants.
8. `"singlethread": false` in all three target specifications, landed with step 7 and never
   before it, or every lock in `std` silently becomes a no-op under contention.

Weak memory ordering is the point of the exercise on aarch64 and riscv64, so every lock and the
TLS setup carries its ordering argument in a comment, per the codebase rule.

## Exit

A test in `script/test`'s suite runs on aarch64, riscv64 and x86_64 and fails if any step is
missing. A program spawns four threads that each increment one shared `AtomicU64` and one
`Mutex<u64>` a million times, joins them, and checks both read four million. It also checks each
thread saw its own thread-local value, and that the address space outlived the first thread to
exit. A second check runs `rayon`'s parallel sum to the same answer.

## What it unlocks

- Read-write ZFS through OpenZFS's `libzpool`, hosted as a confined CDDL program. A later
  milestone, not prioritized; see notes/filesystem-crates-2026-10-07.md.
- Read-write btrfs through LKL (Linux Kernel Library), recorded as an alternative in the same note.
- `tough`, the TUF client pull request #1805 names, whose `tokio` `fs` feature runs file I/O on a
  blocking thread pool.
- `ripgrep`'s parallel directory walk; milestone 121 (`ripgrep` on nife: enumeration as a capability) is single-threaded today.
- Parallel `rustc` and `cargo` for milestone 173 (`rustc`/`cargo`/LLVM natively on nife), which today
  plans on the single-threaded fallback.
- The Rayon-parallel variants of milestone 149 (a common HPC benchmark suite, in Rust).

## BUGS

- Not yet scheduled. It has no customer on the current path, by calef's own ruling.
- The costing in `notes/thread-spawn-fork.md` is from 2026-08-22; the kernel has changed since,
  and the first act of a lane here is to re-check its file references.

## Index row

Real shared-memory threads: `std::thread::spawn` on all three architectures, by letting several
TCBs bind one address space (§105's option A, reopened 2026-10-07). It unlocks read-write ZFS
through `libzpool`, LKL's btrfs, `tough`, parallel `rg` and parallel builds. Not yet scheduled.
