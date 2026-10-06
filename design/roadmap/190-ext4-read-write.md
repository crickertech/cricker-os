---
status: NOT-STARTED
raised: 2026-08-30
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 190. ext4, read and write: a Rust implementation with libext2fs as the host-side oracle

Minted 2026-08-30 by calef, from a session that costed the entire option
space before any code was written. *(Number provisional until the merge queue lands it, per
AGENTS.md's rule that anything global to the tree stays provisional until the queue arbitrates.)*

Phase 1 needs nothing that does not exist, which is what the gate records. Phases 3
and 4 carry forks named below, and phase 4's is a real one; a single gate on the front of an arc this
long would block the part that is buildable today.

In brief. calef, 2026-08-30: *"I've invested heavily in an ext4 ecosystem so nife needs to
support it."* This is a standing requirement with no deadline, which is a stronger position than
it sounds: it is the condition under which the right implementation can be built instead of the fast
one. The arc is four phases, each independently useful, ending in a Rust ext4 that reads and writes
drives Linux formatted, verified against libext2fs and against Linux itself rather than against our
own confidence.

This is milestone 140's (mount a drive this system did not create) ext4 row, promoted from "some
day" to a sequenced arc. It does not change 140's ordering for FAT32 or ext2, and it does not touch
RedoxFS.

## Why the requirement exists, and why it is not urgent

The requirement is calef's own infrastructure, and the history matters because it is what retired
the urgency:

- The family's backup solution is borg over SSH, on cordoba, plus Immich for images. Both were
  built in another session with the existing Linux ecosystem, because the data problem was pressing
  and nife was not going to be ready in time.
- The drives holding those borg repositories are ext4, formatted by Linux. They are not going to
  be reformatted, so nife supporting them means supporting the format as it exists on those disks.
- Time Machine and SMB are no longer the backup path. Milestone 55 (Time Machine: SMB3 with Apple's
  extensions) and the surrounding blocks are aimed at a workload the customer retired; that
  repricing is its own edit and is not this block's scope, but this block should not be read as
  reinstating it.

What that leaves is a durable requirement with no deadline, which is exactly the condition
AGENTS.md's *elegance and performance beat implementation convenience* tenet was written for: an
argument from implementation cost is the weakest one available here, and it is weakest of all when
nothing is waiting on the answer.

## The survey, which is the research this block exists to preserve

Every candidate for read/write ext4, in any language, as of 2026-08-30:

| candidate | language | writes | writes the journal | `metadata_csum` on write | needs shared-memory threads | licence |
|---|---|---|---|---|---|---|
| `ext4-view` | Rust, `no_std` | no, explicit non-goal | n/a | n/a | no | MIT OR Apache-2.0 |
| `ext4plus` | Rust, `no_std` | yes | no, and it tells you to disable journaling | **read-only** | no | MIT OR Apache-2.0 |
| `libext2fs` (e2fsprogs) | C | yes, mature | **no**, replay only | yes | no | LGPLv2 |
| `lwext4` | C | yes | no | yes | no | BSD-3 except two GPLv2 files |
| LKL (Linux Kernel Library) | C | yes, the reference | **yes**, real jbd2 | yes | **yes** | GPLv2 |
| write our own | Rust | eventually | eventually | eventually | no | ours |

### Two facts decided most of it

**1. `metadata_csum` is not optional.** It has been mke2fs's default for years, so a drive cordoba
formatted has it. `ext4plus` reads it and cannot update it on write, which means writing to calef's
actual repositories with it would produce metadata Linux flags as corrupt. That is a concrete
disqualification for this requirement rather than a judgement about maturity.

2. No userspace ext4 implementation writes the journal. libext2fs replays a journal it finds and
never writes one, and fuse2fs, its own reference read/write client, says so in its manual:

> Warning: fuse2fs does not support using the journal. There may be file system corruption or data
> loss if the file system is not gracefully unmounted.
>
> -- fuse2fs(1), https://man7.org/linux/man-pages/man1/fuse2fs.1.html

Reading its source confirms it: there are no transaction start or commit calls. Only LKL writes a
real jbd2 journal, because only LKL is Linux. That single fact is why LKL kept resurfacing in the
discussion, and it is why the journal question has to be answered on its own rather than assumed
away.

## The refusals, each with its reason

The refusals are the valuable half of this block, and each is recorded so a later reader can
disagree with an argument rather than rediscover it.

`lwext4`, refused on licence and on the seam. It is the obvious embedded C candidate (its own
blockdev abstraction, `malloc` plus string functions, tier two of milestone 36's (the foreign
component) libc tiers). But `ext4_extents.c` and `ext4_xattr.c` are GPLv2, which makes the library
GPLv2 as distributed, and extents are what make ext4 ext4, so the GPL file cannot be dropped.
`deny.toml`'s allow-list is permissive-only and says why in its own comment: this is a kernel people
are meant to be able to vendor. Separately, its blockdev abstraction is function pointers the Rust
side would implement, which is a callback into Rust that DECISIONS §31 (the foreign-language seam)
rule 2 forbids today. Either objection alone is sufficient.

`ext4plus`, refused on `metadata_csum`. See above. Its own README also recommends ramdisks for
writing and warns of known bugs, and it is pre-0.1.0. Worth watching; it is the closest thing to
what phase 3 wants to be, and adopting or contributing to it is a live option that this block does
not foreclose.

LKL, refused for now on the syscall surface. `lkl_host_operations` requires `thread_create`,
`thread_join`, `thread_exit`, `thread_self`, `thread_stack`, four TLS operations, semaphores and
recursive mutexes, because LKL is a kernel linked into your address space with kthreads, workqueues
and softirq contexts. nife cannot do that: `Tcb::CONFIGURE` consumes the address-space
capability it binds, so no two TCBs share an address space, which is exactly what DECISIONS §105
(`std::thread::spawn` stays declined) recorded and declined. Building Option A because a filesystem
wants it is a syscall-surface change entered sideways, and AGENTS.md's *move fast on what can be
undone* tenet puts the syscall surface in the expensive category.

Three further costs, recorded so a later reader is not surprised: the rest of the host-ops surface
(`jmp_buf_set`/`longjmp` in assembly for three ISAs, a timer service with callbacks, `ioremap` and
`iomem_access` for a virtio-mmio shim onto the block server); GPLv2 and therefore vendoring Linux;
and, most likely to be fatal, `arch/lkl`'s documented hosts are POSIX and Windows userspace, so
whether it builds for a freestanding non-x86 target at all is unverified and DECISIONS §19
(architectural parity is a tenet) requires all three.

**LKL is not refused forever.** If §105 Option A is ever built because shared-memory threading is
right on its own merits, LKL becomes a genuinely exciting lane that unlocks far more than ext4, and
it is the only path to a real journal. The refusal is about what may force that decision, not about
the destination.

libext2fs as a shipped dependency, refused, and the honest reason is recorded. This was the
session's first recommendation and it was substantially an argument from effort. Applying the
tenet's own test, *would I still choose it if both options cost the same?*, the answer is no. A Rust
implementation wins on memory safety over a parser eating bytes from drives we did not write, on
being host-testable and Kani-reachable, on needing no libc tier, on carrying no LGPL into the image,
on parity across three ISAs by construction rather than by luck, and DECISIONS §83 (when the same
thing exists in C and in Rust, take the Rust one) says so directly. The deadline was doing the work
in that recommendation, and the deadline is gone. DECISIONS §34's (RedoxFS as the primary store) own
objection to littlefs applies too: it would put a foreign component in the storage path.

Writing our own with no reference, refused. DECISIONS §46 (thin primitives or whole subsystems)
rule 4 prefers depending where correctness is won by exposure rather than by reading a
specification, and a filesystem's on-disk format is the exposure case. A young implementation of a
hostile-input parser with nothing to check it against is not trustworthy, whatever language it is
in. That objection is real, and phase 2 exists to answer it.

## The decision: Rust on the target, C on the host as the oracle

**Generalized 2026-08-30 into DECISIONS §136 (a mature foreign implementation earns its place as an
oracle, not as a dependency)**, at calef's direction, because the position is not about filesystems
and was discoverable only by reading this block. What follows is the ext4 application of it; §136
carries the argument, what it refuses, and what it does not license.

**libext2fs's real value here is as an oracle, not as a dependency.** It runs on the host, in
`tools/`, its own workspace, `std`, never in the shipping graph. That is the pattern
`tools/redoxfs_host` already establishes, for the same reason.

Every question raised by porting it then evaporates: no bare-metal C, no libc tier, no third clang
backend to worry about, no LGPL anywhere near the image, no `com_err`, no `nm -u` spike. And what it
is actually best at is what we keep: thirty years of exposure to real ext4 images, used as the
reference our implementation is differentially tested against.

This is the same instinct as `script/vendor-verify`, which asks "is this tree what we say it is"
rather than "does it build".

## The four phases

Each is independently useful, and none depends on the next being funded.

| phase | what | needs | worth on its own |
|---|---|---|---|
| **1** | **Read-only ext4**, one confined `ext4_server` holding one block-device capability | nothing new | satisfies milestone 140's (mount a drive this system did not create) original requirement |
| **2** | **The differential oracle**: our reader against libext2fs against Linux, over a corpus of real images | `tools/` host harness | this is what makes a young implementation trustworthy, and building it *before* the writer is the whole trick |
| **3** | **Write support**, `metadata_csum` mandatory from the first commit | phase 2 green, plus milestone 37's (RedoxFS crash consistency) injector | the requirement calef actually stated |
| **4** | **jbd2-format journaling**, written at our own layer | a design fork, below | the property ext4 users expect, and the thing no other userspace implementation has |

### Phase 4 is a real design and it deserves its own decision

Recorded here so the idea is not lost, not because it is decided.

Transaction boundaries are free, because we control the caller. One `filesystem_protocol` request is
one filesystem operation. The server brackets it, and every block write the engine emits in between
arrives at our own IO layer, which is the interposition point we have to write anyway. Log those
blocks physically to the journal, flush, then write them in place.

The barrier exists, which is what makes this different from RedoxFS. `filesystem_protocol`'s
`blk::FLUSH` is a real `VIRTIO_BLK_T_FLUSH` the block server does not reply to until the device
completes it, with `EOPNOTSUPP` passed through honestly when the device cannot flush. notes/fs-server.md
names the absence of exactly this as RedoxFS's honest limit: its `Disk` trait has no flush and no
barrier, so ordering is the device's job. A journal without a barrier is theatre; milestone 55's
durability half bought the barrier.

**Use jbd2's on-disk format, not our own**, so that a drive we crash on is replayed by **Linux**, on
another machine, with no nife present. A private journal would make our crash a drive only we can
repair, which defeats the interop the whole milestone is for.

And the test cannot be faked. Linux is the oracle: write from nife, cut power with milestone 37's
(RedoxFS crash consistency) injector, mount on Linux, let it replay, then `e2fsck -fn`.

The hard parts, named rather than waved at. Revoke records, because a metadata block that is
freed and reused as data must not be replayed over. Journal checksum v3 exactness, since a wrong
checksum makes recovery do the wrong thing quietly. And physical journaling writes every block
twice, so it is slow, which is measurable and is a trade this workload can afford.

## Why the missing journal is survivable in the meantime

Not as an excuse, as a measured property of the workload this requirement actually has. A borg
repository is itself a transaction log:

- Segment files are append-only, roughly 500 MB, written sequentially. No random rewrites into
  existing files, little metadata churn, few files.
- Every log entry carries a CRC32, and a transaction is defined by a `COMMIT` tag. Incomplete
  transactions lacking a COMMIT are discarded when the repository is reopened.
- `borg check` verifies every chunk cryptographically, end to end. "Did the backup survive" is
  answerable with certainty, which is more than any filesystem journal offers.

So a crash on a non-journaling ext4 costs the tail of a segment, which borg discards anyway, plus
possible filesystem metadata damage, which the append-only shape minimises. **That is a mitigation
and not an equivalence**, and phases 1 through 3 must say so where a reader meets the feature rather
than in this block.

## Prior art, read rather than recalled

libext2fs has already been run as a filesystem server on a capability microkernel. Paul Boddie
integrated it into L4Re: a filesystem server program, per-file resource objects, IPC between clients
and the server, and a custom io_manager whose read and write block functions issue IPC calls to a
block server. That is the `ext4_server` architecture milestone 140 (mount a drive this system did
not create) already decided on, built by someone else, on a system in the same family. His honest
caveat is the one that matters to us: he deferred the C library work as too much of a challenge at
that stage, and he had L4Re's uclibc available. We would not, which is another reason the C stays on
the host.

The io_manager seam itself is 17 function pointers of which a real backend implements about six
(`open`, `close`, `set_blksize`, `read_blk64`, `write_blk64`, `flush`). It is already exercised
off-POSIX upstream: on Windows, `unix_io_manager` does not exist and `windows_io_manager` replaces
it, so the abstraction is load-bearing rather than aspirational.

Sources, for a reader who wants to check rather than trust:

- fuse2fs(1): https://man7.org/linux/man-pages/man1/fuse2fs.1.html
- fuse2fs source: https://github.com/tytso/e2fsprogs/blob/master/misc/fuse2fs.c
- `struct io_manager`: https://raw.githubusercontent.com/tytso/e2fsprogs/master/lib/ext2fs/ext2_io.h
- libext2fs on L4Re: https://blogs.fsfe.org/pboddie/?p=2457
- The EXT2FS Library manual: http://fs.csl.utoronto.ca/~sunk/libext2fs.html
- `ext4-view`: https://github.com/nicholasbishop/ext4-view-rs
- `ext4plus`: https://lib.rs/crates/ext4plus
- `lwext4`: https://github.com/gkostka/lwext4
- LKL `host_ops.h`: https://raw.githubusercontent.com/lkl/linux/master/arch/lkl/include/uapi/asm/host_ops.h
- borg data structures: https://borgbackup.readthedocs.io/en/stable/internals/data-structures.html

## The corpus, which is an asset most filesystem projects do not have

calef's drives are real images, written by a real Linux with the feature flags a real `mke2fs`
chose, containing borg repositories that verify themselves cryptographically. Read-only access to
them costs nothing and risks nothing, and `borg check` after a round trip is an end-to-end
correctness oracle no synthetic test suite provides.

**The first concrete action of this milestone is therefore not code.** It is `dumpe2fs -h` on those
drives, so the feature set phase 1 must support is a measured list rather than an assumption.

## What this milestone does not decide

- Whether nife hosts borg repositories or originates borg archives. Hosting (an SMB export, a
  `borg serve`, or borg 2's borgstore REST backend) needs only this milestone. Originating needs a
  borg client, and no native Rust implementation of the repository format exists: the `borgbackup`
  crate on crates.io shells out to the Python binary. That is a separate and much larger block, and
  it should be minted only if calef wants nife to make backups rather than hold them.
- FAT32 and ext2, which keep milestone 140's ordering.
- RedoxFS, which is untouched.

## BUGS

- No phase here is priced. The effort is unknown and this block deliberately does not guess.
  Phase 1 is small, phase 4 is not, and the honest number for phases 2 and 3 comes after the corpus
  survey rather than before it.
- Phase 3 ships a filesystem that can lose data on power failure, and no wording in a roadmap
  block changes that. The obligation is a `BUGS` section beside the feature and a `fsck` story, and
  neither exists yet.
- **`e2fsck` has no Rust equivalent and this block does not provide one.** Under the host-oracle
  decision, repair is a host-side operation, which means a drive damaged on nife is repaired by
  plugging it into Linux. That is acceptable today because cordoba exists; it would not be
  acceptable for a system meant to stand alone, and nothing here plans for that case.
- **The `metadata_csum` claim about calef's drives is inferred, not measured.** It is mke2fs's
  default and almost certainly true, and the `dumpe2fs` action above is what turns it into a fact.
- **The borg-survivability argument is reasoning about a workload, not a measurement.** Milestone
  37's injector is what would turn it into one, and nobody has run it against ext4.

## Index row

Minted by calef on 2026-08-30, after a session costed the whole option space. His ext4 drives hold
the family's borg repositories, so nife must eventually read and write the format Linux wrote; the
requirement is standing and has no deadline. Four phases: read-only, then a differential oracle
against libext2fs and Linux, then write with `metadata_csum`, then jbd2-format journaling. The
refusals are the valuable half: `lwext4` is GPLv2 where it counts, `ext4plus` cannot update `metadata_csum` on write, LKL needs §105's declined shared-address-space threads, and shipping
libext2fs itself was an argument from effort. C stays on the host as the oracle, the way `tools/redoxfs_host` already does.
