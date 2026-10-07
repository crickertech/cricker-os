---
status: PARTIAL
raised: 2026-09-24
promoted_from: radons-reboot-dies-in-opensbis-pmic-write
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: radon (the fault is in radon's own OpenSBI and PMIC)
needs_person: yes
---
# 592. radon's cold reboot dies in OpenSBI's PMIC write, and the kernel sets that write up

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal radons-reboot-dies-in-opensbis-pmic-write, filed 2026-09-24 by the lane that brought
milestone 249 (the boot lottery is sampled by a person walking to the board)'s reboot to aarch64
and `x86_64`. Option A was built on 2026-09-25 on `milestone/592-radon-pmic-bus`: before the
rebooting soak's SBI SRST call, a JH7110 kernel ungates I2C5's clock and releases I2C5's reset.
It is green on everything a host and QEMU can gate. It has not run on radon.

One bench boot of radon decides it, and the procedure below says what each
outcome means before it runs. Option C (new firmware) stays calef's call and is not touched here.

## What happens

On radon, SBI SRST reset and shutdown are both a single I2C write to the AXP15060 PMIC at `0x36` on
I2C5: register `0x32`, bit 6 for a reset and bit 7 for a power-off. OpenSBI's `pm_system_reset` has
no other route. When the read that comes before the write fails ten times, it prints `cannot read
pmic power register` and waits for an interrupt forever. radon's 2026-09-04 log
(`target/board/radon-2026-09-04-srst-reset-pmic.log`) shows exactly that: ten `i2c read: write
daddr 36 to` lines, the message, and no second boot.

## The cause, read rather than inferred, and a correction to the proposal

The proposal read the vendor OpenSBI's current `JH7110_VisionFive2_devel` branch. radon does not
run that code. Its banner says `U-Boot SPL 2021.10 (Feb 12 2023)` and `OpenSBI v1.2`, and its
messages (`i2c read: write daddr 36 to`, `cannot read pmic power register` with no function-name
prefix) appear in neither the current branch nor upstream. They do appear, word for word, in the
OpenSBI commit the VisionFive 2 SDK tag `VF2_v2.10.4` pins, `starfive-tech/opensbi` `ced60104df4f`,
in `lib/utils/i2c/fdt_i2c_starfive.c` and `platform/generic/starfive/jh7110.c`. That tag's U-Boot is
`starfive-tech/u-boot` `1539c1fb5a49`. All four files were fetched 2026-09-25. This is the code on
radon's flash, by string match; the binary itself is not published.

Three facts in those files, and the one this lane found that the proposal did not:

1. U-Boot asserts I2C5's reset before it hands over. At `Starting kernel`, U-Boot removes every
   device flagged `DM_FLAG_OS_PREPARE`, the DesignWare I2C driver among them.
   `designware_i2c_remove` (`drivers/i2c/designware_i2c.c` line 811) disables the clocks and then
   calls `reset_release_bulk`, and `reset_release_all` (`drivers/reset/reset-uclass.c` line 236)
   asserts each reset before freeing it. That removal is what prints the two `clk ... already
   disabled` lines.
2. radon's OpenSBI never releases a reset. Its I2C driver re-enables a clock before each
   transfer and does nothing else. A DesignWare controller held in reset reads `IC_STATUS` as zero,
   so the transmit-FIFO-empty poll can never succeed. That is the `write daddr` message, ten times.
3. The clock it re-enables is the wrong one. It computes the gate as `0x13020228 +
   index * 4`, with `index = name[3] - '0'` from the bus node's name. radon's U-Boot tree names the
   node `i2c@12050000`, not `i2c5@...`, so `index` is `'@' - '0'` = 16 and the word is `0x13020268`:
   clock 154, `JH7110_SYSCLK_UART4_CORE`. I2C5's own gate is never touched. Upstream OpenSBI commit
   `4d8569df7bd7` (2024-02-22) fixes this lookup, which is the proposal's option C.
4. There is no "core clock" to enable (the correction). The proposal took U-Boot's
   `u5_dw_i2c_clk_core` to be a second gate. radon's U-Boot registers it as
   `starfive_clk_fix_factor(..., "u5_dw_i2c_clk_core", "u5_dw_i2c_clk_apb", 1, 1)`
   (`drivers/clk/starfive/clk-jh7110.c` lines 698 to 700): a divide-by-one child of the APB gate with
   no register of its own. Its id, 298, is above the vendor's `JH7110_CLK_SYS_REG_END` (190), and
   mainline's DesignWare node names only `JH7110_SYSCLK_I2C5_APB`. I2C5 has one gate and one
   reset line, and the reset line is the one nothing on radon ever releases.

## What was built

| where | what |
|---|---|
| `crates/device_tree_blob` | `parent_prop_compatible`: a property of the *parent* of the first node matching a `compatible`, because the PMIC is the node with a binding and the bus's `clocks`/`resets` are one level up. Name provisional. |
| `crates/jh7110_clock_and_reset` | The SYS domain (`SYS`, `SYS_BASE`, `discover_sys`), I2C5's ids, the constant fallback `PMIC_BUS_BRING_UP`, and `pmic_bus`, which reads the plan out of the tree. Names provisional. |
| `kernel/src/memory.rs`, `kernel/src/arch/riscv64/mmu.rs` | The SYS window and the plan are recorded and mapped only under milestone 220 (this kernel drives no clock or reset controller, and the first real device will need one)'s JH7110 guard, so every other machine gets neither. |
| `kernel/src/soak.rs` | `prepare_the_reset_route`, called just before `arch::reboot`, walks the plan through the existing `drivers::jh7110_clock_and_reset::bring_up` and prints every word it read. |

The registers, and the source for each:

| what | register (SYS CRG at `0x1302_0000`) | sources that agree |
|---|---|---|
| I2C5 APB clock, id 143 | word `0x23c`, bit 31 set | mainline `JH7110_SYSCLK_I2C5_APB 143` and `JH71X0_CLK_ENABLE BIT(31)` at `base + 4 * idx`; vendor `JH7110_I2C5_CLK_APB 143`; radon's own OpenSBI's `0x13020228 + 5 * 4` |
| I2C5 APB reset, id 81 | assert word `0x300`, bit 17 cleared; status word `0x310`, bit 17 polled until set | mainline `JH7110_SYSRST_I2C5_APB 81`, `jh7110_sys_info` `.assert_offset = 0x2F8, .status_offset = 0x308`, `jh71x0_reset_update`'s `id / 32`, `BIT(id % 32)` and inverted status; vendor `RSTN_U5_DW_I2C_APB 81` |

Clock first, then reset, for milestone 220's reason: Linux's reset driver warns that a deassert
against a gated clock "might otherwise hang forever".

Driven by the tree. `pmic_bus` finds the AXP15060 by `compatible` (`x-powers,axp15060`, then
radon's `stf,axp15060-regulator`) and reads its parent bus's `clocks` and `resets`. It keeps a
specifier only when its provider is a SYS-domain controller this crate knows, that provider has one
cell per specifier, and the id is inside the SYS domain's bounds. On radon's tree that yields clock
143 and reset 81 and skips 298. A tree with no PMIC, or none usable, gets the constant plan with
`from_tree: false`, and the console line says which. Host tests hold all three against fixtures
transcribed from radon's U-Boot, mainline, and a hostile tree.

No effect anywhere else. QEMU's `virt` board names no JH7110, so nothing is recorded, mapped,
or printed, and the soak's reboot is unchanged. `script/soak-test --reboot --arch riscv64` passed on
2026-09-25 on this branch (see *Evidence*).

## The bench: one reset, watched

What calef does: build and boot the rebooting soak exactly as notes/soak.md already says
(`script/board-image --soak --reboot`, netboot, `script/board-console`), let it reach `t=120s`, and
watch one reset through. Nothing else is new. **If the board ends up hung or dark, one plug-2 cycle
recovers it** (plug 3 is garcia and must never be switched off).

Just before `rebooting now`, the new lines look like this on radon:

```text
soak-test-reboot: JH7110: bringing the PMIC's I2C bus back up first, ... SYS CRG at 0x13020000
                  (named by this machine's device tree); plan from the tree's own clocks and
                  resets of the PMIC's bus (1 specifier(s) skipped).
soak-test-reboot: JH7110: clock 143 0x........ -> 0x8....... (running)
soak-test-reboot: JH7110: reset 81 assert 0x........ -> 0x........, status 0x........ (released, N polls)
```

What each outcome means, decided before it runs:

| what the console shows after `rebooting now` | what it means | what to do |
|---|---|---|
| a second `U-Boot SPL` banner, a netboot, and `soak-test: started` | Built and proven. The reset line was the cause. Mark this milestone BUILT with the transcript, and milestone 249's series is available on radon. | Press a key once to see `DISARMED` (notes/soak.md's escape check), then leave it running. |
| the board goes dark and stays dark | The PMIC write worked, and radon's OpenSBI sets the power-off bit (bit 7) unconditionally before the reset bit, so the PMIC powered the board off rather than resetting it. The bus is fixed; the firmware's choice of bits is not. | One plug-2 cycle. Then the route is a nife AXP15060 write that sets bit 6 alone (option B) or firmware (option C). |
| `U-Boot SPL`, then it stops before netboot | The reset happened; something on the way back failed. Different bug, new record. | One plug-2 cycle; keep the transcript. |
| `i2c read: write daddr 36 to` ten times and `cannot read pmic power register`, with the three JH7110 lines above reading `running` and `released` | The bus was up when the kernel let go and the read still failed. Either OpenSBI's own poll timing against a controller at reset defaults, or pinmux. Option B is the next step. | One plug-2 cycle. |
| the same failure, with a JH7110 line reading `NOT running` or `STILL HELD` | The kernel's writes did not take: wrong window or wrong bit. A bug here, not in the firmware. | One plug-2 cycle; the before/after words in the transcript say which. |
| the same failure, and `the bus was already up` on the reset line | U-Boot's handover is not the cause; this milestone's premise is wrong for radon. | One plug-2 cycle; option B. |
| no `JH7110:` lines at all | `memory::init` did not recognize radon as a JH7110, so nothing was attempted. | One plug-2 cycle; the boot tour's `hw clock` line says why. |

## Options, as the proposal priced them

| | what | status |
|---|---|---|
| A | before the `ecall`, bring I2C5 back up from the kernel | built here, corrected to the reset line |
| B | nife writes the AXP15060 itself: a minimal DesignWare I2C master, and bit 6 alone | not started; the next step on two of the rows above |
| C | update radon's SPL and OpenSBI to a build with the upstream fixes | calef's call: writes the SPI flash of the only board of its kind |

## BUGS

- Nothing here has run on radon. Every register above is read from source, in trees that agree;
  none has been observed on this board. Milestone 220's clock driver has, on the STG window.
- The board test exit's shutdown takes the same road and is not changed. `arch::semihosting::exit`
  under `board` calls SRST shutdown, which on radon is the same PMIC write (notes/visionfive2.md,
  boot 15+). Preparing the bus there would make a board test run power radon off for real, which
  changes the bench workflow, so it waits for this milestone's bench result.
- A controller released from reset is at its hardware defaults, and radon's OpenSBI programs no
  timing, only the target address and the enable. The defaults are the DesignWare IP's synthesis
  parameters and are not published for the JH7110. If they are wrong for a 100 kHz bus, the
  fourth outcome row is the one that appears.
- Radon's OpenSBI may power the board off rather than reset it: it sets bit 7 before choosing
  between bits 6 and 7. That is the second outcome row; nothing in the kernel can change it short of
  option B.
- A provider with more than one cell per specifier ends the walk of that list, so a usable
  specifier after it is lost and the plan falls back to the constant. The safe direction, and no
  JH7110 tree has such a provider.

## Evidence

- Host tests: `cargo test -p jh7110_clock_and_reset -p device_tree_blob`, including
  `radons_pmic_bus_is_one_real_gate_and_one_reset_and_the_virtual_clock_is_skipped` and
  `finds_the_parent_of_a_compatible_node`.
- QEMU, 2026-09-25 on patagonia, `script/soak-test --reboot --arch riscv64`, on this branch rebased
  onto `ac2f2adfd`. The boot tour printed `hw clock : skipped (this machine's tree names no JH7110
  ...)`, no `JH7110:` line appeared before the reset, and the run ended:

  ```text
  soak-test: PASS, the machine reset and this kernel booted and began soaking again, 138s in
  soak-test: the route that reset it: attempt 1 of 1: SBI SRST system_reset, reset type 1 (cold reboot).
  ```

  The same run first failed with `starts=0` on a fresh worktree, because the riscv64 runner needs
  `target/nifefs.img` and the reboot path never built it. `xtask/src/soak.rs` now runs `mkdisk`
  there, as the job-mix path already did.

## Follow-on

- **Outstanding.** The one bench reset above, on radon, by calef. It turns this block BUILT or names
  which of options B and C comes next.

## Index row

radon's SBI cold reboot hangs because OpenSBI resets the board over I2C5 and U-Boot left that bus's reset asserted; OpenSBI also ungates the wrong clock. The kernel now ungates I2C5's clock and releases its reset, read from the device tree, just before the reset. It has not run on radon.
