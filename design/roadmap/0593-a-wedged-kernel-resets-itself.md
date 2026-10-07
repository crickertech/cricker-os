---
status: PARTIAL
raised: 2026-09-25
promoted_from: a-wedged-kernel-resets-itself
milestone_dependencies: none
decision_dependencies: none
machine_requirements: riscv64 and aarch64 silicon with a hardware watchdog
specific_machine: none
needs_person: yes
---
# 593. A wedged kernel resets itself, through a watchdog that does not need asking

Promoted from the proposal `a-wedged-kernel-resets-itself` on 2026-09-25, and
its first step built the same day on `milestone/593-a-wedged-kernel-resets-itself`. *(Number
provisional until the merge queue lands it.)* The proposal was raised by the lane that brought
milestone 249 (the boot lottery is sampled by a person walking to the board)'s reboot to all three
architectures. That milestone resets a kernel that is working and chooses to. A kernel that has
wedged cannot call `arch::reboot`, and a wedge is what the soak (milestones 219 and 225) looks for.

xenon's step is built and proven under QEMU; what is left needs a person at a
board. radon's and argon's steps have no QEMU model to prove them on, and argon also waits on its
first nife boot.

## The shape

The soak arms the board's watchdog when its beat begins and pets it on every beat, every five
seconds. A kernel that stops beating stops petting, and the board resets itself. The reset follows
the stall dump the soak already prints and never replaces it: a failing beat panics before the pet,
so the dump is on the console before the timer runs out. A build that arms the watchdog says so in
its banner, the way `reboot_soak_test` does.

## Step 1, xenon's Intel TCO: built 2026-09-25

`kernel/src/arch/x86_64/tco.rs` is the driver. `find()` reads the chipset's configuration space and
returns the TCO it found, holding its I/O base. It knows two layouts. The ICH9 that QEMU's `q35`
emulates keeps the TCO at `PMBASE + 0x60` and `NO_REBOOT` in `RCBA + 0x3410`. The 100/200-series
PCH that xenon's Q270 belongs to keeps it at the SMBus function's `TCOBASE` and `NO_REBOOT` behind
the P2SB sideband. `arm(60)` loads 50 ticks of 0.6 s. The chipset counts them twice before it
resets, so the reset lands 60 s after the last pet. The driver's header cites what was read: Intel's
200-series datasheet volume 2, Linux's `iTCO_wdt`, `lpc_ich`, `i2c-i801` and `p2sb`, and QEMU's
`ich9_tco`.

`--features watchdog_soak_test` arms it from `kernel/src/soak.rs`. `--features wedge_soak_test` is
test only: thirty seconds in, the supervisor masks interrupts on its core and spins. On any
architecture but `x86_64` the feature is a `compile_error!`, so no build can quietly fail to arm.

Two checks in `script/soak-test` prove it under `q35`, both measured on patagonia on 2026-09-25:

| run | passes when | result |
|---|---|---|
| `--wedge --arch x86_64` | the start line appears again after the wedge, and the second boot reads `SECOND_TO_STS` set | PASS, back 63 s after the wedge |
| `--watchdog --arch x86_64` | nothing resets the machine for 180 s after arming, and a pet saw the count below its reload value | PASS, every pet saw 41 of 50 ticks left |

The first uses milestone 249's criterion, a second start line, and adds one fact. `SECOND_TO_STS`
is set only by the chipset's second timeout and cleared only by a write or `RSMRST#`. So the second
boot can say the watchdog reset it, not a triple fault or a stray `0xcf9` write. The second check's
counting condition matters as much. A halted TCO reads its reload value forever, and surviving a
window proves nothing unless the timer ran.

The wedge check was made to fail once, on purpose. With QEMU's no-reboot strap set
(`-global ICH9-LPC.noreboot=true`, a one-off runner edit), it failed at its 200 s deadline with the
wedge on the console. That run also found a limit: the driver still printed `ARMED`, because
`NO_REBOOT` read back clear while the strap held it. So an `ARMED` line does not prove a machine
resets. Only a wedge that comes back does.

CI runs both in the `watchdog` job, only when a path they depend on changes, and always on `main`.

## What needs calef

- Decided by calef, 2026-09-26 (UTC): *"A soak should reset itself when it freezes."* This
  answered whether a soak should reset itself over its own wedge at all, and went further than the
  recommendation it answered ("allow it, as built"): resetting is wanted, not merely allowed. The
  follow-on it sets is recorded under Follow-on below.
- xenon's POST settings. `notes/xenon-firmware.md` records `Prompt on Warnings and Errors` and
  keyboard error detection on. A watchdog reset that stops at a POST prompt has recovered nothing.
  Changing them is a firmware change and stays off until calef says otherwise.

## xenon's first bench step

This lane found that a soak kernel could not reach xenon's stick, because the UEFI loader's seal
check refused it. Milestone 563 (a seal check that reads bytes cannot see a check that was dropped)
closed that gap. After rebasing onto it, `cargo xtask uefi-image --features watchdog_soak_test`
builds on patagonia (2026-09-26), and `uefi_loader/build.rs` panics `NOT SEALED` on any unsealed
pair, so the image is sealed.

The step is one boot with the POST settings unchanged. Build a `watchdog_soak_test`
image, boot it with the serial chain on the desk, and read three lines. The `found the Intel TCO`
line says whether step 1's discovery agrees with the datasheet. A `NOT ARMED` reason says firmware
holds `NO_REBOOT` visibly; an `ARMED` line cannot rule out a strap. The first `petted` line says whether the
count moves. Only then run a wedge build, and watch whether the machine comes back or stops at POST.

## Later steps

| board | part | how it resets | testable in QEMU |
|---|---|---|---|
| radon | JH7110 watchdog at `0x1307_0000`, SP805-like, unlocked with `0x1ACCE551`; clocks syscrg 122 and 123, resets 109 and 110. Linux `starfive-wdt.c` | interrupt on the first expiry, reset on the second; about 357 s at most | no |
| argon | Tegra X1 WDT0 at `0x6000_5100`, paired with timer 5, 1 MHz. Linux `tegra_wdt.c`, which binds only `tegra30-timer`, so the layout is an inference to check against the TRM | chip reset through the PMC on the fourth expiry | no |

radon's is second. Its reset does not go through OpenSBI's `pm-reset`, so it may come back where
SRST does not. That is untested, and the first watchdog boot on radon is the test.

## BUGS

- The PCH path has never run. q35 is an ICH9, so the SMBus lookup, the P2SB unhide and the sideband
  `NO_REBOOT` write run first on xenon. Each prints what it found.
- `TCO_EN` in `SMI_EN` is left as firmware set it, following Linux. If firmware's SMI handler pets
  the timer, a wedged xenon never resets; that is the second thing to check after `NO_REBOOT`.
- A reboot soak and a watchdog soak in one build are not guarded against. Under QEMU the TCO keeps
  counting across a `0xcf9` reset, so the second boot could be reset before it arms.

## Follow-on

- **Outstanding.** x86_64 soak builds include the watchdog by default, per calef's 2026-09-26
  ruling. An explicit opt-out stays for the no-reset (wedge) check in `script/soak-test`. Checked on
  this branch: `script/soak-test` and `xtask/src/soak.rs` still arm it only for `--watchdog` and
  `--wedge`. It is not built here because it changes how every soak build picks its features, and
  `watchdog_soak_test` is a `compile_error!` off x86_64.
- **Milestone 563.** A soak kernel could not reach xenon's stick. That gap was folded into milestone
  563 and built there.

## Index row

A soak arms the Intel TCO watchdog on x86_64 and pets it every beat, so a wedged kernel is reset by
the chipset 60 s after its last pet. Proven under QEMU q35 both ways: a deliberate wedge resets and
the second boot reads the chipset's own record of it, and a healthy soak survives 180 s. Off by
default; xenon, radon and argon are bench steps.
