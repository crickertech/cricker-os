---
status: BUILT
raised: 2026-08-25
built: 2026-09-15
---
# 166. One boot loader, reached two inconsistent ways: unifying the per-architecture progenitor loaders

Built 2026-09-15. Delivered as one boot loader, `kernel::user::boot_progenitor`
(name provisional), used by all three architectures. The `init`-meaning question this block was
minted for (2026-08-25) was overtaken and settled by other milestones before this was built, so the
scope narrowed to the loader unification that actually remained; see "What overtook the original
premise" below.

## What overtook the original premise

This block was minted from a naming review that asked whether aarch64's `init -> hello` archive
mapping should become `init -> builder`. That framing is gone:

- Milestone 266 (one progenitor) gave the first process one name, `progenitor`, on all three
  architectures, retiring `init` as an alias that meant a different binary per board. The archive's
  `init` slot no longer carries two jobs, so there was no `init`-meaning question left to answer.
- Milestone 291 (thirty-one programs wearing one name) split `hello`'s thirty-one roles into their own programs,
  leaving nine `INIT`/child roles that the kernel still re-enters `hello` at. `builder` and its
  `init_boot` role, and `components/src/builder.rs`, are gone.
- Milestones 182/268/299 brought `x86_64` onto the same loader riscv64 uses and gave it a
  `PortRange` console capability.

So `spawn_init`, `boot_via_init`, `INIT_ROLES_ENTRY`, `INIT_BOOT_ROLE`, `components/src/builder.rs`
and `hello`'s `init_boot` role, all named by the original body, no longer existed when this was
built.

## What actually remained, and what this milestone did

There were two boot-loader functions doing the same job differently:

- `kernel::user::spawn_progenitor` (aarch64): loaded the `progenitor` archive entry, built its
  address space through the `sched::spawn` closure model, granted the boot capability set from
  inside the spawned thread, and returned a `Holding`. The **same function** also re-entered `hello`
  at milestone 19d's test roles, and that sharing was the whole problem: to keep the test roles' slot
  numbers stable, aarch64's boot progenitor was handed two capabilities the interactive system never
  used, a report endpoint at slot 1 and the 19d.2b test interrupt at slot 3, which is why its slot
  layout was not riscv64's.
- `kernel::user::riscv_shell_boot` (riscv64 and, since milestone 182/299, x86_64): loaded the same
  `progenitor` entry through the TCB-builder model (`create_thread_control_block` +
  `thread_control_block_insert_cap` from the parent), granted a boot-only capability set at fixed
  slots, and returned the thread.

Both reached the already-shared orchestrator `crates/system_initializer::boot()`, which was correct
and was not touched. The divergence was in the paths that reach it.

The two functions differed on four things, and only two were the hardware's:

| Difference | aarch64 | riscv64 / x86_64 | Real, or history? |
|---|---|---|---|
| Spawn mechanism | `sched::spawn` closure + `Holding` | TCB-builder + `ThreadId` | **History** (both APIs are arch-neutral) |
| Slot layout | budget 0, report 1, uart 2, test-SGI 3, uart-rx 4, ... | budget 0, uart 1, uart-rx 2, ... | **History** (report + test-SGI are the shared-with-tests residue) |
| Console device authority | `DeviceFrame` (UART page) | riscv64 `DeviceFrame`; x86_64 `PortRange` | **Hardware** (COM1 is port I/O) |
| Interrupt-controller arming | GIC, inline | riscv64 PLIC + supervisor external; x86_64 none | **Hardware** |

The build merged the boot halves into `boot_progenitor`, which uses the TCB-builder model on all
three architectures and `#[cfg]`-gates only the two genuine hardware differences inside one shared
shape. aarch64's boot drops the report endpoint and the test SGI, so all three architectures now hand
the progenitor **the same slot layout**, and `components/src/progenitor.rs`'s two cfg-gated cap
tables collapse into one with no `cfg` at all.

`spawn_hello` stays behind as the milestone-19d/19e test-role `hello` spawner, reduced to that
one job (always `hello`, the five test-role capabilities, the `Holding` return). Its ~28 direct-by-
name fixture call sites in `kernel/src/user/tests.rs` are unrelated to the boot path and are
unchanged; the six `spawn_hello` tests reach `hello`'s roles exactly as before.

## What it touched

- `kernel/src/user.rs`: `riscv_shell_boot` became `boot_progenitor`; `spawn_progenitor` was
  reduced to the test-role spawner and renamed `spawn_hello`; `boot_via_progenitor` deleted.
- `kernel/src/main.rs`: all three hand-off sites call `boot_progenitor`.
- `components/src/progenitor.rs`: the two `GRANTS` tables collapsed into one.
- `crates/system_initializer`: unchanged in logic. Its `BootEndowment.for_test_roles` field is now
  `&[]` on every boot, so nothing exercises the slot-deletion it drives; retiring the field is a
  follow-on, not this milestone's to make (it would be a change to `system_initializer`'s logic).

## Follow-on

- **Done.** `spawn_progenitor` was left a misnomer by the split: it no longer spawns the
  progenitor, it spawns `hello` at a milestone-19d/19e test role. calef ratified **`spawn_hello`**
  on 2026-09-15 and the rename is swept through the tree, with the refusal recorded at the
  function's own definition in `kernel/src/user.rs`.
- **Done.** `boot_progenitor`'s name was provisional when the loader merged; calef ratified it on
  2026-09-15, and the refusal is recorded at its definition in `kernel/src/user.rs`.
- **Recorded.** `BootEndowment.for_test_roles` is now dead data: no boot path fills it, so
  `crates/system_initializer` could drop the field and the slot-deletion it drives. Recorded in this
  block's "What it touched" section; retiring it is a change to `system_initializer`'s logic, out of
  this milestone's scope.

## Index row

Two boot-loader functions loaded the first process differently: aarch64's `spawn_progenitor` (a
closure that also served the 19d test roles, and so carried two capabilities the interactive system
never used) and riscv64/x86_64's `riscv_shell_boot` (the TCB-builder, boot-only). Milestone 166
merged the boot halves into one `boot_progenitor` used by all three architectures, `#[cfg]`-gating
only the two genuine hardware differences (the x86 `PortRange` console vs the others' UART page, and
GIC vs PLIC vs APIC arming); the slot layout is now identical everywhere and
`components/src/progenitor.rs`'s two cap tables became one. `spawn_hello` stays as the
19d/19e test-role `hello` spawner, fixtures untouched; that is `spawn_progenitor` renamed, which
calef ratified on 2026-09-15 once the boot half it was named for had moved out.
