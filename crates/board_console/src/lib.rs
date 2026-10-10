//! **A serial console for a board this repository cannot see** (milestone 216).
//!
//! `script/console` opens a shell in QEMU. `script/board-image` builds the payload a VisionFive 2
//! boots and prints the `dd` commands for a card. Between those two there was nothing, so every
//! milestone gated on real hardware waited on a person at a terminal emulator, reading with their
//! eyes. This crate is the missing middle: **open the port, log every byte, recognise how far the
//! boot got, and stop on a deadline.**
//!
//! The three parts, in the order a session uses them:
//!
//! - [`port`] finds the adapter and puts it in raw mode at 115200 8N1.
//! - [`watch`] reads until the policy says stop, teeing every byte to a log file.
//! - [`progress`] decides, from the text alone, how far the boot got.
//!
//! And one part that reads a *screen* rather than a wire (milestone 243), because six of the
//! machines this project wants to boot have no serial port at all:
//!
//! - [`screen`] turns a screendump of nife's framebuffer console back into the text that was drawn
//!   into it, so that [`progress`] judges a monitor exactly as it judges a cable.
//!
//! And one part that is data rather than code (milestone 324 part 3):
//!
//! - [`board`] declares each board's **firmware prologue**, which is the only thing about this tool
//!   that is a board's rather than the kernel's. calef's ruling of 2026-09-19 is in its header.
//!
//! And one part that reads a capture after the fact rather than a board in front of it:
//!
//! - [`lottery`] takes a log of *many* boots and reports what the thread-placement lottery drew
//!   each time (milestone 249), which is the question a self-rebooting soak exists to answer and
//!   the one nothing that reads a single boot can be asked.
//! - [`exposure`] reads a soak capture into the exposure row `notes/multicore-defect-curve.md`
//!   keeps for milestone 201 (is multicore reliability converging), built for milestone 225 (run
//!   the soak on radon, argon and xenon), so a bench run's numbers are copied by a
//!   program rather than by whoever is still awake at the end of it.
//!
//! And one part that **writes**, which every other part of this crate does not (milestone 324):
//!
//! - [`stop`] sends the single byte that ends milestone 249's self-rebooting soak, after the board
//!   has announced an armed reboot loop and never before it, and prints what it sent into the log.
//!   Its header carries the whole of that argument.
//!
//! # This crate is not run under Miri
//!
//! `script/undefined-behavior-check` excludes it, the way it already excludes `xtask`, and for cost
//! rather than principle (milestone 238). Measured 2026-09-03: **3,307 seconds, 55 minutes, for the
//! lib tests alone** under the interpreter, against roughly four minutes for the whole rest of the
//! workspace. There is no `unsafe` and no dependency anywhere in here, so the rules Miri enforces
//! (aliasing, provenance, uninitialized reads) have nothing in this call graph to be broken by.
//! (Milestone 243 gave it one dependency, `bitmap_font`, which is ours, has none of its own, and
//! contains no `unsafe` either.)
//!
//! Ten of the tests could not run there in any case: five in [`port`] reach the host filesystem
//! and Miri's isolation refuses them, and five in [`watch`] are wall-clock driven, so under an
//! interpreter the timeouts expire mid-replay and report `Reached(Banner)` where a real run
//! reaches `Reached(Tour)`. That is Miri being slow, not this crate being wrong. See
//! notes/undefined-behavior.md and the exclusion list in `xtask`.
//!
//! # Examples
//!
//! Watch a board boot, with a two-minute cap:
//!
//! ```no_run
//! use std::fs::File;
//! use board_console::{port, watch::{self, Policy}};
//!
//! let path = port::choose(None)?;
//! let (device, complaint) = port::open(&path)?;
//! if let Some(complaint) = complaint {
//!     eprintln!("board-console: {complaint}");
//! }
//! let mut log = File::create("target/board-console.log")?;
//! let session = watch::watch(device, &mut log, &Policy::default(), true)?;
//! println!("{}", session.summary());
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! Replay a log that was already captured, which is how a session is re-read after the fact and
//! how the recogniser is exercised with no board present:
//!
//! ```no_run
//! use std::fs::File;
//! use board_console::watch::{self, Policy};
//!
//! let captured = File::open("target/board-console.log")?;
//! let mut nowhere = std::io::sink();
//! // `false`: end of file means the log is over, not that the board went quiet.
//! let session = watch::watch(captured, &mut nowhere, &Policy::default(), false)?;
//! println!("{}", session.summary());
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! # BUGS
//!
//! **The markers are checked against one board, on one day, in four states.** calef captured two
//! successful boots, an extlinux refusal and a measured-boot refusal on 2026-09-01; all four are
//! in `tests/fixtures/captured/` and all four are asserted on, which is what turned "quoted from
//! documentation" into "printed by a machine". What that does not cover is every other way this
//! board can behave and every other board: a different vendor firmware build could word its
//! banners differently, and the aarch64 and `x86_64` boards have not been looked at at all. A
//! marker whose real text differs by a word is missed, and a missed marker reports a healthy board
//! as having got less far than it did.
//!
//! **There is no real sample of a hang**, which is the outcome this tool exists for, since a hang
//! is what a multicore defect looks like from the far end of a serial cable. The synthetic fixture
//! is a real capture truncated before the tour. A genuine one, if risk 5 ever produces one, would
//! be worth more than every other fixture here.
//!
//! **The settle window is two seconds, and two seconds is a guess.** It is what stops a failure
//! printed just after the awaited stage being reported as a success, which the captured
//! measured-boot refusal is: it prints the banner and most of a tour before halting. A failure
//! announced three seconds later would still slip through. `--until none` has no early exit at all
//! and is the answer when being right matters more than being quick.
//!
//! **A missed marker fails toward pessimism, which is the safe direction, and a *matched* one does
//! not.** The recogniser matches substrings anywhere in a line, so a board that echoes the word
//! `Starting kernel ...` back at a U-Boot prompt would be read as having handed over. Nothing
//! guards against a console that repeats its input. **This used to rest on the crate never
//! writing; since milestone 324 it rests on what [`stop`] sends instead**: one byte, which cannot
//! spell a marker, logged in hex beside the line that provoked it so a reader can rule it out by
//! hand. A future mode that typed whole lines would put the hazard back, which is a reason to keep
//! the write surface at named commands rather than at a keyboard.
//!
//! **It does not recognise an OpenSBI trap dump**, which the failure-triage ladder lists as a real
//! and specific outcome (the kernel started the S7 and the vendor firmware died in its own
//! handler). The dump's exact first line is not written down anywhere in this tree, and guessing
//! it would put text in a recogniser that no machine has ever printed. Such a boot is caught as
//! silence or as time running out, with the dump in the log, which is one step worse than being
//! named.
//!
//! **A captured log is not a test result**, and the roadmap block says so first. This crate
//! reports how far a boot got; deciding that a milestone passed on the strength of a vendor's boot
//! message is a line nobody has yet agreed to cross, and `Outcome::Reached` is deliberately named
//! for what it observed rather than for a verdict.
//!
//! **It writes only what a named mode sends, and every byte it sends is printed into the log.**
//! That is the invariant since calef ruled on 2026-09-19 (milestone 324 part 4), and it replaces
//! *"it reads and never writes"*. Today there is exactly one named mode, [`stop`], and it sends
//! exactly one byte. **The narrowness is the decision rather than caution about it**: the hazard
//! is an open keyboard beside U-Boot's autoboot countdown, which milestone 249's lane hit by
//! detaching the console to send this same byte, at the cost of a power cycle.
//!
//! **It does not touch power.** The board is powered by a Kasa strip that was not reachable from
//! either machine when milestone 216 was written, and the roadmap block declines to decide whether
//! this tool should ever drive it. A tool that power-cycles is a different and more dangerous
//! object than one that reads.
//!
//! **No test opens a real serial device.** A pseudo-terminal pair would be the honest stand-in,
//! and making one needs `posix_openpt` and its `ioctl`s, which is `libc`, which is §46's decision
//! (thin primitives or whole subsystems; we write everything in between) and not a lane's. So the
//! port layer is tested as pure logic plus a regular file, and the single claim that leaves
//! unproven is that `tcsetattr` actually took. That was checked by hand against the CH343 and is
//! gated at runtime by the speed read-back, which runs on every real session rather than only when
//! somebody runs the tests.
//!
//! **The board profile is the firmware prologue, and exactly one board has been checked against
//! one** (milestone 324 part 3). calef ruled on 2026-09-19 that this is one tool with a profile
//! rather than a tool per board, and [`board`] is that ruling: radon's four firmware rungs, its two
//! refusals and its relocation note are declared as data, and everything from [`progress::Stage::Banner`]
//! up is shared by every board and always was. What the profile does not do is add evidence.
//! radon's rungs are asserted against bytes off the wire; xenon's *absence* of a prologue rests on
//! one capture on one day; argon has none on purpose, because a boot chain read out of vendor
//! documentation for a board that has never printed a byte is a guess wearing a measurement's
//! clothes. [`board`]'s own `BUGS` carries all of it.
//!
//! **A job-mix sweep can now be told from a wedged one, and none has been watched on a board**
//! (milestone 324 part 2). [`progress::Stage::Sweep`] and [`progress::Stage::SweepDone`] read
//! `crates/job_mix`'s own markers, so `script/job-mix` and `script/board-console` judge with one
//! recogniser and return the same five statuses. It was proved under QEMU on 2026-09-19 in all four
//! outcomes a machine can produce (finished, wedged, out of time, and a refusal built from the
//! shared constant), and radon has never run a sweep this tool watched: that is milestone 168's own
//! HARDWARE gate. The sweep has **no wall-clock heartbeat**, unlike a soak, so its quiet timer is
//! sized against the slowest subrun rather than against a deadline; [`progress::Stage::Sweep`] and
//! `script/job-mix`'s `BUGS` carry the measured number and what it costs a slow board.

//! Name: provisional, and ruled: calef ruled **`serial_console`** on 2026-09-13, pairing it with
//! `screen_console`, because a console is named for where its text comes out. The block stays
//! `provisional` until the rename is performed; coined by milestone 216 (nothing in this tree
//! can read a board)'s lane on 2026-09-01. The
//! axis argument, the consequence for `serial_driver`, and the refusals are in git history.

pub mod board;
pub mod exposure;
pub mod lottery;
pub mod port;
pub mod progress;
pub mod screen;
pub mod stop;
pub mod watch;

/// **Fixtures captured before milestone 297 renamed the console markers**, made readable by the
/// recogniser that reads the current ones.
///
/// `script/soak` became `script/soak-test` on 2026-09-14 (calef's ruling), and the markers moved
/// with it: `soak:` to `soak-test:`, `soak-census:` to `soak-test-census:`. Two files in
/// `tests/fixtures/captured/` are QEMU runs taken before that, and **they are left exactly as the
/// machine printed them**, because a capture is a record and this tree has already carried one
/// rewritten transcript for twelve days (`design/naming/rename-what-moves.md`, "a quotation never
/// moves").
///
/// So the transposition happens here, at test time, on a copy, and it is deliberately the narrowest
/// edit that makes the file parse: the marker prefix and nothing else. Every number, every field
/// and every other line is the machine's own.
///
/// **Why the recogniser itself does not simply match both spellings.** It did for `init/build`
/// (milestone 295) and had to: that marker survives only in a VisionFive 2 capture off real
/// silicon, which cannot be re-taken, so the live parser is the only thing that can ever read it.
/// These two are QEMU runs. What makes them unrepeatable is time rather than hardware, which is a
/// weaker claim, and it is a claim about *these files* rather than about anything a board will
/// print tomorrow. A permanent second spelling in the parser would be paying forever for a fact
/// that belongs to two files, and `captured/qemu-2026-09-14-riscv64-soak-test.log` is a real run in
/// the current vocabulary, so nothing here rests on hand-written text either.
///
/// Name: provisional (milestone 297 (`soak` becomes `soak-test`)): calef names public items, and
/// this one is `pub(crate)` and test-only, so it is the cheap end of that rule rather than an
/// exception to it.
#[cfg(test)]
pub(crate) fn respell_pre_297_markers(capture: &str) -> String {
    capture
        .replace("soak-census:", "soak-test-census:")
        .replace("soak: ", "soak-test: ")
}
