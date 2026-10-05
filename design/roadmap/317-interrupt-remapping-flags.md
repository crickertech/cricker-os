---
status: BUILT
raised: 2026-09-17
built: 2026-09-17
---
# 317. The interrupt-remapping flags, and where MSI confinement actually lives

Minted by the maintainer on 2026-09-17 out of
[DECISIONS §86](../decisions/86-el0-nvme-driver.md)'s research pass, which found the gap while
pricing an EL0 NVMe driver. *(Number provisional until the merge queue lands it.)*

## What this is, and what it deliberately is not

**This makes interrupt confinement askable on all three architectures. It confines no interrupt,
and it does not decide who may.**

The milestone was cut as "add two runner flags" and the flags are the smallest part of it. What it
found is that MSI confinement lives in a different place on each of the three architectures, and
exactly none of the three is exercised today, for three different reasons, only one of which
anybody had written down.

| | where MSI confinement lives | what this machine offers | what this kernel does |
|---|---|---|---|
| `x86_64` | a separate IOMMU feature: VT-d interrupt remapping | **offered** (`ECAP.IR` set, and it always was) | never sets `GCMD.IRE` |
| aarch64 | a separate *device*: the GICv3 ITS | **absent** (`gic-version=2` has no ITS) | cannot reach the question |
| riscv64 | one mode field inside a device context this driver already writes | **offered** (`CAPS.MSI_FLAT` set) | writes `msiptp = 0`, mode Off |

The asymmetry is the finding. §86's survey covered two architectures and concluded the question was
unaskable; the third one has the mechanism sitting inside a structure this tree already programs.

An MSI or MSI-X message is a memory write to an architecturally special address, so an IOMMU doing
DMA remapping alone does not confine it: a component that can write a device's MSI-X table can aim
an interrupt at a vector it was never given. VFIO refuses to hand a device to an untrusted
userspace driver on a machine without interrupt remapping for this reason, and names its escape
hatch `allow_unsafe_interrupts`.

§86 found that no boot this tree runs could exercise the question even if a claim existed, and
`notes/confinement-claims.md` carries it as a claim stated nowhere. Two runner flags were named as
the cheap first move. This milestone is those two flags, the third architecture §86 did not look
at, and the one thing that makes any of it worth having: each machine description now says, from
**inside the guest**, what that machine offers.

Both of §86's findings were reached by reading the runner scripts, and one of them is wrong.
`x86_64` has been offering interrupt remapping in every boot this tree has ever run, and the only
reason nobody knew is that no code read the bit. aarch64's finding survives contact with the
machine, and the machine adds something to it. riscv64 was not surveyed at all.

**So the first deliverable is not a flag. It is that each architecture now reports its own position
from inside the guest**, in its machine description, rather than from a source comment. That is
what makes any of the rest checkable, and it is what turned two of the three answers around.

**Who owns the page holding the MSI-X table is not decided here.** calef held that decision on
2026-09-17 for want of an experiment behind it. This is the experiment.

## x86_64: the premise was false, and that is the finding

§86 said interrupt remapping is off in every x86_64 boot this tree runs. It has been on the
whole time.

That section reached its conclusion by reading `helpers/qemu-runner-x86_64.sh`, which attaches
`-device intel-iommu` with no `intremap=on`. The reading of the file is correct. The conclusion
drawn from it is not, and the difference only shows up when you boot the machine and read the
register. `ECAP` printed from inside the guest, QEMU 11.1.1, `q35` under TCG on patagonia:

| invocation | `ECAP` | `IR` (bit 3) |
|---|---|---|
| `-device intel-iommu` (the default, and what every boot before this ran) | `0xf00f4a` | **set** |
| `-device intel-iommu,intremap=on` | `0xf00f4a` | **set** |
| `-device intel-iommu,intremap=off` | `0xf42` | clear |

QEMU's `intremap` property is tri-state and defaults to `auto`, which resolves ON when there is no
in-kernel irqchip to conflict with. patagonia has no KVM, so it has always resolved ON. Adding
`intremap=on` is a no-op here. Nothing in this kernel read `ECAP.IR`, so nobody could have
noticed, which is the whole reason this lane's first job was to make the guest say what it sees.

### What the knob actually is, given that

`NIFE_INTREMAP` (provisional name) passes `intremap=` through; `on` and `off` are the only values
it accepts, and unset leaves QEMU's default. The useful value is `off`, because it is the only
way to reach a machine without the capability, and a comparison needs both sides. The default does
not move: unset is what the tree already ran, so this adds a knob and changes no existing boot.

The guest reports what it found, which is the part that makes any of this checkable:

    iommu           : VT-d drhd at 0x00000000fed90000, root table default-deny, translating, interrupt remapping offered (unused)
    iommu           : VT-d drhd at 0x00000000fed90000, root table default-deny, translating, interrupt remapping absent (unused)

`arch::x86_64::iommu::interrupt_remapping_available` is the query; `print_summary` and one test are
its callers. That test asserts `GSTS.IRES` is clear whatever `ECAP.IR` says, so the claim that
nothing here remaps an interrupt cannot quietly stop being true.

### The suite passes on both machines

    script/test --arch x86_64                      -> 0
    NIFE_INTREMAP=off script/test --arch x86_64    -> 0

A green suite proves nothing about remapping either way, which is milestone 202's hazard (every
confinement test is a ritual until somebody breaks the confinement). It proves only that removing
a capability nothing uses breaks nothing, which is what you would expect and is worth having
measured rather than assumed.

### `kernel-irqchip=split` is not needed here, and that is measured

The advice that pairs `intremap=on` with `-machine kernel-irqchip=split` is real and is a **KVM**
constraint: QEMU refuses interrupt remapping with an in-kernel irqchip. patagonia has no KVM, so
`q35` under TCG emulates the whole irqchip in the QEMU process and the check never fires. Started
with `-S` and quit from the monitor, so machine init runs and nothing executes:

    -machine q35 -device intel-iommu,intremap=on                      # starts, no diagnostic
    -machine q35,kernel-irqchip=split -device intel-iommu,intremap=on # starts, no diagnostic
    -machine q35,kernel-irqchip=on -device intel-iommu,intremap=on    # starts, no diagnostic

The runner does not add the flag. A Linux host running this suite under KVM would need it, and
adding it here would assert a machine fact nobody on this machine can check.

### Default-on versus opt-in, which the correction mostly dissolved

The question the brief asked was whether `intremap=on` should be the default. It is moot: it is
already the effective default, and stating it changes nothing. What is left is the real choice,
which is whether the knob defaults to `on`, `off`, or QEMU's own answer.

Unset, deferring to QEMU. Any other default would change every existing x86_64 boot to buy
nothing: pinning `on` re-states what already happens and would silently become a real change if
this suite ever ran under KVM, and pinning `off` would remove a capability from every boot to
protect against nothing, since no code reads it. The knob earns its keep as a way to ask a question
on purpose, not as a new default posture.

## aarch64: it cannot be switched yet, and here is precisely why

`gic-version=3` is one flag. Surviving it is milestone 227. QEMU accepts the machine; this
kernel does not survive it, and the failure is silent rather than loud, which is the part worth
recording.

Measured with `dumpdtb` on QEMU 11.1.1, both versions of the same `virt` machine:

| | `gic-version=2` | `gic-version=3` |
|---|---|---|
| node | `intc@8000000` | `intc@8000000` |
| `compatible` | `arm,cortex-a15-gic` | `arm,gic-v3` |
| `reg[0]` | `0x8000000 + 0x10000` (GICD) | `0x8000000 + 0x10000` (GICD) |
| `reg[1]` | `0x8010000 + 0x10000` (GICC) | `0x80a0000 + 0xf60000` (GICR) |
| MSI child | `v2m@8020000` | **`its@8080000`** |

The ITS is right there for the taking, which is what makes this tempting and what makes the rest of
the row matter.

`memory::init` finds the controller by the **`intc@` name prefix** and never reads `compatible`, so
it matches a GICv3 node just as happily, takes `reg[1]`, and hands it to `arch::aarch64::irq::init`
as "the CPU interface". On a GICv3 that region is the redistributor frame array, whose register
layout has nothing to do with a GICv2 CPU interface. Nothing anywhere refuses the mismatch.

`NIFE_GIC=3 cargo xtask boot-check --arch aarch64` (the flag is this milestone's, and the runner
prints a warning saying the configuration is unsupported):

    interrupts      : GICv2, distributor 0x0000000008000000, cpu interface 0x00000000080a0000
    self-test       : timer      FAILED  the 62MHz counter advanced 1250000 and 0 tick(s) arrived in ~20ms
    self-test       : scheduler  FAILED  thread 5 did not run within 2s (saw 0x0)
    nife self-test: 3 of 5 passed, 2 FAILED: timer scheduler

Against `gic-version=2`, the same command is 5 of 5.

Read the first line twice. The kernel says "GICv2" and prints the redistributor base under the
heading "cpu interface". It is not confused; it never asked. The machine description is stating a
belief, and on this machine the belief is false. That is the shape of failure this tree fears most:
a record that is wrong and looks like a reading.

### What milestone 227 would owe

Four things, and only the second is the large one its block warns about:

1. A machine that says which controller it has. `memory::init`'s `intc@` prefix must read
   `compatible` and distinguish `arm,cortex-a15-gic` from `arm,gic-v3`, and
   `arch::aarch64::irq::print_summary` must report what it found rather than the literal `GICv2` it
   prints today. **This is worth doing even if 227 never starts**, because it converts a silent
   wrong answer into a refusal; it is not done here only because it is 227's file and this lane is
   not 227.
2. The GICv3 driver. Distributor programming moves to affinity routing, the CPU interface moves
   from MMIO to system registers (`ICC_*`, which is real `arch/` work), and the redistributor gets
   a per-core frame with its own wake protocol. This is the large piece.
3. The ITS, which is the whole reason interrupt remapping wanted GICv3. Command queue, device
   table, interrupt translation tables, and `MAPD`/`MAPTI`/`INV`. Without it `gic-version=3` buys a
   different interrupt controller and no MSI translation, which is not what this question is about.
4. Parity's bill. DECISIONS §19 makes a kernel capability ship on every architecture or carry a
   scope note. x86_64 can offer interrupt remapping today and aarch64 cannot, so any claim built on
   it starts life with a recorded gap and this block is where it points.

## riscv64: offered, declined in one word, and untestable on silicon we own

**§86 did not survey this architecture, and it is the one where the mechanism is closest to hand.**

The RISC-V IOMMU puts MSI confinement *inside the device context this driver already writes*. There
is no separate feature to enable and no separate device to drive: a context in the extended format
carries `msiptp`, `msi_addr_mask` and `msi_addr_pattern` alongside the `iosatp` that
`arch::riscv64::iommu::attach` fills in on every attach.

**Measured from a boot, not from the source comment** (`CAPS` printed from inside the guest, QEMU
11.1.1's `virt`, `riscv-iommu-pci` as the runner already attaches it):

    CAPS = 0x78c2cf4f10    MSI_FLAT (bit 22) set, Sv39 (bit 9) set, version 0x10

So the comment in that file was right, and now something other than a comment says so:

    iommu           : riscv-iommu at 0x0000000040000000, device directory default-deny, translating, MSI page table offered (mode off)

The extended 64-byte context is the live path, and `attach` writes those four words zero,
which is `msiptp.MODE = Off`. No MSI page table is allocated. Nothing programs an entry in one.
The capability is offered and declined, the same posture `x86_64` is in, reached by a completely
different route.

### Does riscv64 need a flag?

No, and that is a complete answer. `-device riscv-iommu-pci` as the runner already spells it
reports `MSI_FLAT`, so the capability is present with no option to add, and unlike `intel-iommu`
there is no property to turn it off. Both sides of the `x86_64` comparison exist there because
QEMU offers `intremap=off`; here only one side exists, and no flag this lane could add would
create the other. `helpers/qemu-runner-riscv64.sh` is unchanged by this milestone.

### The inversion, which is the sharpest thing here

`arch::riscv64::iommu`'s own comment says the base 32-byte format is kept "because real silicon
without MSI support will report otherwise." That branch has run zero times, and cannot be made to
run. Milestone 143's block states the hardware fact plainly: no RISC-V SoC on the market ships
the ratified IOMMU specification, and radon (the `VisionFive` 2, JH7110) predates it and has no
IOMMU at all.

So the three architectures are not merely in different places, they are in different places *along
two axes*:

| | emulated | on silicon this project owns |
|---|---|---|
| `x86_64` | capability offered, unused | xenon (`OptiPlex` 7050) has a real VT-d unit, unmeasured |
| aarch64 | capability absent (GICv2) | argon (Jetson TX1) predates the question here too |
| riscv64 | capability offered, unused | **no such silicon exists** (milestone 143) |

riscv64 is confined in the case we can test and untestable in the case we cannot, which is the
inverse of `x86_64`, where the emulated answer is easy and a second witness is sitting on a shelf.
Any claim built on MSI confinement inherits that shape, and DECISIONS §19's parity gate is what
will ask about it.

### What is a reading rather than a measurement

`msiptp.MODE = Off` means, per the RISC-V IOMMU specification, that MSI address translation is not
performed and the transaction is translated as an ordinary memory access. If that is right, an
MSI-shaped write from an attached device is still bounded by its `iosatp` domain rather than
escaping it, which would make riscv64's position genuinely different from `x86_64`'s, where VT-d
intercepts the MSI address range and DMA remapping does not cover it.

Nobody here has tested that, and this tree carried a fabricated block quote for twelve days, so
it is marked rather than asserted. It is recorded in the driver's own `BUGS` section beside the
code, which is where a reader meets it.

## EXAMPLES

Run the x86_64 suite against a unit that does **not** offer interrupt remapping, which is the side
of the comparison that needed a flag to reach:

    NIFE_INTREMAP=off script/test --arch x86_64

See what the guest makes of the machine, both ways:

    cargo xtask boot-check --arch x86_64                    # ... interrupt remapping offered
    NIFE_INTREMAP=off cargo xtask boot-check --arch x86_64  # ... interrupt remapping absent

The `iommu` line is in `target/boot-check-x86_64.log`.

Ask each machine where it stands, which is the one command that makes this milestone checkable:

    cargo xtask boot-check --arch x86_64   # ... interrupt remapping offered (unused)
    cargo xtask boot-check --arch aarch64  # interrupts : GICv2, ... (no ITS to report)
    cargo xtask boot-check --arch riscv64  # ... MSI page table offered (mode off)

Reproduce aarch64's position in one command:

    NIFE_GIC=3 cargo xtask boot-check --arch aarch64      # 3 of 5, timer and scheduler dead

Compare the device trees the two GIC versions produce, which is how the table above was made:

    qemu-system-aarch64 -machine virt,gic-version=3,iommu=smmuv3 -cpu cortex-a72 \
        -display none -dumpdtb=/tmp/gic3.dtb
    dtc -I dtb -O dts /tmp/gic3.dtb | grep -A 12 'intc@'

## BUGS

- Nothing here remaps an interrupt, and the reported bit only says the hardware offers it. No
  `IRTE` is allocated or written, `GCMD.IRE` is never set, and no MSI is forged and traced to a
  vector it was not given. `interrupt_remapping_is_reported_and_never_enabled` asserts `GSTS.IRES`
  is clear precisely so this sentence cannot quietly stop being true, but an assertion that
  remapping is *off* is not evidence about what remapping would *do*.
- DECISIONS §86 and `notes/confinement-claims.md` both carry the false half of the premise, and
  a lane may not edit the former. §86's "interrupt remapping is off in every x86_64 boot this tree
  runs" needs the correction above; the note's copy of the same sentence has it already. This is
  flagged for the integrator rather than fixed here.
- **riscv64's `MODE = Off` semantics are a spec reading, not a measurement.** What the hardware
  does with an MSI-shaped write from a device whose context has no MSI page table is stated in the
  specification and has not been observed here. It is flagged in
  `kernel/src/arch/riscv64/iommu.rs`'s `BUGS` as well, where a reader meets the code.
- riscv64's base 32-byte device-context branch has run zero times and cannot be made to. No
  silicon ships the ratified RISC-V IOMMU (milestone 143), so the `MSI_FLAT`-absent path in
  `attach` is dead code kept for a machine that does not exist. Recorded rather than removed,
  because deleting it would assert that no such machine ever will.
- So the confinement claim is still stated nowhere, and `notes/confinement-claims.md` still
  carries it as such, now pointing here for the flags. What would settle it is a test in milestone
  202's shape: give a component the MSI-X table page, have it aim an interrupt at a vector it was
  never granted, and assert the hardware refuses. That needs a driver outside the kernel that wants
  interrupts, which is §86's question and not this one.
- `NIFE_GIC=3` produces a boot that is known-broken, on purpose. It exists so milestone 227
  starts from a reproduction rather than a rediscovery. It is not a supported configuration, no
  gate runs it, and the runner warns on every use. If it ever stops failing, something in the GIC
  path changed and this block is stale.
- **The correction is about QEMU under TCG on one machine, not about VT-d.** `intremap`'s `auto`
  resolving ON is QEMU's behaviour with no in-kernel irqchip. Under KVM it resolves differently and
  the original reading could well be right there. Nobody has checked what the CI Linux runners do,
  and milestone 87's `OptiPlex` (xenon) is real silicon whose firmware answers this question in its
  own way.
- **`NIFE_INTREMAP` and `NIFE_GIC` are provisional names.** Names are an architect's
  (`AGENTS.md`); these follow the existing `NIFE_*` runner-knob convention (`NIFE_SMP`, `NIFE_CPU`,
  `NIFE_EL2`, `NIFE_DISK`) and nothing outside this repository has acted on either.
- The x86_64 measurement is one machine. QEMU 11.1.1, TCG, macOS on Apple Silicon, no KVM. The
  `kernel-irqchip` finding above is specifically a statement about that machine, and the CI Linux
  runners have not been checked.
- **`ECAP.IR` is a capability bit, not a working unit.** It says QEMU's model advertises interrupt
  remapping. Whether that model's remapping behaves like silicon's is a question no boot here asks,
  and milestone 87's `OptiPlex` (xenon) is where it would get a second witness.

## Follow-on

- **Recorded.** That nothing here remaps an interrupt and that the reported bit only says the
  hardware offers it, in `kernel/src/arch/x86_64/iommu.rs`'s `BUGS` section beside the driver, and
  in this block's own `BUGS`.
- **Decision.** `design/decisions/86-el0-nvme-driver.md` states that interrupt remapping is off in
  every x86_64 boot this tree runs, and the machine says otherwise. A lane does not edit
  `design/decisions/`, so the correction is recorded here, in the driver's `BUGS`, and in
  `notes/confinement-claims.md`, and the integrator owes §86 the amendment.
- **Recorded.** That the confinement claim about where a device may *interrupt* is still stated
  nowhere, in `notes/confinement-claims.md`, whose entry now carries what the flags measured and
  what is left.
- **Milestone 227.** The GICv3 driver and its ITS. This block's "What milestone 227 would owe"
  section is the bill, and `NIFE_GIC=3` is the reproduction; item 1 there (read `compatible`, and
  report what was found rather than a hardcoded `GICv2`) is worth doing on its own and is not done
  here because it is 227's file.
- **Decision.** `design/decisions/86-el0-nvme-driver.md`: who owns the page holding the MSI-X table.
  calef held it on 2026-09-17 for want of an experiment behind it, and this milestone is that
  experiment. It is now answerable on x86_64 without any further machinery, and the honest cost of
  answering it on aarch64 is milestone 227.
- **Recorded.** riscv64's position, in `kernel/src/arch/riscv64/iommu.rs`'s new `BUGS` section
  beside `attach`: the MSI page table is offered (`CAPS.MSI_FLAT`, measured), every device context
  is the extended 64-byte format, and all four MSI words are written zero. The `MODE = Off`
  semantics are marked there as a spec reading rather than a measurement.
- **Recorded.** That riscv64's base 32-byte context branch is unreachable on any silicon that
  exists, in the same `BUGS` section, pointing at milestone 143's hardware gate.

## Index row

An MSI is a memory write to a special address, so DMA remapping does not confine it and a component
that can write a device's MSI-X table can aim an interrupt at a vector it was never given. DECISIONS
§86 surveyed two architectures from the runner scripts and concluded the question was unaskable
here. Booting all three says something different: MSI confinement lives in a different place on
each of them, and none of the three is exercised. `x86_64` has had it offered in every boot this
tree ever ran (QEMU's `intremap` defaults to `auto`, which resolves ON with no in-kernel irqchip),
and nothing read the bit; `NIFE_INTREMAP=off` is the flag that reaches the machine without it, and
a test asserts `GSTS.IRES` stays clear whatever `ECAP.IR` says. riscv64 was never surveyed and is
the closest of the three: `CAPS.MSI_FLAT` is set, every device context is the extended format, and
`attach` writes `msiptp = 0`, so the mechanism is one word inside a structure this driver already
programs, and no flag is needed or available. It is also the one architecture whose answer can
never get a second witness, since no silicon ships the ratified IOMMU (milestone 143), which
inverts `x86_64`, where xenon is waiting. aarch64's half of §86 survives, and turned out not to be
one flag away, and finding out how was the point: `gic-version=3` moves `reg[1]` from the CPU
interface to the redistributor, `memory::init` matches the node by the `intc@` name prefix and never
reads `compatible`, and the result is a boot that prints `GICv2, cpu interface 0x80a0000` and
receives no timer tick. Milestone 227's bill is itemised here, and §86's MSI-X ownership decision is
now answerable on x86_64 without further machinery.
