# `std` threads on nife

*Written 2026-10-10 (UTC) by milestone 812 (`std::thread::spawn` runs real threads in one address
space)'s lane. The kernel side is in `notes/processes.md`, `notes/futex.md` and
`notes/thread-pointer.md`; this note is the `std` PAL.*

## What a program needs, and where it gets it

Three slots of `std_runtime_protocol`, all of them the program's own:

| slot | capability | used for |
|---|---|---|
| 10 | its process, `BIND` only | `thread::spawn` joins each new thread to it |
| 11 | its first thread, `WRITE` | the main thread sets its own thread pointer |
| 12 | its address space, `READ` | the futex waits `std`'s locks make |

The progenitor gives all three to every `std` program it builds (`StdLayout`), through
`supervision_protocol`'s `CHILDS_OWN_PROCESS`, `CHILDS_OWN_THREAD` and `CHILDS_OWN_SPACE`
markers. Threads add no authority beyond the program's own memory and time: a thread's TCB, stack and
thread-local block all come from the heap's region.

A program without slot 10 cannot spawn (`Unsupported`, as before 812). Without slot 12 its locks yield
instead of sleeping, which is correct and slower.

## The pieces

- **`sys/thread/nife.rs`.** A spawn retypes a TCB from slot 0 above the fixed slots (`RETYPE_OBJ`'s
  floor), allocates a stack and a thread-local block from the heap, and calls `CONFIGURE` with the
  process capability and the block as the thread pointer. `CONFIGURE` answers the thread's
  current-CPU page, which goes in its block before `START`. A thread ends with `SYS_EXIT_THREAD`
  naming its packet's word. The kernel clears that word once the thread has left user mode, so a
  `join` that sees it cleared may free the stack.
- `sys/thread_local/key/nife.rs`. Thread-local keys, xous's shape. The thread pointer names the
  thread's block of key slots, whose first word is the block itself for `x86_64`'s `fs:[0]`. The
  main thread makes its block on first use, before any second thread exists.
- `sys/sync/futex/nife.rs`. `AddressSpace::WAIT` and `WAKE` on slot 12, under the futex locks
  `std` already has for Linux.
- `available_parallelism` reads the allowance on the thread's current-CPU page: the online core
  count today.

## How it is proved

`system_tests/src/user/std_threads_tests.rs` runs `std_exerciser/src/bin/std_threads.rs` on all three
architectures. Four threads add to an `AtomicU64` and a `Mutex<u64>` a million times each, every
thread keeps its own `thread_local!`, the space outlives the first thread to exit, and
`available_parallelism` is the online count.

## The heap under four threads

The block's item 12 asked for a measurement before any change to the heap's spinlock.
`std_heap_contention` does 200,000 small allocations and frees with one thread, then splits the same
work across four. Measured 2026-10-10 (UTC) under QEMU's TCG emulation, four emulated cores, by
`std_threads_tests::the_std_heap_is_measured_under_four_threads`:

| architecture | one thread | four threads | four against one |
|---|---|---|---|
| aarch64 | 63,442 us | 152,530 us | 2.4 times slower |
| riscv64 | 91,426 us | 189,922 us | 2.1 times slower |
| `x86_64` (two firmware legs) | 38,393 and 63,726 us | 123,778 and 132,899 us | 2.1 to 3.2 times slower |

Emulated time is a ratio to read, not a speed. The ratio says the lock is contended badly enough
that four threads do the same work slower than one: a thread preempted holding it makes the others
spin out their time slices.

## BUGS

- The heap's one spinlock makes allocation slower with four threads than with one (the table
  above). A lock that sleeps on the futex instead of spinning, or per-thread caches in front of
  it, is the replacement; measure it with the same program.

- A spawned thread's stack has no guard page. It is heap memory, so an overflow writes into the
  heap. A stack mapped from the region with an unmapped page below it is the fix.
- A detached thread's stack is never freed, because nothing remains to free it.
- A timed wait spins on `yield` until its deadline, because the kernel's `WAIT` has no timeout.
- One TCB page per spawn is spent for good, and 16 threads at most per process
  (`notes/processes.md`).
- The `rayon` half of the block's exit test is not built: it needs a dependency calef has not
  ruled on.
