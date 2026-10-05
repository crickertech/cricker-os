---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: 14, 18
machine_requirements: none
specific_machine: none
needs_person: no
---
# The shadow descriptor is published in two stores

Raised by lane/633-outsider-2, milestone 633 (an outside agent attacks the confinement claim)'s
second pass, on 2026-10-05 (UTC). Title and slug provisional.

**Reuse:** the validate-then-publish shape is the one `crates/virtio`'s own avail-ring publish
already uses (the `avail.idx` write is the single publish point), and variant 2 reuses the used
ring the device already writes; no new mechanism is proposed, and the crate stays on the
verification path per §46 (thin primitives or whole subsystems; we write everything in between),
so nothing external is taken.

## The hazard

`direct_memory_access_validator::shadow_one_head` copies a validated descriptor into the shadow
table as two `write64`s, address then length-and-flags, with no barrier between them and no check
that the shadow slot is idle. It also writes each descriptor before it has checked the next in the
chain. A driver that republishes a head the device still holds (a posted receive buffer is the
ordinary case) can have the device observe the new address with the old length, a span the
validator never admitted. On virtio-mmio there is no IOMMU behind this walk, so it is the only
boundary.

`notes/confinement-claims.md` row 17 keeps this property `unfalsified` on purpose, because the Kani
harness proves a property of the design (table and shadow are disjoint arrays) and cannot model a
device reading between two stores. This is the hazard under that honesty, reasoned from the code
and recorded in the crate's `BUGS`. It has not been reproduced: a device race under QEMU is not
deterministic enough to turn into a test without a fault injector.

## The fix, priced

The shape is the one every ring protocol uses. Validate the whole chain into a staging copy first,
then publish it. Two variants:

1. **Validate, then copy.** Walk the chain checking every descriptor and recording the indices;
   only if every check passes, copy them. Removes the partial-copy case entirely; the torn single
   descriptor remains unless combined with 2. Cost: a second pass over at most `qsize`
   descriptors, which is eight today.
2. **Never rewrite a live slot.** Keep a shadow free-list and copy a republished chain into fresh
   shadow slots, remapping `next` links; the old slots stay intact until the device returns them
   through the used ring. This is what makes the publish atomic from the device's view, because the
   avail-ring index write is already the single publish point. Cost: shadow table twice the driver's
   queue size, and a used-ring reader in the validator, which it does not have.

Variant 1 alone closes the partial-copy half and is a morning's work. Variant 2 closes the torn
descriptor and needs the used ring, which is where the device says which slots it is done with.
Recommended: 1 now, 2 when a driver republishes live heads, which none in the tree does today.

## Acceptance

- A host test with a `ChainMem` that records write order and asserts no shadow slot the device may
  hold is written to, which turns red on the current code.
- The Kani model extended with a "device reads here" point between the two stores, so the proof
  says what it covers.
- Row 17's `Falsification:` moves from `unfalsified` to `replayable` against the fixed walk.
