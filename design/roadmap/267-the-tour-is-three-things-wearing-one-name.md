---
status: BUILT
raised: 2026-09-09
built: 2026-09-13
---
# 267. The milestone tour is three things wearing one name, and only one of them belongs in the kernel

Built 2026-09-13. Minted 2026-09-09 by calef, from one question: *"Does the milestone
tour need to be part of the build at all? Can the milestone tour just be a userspace program that we
run if we want the milestone tour?"* Built on `milestone/267-tour-split`. The split, the remainder
list and the measurement are below and all three stand.

The narrative program was deleted on 2026-09-13, by calef's ruling, on `maintainer/delete-narrator`.
The next section is what it said and why it went, and it is deliberately the first thing in this
block rather than a footnote at the end. This block turned BUILT at that deletion rather than
before it, and the reason is worth stating because the word looks wrong at a glance: the only
thing holding 267 at PARTIAL was that the program could not be typed at a prompt, and deleting the
program does not deliver that half, it **dissolves** it. There is no program, so there is nothing
left outstanding, and `PARTIAL` asserts remaining work that no lane could now pick up.

`REMOVED` was considered and refused. It is the right word when a milestone's code is deleted, and
some of this one's was; but three of the four things this block built are still in the tree and
enforced (`print_machine_description`, its `script/lint` check, and the kernel-only remainder
comment), so a `REMOVED` row would tell a reader at a glance that the tour split was reversed. It
was not. What was reversed is the vehicle the narrative moved into, and that is a paragraph's worth
of fact rather than a status word's.

## What the narrative said, and why it is gone

The text, verbatim, as `user/src/narrator.rs` held it and as the boot printed it. It is kept
here because AGENTS.md's rule about branches applies to deleted files with nothing changed: nobody
reads a branch, and nobody reads a file that is not there. This is a well-made artifact and the
argument for this system's design in nine lines; the git history is not where an argument lives.

```
milestone 1: we are running our own code on a CPU with nothing underneath it.
milestone 2: and when it goes wrong, we get told.
           : and the machine now tells us what it is, instead of us guessing.
milestone 3: and we know which parts of it are ours to give away.
milestone 4: and nothing writable is executable, and Vec works again.
milestone 5: and the machine can now interrupt us. we are preemptible.
milestone 6: and a thread that refuses to yield gets preempted anyway.
milestone 7: and now it runs a binary it did not compile, unprivileged.
           : and that binary can talk to a server it can only name, not reach.

  every line above was printed by a program at EL0, through a console driver that is
  also a program at EL0, and the kernel does not contain a line of code that puts a
  user's bytes on the wire.
```

And the observation that made this milestone happen, which should outlive the program it
produced. That closing claim used to be printed by the kernel, on the program's behalf. It was
the one line of the tour not demonstrated by the thing saying it: a `println!` at EL1 asserting that
no kernel code puts a user's bytes on the wire. Noticing that a record can be in the wrong mouth is
what 267 is about, and it is a reusable way to read this tree. The fix held for four days and is
worth keeping as a habit rather than as a program.

Why calef ruled it deleted, and none of this is "267 was wrong". Each reason is about what the
program became, not about the argument that created it.

- Nothing verified it. No test asserted those lines ever reached the console. If it had silently
  stopped printing, nothing in this tree would have gone red, which makes it the shape of record
  AGENTS.md's ladder puts at rung zero: true only while somebody happens to notice.
- The text was frozen at milestones 1 through 11, and there are 267. No gate compared it with
  `design/roadmap/`. The program's own header defended the freeze (*"the story stops early on
  purpose: it is the argument for the design, not a changelog"*), which is a fair defence and is
  still a boot that prints an argument where a reader may reasonably expect a status.
- It existed on aarch64 only. riscv64 and x86_64 each halt in their own arch tour before the
  shared path it belonged to, so two of three supported architectures never printed a word of it.
  Rule 5 (parity is a gate) is not violated by a demonstration, but a demonstration one architecture
  in three can see is a weak demonstration.
- The demonstration is redundant with the system existing. At milestone 11, *"a userspace
  program printed to the screen"* was remarkable. At 267, with a shell, a compositor, a filesystem
  and a network stack all printing through userspace, it is ordinary. The claim stays true; it
  stopped needing a dedicated program to say it.

What went with it: `user/src/narrator.rs`, its `[[bin]]` stanza in `user/Cargo.toml`, its
`("narrator", "narrator")` entry in `xtask`'s `initrd_aarch64` archive table, the spawn block in
`kernel/src/main.rs`, and `kernel/src/user/console_service.rs`'s `spawn_client`, which had exactly
one caller and was that block.

**What deliberately did not go with it:** `console_service::start` and `components/src/console.rs`.
Deleting those is a larger change than was authorised, because it removes infrastructure rather than
a demonstration, and the question it raises is in the Follow-on section below with what the compiler
says about it.

## The measurement, first, because it changed what this milestone is about

`script/fastpath-footprint`'s method: release kernels, `llvm-nm --print-size`, `.text` symbols
summed. Measured at `8dd9dbf6`, before anything moved.

| arch | tour build | `--features shell` | what the tour costs |
|---|---|---|---|
| aarch64 | 193,332 | 179,312 | **14,020 (7.3%)** |
| riscv64 | 162,964 | 157,136 | 5,828, and it is not the tour |
| x86_64 | 140,842 | 140,842 | **0** |

x86_64 is zero, and had always been. Its boot arm is self-contained and ends in `arch::halt()`,
so the shared milestone tour is unreachable and LLVM had already deleted it. The boot-mode features
have never removed a byte on that architecture.

riscv64's number is a different quantity. That arm halts too, and its `shell` feature swaps its
own arch tour for `user::riscv_shell_boot`. 5,828 bytes is the difference between two RISC-V boot
paths, not the cost of the 230 lines this block was written about.

So the tour is one architecture's 14 KB. And the symbol-level attribution is the part that
matters, because it says the narrative is not where the bytes are:

| tour-only symbol group | bytes |
|---|---|
| `console_service::start` + `spawn_client` and their closures | ~3,040 |
| the two never-yielding spinner closures in `kernel_main` | ~3,336 |
| `virtio_service::wire` and its closures | ~1,692 |
| `memory_region_service::start` and its closures | ~1,276 |
| `kernel_main` itself (the driving code and the format strings) | +1,460 |
| `pci::find_block_device` | 436 |

Every one of those is a **demonstration**, and every demonstration is on the kernel-only list below.
The prose lived in `.rodata` as anonymous string constants, below the linker's 4 KiB section
granularity and, as it turned out, below the optimizer's noise floor: writing the new console-server
hoist two equivalent ways moved `.text` by 340 bytes, which is more than nine lines of text ever
weighed.

After the move: 193,996 tour, 179,332 shell. The kernel is 664 bytes larger than it was. That is
the finding and it is worth stating without dressing: the compile-time exclusion was not buying what
its name suggests, and moving the narrative out cannot make it buy less, because what costs bytes is
exactly what cannot leave.

The consequences for the feature pair are in
`design/roadmap/proposals/two-boot-mode-features-for-one-decision.md`. Nothing was retired here.

## The three things, which want opposite treatment

Boot output was one undifferentiated stream, compiled in or out as a unit. It is three audiences.

**The machine description** is diagnostics and belongs on **every** boot, never compiled out.
Paging, ISA, firmware, cores, ACPI tables, PCI ECAM, timers, memory map. **xenon proved this is not
decoration**: at first light there was no serial console this project could read, so the boot output
*was* the transcript, read off a photograph, and it is what diagnosed both the local-APIC collision
and the PCI BAR window landing in RAM. A port is verified by reading these lines.

The milestone narrative is a demonstration. *"milestone 1: we are running our own code on a CPU
with nothing underneath it."*

And a genuine kernel remainder that cannot move anywhere, because it demonstrates facts only
kernel code can establish.

## What was built

1. The machine description is `print_machine_description`, called unconditionally from
`kernel_main`, with byte-identical output. It was not gated on `shell` or `initboot` before either,
which is the one thing this block's original text got wrong: the `cfg` at what was line 1540 sat
*after* the description and governed only the tour. What was true is the rest of the diagnosis, that
the boundary between the two audiences was a reader's inference about where a line happened to sit,
and a line moved a few rows down would have changed audience with nothing saying so.

`script/lint` now fails if a `feature = "..."` cfg appears anywhere inside that function. The
signature keeps `#[cfg(not(any(test, feature = "bench")))]` and the check deliberately does not look
at it: a `bench` boot diverges into `bench::run` before this point and a `test` boot exits through
semihosting, and neither is a boot anybody reads to bring up a board.

2. The narrative was `the narrator program`, a program at EL0, spawned as the console server's
client, and it is deleted (2026-09-13, the section above). The console server was moved to the
top of the tour to feed it, which kept the transcript in the order it always read, and the `hello`
printing client it used to feed went at the same time: running two programs to demonstrate one thing
was one program too many.

This closed a gap nobody had named, and closing it is the part of item 2 that survives the deletion
as a finding. The kernel used to print *"a userspace program printed to the screen, and the kernel
does not contain a line of code that puts a user's bytes on the wire"* on the program's behalf,
which was the only claim in the tour not demonstrated by the thing making it. For four days the
narrator said it, through a driver at EL0 that holds the UART, from an address space that could not
reach the device at all. The default boot now prints no narrative at all, which is the reading
this block's own BUGS section recorded as considered and refused in September and which calef ruled
for in the end; the BUGS entry below is rewritten rather than deleted, because what changed is the
answer and not the fact that the question was live.

3. The remainder is listed in one comment at the top of the tour block, with the privilege each
entry needs:

| survivor | why it cannot leave |
|---|---|
| two threads that never yield | `sched::spawn`, `sched::preemptions()`, `timer::spin_for`. An EL0 spinner proves only that EL0 is preempted; these run *inside* the kernel and are still taken off the CPU, which is the stronger claim |
| the virtio and PCIe block demos | the kernel enumerates the bus and mints registers, a DMA page and an interrupt into a driver's world, then reads the report with `sched::ipc_recv`. The granter cannot be the grantee |
| the outlaw | `&raw const USER_FAULTS` is the address of a kernel static, handed to a program that faults reading it and increments the counter it reached for. A program cannot name that address |
| the memory-region demo | it prints `memory::stats().used` before and after a process spends its own budget, and the claim is that the number did not move. That number is the kernel's own frame accounting and is not exposed to EL0 |

4. The measurement is above.

## The `Reached(Tour)` consumers, since the block asked for them by name

All of them, and none of them observe the thing this milestone moved.

`Stage::Tour` is reached by exactly one line of text, `"nife: the capability core runs on "`, printed
at `kernel/src/main.rs:1401` in the RISC-V arm. The aarch64 tour prints no marker at all. The
consumers are `crates/board_console/src/progress.rs` (which reaches the stage and labels it),
`watch.rs` (whose `quiet_after` policy exempts exactly this stage, because a boot that ends in `wfi`
is quiet on purpose), `port.rs`, and `xtask/src/main.rs:9383`, which parses `--until tour` on the
command line. Six tests in `board_console` assert on it.

Nothing in that set was touched, and nothing in it could have been: it is the VisionFive 2's boot
that it watches.

## What must not break, and did not

- The machine description prints on every boot, on all three architectures. It is now harder to
  break than it was: it is a function with a name and a gate rather than the first half of a block.
- `xtask`'s archive manifest. `narrator` is packed into `initrd_aarch64`'s table. aarch64 only:
  the other two boots halt before the shared path, so packing it there would be archive bytes
  nothing can reach.
- `script/swish-check` runs the real boot on both legs. Green.

## The proof

As built (2026-09-09). `script/server` printed the machine description, then the narrative
arrived from a program at EL0 through a driver at EL0, then the kernel-only demonstrations, then the
progenitor built the system and the prompt appeared. `--features shell` printed the machine
description and went straight to the prompt.

After the deletion (2026-09-13). The same boot with the narrative paragraph missing:
`script/server` prints the machine description, then the kernel-only demonstrations, then the
progenitor builds the system and the prompt appears. `--features shell` is untouched, because it
never compiled the narrative in. The console server still comes up on a tour boot and prints
nothing, which is the Follow-on question below rather than a step in this proof.

## BUGS

- The default boot now prints no milestone narrative, and this block argued both sides before it
  got there. Its "proof" paragraph asked for *"a boot that prints the machine description and
  nothing else"*; its BUGS section argued back in the same breath that `script/server` is what a
  stranger runs first and *"run this other program" is a worse default than "it prints"*, and in
  September the second argument won. calef ruled for the first on 2026-09-13, and the route
  there was not the argument being re-run: it was that nothing verified the narrative, nothing
  checked its text, and two of three architectures never saw it. A reader meeting an empty-feeling
  boot should know the trade was made deliberately, twice, in opposite directions, with the reasons
  recorded both times.
- **`narrator` was never ratified and now never will be.** Names in this tree are an architect's,
  and this one shipped provisional under the rule that lets a lane ship rather than wait. `tour` had
  been refused because this tree spends the word on the boot path itself (`Stage::Tour`,
  `script/board-console`). The name is gone with the program; the refusal of `tour` stands on its
  own and is recorded here so the sweep that deleted the program does not also delete the reasoning
  that would be re-derived by the next person who wants the word.
- **A program that prints and nothing that checks it is the shape to watch for, not a fixed
  defect.** The narrative was unverified for its whole life, on both sides of the move: nothing
  asserted the lines reached the console when it was twenty `println!`s at EL1 either. It is worth
  a reader knowing that the tour's remaining demonstrations below are in the same position, with
  the exception of the ones `script/swish-check` boots.
- **The move made the kernel 664 bytes bigger, and the deletion has not been re-measured.**
  Measured, above, on 2026-09-09. The move bought the split, the self-demonstrating claim, and a
  program; it did not buy bytes, and a reader should not infer that it did from the fact that a
  milestone about the cost of the tour exists. The 2026-09-13 deletion removed the spawn block,
  `spawn_client` and one archive entry, so the direction is certainly downward, but no number in
  this block was re-taken and none should be quoted as if it had been. `script/fastpath-footprint`
  is the method if somebody wants it; it was not run because the deletion's correctness does not
  depend on the answer and an unverified number in a table of verified ones is worse than a gap.
- **The interleaving hazard grew and then went away again, and the underlying one did not.** The
  kernel's own UART driver and the userspace console server write the same device with nothing
  arbitrating, which `script/swish-check`'s BUGS section documents at length and
  milestone 230 (`script/shell-check` is red on `main`, on both architectures) proposes fixing. This milestone put fifteen more userspace-printed lines on the default boot; the
  deletion took all fifteen back, along with the `timer::spin_for` window that had been keeping the
  two writers apart. The unarbitrated device is unchanged, because it was never the narrative's
  doing: the boot-time console server still holds the UART's registers on a tour boot, it simply has
  nothing to print through them now.
- **Preemption counts are still not proposed for exposure.** A supervisor or benchmark might want
  them one day, and the shape is already established by §139 and milestones 229/237: a per-thread
  grant, not an ambient fact. Nothing needs it today, so nothing is built. This is why the spinner
  demo is on the kernel-only list rather than being a program.

## Follow-on

- **Milestone 394.** The boot-time console server now comes up with no client at all: `start` is
  still called on a tour boot and `rustc` reports every field of its `Console` handle as never read.
  Whether it should keep being started, and what it was infrastructure for, is an architect's rather
  than a lane's, and 394 carries the three options with their real costs. Numbered on 2026-09-19 by
  milestone 433's drain of the pile.
- **Refused.** "A narrative program the shell cannot start" is moot and its proposal file is
  retired. It asked for one of three ways to let `swish` start `narrator`, all of which presupposed
  a narrator; calef deleted the program on 2026-09-13, so the missing half of "a program you can run
  on purpose" is not unfinished work but a question about a thing that no longer exists. The
  proposal's one durable finding is not lost: that this tree has two ways for a program to
  receive an output channel (the console server's raw shared-page-plus-two-endpoints protocol, and
  `crates/byte_sink_proto`'s sink), and that a program written against one cannot be started by the
  other. That is recorded here rather than left in a deleted file, because it is true of every
  program and was only ever illustrated by this one.
- **Milestone 296.** Whether `initboot` and `shell` should collapse to one feature, or stop being a
  compile-time switch at all, now that the measurement exists: 7.3% on one architecture, zero on
  x86_64, and something else entirely on riscv64. Not acted on here, on purpose.
  `design/roadmap/proposals/two-boot-mode-features-for-one-decision.md` was this lane's file and is
  the block milestone 296 was promoted from. Answered: the switch is kept on that 7.3%, and it is
  one feature, `shell`. `initboot` was deleted after a build of each was found to carry the same
  3,109 symbols at the same total size.
- **Milestone 296.** `design/roadmap/proposals/what-the-boot-path-is-called.md` is **not** retired by
  this milestone, which is the opposite of what this block originally predicted. Checked by reading
  it: it chooses between `progenitor-boot` and `handoff` for the boot mode, and both features still
  exist and still mean the same thing, so the question it holds is untouched. What changed is that
  there is now a fourth candidate, because the feature is better described as naming the absence of
  the demonstrations than as naming a boot path. (Carried here as `**Proposed.**` rather than
  `**Outstanding.**` because this block is BUILT and the work is nobody's.) It was retired by
  milestone 296, by a fifth answer none of the four candidates could be: calef deleted the thing
  the proposal was naming.
- **Recorded.** The kernel's UART driver and the userspace console server write the same device
  unarbitrated. The limitation is `script/swish-check`'s and milestone 230's, and it is **not**
  this milestone's doing in either direction: 267 briefly added fifteen userspace-printed lines to
  the default boot and the 2026-09-13 deletion removed all fifteen along with the `timer::spin_for`
  window that separated the writers, leaving the hazard exactly where it was found. Recorded in the
  BUGS section above and beside the code in `kernel/src/main.rs`.

## Index row

calef, 2026-09-09. The machine description is diagnostics and must always print, xenon proved that
with no serial console; the narrative is a demo that could be a program; the remainder needs
sched::spawn and a kernel static. It also dissolves the shell/initboot feature pair, which mean
the same thing to the kernel. The narrative program was deleted 2026-09-13 on calef's ruling and
the block keeps its text; the split, the lint check and the remainder list stand
