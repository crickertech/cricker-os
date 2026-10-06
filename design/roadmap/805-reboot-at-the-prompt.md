---
status: NOT-STARTED
raised: 2026-10-06
promoted_from: reboot-at-the-prompt
milestone_dependencies: 592
decision_dependencies: 251
machine_requirements: none
specific_machine: radon (calef's preferred lab machine, and the one whose reset is unproven)
needs_person: yes
---
# 805. `reboot` at the prompt: a person restarts the machine without touching its power

*(Promoted from the proposal pile on 2026-10-06 (UTC); number provisional until the merge queue lands it.)*

*(Ruled 2026-10-06 (UTC), calef on PR #1766: "Approved, go with the new reboot object." §251
(restarting the machine is a kernel object the progenitor hands out) records the object, its one
method and the refused options. The fork below is kept as the argument; the recommendations in it
are now the ruling.)*

*(The dependency on milestone 592 (radon's cold reboot dies in OpenSBI's PMIC write) gates exit
criterion 6, the radon silicon run, and nothing else. The QEMU gate on aarch64, riscv64 and x86_64
does not wait for 592, and a lane can build and land everything else first.)*

calef asked for this on 2026-10-06 (UTC), in a maintainer session. A writing-only lane wrote it the
same day and built nothing. The design fork below is ruled (§251). Every name here is provisional:
the program `reboot`, the kernel object, its method and the manifest field. They are an architect's
call, and §251's ruling does not name them. Under design/naming.md, `reboot` is a verb that would pass only as a term of art, like `bind`.

## Why

radon (the VisionFive 2) is calef's preferred lab machine for its form factor. Restarting it today
means a person cycling smart plug 2. Milestone 224 (nothing can power-cycle radon) records that as a
standing choice to stay manual. A warm reset typed at the prompt is quicker, and it does not cut
power to a board mid-write.

An installed system needs one too. The installer's last line, which `xtask/src/install.rs` checks
for, is `DONE. Remove the installation medium and reboot.` Nothing at the prompt can do the second
half. Milestone 802 (the trivial install) is where a stranger meets that line.

Against principle 1, honestly: the customer path is vacant, and this moves no fatal-risk verdict.
Milestone 249 (the boot lottery) already resets from the soak path for fatal risk 5's experiment,
so a prompt command adds nothing to that series. It is a convenience for the bench and a
prerequisite for an installed system. Its claim on a lane is that it is small, because the
per-architecture half is already written.

## What the tree has

Read on 2026-10-06 (UTC):

| | aarch64 | riscv64 | x86_64 |
|---|---|---|---|
| route | PSCI `SYSTEM_RESET` over the `/psci` conduit | SBI SRST, reset type 1 | FADT reset register, then `0xCF9`, then the 8042 |
| code | `kernel/src/arch/aarch64/mod.rs` | `kernel/src/arch/riscv64/semihosting.rs` | `kernel/src/arch/x86_64/reset.rs` |
| QEMU proof | `virt`, over `hvc` | `virt` | `q35`, attempt 1 only |
| silicon | no aarch64 board has booted nife | radon hangs in OpenSBI (below) | xenon untested |

All three are `arch::reboot(marker)`, compiled only under `--features reboot_soak_test`.
`script/soak-test --reboot --arch <arch>` proves each by a second boot under QEMU. No syscall,
capability or program reaches them in a normal build.

radon's reset does not work today. On 2026-09-04 its OpenSBI accepted SRST type 1 and then hung.
OpenSBI resets the board with an I2C write to the AXP15060 PMIC, and U-Boot had left that bus held
in reset. The transcript is `target/board/radon-2026-09-04-srst-reset-pmic.log`. Milestone 592
(radon's cold reboot dies in OpenSBI's PMIC write) built a kernel fix on 2026-09-25 that releases
the bus first, and it is waiting on one bench reset. Until that bench run passes, `reboot` cannot
work on radon whatever this milestone builds. That is why 592 is this block's dependency, for the radon exit criterion only.

The writable filesystem is RedoxFS behind `redoxfs_server`, not nifefs. nifefs is the read-only
boot archive. notes/fs-server.md measures RedoxFS as prefix-consistent at every power-cut point.
Its block cache is write-through, and `filesystem_protocol::fs::SYNC` (op 19) asks the block
server for a real device flush. Both the virtio and NVMe block servers implement that flush.

**Reuse:** the reset routes are the tree's own `arch::reboot`, and the kernel is written here by
rule. The program is a capability invoke after a `SYNC`, so Linux's `reboot(8)` and busybox's
have nothing to lend: what they wrap is `reboot(2)`, option D below.

## The fork, ruled by §251

Three questions, one recommendation each. Only the first is expensive to change, because it adds
to the syscall surface (§10 (process model: capability-based, microkernel) and §16 (object
revocation)) and every later program is written against it.

### Which object carries the authority

The constraint that decides most of this: PSCI and SBI calls are made from EL1 and S-mode. A user
program cannot make them, so on two of three architectures the kernel must act on its behalf.

| option | what it is | verdict |
|---|---|---|
| A | A new kernel object with no payload, one method that calls `arch::reboot`. The kernel grants it to the progenitor at boot. The progenitor endows `reboot` through a manifest field, the way `entropy` is endowed. | recommended |
| B | A method on an existing object | refused |
| C | A user-mode driver holding device authority | refused |
| D | A new syscall number | refused |
| E | An op on a kernel-served endpoint, the shape milestone 391 (kernel introspection over an endpoint) asks about | refused for now |

B fails because no existing object names the machine. The root `MemoryRegion` comes closest. Its
authority is split and handed to every child, and the generic rights (read, write, grant,
enumerate) have no bit for "may reset". Adding one would make every memory grant a question about
power.

C works on x86_64 alone, where a `PortRange` on `0xCF9` is already a capability kind. PSCI and SBI
have no device to grant, so C breaks §19 (architectural parity) by construction.

D is ambient: any thread could reset the machine unless the kernel checked an identity, and the
kernel has none. It is a brand-new syscall number besides, which CLAUDE.md says is a fork of its
own.

E would be right if 391 had been decided. §149 (may the kernel answer on an endpoint) deliberately
left that question open until a consumer existed, and milestone 269 (`machine` at the prompt) is
that consumer. Folding reset into 391 would also put a harmless read (what is this machine) and a
destructive act behind one capability. That pairing would need rights to split them, and this tree
has none for it.

A is the shape this tree already uses for an authority the kernel must exercise itself: `Irq` and
`Timer` are kernel objects a holder invokes. It costs one `Object` variant, one `abi::objtype`
number, one method, a `GRANTS` slot and a `grant_plan` field. `script/fastpath-footprint` measures
the syscall decoder flat, so it prices the new dispatch arm. That arm is cold and should cost
nearly nothing, but no number exists until it is built.

Prior art, from memory and not re-read for this proposal: Linux gates `reboot(2)` on
`CAP_SYS_BOOT` plus magic numbers, which is D with an identity check. Fuchsia's `zx_system_powerctl`
takes a resource handle, which is A. seL4 has no reset call and leaves it to the platform, which
is C.

Would A still win if every option cost the same? Yes. It is the only option that is both a
capability and possible on all three architectures.

If calef says no to A, the next best is to wait for 391 and make reset one op of E. That would
park this milestone behind an undecided fork.

### What happens before the reset

Recommended: this milestone does `SYNC`, then the reset, and no more.

RedoxFS needs nothing more for integrity. A reset after `SYNC` loses nothing acknowledged. Without
it, milestone 37 (prove RedoxFS's crash consistency) measured that a cut loses only a suffix of the
workload. `SYNC` is what makes "nothing acknowledged is lost" true on a device with a volatile write
cache.

A full ordered shutdown (tell every service to stop, wait, then reset) has no consumer yet. The
system log is in memory only (milestone 613 (a system log service: the in-memory half)). No service
holds state that a stop message would save. When one does, that milestone adds the ordered stop, and
`reboot` calls it first.

How `reboot` reaches `SYNC` is the building lane's call. It could hold a filesystem grant of its
own, or the progenitor could sync on its behalf. A filesystem the person never mounted needs no
sync.

The x86_64 reset file's `BUGS` says it is "only called from a soak, never from a machine holding a
filesystem open". This milestone makes that sentence false, so the build rewrites it.

### Whether power-off belongs here

Recommended: no. Power-off is a separate small program and a separate milestone, by calef's "one
program, one thing" ruling. It should be a separate object kind too, so that holding `reboot`
confers no power to switch the machine off. On radon, power-off and reset are the same PMIC write
(592's `BUGS`), and a powered-off radon needs a person at plug 2. A lab machine that can only be
switched off from the prompt is worse off than one that cannot be.

If calef wants both now, the cheaper shape is two objects in one milestone, still two programs. A
`--off` flag on `reboot` is the shape his ruling refuses.

### Parity

The kernel half lives on all three architectures already. This milestone moves `arch::reboot` out
from behind `reboot_soak_test` and gives it a second caller. The soak keeps its own feature for the
timer loop. The proof is one gate on all three under QEMU, plus radon.

## Exit criteria a stranger could check

1. Under QEMU, on aarch64 (`virt`), riscv64 (`virt`) and x86_64 (`q35`): one `cargo xtask` gate
   boots to the `swish` prompt, writes a file, types `reboot`, and sees the firmware banner and a
   second prompt. The file reads back after the second boot. It exits 0 on all three, and
   `script/test` runs it.
2. The same gate shows `SYNC` happened: the block server reports a completed `blk::FLUSH`
   before the reset line.
3. A program whose manifest does not declare the endowment holds no reset capability. A fixture
   proves it by looking for one and finding none, and `caps reboot` names the authority.
4. A refusal is loud. When `arch::reboot` returns, `reboot` prints the firmware's refusal and exits
   non-zero. A firmware that hangs instead (radon before 592) cannot be detected from inside, and
   the program's `BUGS` says so.
5. `script/soak-test --reboot` still passes on all three architectures.
6. On radon, after milestone 592 is BUILT: `reboot` typed at the prompt returns the board to a
   prompt with nobody touching plug 2. The serial capture is committed under `target/board/`.
   calef is at the bench, hence `needs_person`.
7. §251 records the object's semantics. If the build finds the method must differ from it, the
   section is amended in the same pull request.

xenon and argon are not exit criteria, because neither is a lab machine for this. xenon's first
reset belongs to milestone 249's bench steps. If it fails there, that is a `BUGS` line here.

## What was blocked until the ruling

The kernel object, the endowment, the program and the gate waited on the first question. calef
picked A on 2026-10-06, so nothing in this block waits on a ruling now. Only exit criterion 6 waits,
on milestone 592.

## BUGS

- radon's reset is unproven on silicon. 592 has a fix waiting on one bench run, and if that fails,
  592's options B and C (a nife PMIC write, or new firmware) come before this.
- Who at the prompt may reboot is not decided here. Any session the progenitor endows can.
  Restricting it to some logins is a policy question for whichever milestone decides what login
  grants.
- A reset does not quiesce devices. A DMA transfer in flight is cut off. After `SYNC` the block
  servers are idle, and nothing else on the machine writes to persistent storage.

## Index row

Typing `reboot` at the `swish` prompt restarts the machine, after flushing the writable filesystem. The authority is a new kernel object with one method (§251), granted to the progenitor and endowed to one program, so no other program can reset the machine. The per-architecture reset routes are already written and QEMU-proven; this gives them a caller outside the soak. It is a bench convenience on radon and the missing second half of the installer's last line.
