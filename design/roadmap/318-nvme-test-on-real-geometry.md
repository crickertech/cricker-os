---
status: BUILT
raised: 2026-09-17
built: 2026-09-17
---
# 318. The NVMe boot test on real geometry

Built by a lane on `milestone/318-nvme-test-on-real-geometry`.
*(Number provisional until the merge queue lands it.)*

`kernel/src/user/non_volatile_memory_express_tests.rs::a_confined_el0_process_serves_the_block_interface_end_to_end` is
milestone 261's proof and `design/fatal-risks/README.md` risk 6's decisive experiment. It passed under
QEMU and **four of its assertions would have failed on xenon's 256 GB Micron**, for reasons with
nothing to do with confinement. All four now hold on any namespace, and nothing about what the test
proves has been weakened to get there.

## The assumptions, and how they were found

They were found by reading the test against the bench procedure it is about to be run under, not by
a gate: nothing in this tree can tell a constant that is a fact about the code from one that is a
fact about the runner's disk image. Two of the four were named in the lane's brief; the other two
turned up in the same read, in the helper rather than the test body, which is the more interesting
half of the finding.

| Where | Assumed | Would do on a 256 GB disk |
|---|---|---|
| `start`, the readiness report | `report[1] == 8 * 1024 * 1024` | fail: the report carries 256 GB |
| the `SIZE` verb | the answer is `8 * 1024 * 1024` | fail: the server answers 256 GB |
| the neighbour block | block 38 reads as zeros | fail on any disk that has held anything |
| the out-of-range block | block `8 MiB / 4096` is past the end | fail: block 2048 is valid and is read |

**Milestone 261's lane flagged the symptom without connecting it.** Its bench handoff says to check
that the namespace size reads about 256 GB rather than 8 MiB. That is precisely the number two of
these assertions were written against, and nobody noticed the assertions depended on it. It is the
shape this tree keeps finding: a test whose green depends on a condition nobody wrote down.

## What each assertion became

**The geometry assertions now compare against the geometry.** `nvme_service::Wiring` gained a
`size_bytes` field carrying what the kernel's admin plane read from IDENTIFY, which is the number
`nvme::Handoff` packs into `arg2` for the spawn. The readiness assertion compares `report[1]`
against it, which is the property it always wanted (*the size survived the handoff into ring 3*)
and is now stated in those terms rather than in QEMU's. The `SIZE` assertion compares the server's
answer against the same number: *the server answers what it was told*, again the actual property.

A new assertion guards the degenerate case the other two used to rule out by accident: a namespace
smaller than two blocks would make everything below it vacuously true, so `start` refuses one
loudly. `kernel/src/non_volatile_memory_express.rs::bring_up` already refuses a zero-block namespace; this says so where
the assertions that depend on it are.

**The neighbour assertion became a second pattern.** The property is *the write landed where it
said and not everywhere*. The old shape established it by reading block 38 and expecting the
image's zeros, which is a claim about the disk's prior contents and is true only of a file that was
just created. The new shape writes a second, different function-of-offset pattern to block 38,
then reads both blocks back and requires each to hold its own. A write that went everywhere puts
`second` into block 37 and the first read-back catches it; a read that always returns the same
block fails for the same reason. **Both writes happen before either read**, which is what makes the
block 37 read-back load-bearing rather than a restatement of the write it just did.

This is strictly stronger than what it replaced, which is worth saying because portability
arguments usually cost something. The old version could not distinguish "the write landed only on
block 37" from "the write landed on block 37 and on blocks the test never looked at". The new one
cannot either, but it does catch a *second* write smearing backwards, which the old one could not:
zeros in block 38 were consistent with block 38's write never having been attempted, because it
never was.

**The out-of-range assertion computes the block.** `size_bytes / BLOCK_SIZE`, which is the first
refused block for *any* size and not only for a size that divides evenly. When the namespace
divides evenly, that block begins exactly at the end. When it does not, that block's transfer runs
off the end. `nvme::Handoff::holds_block` refuses both, since its test is `(block + 1) * unit <=
size_bytes`, and that is the Kani-proved arithmetic this assertion exists to exercise.

## Why it now passes on a 256 GB namespace, argued rather than run

This cannot be run on xenon from a lane, so the argument is the deliverable. Take the namespace to
be 256,060,514,304 bytes (a typical 256 GB Micron capacity, and nothing below depends on the exact
figure):

1. **The readiness assertion.** Both sides of it are the same value from the same source:
   `w.size_bytes` is `handoff.size_bytes`, and `report[1]` is the `arg2` word the server was
   spawned with, which `Handoff::pack` takes from the same field. The assertion is now that the
   handoff is lossless, and `Handoff::pack`/`unpack` are Kani-proved to round-trip. It cannot
   depend on the magnitude.
2. **`SIZE`.** `components/src/non_volatile_memory_express.rs` answers the verb out of the unpacked handoff, and
   the assertion compares against the kernel's copy of the same number. Same argument.
3. **The size floor.** 256 GB is comfortably more than two 4096-byte blocks.
4. **Blocks 37 and 38 exist.** `holds_block(38, 4096)` needs `39 * 4096 <= size`, which holds for
   anything above 160 KiB. On QEMU's 8 MiB it also holds, which is why both legs run the same path.
5. **Each block reads back its own pattern.** Neither pattern refers to the disk's size or to its
   prior contents. The only way this fails on hardware and not under QEMU is a real driver or
   controller bug, which is exactly what the bench run is for.
6. **The out-of-range block.** `256,060,514,304 / 4096 = 62,514,774`. `holds_block` computes
   `62,514,775 * 4096 = 256,060,518,400`, which exceeds the namespace by 4,096 bytes, so the block
   is refused in the driver's own arithmetic before a command is built. The multiplication does not
   overflow `u64` by many orders of magnitude, and `holds_block` uses `checked_mul` in any case.
   The refusal reaches the client as a negative `blk` status, which is what the assertion reads.

The one thing this argument cannot cover is the device answering IDENTIFY with a geometry
`kernel/src/non_volatile_memory_express.rs::bring_up` refuses: a namespace whose LBA size does not divide 4096 into
`1..=8` logical blocks makes `blocks_per` `None` and the server never starts, and the test skips
rather than fails. That is pre-existing behaviour, correct, and worth knowing at the bench: **a
skip is not a pass**, and the transcript distinguishes them.

## Both new assertions were broken on purpose

A geometry-independent assertion that is also vacuous would pass everywhere, which is the failure
this milestone would otherwise be most likely to introduce. So each was falsified by hand, with
`cargo xtask test --arch x86_64 --test confined_el0`:

| Defect injected | What fired |
|---|---|
| `past_end` computed as `size / BLOCK_SIZE - 1` | `the server read block 2047, which a namespace of 8388608 bytes does not have` |
| both writes sent to block 37 instead of 37 and 38 | `assertion left == right failed: byte 0 of block 37 came back wrong` |

The second is the one worth reading twice: it is a write landing somewhere other than where it
said, which is exactly the property the retired zeros check was there for, and the new shape
catches it at the *first* read-back rather than at the neighbour.

## What else assumed QEMU's geometry

Two records, now corrected, and both had the assumption written down as a *reason*, which is how
they were found:

- `xtask::mknvmedisk`'s header said the image is 8 MiB "because the test asserts IDENTIFY's size
  answer against exactly this number, so the file and the assertion must move together". That
  coupling is gone; the image may now be any size a controller will take. The file stays 8 MiB
  because a small zero file is cheap and fast to attach, which is a different reason and is what
  the header says now.
- `notes/non-volatile-memory-express.md`'s `EXAMPLES` and its "what the test proves" section both stated the zeros check
  as the property. Both now state the two-pattern shape and say that nothing is written against the
  runner's image size.

**No other block-device test carries the same assumption.** `kernel/src/user/disk_tests.rs` works
against the tree's own built image through `filesystem_protocol::blank`'s constants, which are
derived from the partition layout the build produced rather than from a number typed twice; its
zero-expectations are about blocks the build wrote, not about a disk nobody formatted.
`crates/non_volatile_memory_express`'s host tests use `8 * 1024 * 1024` as a fixture size, which is correct: those are
tests *of* `Handoff`'s arithmetic and pick their own geometry rather than meeting a machine's.

## BUGS

**Nothing proves an unrelated block was left alone, on any disk.** The test can show that two
blocks each hold what was written to them; it cannot show that block 5,000,000 was untouched
without reading it, and reading it proves nothing without knowing what was there. The IOMMU
assertion is what bounds the damage a confused server can do, and it is the claim risk 6 turns on.

**The test writes blocks 37 and 38 of whatever disk is attached.** True before this milestone and
still true. On xenon that is the disk calef is about to wipe; on any other machine it is data loss.
Nothing in the runner checks, and a check would need a notion of "a disk it is safe to write" that
this tree does not have.

## Follow-on

- **Recorded.** Milestone 261's block and `design/fatal-risks/README.md` risk 6 both describe the test as asserting the
  image's zeros and the 8 MiB size. Deliberately not edited from this lane (they are another
  milestone's account and a decisions-adjacent record); the maintainer reconciles.
- **Milestone 323.**. This test carries no `Falsification:` record, so the two falsifications above live in this block rather than in
  `kernel/falsifications/`, and nothing replays them. Both were edits to the test, which proves the
  assertions are not vacuous and proves nothing about a real defect reaching them.
- **Recorded.** The bench procedure in milestone 261's handoff tells the operator to confirm the namespace reads
  about 256 GB rather than 8 MiB. That is now a *diagnostic* rather than a pass condition, since
  the test no longer cares, and the procedure should say so: the number worth reading off the
  transcript is still the size, because a controller answering 8 MiB on a 256 GB disk means
  IDENTIFY was misparsed, but it is no longer what turns the test red.

## Index row

`a_confined_el0_process_serves_the_block_interface_end_to_end` is milestone 261's proof and fatal
risk 6's decisive experiment, it passed under QEMU, and **four of its assertions were written
against QEMU's 8 MiB zeroed image** and would have gone red on xenon's 256 GB Micron for reasons
with nothing to do with confinement: the readiness report's size, the `SIZE` answer, a neighbour
block expected to read as zeros, and an out-of-range block computed from a hardcoded 8 MiB. The
first two now compare against the geometry the kernel read from IDENTIFY, which is the property
they always wanted (*the size survived the handoff into ring 3*, *the server answers what it was
told*) rather than a fact about the runner's image; the third writes a second distinct pattern to
the neighbour and requires each block to read back its own, which establishes *the write landed
where it said and not everywhere* without any claim about what the disk held before; the fourth
computes `size / BLOCK_SIZE`, the first refused block for any size whether or not it divides
evenly. **Milestone 261's lane flagged the symptom without connecting it**: its bench handoff says
to check that the namespace reads about 256 GB rather than 8 MiB, which is exactly the number two
of these assertions were written against. `xtask::mknvmedisk`'s header and `notes/non-volatile-memory-express.md` both
recorded the now-retired coupling as a reason and are corrected. No other block-device test carries
the assumption. The 256 GB argument is stated rather than run, because a lane cannot reach xenon.
