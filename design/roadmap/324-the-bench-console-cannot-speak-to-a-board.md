---
status: BUILT
raised: 2026-09-18
built: 2026-09-19
---
# 324. The bench console cannot speak to any of the three boards

Minted 2026-09-18 by calef, promoting a cluster rather than its
members: four proposals from three lanes, all about the same tool. *(Number provisional until the
merge queue lands it.)* **Parts 1 and 4 landed first** (`milestone/324-board-console-write-mode`),
**parts 2 and 3 the same day** (`milestone/324-sweep-recogniser-and-board-profile`). All four are
in. What no part of this milestone could close is stated in each part and again below: **no byte of
the writing mode has reached a board, and no job-mix sweep has been watched on one.**

It was `DECISION` on parts 3 and 4 when this block was minted; calef answered both on 2026-09-19, in
the order this block asked for, and every part was taken the same day.

Part 4, decided: yes, it writes, under a mode that names its purpose. The invariant in
`script/board-console`'s header changes from *"it reads and never writes to the board"* to it
writes only what a named mode sends, and every byte it sends is printed into the log. That second
clause is not decoration: the log is the artifact a bench run is judged from, and a byte the board
received that the capture does not show makes the capture a lie about the run.

The reason for *narrow* rather than a general typing mode is an incident rather than a preference.
Milestone 249's lane sent the escape byte by detaching the console, and its block records what that
cost: *"Sending it at the wrong moment also stopped U-Boot's autoboot countdown and cost a power
cycle."* The hazard is not writing; it is an open keyboard beside a countdown a stray byte consumes.
So the shape is 249's own proposal, `--stop` and `--stop-after <n>`, each a command with a purpose.

The reason this block offered for answering "no" had expired, and that is worth recording. Part
4 was filed on the possibility that *"Milestone 218 may remove the need by fixing autoboot"*. 218 is
BUILT and was confirmed on a real boot on 2026-09-16, with the countdown expiring and nobody typing
(`bench/radon-2026-09-16/tour-083200.log`). It removed the four `StarFive #` commands and did not
touch 249's escape, which had created a second reason to write thirteen days earlier.

Part 3, decided: one tool with a board profile, and the profile is the firmware prologue only.
A tool each would fork 3,545 lines whose portable majority is already proven shared: `script/soak-test
--arch <a>` runs on all three architectures and its own header says the judging is literally
`script/board-console`'s code, exit statuses included.

And the split the profile needs is already in the source, undocumented as a design. `Stage` in
`crates/board_console/src/progress.rs` is two things in one enum. `Spl`, `OpenSbi`, `UBoot` and
`Handoff` are radon's firmware chain. `Banner`, `Machine`, `SelfTest`, `Tour` and `Soak` are the
kernel's own ladder, and that half's doc comments already record it as reachable on all three
architectures since milestone 268. A board profile is a firmware prologue and nothing else; the rest
is shared and always was.

Argon is deferred rather than profiled, and the scope note is the point. It has never booted
nife and sits behind milestone 127, which is NOT-STARTED. A profile written for it today would be a
Jetson boot chain read out of vendor documentation and never watched on a wire, which is the
assertion-shaped-as-measurement failure this tree keeps catching. Its prologue stays unwritten until
a board prints something.

Why a cluster. `script/board-console` and `crates/board_console` are one tool, and four lanes
filed four limitations against it from four directions: milestone 247's sweep of 216's block (twice),
milestone 249's boot-lottery lane, and milestone 168's job-mix lane. Each is small. Together they are
one statement: the bench console is a reader, for one board, that cannot tell every finished run
from a wedged one, and every one of the three boards is a machine somebody currently has to walk
to.

That matters beyond tidiness because `design/fatal-risks/README.md`'s remaining experiments are
hardware-gated in the sense that means *a person has to sit at the board*: milestones 16, 168 and 201
all wait on hands rather than on hardware. A console that can drive a board is the difference between
those being a bench session and being a job.

## The four parts

1. BUILT 2026-09-19. It cannot stop a reboot loop. Milestone 249's self-rebooting soak is
   stopped by pressing a key, so nine draws is a whole evening at the board. A mode on a script that
   already holds the port. Found by milestone 249's lane, which built the escape this would
   automate. Part 4's answer makes this the smallest part rather than the blocked one: `--stop`
   sends the byte and logs that it sent it, and `--stop-after <n>` ends a series with exactly the
   sample it was asked for, which is 249's own wording for what a writing mode buys. See *What
   landed* below, and read its second paragraph before extending this: the obvious trigger is the
   wrong one.
2. BUILT 2026-09-19. It cannot tell a finished job-mix sweep from a wedged one.
   `crates/board_console` recognised the boot sequence and, since milestone 219, a soak's stages and
   heartbeat; the job-mix sweep had no recognizer, so its exit status could not distinguish the two.
   Found by milestone 168's lane. Two rungs close it, `sweep` and `sweep-done`; see *What landed,
   parts 2 and 3* below.
3. **BUILT 2026-09-19. It serves two boards of three, not one, and the third is not ready to be served.** The part was
   filed on 2026-09-03 saying *"it serves radon only"*, and that was true then. On **2026-09-17** it
   captured xenon's boot over `/dev/cu.usbserial-A28FR8LZ`; the capture is in the tree at
   `bench/xenon-2026-09-17/first-light-095500.log` and a `--replay` of it on 2026-09-19 reports the
   banner, the machine line, the five-of-five self-test and the measured-boot refusal with the right
   diagnosis. Nothing was added for xenon, because xenon boots through PVH straight into our banner
   and the portable half of `Stage` is its whole boot. So what is left of this part is radon's
   firmware prologue being hard-coded rather than declared, which is refactoring against a working
   second case rather than a port. Found by milestone 247's sweep, from milestone 216's block. The
   prologue is now `crates/board_console/src/board.rs`; see below.
4. BUILT 2026-09-19. It cannot type at a board, and it should be able to. Decided above: a
   named mode, its bytes logged, not a keyboard. What a lane builds is `--stop`, `--stop-after <n>`,
   and the rewritten invariant in `script/board-console`'s header. `notes/board-console.md` already
   carried the ruling beside each limitation it supersedes, and said plainly that until this lane
   landed the tool still behaved as the note described; those sentences are now true rather than
   pending. Found by milestone 247's sweep, from milestone 216's block.

## What landed, 2026-09-19: parts 1 and 4

`--stop` and `--stop-after <n>` on `script/board-console`, the rewritten invariant in its header,
and the same invariant in `crates/board_console/src/lib.rs` and `notes/board-console.md`. The engine
is a new module, `crates/board_console/src/stop.rs`, whose header carries the argument; the reading
loop was not duplicated, it took one parameter (`watch::watch_with`).

The invariant now reads: *it writes only what a named mode sends, and every byte it sends is
printed into the log.* That is a mechanism rather than a promise. `Escape::observe_line` is the only
thing in the crate that writes to a port, and it writes the announcement into the log and flushes it
**before** the byte goes out, so the two failure orders are the readable one: a log that says a byte
is going and then says the write failed is honest, and there is no path that puts a byte on a wire
the log does not name. The byte is rendered as `0x0d` and never written raw, so a capture stays
greppable and a replay of it cannot be fooled; a host test asserts that no line this mode writes
reads as a boot marker.

The finding worth carrying out of this lane is that the obvious trigger is the wrong one, and
silently so. The natural place to send the escape is `soak-test: started`, the line that reaches
`Stage::Soak`. That would lose. `kernel/src/soak.rs` prints `START_MARKER`, then four more lines,
and *then* calls `arm_reboot`, whose first statement is `console::discard_rx`: about a kilobyte
between the two, roughly ninety milliseconds at 115200 baud, and a host that reads a line and
answers it is comfortably inside that. The byte would be thrown away by the kernel's own drain, on
almost every run, and the failure would look exactly like a board whose receive path is miswired.

So the trigger is the **arming announcement** (`THIS BUILD REBOOTS THE BOARD`), which `arm_reboot`
prints after the drain, and nothing else in a session can cause a byte to go out. That one gate
answers every state the ruling did not cover, in the same way and with the same answer, nothing is
sent: a board powered off, a board at a U-Boot prompt, a board at a `swish` prompt, a kernel with
no soak in it, a soak with no reboot loop. It also closes the hazard the ruling is actually about
without relying on the byte's value, because a kernel that has printed that banner has been running
for the length of a boot tour and U-Boot's countdown is long over. The price is one sentence and it
is recorded in three places: a session attaching to a board already mid-draw waits out that draw,
up to two minutes, before a banner arrives.

A `--stop` is also the first thing in this tree that can verify the soak escape end to end.
`kernel/src/soak.rs`'s own `BUGS` says the escape is a poll of one bit and that *"nothing in this
kernel can prove otherwise, because a UART cannot receive a byte it sends"*, and names a procedure
as the substitute: press a key on the first boot and confirm `DISARMED` before walking away. A host
holding the far end is not under that limit. `--stop` sends a byte it did not print and waits
fifteen seconds (three of the kernel's five-second polls) for the board's own `DISARMED` line, so
that procedure becomes a command with an exit status, which is rung four moving up to rung two.
notes/soak.md's step 4 says so where a reader meets it.

No exit status was added. Confirmed is `0`; sent-and-unacknowledged and never-sent are both `3`,
the time running out with the requested thing unreached, which is what `3` already meant. A failure
the board announced (`1`) and a hang (`2`) still win, because those are facts about the board where
an unconfirmed stop is a fact about this tool.

**What could not be tested, stated plainly: no byte has reached a board.** This lane had no board
attached to it and neither radon nor xenon was reachable, so nothing here was run on hardware and
nothing in this block should be read as saying otherwise. What is tested is the decision and the
bytes: eleven host tests in `stop.rs` drive the state machine against transcripts with a `Vec<u8>`
standing in for the port, so a test asserts the exact bytes a board would have received, and two
more in `watch.rs` run the same thing through the real loop. What that leaves unproven is that a
write to the descriptor reaches the UART and that the kernel's poll finds it. The first real
`--stop` is the experiment and its `DISARMED` line is the result; it is the same shape as the gap
milestone 249 already carries, one milestone on.

Two things this lane chose that calef may want to overrule, both cheap to change and both
recorded where a reader meets them rather than only here:

- The byte is a carriage return. The kernel takes any byte, so this is a choice inside a fixed
  contract rather than a wire value two programs agree on. Carriage return because the kernel's own
  instruction is *"press any key"* and Enter is what a person presses when told that, so the tool
  runs the same experiment the documented manual procedure runs. `NUL` was the other candidate and
  the refusal is recorded beside the constant.
- `--for` defaults to 150 seconds per requested draw plus two minutes when a stop mode is given
  and `--for` is not, derived from `kernel/src/soak.rs`'s two-minute draw plus a boot. It is a
  default and not an agreement (getting it wrong costs a re-run, never a wrong answer), which is why
  the number is duplicated rather than hoisted into a shared crate.

**One piece of identified work with no home yet, recorded in `stop.rs`'s `BUGS` where the next
reader meets it**: the soak's console markers are string literals in three places (the kernel, the
recognizer, and now this module), agreeing by a reader having checked rather than by the compiler.
`crates/boot_ladder` exists for exactly that and holds the boot tour's markers as shared constants;
`START_MARKER`, `REBOOT_MARKER` and the arming and disarming lines were never hoisted into it. That
is a change to the kernel and is outside parts 1 and 4.

Parts 1 and 4 are close enough to collide, and that is an argument for the cluster rather than
against it: part 1 wants to send one keystroke and part 4 asks whether sending anything is allowed.
Answering 4 first makes 1 either trivial or refused, and nothing in four separate proposals said so.
It was answered first, on 2026-09-19, and it made 1 trivial, which is the cluster earning its
keep in the one way a cluster can.

The title changed with this ruling, and the file was renamed with it. It read *"reads one board
of three and cannot speak to any of them"* when minted on 2026-09-18, and the first half of that was
already false: xenon had been captured the day before. Only the generated index cited the path, so
the rename cost nothing, and a block whose own H1 states a falsehood is the thing this tree objects
to hardest.

Two of the four parts had decayed by the time they were read, the same as milestone 323's two.
Part 4's reason for a possible "no" was spent by milestone 218 on 2026-09-16, and part 3's premise
was falsified by xenon's own capture on 2026-09-17, both after filing and both before promotion on
2026-09-18. See 323's *What the two decisions cost*, which argues that promotion is where a stale
premise is cheapest to catch; four instances across two clusters is the evidence for it.

## What landed, 2026-09-19: parts 2 and 3

Two changes to one tool, plus the hoist that both of them turned out to need.

Part 2, the sweep recognizer. `crates/board_console::progress` gained two rungs, `Stage::Sweep`
(`job_mix::STARTED`) and `Stage::SweepDone` (`job_mix::DONE`), one failure (`Failure::SweepFailed`,
carrying the reason the kernel gave) and the sweep's numbers (`SweepPoint`, `SweepSubrun`). So
`script/board-console --until sweep-done` now answers the question the part named, and it needed no
new exit status: `0` finished, `1` refused or otherwise announced, `2` spoke and stopped, `3` ran
out with points still to print.

`cargo xtask job-mix` now judges with that recognizer instead of its own loop, which is the move
`soak_test` already made and the reason milestone 219's block gives: two readers drift the first
time either changes. What stood there was `starts_with("job-mix")` with no timeout at all, so a
wedged sweep hung the command forever and a bench script could not tell it from a finished one. It
now tees the whole boot to a log under `target/` that replays through `script/board-console
--replay`, and returns the same five statuses.

All four outcomes were exercised on a machine rather than argued, which is what makes the
discrimination a fact: `cargo xtask job-mix` exits **0** on a complete sweep, **2** with
`--quiet-after 1s`, **3** with `--for 30s`, and the refusal path is a host test built from
`job_mix::FAILED` itself because no kernel here has refused one. All four were re-run after
milestone 168 was merged in; the windows moved because the sweep now takes 165 seconds rather than
28, and the statuses did not.

Part 3, the prologue as data. `crates/board_console/src/board.rs` declares a `Profile`: ordered
`Rung`s, the firmware's own `Refusal`s, and the relocation discriminator. `Stage`'s four firmware
variants became one, `Stage::Firmware(&'static Rung)`, ordered below `Banner` because firmware runs
before the kernel whatever the board. `Failure::BadImageMagic` and `Failure::UBootRefused` became
one `Failure::FirmwareRefused`, because both were U-Boot's words rather than ours and a board with
different firmware refuses in its own. `--board radon|xenon` selects; radon is the default, so no
existing behavior moved.

The test that says the split is real reads one capture twice. radon's own 2026-09-01 boot, read
through radon's profile, climbs SPL, OpenSBI, U-Boot and the handoff on its way to the tour. The
same bytes read through xenon's profile reach **the same tour** and report no firmware rung, because
none of those lines are xenon's to claim. A second test replays
`bench/xenon-2026-09-17/first-light-095500.log` through the xenon profile and asserts the banner, the
machine line, the five-of-five verdict and the measured-boot refusal, with an empty prologue. That
pair is the ruling made mechanical.

`--until spl` against a board with no SPL is now refused rather than waited out, which is the
smallest visible benefit and the one an operator meets: it used to be a two-minute watch ending in
"the time ran out".

**One hoist, and it was not optional.** The sweep's console markers were three private `const`s in
`kernel/src/job_mix.rs` and four string literals in `xtask/src/main.rs`. Adding a recognizer
would have made a third copy, which is milestone 268's finding 3 exactly. They are now
`crates/job_mix`'s (`STARTED`, `DONE`, `FAILED`, `POINT`, `SUBRUN`, `CENSUS`, and `KIND` once
milestone 168's per-kind line was merged in), beside the workload
definition both halves of the instrument already read, on `crates/boot_ladder`'s argument and with
its stable-head convention. `board_console` takes `job_mix` as a dependency for the same reason it
took `boot_ladder`: ours, in this workspace, `no_std`, no dependencies of its own, no `unsafe`.
**This is the kernel-side half of the limitation `stop.rs`'s `BUGS` recorded for the soak**, done
for the sweep because a recognizer could not be written without it; the soak's markers are still
literals in three places and that entry stands.

**What could not be tested, stated plainly: no sweep has been watched on a board.** This lane had no
board attached to it and neither radon nor xenon was reachable, so every claim here rests on QEMU
captures and host tests, the same stand-in `notes/board-console.md`'s testing section already uses.
radon running a sweep is milestone 168's own HARDWARE gate and part 2 could not close it. The board
profiles carry the same honesty one level down: radon's rungs are asserted against bytes off the
wire, xenon's *empty* prologue rests on one capture on one day, and argon has no profile at all.

One thing this lane chose that calef may want to overrule, recorded where a reader meets it
rather than only here: a sweep has no wall-clock heartbeat, so its quiet timer cannot be three
missed beats the way a soak's is. It is sized against the slowest subrun instead: 4.0 seconds
measured (249,234,771 ticks on a 62.5 MHz counter) in the capture at
`crates/board_console/tests/fixtures/captured/qemu-2026-09-19-aarch64-job-mix-medians.log`, and
`script/job-mix` defaults `--quiet-after` to sixty seconds, fifteen times that. A board outside that
margin reads as wedged when it is merely slow; `--quiet-after 0` is the escape and it costs the
detection. The margin was twenty to one when this was written and milestone 168 spent a quarter of
it the same day, by taking twenty-one repeats of a seven-kind mix instead of three of a five-kind
one, which is an argument for the heartbeat rather than for a larger default. The alternative is a heartbeat in `kernel/src/job_mix.rs`, which
is a kernel change and is recorded below rather than taken.

## The recognizer was broken by another session before it was merged, 2026-09-19

**Worth the section because the mechanism is the finding, not the bug.** While parts 2 and 3 were
being built, another session's lane finished milestone 168 and landed on `main`. That lane changed
what `kernel/src/job_mix.rs` prints for a sweep point: `ticks=<t> jpm=<r>`, the best of three,
became `repeats=21 ticks_min= ticks_median= ticks_max= jpm_median=`, the median of 21 with its two
ends. The head, `job-mix: tasks=`, did not move.

So the hoist this lane made did its job and the recognizer still broke. Every marker matched;
`SweepPoint`'s parse read **nothing**, because it looked for `ticks=` and `jpm=` and the line no
longer carried either. `ticks_min=` does not start with `ticks=`.

Both branches were green, and that is the part to keep. The parser and the committed fixture had
been made from the same pre-168 kernel, so they agreed with each other and neither agreed with the
kernel. No gate compares a recognizer against a kernel; nothing could have. It was found by reading
the merged source, which is rung zero of AGENTS.md's ladder.

What the fix changed. `SweepPoint` carries the seven fields the line now prints (`tasks`,
`jobs`, `repeats`, `ticks_min`, `ticks_median`, `ticks_max`, `jpm_median`), spelled exactly as the
wire spells them so the struct can be diffed against a line by eye. The fixture was re-captured from
a current kernel and the old one deleted. `crates/job_mix`'s `KIND` joined the six markers, because
168's lane added a seventh printed line (`job-mix-kind:`) in the style the hoist had just retired.

The limitation this leaves is recorded rather than fixed, in
`crates/board_console/src/progress.rs`'s `BUGS` where the next reader of the parser meets it: the
markers are shared through `crates/job_mix`, **the field names inside the line are not**. A kernel
that renames or adds a field still prints a line this recognizer matches and still parses to
nothing, silently. Sharing the field names the way the heads are shared is the fix and is not taken
here; it wants a decision about what shape that sharing takes, which is below.

## Follow-on

- **Done.** Part 2, the job-mix sweep recognizer, and part 3, the board profile, both on
  `milestone/324-sweep-recogniser-and-board-profile` on 2026-09-19. See *What landed, parts 2 and 3*
  above, and the section above that for the defect the merge with milestone 168 created and this
  branch fixed.
- **Recorded.** *Milestone 439's block quotes a figure this lane has since re-measured*, here
  because a developer does not edit another milestone's block and the correction has to live
  somewhere a reader will meet it. 439's block and its index row quote *2.6 seconds
  (163,224,570 ticks)* for the slowest subrun, which is this lane's own figure from the capture that
  milestone 168 invalidated. Re-measured on the merged tree it is 4.0 seconds (249,234,771 ticks
  on a 62.5 MHz counter), and the ratio against the 60-second default is fifteen to one rather
  than twenty. Corrected everywhere this lane owns; 439's block is another milestone's and a
  developer does not edit one. The correction strengthens 439's own argument rather than weakening
  it: the number moved by half again in a day, without anybody touching the watcher.
- **Recorded.** *A marker is shared and the fields inside the line are not*, in
  `crates/board_console/src/progress.rs`'s `BUGS` and in `crates/job_mix`'s marker header. This is
  the second item here a reader may want minted as a milestone. The shape is not obvious and that is
  why it is not built: a shared `&str` per field would make the kernel's `println!` a format string
  assembled from constants, which is less readable at the place it matters most; a shared parser in
  `crates/job_mix` that both the kernel's printer and the recognizer are written against is the
  stronger version and is a bigger change than this lane's remit. The cheap partial measure is
  already taken (the struct's fields are spelled as the wire spells them, so a diff by eye works),
  and the fixture being a real capture means a re-capture catches it the moment somebody re-captures.
- **Recorded.** *The job-mix sweep has no wall-clock heartbeat, so its wedge timer is a guess with
  headroom*, in the `BUGS` of `script/job-mix` and of `notes/board-console.md`, and beside
  `Stage::Sweep` in `crates/board_console/src/progress.rs`. This is the one item here that a
  reader may want minted as a milestone rather than left a limitation, and the lane that found it
  says so plainly rather than claiming a number that is the integrator's.
  `kernel/src/soak.rs` prints every five seconds whatever the workload is doing, which is what lets
  a watcher call a hang after three missed beats and be right about it. `kernel/src/job_mix.rs`
  prints only when a subrun ends, so the sweep's wedge timer is twenty times the slowest subrun
  anybody has measured and a slow board reads as wedged. It is a kernel change (the supervisor
  would have to print from a timer rather than between subruns) and it is outside parts 2 and 3.
  Recorded in `script/job-mix`'s and `notes/board-console.md`'s `BUGS` as well, where a reader meets
  the number that would stop being a guess.
- **Recorded.** *No job-mix sweep has been watched on a board*, in the `BUGS` of `script/job-mix`,
  `crates/board_console/src/lib.rs`, `notes/board-console.md` and
  `crates/board_console/tests/fixtures/README.md`. radon running one is milestone 168's own
  HARDWARE gate; the first bench sweep closes it and its exit status is the result.
- **Recorded.** *Only radon's profile has been checked against a machine, xenon's emptiness rests on
  one capture, and argon has none*, in the `BUGS` of `crates/board_console/src/board.rs` and of
  `notes/board-console.md`. argon's prologue stays unwritten until a board prints something, which
  is this block's own ruling rather than a new decision.
- **Recorded.** *Nothing gates a board profile against the board it claims to describe*, in
  `crates/board_console/src/board.rs`'s `BUGS`. The same gap `crates/boot_ladder` carries against
  the kernel, one level out, and the same mechanism: review, plus a capture for every rung anybody
  asserts on.
- **Recorded.** *The soak's console markers are string literals agreeing by a reader having checked,
  in three places now rather than two*, in the `BUGS` of `crates/board_console/src/stop.rs`.
  `crates/boot_ladder` is where the boot tour's markers were hoisted for exactly this reason, and
  the soak's never were. It is a kernel change and outside parts 1 and 4. The sweep's markers were
  in the same state and are no longer: parts 2 and 3's lane hoisted them into `crates/job_mix`
  because a recognizer could not be written without it, which leaves the soak's as the last set
  still agreeing by a reader having checked.
- **Recorded.** *No byte of the writing mode has reached a board*, in the `BUGS` of
  `crates/board_console/src/stop.rs` and of notes/board-console.md, and in *What landed* above. The
  first bench run closes it, and its `DISARMED` line is the result rather than a green check.
- **Recorded.** *A `--stop` against a board already mid-draw waits that draw out*, in the same two
  `BUGS` sections. It is the price of the gate that makes every other state safe, and it is a
  property of the kernel's announcement being once per boot rather than something this tool can fix.
- **Refused.** A general typing mode, or `--send <text>`. It is the thing the ruling declined and
  the argument is an incident rather than a preference: an open keyboard beside U-Boot's autoboot
  countdown cost milestone 249's lane a power cycle. A second named command with its own purpose is
  the shape to add when one is wanted; a keyboard is not.
- **Refused.** Hoisting `--stop`'s confirmation window and the draw length into a crate shared with
  the kernel. They are a timeout and a default, not an agreement: getting either wrong costs a
  re-run with `--for`, never a wrong answer, and §46's line is that a shared crate is for what two
  binaries must agree on. The markers above are the thing that does qualify.

## Index row

`script/board-console` watches a board's serial port and could not type at it, hard-codes radon's
firmware chain, and cannot tell a finished job-mix sweep from a wedged one. Four lanes filed those
separately between 2026-09-03 and 2026-09-04; read together they say the bench console is a reader.
That is load-bearing rather than cosmetic, because the remaining fatal-risk experiments (milestones
16, 168, 201) are gated on a person sitting at a board rather than on hardware existing, and a
console that can drive one is what turns those from a bench session into a job. calef decided both
gated parts on 2026-09-19: it writes, under a named mode whose bytes are logged, and the three boards
share one tool whose board profile is the firmware prologue alone, with argon deferred behind
milestone 127 rather than guessed from vendor documentation. All four parts landed the same day.
`--stop` and `--stop-after <n>` end milestone 249's rebooting soak from a script, the byte goes out
only on the board's own arming announcement and never before it, and every byte sent is printed into
the log in hex. `--until sweep-done` tells a finished job-mix sweep from a wedged one, with no new
exit status and with `cargo xtask job-mix` judging through the same recognizer rather than a second
copy that had no timeout at all. And the firmware prologue is data: `--board radon|xenon`, with
radon's four rungs and two refusals declared in `crates/board_console/src/board.rs` and everything
from the kernel banner up shared, proved by reading one radon capture through both profiles and
getting the same tour with and without a prologue. What none of it proves is hardware: no
`--stop` has reached a board and no sweep has been watched on one.
