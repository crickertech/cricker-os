# Giving up a mapping: `AddressSpace::UNMAP`

Milestone 95 (an unmap primitive, and the mappings init never lets go), built 2026-10-05 (UTC) on
§162 (whether a holder can give up a mapping), which calef decided as option A that day. §162 left
two semantics owed by this milestone. Both are built here provisionally, argued below, and
waiting on an architect in pull request #1678. The method number (2) is the next free one on the
object, and `forget_mapping_at` is a provisional name.

There is a third finding, and it matters more than either semantic: **as the object model stands,
`UNMAP` cannot reach the window §162 was decided to close.** It is under
[the finding](#the-finding-no-capability-names-a-running-space).

## What it does

`invoke(aspace, UNMAP, va, 0, 0)` takes the one page the space maps at `va` out of three places, in
one hold of the mapping registry:

1. The space's page tables (`arch::mmu::unmap_user_at`, the function every revoke uses).
2. Every core's TLB. aarch64 issues `tlbi vaae1is` (every ASID, every core in the inner-shareable
   domain). riscv64 runs a local `sfence.vma` and an SBI remote fence to every other online hart.
   x86_64 runs `invlpg` and the NMI shootdown, and with `CR4.PCIDE` off a space not installed on a
   core has nothing cached there.
3. The space's mapping record, so `LIST` stops reporting the page and a later revoke of the frame
   does not reach `va`.

It needs `WRITE` on the address-space capability, which is the right `MAP_INTO` takes. It touches
no capability and no other space.

## EXAMPLES

A builder that mapped the wrong page into a child before starting it:

```rust
// SAFETY: invoke is the syscall; the kernel validates the capability, method and address.
unsafe { invoke(aspace, abi::address_space::MAP_INTO, va, frame, abi::address_space::MAP_RW) };
// ... wrong frame. Give the page up and map the right one at the same address.
let r = unsafe { invoke(aspace, abi::address_space::UNMAP, va, 0, 0) };
assert_eq!(r, 0);
unsafe { invoke(aspace, abi::address_space::MAP_INTO, va, right_frame, abi::address_space::MAP_RW) };
```

Asking twice is an error the second time:

```rust
assert_eq!(unsafe { invoke(aspace, abi::address_space::UNMAP, va, 0, 0) }, 0);
assert_eq!(
    unsafe { invoke(aspace, abi::address_space::UNMAP, va, 0, 0) },
    abi::Error::BadPointer as i64,
);
```

The proof is `system_tests/src/user/unmap_tests.rs`, four tests on all three architectures, each with
a replayable falsification under `system_tests/falsifications/user.unmap_tests.*`.

## Semantic one: `UNMAP` of a `va` with nothing mapped answers `BadPointer`

Provisional behaviour. Refused with `BadPointer`, nothing changed. The same answer covers a
misaligned or kernel-half `va`, and a page already given up.

1. What else was considered. Success with nothing done, which is idempotent. It loses on what
   the method is for. A program calls `UNMAP` to close a window. If it passes the wrong address,
   an idempotent answer says the window is shut while it is still open, and nothing tells the
   program otherwise. A new error code (`NotMapped`) was also considered. It loses because adding a
   code to `abi::Error` is a wire change of its own, and `BadPointer` already means "not memory you
   could have touched yourself", which an unmapped page is.
2. What the tree does in the analogous case. `MAP_INTO` of an address already mapped answers
   `BadPointer` rather than succeeding or replacing, and `supervision_protocol::map_scratch` relies
   on that refusal as its free list. `UNMAP` is the same rule read the other way. Inside the
   kernel, `MappingHold::forget_mapping` is silent on a missing record, but it is a rollback, and
   for a rollback "nothing left to undo" is the right answer.
3. Prior art, read on 2026-10-05. It splits. Linux `munmap(2)` succeeds ("It is not an error if
   the indicated range does not contain any mapped pages"). seL4's aarch64
   `performPageInvocationUnmap` unmaps only when the frame capability records a mapping, and returns
   success either way. Zircon `zx_vmar_unmap` answers `ZX_ERR_NOT_FOUND` ("Could not find the
   requested mapping"). Mach's `vm_deallocate` man page answers `KERN_INVALID_ADDRESS` ("The address
   is illegal or specifies a non-allocated region"). seL4's case differs in kind, since its unmap names a capability that
   knows its own mapping, so "is there a mapping" is never ambiguous there. The two systems whose
   unmap names an address, as ours does, split one each way.
4. Is the premise true? Yes. `mmu::unmap_user_at` already returns `None` for an empty address,
   and the record lookup returns `None` for an absent record, so the handler can tell the two cases
   apart at no cost.
5. Cost. One branch. Neither option is cheaper to build.
6. Reversibility. This is the more reversible of the two. A caller written against
   `BadPointer` that treats it as "nothing there" keeps working if this ever becomes success. A
   caller written against idempotent success breaks if it ever becomes an error. Nothing outside
   this branch has called the method.
7. Same cost? Yes. The choice is about what a caller can learn, not effort.

## Semantic two: the frame capability survives the unmap, untouched

Provisional behaviour. `UNMAP` reads, consumes and changes no capability. A frame capability
the caller still holds can map the page again; one already deleted was never needed.

1. What else was considered. Consuming the frame capability the page was mapped under. It does
   not work, and the reason is structural: `UNMAP` names an address, not a capability. The mapping
   record stores the run's base address, which every derivative of a capability shares (§132 (what
   `PageFrame::REVOKE` owes an overlapping run)), so there is no single slot to consume. And in the
   case §162 was decided for, there is nothing to consume: `supervision_protocol::fill_and_map` and
   `system_initializer::receive_image` delete each frame capability right after mapping it. A
   second alternative, refusing `UNMAP` unless the caller still holds the capability, loses for the
   same reason. It would make the progenitor's windows impossible to give up.
2. What the tree does in the analogous case. Mapping does not consume either: `MAP_INTO` and
   `PageFrame::MAP` leave the capability in its slot, and the tree's builders delete it themselves.
   `PageFrame::REVOKE` deletes capabilities because taking authority back is what it is for. That
   is §13 (capability revocation and untyped reclamation), and `UNMAP` is not a revoke. It gives up
   one space's use of a page, and the capability is a separate authority.
3. Prior art, read on 2026-10-05. seL4's `performPageInvocationUnmap` clears the mapped address
   and ASID on the capability in its slot and leaves it there, remappable. Zircon's `vmar_unmap`
   page does not mention the VMO handle; that it survives is recalled, not read.
4. Is the premise true? One part of §162's framing is worth checking: "whether the frame
   capability survives" assumes the unmap can find one. It can find a family (the record's object
   word), never a slot, which is why consuming was never really on the table.
5. Cost. Nothing either way. The surviving behaviour is the absence of code.
6. Reversibility. Consuming later would break any caller that remaps after unmapping. Nothing
   does yet.
7. Same cost? Yes.

Neither contradicts §13 or §41 (the endpoint is the broker, and a device is revoked by taking it
back), which §162 required. `UNMAP` touches only the named space's record, so a revoke still finds
every other mapping of the page. A device page the space gave up is no longer in that space, which
is what §41's take-back would have done to it anyway, and the device capability is untouched.

## The finding: no capability names a running space

§162 decided that "this space gives up this window". The window milestone 95 exists to close is the
progenitor's scratch window, which lives in the progenitor's own, running address space. **No
capability names a running address space, the caller's own included**, so `UNMAP` cannot be invoked
on it.

The evidence, each one a lookup:

- `AddressSpace` capabilities are minted in one place, `RETYPE_OBJ(ADDRESS_SPACE)` (`kernel/src/syscall.rs`).
  The kernel grants none at boot, so init holds none for the space it runs in.
- `ThreadControlBlock::CONFIGURE` consumes the builder's capability and calls
  `user::take_user_address_space`, which removes the name from `USER_SPACES`. A copy made earlier
  resolves to nothing afterwards: `user_address_space_root`'s own comment says so for `LIST`.
- The progenitor maps its scratch pages with `PageFrame::MAP`, which acts on the caller's own space
  and needs no address-space capability at all.

So `UNMAP` today reaches a space under construction, which serves a builder that mapped a wrong
page, and nothing more. The negative control §162 and the roadmap block name (the progenitor writes
to a boot server's page and faults) cannot be built until a running space can be named. This is a
design fork on the syscall surface, so it is an architect's call and this lane did not build an
answer. The proposal is
`design/roadmap/proposals/a-running-process-can-name-its-own-address-space.md`, with the options and
a recommendation.

This bears on the running-root fix
(`design/roadmap/proposals/a-destroyed-region-cannot-free-a-running-root.md`). Suppose the answer
keeps a bound space in `USER_SPACES` rather than moving it into the thread. Then
`reap_address_spaces_in_region` sees bound spaces for the first time, and that fix's options A and B
change shape. The two should be decided together, or the second written knowing the first.

## What it costs

`UNMAP` is spawn-time and teardown work, never a step of the IPC round trip, and its body is
`#[inline(never)]` outside `invoke` for the reason `memory_region_map` gives. The dispatch arm adds
one rights check to `invoke`. Not measured: no `icount` row exists for unmap, so nothing here says
what one call costs. The record lookup is a linear scan of the space's log, the same shape and
order as `forget_mapping`, at most `LOG_ENTRIES` (170) entries a page.

## BUGS

- It cannot reach a running space, so it does not close milestone 95's hole yet. Above, and
  in the proposal.
- No TLB is ever stale for it to flush. A space it can reach has never run, so no core caches
  a translation of it. The flush is issued anyway, by the function revoke already uses across
  running spaces. No test shows a remote core losing a translation through `UNMAP`, and none can
  until the fork is answered; the proposal names that test as owed.
- One page per call. A run `MAP_INTO` laid down in one call takes one `UNMAP` per page. §162
  ruled `UNMAP(va)`, and a count argument would be a second semantic to argue. Unmeasured.
- Intermediate page tables stay linked, and their `TABLE` records stay in the log. They come
  back when the space or the region that paid for them dies, as before.
- Not measured (above).
