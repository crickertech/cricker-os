---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 241. What a fourth board would have to be for, so that GICv3 is bought rather than justified

Minted 2026-09-03 by calef, from asking the right question about milestone
227 (a GICv3 driver, because GICv2 boots and silently loses every interrupt): not *should we build
the driver* but *what board would force it, and what would that board prove*. *(Number provisional
until the merge queue lands it.)*

Deferred rather than open. calef, 2026-09-05: *"On the fourth board,
making the decision and making the purchase are two different things. But I think we defer both
because we don't have enough to know what we need from a fourth board to drive a decision."*

The decision this gate names is calef's own, it is dated, and it is quoted above, which is
why milestone 435's sweep leaves the token where it is and mints no `design/decisions/` section for
it. This block is one of the 45 that sweep counted as naming no section and no `§`, and it is a
false positive of that count rather than a gap: what a numbered section would hold (what is being
decided, the options, the recommendation, what is blocked) is already here, and the status of it is
*answered with "not yet"* rather than *pending*. Filing a deferral as `**Status: PROPOSED.**` would
put a question calef has closed back on the queue that exists to show him what is open.

One premise has changed since this was written, and it is half the trigger below. This block
says argon and xenon have never booted. xenon has: first light 2026-09-05, and
`nife self-test: 5 of 5 passed` at 09:55 UTC on 2026-09-17, which turned milestone 87 `BUILT`
(`bench/xenon-2026-09-17/first-light-095500.log`). **argon still has not**; `notes/target-hardware.md`
records it as *"in hand"* and nothing in `bench/` carries its name. So the trigger's precondition is
half met and the block's own argument for the discipline stands unchanged: the machine that has
never printed a byte is still the one a fourth board would be bought ahead of.

The separation is his correction and it matters. The maintainer had called this block a purchase,
which is what its own gate line said. It is not: the title asks what a fourth board would have to
be for, which is criteria, and a purchase is downstream of criteria. Conflating them makes the
cheap half look as irreversible as the expensive half, and that is backwards. Criteria can be
written and rewritten; only the money cannot be recalled.

**Both are deferred for one reason: the requirement does not exist yet.** Nothing in the tree says
what a fourth board must prove that the three already owned do not. Deciding criteria without that
would be inventing a requirement to justify a driver, which is the exact failure this block was
minted to prevent, one level up: *"a driver bought to justify a board"* becomes *"criteria written to
justify a driver"*.

What would make it decidable, and none of these has happened:

- A fatal risk needing it. Milestone 227's own block says it is **not on any fatal risk's
  critical path**, and that is still true.
- A parity gap the three boards cannot close. §19 makes parity a gate, and today argon, radon
  and xenon cover the three architectures; a fourth board earns its place by proving something none
  of them can.
- Accelerated testing on patagonia mattering enough to buy for. HVF requires GICv3, so there has
  been no accelerated coverage on the development machine since milestone 222. That is the strongest
  live argument and it is a convenience, which this block already says is *"an option, not a
  claim."*

Recorded rather than closed, because the question is good and the answer is only "not yet". The
trigger is any one of the three above.

In brief. Milestone 227 is real, priced, and **not on any fatal risk's critical path**, which its
own block says. It restores accelerated testing on patagonia (HVF requires GICv3, so since milestone
222 there is no accelerated coverage on the development machine at all) and opens most modern
aarch64 boards. That is a convenience and an option, not a claim.

So GICv3 is a means and this block is about the end. A driver bought to justify a board, or a
board bought to justify a driver, is the same mistake twice.

## The three claims a fourth machine could test that these three cannot

1. Is a second board of a *known* architecture cheap? This is fatal risk 9's (the HAL is a
fiction, and an architecture costs a restructure rather than a port) stronger form and it has never
been tested. argon, radon and xenon are one board per architecture, so the tree has
proved three times that a new architecture is a new directory and **not once** that a second board
within an architecture is nearly free. That is the claim a stranger actually cares about, and GICv3
is precisely what would make the answer *no* today. Either result is evidence.

2. Asymmetric cores. Every machine here is homogeneous: four A57s, four U74s, and the OptiPlex's
identical cores. **Fatal risk 5 (it cannot be made reliable on multicore, and the bugs appear only on
silicon) has never met cores that are not interchangeable**, and this kernel deliberately does not
rebalance (DECISIONS 138, how a saturated workload is made to hand threads across cores). On
2026-09-03 two soak runs on four *identical* cores differed eightfold by placement alone (milestone
240, the soak reports what happened and not where). Asymmetry is the harder version of the same
question.

3. An IOMMU that exists in silicon. Milestone 143's (the IOMMU on real silicon) gate is blunt:
*"The board does not exist. No RISC-V SoC on the market today ships the ratified"* IOMMU. So radon
can never test fatal risk 6's (a capability-confined userspace driver cannot drive real hardware at
real speed) confinement half on hardware. xenon has VT-d and milestone 195 (finish the UEFI boot
path) exercised it, but that is one vendor's answer. SMMUv3 on a real aarch64 board is the second
independent confirmation, and SMMUv3 boards are the GICv3 generation.

A board with GICv3, SMMUv3 and asymmetric cores serves all three at once, which is what makes
this worth a block rather than a shrug. Rockchip's RK3588 (four A76 plus four A55) and NVIDIA's
Jetson Orin family both fit. Orin has one extra property: it is argon's successor, which
separates *what does one generation cost* from *what does a different vendor cost*. Neither has been
surveyed with the rigour `notes/aarch64-board-survey.md` applied to the TX1, and that survey is where
the work would start.

## The trigger, which is the point of writing this down

Buy nothing yet. This project owns two machines that have never run nife: argon has never booted
and xenon has never booted. Acquiring a fourth board before the second and third have printed a
byte would be buying evidence we have not collected.

2026-09-03 is the argument for that discipline. radon has booted for weeks, and only that afternoon
did anyone learn its device tree omits the TRNG (milestone 239, radon's device tree does not describe
the TRNG, so a working driver never runs), a fact which then explained a second number nobody had
questioned. Boards teach you things only once you run them.

So: buy the fourth board when argon and xenon have both booted, and one of these is true:

- a customer path needs hardware this project does not have;
- fatal risk 5 cannot make further progress without asymmetric cores;
- fatal risk 6 needs a second IOMMU vendor to be credible.

Until one fires, milestone 227 stays held and this block is the reason.

## The argument this block did not have when it was written

calef, the same afternoon, on booting from a USB stick:

> that opens up a lot of different hardware in our house that I can use for testing: Graeme's
> laptop, his desktop, cordoba, two MacBooks, Clay's desktop
>
> -- calef, 2026-09-03

That is a stronger answer to this block than the trigger above. Milestone 87 (the x86_64
bare-metal machine) boots from a FAT32 stick at `\EFI\BOOT\BOOTX64.EFI`, the removable-media
fallback every UEFI firmware looks for with no configuration, and nothing about it is specific to
xenon. So the fourth machine, and the fifth and sixth, may already be in the house.

Six machines with six firmwares, chipsets and core counts is a far better test of claim 1 above,
whether a second board of a known architecture is cheap, than any single purchase would be. It costs
a USB stick.

What it does not give, and this is why milestone 227 (a GICv3 driver, because GICv2 boots and
silently loses every interrupt) stays held rather than dying: those machines are all x86_64. They
say nothing about GICv3, nothing about SMMUv3, and nothing about asymmetric cores. Claims 2 and 3
still want an aarch64 board this project does not own. Claim 1 no longer does.

And the blocker on using them is not a driver, it is milestone 243 (a machine with no serial port
has no way to say anything, and no gate can read it): those machines have no serial port, so nife
would boot and say nothing after the loader.

## BUGS

- This block does not survey the candidates. RK3588 and Orin are named from general knowledge and
  neither has been checked the way `notes/aarch64-board-survey.md` checked the TX1: bootloader
  access, documented peripherals, a reachable serial console. Treat both as leads, not
  recommendations.
- It assumes the fourth board is aarch64, because that is where GICv3 and SMMUv3 live. A second
  riscv64 board or a second x86_64 machine would test claim 1 as well and claims 2 and 3 not at all.
- A trigger nothing checks is still rung four. Nothing will fire when argon and xenon boot; a
  person has to notice and reread this.

## Index row

two machines here have never booted nife, so the trigger is written down rather than the purchase
made
