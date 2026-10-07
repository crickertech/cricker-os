---
status: BUILT
raised: 2026-08-23
built: 2026-09-04
---
# 159. A real hardware entropy source: the JH7110's TRNG

Minted 2026-08-23, surfaced while investigating milestone 49's boot-wiring
fork (DECISIONS §120): the entropy service (milestone 56, `BUILT`) only has a virtio-rng backend,
which exists in QEMU and not on the VisionFive 2 the tree already boots (milestone 16a). Checked
before minting: the StarFive JH7110's TRNG is already named as the real-hardware candidate in two
places (`notes/entropy.md`, milestone 56's own doc) and tracked by no milestone in either.

The driver's own logic can be written and host-tested without silicon; whether it actually produces
usable entropy, and at what rate, can only be verified by reading real bits off a real TRNG.

**Status deliberately did not move on 2026-08-24, and again on 2026-09-01, and this section is
both lanes' report of why**, per this directory's own rule that a branch touching
nothing else in this file must at least say so.

The first lane (2026-08-24) confirmed the TRNG from documentation, wrote and host-tested its
register and DTB-discovery logic (`crates/jh7110_entropy`), and wrote an unwired driver program
(`components/src/jh7110_entropy.rs`). The second lane (2026-09-01) wired that program to a spawner, gave it
a boot-tour step that says something falsifiable, and moved the one remaining piece of untested
logic into the host-tested crate. Neither clears this tree's bar. Milestone 53's `PARTIAL`
names a phase that runs end to end, proven in QEMU; the phase here is "read real bits off a real
TRNG", and there is no JH7110 TRNG in QEMU to run it against, so what runs in QEMU is the
*absence* path. `NOT-STARTED` ("specified, nothing built") is not right either, and it is less
right than it was; it is still the closer of the two available tokens, and the honest gap is
recorded here rather than folded into a status word this vocabulary does not have.

## What the second lane built (2026-09-01), and what it proves

The device is on **radon**, the `StarFive` VisionFive 2 (JH7110, riscv64). radon booted nife with
userspace on 2026-09-01, reaching `init/build`, `device IRQ` and four cores online, so a userspace
process holding a capability runs there; what has never happened is one talking to a real
non-virtio device.

- A spawner. `kernel/src/user/entropy_service.rs` grew `Bus::Jh7110`, beside `Mmio`, `Pci` and
  `Instruction`. **The fork the first lane held open turned out not to be one**: it declined to add
  the variant because "adding one needs a real decision about how the service that wires it locates
  this binary in an initrd", and `entropy_service::ensure` already takes the program's bytes from
  its caller, exactly as it does for `entropy`. The caller reads `user::program("jh7110_trng")`.
  Nothing about the interactive boot changed, so DECISIONS §120's stopgap question is untouched.
- The authority, which is the risk-6 claim. The driver is granted two rendezvous capabilities
  (a request endpoint it RECVs on, a readiness endpoint it SENDs once) and one page of device
  memory: the TRNG's register block, device-typed. No DMA page (the device writes nothing to
  memory), no `Irq` capability (it polls), no `Virtio` capability (there is no transport). The
  binding's `reg` window is `0x4000` and the spawner maps `0x1000` of it, because
  `jh7110_trng::regs` reaches only `0x68`. An earlier draft of the program named a third slot for a
  `DeviceFrame` capability; the spawner that exists grants the page as a `Mapping`, so the slot was
  describing something nobody hands over, and it is gone.
- A falsifiable boot-tour line. The riscv64 tour's new `hw entropy` step plays a client over
  the same request endpoint any client would hold, draws 32 bytes twice, and prints the success
  line **only** when the bring-up report says `READY`, both draws come back full, the first is not
  all zeros, and the two differ. Anything else prints `FAILED` with the numbers. The kernel never
  reads a `RAND` register.
- The skip, which is what CI actually exercises. QEMU's riscv64 `virt` board has no
  `starfive,jh7110-trng` node, so on every machine this repository boots the step prints
  `hw entropy  : skipped`. `kernel::user::entropy_tests`'
  `the_jh7110_backend_refuses_to_wire_where_there_is_no_jh7110` pins that the wiring *refuses*
  rather than spawning a driver holding a device mapping of an address nobody named.
- The buffer, moved somewhere testable. `jh7110_trng::Pool` is the one part of the driver that
  can serve a byte twice, lose the seam between two generations, or pad a short answer with zeros a
  client would mistake for entropy, and none of that is visible in the register decode. It is now
  in the crate with five host tests (no byte served twice across ten generations, a request
  straddling the seam, a dry device shortening the count rather than padding, a served byte zeroed
  behind the cursor, an oversized request clamped). Writing the first of them found `take` shifting
  left by `8 * n` for unclamped `n`: unreachable through the wire format, because
  `entropy_proto::want` clamps to 8 first, but a real overflow at the API. `take` clamps, and a
  `const` assert in the program ties the two 8s together.

What none of that establishes. Nothing here has read a bit off a TRNG. Every register offset is
still transcribed from Linux's driver rather than observed, the polling bounds are still guesses
with no board measurement behind them, and the rate question this milestone opens ("a rate and
quality argument, measured rather than assumed") is untouched, because a rate cannot be measured
against a device that has not run.

## The bench procedure, in order

Written to be followed without rederiving anything. It needs radon powered on (the Kasa plug), a
serial console attached before power, and a microSD card. See `notes/visionfive2.md` for the board
facts this leans on and its failure-triage ladder for everything that goes wrong before the tour.

1. On the dev machine, from a checkout with this milestone's work on it:

   ```
   script/board-image
   ```

   It builds the userspace archive first and the kernel second, in that order, because packing the
   archive regenerates the measured-boot manifest the kernel compiles in.

2. Copy all three files to the card. The archive is **not** optional, whatever the script's own
   printed instructions say (that wording is milestone 217's, and a stale pair already cost a boot
   with `MEASURED BOOT REFUSED`):

   ```
   target/board/nife-vf2.img          -> /Volumes/NIFE/nife-vf2.img
   target/board/nife-initrd.img       -> /Volumes/NIFE/nife-initrd.img
   target/board/extlinux/extlinux.conf -> /Volumes/NIFE/extlinux/extlinux.conf
   ```

   The kernel and the archive must be from the same `script/board-image` run. A mismatched pair
   halts at `MEASURED BOOT REFUSED` before any of this milestone's code runs.

3. Insert the card, DIP switches to QSPI, serial at 115200 8N1, then power. Interrupt U-Boot's
   countdown (the extlinux path does not work; that is milestone 218) and type these five, which
   are what `script/board-image` prints:

   ```
   StarFive # load mmc 1:1 ${kernel_addr_r} /nife-vf2.img
   StarFive # load mmc 1:1 0x90000000 /nife-initrd.img
   StarFive # fdt addr ${fdtcontroladdr}
   StarFive # fdt move ${fdtcontroladdr} 0x86000000
   StarFive # booti ${kernel_addr_r} 0x90000000:${filesize} 0x86000000
   ```

4. Read the `hw entropy` line, which is the last line of the tour before the banner. It is one
   of five, and each one means a different thing:

   | Line | What it means |
   |---|---|
   | `hw entropy  : JH7110 TRNG at 0x1600c000 served 32+32 bytes ... second differs; STAT after init 0x........ (256-bit: ...)` | **The milestone.** A confined userspace process drove a real non-virtio device and a client got bytes through a capability naming no device. Record the whole line; it is the first such number this project has. **Read the mode note before believing the byte count**: it must say 256-bit. |
   | the same line, but the mode note says `128-BIT` | The device ignored the `MODE.R256` write, so only `RAND0..RAND3` are a generation's answer and **16 of every 32 bytes are not device output**. The draws still differ and the tour still calls it a pass, because the tour cannot tell. This needs the driver to serve 16 bytes per generation instead of 32; nothing is wrong with the capability story. |
   | `hw entropy  : skipped (this machine's tree describes no starfive,jh7110-trng; ...)` | The board's own device tree has no TRNG node where mainline's binding says it should be. **Capture the tree** (see step 6) rather than guessing; this is the one fact the first lane flagged as unconfirmed. |
   | `hw entropy  : JH7110 TRNG at 0x..., but no 'jh7110_trng' in the initrd` | The card has a stale archive. Redo step 2 with a matched pair. |
   | `hw entropy  : FAILED: ... bring-up diagnostic 0x0000000000000000 ...` | The register window read as nothing. Most likely the block's clocks are gated or its reset is not deasserted (see the driver's `BUGS`), and next most likely the base address is not the TRNG. |
   | `hw entropy  : FAILED: ... bring-up diagnostic 0x<nonzero> ...` | The device answered and the sequence is wrong. The high 32 bits are `STAT` and the low 32 `ISTAT`. `STAT`: bit 3 `R256`, bit 8 `MISSION_MODE`, bit 9 `SEEDED`, bits 16-18 `LAST_RESEED` (`0x7` means unseeded/zeroized), bit 27 `SRVC_RQST`, bits 30/31 generate/seed in flight. `ISTAT`: bit 0 `RAND_RDY`, bit 1 `SEED_DONE`, bit 2 `AGE_ALARM`, bit 3 `RQST_ALARM`, bit 4 `LFSR_LOCKUP`. Record the raw word. **Every bit above is decoded in `crates/jh7110_entropy`**; a bit outside them is undocumented in all three drivers and the TRM, and is a finding rather than a lookup. |
   | any line whose numbers all look like a success | **Read them against `crates/jh7110_entropy` before theorizing.** The 2026-09-04 session lost an hour to a diagnostic of `0x20` read as an `ISTAT` bit that does not exist, when it was the number 32 in a word whose meaning changed with the report beside it. That word is unconditionally `(STAT << 32) \| ISTAT` now, so the ambiguity is gone, but the habit is the lesson. |

   Correction, 2026-09-14: the fourth row quotes a program name the board no longer prints. The
   line it shows says `no 'jh7110_trng' in the initrd`, which is what `kernel/src/main.rs` printed
   when this table was written. The program was renamed twice on 2026-09-13, to
   `jh7110_entropy_source` and then to `jh7110_entropy`, so a board printing that row today names
   `jh7110_entropy` instead. The row itself is left as written, because this block is an account
   and a bench operator matching an old transcript needs to find the words that were in it; the
   meaning and the remedy are unchanged either way.

5. If the success line appears, do the three things that make it a measurement rather than an
   anecdote. **Boot twice** and confirm the two first-draw prefixes differ across boots (a device
   reseeded per boot, rather than a constant baked into silicon or a stale register file). Read
   the mode note in that same line: anything but `256-bit` means the byte count is overstated,
   per the table above. And read the three timing figures, which are the last clause of the
   same line and did not exist when this procedure was first written.

   Bring no stopwatch. Until 2026-09-10 nothing in the tour timestamped either line, so the
   only clock available was a person watching a serial console at about a second of resolution:
   enough to answer the question risk 6 actually asks (is this milliseconds or is it minutes) and
   not enough for a bytes-per-second number worth publishing. The tour now times itself. The line
   carries `since the pcie line N us` (the whole gap, which is what a stopwatch was measuring), the
   bring-up alone, and the two draws with a rate over their eight round trips. Record all three
   verbatim. `design/roadmap/306-time-the-hw-entropy-step.md` has what each one counts, the
   QEMU reference figures to read them against, and the reason the bring-up excludes the `hw clock`
   line's own console time.

6. Whatever happened, capture the board's device tree while you have it: at the `StarFive #`
   prompt, `fdt addr ${fdtcontroladdr}` then `fdt print /soc/rng@1600c000` (and `fdt list /soc` if
   that finds nothing). That answers the first lane's one unconfirmed fact, and a blob dumped off
   the board is a drop-in fixture for `crates/jh7110_entropy`'s existing discovery test rather than a
   new code path.

## Why this is the same shape of gap as 53 and 157

Milestones 53 (network/storage on real silicon) and 157 (display via U-Boot's framebuffer handoff)
both exist because virtio only carries an emulator's paravirtual devices, and real hardware has
none. Entropy is the same story one subsystem over: `entropy_service` (milestone 56) speaks to a
`Virtio` capability today, proven end to end in QEMU, and has never run against real hardware
because nothing here has a driver for the JH7110's TRNG.

## What it needs

- Confirm the TRNG actually exists and is reachable on this board, **done from documentation,
  2026-08-24, not from hardware.** The JH7110 datasheet (v1.67) §2.8.2 documents a TRNG module
  ("Ring-oscillator based entropy source... LFSR based digital post process... self re-seeding...
  256-bit random number generation") and Linux carries a shipped, mainline driver
  (`drivers/char/hw_random/jh7110-trng.c`) and device-tree binding
  (`starfive,jh7110-trng`, `reg = <0x1600C000 0x4000>`, `interrupts = <30>`, clocks `hclk`/`ahb`,
  one reset line) for it. `crates/jh7110_entropy` transcribes the register layout from the Linux driver
  and proves, on the host, that its DTB-discovery query finds a tree shaped like the binding's own
  example and correctly finds nothing on QEMU's riscv64 `virt` board (which has no such node). What
  remains unconfirmed is whether the VisionFive 2's own shipped device tree actually carries this
  node the way mainline's does; nobody has captured one from the board to check.
- **A driver, not a new protocol.** `entropy_service`'s own contract with its clients does not
  change; this is a new backend behind the existing service, the same relationship milestone 157's
  framebuffer driver has to rung one's existing `gfx_proto` contract. Rule 2 applies: it takes a
  base address and knows nothing else. `components/src/jh7110_entropy.rs` is that backend, speaking
  `entropy_proto` unchanged. Wired as of 2026-09-01: `entropy_service`'s `Bus` enum has a
  `Jh7110` variant and the riscv64 boot tour spawns it when the machine's device tree describes the
  device. Still never run against one.
- A rate and quality argument, measured rather than assumed. Still open, and now sharper rather
  than answered. The datasheet documents exactly one hardware fault signal
  (`ISTAT.LFSR_LOCKUP`, an SEU in the post-processing stage), which is cheap to read and this
  driver reads it; neither the datasheet nor the Linux driver document anything resembling a NIST
  SP 800-90B health test or a compliance claim (no FIPS, no AIS-31) over the raw bitstream.
  Whether this tree needs one before trusting these bytes for anything security-shaped is a real
  design question the documentation does not resolve. It is **not decided by this lane** (a
  developer does not edit `design/decisions/`); see `crates/jh7110_entropy/src/lib.rs`'s "Health
  testing" section for the full argument, and treat this as a candidate for a PROPOSED entry in
  `design/decisions/` if calef wants the question tracked formally rather than left in this
  paragraph and the crate's own doc.

## It served, 2026-09-04, and the experiment is run

The success line printed. Two boots of radon, transcript at
`target/board/radon-2026-09-04-trng-success.log`:

```
hw entropy : JH7110 TRNG at 0x1600c000 served 32+32 bytes to a client through a capability
             that names no device; first draw 3faa07e1.., second differs;
             STAT after init 0x00040308 (256-bit: all eight RAND words are the answer)
```

Four things had to be true at once and each was fixed separately: milestone 239 taught `discover`
the vendor U-Boot's `starfive,trng` spelling; milestone 220 clocked the block and released its reset;
this milestone's `fill` replaced a `get` that asked for 32 bytes down a channel carrying 8, so the
tour's success line had been unreachable on any device, working or dead, since the day it was
written; and `ISTAT`'s `R/W1C` clear stopped the second generation returning the first one's
latched register file.

It is a measurement rather than an anecdote, which is what the bench procedure above asks for.
Two boots, and the first draw differs across them:

| boot | first draw | second draw |
|---|---|---|
| 1 | `3faa07e1` | differs |
| 2 | `731191ba` | differs |

So the device is reseeded per boot rather than returning a constant baked into silicon or a stale
register file, and each boot's two draws differ from each other, so the pool is genuinely refilled
between them.

And the one question no documentation could settle is answered. `STAT after init 0x00040308`
reports **256-bit** mode, so all eight `RAND` words are the answer and no byte-count correction is
owed. It is a build-time silicon parameter; the lane that wrote `MODE.R256` recorded that one bench
line would close it, and it did.

### What it establishes for fatal risk 6, and what it does not

Risk 6 is *a capability-confined userspace driver cannot drive real hardware at real speed*, and it
has three parts:

- Confined: demonstrated 2026-09-03. An EL0 process from the archive, reaching the device through
  a capability that names no device.
- Drives real hardware: demonstrated now, reproducibly, on the tree's only confined driver for a
  real non-virtio device.
- At real speed: unmeasured on silicon, and since 2026-09-10 measurable by one boot rather
  than by eye. The tour now reads the timebase around this step and prints the whole `pcie`-to-`hw
  entropy` gap, the bring-up, and the draws with a rate; the instrument was exercised under QEMU
  against virtio-rng, which is the same client path with a free device at the end and so gives the
  denominator radon's figure will be read against. Nothing has run it on the board.
  `design/roadmap/306-time-the-hw-entropy-step.md` has the numbers and the caveats.

Nothing here says the driver is fast, and the block should not be quoted as if it did.

## The bench ran it, 2026-09-04, and the failure is cleanly attributed

The confined driver ran on silicon for the first time. Two boots of radon, byte-identical:

```
hw entropy  : FAILED: JH7110 TRNG at 0x1600c000 (tree says starfive,trng, status disabled):
              report 0x524e475550, bring-up diagnostic 0x0000000000000000,
              draws 0/0 bytes, first-all-zero true, draws-differ false
```

Transcript: `target/board/radon-2026-09-04-trng-bringup.log`.

Milestone 239's fix works. `tree says starfive,trng` is the vendor U-Boot spelling, matched by the
second arm 239 added on 2026-09-03. Every boot before that read `skipped`. The device-tree half of
this milestone is done.

The diagnostic is the all-zero case, which this block's own outcome table routes to **milestone
220** (this kernel drives no clock or reset controller, and the first real device will need one)
rather than to a defect here: the register window read as nothing, most likely because the block's
clocks are gated or its reset is not deasserted.

Two independent signals agree, which is what makes this a diagnosis rather than a guess. The
device tree marks the node `status disabled`, and the register window reads zero. Milestone 239
deliberately *reports* `status` without acting on it, because that same tree lies about the S7 core;
here the tree and the hardware say the same thing.

### What it establishes for fatal risk 6, which is more than "FAILED" suggests

`report 0x524e475550` is **`entropy_proto::READY`**, ASCII `RNGUP`, and that crate's own comment says
a bring-up failure reports `0xDEAD_0000_0000_0000 | step` **instead**. So the service did not report a
failure. It reported ready.

So on real silicon, for the first time: a userspace driver **started** from the archive as an EL0
process, reached the device through a capability that names no device, and completed its
bring-up far enough to send `READY`. What it could not do is get a non-zero byte out of a block
nothing has powered.

Risk 6 is *"a capability-confined userspace driver cannot drive real hardware at real speed."* This
splits it: **confined** is demonstrated on silicon, drives real hardware is blocked on milestone
220, and at real speed stays unmeasured and unmeasurable until the block is on.

### A defect in this milestone, found by the same line

The service sent `READY` while holding 32 bytes of zeros. `entropy_proto::READY`'s own doc says
it is sent *"once the device is up and its first bytes are in hand"*, and two paragraphs later the
same file argues that a caller who cannot be given randomness must find out, because the
alternative is *"the exact silent-degradation failure"* it exists to prevent.

A service that reports ready on a dead device is that failure. The tour caught it only because it
prints the draws beside the report and checks `first-all-zero`; a client that trusted `READY` would
have consumed zeros believing them random. **That is this milestone's to fix, not milestone 220's**,
and it is worth fixing before 220 lands rather than after, because once the clock works the symptom
disappears and the defect does not.

### Fixed 2026-09-04, and the fix is wider than the report word

`entropy_proto::readiness` now decides the readiness word from the bytes: `READY` only when the
first bufferful has a nonzero byte in it, and `0xDEAD_0000_0000_0000 | step` otherwise, with two
shared steps (`0x10` nothing arrived, `0x11` everything that arrived was zero) that cannot collide
with a backend's own. All three backends call it: the JH7110 driver, the virtio-rng one, and the
`RDSEED`/`RNDRRS` instruction one, which had the same shape of bug and had simply never met a dead
device.

Fixing the report word alone would have left the defect in place, which is the part worth
recording. A report reaches whoever wired the service; a client only ever sees a reply. So a service
that reported dead and went on serving its zero buffer would still have handed those zeros out as
randomness to every client that was not watching the handshake. A backend that draws an all-zero
first bufferful is therefore condemned for the boot and answers `NO_ENTROPY` to everything after
it. A backend that drew *nothing* is not condemned: it has told the truth at every step, its replies
already say `NO_ENTROPY` while it stays dry, and it recovers by itself if the device starts
answering.

`system_initializer` already gated the login stack on this word (`entropy_ready = verdict ==
entropy_proto::READY`), so on radon as it stands today the real init now declines to build a
credential stack on a gated TRNG instead of building one on zeros.

The judgment, stated where it can be argued with. An all-zero bufferful is legitimate output
with probability 2^-2048 (virtio), 2^-256 (JH7110) or 2^-64 (the instruction backend), so refusing
one is a correctness claim about a random variable, and it is recorded as a `BUGS` entry in
`entropy_proto`, in `components/src/entropy.rs` and in `components/src/jh7110_entropy.rs` rather than left implicit.
A false "the device is dead" costs one boot's entropy; a false "the device is alive" costs every
secret derived from it.

It stops at bring-up, deliberately. A source that answers once and degrades, or whose register
file latches and repeats a nonzero answer, still passes. That is continuous health testing,
`design/decisions/0137-trng-health-tests.md` is `PROPOSED` and owns it, and its hard half is the
failure action for a *running* service (refusing to serve is a denial of service that can brick a
boot). A readiness handshake does not have that problem, because nothing depends on the service at
the moment it reports, which is why this could ship without 137 and does not pre-empt it.

The boot tour keeps its own two-draw client-side check. A tour that only repeated the service's
verdict would have caught nothing on 2026-09-04.

## The second bench run, 2026-09-04: the device worked and the tour was wrong

Milestone 220 landed the clock and reset driver and radon was booted again. Transcript:
`target/board/radon-2026-09-04-clock-and-first-entropy.log`.

```
hw clock   : JH7110 STG CRG at 0x10230000 (named by this machine's device tree):
             clocks 0x00000000,0x00000000 -> 0x80000000,0x80000000 (running);
             reset 3 assert 0x007ffffe -> 0x007ffff6, status 0x00000009 (released, 1 polls)
hw entropy : FAILED: JH7110 TRNG at 0x1600c000 (tree says starfive,trng, status disabled):
             report 0x524e475550, bring-up diagnostic 0x0000000000000020,
             draws 8/8 bytes, first-all-zero false, draws-differ true
```

Milestone 220's premise is confirmed. The clocks read `0x00000000` before and `0x80000000`
after, so bit 31 was genuinely clear and the block genuinely was gated. The section above predicted
that and it was right.

And then the `hw entropy` line said FAILED while every condition it names was satisfied: the
report word is `READY`, the first draw is not zeros, both draws are full, and the two differ.

### The failure was in the tour, and the number that misled everyone was not a register

`entropy_proto` carries `MAX_BYTES = 8` per exchange and `Wiring::get` is exactly one exchange, so
`w.get(32, &mut a)` returns **8**, never 32. The tour then required `na == 32`. Its success line
was unreachable on any device, working or dead, from the day it was written.

It survived three days because this branch runs on exactly one machine in the world. QEMU's `virt`
has no TRNG node, so CI takes the `skipped` arm and never evaluates the condition, and the first
radon boot failed earlier (on the gated clock) than the check that was broken.

The `0x20` was the number 32. The driver's third report word was the byte count when the report
said `READY` and a `(STAT << 32) | ISTAT` snapshot otherwise, so the same field meant two things
depending on a word printed beside it. Read as a register it says `ISTAT` bit 5, which no Linux
driver, no `NetBSD` driver and the TRM all fail to name, and an hour went into that bit. Read as
what it was, it says the pool held all 32 bytes it had generated. There is no evidence this device
has ever set `ISTAT` bit 5, and the entry that claimed otherwise is corrected here rather than
quietly dropped.

The diagnostic is now unconditionally the register snapshot. `Wiring::fill` loops until a buffer is
full and the tour uses it, which also makes the two draws stronger than they were: 32 bytes is
exactly one generation, so draw `a` empties the driver's pool and draw `b` forces a second trip to
the hardware. Two 8-byte draws both came out of one buffer, so `a != b` proved only that a cursor
advanced. And `kernel::user::entropy_tests::a_fill_gathers_across_round_trips` asks the virtio
backend for 32 bytes in CI, because nothing in this tree had ever asked this protocol for more than
one word.

### What the run establishes for fatal risk 6

Risk 6 is *"a capability-confined userspace driver cannot drive real hardware at real speed."*

- Confined: demonstrated 2026-09-03, and again here. The driver holds two rendezvous
  capabilities and one page of device memory, no IRQ, no DMA, no `Virtio` capability.
- Drives real hardware: demonstrated on 2026-09-04, which is the half that was blocked. A
  confined EL0 process wrote a JH7110 register, polled it, and handed a client bytes that were not
  zero and that changed between draws, through a capability that names no device. The clock work
  that made it possible was milestone 220's.
- At real speed: still unmeasured on silicon. It was "measurable for the first time" here and
  the instrument to measure it with landed on 2026-09-10: the step times itself now, so the bench
  procedure's step 5 asks for three numbers off the line rather than for a stopwatch.

**The success line has still never printed**, which is why this block's status has not moved. What
printed was a FAILED line whose numbers, read correctly, describe a working device.

## What the third lane changed, 2026-09-04, none of it run on silicon

Read this as "matches upstream's order, host-tested, unverified on hardware." radon was powered
down for all of it and no part of a JH7110 exists in QEMU, so nothing below has met the device.

The prior art was fetched rather than recalled, which mattered: `crates/jh7110_entropy` was
transcribed from a *summary* of Linux's driver and recorded three of its own facts as unconfirmed.
Three sources settle them, all cited in the crate with URLs and fetch dates: mainline
`jh7110-trng.c`, the JH7110 TRM's TRNG register page (new to this tree), and `NetBSD`'s
`jh7110_trng.c` (also new, and the most useful because it is the only one of the three that
polls, which is what this driver does).

- `ISTAT` is `R/W1C`, per the TRM's register map. The crate said this was "not confirmed from
  the summarized driver source" and the driver therefore never wrote the register. That is a real
  defect and it is fixed: `RAND_RDY` is latched, so an unacknowledged one makes every generation
  after the first appear complete instantly, and the driver reads the `RAND` words without the
  device having refilled them. **Silicon has not seen this bug**, because the tour never reached a
  second generation: draw `b` came out of the buffer draw `a` had left. Looping the tour to 32 bytes
  is exactly what would have exposed it, so the two fixes had to land together.
- `MODE.R256` is now written, and `STAT` after init is reported so a bench session can check it
  read back. The width a JH7110's TRNG resets to is a build-time parameter of the silicon
  (`BUILD_CONFIG.PRNG_LEN_AFTER_RST`), so it cannot be assumed from documentation. If the block is
  in 128-bit mode only `RAND0..RAND3` are the answer and half of every 32 bytes this driver serves
  is not device output. This is the one open correctness question about the bytes, and one line
  of a bench transcript closes it.
- `AUTO_AGE` and `AUTO_RQSTS` are zeroed, which is how the TRM says the two reseed-reminder
  alarms are disabled and what Linux's default module parameters do.
- `STAT.SEEDED` now gates `RAND_RDY`. `jh7110_trng::interpret` takes `STAT` and returns a new
  `Outcome::Unseeded`. This is `NetBSD`'s gate: `RAND_RDY` is a latch that can stand from before
  this driver ran, while `SEEDED` is live state, and the TRM confirms an unseeded core is a state
  the block reports (`STAT.LAST_RESEED == 0x7`, "Unseeded (zeroized state)"). A Kani harness pins
  that no unseeded snapshot reaches `Ready`.
- Two `ISTAT` bits nobody had named: `AGE_ALARM` (2) and `RQST_ALARM` (3), documented in the TRM
  and defined by `NetBSD`, absent from both Linux drivers and therefore absent from this crate. A
  status word with unnamed bits is one a bench session cannot read, which is the whole of how the
  `0x20` went wrong.
- `IE` is left at zero, and now with evidence rather than a shrug. The TRM's wording on whether
  `ISTAT` latches with interrupts disabled is not decisive. The board settled it: on 2026-09-04 a
  reseed and a generation both completed and were detected by polling `ISTAT`, with `IE` never
  written. Measured, not inferred, and recorded at `IE_GLBL_EN`.

## Follow-on

- **Done.** The tour's unreachable success condition, the ambiguous diagnostic word, and the missing
  bring-up steps: this branch (`milestone/159-trng-sequence`), pull request #729.
- **Recorded.** The 128-bit-mode question stays a `BUGS` entry in `components/src/jh7110_entropy.rs` until a
  bench session reads `STAT.R256` off the board. It cannot be resolved from documentation, because
  the reset width is a build-time parameter of the silicon.
- **Recorded.** `POLL_TRIES` and `LOCKUP_RETRIES` bound loop iterations, not time, so what they
  bound depends on the core and on what the compiler did to the loop. Noted in the same `BUGS`
  section; a real timeout needs a clock the driver does not hold.
- **Milestone 306.** Timing the `hw entropy` step, which is the "at real speed" half of fatal risk 6.
  Its QEMU half was built 2026-09-10 (the tour reads the timebase around the step and prints the
  gap, the bring-up and the draw rate, exercised against virtio-rng because QEMU has no JH7110), and
  the one boot of radon that remained happened on 2026-09-16: **955,223 bytes/s**, 8.4 us per round
  trip, bring-up 562 us. The proposal was promoted to milestone 306 at that point, because a
  proposal whose work is finished is not a proposal.
- **Decision.** Whether these bytes need a NIST SP 800-90B-class health test before anything
  security-shaped trusts them: `design/decisions/0137-trng-health-tests.md`, already `PROPOSED` and
  untouched by this lane.

## What this does not decide

**DECISIONS §120's stopgap question is separate and not reopened by this milestone.** Whether the
interactive boot should grant *any* entropy source (virtio-rng or this driver) before there is a
real customer for interactive login is §120's call, declined for now; building this driver does not
require revisiting that, and landing this driver does not by itself grant it to the boot path.

## What it unblocks

The real-hardware half of milestone 56's own claim: an entropy service this tree can trust on the
board it actually ships to, not only in QEMU. Downstream of that, whenever §120's stopgap question
is revisited with a real customer, the answer can be "real hardware entropy," not only "QEMU's
virtio-rng."

## Index row

Surfaced while investigating milestone 49's boot-wiring fork (DECISIONS §120): the entropy service
(milestone 56) only has a virtio-rng backend, which does not exist on real silicon. A confined
userspace driver drove the TRNG on radon on 2026-09-04 and a client got real, differing bytes
through a capability naming no device, which is fatal risk 6's "drives real hardware" half; the
tour still printed FAILED, because its success line asked for 32 bytes down a protocol carrying 8
and was unreachable on any device. Fixed, with the bring-up steps upstream has and this driver
lacked, none of it yet run on silicon.
