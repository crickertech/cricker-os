# Rust `std` on the native ABI

*(Milestone 27. The first wall an application hits on nife was "no std": you could write a
`no_std` binary against `crates/user_mode_runtime`, and nothing else. This milestone makes ordinary Rust,
`Vec` and `String` and `println!` and `Instant`, compile and run on the capability ABI. See
DECISIONS §22 for the decision and why; notes/abi.md for the ABI it binds to.)*

The shape is Hermit's, not Redox's. Hermit implements std's platform layer directly on a
non-POSIX unikernel ABI; Redox writes a POSIX C library (relibc) first and puts std on top of that.
We took the native road: there is no errno, no fd table, no `open`, no `fork` under our `sys`
backend, because the OS does not have them and std does not actually need them to run a workload
that stays off files and sockets. That is the whole point of having done the native ABI first
(DECISIONS §14, §15): std widens "runs real workloads" from hand-built `no_std` binaries to most of
crates.io, without smuggling in the POSIX assumptions the ABI deliberately excludes.

## What a std program is given

A std program is an ordinary nife ELF (notes/abi.md §3): entered at `_start`, linked at
`0x6000_0000` (notes/address-space-map.md), capability table populated by its parent. std's runtime contract needs two things, and the ABI's
out-of-band convention (notes/abi.md §4) grants them at fixed slots:

- slot 0: an untyped budget. The global allocator draws heap pages from it lazily via
  `untyped::MAP`, one page per invoke, at `0x4000_0000`. This is the same untyped-backed heap the
  `allocator_exerciser` workload proved (`crates/user_mode_heap` algorithm, host-tested), restated inside std because
  std cannot depend on an out-of-tree crate.
- slot 1: an endpoint with WRITE. `stdout` and `stderr` SEND here, 16 bytes per message (w0 =
  byte count, w1|w2 = the bytes, little-endian). std's own `LineWriter` batches user writes; the
  receiver reassembles.

Three more slots exist, and a program holds each only if it was *given* the thing behind it
(milestone 27 phase two, the `std::net` and `std::fs` bindings below):

- slot 2: a `Stack` endpoint with WRITE. `std::net` speaks net_stack's socket contract over it.
- slot 3: an untyped budget the net PAL mints each socket's shared frame from.
- slot 4: an FS-service endpoint with WRITE, which *is* a directory capability, plus the page it
  shares with the FS server mapped at `0x1100_0000`. `std::fs` speaks the §27 file contract over it.
- slot 5: a `Frame` capability naming the clock page, with `READ`, plus a read-only mapping of it
  at `0x1200_0000`. `SystemTime::now()` is the offset it finds there plus the ambient counter
  (milestone 51, §43).
- **slot 6: the entropy service's request endpoint**, with WRITE (milestone 56, §44). It means "you
  may obtain randomness" and names no device; there is no mapping alongside it, because randomness is
  obtained by asking rather than by reading. `std::random::SystemRng` is a `CALL` on it.

A program that never allocates, prints, opens a socket, or opens a file never touches the slots it
does not use. The absence of slots 2 and 3 is exactly what "no ambient network" feels like from
inside a process, and the absence of slot 4 is "no ambient filesystem": each returns `Unsupported`
because there is no capability to reach, not because the code was compiled out. A program can hold
one and not the other, so the slots do not fill contiguously; notes/abi.md §4 records how the
kernel-side wiring places slot 4 while leaving 2 and 3 empty, and why the gap matters.

## The PAL surface, and what each piece binds to

The backend lives in `patches/std-nife/overlay/std/src/sys/` and is materialized into a patched
std by `cargo xtask std-src`. Each file binds one std concept to the ABI:

| std concept | nife binding |
|---|---|
| `GlobalAlloc` | `untyped::MAP` from slot 0, grow-on-demand (`sys/alloc/nife`) |
| `stdout` / `stderr` | `endpoint::SEND` on slot 1 (`sys/stdio/nife.rs`) |
| `Instant`, `SystemTime` | the virtual counter, `CNTVCT_EL0` / `rdtime` (`sys/time/nife.rs`) |
| `panic!` | print, then `brk`/`ebreak`: a fault the kernel attributes. No unwinding. |
| `thread::spawn` | `Unsupported` in phase one; `sleep`/`yield` are real |
| `net` (`TcpStream`, outbound `UdpSocket`) | net_stack's socket contract on slots 2/3 (`sys/net/connection/nife.rs`), or `Unsupported` when not granted |
| `fs` (`File`, `metadata`, `read`/`write`) | the FS service's file contract on slot 4 (`sys/fs/nife.rs`), or `Unsupported` when no directory was granted |
| `std::random::SystemRng` | the entropy service's endpoint on slot 6 (`sys/random/nife.rs`), or a **panic** when not granted |
| `HashMap` seed | the same service when granted; splitmix64 from the counter when not, and labelled |
| `std::env::consts::OS` | `"nife"` (patched into `env_consts.rs`) |
| `std::env::var` / `vars` / `set_var` | a **process-local table** (`sys/env/nife.rs`), seeded only with `TZ`/`LANG`/`TERM` from a granted inert-configuration page (slot 7, milestone 47, DECISIONS §111) if one exists; otherwise empty at start |
| `std::env::temp_dir` / `split_paths` / `join_paths` | `TMPDIR` or `/`, and a `:`-separated list (`sys/paths/nife.rs`) |
| `std::env::current_dir` | `/`, the root of this process's own namespace (milestone 47); `Unsupported` when it holds no directory. `current_exe` and `chdir` refuse, `home_dir` is `None` |
| `std::process::id` | `0`, because this system issues no process identifier (`sys/process/nife.rs`); everything else in `std::process` refuses |

The syscall glue (`sys/pal/nife/rt.rs`) is a deliberate twin of `crates/user_mode_runtime`: the same
`svc`/`ecall` wrappers, restated because std cannot depend on the crate. The ABI constants are
not restated: `abi.rs` is generated verbatim from `crates/abi` by `std-src`, so the numbers cannot
drift. Likewise `user_mode_heap.rs` from `crates/user_mode_heap` (the host-tested heap algorithm is the only heap
algorithm), `netproto.rs` from `crates/socket_protocol/src/lib.rs`, and `fsproto.rs` from `crates/filesystem_protocol`: every
wire format the PAL speaks has exactly one definition, and it lives with the server that answers it.
The slot numbers and page addresses themselves (`rt::FS_DIR_SLOT`, `rt::CLOCK_PAGE` and the rest)
come the same way, as `runtimeproto.rs` from `crates/std_runtime_protocol`, since milestone 595
(provisional) made the progenitor a loader of `std` programs too.

## The toolchain: build-std against a patched rust-src

There is no crate to adopt; the deliverable IS the PAL, plus the machinery to build it. Rust's
`-Zbuild-std` compiles std from source, and it finds that source in the sysroot of the rustc it
invokes. So a patched std means a toolchain whose sysroot is patched. `cargo xtask std-src`
builds one:

1. Hardlink-clone the real nightly (`cp -al` of `bin` and `lib`). Blocks are shared, so the
   clone costs almost no disk. rustc resolves *this* directory as its sysroot (it derives the
   sysroot from the location of `librustc_driver`, which the clone puts inside the farm; a symlink
   farm does not work, because the symlink resolves back to the real toolchain, which was the first
   thing tried and measured).
2. Replace the `src` subtree with a real copy (independent inodes), so patching it never
   touches the shared rustup toolchain.
3. Patch that copy: drop in the overlay PAL files, generate `abi.rs`/`user_mode_heap.rs`, and insert a
   `target_os = "nife"` arm into std's `cfg_select!` dispatchers (pal, alloc, stdio, random,
   thread, time, io/error, thread_local storage and guard) plus `env_consts` and the
   `restricted_std` chain in std's `build.rs`.
4. Link it as the `nife-dev` toolchain (`rustup toolchain link`).

`cargo xtask std-exerciser` then builds the `std_exerciser` demo for both custom targets against it. The build
sets `RUSTUP_TOOLCHAIN=nife-dev` explicitly rather than `+nife-dev`, because the cargo proxy
that launched xtask already exports `RUSTUP_TOOLCHAIN=nightly`, which would override a `+` selector
and silently build std from the *unpatched* sysroot.

`std-src` is idempotent: a stamp of all inputs (the toolchain version, the ABI/heap crates, the
target specs, every overlay file, and a patch-logic version) guards the rebuild, so a warm farm and
its build-std cache survive across runs and only a PAL change forces std to recompile.

### `nife-dev` is global to the machine, and the stamp does not guard it

The farm is per-worktree; the name is not. `rustup toolchain link` writes one symlink under
`$RUSTUP_HOME/toolchains` for the whole user account, so `nife-dev` means whichever worktree ran
`std-src` last, while every build downstream resolves std through that name rather than through a
path. Two agent lanes gating at once therefore contend for it, and the loser does not fail: it
compiles against a farm inside somebody else's worktree.

A warm stamp used to be enough to skip the link entirely, which is what made the failure silent.
The stamp answers *is this worktree's farm built*, and nothing was asking *does `nife-dev` still
mean it*. On 2026-08-18 lane `55-durability` relinked mid-run and lane `64-more`'s `std_exerciser`
built against 55's farm; it was caught by a person reading the `Compiling std` path out of the build
output, and nothing else would have caught it. AGENTS.md had warned about this shape in prose since
2026-08-01 and the warning is rung four, which is exactly as much as it turned out to be worth.

`std_src` now verifies the link on the warm path and relinks, loudly, when it points elsewhere.
Relink rather than refuse, because the lane calling it is about to build and needs the name to mean
its own farm; taking the link is what every lane already does by design. What changed is that the
theft is deliberate and printed, so a foreign `Compiling std` path cannot happen without a line
above it naming who took what. It also fixes the dangling case AGENTS.md describes, where a pruned
worktree left `nife-dev` pointing at nothing and unrelated builds failed far from the cause with
`override toolchain 'nife-dev' is not installed`.

**Telling a lane not to take the link was never possible**, which milestone 57's lane established on
2026-08-01 by reading the code rather than by failing. `script/test` calls `std_src()` transitively
and a fresh worktree always has a cold farm, so any lane that runs the gate takes the
account-wide name. `AGENTS.md` had at that point given two instructions that could not both be
obeyed: gate before reporting, and do not run `xtask std-src`. The honest rule that replaced them is
the integrator's, and is all that `AGENTS.md` still carries: expect every lane to take it, and
relink from the main checkout at merge.

The workaround worth knowing, from the same lane: symlink the worktree's `target/nife-farm` at
the main checkout's farm once `cargo xtask std-stamp` shows the stamps match, and `std_src()`
early-returns instead of rebuilding a second copy.

This does not make concurrent lanes safe, and must not be read that way. It makes the loss
visible and self-healing at the next call. A lane whose build is already in flight when another
relinks still loses; the honest fix is a per-worktree toolchain name, which nobody has priced.

### The target specs

`targets/{aarch64,riscv64,x86_64}-unknown-nife.json`, built with `-Zbuild-std` and
`-Zjson-target-spec`. x86_64 joined at milestone 184.
The load-bearing fields:

- `"os": "nife"` selects our `sys` backend through every dispatcher.
- `"panic-strategy": "abort"` means unwinding machinery is never even linked; `panic!` prints and
  faults.
- `"singlethread": true` turns off `target_has_threads`, so std uses its `no_threads` sync
  primitives and single-`static` TLS. This is honest for phase one (one thread of execution per
  process, `thread::spawn` is `Unsupported`); it flips off when real threads arrive.
- softfloat (aarch64 `-neon`, riscv `lp64`, x86_64 `-mmx,-sse...,+soft-float` with
  `"rustc-abi": "softfloat"`) matches EL0/U-mode/ring 3 with no FP save area, the same choice the
  `no_std` programs make. On x86_64 this is a correctness requirement, not a preference:
  `kernel/src/arch/x86_64/` saves no FPU or SSE state on a context switch, so a std program that let
  LLVM emit SSE would have its `xmm` registers overwritten by whichever thread ran next.

The reasoning of milestone 184 (extend the `std` port to x86_64) for each field, the probe that
shows no SSE leaked in, and the entry stub that fixes the stack offset are in [the x86_64 spec
appendix](std/x86-64-target.md).

## `std::net` and `std::fs`

Both are clients of a frozen contract and nothing more. Their wire constants are generated verbatim
by `std-src` (`netproto.rs` and `fsproto.rs`), so the PAL's numbers cannot drift from the server's.

- `std::net` binds `TcpStream`, `TcpListener` and outbound `UdpSocket` to net_stack's socket
  contract on slots 2 and 3, under §25 (socket identity). Each socket gets its own shared frame; a listener gets
  none. Errors map by meaning, not by errno: a refused connect is `ConnectionRefused`, and a port
  outside the listen grant is `PermissionDenied`. The full mapping is in
  [the net appendix](std/net.md).
- `std::fs` binds `File` and the namespace verbs to the FS server's file contract on slot 4,
  under §27 (the filesystem service). A path is resolved under the one directory this process was granted, so `/motd`
  and `motd` are one file. Any `..` is refused with `InvalidFilename`, because no capability
  designates what is above the grant. A nested path is a chain of `OPENDIR` descents
  since milestone 122 (a directory handle `std` can hold). The mapping is in [the fs appendix](std/fs.md), the walk and `std::fs::Dir` are in
  [the descent appendix](std/fs-descent.md), and file times are in
  [their own appendix](std/file-times.md).

A program granted neither gets `Unsupported` from both, because there is no capability to reach.

## What still ends a nife process

The dangerous std call is not the one that returns `Unsupported`. It is the one that compiles and
then kills the process. Milestone 64 (enough `std` to run somebody else's crate) found five on
2026-08-18: `env::vars`, `env::temp_dir`, `env::split_paths`, `process::id` and `process::exit`. All
five now answer or exit cleanly. A gap list built from `Unsupported` answers could never show them,
so the reading became a check. It greps the `sys/` sources the compiler actually built for bodies
that end a process. Each hit is compared against `ABORTS_ACCEPTED` in `xtask/src/farm.rs`, which
carries a reason per entry.

```sh
cargo xtask std-aborts       # on its own, against whatever the farm last built
script/test                  # runs it too: `std-exerciser` ends with it
```

The five calls, how the check works and its own BUGS are in
[the std-aborts appendix](std/std-aborts.md).

## EXAMPLES

Build the patched toolchain and the demo, then check that this worktree's farm matches the main
checkout's before borrowing it:

```sh
cargo xtask std-src           # patch rust-src and link it as nife-dev
cargo xtask std-exerciser     # build std_exerciser for the nife targets against it
cargo xtask std-stamp         # run in both checkouts; equal output means the farms match
```

Put `nife-dev` back at the main checkout's farm after a merge, and read it back:

```sh
cd "$(git rev-parse --path-format=absolute --git-common-dir)/.."
rustup toolchain link nife-dev "$(pwd)/target/nife-farm"
rustup toolchain list -v | grep nife-dev
```

What a program granted a directory sees, as `std_exerciser` runs it:

```rust
let text = std::fs::read_to_string("/motd")?;            // the same file as "motd"
let err = std::fs::File::open("../motd").unwrap_err();
assert_eq!(err.kind(), std::io::ErrorKind::InvalidFilename); // nothing is above the grant
std::fs::write("f", b"x")?;                                 // CREATE or TRUNCATE, then WRITE
```

And a clean abort sweep:

```
$ cargo xtask std-aborts
std-aborts: 26 process-ending bodies across 79 compiled std sources, all accounted for
```

## The proof

`std_exerciser` is an ordinary Rust program with no `no_std` and no `unsafe`. It is one binary with
three behaviours, chosen by the authority it was granted. With a directory it walks the `std::fs`
surface; with the network it runs a UDP DNS query and a TCP echo; with neither it runs `Vec`,
`String`, `HashMap` and `Instant` and checks that `fs` and `net` refuse. Three kernel tests spawn it
and compare its output byte for byte on both ISAs. What each branch asserts is in
[the proof appendix](std/the-proof.md).

## BUGS

One line each. The full entry, with its reasoning and history, is in
[the caveats appendix](std/caveats.md) unless another link is given.

- `thread::spawn` returns `Unsupported`, and `Condvar::wait` and `Once::wait` end the process. Both
  wait on milestone 64's `thread::spawn` fork.
- `std::process::exit` exits cleanly, but the kernel drops the code. A supervisor can tell exit from
  crash and cannot tell `exit(0)` from `exit(1)`. Carrying it is a wire-format change.
- `SystemTime::now()` and `std::random` panic when the process holds no clock or entropy grant,
  because neither call has an error channel. `HashMap`'s seed is the one caller that degrades.
- The environment holds only what a grant seeded (`TZ`, `LANG`, `TERM`) plus what the program sets.
  `current_exe` and `chdir` refuse, `home_dir` is `None`, and `process::id()` is `0`.
- `net` has no peer address on `accept` (it reads `0.0.0.0:0`), no non-blocking mode or timeouts, no
  DNS through `lookup_host`, and no IPv6. A listener's backlog is one connection. A socket's local
  port follows its id, so fast churn can reuse a port slirp has not cleared
  ([the net appendix](std/net.md)).
- `fs` has no `canonicalize`, links, permissions, locks or `File::duplicate`. A revoked FS endpoint
  reads as `PermissionDenied`, because the wire's errno space overlaps the kernel's invoke errors
  ([the fs appendix](std/fs.md)). The rights discipline is tested only under a full-rights grant
  ([the descent appendix](std/fs-descent.md)).
- A file written on nife reads as early 1970, since the FS server holds no clock. A write never moves
  a real timestamp, times are whole seconds, and the handle-shaped time calls refuse
  ([file times](std/file-times.md)).
- stdout and stderr share one endpoint, so they interleave by 16-byte chunk.
- The `std-src` patches are string-anchored to the pinned nightly. A bump that reshapes a dispatcher
  fails with "anchor not found", which is the intended tripwire.
- `nife-dev` is one name for the whole account. Relinking loudly makes a stolen link visible, and a
  lane whose build is already in flight when another relinks still loses (above).
- `std-aborts` covers `sys/` only, and proves a body reachable, never a call. A stale or foreign
  build under `std_exerciser/target` can be reported as a source defect, or fail inside the unpatched
  std; the recovery for both is `rm -rf std_exerciser/target`
  ([the std-aborts appendix](std/std-aborts.md#bugs)). The name `std-aborts` is provisional.

## Appendices

| appendix | what it holds |
|---|---|
| [x86-64-target](std/x86-64-target.md) | the x86_64 spec field by field, and the SSE probe |
| [net](std/net.md) | `std::net` over the socket contract |
| [fs](std/fs.md) | `std::fs` over the file contract, and what stays `Unsupported` |
| [file-times](std/file-times.md) | `modified` and `set_times`, with their own BUGS |
| [fs-descent](std/fs-descent.md) | nested paths, `std::fs::Dir`, and the write-path correction |
| [caveats](std/caveats.md) | the full caveat list behind BUGS above |
| [std-aborts](std/std-aborts.md) | the calls that ended a process, and the check that finds them |
| [the-proof](std/the-proof.md) | what `std_exerciser` asserts under each grant |
