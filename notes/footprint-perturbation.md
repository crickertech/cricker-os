# Taking a benchmark on radon: E1, E3 and E4 on the small-cache board

*(Milestone 134. Named by calef on 2026-09-04, from three candidates. `board-bench.md` was refused
for a collision the file itself could not fix: `notes/bench-runbook.md` already exists and answers
"which machine should an evening be spent on", so two pages about benching on boards would have had
names that do not tell them apart. This one is named for the experiment, which is milestone 134's own
word for E3, so a reader looking for "does the fastpath's size cost anything" finds it. That it is a
board procedure is the first line's job rather than the filename's.)*

The session was run on 2026-09-04 and the results are below, under "What the session
measured". Six boots on radon, interleaved, one card written six times. Read that section first;
everything after it is the procedure that produced it and is kept because the session is worth
repeating.

The headline is not the one this page was written to expect. E3 separated cleanly in all three
rows and one of them has the wrong sign, which says the experiment is confounded by code layout
rather than that its hypothesis is false. The two experiments riding along, E1 and E4, produced the
more decisive results of the evening.

*(The board columns were empty before the session and are filled now. The static footprints below
are still `objdump` on patagonia, and the 2026-08-22 figures are still dev-Mac runs.)*

## The next radon evening: E3 under the layout control, and how it shares a night with 168

*(Written 2026-09-19 by milestone 134's stack-depth lane, rewritten that day by the control's lane.
It says what each half of an evening shared with notes/job-mix.md needs, and in what order.)*

What is already taken and does not need the board again. E1 and E4 ran on radon on 2026-09-04
and both are single-build sweeps, so the confound below does not touch them; they are quotable as
they stand. The per-IPC kernel stack depth, which E1's prediction used to estimate, is now measured
(notes/stack-high-water.md, "Per-IPC depth").

What is owed is E3, and only E3 under a layout control. Re-running E3 as it was built would
reproduce the 2026-09-04 confound exactly (this page's BUGS: a second card or a second board
changes nothing about addresses, so it would look like confirmation). The control is built, as
of 2026-09-19: `kernel/src/fastpath_pad.rs`, sized by two build-time variables.

### The knob, in one paragraph

`fastpath_pad` still turns E3's padding on. Two environment variables, read by `kernel/build.rs`
and only when that feature is on, say how much of it there is:

- `NIFE_FASTPATH_PAD=<units>` is the sled's length in units of what the feature has always
  linked (5,092 bytes on riscv64, 5,796 on aarch64). Unset is 1, the sled the 2026-09-04 session
  booted. 0 is the ladder's zero: the guard and a `ret`, so every rung runs the same
  instructions and the guard stops being a difference between conditions.
- `NIFE_FASTPATH_SHIFT=<bytes>` appends that many zero bytes after the sled, under a symbol
  nothing references. It is the layout variant: bytes that move code and that the footprint
  gate does not count.

The linker script pins the sled first in `.text`, right after the boot stub, so both numbers do
the same thing: move the whole kernel text by that many bytes. Measured on the card build, only the
7 boot symbols keep their address; all 1,038 others move by exactly the bytes asked for and none
changes size. An ordinary build has no such section, so the default link order is unchanged, which
was checked by comparing default boot images byte for byte across the change on both ISAs.

Why that matters more than the knob does. A pad and a shift of equal displacement produce
identical binaries apart from the sled's own bytes: `PAD=1` and `SHIFT=5088` put all 1,047
riscv64 text symbols at the same addresses. So the two ladders differ in exactly one thing, whether
the bytes sit inside the footprint `script/fastpath-footprint` counts, which is the variable E3
claims to test. Before the pin, where the linker dropped the sled was redrawn every commit: on
2026-09-04 it landed ahead of the whole trap path, and by 2026-09-19 past all but 5% of the
fastpath's bytes, so how much of the hot path a pad perturbed was an accident of that day's build.

They are variables rather than a feature per size because it was built that way first and
measured. A feature's name enters cargo's `-C metadata` hash, which renames every symbol and
reorders codegen units: seven sibling features moved code linked *ahead* of the sled by up to
11 KB on the card build, and `syscall::dispatch` wandered across 240 of the 256 L1i sets, so each
image was an uncontrolled layout draw. Editing this module's own source does the same thing for the
same reason, which is why every image in an evening must come from one commit.

### The eight images, and why these sizes

| image | `NIFE_FASTPATH_PAD` | `NIFE_FASTPATH_SHIFT` | the whole kernel text moves by | what it is for |
|---|---|---|---|---|
| pad 0 | 0 | 0 | 0 | the reference for both ladders |
| pad 1 | 1 (or unset) | 0 | 5,088 (79.5 L1i sets) | the 2026-09-04 sled |
| pad 2 | 2 | 0 | 10,176 (159 sets) | |
| pad 3 | 3 | 0 | 15,264 (238.5 sets) | |
| layout A | 0 | 820 | 820 (12.8 sets) | a small layout draw |
| layout B | 0 | 2460 | 2,460 (38.4 sets) | another |
| match 1 | 0 | 5088 | the same bytes as pad 1 | pad 1's twin, uncounted |
| match 2 | 0 | 10176 | the same bytes as pad 2 | pad 2's twin, uncounted |

The two matched images are the sharpest comparison available and cost nothing extra. Each is
byte-identical to its pad outside the sled, so any difference between a pad and its twin is the
counted footprint alone, and the physics says there should be none: the sled is never fetched. A
difference there is the instrument telling you something is wrong with the measurement rather than
a finding about footprint.

The four un-padded images are the layout distribution. Two of them are matched to pads and two
are small, so the distribution covers both scales the pads reach into.

The two small images sample layout at a scale the pads do not. They move the text by under one
page, and they differ where the U74 is known to care: 820 is 4 mod 8 and 2,460 is 4 mod 8 with
different 64-byte line phases (52 and 28) from the pads' (32 and 0), and the manual says the BTB
predicts a taken branch or jump with no bubble only when the target is 8-byte aligned (`SiFive`
U74-MC Core Complex Manual 21G3.02.00, §4.2.6). One leaves `.data` on the page pad 0 has and the
other moves it, which matters because the L1d is 32 KiB 4-way, an 8 KiB way, so an odd page moves
every static to a new set (§4.4.1).

### Step 0, on patagonia, before anyone walks to the bench

```sh
git log -1 --format=%h                     # the commit every image below is built from
for c in 0:0 1:0 2:0 3:0 0:820 0:2460 0:5088 0:10176; do   # pad:shift
  NIFE_FASTPATH_PAD=${c%%:*} NIFE_FASTPATH_SHIFT=${c##*:} script/fastpath-footprint \
    --arch riscv64 --features board,bench,single_hart,fastpath_pad --layout
done
```

Two lines out of each run are the check, and they are the reason this step exists:

- `layout: code hash <...>` must be the same 16 hex digits for all eight. It is a hash of
  every instruction in both IPC closures and the entry set, with address operands normalized away
  (direct call and branch targets, `auipc`/`adrp` uppers, and the low-12 immediates that pair with
  them; the script prints how many of each it touched). Equal hashes are the proof that the eight
  images execute the same code and differ only in where it sits. A different hash means something
  changed the fastpath itself and the evening is measuring two things again.
- `layout: fastpath_pad_body at <addr>, <n> bytes` must show the sled at the same address in
  all eight, with the requested length, and the line under it must read `the sled precedes 17 of
  17 hot symbols, 100% of their bytes`. That is the pin working. Anything less means the section
  is no longer first in `.text` and the pad is perturbing only part of the path.
- The two matched images must place the hot path exactly where their pads do. Compare the
  `layout: 0x...` lines of `PAD=1` against `SHIFT=5088`, and `PAD=2` against `SHIFT=10176`: every
  address should be equal.

Keep the whole readout per image: it carries each hot symbol's address, its L1i set on the U74
(32 KiB, 2-way, 64-byte lines, virtually indexed, §4.2.2, so 256 sets from address bits 6 to 13),
its line phase, whether it starts 8-byte aligned, and where `.text` ends and `.data` starts. Those
are the pre-registered layout facts each image's numbers get read against.

### The boots

```sh
NIFE_FASTPATH_PAD=0 NIFE_FASTPATH_SHIFT=0 \
  script/board-image --bench --tftp --extra-features fastpath_pad
script/board-console --for 20m --until none --log bench/radon-<date>/bench-e3-pad0-1.log
```

- Eight images, three boots each, interleaved, in the order pad0, pad1, pad2, pad3, A, B,
  match1, match2, then around again twice. Interleaving is this page's own rule (step 5 below) and it is what keeps
  a room warming up out of the comparison. Twenty-four boots at about 75 seconds each is roughly
  half an hour of booting plus a rebuild between images; `--tftp` (milestone 257) makes an image
  change a rebuild and a power cycle rather than a card write.
- Check the first two lines of every capture. `bench: cntfrq 4000000` says this is the board
  and not QEMU `virt`. `bench-probe: fastpath_pad units <u> shift <s>` says which image booted,
  which nothing on a card said before 2026-09-19 and which is the mistake an interleaved run of
  eight images is most likely to make. The last line must be `bench: done`.
- `script/board-image` echoes the two variables on its `features:` line, and refuses to build if
  they are set without `fastpath_pad` in the feature list.

### Reading it

Three rows matter, as before: `call_reply` (the shape services run, and the one the verdict should
rest on), `ipc_rtt`, `ipc_rtt_el0`.

1. Read each matched pair first, pad 1 against `SHIFT=5088` and pad 2 against `SHIFT=10176`.
   They are the same binary outside a sled nothing fetches, so a difference beyond the
   boot-to-boot spread means the instrument is measuring something nobody has accounted for, and
   the rest of the reading is suspect until it is explained.
2. Then take the layout distribution. Pad 0 and the layout images differ in nothing a CPU
   executes, so whatever spread appears across them is the layout effect at these
   displacements, per row. Report it as a range, not a mean.
3. Then read the pad ladder against it. With the sled pinned, a pad and a shift of equal
   displacement are the same binary, so the honest question the ladder answers is whether
   displacement costs time and whether more of it costs more. A row that rises monotonically
   and leaves the layout range says it does; one that jumps and comes back is a particular
   alignment rather than a trend; one inside the boot-to-boot spread is nothing. None of those
   is a footprint result, which is the paragraph below.
4. Keep every boot's `cycles_per_tick` line, so the rows convert to cycles (M5) on the
   evening's own commit.

| what the capture shows | what it means | where it routes |
|---|---|---|
| each pad sits on its matched twin, and the pads move no further than the layout images do | the counted footprint predicts nothing: doubling and tripling it costs no more than moving the same code the same distance | §95 (a hand-written IPC fastpath, and whether it can stay proven)'s premise is in serious doubt. 188 phase 4 would buy a standing verification obligation for an effect this instrument cannot find; route to `design/decisions/0095-*` |
| a pad differs from its matched twin, beyond the boot-to-boot spread | impossible on the physics as understood, since the two binaries differ only in bytes nothing fetches. Something else differs between the images or the measurement is not what it seems | nothing routes until it is explained; it is the instrument's own check failing |
| the pad ladder rises monotonically and leaves the layout range, matched twins tracking their pads | displacement costs time and more displacement costs more, which is the strongest reading available here. It is a layout result stated honestly, not a footprint one | 188 phase 4: the magnitude is what a hand-written fastpath would have to beat, and notes/benchmarks.md gets a caveat on every stored baseline |
| the layout range is itself large (several percent) | the 2026-09-04 reading was an artifact, as suspected, and every between-build comparison in `bench/` inherits the same exposure | notes/benchmarks.md, as a caveat on stored baselines; and E3 as built cannot answer §95 at all |
| everything inside the boot-to-boot spread | this kernel's IPC path does not care where it sits, at these displacements, on this core | 188 phase 4 loses its last cheap instrument; M6 is what is left |
| rows disagree (one moves, another does not) | shape-specific, which is a result about *which* path to optimize | 188 phase 4, narrower than sketched |

What this cannot decide, and it should be said before the numbers exist. The padding is never
executed, so it cannot evict anything on its own; the only way it can reach a clock is by moving
other code. E3 therefore tests whether the number `script/fastpath-footprint` reports predicts
latency, which is what milestone 188 (the IPC fastpath) leans on, rather than Liedtke's claim about an *executed*
footprint. That claim needs either M6 (instruction-cache misses per IPC) or a perturbation that
adds executed instructions, and neither is this experiment.

Optional, one boot, if there is time: a card built with `--extra-features ipc_stack_depth`
prints the per-IPC depth lines on the board itself. It should reproduce the QEMU release numbers
to the byte (the `board` and `single_hart` features are the only differences); a disagreement would
mean one of them changes the IPC path's codegen, which nothing currently expects. Its timing rows
are not results, since the instrument paints on every sample.

### Sharing the night with milestone 168

E3 is eight images now, where it was two. The halves cannot share an image (`script/board-image`
refuses `--bench` with `--job-mix`), and `--tftp` makes switching a rebuild and a power cycle. E5,
the executed-footprint test E3 cannot be, needs a build and an evening of its own:
[its plan](footprint-perturbation/executed-footprint.md).

| order | what | boots | why this order |
|---|---|---|---|
| 1 | on patagonia: step 0 above, all eight images | none | the check that invalidates the whole E3 block if it fails, and it needs no board |
| 2 | E3, `script/board-image --bench --tftp --extra-features fastpath_pad` per image | 3 per image, interleaved | interleaving is the design, so it goes first while the room and the board are at one temperature |
| 3 | milestone 168's job mix, `script/board-image --job-mix --tftp` | at least 5 | notes/job-mix.md, "The next bench evening on radon", steps 0 to 8, unchanged |
| 4 | optional: one `--bench --extra-features ipc_stack_depth` boot | 1 | last, because nothing is blocked on it |

Log names: `bench-e3-pad<n>-<boot>.log` and `bench-e3-shift<bytes>-<boot>.log` here,
`jobmix-boot<n>.log` for 168's, both under `bench/radon-<date>/`. Write the commit down once. The
console rules are notes/job-mix.md's step 0 for both halves: one capture per serial port, and clean
each log with `tr` before committing it.

## What the session measured, 2026-09-04

Six boots on radon, interleaved unpadded/padded, one card written six times, exactly the order
the section above requires. Every boot printed `bench: cntfrq 4000000` (the board, not QEMU
`virt`), `bench: cycles_per_tick 250.00` (milestone 74's riscv64 PMU reading real cycles), and
`bench: done`. Nothing self-skipped, so E1 and E4 produced full sweeps as well, and they are below
because they are the more decisive half of the session.

The static check of step 1 was taken first, on the exact commit flashed: riscv64 `ipc_call_reply`
5,936 unpadded, 11,070 padded, 1.87x. The card was verified byte-for-byte against the built
image before each of the six ejections, because nothing on the card says which build it carries.

### E3: three non-overlapping separations, and one of them points the wrong way

Ticks at 4 MHz, so one tick is 250 ns. Boots 1, 3, 5 unpadded; 2, 4, 6 padded.

| row | unpadded (3 boots) | padded (3 boots) | delta | per round trip |
|---|---|---|---|---|
| `ipc_rtt` (1000 iters) | 4259 · 4259 · 4259 | 4311 · 4310 · 4310 | +1.21% | +12.8 ns |
| `call_reply` (1000 iters) | 5015 · 5013 · 5013 | 5088 · 5089 · 5088 | +1.49% | +18.7 ns |
| `ipc_rtt_el0` (5000 iters) | 128606 · 128626 · 128615 | 124958 · 124391 · 124903 | -3.01% | -193 ns |

The measurement is as clean as this instrument can be. `ipc_rtt` returned 4259 three times out
of three. The unpadded `call_reply` band is 2 units wide across three boots and the padded one is
1; the gap between them is 74, which is roughly 37x the within-condition spread. `single_hart`
did what it was built for: the fifteenfold placement lottery of notes/soak.md cannot appear on a
card with one core, and the boot-to-boot spread collapsed accordingly.

And the third row is the finding, not the first two. The padded build is 3% *faster* on the
round trip that crosses EL0, reproducibly, with no overlap, and the effect is 193 ns where the
kernel-side one is 19 ns. It is an order of magnitude larger than the cost it is supposed to be
paying, and it has the wrong sign.

### Why that sign matters more than the magnitude

Adding 5 KB of resident dead code cannot make anything faster. The padding is never executed:
`kernel/src/fastpath_pad.rs` is reached through `core::hint::black_box` on a compare-and-branch
that is never taken. Whatever produced a 193 ns improvement is therefore not the footprint, because
the footprint has no mechanism by which to produce one.

What it is, almost certainly, is code layout. The padded build is not "the same kernel plus dead
bytes"; it is the same kernel at different addresses, with different cache-line boundaries,
different set indices in a 32 KB 2-way L1i, and different branch-predictor aliasing. A lucky layout
can be worth more than 5 KB of dead text costs, and here it was.

This is the effect Mytkowicz, Diwan, Hauswirth and Sweeney named in *Producing Wrong Data Without
Doing Anything Obviously Wrong* (ASPLOS 2009): changing a link order or a UNIX environment variable
size, which alters no instruction, moves measured performance by more than the optimization under
study. E3's design has exactly the shape they warn about. It varies one Cargo feature and reads
the difference as an effect of that feature, and a Cargo feature that inserts a symbol changes the
layout of everything after it.

So the +1.49% on `call_reply` cannot be attributed to footprint either. We have one unpadded
layout and one padded layout, and we have just watched layout move a number by 3% in a direction
footprint cannot explain. The +1.49% is a real, reproducible difference between these two binaries;
it is not evidence that it is a difference between these two footprints.

It is not a null result and should not be filed as one. A null would be "padding changed
nothing". What happened is that padding changed three things, one of them impossibly, which says
the instrument is confounded rather than that the hypothesis is false.

### Where that leaves milestone 188 phase 4, and what would actually decide it

The routing table above offered four rows and the capture matches none of them, because all four
assume the only variable is footprint.

Phase 4 stays where it is. A hand-written fastpath is a permanent verification obligation, and
the case for it now rests on a 19 ns difference that a layout artifact of 193 ns sits on top of.
That is not evidence a reader should be asked to buy a second IPC path with.

What would decide it is a layout control, and it is cheap. The confound is testable directly:
build several unpadded kernels that differ only in layout (a dead symbol of a different size, or a
link-order change, in both cases adding no reachable instruction) and read the same three rows. That
gives a layout distribution rather than a single unpadded point, and E3's padded reading is then
interpretable for the first time: an effect inside the layout distribution is layout, and one
outside it is footprint. The same fix answers the E4 and E1 readings below, which have the same
exposure.

Written up as a proposal the day this was captured, promoted to milestone 370 (a layout control), and built on
2026-09-19: the images and the procedure are in "The next radon evening" above. Without it every perturbation
experiment in this tree measures footprint and layout summed together and reports the total as
footprint.

### E1: the knee is real, it is where the prediction put it, and then it stops

Six boots, and this sweep does not depend on the padding at all, so all six are one sample.

| threads | ticks/iter (range over 6 boots) | ns | vs 2 threads |
|---|---|---|---|
| 2 | 4.365 to 4.719 | 1152 | 1.00x |
| 4 | 4.737 to 4.795 | 1189 | 1.03x |
| 8 | 5.401 to 5.628 | 1377 | 1.20x |
| 16 | 7.649 to 7.903 | 1938 | 1.68x |
| 32 | 7.713 to 7.919 | 1953 | 1.69x |
| 64 | 7.744 to 7.974 | 1966 | 1.71x |
| 96 | 7.790 to 8.026 | 1975 | 1.71x |

This is the strongest result of the session and it is not the one the session was for. IPC
latency rises 68% between 2 and 16 threads, with the bend between 8 and 16, and is then flat to 96:
the last four points span 1.2%. The prediction E1 was designed around, computed against radon's
32 KB by name, was a knee in the low tens. It is at 16.

The shape is a working set crossing a cache and then saturating, which is what it should look
like: once the per-thread state no longer fits, every additional thread misses anyway and the curve
has nowhere further to go. On patagonia the same sweep rose 8 to 11% across the whole range with no
identifiable knee, which is the large-cache muting E1's note predicted.

What it says about `design/decisions/0096-process-kernel-or-event-kernel.md`. The performance
input to §96 (process kernel or event kernel) is live: a per-thread kernel state that a process kernel carries and an event kernel
does not is measurably expensive on the smallest cache this project targets. But read it against
E2's finding that the customer path runs 4 to 8 threads, which is on the flat part of the curve,
below the knee. So the honest statement is that the effect §96 worries about is real on this
machine and the workload this project has is not in the regime where it bites. That is a
sharper input than §96 had, and it does not answer §96 on its own.

And this sweep has the same layout exposure as E3. It is a single build, so nothing here is a
between-build comparison and the confound does not apply to the shape of the curve. It does apply
to anyone who later compares this curve against a different build's.

### E4: application displacement is real and load-dependent

Percentages of throughput the application loses while IPC runs, identical across all six boots
except one 4k reading that split 6 and 7.

| working set | under IPC load | under 96-thread IPC load |
|---|---|---|
| 4 KiB | 1% | 6 to 7% |
| 16 KiB | 1% | 7% |
| 32 KiB | 1% | 8% |
| 64 KiB | 0% | 5% |
| 128 KiB | 0% | 5% |

This is the Liedtke measurement proper, and it is the first time this tree has taken it on a
machine with a small cache. Two things in it are worth more than the headline.

The peak is at 32 KiB, which is radon's L1d exactly. An application whose working set just fits
the cache is the one with the most to lose when something else evicts it. Below that there is room
for both; above it the application is already missing and IPC displacement costs it proportionally
less. That the peak lands on the cache size rather than at one end of the sweep is what makes this a
cache measurement rather than a scheduling one.

Load dependence is the whole effect. At ordinary IPC load displacement is 0 to 1%, at the
boundary of measurable. At 96 threads it is 5 to 8%. Liedtke's claim is about a kernel whose
footprint evicts the application, and the honest reading here is that it needs both a large
footprint and heavy IPC before it is worth 5% of anything.

The 2026-08-22 patagonia session read 0 to 3% and could not separate it from noise. The board
reproduces it at three times the magnitude with a clean peak at the cache size, which is E3's own
argument for why the small-cache machine was worth the trip, arriving through E4 instead.

## Why this page exists: an instrument aimed at the wrong machine

Milestone 134 (the register of measures)'s E3 pads the IPC fastpath with resident dead code and asks whether latency moves. Its
own block states the asymmetry that makes it worth re-taking:

> A *positive* result on the dev Mac is conclusive: if padding hurts on a machine with large caches,
> it certainly hurts on a 32 KB L1i. A *negative* result on that machine proves little, because an
> M-series core may simply absorb the whole path.

E3 ran on patagonia on 2026-08-22 and read 2 to 3% between the padded and un-padded builds, inside
run-to-run noise. That is the weak direction. E1 (IPC latency against thread count) and E4
(application displacement) are in the same position: both found small, direction-consistent effects
that their own notes attribute to the dev Mac's large L1d muting the knee.

radon is the machine all three were designed against. A StarFive VisionFive 2, four SiFive U74
cores, 32 KB L1i and 32 KB L1d each (notes/benchmarks/kernel-footprint-and-caches.md's machine table). E1's prediction was
computed against that number by name. Until 2026-09-04 none of the three could run there.

## What was in the way, and what closed it

Three separate things, and only the first was obvious.

| the obstacle | why it blocked | what closed it |
|---|---|---|
| E1 and E4 were `#[cfg(target_arch = "aarch64")]` | this tree has no riscv64 *accelerator* with a real cache, so they were gated on the accelerator rather than on the cache | the cfgs are `any(aarch64, riscv64)`; the TCG self-skip is now a per-arch counter frequency (10 MHz on QEMU `virt`, 4 MHz on the JH7110) rather than an arch-specific one |
| both require one hart, and a card has no `-smp 1` | radon seats four U74s, so a `board,bench` card would boot four and both benchmarks would print a skip line on the machine they were built for | the `single_hart` kernel feature: `smp::bring_up_secondaries` marks the boot core online and starts nobody |
| the padding was reachable only from `ipc_send` | milestone 188 phase 1 split the footprint gate into two closures and found the CALL/reply one is larger and is the shape real services run; the pad landed on the other one | `ipc_call` calls `maybe_pad` too |

That third row is a finding, not a chore, and it is the reason this lane touched the kernel at
all. Measured on riscv64 before the fix, `--features fastpath_pad` moved `ipc_send_receive` to
2.10x and `ipc_call_reply` to 1.00x. E3 as built was padding a shape nothing in this tree
runs. It was correct when it was written: the split did not exist on 2026-08-22, and `ipc_fastpath`
was one number. Anyone who had taken E3 to the board in the four days after milestone 188 landed
would have measured the padding of a path their own benchmark barely uses.

## The static half, which needs no board and is already taken

`script/fastpath-footprint` is `objdump` over the built kernel. Run on patagonia, 2026-09-04, with
`ipc_call` padded:

| ISA | shape | un-padded | padded | ratio |
|---|---|---|---|---|
| riscv64 | `ipc_send_receive` | 4,632 | 9,726 | 2.10x |
| riscv64 | `ipc_call_reply` | 5,936 | 11,070 | 1.86x |
| aarch64 | `ipc_send_receive` | 5,356 | 11,192 | 2.09x |
| aarch64 | `ipc_call_reply` | 7,028 | 12,860 | 1.83x |

Read the riscv64 `ipc_call_reply` row against 32 KB. 5,936 bytes is 18% of radon's L1i; 11,070
is 34%. The padded build still fits, which is exactly the condition that makes the experiment
interesting rather than trivial: this is a footprint change large enough to matter under contention
and small enough that a naive "does it still fit" reading predicts no effect at all.

`ipc_fastpath` is the max of the two shapes, so it reads 1.86x on riscv64 rather than the 2.00x the
2026-08-22 run recorded against the old single-closure number. That is the same padding measured
against a larger denominator, not a weakening of the pad. "Roughly double", which is the block's own
wording, still holds.

x86_64 is not padded on either shape: `kernel/src/arch/x86_64/fastpath_pad.rs` does not exist.
Recorded in `script/fastpath-footprint`'s own BUGS and unchanged by this lane. This said "xenon has
no first light, so nothing is waiting on it"; xenon had first light on 2026-09-05
(notes/x86-uefi-boot.md), so the second clause is the one to re-ask rather than the first.

## What you need at the bench

Everything `notes/visionfive2.md`'s bench runbook and `notes/board-console.md` already list, and
nothing more:

- radon, DIP switches on QSPI, powered from its own Kasa outlet (smart plug 2; plug 3 is garcia
  and must never be switched off).
- The USB TTL adapter on the 40-pin header, TX/RX crossed, 3.3 V, `/dev/cu.usbmodem*` (`cu.`, never
  `tty.`).
- A microSD card already formatted and mounted, and its mount path.
- patagonia, this checkout, and about ten minutes of building per card.

Two cards, or one card written six times, and this page said something looser until 2026-09-04.
E3 is a comparison of two builds differing in exactly one Cargo feature. It said "writes the card
twice and boots twice", which describes a *blocked* order, and the section below requires an
interleaved one. With two cards those agree. With one card they do not, and calef has one.

So, with a single card, the order is:

```
unpadded -> padded -> unpadded -> padded -> unpadded -> padded
```

six writes, not two. A rewrite is about two minutes once the build is warm (`script/board-image`
rebuilds only what changed and the copy is 9 MB), so the session costs roughly 75 minutes rather
than 60.

Do not take the blocked order to save the flashes. Three unpadded boots followed by three padded
ones puts everything that drifts across the session, board temperature most obviously, entirely on
the second group, where it is indistinguishable from the effect being measured. The interleaving is
not tidiness; it is what makes a few-percent difference mean anything.

They produce the same three filenames, so nothing on the card says which is which;
`script/board-image` echoes its feature list for exactly this reason and that line belongs in the log
beside the numbers. That the card cannot say what it is, is the reason six writes are risky rather
than merely slow, and `design/roadmap/0367-a-boot-banner-that-names-the-build.md` is the fix.

## The procedure, in order

### 1. Take the static numbers first, on patagonia, with no board

```sh
script/fastpath-footprint --arch riscv64
script/fastpath-footprint --arch riscv64 --features fastpath_pad
```

Two minutes, no hardware. It proves the padding still doubles what it claims to on the exact commit
about to be flashed, which is the one thing that would silently invalidate the whole session. Copy
both `ipc_call_reply` figures into the log.

### 2. Build and write the un-padded card

```sh
script/board-image --bench --card /Volumes/NIFE
```

Confirm the line it prints:

```
  features: board,bench,single_hart
```

`single_hart` is not optional and not a tuning knob. Without it the kernel boots four U74s, and E1
and E4 both print `skipped (needs a single hart ...)`. Its cost is stated where a reader meets it:
`smp_throughput`, `fs_read` and `fs_throughput` self-skip on this card. That is a fair trade for
this session and a bad one for any other, which is why it is a flag.

### 3. Boot it and capture everything

```sh
script/board-console --for 20m --until none --log target/radon-bench-unpadded-$(date +%s).log
```

Then power radon on. `--until none` because the bench boot prints a couple of dozen rows over
several minutes and then halts; there is no single banner worth stopping at, and a deadline that
fires mid-sweep loses the run. Twenty minutes is generous on purpose. The last line is
`bench: done`, and it is what says the suite finished rather than faulted.

Do not power-cycle to "hurry it along". E4 alone runs 5 working sets x 3 load conditions x 5
repeat batches, and E1 sweeps 7 pair counts with 4 repeats each.

### 4. Repeat the boot, unchanged, at least three times

Same card, same image, power-cycle between. This is the run-to-run distribution, and every
comparison below is against it rather than against a single pair of numbers. The 2026-08-22 dev-Mac
session found this the hard way: an unrepeated E4 swung 2 to 3x between nominally identical
conditions.

### 5. Build the padded card and repeat steps 3 and 4

```sh
script/board-image --bench --extra-features fastpath_pad --card /Volumes/NIFE
```

Confirm `features: board,bench,single_hart,fastpath_pad`. Log to a filename that says `padded`.

Interleave the boots if the session has time: unpadded, padded, unpadded, padded. Anything that
drifts over a session (ambient temperature, a card that is warming up) then lands on both conditions
instead of on whichever was measured second.

### 6. Read the rows

| row | experiment | what it is |
|---|---|---|
| `bench: ipc_rtt <ticks> 1000` | E3 | kernel-side round trip, the SEND/RECEIVE shape |
| `bench: ipc_rtt_el0 <ticks> <iters>` | E3 | the same crossing EL0, which is what lmbench measures |
| `bench: call_reply <ticks> 1000` | E3 | the CALL/reply shape, the one services run |
| `bench: ipc_scale_<threads> <ticks> <iters>` | E1 | 7 rows, 2 to 96 threads |
| `bench: appdisp_<kib>k_solo/_ipc/_ipc96` | E4 | 15 rows, 5 working sets x 3 load conditions |
| `bench-probe: appdisp_<kib>k_*_lost_pct` | E4 | the derived percentages, printed so nobody has to divide |
| `bench: cntfrq 4000000` | all | the proof this ran on the board. 10,000,000 is QEMU `virt` |

That last row is the one to check before reading any other. A capture reading `cntfrq 10000000` is
an emulator, and E1 and E4 will have self-skipped in it.

`call_reply` is the row E3's verdict should rest on, not `ipc_rtt`. The padding now lands on both
shapes; the CALL one is what a service issues, and milestone 188's phase 4 decision is about that
path. `ipc_rtt` is kept because it is the row every previous E3 reading used and dropping it would
break the comparison with 2026-08-22.

## Controlling for the thing that has already burned this project

Placement decides throughput on radon by up to fifteenfold (milestone 240 (the soak reports what happened and not where), notes/soak.md): four
soak runs on the same card spanned that range, and the census explains them by how many cores held
an IPC thread and no grinder. A latency number from a single boot is a draw from that distribution.

A `single_hart` card removes the lottery rather than controlling for it, and that is the strongest
form available. There is one core, so there is no spawn placement to draw, no work stealing, and
no migration; `pick_spawn_target` has one answer. The fifteenfold spread cannot be reproduced on this
card because the mechanism that produces it is not present.

That is not the same as saying the boots are identical, and the remaining variation is why step 4
repeats them anyway:

- DRAM training and the U74's own cold state differ boot to boot.
- The archive is measured at boot, the heap is laid out fresh, and where E1's 96 thread stacks land
  relative to each other is a property of one boot's allocator history.
- Nothing here is pinned to a hart *number*: OpenSBI's boot hart is not guaranteed to be 0, so a
  given boot runs on whichever hart it woke on. All four U74s are the same core, but they are not
  the same silicon.

So: three boots minimum per condition, interleaved, and report the spread rather than a mean.
An E3 effect smaller than the boot-to-boot spread is not an effect, and saying so is the finding.

## What each outcome means, and where it goes

This is the table the session exists to fill in. Milestone 188's phase 4 is a hand-written IPC
fastpath, held by calef pending evidence that the footprint costs anything real; its phases 1 to 3
measured `ipc_call_reply` at 48% to 103% over the 4 KiB target, so the bytes are settled and the
cycles are not.

| what the capture shows | what it means | where it routes |
|---|---|---|
| `call_reply` and `ipc_rtt_el0` clearly slower padded, beyond the boot-to-boot spread | Liedtke's claim is live on this machine at this footprint. §95's premise holds, measured rather than argued | milestone 188 phase 4 is justified; the magnitude is the expected payoff of a hand-written fastpath and the number to hold it to |
| padded and un-padded within the spread, as on patagonia | a doubling of footprint costs nothing measurable on a 32 KB L1i either. This is the strong direction of the negative: E3's own design says a null here is worth far more than a null on patagonia | §95's premise is in serious doubt. 188 phase 4 buys a standing verification obligation for an effect two machines cannot find. Route to `design/decisions/0095-*` as evidence for closing it |
| `ipc_rtt` moves and `call_reply` does not (or the reverse) | the effect is real but shape-specific, which is a result about *which* path to optimize rather than whether to | 188 phase 4, with a narrower scope than currently sketched |
| E1 `ipc_scale_*` bends sharply in the low tens | Warton's effect reproduced on the machine the prediction was computed for. §96's performance input is live | `design/decisions/0096-process-kernel-or-event-kernel.md`, read against E2's finding that the customer path runs 4 to 8 threads |
| E1 flat to 96 threads | the process kernel costs nothing on this axis on the smallest cache we target. Stronger than patagonia's 8-11% rise, in the opposite direction | §96 answered no, on data |
| E4 `_ipc96` clearly above `_ipc`, and both above zero | application displacement is real and load-dependent: the Liedtke measurement proper | the register, and 188 phase 4 as supporting rather than deciding evidence |
| E1 or E4 print `skipped` | the card was built wrong. `needs a single hart` means `single_hart` was missing; `QEMU virt detected` means this is not the board | rebuild, do not interpret |
| `MEASURED BOOT REFUSED` | kernel and archive came from different builds | `script/board-image --card` copies them as a set; copy all three files again |

Two outcomes are worth naming as genuinely decisive and one is not. Row 1 and row 2 both settle
188 phase 4, in opposite directions, and both are worth the session. A result that lands inside the
spread but "looks like" a trend is the outcome to resist: this project has a recorded habit of
reporting overlapping ranges as directional findings, and E4's own 2026-08-23 follow-up says so in
its own words.

## What milestone 74's riscv64 half adds, and what it still cannot see

`kernel/src/arch/riscv64/pmu.rs` landed 2026-09-04 and reads real cycles through the SBI PMU
extension. It was not available when E3 was designed, and E3 was designed to work without it: the
whole point of padding is that it tests Liedtke's claim with no cache counter, by making a
static footprint tool agree or disagree with a wall clock.

What the PMU adds here is precision, not a new answer. Every row above is a `rdtime` tick count
at 4 MHz, which is a 250 ns quantum; a 2% effect on a low-microsecond round trip is a handful of
ticks. Cycles at the core clock resolve that by three orders of magnitude, and they remove the one
methodological complaint nobody could answer on patagonia, which is whether a small percentage was
a real effect or a timer artifact.

What it still cannot see is the mechanism. A cycle counter says a round trip got slower; it does
not say the instruction cache is why. The direct measurement is M6, instruction-cache misses per
IPC, and nothing in this tree reads a cache-miss counter on any architecture. Milestone 134's
Tier B says so and its own BUGS warns that real PMUs do not implement every architected event, so
whether the U74 counts what M6 wants is unverified. Until then E3 remains what it was designed to
be: an inference from a perturbation, not an observation of a cache.

Wiring the PMU into these rows is `design/roadmap/0374-cycles-per-ipc-on-the-bench-card.md`.

Since 2026-09-16 the conversion exists without that wiring. radon measured
`cycles_per_tick 250.00` (milestone 74's block, transcript `bench/radon-2026-09-16/bench-134300.log`),
so every tick row above converts at 250 cycles per tick: the unpadded `call_reply` is about 1,254
cycles and the padded one 1,272. And the 2026-09-04 capture answers the question that proposal said
to wait for, which is whether the 250 ns quantum was the binding problem: it was not. The
within-condition spread was 0 to 2 ticks against a gap of 74, so E3's difficulty is the layout
confound, not the ruler.

## EXAMPLES

### The whole session, as a shell transcript

```sh
# on patagonia, no board
script/fastpath-footprint --arch riscv64
script/fastpath-footprint --arch riscv64 --features fastpath_pad

# un-padded card
script/board-image --bench --card /Volumes/NIFE
diskutil unmount /Volumes/NIFE
# power radon on, then, for each of three boots:
script/board-console --for 20m --until none --log target/radon-bench-plain-$(date +%s).log

# padded card
script/board-image --bench --extra-features fastpath_pad --card /Volumes/NIFE
diskutil unmount /Volumes/NIFE
# three more boots:
script/board-console --for 20m --until none --log target/radon-bench-padded-$(date +%s).log

# read the comparison out of the captures
grep -h "^bench: \(call_reply\|ipc_rtt\|ipc_rtt_el0\|cntfrq\) " target/radon-bench-*.log
grep -h "^bench: ipc_scale_" target/radon-bench-plain-*.log
grep -h "^bench-probe: appdisp_" target/radon-bench-plain-*.log
```

### Checking the card before walking to the board

```sh
$ script/board-image --bench
...
built:
  features: board,bench,single_hart
  target/board/nife-vf2.img  (669792 bytes, Image header verified)
  target/board/nife-initrd.img  (... bytes, the userspace archive this kernel measures)
```

A `features:` line missing `single_hart` is a wasted boot, and it is cheaper to read it here than to
read `ipc_thread_scaling skipped` twenty minutes later.

## BUGS

- E3 could not separate footprint from code layout, which is the defect the control was built
  for. The experiment varied one Cargo feature, which changes both the amount of resident text
  and the address of every symbol after it. The 2026-09-04 session found a 193 ns effect with a
  sign footprint cannot produce, so the 19 ns effect it *can* produce is not attributable. No
  reading taken that way is a footprint result, including the 2026-09-04 table on this page. The
  control landed 2026-09-19 (the section above); until an evening is run with it, E3 has no
  attributable number at all.
- The padding is never executed, so it can only ever act through addresses. A dead sled evicts
  nothing by itself; what it does is push other code apart. So even a clean dose-response measures
  whether `script/fastpath-footprint`'s number predicts latency, not Liedtke's claim about an
  executed footprint. Said in the procedure above as well, because it is the sentence most likely
  to be dropped when a result is quoted.
  [E5](footprint-perturbation/executed-footprint.md) plans the executed version.
- Four layout images are a small sample of a distribution. Stabilizer (Curtsinger and Berger,
  ASPLOS 2013) randomizes layout repeatedly for exactly this reason. Four draws can bound an effect
  loosely and cannot prove one absent; a pad reading just outside the layout range is weak evidence
  rather than a finding.
- The bench card's kernel is not the kernel the static table below measures. `bench` changes
  the IPC path's codegen (`ipc_call_reply` is 5,212 bytes with it against 5,936 without, on
  riscv64, and the normalized instruction stream differs), and `single_hart` adds four
  instructions; `board` alone changes nothing. Step 0 above therefore measures the card's own
  feature set rather than the bare release kernel, which the 2026-09-04 session did not: its
  "5,936 unpadded, 11,070 padded" is a true statement about a kernel nobody booted that evening.
  Milestone 134's optional per-IPC-depth boot rests on the same assumption and inherits this.
- `cargo xtask bench --riscv --extra-features <f>` silently ignored the features until
  2026-09-19, building a plain `bench` kernel and printing its numbers under the flag. Found by
  this control's QEMU boot proof; fixed in `xtask` (the x86_64 arm had it too). Any riscv64 E3
  rehearsal taken through that path before this date measured an un-padded kernel.
- A bench boot takes about 75 seconds, and step 3's twenty-minute window is therefore very
  generous. Measured over six boots on 2026-09-04, all of which reached `bench: done`. The
  deadline was the thing this page was most worried about being wrong and it was wrong in the safe
  direction.
- `single_hart` booted on hardware six times and nothing noticed, which was this page's second
  unverified claim. Three U74s left parked exactly as OpenSBI handed them over produced no fault,
  no timeout and no skip line. The boot-to-boot spread with one core is 0 to 2 units on a
  four-figure tick count, against the fifteenfold placement spread notes/soak.md records for a
  four-core card.
- Nobody has repeated the session on a second card or a second board. Everything here is one
  microSD card in one VisionFive 2 on one evening, and the layout confound above is the reason to
  care: a different card or a different board would not change the layout, so it would reproduce
  the same artifact and look like confirmation.
- ~~`single_hart` has never been booted on hardware either.~~ Superseded by the entry two
  above, which records six hardware boots on 2026-09-04 in which nothing noticed. The two entries
  contradicted each other from that day until 2026-09-19; this one was written before the session
  and the other after it, and only the other was updated.
- Nothing in CI compiles this card, or any card. `board`, `soak_test`, `job_mix`,
  `reboot_soak_test`,
  `single_hart` and `fastpath_pad` are built when a person runs `script/board-image`, minutes before
  walking to the bench. A refactor that breaks a card build leaves the tree green until then, and
  the error arrives at the worst possible moment. Six release builds of one crate would close it:
  `design/roadmap/0373-board-only-features-nothing-compiles.md`.
- The bench card measures fewer things than an ordinary one. `smp_throughput`, `fs_read` and
  `fs_throughput` self-skip under `single_hart`. A session that wants a multi-core number from
  radon builds a second card without the flag, and that card cannot produce E1 or E4.
- The images are still mostly indistinguishable on the card, with one exception since
  2026-09-19. Same three filenames, no build stamp in the payload, and nothing on the board
  prints its feature set, so the mitigation for everything else is the `features:` line in the
  build output and the operator's own log filename, which is rung four of AGENTS.md's ladder. The
  exception is exactly the case an eight-image interleave would break on: a `fastpath_pad` bench
  boot prints `bench-probe: fastpath_pad units <u> shift <s>`, so E3's images name themselves. A
  kernel that printed its whole feature set would be rung three for the rest and is proposed in
  `design/roadmap/proposals/a-boot-banner-that-names-the-build.md`.
- E3's `black_box` guard is a confound on both shapes, and the dose-response removes it from the
  comparison rather than from the kernel. Every image in the ladder above carries the guard,
  including the zero rung, so it cancels between conditions; it remains a difference between any of
  them and a build with the feature off, which is what the 2026-09-04 table compared. `ipc_send`
  and `ipc_call` each carry one untaken compare-and-branch when the feature is on. `kernel/src/fastpath_pad.rs`'s
  module doc prices it at around a nanosecond against a low-microsecond round trip; on radon the
  round trip is longer and the branch is not faster, so the ratio only improves. It is still a real
  asymmetry between the two builds and it is why an effect at the 1% level should not be believed.
- The riscv64 TCG self-skip is a single-value test. It says "not QEMU `virt`", not "has a
  32 KB L1". Another riscv64 machine with a different timebase would run E1 and E4 and be believed.
  Which machine a capture came from lives in this page and in the operator's log filename, not in
  the kernel.
- Nothing here re-takes E2. The thread census (4 new threads on the SMB/FS path) was taken on
  both ISAs under QEMU on 2026-08-22 and is a topology fact rather than a timing one, so it does
  not need the board. If the customer path changes, E2 changes, and E1's reading against it
  changes with it.
