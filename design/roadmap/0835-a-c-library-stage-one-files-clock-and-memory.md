---
status: NOT-STARTED
raised: 2026-10-08
milestone_dependencies: none
decision_dependencies: 31, 265
machine_requirements: none
specific_machine: none
needs_person: no
---
# 835. A C library, stage 1: files, clock and memory

*(Minted 2026-10-08 (UTC) by lane/c-library from calef's ruling the same day; number provisional
until the merge queue lands it. The file name is a lane's coinage.)*

calef chose relibc for nife's C library on 2026-10-08 (UTC), expecting "we end up building our own
with a similar approach". §265 (a C library started from relibc) records the ruling.
notes/c-library.md is the evaluation this stage is shaped by: take relibc's OS-neutral code and its
`Pal` seam once, into an in-tree crate nife owns, and write the nife platform layer in Rust. This
stage is the smallest library that runs a real C benchmark unmodified. It supersedes part of
milestone 478 (tier three: full POSIX behind the foreign-language seam).

## First consumers

Numbered as PR #1854 mints them, provisionally: ioping (834), SQLite's speedtest1 (831) and STREAM
(832). Each needs files or memory and a clock, and none needs threads, sockets or a second process.
STREAM's number means little until the userspace targets stop being softfloat (that is milestone 534
(the soft-float targets could now be flipped)); it can still run first, as the library's simplest
workload.

Reuse: relibc (MIT), chosen by calef on 2026-10-08 over musl and picolibc/newlib; its OS-neutral
header code and `Pal` seam are seeded, its Redox runtime, dynamic linker and dlmalloc are not
(notes/c-library.md has the measurement). The platform layer reuses the protocol crates `std` speaks
and `crates/user_mode_heap`.

## What it builds

- The seed: relibc's header modules a stage-1 consumer reaches, copied from a recorded relibc
  commit with its MIT notice and a provenance line per file. Nothing from `redox-rt`, `ld_so`,
  `platform/redox` or `platform/linux`. Each crate dependency kept is named in the `Reuse:` line
  with a reason, under §46 (thin primitives or whole subsystems).
- The platform layer, in Rust, against the protocol crates `std` already speaks:
  - files: `openat`, `close`, `read`, `write`, `pread`, `pwrite`, `lseek`, `fstat`, `fstatat`,
    `ftruncate`, `fsync`, `fdatasync`, `unlinkat`, `mkdirat`, `renameat`, `getdents`, `getcwd`, on
    slot 4's directory grant through `filesystem_protocol`, with a userspace descriptor table;
  - clock: `clock_gettime`, `gettimeofday`, `clock_getres`, `nanosleep`, from slot 5's clock page
    and the counter;
  - memory: anonymous `mmap` and `munmap` from slot 0's untyped budget, `malloc` on
    `user_mode_heap`, as rule 4 of §31 (the foreign-language seam) has it; `brk` refused;
  - process: `exit`, `getrandom` on slot 6, `uname`, stdout and stderr on slot 1, and fixed answers
    for the identity calls, each in `BUGS`.
- `fork` returns `ENOSYS`. No C translation unit contains a syscall instruction, and `syscall()` is
  not built (§31 rule 1 as amended by §265).
- All three architectures (§19 (architectural parity is a tenet)), proven by the same test.

## What this stage decides in its own block

Where the crate lives, and its name (provisional until calef rules). Then whether cbindgen generates
the headers or they are written by hand, how `user/build.rs`'s clang finds them, and the startup
path (`crt0`, static TLS for one thread) that replaces relibc's dynamic linker.

## Done when

ioping and SQLite's speedtest1 build from their unmodified upstream sources against this library
and run to completion on nife under QEMU on all three architectures, and a host test proves the
platform layer's `errno` mapping. STREAM building and running counts too; its number waits on 534.

## BUGS

- No signals. `sigaction` records a handler that never fires. Stage-1 consumers are checked for
  depending on one.
- SQLite runs without file locking (`unix-none`), which is safe only with one process on the file.

## Index row

The first C library that runs somebody else's C program unmodified on nife: relibc's OS-neutral
code, seeded once, over a Rust platform layer that holds the process's file, clock and memory
capabilities, so the C itself makes no syscalls.
