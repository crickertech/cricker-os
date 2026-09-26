# A directory walk against Linux on the same machine

*(Milestone 606 (a directory walk costs what it does on Linux). Measured 2026-09-26 by lane
`milestone/606-walk-cost`. The number, the page's stem and every new name here are provisional;
the integrator mints the number and calef names things.)*

calef, 2026-09-26: "20x slower is unacceptable." This page is what the walk cost, where the time
went, what was fixed inside the file-service contract, and what closing the rest would take. The
last part is a set of wire changes, which are calef's call. They are written up below as a
proposal with a measured number for each option.

The walk is the one milestone 121 (`ripgrep` on nife: enumeration as a capability) priced:
`walk_pricing::walk` over `filesystem_protocol::fixture::walk`, 153 entries and 141 files, read
whole. [What a directory walk costs](walk-pricing.md) describes the
tree and the instrument.

## The numbers

Every figure is a warm walk, the median of five in one boot, on the same M-series host. nife is
`script/bench --real --release`: the release kernel under HVF, with `std_exerciser` holding the tree
behind an `fs_subtree_caretaker`. That grant is the one a confined `rg pattern src/` will hold.
Linux is `bench/host/run_linux_walk.sh`: the Alpine `virt` kernel on the same `virt,accel=hvf`
machine, running the same walker as PID 1.

| | whole walk | against Linux |
|---|---:|---:|
| nife before this milestone | 27.0 to 29.5 ms | about 68x |
| nife now, inside the current contract | 0.76 to 0.82 ms | about 1.9x |
| Linux (Alpine 3.20 `virt`), tmpfs, same machine | 0.41 to 0.42 ms | 1x |
| macOS, APFS, native (for reference only) | 1.7 ms | |

Three boots each, on a machine with a load average of 4 to 9 from other lanes. Under that load a
run sometimes lands on an efficiency core and reads about 1.55x slower across every figure at
once. Those runs are visible as a separate mode and are left out of the ranges above.

The split by `std::fs` call, from the same runs:

| | list (13 dirs) | open (141) | read (141) | close (141) |
|---|---:|---:|---:|---:|
| nife before | 7.6 ms | 11.5 ms | 8.2 ms | 0.18 ms |
| nife now | 89 us | 308 us | 333 us | 82 us |
| Linux | 81 us | 103 us | 172 us | 34 us |

Listing is now at parity. The rest is per request.

## Where the 28 ms went

The device, not the kernel. The debug kernel under the kernel suite took 41 ms and the release
kernel 28 ms, so optimization bought a third. The FS server's cache held 64 blocks and the
walk's metadata alone is 207. `redoxfs_server/examples/walk_replay.rs` replays the walk's
requests against the server core on the host and counts what reaches the disk: 415 device reads
per warm walk, each about 37 us through the block server under HVF. What was not device time was
RedoxFS answering every lookup by walking its node tree from the root, five blocks deep, hashing
each block. Replayed on the host with no device at all, that was 6.4 ms of server time per walk.

## What was fixed inside the contract

None of these changes a message.

1. The FS server remembers listings, names it found, and small files, until something changes
   the image (`redoxfs_server/src/memo.rs`). Every mutating verb goes through `change`, which
   forgets. A warm walk now reads nothing from the device, and `tests/walk_cost.rs` asserts that.
2. The block cache grew from 64 slots to 256. Slots are allocated as they fill and the header
   ring is never cached, so the crash test's 2 MiB servers still fit. After a namespace change
   forgets the memo, the next walk reads the device 2 times instead of 372.
3. `OPEN` and `CLOSE` stopped opening a RedoxFS transaction when the memo answered. A transaction
   clones the engine's allocator, and that clone was 80% of the server's remaining time.
4. The `std` PAL copies the shared page with `memcpy` instead of a volatile byte loop.

Server time per warm walk, replayed on the host: 6.4 ms before, 43 us after.

## Where the remaining 0.8 ms goes

Round trips. The walk sends 1,259 requests (`walk_model` counts them exactly): 225 `OPENDIR`,
26 `READDIR`, 141 `OPEN`, 141 `FSTAT`, 360 `READ` and 366 `CLOSE`. Each costs about 0.63 us, of
which the FS server's own work is 34 ns. The rest is two IPC round trips (client to caretaker,
caretaker to server) and the PAL. Linux answers each of the same calls with one trap and a
dentry or page cache hit.

Two things that look like levers are not:

- Dropping the size hint. `std::fs::read` asks `metadata()` for a size hint, which is one
  `FSTAT`. Without it `read_to_end` probes with a 32-byte read first. For the fixture's 16-byte
  files that saves a request; for any file over 32 bytes it costs one back. It would flatter this
  benchmark and slow a real source tree.
- Caching directory handles in the PAL. A handle held across calls keeps naming the directory
  after a rename, where a path should name what is there now. That is a semantic change, not an
  optimization.

## Proposed: what would close the gap

Status: PROPOSED. Each option changes something two programs agree on, so each is calef's
call. Each was built on an experimental branch and measured in the same session as the table
above. The branches were not kept: the numbers are the finding, and each prototype was a few
dozen lines.

| | requests | with the caretaker | no caretaker |
|---|---:|---:|---:|
| today (inside the contract) | 1,259 | 0.76 to 0.82 ms | 0.57 to 0.59 ms |
| A: a path in one `OPEN` or `OPENDIR` | 833 | 0.60 ms | 0.39 ms |
| A and B: `OPEN` also returns the size | 692 | 0.52 to 0.53 ms | 0.35 to 0.36 ms |
| Linux | | 0.41 to 0.42 ms | |

**A. A name may be a relative path.** `OPEN` and `OPENDIR` resolve `a/b/c` under the handle, one
component at a time on the server, each needing `DESCEND`. It removes an `OPENDIR` and a `CLOSE`
per component, 426 of the walk's requests. Today a name with a `/` is `EINVAL`, so an old server
refuses it loudly and a new PAL can fall back. Two things are owed before it could land. First,
`fs_nameset_caretaker` compares whole names, so it would refuse `d/x` where it allows `d` then
`x`; it must split on `/` and check the first component. Second, the rights on the minted handle
have to match what the per-hop walk gives. Today the PAL asks each hop for `DESCEND | READ`, so a
file opened for reading carries `READ` only. A server-side walk needs that request on the wire,
and `OPEN`'s second word is free for it.

**B. `OPEN`'s reply carries the file's size** in its second word, which is unused today. It saves
the `FSTAT` in front of every whole-file read. An old client ignores the word. The cost is a
semantic choice in the PAL, because `std` asks `metadata()` for the hint. Either the first
`metadata()` after an open answers with the size at open time, or the overlay patches
`std::fs`'s `buffer_capacity_required` to ask the PAL for a hint instead. The experiment did the
first.

**D. No relay process for a subtree grant.** The caretaker exists because the server's handle
table is per server, not per client, so narrowing needs a process in front. If the server could
receive on several endpoints and know which one a request came in on, it could bind each to a
directory handle and do the narrowing itself. That halves the IPC per request: 0.21 ms of this
walk today, and it takes A and B below Linux. It is a kernel question and the biggest of the three: a receive that says which capability a
request arrived through. §101 (notification objects) gave notifications a badge; this lane found
no equivalent for an IPC endpoint, which is a search, not a proof.

**E, not built. A wider channel for `std`.** `READ` already accepts 64 KiB, but a `std` program
maps one page at `FS_PAGE`, which `std_runtime_protocol` fixes, and the caretaker clamps to it.
The walk's three sized files take 80 `READ`s where they could take 6. In a real source tree most
files are over 4 KiB, so this matters more there than here. It changes the loader's mapping, which
the progenitor and the kernel harness share with the PAL.

The recommendation, for what it is worth to the decision: A first. It is the one with prior art
everywhere (`openat` resolves a path), it is backward compatible by refusal, and alone it takes a
quarter off. B is small and additive. D is the one that reaches parity, and it is a design fork in
the kernel, not in the file service.

What is blocked until this is answered: nothing already built. The milestone is BUILT on what the
contract allows, and parity waits on these.

## Reproduce

```text
script/bench --real --release --smp          # fs_walk, and the split as probes
sh bench/host/run_linux_walk.sh              # Linux, same machine, same walker
cargo run --release --manifest-path redoxfs_server/Cargo.toml --example walk_replay
```

`--smp` is the leg that builds `std_exerciser` and the FS server, and it takes the machine-wide
`nife-dev` link, as any gate does. `fs_walk` also runs on one hart if its `online_count` gate is
lifted, which is how the ranges above were taken; the figures agree with the fast mode of `--smp`.

## BUGS

- Linux is on tmpfs, not ext4 on virtio. Every timed walk is warm, and a warm walk on Linux is
  answered from the dentry and page caches whichever filesystem is under them. A cold ext4 walk
  is a different number, and `bench/host/run_linux_fs.sh`'s podman-made image is how to take it.
- Warm only, on both sides. A cold nife walk after boot reads the device 207 times.
- The memo forgets everything on any namespace change. `memo.rs` records why that is coarse
  on purpose and what a build that writes while it walks would pay.
- `fs_walk` waits forever on a program that aborts. It drains the program's stdout until the
  end-of-stream marker, which an aborting program never sends; the kernel suite's `drain_sink`
  records the same gap.
