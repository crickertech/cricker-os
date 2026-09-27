//! **The rungs of the boot ladder, as the exact bytes that appear on a console** (milestone 268).
//!
//! A boot on this system climbs a fixed ladder: the console works, the machine has described
//! itself, the kernel has tested itself, the machine has been handed to userspace. Each rung
//! announces itself with a line, and each of those lines is **a contract between the program that
//! prints it and the programs that read it**: `crates/board_console` on a bench or in CI, and a
//! person reading a photograph of a monitor.
//!
//! Rule 7 of `AGENTS.md` ("anything two binaries must agree on is a crate, never a `#[path]`
//! module") is why this is a crate rather than a `const` in each place. Three binaries agree on
//! these strings today: the kernel prints [`BANNER`], [`MACHINE`] and [`SELF_TEST`]; `swish` prints
//! [`PROMPT`]; `board_console` matches all of them. Before this crate, the one marker that existed
//! was a string literal inside one architecture's arm of `kernel/src/main.rs` and a second copy of
//! it inside a recogniser, which is milestone 268's finding 3: the two could not disagree *loudly*,
//! only silently.
//!
//! # Why a prefix rather than a whole line
//!
//! Every constant here is the **stable head** of a line whose tail carries numbers. That split is
//! the design: the head is what a matcher keys on and never changes, the tail is what a human
//! reads and is free to improve. A contract on the whole line would make every improvement to the
//! diagnosis a breaking change, and this tree has learned twice that a marker nobody may touch is
//! a marker that stops describing the boot.
//!
//! Each carries its own trailing space or punctuation where it has one, so a match is a match on a
//! boundary rather than on the start of a longer word.
//!
//! # BUGS
//!
//! - **Nothing gates the kernel against this crate.** The kernel *uses* these constants, so its own
//!   output cannot drift from them; what nothing checks is that a new rung is added here rather
//!   than as a fresh literal somewhere, which is the mistake this crate exists to have already
//!   made once. Review is the mechanism, which is rung four of `AGENTS.md`'s ladder, and it is
//!   named here rather than implied.
//! - **[`PROMPT`] is `swish`'s banner, not its prompt.** The prompt itself is `$ `, two bytes, which
//!   is far too weak to key on in a console log that has just carried a kilobyte of hex. The banner
//!   is printed immediately before the first prompt by the same program, so a reader that finds it
//!   knows the shell is up; what it does not know is that the *prompt* came out, which is a
//!   distinction `cargo xtask swish-check` cares about and this crate cannot make.
//! - **The wordings are provisional** (milestone 268 (every architecture boots the same way)). They
//!   are contracts, so they are an architect's under `AGENTS.md`'s *move fast on what can be
//!   undone* tenet; a lane ships one and says so rather than waiting, which is what the milestone
//!   block instructed.
//!
//! Name: ratified 2026-09-14 (calef, working the unratified worklist), the same day milestone
//! 268's lane minted it. A noun for the thing it describes: the ladder a boot climbs.
//!
//! **The collision was weighed and accepted, and that is the part worth recording.** `AGENTS.md`
//! already spends *ladder* on its own most-cited metaphor: make the wrong state unrepresentable,
//! then a gate, then a record at the thing, then a note. So a reader who has internalised the
//! constitution meets this crate carrying a different sense of the word. It was kept because the
//! two are unambiguous in context (nothing here is about mechanism strength, and nothing there is
//! about a console), because *rungs of a boot* is legible on sight, and because the alternatives
//! each lose something this keeps: `boot_stages` and `boot_markers` drop the ordering that is the
//! whole claim, and `console_contract` describes the mechanism rather than the thing.
//!
//! **Coined rather than standard**, so it is not the protected class `virtio` and `elf` sit in and
//! it earned no shelter from being the field's word. It is this tree's metaphor, ratified as one.
//!
//! Refusals, because they are the valuable half:
//!
//! - **`boot_markers`** was refused for naming the *mechanism* (they are strings that get matched)
//!   rather than the thing. The tree's own habit is against it: `block_roster` is not
//!   `block_records`, `soak_page` is not `soak_layout`.
//! - **`boot_proto`** was refused on `soak_page`'s line, which this tree already draws: the
//!   `_proto` suffix names a request/reply vocabulary (`clock_protocol`, `cred_proto`,
//!   `supervision_protocol`), and nothing here is a protocol. It is one-way text on a console.
//! - **`boot_stages`** was refused as the plural of a word `board_console` already spends:
//!   `progress::Stage` is the recogniser's ordered enum, and a crate with almost that name holding
//!   almost that content is the collision a reader pays for.

#![no_std]

/// **The earliest line: `nife on <architecture> (...)`.**
///
/// Printed by `kernel_main` as the first thing after `console::init`, on every architecture since
/// milestone 268. aarch64 had no such line before it, so this rung was unreachable there for as
/// long as the recogniser has existed. It says the console works and it says nothing else, which
/// is exactly the claim wanted at first light on a board where the console is the thing in doubt.
///
/// Deliberately generic across the three: a recogniser that knew only the VisionFive 2's wording
/// would report a healthy aarch64 or `x86_64` board as never having booted.
pub const BANNER: &str = "nife on ";

/// **The machine description finished: `nife machine: <arch>, <n> processor(s), ...`.**
///
/// Printed by `kernel/src/main.rs`'s `print_machine_description` as its **last** line, and that
/// position is the point: reaching it means the whole description printed, which is the claim a
/// ladder rung should make. A header would only have meant the block started.
pub const MACHINE: &str = "nife machine: ";

/// **The self-test verdict: `nife self-test: 5 of 5 passed`, or `... , 1 FAILED: <names>`.**
///
/// Printed by `kernel/src/self_test.rs` after the description and before anything is handed to
/// userspace. The counts are there on success as well as on failure, so that "ran nothing and
/// passed" is distinguishable from "ran the set and passed".
pub const SELF_TEST: &str = "nife self-test: ";

/// **What a red verdict says before it names the checks that failed.**
///
/// Uppercase, and the same token `kernel/src/main.rs`'s preemption check has used since milestone
/// 267 (`FAILED: a spinner did not run, or nothing was preempted.`), so this is the tree's word
/// rather than a new one. It appears *inside* a [`SELF_TEST`] line; a matcher wanting a failed
/// verdict must find both.
pub const SELF_TEST_FAILED: &str = "FAILED: ";

/// **The boot self-test's checks, by name, in the order they run** (milestone 268).
///
/// One list for every architecture, and that is the whole point. Before it the set was a
/// compile-time fact inside `kernel/src/self_test.rs`, so an architecture that ran a different set,
/// or a smaller one with the count adjusted to match, still printed *N of N passed* and read as
/// green. Measured on 2026-09-14: `x86_64` with `scheduler` cut and the count set to four printed
/// `nife self-test: 4 of 4 passed` and `boot-check` passed it.
///
/// So the kernel takes its check names from here, and its verdict counts against this list rather
/// than against what ran. A listed check that did not run fails by name, a check that is not
/// listed fails by name, and `board_console` fails a verdict whose total is not this list's length.
/// Changing the set is an edit to this one list, which every architecture and the recogniser read.
///
/// Names provisional (milestone 268): they are printed, and matched, so they are a contract.
pub const SELF_TEST_CHECKS: &[&str] = &["exceptions", "mapping", "frames", "timer", "scheduler"];

/// **The RISC-V demonstration tour ran to its end.**
///
/// Printed by the RISC-V arm of `kernel/src/main.rs` and by nothing else, which is milestone 268's
/// finding 3 stated as a constant rather than as a defect: this rung is **one architecture's**, and
/// keeping it here beside the three that are not is what makes that visible. It is not a rung of
/// the portable ladder and the other two architectures do not print it.
pub const TOUR: &str = "nife: the capability core runs on ";

/// **Userspace is up and the shell is offering a prompt.**
///
/// `components/src/swish.rs` prints this immediately before its first `$ `. It is the terminal
/// state of a default boot: milestone 268 decided that *nothing halts*, and that the prompt rather
/// than a halt is the signal that the boot finished.
///
/// See this module's `BUGS` for why the banner and not the prompt itself.
pub const PROMPT: &str = "nife capability shell";

/// **The kernel is holding the screen open for a host that wants to photograph it**
/// (milestone 445 (the screen check stops sampling and starts asking)), and will clear it as soon as one byte comes
/// back on the serial line.
///
/// *Wording provisional, like every other constant here.*
///
/// **Not a rung of the ladder, and it is here for the reason the rungs are**: it is a line two
/// binaries agree on. `kernel::console::yield_screen` prints it when the boot command line carried
/// `machine_discovery::framebuffer::SCREEN_HOLD`; `cargo xtask uefi-boot` waits for it, takes its
/// screendump while the tour is guaranteed to still be on the framebuffer, and answers. A boot that
/// was not asked to hold never prints it.
///
/// It exists because the alternative was sampling. The window between the kernel painting its tour
/// and the handover clearing it closes in *guest* time, so no host-side deadline widens it: on
/// 2026-09-20 a loaded `script/test` run caught zero rows where the same leg run alone caught 56.
pub const SCREEN_HELD: &str = "nife screen: held for the host; ";
