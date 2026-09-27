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

**D. No relay process for a subtree grant.** Re-costed 2026-09-26 after milestone 599 (a frame per
filesystem client channel) landed badged endpoints under §230 (badged endpoint capabilities). The
server would narrow a grant itself, keyed on the badge `RECV_CAP` now returns, and the caretaker
process would go. Its own section follows, because it is as much about trust as speed.

## D in detail: the server narrows, keyed on the badge

### Is the premise true?

Half. The first version of this page said the server could not tell which endpoint a request came
through. Since milestone 599 it can: `rendezvous::BADGE` mints a badged copy of an endpoint
capability, and `RECV_CAP` returns the badge in x3. That part of the premise is now false.

The other half was that the caretaker is where confinement is enforced. That is only partly
true. The rights arithmetic is already the server's, with three Kani harnesses in `filesystem_protocol` (`attenuate_never_widens`,
`a_grandchild_is_bounded_by_the_root`, `a_root_carries_nothing_undefined`). What the subtree
caretaker adds is handle scoping. The server's handle table is one per server, and handle 0 is the
image root with every right. The caretaker keeps its own 64-slot table, so a client can only name
the handles its own caretaker minted, and its `ROOT` means the granted directory. D moves that
table into the server.

### What it would take

No kernel change. Every kernel piece exists: `BADGE`, the badge on `RECV_CAP`, `SEND_CAP` to hand
the badged copy over. `REPLY` cannot carry a capability, so the server cannot mint a grant and
return it. Whoever holds an unbadged endpoint capability with `GRANT` has to mint it: the
progenitor, or the kernel's wiring in the test harness. Then:

1. The minting holder opens the subtree with `OPENDIR`, as a caretaker does at startup today, and
   tells the server "badge `b` is rooted at this handle". That is a new verb, call it `BIND`, on
   the holder's own channel. `UNBIND` is its inverse.
2. The server keeps a small table from badge to grant: root handle, rights, and which window the
   badge reads. Today the badge is the window index (§230's pool of K = 8). Under D a badge names a
   grant, so the window becomes a column in that table, or both go into the badge as packed bits.
   That choice is a wire format.
3. Every handle records the badge that minted it. A request under badge `b` resolves `ROOT` to
   `b`'s root and refuses any handle `b` did not mint, with `EBADF`, the answer the caretaker gives
   today.

The wire formats are the two verbs' numbers and layout and the badge's meaning. Everything else is
code inside `redoxfs_server`.

### What it costs, measured

The IPC saving was measured by the no-caretaker prototype above: 0.57 to 0.59 ms against 0.76 to
0.82 ms today, and 0.35 ms with A and B, under Linux's 0.41 ms. That prototype used an unbadged
endpoint and the image root. What a badged version adds per request is one table index by badge
and one compare of a handle's owner. It was not prototyped: the server's whole per-request work in
`walk_replay` is 34 ns, and an array index and a compare are inside that noise. That is an
estimate, and it is the only figure in this section that is.

It also saves a process per grant.

### The trust tradeoff

Lines of code outside tests, blank lines and comments, counted 2026-09-26:

| | code lines | shares an address space with |
|---|---:|---|
| `fs_subtree_caretaker` | 111 | nothing; no heap, one stack page |
| `fs_nameset_caretaker` | 170 | nothing |
| `redoxfs_server`: serve loop (`bin/redoxfs_server.rs`) | 341 | the engine below |
| `redoxfs_server`: core and memo (`lib.rs`, `memo.rs`) | 836 | the engine below |
| vendored RedoxFS engine | about 8,900 | the server |

What a bug in each exposes:

- A handle-scoping bug in a caretaker lets its client use a server handle it did not mint. The
  first one it would find is handle 0, the image root with every right. So the blast radius is the
  whole image. That is also the blast radius of the same bug in the server under D.
- The difference is what else can reach the scoping state. Under D the handle table sits beside
  the RedoxFS engine, so a memory-safety bug in parsing a hostile image could corrupt it. That
  buys an attacker little: a compromised engine already holds the whole image, with or without a
  caretaker in front. The caretaker protects against clients, not against the server.
- What D gives up is the caretaker's case in §27 (the filesystem service). There, "it cannot
  reach a second file" is a property of the client's capability space: it holds nothing that
  names the FS server. Under D the client holds a badged endpoint to the server itself, and confinement
  becomes a branch the server is trusted to take. That was calef's reason, and it is the real
  cost.

What verification exists or is feasible:

- Today: the three Kani harnesses on the rights arithmetic, which both designs share. Neither
  caretaker's handle table has a harness.
- Under D the scoping is a pure function of (badge table, handle table, request), which could live
  in a host-testable crate with a Kani harness proving "a request under badge `b` resolves only
  handles `b` minted, and `ROOT` only to `b`'s root". Nothing stops the same harness being written
  for the caretaker's table today; neither side has one.

### Does D cover the name-set caretaker too?

It can, at a larger price. `fs_nameset_caretaker` filters every name-taking verb in the granted
directory against a set of names. Under D the server would hold each badge's name set as well and
apply `filesystem_protocol::nameset::contains` on those verbs. The set is variable-length, so
`BIND` would carry it through the channel page. That is a second, larger piece of policy moving into
the server, with no walk measurement behind it: glob grants are the shell's and are rarely on a
walk's path. D for subtrees alone is the smaller step.

### Who holds what, and revocation

- Today: the progenitor holds the FS endpoint; each caretaker holds a badged copy with its window;
  the client holds the caretaker's endpoint. Revoking a grant is ending the caretaker
  (`Holding::release`), which §41 (the endpoint is the broker) describes as taking the endpoint back.
- Under D: the progenitor holds an unbadged copy with `GRANT` and mints one badge per grant; the
  client holds the badged copy. There is no kernel method to pull a capability back out of another
  process's slots, today or under D. So revocation becomes `UNBIND`: the server closes every handle
  the badge minted and answers `EBADF` to it from then on. The client keeps a capability that names
  the server and does nothing. §41's "take it back" turns into "the server stops honouring it",
  which is weaker as a statement even when the effect is the same.

### What else was considered

- Keeping the caretaker and making the hop cheaper. The only lever is the IPC path, which milestone
  188 (the IPC fastpath) has already been over.
- The server minting badges itself and handing them out. Refused: `REPLY` carries no capability,
  so it would need a kernel change, and it would put delegation authority in the process D is
  already asking us to trust more.
- A receive over a set of endpoints, which §27 named as the other way to tell clients apart.
  Superseded by §230, which chose badges.

### Prior art

From memory, not read for this note: seL4 servers identify clients by badge. A 9P server keeps
fids per connection, which is D's per-badge handle table under another name.

### Reversibility, and who has acted

The server code is reversible. The verbs and the badge's meaning are not, once a progenitor mints
badges that way. Already acted on: milestone 599, which made the badge mean a window index, and
§230's text, "the caretaker is still what narrows a client to its subtree".

### If both cost the same

Performance picks D, and it is the only option that reaches Linux. The effort is not the argument
either way; the argument is §27's capability-space property against a measured 0.21 ms per walk.
That is calef's call, and this section does not recommend on it.

**E, not built. A wider channel for `std`.** `READ` already accepts 64 KiB, but a `std` program
maps one page at `FS_PAGE`, which `std_runtime_protocol` fixes, and the caretaker clamps to it.
The walk's three sized files take 80 `READ`s where they could take 6. In a real source tree most
files are over 4 KiB, so this matters more there than here. It changes the loader's mapping, which
the progenitor and the kernel harness share with the PAL.

The recommendation, for what it is worth to the decision: A first. It is the one with prior art
everywhere (`openat` resolves a path), it is backward compatible by refusal, and alone it takes a
quarter off. B is small and additive. D reaches parity and needs no kernel change since milestone
599, but it trades §27's confinement-by-capability-space for a check in the server, which the
section above lays out rather than decides.

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
