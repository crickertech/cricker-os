# Reading a board, and the one thing this tool says back

Milestone 216. `script/console` boots a shell in QEMU. `script/board-image` builds the VisionFive
2 payload and prints the `dd` commands for a card. Between those two there was nothing, so every
milestone gated on real hardware needed somebody at a terminal emulator reading with their eyes.
`script/board-console` is the missing middle: it opens the port, logs every byte, recognises how
far the boot got, and stops on a deadline.

The last clause is the only hard part. Opening a serial port is a `screen` invocation. Knowing
when to stop reading is what kept this a milestone.

And since milestone 324 it writes, under one rule: *it writes only what a named mode sends,
and every byte it sends is printed into the log.* calef ruled that on 2026-09-19, replacing this
note's original *"it reads and never writes to the board"*. There is exactly one named mode today
and it sends exactly one byte: `--stop`, which ends milestone 249's self-rebooting soak. The
second clause is the half that makes the first safe, because the log is the artifact a bench run
is judged from and a byte the board received that the capture does not show makes the capture a
lie about the run. **This is not a keyboard**, and the narrowness is the decision rather than
caution about it; "Writing to a board" below has the argument.

## The commands

```
script/board-console                                # watch until the kernel banner, 120s cap
script/board-console --for 30m --until none         # sustained watching, for a stress run
script/board-console --port /dev/cu.usbmodemXXXX    # when two adapters are plugged in
script/board-console --replay target/board-console-1756744000.log   # re-read a capture
script/board-console --stop                         # end a rebooting soak at its next draw
script/board-console --stop-after 50                # end one with exactly fifty samples in it
```

| flag | default | what it does |
|---|---|---|
| `--port <dev>` | the only USB serial adapter in `/dev` | Which device to open. `NIFE_BOARD_PORT` sets it too. Two adapters with no `--port` is an error, not a guess. |
| `--log <file>` | `target/board-console-<epoch>.log` | Where the bytes go. There is no way to turn it off. |
| `--for <duration>` | `120s` | The hard cap. `90`, `90s`, `30m`, `2h`. |
| `--board <name>` | `radon` | **Which firmware prologue to expect** (milestone 324). `radon` is the VisionFive 2's chain; `xenon` has none, because it boots through PVH straight into our banner. argon has no profile on purpose. |
| `--until <stage>` | `banner` | Stop early at one of this board's firmware rungs (`spl`, `opensbi`, `uboot`, `handoff` on radon), or at a shared rung: `banner`, `machine`, `selftest`, `tour`, `prompt`, `soak`, `sweep`, `sweep-done`. `none` watches the whole duration. A word this board has no rung for is refused, not waited out. |
| `--quiet-after <duration>` | `15s` | Give up if the board speaks and then stops. `0` disables it. Suppressed once the tour completes, always. |
| `--stop` | off | `--stop-after 1`. Send the byte that ends milestone 249's rebooting soak, at the next draw. |
| `--stop-after <n>` | off | Send it at the n-th armed draw this session sees, so the series has exactly n samples. |

The two stop flags change two other defaults, because both answer "when does this session
end" and a stop mode is the answer. `--until` becomes `none` (the escape ends the session, not a
stage), and an explicit `--until <stage>` alongside a stop is refused rather than overridden: a
soak is reached long before its reboot loop arms, so `--until soak --stop` would return before
sending anything. `--for` defaults to `150s` per draw plus two minutes, derived from
`kernel/src/soak.rs`'s two-minute draw and the twenty-odd seconds a boot takes, so `--stop-after
50` gets the two unattended hours milestone 249 prices it at. Both are overridable and neither is
an agreement with anything; getting them wrong costs a re-run. `--stop` with `--replay` is refused
outright, because a file has no board on the other end of it.

And one mode that opens no port at all (milestone 249): `--tally <log>` reads a capture of many
boots and reports what the thread-placement lottery drew on each. It is `board_console::lottery`
rather than the recogniser, it answers a question no single-boot reader can be asked (how often does
each arrangement come up), and its exit statuses are only `0` and `4`, because an analysis of a
finished log has no board left to have gone quiet. See notes/soak.md.

## The exit statuses, which are the point

A bench script needs to tell a hang from a refusal, and a tool with two exit codes cannot.

| status | meaning |
|---|---|
| 0 | Reached the stage asked for. With `--until none`, watched the whole duration and the board announced nothing. |
| 1 | The board announced a failure: a bad image header, a measured-boot refusal, or a kernel panic. |
| 2 | It spoke and then went quiet. **This is what a hang looks like from this end**, and it is the one the multicore risk cares about. |
| 3 | The time ran out with the requested stage unreached. |
| 4 | The port could not be opened, or the arguments were wrong. No session happened. |

## Writing to a board, which is one byte and one reason

Milestone 324, parts 1 and 4, on calef's ruling of 2026-09-19. Everything above this heading
reads. This section is the only part of the tool that writes, and it is worth reading before
anyone extends it, because the shape is the decision.

### What it sends, and what it will not

One byte, `0x0d`, a carriage return. `kernel/src/soak.rs`'s escape accepts any byte at all, so
this is a choice inside a contract that is already fixed rather than anything two programs agree
on. Carriage return because the kernel's own instruction is *"press any key on this console"* and
Enter is what a person presses when told that: a tool whose byte is the one the documented manual
procedure produces is running the same experiment the procedure runs. `NUL` was the other
candidate, refused narrowly, and the reasoning is in `crates/board_console/src/stop.rs` beside the
constant.

It will not send anything else. There is no mode that types a line, no mode that drives U-Boot,
and no keyboard. **The hazard is not writing; it is an open keyboard beside a countdown a stray
byte consumes.** Milestone 249's lane sent this same escape by detaching the console, hit U-Boot's
autoboot countdown with it, and paid a power cycle. That incident is the argument, not a
preference about tidiness.

### When it sends, which is the whole of the safety

Only after the board has printed its arming announcement, the `THIS BUILD REBOOTS THE BOARD`
line `kernel/src/soak.rs` prints from `arm_reboot`. Nothing else in a session can cause a byte to
go out. Four things follow from that one gate:

- The kernel's own drain is already behind us. `arm_reboot` calls `console::discard_rx` and
  *then* prints, so a byte sent any earlier is a byte the kernel deliberately throws away. This is
  the trap worth knowing about: `soak-test: started` is printed **before** `arm_reboot`, with four
  more lines between them, about a kilobyte, roughly ninety milliseconds at 115200 baud. A sender
  triggered on that line would lose the race nearly every time, silently, and the failure would look
  exactly like a board whose receive path is miswired.
- There is a reboot loop to stop. A plain `--features soak_test` kernel never prints the banner
  and never polls for an escape.
- The board is long past its firmware, because a kernel that has printed this has been running
  for the length of a boot tour. The countdown hazard is closed by the gate rather than by the
  byte.
- It counts draws. One banner per boot of a rebooting build, one-to-one with the
  `soak-test: started` lines `script/board-console --tally` opens a draw on, so a log produced by
  `--stop-after n` tallies as exactly n draws.

So the answer to *what if `--stop` is given and the board is powered off, or at a U-Boot prompt, or
running a kernel with no soak in it* is one answer in all three cases: nothing is sent, the
session ends on its deadline, and the tool says no armed reboot loop announced itself. The cost of
the gate is one sentence: a session that attaches to a board already mid-draw waits out the rest of
that draw, up to two minutes, before the next banner arrives.

### What confirms it, and why that is new

The kernel prints `soak-test-reboot: DISARMED` when the escape lands. The tool waits fifteen
seconds for it (three of the kernel's five-second polls) and the exit status says whether it came:
`0` confirmed, `3` sent and unacknowledged, `3` never sent. No new exit status was added.

**That closes a gap `kernel/src/soak.rs` states in its own `BUGS` and cannot close from where it
stands**: *"the escape is a poll of one bit and nothing verifies that the bit can ever be set
[...] Nothing in this kernel can prove otherwise, because a UART cannot receive a byte it sends."*
A host holding the far end of the cable is not under that limit, so a confirmed `--stop` is the
first end-to-end evidence that a board's receive path works. The procedure that kernel BUGS entry
names as the substitute (press a key on the first boot, confirm `DISARMED` before walking away)
becomes something a script does and records in the log.

### What the log shows

Three lines, and the ordering is deliberate: the announcement is written and flushed **before** the
byte goes to the port, so a write that fails reads correctly rather than leaving a log that claims
a byte nobody sent.

```
soak-test-reboot: THIS BUILD REBOOTS THE BOARD. It soaks for 120s, then asks the firmware ...

board-console: sending the soak escape to the board now: 1 byte, 0x0d. Draw 50 of 50 armed its
reboot at +7412.3s and this is the sample the series was asked for.
board-console: sent 1 byte, 0x0d. Waiting up to 15s for the kernel to say it found it.
soak-test: t=5s beat=1 rounds=... rate=.../s workers=24 refused=0 mismatch=0 stalled=0
soak-test-reboot: DISARMED at t=5s: a byte arrived on this console. This board will not reboot
itself again.

board-console: the board acknowledged the escape and will not reboot itself again. Its own line is
above; the soak keeps running.
```

The byte is rendered in hex and never written raw, so the capture stays greppable, cannot move a
reader's cursor, and cannot be mistaken for something the board said. A host test asserts that no
line this mode writes into a log reads as a boot marker when that log is replayed.

## What it recognises, and where each marker came from

Every marker was first quoted from `notes/visionfive2.md`'s bench runbook ("What appears, in order,
on a good day" and the failure-triage ladder) or from this tree's own source. They were then
checked against the board, on 2026-09-01, against a captured success and a captured failure that
now live in `crates/board_console/tests/fixtures/captured/` and are asserted on by the tests.

The first four rows are radon's and live in a board profile (milestone 324 part 3,
`crates/board_console/src/board.rs`); the rest are the kernel's and are shared by every board.

| stage | board | marker | source |
|---|---|---|---|
| `spl` | radon | `U-Boot SPL` | runbook, confirmed on the board |
| `opensbi` | radon | `OpenSBI v` | runbook ("record the version line"), confirmed: `OpenSBI v1.2` |
| `uboot` | radon | `U-Boot ` followed by a word that is not `SPL`/`TPL`, or `StarFive #` | runbook, confirmed both ways |
| `handoff` | radon | `Starting kernel ...` | runbook, confirmed |
| `banner` | every | `boot_ladder::BANNER` (`nife on `) | `kernel/src/main.rs`, confirmed |
| `machine` | every | `boot_ladder::MACHINE` | milestone 268 |
| `selftest` | every | `boot_ladder::SELF_TEST` | milestone 268 |
| `tour` | riscv64 | `boot_ladder::TOUR` | `kernel/src/main.rs`, confirmed |
| `prompt` | every | `boot_ladder::PROMPT` | milestone 268 |
| `soak` | every | `soak-test: started` | milestone 219 |
| `sweep` | every | `job_mix::STARTED` | milestone 324, confirmed under QEMU 2026-09-19 |
| `sweep-done` | every | `job_mix::DONE` | milestone 324, confirmed under QEMU 2026-09-19 |

xenon's profile is an empty prologue, and that is a measurement rather than a gap.
`bench/xenon-2026-09-17/first-light-095500.log` shows nothing before `nife on ` that this tool
matches, because the machine boots through PVH straight into our banner. A test replays that exact
file through the xenon profile and asserts the banner, the machine line, the five-of-five verdict
and the measured-boot refusal, all with no firmware rung climbed at all. That is what the split is
for: the same radon capture read through xenon's profile reaches the same tour and reports no
firmware rung, because none of those lines are xenon's to claim.

One more thing is reported and is deliberately **not** a stage: `init/build`, meaning userspace init
built its child. It cannot be a stage without breaking the ladder, because a card with no archive
runs the whole tour and never reaches it, so putting it below `tour` would make reaching the tour
imply something that did not happen. It is a detail of a successful boot, like `Moving Image from`,
and it is the only difference between the two successful captures.

Since milestone 295 it reads captured logs and nothing else. calef retired
`components/src/builder.rs` on 2026-09-14, so no kernel this tree builds prints `init/build` and
`userspace_ran()` is `false` on every live board. The matcher stays because
`tests/fixtures/captured/vf2-2026-09-01-userspace.log` carries the line, and that capture is
evidence off real VisionFive 2 silicon that cannot be re-taken with a different kernel; deleting the
recogniser to tidy the code would throw the evidence away. What to ask of a board booted today is
`reached() >= Stage::Prompt`, and it is a stronger question: `init/build` meant userspace built one
child out of two capabilities, where the prompt cannot appear unless userspace built the console
server, the line discipline, the input driver and the shell. The two successful captures are still
the two successful captures; what distinguishes them is now a fact about 2026-09-01 rather than a
test to run.

And five things that end a session early rather than waiting the clock out:

| what | marker | source |
|---|---|---|
| **U-Boot giving up**, with the reason on the line before | `### ERROR ### Please RESET the board ###` | **captured 2026-09-01**; no documentation in this tree named it |
| a stale or wrong card | `Bad Linux RISCV Image magic!` | triage ladder row 3, never seen at a bench |
| the trust boundary refusing | `MEASURED BOOT REFUSED` | `kernel/src/trust.rs`, **captured 2026-09-01** |
| our own panic, with its message | `[PANIC] ` | `kernel/src/panic.rs` |
| U-Boot relocating (a note, not a stage) | `Moving Image from` | triage ladder row 4, confirmed |

### Four outcomes, three of them real

The captures of 2026-09-01 turned this from a two-way question into a four-way one, and three of
the four are things a board actually did.

| outcome | what it looks like | exit |
|---|---|---|
| **Success** | the tour reaches `nife: the capability core runs on RISC-V.`, then the board goes quiet because the kernel halts in `wfi` | 0 |
| **U-Boot refused before the kernel ran** | the image loads and relocates, then `### ERROR ### Please RESET the board ###` | 1 |
| **The kernel booted and then halted on purpose** | `MEASURED BOOT REFUSED`, after the banner and most of a tour | 1 |
| **A genuine hang** | it starts, says a few lines, and stops before the tour | 2 |

Three of those four are traps for a naive recogniser, and each got a fix:

Both successes and the measured-boot refusal contain the banner. The refusal prints
`Starting kernel ...`, the whole nife banner, and most of a tour before halting, so a watcher that
returned the moment `--until banner` was satisfied would report it as a success, in the case a
bench script most needs to be right about. So reaching the wanted stage starts a **settle window**
(`settle`, two seconds) rather than ending the session, and a failure arriving inside it wins.

Silence after the tour is how a good boot ends. The kernel halts in `wfi`, so the board goes
quiet and stays quiet. Treating silence alone as a hang would fail every successful boot, so the
quiet timer is suppressed once the tour completes, whatever `--quiet-after` says. The synthetic hang
fixture stops *before* the tour, which is exactly the difference.

**A measured-boot refusal is the gate working, not a crash.** It exits 1, because the board did not
boot and a script asking "did it boot" needs a no. But the message says what actually happened and
what to do, because a report that read like a crash would send somebody debugging the boot mechanism
when what is wrong is that two files on the card came from different builds.

### The extlinux outcome, which documentation did not have

The capture that mattered most is the one that failed. From power-on, the extlinux path ends:

```
Moving Image from 0x40200000 to 0x80200000, end=802ff000
Device tree not found or missing FDT support
### ERROR ### Please RESET the board ###
```

That is exactly the caveat `notes/visionfive2.md` records about U-Boot's fallback DTB addresses,
and it arrives **after** the image has loaded and relocated and **before** the kernel has run a
single instruction. A recogniser that knew only the stages would have watched the image load, seen
nothing more, and called the silence a hang, which is the worst available answer: it sends somebody
hunting a multicore bug in a kernel that never started. **Booted, hung, and refused-before-the-
kernel are three outcomes, not two.** The tool exits 1 for the refusal and 2 for a hang, and the
difference is the difference between resetting the board and opening a debugger.

### And `tour` is a stronger claim than `banner`

`nife on RISC-V (rv64, S-mode, Sv39)` is printed before the device tree is touched, so it says the
console works and nothing else. `nife: the capability core runs on RISC-V.` is the last line of the
boot tour, so it says paging, traps, the timer, the frame allocator, SMP and the scheduler all came
up. `--until tour` is the one to want when the question is "did it work"; the default stays
`banner` because only the milestone-tour build prints the other, and a shell build would wait for a
line that is never coming.

### The card's U-Boot environment is degraded, and that is not ours

Both captures carry `*** Warning - bad CRC, using default environment`, several
`** Invalid partition 3 **` / `Couldn't find partition mmc 1:3` / `Can't set block device`
complaints, and `## Error: "boot2" not defined`, before U-Boot finds `mmc 1:1` and gets on with it.
The board boots through all of it. Nobody should read those lines as a defect in our payload, and
whether the environment is worth repairing is somebody else's milestone.

### Telling a finished job-mix sweep from a wedged one

Milestone 324 part 2, found by milestone 168's lane. `script/job-mix` boots a `--features job_mix`
kernel whose boot tour ends in a sweep over task counts rather than in a halt. Until this milestone
nothing recognised any of it: a finished sweep and one that stopped partway both ended as the clock
running out, so they shared an exit status, and `cargo xtask job-mix` had no timeout at all and hung
forever on a wedge.

Two rungs close that. `sweep` is `job_mix::STARTED`, meaning the pool spawned and the workload
announced itself. `sweep-done` is `job_mix::DONE`, meaning every point printed and the kernel is
parking. So:

```
script/board-console --until sweep-done --for 30m     # at a board
script/job-mix --arch riscv64 --smp 4                 # under QEMU, same judging, same statuses
```

returns `0` for a sweep that finished, `1` for one the kernel refused to start (`job-mix: FAILED`),
`2` for one that spoke and then stopped, and `3` for one still printing when the clock ran out.

**The quiet timer is the hard part, and a sweep is harder than a soak.** A soak beats on the wall
clock every five seconds whatever the workload is doing, so a missed beat is a missed deadline and
fifteen seconds is three of them. A sweep speaks only when a subrun ends. The longest subrun is the
top of `job_mix::TASK_SWEEP`, measured at 4.0 seconds
(`crates/board_console/tests/fixtures/captured/qemu-2026-09-19-aarch64-job-mix-medians.log`,
249,234,771 ticks on a 62.5 MHz counter), so `script/job-mix` defaults `--quiet-after` to sixty
seconds, fifteen times that. A board outside that margin will be called wedged when it is merely
slow; `--quiet-after 0` is the answer and it gives up the wedge detection. This is in both `BUGS`
sections because it is the one number here a bench operator may have to change.

And the margin is spent by changes nowhere near it. It was twenty to one against a 2.6-second
subrun when this paragraph was first written, earlier on 2026-09-19. Milestone 168 landed the same
day and took twenty-one repeats of a seven-kind mix where there had been three of a five-kind one,
which made the slowest subrun half again as long and cost a quarter of the headroom without anybody
touching `--quiet-after`. A number quoted from a capture is only as current as the capture.

And `job-mix: done` joins the quiet exemption, with `tour` and `prompt`, because the kernel
halts in `wfi` after it. `sweep` deliberately does not: silence during a sweep is the wedge.

## Two things in the design that are not obvious

A partial line is weaker evidence than a complete one. The recogniser is offered the
incomplete tail of the stream as well as the finished lines, because U-Boot's `StarFive #` prompt
has no newline after it and a tool that waited for one would sit there while the board sat waiting
for it. But a tail is ambiguous in two ways that a byte-at-a-time test found and reasoning did
not. `U-Boot ` reads as U-Boot proper right up until the next three bytes turn out to be `SPL`, so
a board dying in SPL was being reported two stages further along than it got. And a marker
carrying a payload captures a truncated one: the tail `nife on ` recorded the banner as the empty
string, and `[PANIC] ` recorded a panic with no message, both latched before the rest arrived. So
a tail may ratchet a stage, because a substring match is monotone and more bytes cannot unmake it;
it may not settle a word boundary and it may not capture text.

The read happens on its own thread, so the deadline holds whatever the reader does. The port
is configured with `min 0 time 1`, which makes a `read` return after a tenth of a second with
whatever arrived, including nothing. When that works, a single-threaded loop would be fine. There
are several ordinary ways for it not to work: an `stty` that failed and was only warned about, a
driver that ignores the setting, a file or a pipe standing in for a port. So the read runs on a
worker and the loop waits on a channel with a timeout. The worker may still be parked in `read`
when the tool decides to stop, and that is fine: deciding to stop means returning, the process
exits, and the descriptor goes with it. This is `CLAUDE.md`'s *Never leave QEMU running* rule
wearing different clothes. An emulator that never exits and a board that never speaks are the same
bug seen from the tool's side.

Open the device first, then run `stty` on it, then check that the speed took. Opening a macOS
`cu` device resets its termios towards the driver's default, so a configuration made before the
read descriptor exists is undone by the open that follows it; and holding the descriptor is what
keeps the setting alive, because it reverts when the last user closes. Measured on the rig: the
CH343 sits at 9600 when nobody holds it and reads 115200 while this tool does.

**This is not a theory, and somebody already paid for it.** calef's first capture of the board on
2026-09-01 was pure garbage, because it configured the port with `stty` and then ran `cat`, and
`cat`'s open put the device back to the default rate. It looks exactly like a wiring fault, which
is the triage ladder's second row, and it cost a power cycle to diagnose. So the tool now reads the
speed back after configuring and says loudly when it is not 115200: an invisible wrong baud sends
the next person chasing hardware, and a stated one takes a second to fix.

The residual hazard, stated rather than hidden: because the configuration is a separate process
rather than a `tcsetattr` on our own descriptor, any *other* process that opens the device
mid-session can put it back, and nothing here would notice. That is the honest cost of having no
dependency, and it is the case that would justify taking one.

**`O_NONBLOCK` is not used, and that is a decision rather than an oversight.** Its job would be to
stop a silent board hanging the tool in `read`, and the worker thread above already guarantees
that. The thread's guarantee is the stronger one: it holds even when the `stty` fails outright,
where a non-blocking read on its own would not, because a non-blocking read returns immediately but
only a deadline decides when to stop asking.

**And `cu.`, never `tty.`** On macOS the `tty.*` name is the dial-in side and blocks in `open`
until carrier detect asserts, which a three-wire console cable never does. That hangs *before* any
deadline of ours has started. The tool warns if it is handed one.

## Why there is no dependency

Configuring a UART is one `tcsetattr`, and the two ordinary ways to reach it from Rust are the
`serialport` crate or `libc` and the struct by hand. Both are new dependencies in the shipping
graph, and §46 (thin primitives or whole subsystems; we write everything in between) makes taking
one a decision rather than a convenience. A lane does not take that decision. A serial
configuration call is squarely the "in between" that section refuses: not a thin architectural
primitive, and nothing like a whole subsystem. `stty(1)` makes the
same call, is in every base system, and costs a process spawn per session. **Milestone 324's
writing mode did not change this**, which is worth saying because it looks like it should have: the
port is already opened read-write and one byte goes out through an ordinary `write`, with no flow
control to negotiate and no baud to change. If this ever needs to write *with* flow control, or at
a non-standard baud, the trade changes and the dependency is worth proposing.

## Testing it with no board

The board was powered off for all of this and its power strip was unreachable, so nothing below
involved a booting machine.

- **The recogniser** runs against four fixtures under `crates/board_console/tests/fixtures/`, fed
  one byte at a time, which is the worst case a real UART delivers and the case that catches a
  recogniser depending on chunk boundaries. Two of them are in `captured/` and are raw bytes off
  the wire on 2026-09-01, control characters and all; two are in `synthetic/` and are cases
  nobody has yet seen at a bench. The directory split is the provenance, deliberately, because a
  claim in a README is a weaker record than a path.
- The deadline runs against sources that block forever, which is what a powered-off board
  looks like through a port whose read timeout did not take.
- **A real descriptor**: a FIFO standing in for a port covers `stty` failing (a warning, not
  fatal) and a source that speaks and then stops (caught as silence, exit 2).
- The port layer, which is the part that looks untestable and mostly is not. The argument list
  `stty` is given, the complaint it produces when it fails, the speed read back afterwards, the
  dial-in warning, and the choice between zero, one and several adapters are all pure functions
  with the IO lifted off them, so a host test asserts on the exact words a person meets at a bench.
  `open` itself is exercised against a temporary file holding the real capture, so the path from
  `port::open` to a recognised boot runs in a host test.
- The real adapter, with the board off, covers everything except the board: discovery finds
  `/dev/cu.usbmodem*`, the `stty` moves it to 115200 and it reverts on exit, and the deadline
  returns with zero bytes and exit 3.
- **The writing mode** (milestone 324) is tested against a `Vec<u8>` standing in for the port, so a
  host test asserts the exact bytes a board would have received and the log entries beside them.
  Eleven tests in `stop.rs` cover the decision (nothing is written until the board announces an
  armed reboot loop; one byte, once; the n-th draw and not the one before it), the invariant (the
  log names the byte in hex, and names it *before* the write, so a failed write reads correctly),
  the confirmation, a port that refuses the write, and the agreement between this mode's draw count
  and `--tally`'s. Two more in `watch.rs` run the same thing through the real loop. What none of
  them prove is the wire: no byte has reached a board, and `stop.rs`'s `BUGS` says so where a
  reader meets it.

The residue no host test reaches is one claim: that `tcsetattr` actually took. Nothing on a
host is a tty, so a test can prove the right arguments were sent and cannot prove the device
listened. That was checked by hand against the CH343 (9600 before, 115200 while held, reverting on
exit), and it is what `confirm_speed` gates at runtime, which is the better answer anyway: the
check runs on every real session rather than only when somebody runs the tests.

## EXAMPLES

A boot check, the ordinary case:

```
$ script/board-console --for 90s
--- /dev/cu.usbmodem5C7B0104661 at 115200 baud, logging to target/board-console-1756744000.log, up to 90s ---
U-Boot SPL 2021.10 (Feb 12 2023 - 20:24:34 +0800)
...
Starting kernel ...

nife on RISC-V (rv64, S-mode, Sv39)

board-console: reached kernel banner (1437 bytes in 6.2s)
board-console: banner: nife on RISC-V (rv64, S-mode, Sv39)
board-console: log at target/board-console-1756744000.log
$ echo $?
0
```

A hang, which is what a multicore defect looks like:

```
$ script/board-console --for 90s --quiet-after 10s
...
Starting kernel ...

board-console: went quiet after kernel handoff (491 bytes in 14.1s)
board-console: log at target/board-console-1756744100.log
$ echo $?
2
```

Nothing at all, with the triage ladder's first row said out loud:

```
board-console: time ran out after nothing recognisable (0 bytes in 90.0s)
board-console: not one byte arrived. Check TX/RX are crossed, that the board has power, that the
DIP switches are on QSPI, and that this is the cu.* device.
```

Re-reading a capture, which needs no board and no adapter:

```
$ script/board-console --replay target/board-console-1756744100.log
```

Watching a job-mix sweep, which is the same recogniser under QEMU and is where it was proved
(milestone 324 part 2, 2026-09-19, aarch64 `virt` with four cores). The three endings, all real
runs:

```
$ cargo xtask job-mix --arch aarch64 --smp 4
...
job-mix: tasks=32 jobs=4096 repeats=21 ticks_min=228108401 ticks_median=233958411 \
  ticks_max=249234771 jpm_median=65652
job-mix: done

job-mix: reached job-mix sweep complete (15924 bytes in 164.9s)
job-mix: last point tasks=32 jobs=4096 repeats=21 ticks_min=228108401 \
  ticks_median=233958411 ticks_max=249234771 jpm_median=65652
$ echo $?
0

$ cargo xtask job-mix --arch aarch64 --smp 4 --quiet-after 1s
job-mix: went quiet after job-mix sweep running (8545 bytes in 13.7s)
job-mix: last point tasks=2 jobs=256 repeats=21 ticks_min=18753659 \
  ticks_median=20392064 ticks_max=24330391 jpm_median=47077
$ echo $?
2

$ cargo xtask job-mix --arch aarch64 --smp 4 --for 30s
job-mix: time ran out after job-mix sweep running (9437 bytes in 30.0s)
job-mix: last point tasks=2 jobs=256 repeats=21 ticks_min=19380785 \
  ticks_median=23557551 ticks_max=51605248 jpm_median=40751
$ echo $?
3
```

The point and summary lines are one line each on a terminal; they are folded here with a trailing
backslash, which is the only edit made to them.

The second is a wedge manufactured with a one-second quiet window rather than a real one: no sweep
has wedged on a board, and none has been watched on one. The jobs-per-minute figures are a draw and
not a result; `notes/job-mix.md` has why.

These three were re-run on 2026-09-19 after milestone 168 landed, and the numbers moved for a
reason worth knowing. The earlier transcripts here showed a whole sweep in 27.9 seconds and a
`--for 12s` cut at the third point. The sweep now takes 164.9 seconds, because each point is
twenty-one repeats rather than three and the mix has seven job kinds rather than five, so the same
wall-clock windows cut it far earlier. The commands did not change; the workload underneath them
did.

At a board the same question is `script/board-console --until sweep-done --for 30m`, and it returns
the same statuses because it is the same code.

Ending a rebooting soak from a script, which is the one thing this tool writes for. No run of
this against a board exists yet; what follows is what the code produces, from the transcripts the
host tests feed it:

```
$ script/board-console --stop
--- /dev/cu.usbserial-A28FR8LZ at 115200 baud, logging to target/board-console-1758290000.log, up to 270s ---
--- stopping after 1 armed draw(s); one byte will be sent to the board and printed into the log ---
...
soak-test: started 4 groups of one responder, 3 callers, 1 grinder and 1 tick waiter ...
soak-test-reboot: THIS BUILD REBOOTS THE BOARD. It soaks for 120s, then asks the firmware ...

board-console: sending the soak escape to the board now: 1 byte, 0x0d. Draw 1 of 1 armed its
reboot at +24.8s and this is the sample the series was asked for.
board-console: sent 1 byte, 0x0d. Waiting up to 15s for the kernel to say it found it.
soak-test-reboot: DISARMED at t=5s: a byte arrived on this console. ...

board-console: the board acknowledged the escape and disarmed its reboot loop (2914 bytes in 30.1s)
board-console: log at target/board-console-1758290000.log
$ echo $?
0
```

A series with exactly fifty samples in it, which is what milestone 249's distribution wants:

```
$ script/board-console --stop-after 50 --log target/radon-lottery-$(date +%s).log
...
$ script/board-console --tally target/radon-lottery-....log     # reports fifty draws
```

And the same command against a board that is not running a rebooting soak, which is every case the
ruling did not cover arriving as one answer:

```
board-console: time ran out after soak running (184213 bytes in 270.0s)
board-console: no byte was sent: no armed reboot loop announced itself before the session ended
$ echo $?
3
```

## BUGS

The markers are checked against one board, on one day, in four states. That is much better than
where this started, which was documentation only, and it is not the same as proven. Not covered:
every other way this board can behave, a different vendor firmware build with differently worded
banners, the two synthetic cases nobody has yet seen at a bench, and argon, which has never printed
a byte to this tool. x86_64 is no longer in that list: xenon was captured on 2026-09-17
(`bench/xenon-2026-09-17/first-light-095500.log`) and the markers that matched were the portable
ones, which is a second board's worth of evidence for exactly the half of `Stage` that claims to be
portable and none at all for the half that is radon's firmware.

There is no real sample of a hang, which is the outcome this tool exists for, since a hang is
what a multicore defect looks like from the far end of a serial cable. The synthetic fixture is a
real capture truncated before the tour. If risk 5 ever produces a genuine one at a bench, capture
it; it would be worth more than every other fixture here. A marker whose real text differs by a word is missed, and a
missed marker reports a healthy board as having got less far than it did.

**A missed marker fails toward pessimism; a matched one does not.** Matching is `contains`
anywhere in a line, so a console that echoed `Starting kernel ...` back would be read as having
handed over. Nothing guards against that. **This bullet used to rest on the tool never writing, and
since milestone 324 landed it rests on what the writing mode sends instead**: `--stop` sends one
byte, which cannot spell a marker, and the mode logs what it sent, in hex, so a reader can rule it
out by hand. A host test asserts the same of the mode's own log annotations, which are the other
text this tool adds to a capture. A future mode that typed whole lines would put this hazard back,
which is a reason to keep the write surface at named commands rather than a keyboard.

**It does not recognise an OpenSBI trap dump**, which the triage ladder lists as a real and
specific outcome (the kernel started the S7 and vendor firmware died in its own handler). The
dump's exact first line is written down nowhere in this tree, and guessing it would put text in a
recogniser that no machine has ever printed. Such a boot is caught as silence or as time running
out, with the dump sitting in the log, which is one step worse than being named.

**A captured log is not a test result.** The roadmap block says this first and it is worth
repeating where the tool is: this reports how far a boot got, and deciding a milestone passed on
the strength of a vendor's boot message is a line nobody has agreed to cross. `Reached` is named
for what was observed rather than for a verdict.

It drives no firmware, and the reason that was fatal on this board is gone. The captured
failure was the proof: the extlinux path from power-on ended at `### ERROR ###`, so reaching nife
meant interrupting autoboot and typing the four `StarFive #` commands. Milestone 218 closed that
on 2026-09-16, confirmed by a boot whose countdown expired with nobody typing
(`bench/radon-2026-09-16/tour-083200.log`), so a reader is now enough to get radon from power-on to
the kernel, and nothing here types at U-Boot. What is not gone is the next bullet, which arrived
from a different direction and is what the writing mode answers.

Stopping a rebooting soak no longer needs a person, and no byte of it has reached a board yet
(milestones 249 and 324). `--features reboot_soak_test` makes a board cold-reboot every two
minutes, and its escape is a byte on the console UART: any byte, checked every five seconds. Until
milestone 324 this tool held the port and could not send one, so the escape was reached by a
person typing, either into this session's terminal or by detaching it first, and detaching a
console is not free (notes/soak.md records a 6% rate change from doing it mid-run). `--stop` and
`--stop-after <n>` are now that escape from a script. **What is not yet proven is the wire.**
Milestone 324's lane had no board attached to it, so the decision to send is tested against
captures and the bytes against a buffer standing in for the port, and nothing has tested that a
write to the descriptor reaches the UART or that the kernel's poll finds it. The first real
`--stop` is that experiment and its `DISARMED` line is the result.

The `--stop` gate costs a draw when a session attaches mid-run. The byte goes out only on the
board's arming announcement, which is printed once per boot, so a `--stop` against a board already
two minutes into a draw waits for that draw's reboot before it can act. Attaching at power-on has
no such gap. The gate is what makes every other state safe (see "Writing to a board" above), and
this is its price, stated rather than hidden.

An unconfirmed send is three faults wearing one report. The byte may not have left the host,
the board's receive path may be dead (`kernel/src/soak.rs`'s own `BUGS` says nothing in the kernel
can tell), or the kernel may be wedged in a way the beat has not yet shown. The tool says the byte
went out and was not acknowledged, and cannot say which. The surrounding beats in the log separate
them by hand: beats still arriving with no `DISARMED` points at the receive path.

**It does not touch power.** The board's Kasa strip was not reachable from either machine when
this was written, and the roadmap block declines to decide whether this tool should ever drive it.
A tool that power-cycles is a different and more dangerous object than one that reads.

The board profile is real now, and exactly one board has ever been checked against one. calef
ruled on 2026-09-19 that this is one tool whose profile is the firmware prologue and nothing else,
and milestone 324 part 3 built it: `crates/board_console/src/board.rs` declares radon's four rungs,
its two refusals and its relocation note as data, `--board` chooses, and `Stage::Firmware` is what a
boot reaches while climbing one. What that does **not** do is add evidence. radon's rungs are
asserted against bytes off the wire on 2026-09-01; xenon's profile says it has no prologue, which
rests on one capture on one day (`bench/xenon-2026-09-17/first-light-095500.log`) and would miss a
xenon that fell over inside its own firmware; argon has no profile on purpose, because a Jetson boot
chain read out of vendor documentation and never watched on a wire is the assertion-shaped-as-
measurement failure this tree keeps catching. Its prologue stays unwritten until a board prints
something.

Nothing gates a profile against the board it claims to describe. The same gap `crates/boot_ladder`
records against the kernel, one level out, and the same mechanism: review, plus a capture in
`tests/fixtures/captured/` for every rung anybody asserts on. A rung declared with a marker no
machine prints fails in the direction that looks like success.

A sweep has no heartbeat, so its wedge timer is a guess with headroom. The sixty-second default
on `script/job-mix` is twenty times the longest subrun measured on one host under TCG. A board
slower than twenty-to-one is called wedged when it is merely slow, and `--quiet-after 0` gives up
the detection to avoid that. Giving the sweep a real wall-clock heartbeat, the way
`kernel/src/soak.rs` has one, is a kernel change and was not made here.

No sweep has been watched on a board. The recogniser was proved against QEMU on 2026-09-19: a
finished sweep exits 0, one wedged by a one-second quiet window exits 2, one cut off by a
twelve-second cap exits 3, and a refusal is a host test built from `job_mix::FAILED` rather than
from a capture, because no kernel here has refused one. radon has never run a sweep this tool
watched; that is milestone 168's own HARDWARE gate and not something part 2 could close.

The settle window is two seconds, and two seconds is a guess. It is long enough for the
captured measured-boot refusal, which follows the banner within a tour's worth of printing, and
there is no principle behind it beyond that. A failure that a board announces three seconds after
the awaited stage would still be reported as a success. `--until none` has no early exit at all and
sees everything up to the cap, which is the answer when being right matters more than being quick.

No test opens a real serial device. A pseudo-terminal pair would be the honest stand-in, and
making one needs `posix_openpt` and its `ioctl`s, which is `libc`, which is §46's decision and not
a lane's. So the port layer is tested as pure logic plus a regular file, and the one claim that
leaves unproven is named in the testing section above. If this crate ever takes a serial
dependency, a pty test should arrive with it.

Neither the tool nor this note knows whether a session was interrupted. Ctrl-C kills the
process, the descriptor closes, and the log holds every byte that had arrived, but no summary line
is written and no exit status distinguishes it from a crash. The log is the record; the summary is
a convenience.
