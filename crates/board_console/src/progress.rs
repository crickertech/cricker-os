//! **How far did the boot get?**, decided from the console text alone.
//!
//! Every marker below is quoted from something in this tree rather than remembered: the bench
//! runbook in `notes/visionfive2.md` ("What appears, in order, on a good day" and the
//! failure-triage ladder), and `kernel/src/lib.rs` and `kernel/src/panic.rs` for the lines that
//! are ours. Nothing here was invented, which matters more than usual, because a recogniser that
//! matches text no board ever prints fails in the direction that looks like success.
//!
//! **And every one of them has now been checked against the board**, which is a different and
//! better claim than "quoted from documentation" and was not available when this was written.
//! calef captured a full boot and a full failure on 2026-09-01; both are in
//! `tests/fixtures/captured/` and both are asserted on. The documentation was right about the
//! seven markers it named. It was silent about two things the board does, and both are here now:
//! U-Boot refusing outright before the kernel runs, and the line that means our whole boot tour
//! finished rather than merely started.
//!
//! # Two halves, and only one of them is this board's
//!
//! **The firmware prologue is a board profile and lives in [`crate::board`]** (milestone 324 part
//! 3). `U-Boot SPL`, OpenSBI, U-Boot proper and `Starting kernel ...` are radon's firmware chain
//! and nobody else's; they used to be four `if line.contains(...)` arms here and four variants of
//! [`Stage`], which is the hard-coding calef's ruling named. They are now declared as data, and a
//! boot that climbs them reaches [`Stage::Firmware`].
//!
//! **Everything from [`Stage::Banner`] up is the kernel's own ladder**, reachable on all three
//! architectures since milestone 268, and it is shared by every board this tool will ever watch.
//! `crates/boot_ladder` holds its markers; a board profile may not name one.
//!
//! # BUGS
//!
//! **A marker is shared; the fields inside the line are not, and that gap is silent.** The sweep's
//! seven heads are `crates/job_mix`'s own constants, so a kernel that renames one cannot disagree
//! with this file. Everything after the head is matched here by a string literal that exists
//! nowhere else: `tasks=`, `ticks_median=`, `rounds=`, `beat=`. A kernel that renames a field, or
//! inserts one, still prints a line this recogniser matches, and the parse reads **nothing** while
//! every test stays green.
//!
//! That is not hypothetical. On **2026-09-19** two sessions did it to each other inside a day. One
//! lane moved the markers into `crates/job_mix`; another changed
//! `kernel/src/job_mix.rs`'s point line from `ticks=<t> jpm=<r>` to a median of 21 repeats with its
//! two ends, so `ticks=` and `jpm=` stopped existing. The head `job-mix: tasks=` never moved.
//! Both branches were green: the parser and the committed fixture had been made from the same old
//! kernel and agreed with each other, and neither agreed with the kernel. It was found by reading,
//! which is rung zero of `AGENTS.md`'s ladder.
//!
//! Two things blunt it and neither closes it. The fixtures are **captures from a real boot** rather
//! than hand-written lines, so re-capturing catches a rename the moment somebody re-captures; and
//! this module's fields are spelled exactly as the wire spells them, so the two can be diffed by
//! eye. Sharing the field names the way the heads are shared is the fix, and it is not built here.
//!
//! **A sweep's longest legitimate silence is a measurement, and it belongs to the machine that
//! measured it.** The quiet timer that `script/job-mix` and `script/board-console` set against
//! [`Stage::Sweep`] is sized against the slowest subrun at the top of `job_mix::TASK_SWEEP`, since
//! a sweep has no wall-clock heartbeat to miss. Under TCG on 2026-09-19 that subrun was
//! 249,234,771 ticks on a 62.5 MHz counter, which is **4.0 seconds**, from the capture in
//! `tests/fixtures/captured/`. The default is sixty seconds, fifteen times it. That margin used to
//! be twenty to one against a 2.6-second subrun, and milestone 168 spent a quarter of it by taking
//! twenty-one repeats of a seven-kind mix instead of three of a five-kind one; a figure quoted from
//! a capture is only as current as the capture. A board outside the margin reads as wedged when it
//! is merely slow, and `--quiet-after 0` is the escape that gives up the detection entirely.

use core::fmt;

use crate::board;

/// How far the boot got, as an ordered ladder.
///
/// Ordered because the only question the tool ever asks is "did we reach at least here", and
/// because the sequence is what the runbook records: SPL, OpenSBI, U-Boot, handoff, ours. The
/// derived `Ord` is the comparison; do not reorder these variants without reading the runbook
/// again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Nothing recognisable yet. Either the board is unpowered, the adapter is on the wrong pins,
    /// or the baud is wrong; the failure-triage ladder's first row covers all three.
    #[default]
    Cold,
    /// **A rung of the active board's firmware prologue** (milestone 324 part 3), which is
    /// whichever [`board::Profile`] this session was given.
    ///
    /// Four variants used to sit here instead (`Spl`, `OpenSbi`, `UBoot`, `Handoff`), and every one
    /// of them was radon's. They are now [`board::RADON`]'s four rungs, ordered by
    /// [`board::Rung::depth`], and a board with a different firmware chain declares its own rather
    /// than sharing radon's words. A board with no prologue this tool recognises, which is what
    /// xenon's capture shows, never reaches this stage at all and goes straight from
    /// [`Stage::Cold`] to [`Stage::Banner`].
    ///
    /// Below [`Stage::Banner`] because the firmware runs before the kernel does, whatever the
    /// board: that is what makes one ordering serve every profile.
    Firmware(&'static board::Rung),
    /// Our own banner (`boot_ladder::BANNER`). The kernel's console works, which on this board is
    /// not a given: the runbook is explicit that this line is the *second* target, after the
    /// DW-8250 driver work.
    ///
    /// **Reachable on all three architectures since milestone 268.** It was not before: aarch64
    /// printed no opening line at all, so a healthy aarch64 board reported as never having booted,
    /// which is the same defect [`Stage::Tour`] carries and is why that one is documented as one
    /// architecture's rather than quietly left in the ladder.
    Banner,
    /// The machine described itself and finished (`boot_ladder::MACHINE`), so the boot has read the
    /// device tree or the ACPI tables, counted the processors, found the console's interrupt line,
    /// the PCIe window and the IOMMU, and printed all of it.
    ///
    /// **A stronger claim than the banner and a weaker one than the self-test**, which is exactly
    /// what a rung is for: the banner says the console works, this says discovery worked, and the
    /// verdict below says the kernel works. A board that reaches the banner and stops here has a
    /// console and a machine it cannot read.
    Machine,
    /// The boot self-test printed its verdict (`boot_ladder::SELF_TEST`).
    ///
    /// **This is the stage to wait for**, and it is the one milestone 268 built: it is reachable on
    /// every architecture, it means paging, traps, the frame allocator, the timer with its
    /// interrupt, and the scheduler all came up, and the line it matches carries counts rather than
    /// a claim. A *failed* verdict is not this stage: it is [`Failure::SelfTestFailed`], so a
    /// degraded board on the bench does not read as a good one.
    SelfTest,
    /// The boot tour ran to its end (`nife: the capability core runs on ...`).
    ///
    /// A stronger signal than the banner and a different claim: the banner is printed before the
    /// device tree is touched, so it says the console works and nothing else, while this says
    /// paging, traps, the timer, the frame allocator, SMP and the scheduler all came up. It is
    /// **not** the default to wait for, because only the milestone-tour build prints it; a shell
    /// or a test build reaches its banner and then does something else entirely.
    ///
    /// **It is one architecture's rung and it is kept as one** (milestone 268). The other two never
    /// print it and are not expected to; what they print instead is [`Stage::Machine`] and
    /// [`Stage::SelfTest`], which every architecture reaches. Waiting for this on aarch64 or
    /// `x86_64` is waiting for something that does not happen, and that was invisible until
    /// milestone 268's finding 3 named it.
    Tour,
    /// Userspace is up and the shell is offering a prompt (`boot_ladder::PROMPT`).
    ///
    /// **The terminal state of a default boot** since milestone 268 (every architecture boots the
    /// same way): nothing halts, and the prompt
    /// rather than a halt is the signal that the boot finished. Above [`Stage::Tour`] because a
    /// boot that reaches a prompt has gone past any demonstration on the way.
    ///
    /// **Reachable on all three architectures since 2026-09-19**, on each one's *default* boot with
    /// an archive attached: aarch64 and riscv64 hand over at the end of the boot ladder's tour,
    /// and `x86_64` does too, once DECISIONS §149 (may the kernel answer on an endpoint) said how
    /// a shell reaches a console there and milestone 299 (the x86 port-range capability) made
    /// `console` a userspace driver holding one. It was aarch64-and-riscv64-only before that, which is stated
    /// here rather than left for a reader to discover from a watch that times out, and
    /// `cargo xtask boot-check` asserts it on every architecture now.
    Prompt,
    /// A sustained workload announced itself and is expected to keep speaking (milestone 219).
    ///
    /// **This is the only stage after which silence is a failure again.** Every stage below it is
    /// a step in a boot that ends with the kernel halting in `wfi`, so quiet after [`Stage::Tour`]
    /// is how a good boot ends and reporting it as a hang would fail every one. A soak is the
    /// opposite contract: `kernel/src/soak.rs` prints a heartbeat on the wall clock every five
    /// seconds whatever the workload is doing, so a gap says the thing that prints is itself
    /// wedged. See `watch::Policy::quiet_after`, which is where that asymmetry is implemented.
    Soak,
    /// **The job-mix sweep announced itself and is expected to keep making progress** (milestone
    /// 324 part 2, matching [`job_mix::STARTED`]).
    ///
    /// Like [`Stage::Soak`] in the way that matters: it is past the end of the boot tour, and
    /// silence from here is a failure rather than a kernel that halted on purpose. **Unlike it in
    /// the way that bites.** A soak beats on the wall clock, so a missed beat is a missed deadline.
    /// A sweep's finest progress line is [`job_mix::SUBRUN`], one per measured subrun, and a subrun
    /// takes as long as it takes; the longest legitimate silence is the slowest subrun at the top
    /// of [`job_mix::TASK_SWEEP`]. `watch::Policy::quiet_after` has to be set against that, and
    /// this module's `BUGS` carries the measured figure.
    ///
    /// Above [`Stage::Soak`] because the enum has to order them somehow and neither is reachable in
    /// the other's build: `script/job-mix`'s own `BUGS` records that the two features are
    /// alternatives and that a kernel carrying both would sweep and never soak. Nothing compares
    /// them, and nothing should.
    Sweep,
    /// **The sweep ran to its end** ([`job_mix::DONE`]), and the kernel is parking in `wfi`.
    ///
    /// **This is the rung milestone 324 part 2 exists for.** Before it, a finished sweep and a
    /// wedged one both ended as the clock running out, so they shared an exit status and a bench
    /// script could not tell them apart. Now `--until sweep-done` is `0` when the sweep finished,
    /// `2` when it spoke and then stopped, and `3` when the time ran out with points still to
    /// print.
    ///
    /// Silence after this is the correct end state, the same as [`Stage::Tour`]'s, and
    /// `watch::Policy::quiet_after` exempts it for the same reason.
    SweepDone,
}

impl Stage {
    /// The stage's name as a person would say it, for a report line.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Stage::Cold => "nothing recognisable",
            // The profile's own word for the rung, which is why a report can name `U-Boot SPL`
            // without this file knowing what a VisionFive 2 is.
            Stage::Firmware(rung) => rung.label(),
            Stage::Banner => "kernel banner",
            Stage::Machine => "machine described",
            Stage::SelfTest => "self-test verdict",
            Stage::Tour => "boot tour complete",
            Stage::Prompt => "shell prompt",
            Stage::Soak => "soak running",
            Stage::Sweep => "job-mix sweep running",
            Stage::SweepDone => "job-mix sweep complete",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A failure the console text names outright, so the tool can stop now instead of waiting out a
/// deadline that will tell it nothing new.
///
/// Every one of these is a *positive* statement printed by something that is still alive. A board
/// that has gone silent is not in here; silence is the watcher's business, not the recogniser's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// **The board's firmware gave up before the kernel ran**, carrying the profile's own
    /// diagnosis and, where that firmware puts the reason on the line before, that line.
    ///
    /// **One variant for a whole board profile** (milestone 324 part 3), where two used to be hard
    /// coded here: `BadImageMagic` for U-Boot rejecting the payload's header, and `UBootRefused`
    /// for `### ERROR ### Please RESET the board ###`. Both are U-Boot's words rather than ours, so
    /// they are [`board::Refusal`]s declared beside the rungs they belong to, and a board with
    /// different firmware refuses in its own words without a variant being added here.
    ///
    /// `reason` is empty when the firmware said only that it gave up.
    FirmwareRefused {
        /// One line naming what went wrong, from [`board::Refusal::diagnosis`].
        diagnosis: &'static str,
        /// The line before the refusal, when this firmware puts the reason there.
        reason: String,
    },
    /// `MEASURED BOOT REFUSED`: the kernel would not vouch for the archive it was handed. Boot 12
    /// (2026-08-15) is the worked example, and it was `script/board-image` building the pair in the
    /// wrong order rather than anything on the board.
    MeasuredBootRefused,
    /// `[PANIC] ...`: our own panic handler (`kernel/src/panic.rs`), carrying its message.
    KernelPanic(String),
    /// **`nife self-test: 4 of 5 passed, 1 FAILED: <names>`**: the kernel tested itself on this
    /// machine and one of the checks did not pass, carrying the names of the ones that did not.
    ///
    /// **This is a failure and not a stage**, which is milestone 268's item 5: a degraded board on
    /// the bench must not read as a good one, and a boot that reaches the verdict red has reached
    /// something different from a boot that reaches it green.
    ///
    /// **The boot did not stop**, and that is the one thing about this variant a reader has to
    /// know. The self-test reports and does not gate (calef, 2026-09-09: a machine you cannot log
    /// into is a machine you cannot fix), so the kernel carried on to userspace and a prompt may
    /// well be waiting. What this says is that something the kernel needs is broken, and the board
    /// is worth looking at rather than worth using.
    SelfTestFailed(String),
    /// **The kernel would not start the job-mix sweep** ([`job_mix::FAILED`]), carrying the reason
    /// it gave (milestone 324 part 2).
    ///
    /// Printed before [`Stage::Sweep`] and followed by a halt, in all three cases
    /// `kernel/src/job_mix.rs` has: no `job_mix_task` in the archive, or either kind of spawn
    /// failing. It is a failure rather than a stage for milestone 268's item 5's reason: a board
    /// that refused to run the workload must not read like one that ran it.
    SweepFailed(String),
}

impl Failure {
    /// One line naming what went wrong, for the report and the exit message.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            // The profile wrote the sentence; this only decides whether the reason is appended.
            // `notes/board-console.md` has where each board's wording came from.
            Failure::FirmwareRefused { diagnosis, reason } if reason.is_empty() => {
                (*diagnosis).to_string()
            }
            Failure::FirmwareRefused { diagnosis, reason } => format!("{diagnosis}: {reason}"),
            // Worded carefully, because this is the one failure that is not a defect. The gate
            // did its job: it noticed that the archive on the card is not the one this kernel was
            // built to vouch for, and halted instead of running it. A report that read like a
            // crash would send somebody debugging the boot mechanism, when what is wrong is that
            // two files came from different builds.
            Failure::MeasuredBootRefused => "the measured-boot gate refused the archive and \
                 halted, which is the gate working rather than a crash: the kernel and the archive \
                 on the card are from different builds. Rebuild both with script/board-image, \
                 which orders those steps"
                .to_string(),
            Failure::KernelPanic(message) => format!("the kernel panicked: {message}"),
            // Worded to say what happened next, because the obvious reading of "a self-test
            // failed" is that the machine stopped, and it did not. Someone acting on this report
            // needs to know the board is still up and is worth logging into.
            Failure::SelfTestFailed(names) => format!(
                "the kernel's boot self-test failed on this machine: {names}. The boot continued to \
                 userspace anyway (it reports, it does not gate), so the board is up and degraded \
                 rather than dead"
            ),
            // Worded to say that nothing ran, because the sweep's own output is absent either way
            // and an empty log reads the same as a wedge until this line is found.
            Failure::SweepFailed(why) => format!(
                "the kernel refused to start the job-mix sweep and halted: {why}. No point of the \
                 sweep was measured"
            ),
        }
    }
}

/// One soak heartbeat's numbers (milestone 219), as the kernel printed them.
///
/// Every field is cumulative except [`rate`](Self::rate), which is the last interval's. The one a
/// later run is compared against is [`rounds`](Self::rounds); the rest are what make it
/// interpretable, and [`refused`](Self::refused) is the one that is a finding rather than a
/// statistic.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SoakBeat {
    /// Seconds since the soak started, by the kernel's own timer.
    pub seconds: u64,
    /// Which heartbeat this is. A gap in this sequence means console output was lost, which is a
    /// different fault from the workload stopping and is worth being able to tell apart.
    pub beat: u64,
    /// **Cumulative IPC round trips completed by every worker.** The comparable number.
    pub rounds: u64,
    /// Round trips per second over the last interval.
    pub rate: u64,
    /// Cumulative refused wakes. Expected to be zero, and a nonzero value is the defect
    /// `design/fatal-risks/README.md`'s multicore entry exists for.
    pub refused: u64,
    /// Cumulative wrong replies seen by callers. Expected to be zero.
    pub mismatches: u64,
    /// How many workers made no progress in the last interval. Expected to be zero.
    pub stalled: u64,
    /// **Cumulative times a thread ran on a different core than the one it last ran on.** The
    /// honest cross-core handoff count; see `kernel/src/soak.rs` and `notes/soak.md` for the
    /// measured reason a steady-state workload barely moves this at all.
    pub crossings: u64,
    /// Cumulative wakes and placements that named a remote core. Narrower than
    /// [`crossings`](Self::crossings) and kept beside it because the two disagreeing is the
    /// finding: a rendezvous wake queues its peer locally, so a migration it performs is invisible
    /// here.
    pub remote: u64,
    /// **Cumulative tick-route wakes** (milestone 221), and the reason a soak crosses cores at all.
    ///
    /// A soak build signals a rendezvous from `sched::on_tick`, and one worker per group blocks on
    /// it through `Irq::WAIT`; every wake takes the real device-interrupt path down through
    /// `sched::wake_load_aware`, which is where this project's one observed multicore defect was.
    ///
    /// Zero in a log from before milestone 221, which is exactly how a reader (and the summary
    /// below) tells a run whose scheduler had nothing to make it cross from a run that did and
    /// still did not.
    ///
    /// It is **not** part of [`rounds`](Self::rounds), because a wake is not a round trip.
    pub wakes: u64,
}

/// **One completed point of the job-mix sweep**, as the kernel printed it (milestone 324 part 2,
/// from a [`job_mix::POINT`] line).
///
/// One per entry in [`job_mix::TASK_SWEEP`], carrying the [`job_mix::Spread`] of that entry's
/// [`job_mix::REPEATS`] subruns and the jobs-per-minute figure the kernel computed from the median.
/// See `crates/job_mix` for what a jobs-per-minute figure is and is not, and `notes/job-mix.md` for
/// why one boot's is a draw rather than a result.
///
/// # Why all seven fields, when liveness needs none of them
///
/// Counting points is the ratchet's job, not this struct's: a watcher deciding whether a sweep is
/// moving reads [`Stage`] and [`SweepSubrun`]. This exists to be **reported**, which is the one
/// place a person reads a sweep's numbers without opening the log, and that is what settles the
/// field list.
///
/// The kernel prints the fastest and slowest repeats beside the median *so the spread is never
/// hidden* (`job_mix::REPEATS` carries the argument, and milestone 168 changed the statistic for
/// exactly that reason). A reader that kept only the median would re-hide it at the last step, and
/// would have reintroduced the defect one line away from where it was fixed.
/// [`repeats`](Self::repeats) is here for the same reason and is the one easiest to think
/// unnecessary: a median of 21 and a median of 3 are different claims, and a struct that drops the
/// count lets a transcript from either be quoted as the other.
///
/// The field names are the wire's, so the struct can be diffed against a [`job_mix::POINT`] line by
/// eye. That is not tidiness; it is the cheapest defence available against this module's first
/// `BUGS` entry.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SweepPoint {
    /// Tasks released for this point, an entry of [`job_mix::TASK_SWEEP`].
    pub tasks: u64,
    /// Jobs the pool completed, which is `tasks` times [`job_mix::JOBS_PER_TASK`].
    pub jobs: u64,
    /// Subruns measured at this point, which is [`job_mix::REPEATS`] on any kernel that printed
    /// the field at all. Read from the line rather than assumed, because the whole point of
    /// reading it is to catch a log whose kernel disagrees with this build.
    pub repeats: u64,
    /// The fastest repeat's wall-clock ticks, on the kernel's own counter.
    pub ticks_min: u64,
    /// The median repeat's ticks, which is the statistic [`jpm_median`](Self::jpm_median) is
    /// computed from.
    pub ticks_median: u64,
    /// The slowest repeat's ticks. With [`ticks_min`](Self::ticks_min) it is the spread, and the
    /// spread is what says whether the median means anything on this machine.
    pub ticks_max: u64,
    /// Jobs per minute at the median, as the kernel computed it.
    pub jpm_median: u64,
}

/// **One completed subrun of the sweep** (a [`job_mix::SUBRUN`] line).
///
/// Finer than [`SweepPoint`] by a factor of [`job_mix::REPEATS`], and it is the finest progress a
/// sweep emits. That is what makes it worth keeping separately: a sweep still inside a point has
/// printed one of these and no point at all, and a watcher deciding whether a sweep is moving has
/// nothing else to look at.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SweepSubrun {
    /// Tasks released for the point this subrun belongs to.
    pub tasks: u64,
    /// Which repeat of that point this was, counted from zero.
    pub repeat: u64,
    /// Its wall-clock ticks, on the kernel's own counter.
    pub ticks: u64,
}

/// The ratchet: how far the boot got, and the first failure it announced.
///
/// It only ever moves forward. That is what lets the caller re-offer a partial line as more bytes
/// arrive without the recogniser double-counting, which it must do because U-Boot's `StarFive #`
/// prompt has no newline after it and would otherwise never be seen.
#[derive(Debug, Clone)]
pub struct BootProgress {
    /// **Which board's firmware prologue to expect** (milestone 324 part 3). Fixed for the life of
    /// the ratchet: a session watches one board, and a profile swapped halfway through would make
    /// two rungs at the same depth compare equal while meaning different things.
    board: &'static board::Profile,
    reached: Stage,
    failure: Option<Failure>,
    relocated: bool,
    userspace_ran: bool,
    banner_line: Option<String>,
    /// The machine description's summary line (milestone 268), so a report can say what machine
    /// answered without a reader going back to the log. `None` until [`Stage::Machine`].
    machine_line: Option<String>,
    /// The self-test verdict exactly as it arrived (milestone 268), counts and all. Kept even when
    /// it is green, because "five of five" and "one of one" are different facts about a build and
    /// the difference is what a vacuous pass looks like.
    self_test_line: Option<String>,
    /// The most recent soak heartbeat's numbers (milestone 219), so the tool can put the run's own
    /// figure in its summary instead of making a reader go back to a log for it. `None` until a
    /// heartbeat has been seen and parsed.
    soak: Option<SoakBeat>,
    /// The last job-mix point that printed (milestone 324 part 2). `None` until one has.
    sweep_point: Option<SweepPoint>,
    /// The last job-mix subrun that printed. `None` until one has.
    sweep_subrun: Option<SweepSubrun>,
    /// The last complete non-empty line, kept for exactly one reason: U-Boot's `### ERROR ###`
    /// says that it gave up and the line before it says why, and a reader handed only the first
    /// half has to go back to the log to learn anything.
    last_line: String,
}

/// A ratchet for [`board::RADON`], which is what every caller watched before board profiles
/// existed and what `watch::Policy::default` still chooses.
///
/// **A default that names a board is a foot gun and says so.** A report from a session that took
/// this default over an emulator will say `U-Boot SPL` was never reached, which is true and
/// useless. Prefer [`BootProgress::new`] with the board in hand; see [`board`]'s `BUGS` for why
/// QEMU's callers do not bother.
impl Default for BootProgress {
    fn default() -> Self {
        Self::new(&board::RADON)
    }
}

impl BootProgress {
    /// A fresh ratchet for one board, before any byte has arrived.
    #[must_use]
    pub fn new(board: &'static board::Profile) -> Self {
        Self {
            board,
            reached: Stage::default(),
            failure: None,
            relocated: false,
            userspace_ran: false,
            banner_line: None,
            machine_line: None,
            self_test_line: None,
            soak: None,
            sweep_point: None,
            sweep_subrun: None,
            last_line: String::new(),
        }
    }

    /// The board profile this ratchet was given.
    #[must_use]
    pub fn board(&self) -> &'static board::Profile {
        self.board
    }

    /// The furthest stage recognised so far.
    #[must_use]
    pub fn reached(&self) -> Stage {
        self.reached
    }

    /// The first announced failure, if the board announced one.
    #[must_use]
    pub fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }

    /// Whether a **pre-milestone-295** kernel's userspace init built its child
    /// (`init/build  : ...`, which `kernel/src/lib.rs` printed until 2026-09-14).
    ///
    /// **This answers a question about captured logs, not about a live board**, and that is the
    /// whole of what milestone 295 changed here. calef retired `components/src/builder.rs` on
    /// 2026-09-14, so no kernel this tree builds prints `init/build` any more and this is `false`
    /// on every live boot. The matcher stays because
    /// `tests/fixtures/captured/vf2-2026-09-01-userspace.log` carries the line: that is evidence
    /// off real VisionFive 2 silicon and cannot be re-taken with a different kernel, and a
    /// recogniser that could no longer read it would throw the evidence away to tidy the code.
    ///
    /// **The live rung that replaced it is [`Stage::Prompt`]** (`boot_ladder::PROMPT`), and it says
    /// more: `init/build` meant userspace built one child from two capabilities, where a prompt
    /// cannot appear unless userspace built the console server, the line discipline, the input
    /// driver and the shell. Ask `reached() >= Stage::Prompt` of a board booted today.
    ///
    /// Not a stage, and deliberately, because the ladder has to stay a ladder: a kernel with no
    /// archive on the card ran its whole tour and never reached this, so putting it below
    /// [`Stage::Tour`] would make reaching the tour imply something that did not happen. It is a
    /// detail of a successful boot, like [`Self::is_relocated`], and it is the difference between
    /// the two successful captures.
    #[must_use]
    pub fn userspace_ran(&self) -> bool {
        self.userspace_ran
    }

    /// Whether U-Boot said it moved the image (`Moving Image from ...`).
    ///
    /// Not a stage, because the runbook lists it as a *discriminator* rather than a step: it is
    /// what you check when `Starting kernel ...` is followed by silence.
    #[must_use]
    pub fn is_relocated(&self) -> bool {
        self.relocated
    }

    /// The latest soak heartbeat, if this session saw one.
    ///
    /// **This is the number milestone 219 is for.** A soak that ends with nothing printed proves
    /// very little; one that ends with a round-trip total is something a later run can be compared
    /// against. It is a progress figure and nothing else: see
    /// `design/roadmap/219-a-workload-that-does-not-stop.md` for why a clean run is weak evidence.
    #[must_use]
    pub fn soak(&self) -> Option<&SoakBeat> {
        self.soak.as_ref()
    }

    /// The last job-mix sweep point that printed, if this session saw one (milestone 324 part 2).
    ///
    /// A bench script reading this gets the curve's latest entry without re-parsing the log. What
    /// it does **not** get is the whole curve: this is a progress ratchet, and the points are in
    /// the capture. `crates/board_console::lottery` is the precedent for reading a finished log for
    /// its results.
    #[must_use]
    pub fn sweep_point(&self) -> Option<&SweepPoint> {
        self.sweep_point.as_ref()
    }

    /// The last job-mix subrun that printed, if this session saw one.
    ///
    /// The finest sign a sweep is still moving, and the only one during the long subruns at the top
    /// of [`job_mix::TASK_SWEEP`], where no point line is due for minutes.
    #[must_use]
    pub fn sweep_subrun(&self) -> Option<&SweepSubrun> {
        self.sweep_subrun.as_ref()
    }

    /// Our banner line exactly as it arrived, which is how the reader learns which architecture
    /// the board that answered actually is.
    #[must_use]
    pub fn banner_line(&self) -> Option<&str> {
        self.banner_line.as_deref()
    }

    /// The machine description's summary line, if this session saw one (milestone 268). It names
    /// the architecture, the processor count, the memory size and the tick rate, which is the
    /// shortest honest answer to "what did I just boot".
    #[must_use]
    pub fn machine_line(&self) -> Option<&str> {
        self.machine_line.as_deref()
    }

    /// The self-test verdict exactly as the kernel printed it, if this session saw one (milestone
    /// 268), whether it was green or red.
    ///
    /// **Kept on a green boot too**, deliberately: the counts are the only defence against a
    /// vacuous pass, and a reader comparing two runs needs to see that both ran the same number of
    /// checks. A red verdict also appears as [`Failure::SelfTestFailed`]; this is the raw line.
    #[must_use]
    pub fn self_test_line(&self) -> Option<&str> {
        self.self_test_line.as_deref()
    }

    /// Offer a line that ended with a line terminator.
    ///
    /// Idempotent: stages ratchet and the failure is recorded once, so the same text may be
    /// offered any number of times.
    pub fn observe_line(&mut self, line: &str) {
        self.observe(line, true);
    }

    /// Offer the incomplete tail, the bytes after the last line terminator.
    ///
    /// This exists because U-Boot's `StarFive #` prompt is printed with no newline after it, so a
    /// tool that waited for complete lines would sit there while the board sat waiting for it.
    ///
    /// **A partial line is not the same evidence as a complete one**, which is the distinction the
    /// two methods exist to keep, and it was found by a test feeding a good boot one byte at a
    /// time rather than by reasoning. Two things go wrong if a tail is treated as a line. `U-Boot `
    /// on its own looks like U-Boot proper right up until the next three bytes turn out to be
    /// `SPL`, so a board dying in SPL is reported two stages further along than it got. And any
    /// marker carrying a *payload* captures a truncated one: the tail `nife on ` recorded the
    /// banner as the empty string, and `[PANIC] ` recorded a panic with no message, both of them
    /// latched before the rest of the line arrived. So a tail may ratchet a stage, because a
    /// substring match is monotone and more bytes cannot unmake it; it may not settle a word
    /// boundary, and it may not capture text.
    pub fn observe_partial(&mut self, tail: &str) {
        self.observe(tail, false);
    }

    fn observe(&mut self, line: &str, complete: bool) {
        let line = line.trim_end_matches(['\r', '\n']);

        // **The firmware prologue, from the board profile** (milestone 324 part 3). Four `if`s
        // with radon's markers written into them used to stand here; the rungs are now declared in
        // `crate::board` and this loop is the whole of what reads them, which is what makes adding
        // a board a profile rather than an edit to a recogniser.
        //
        // Every rung is offered the line rather than stopping at the first match, because the
        // ratchet is what decides and a log can carry two rungs on one line.
        for rung in self.board.prologue {
            if rung.is_seen_in(line, complete) {
                self.reach(Stage::Firmware(rung));
            }
        }
        // **Every marker below is `boot_ladder`'s rather than a literal** (milestone 268), so this
        // recogniser and the kernel cannot hold two copies of one contract. Before that crate the
        // only marker that existed was a string literal in one architecture's arm of
        // `kernel/src/lib.rs` and a second copy of it here, which is exactly how finding 3 hid.
        //
        // `nife on ` rather than the RISC-V line specifically: `kernel/src/lib.rs` prints one of
        // these per architecture, and a recogniser that only knew the VisionFive 2's would report
        // a healthy aarch64 or x86_64 board as never having booted. The full line is kept so the
        // reader sees which one answered, and only from a complete line, or it is kept truncated.
        if let Some(at) = line.find(boot_ladder::BANNER) {
            self.reach(Stage::Banner);
            if complete && self.banner_line.is_none() {
                self.banner_line = Some(line[at..].to_string());
            }
        }
        // The machine description's closing summary (milestone 268). Its *last* line rather than
        // its first, so reaching this rung means the whole block printed.
        if let Some(at) = line.find(boot_ladder::MACHINE) {
            self.reach(Stage::Machine);
            // `&&`, not `||`: kept the same way `banner_line` is, from first arrival only. A
            // second machine line in one session is not expected, but the field's own doc says
            // "the machine description's summary line", singular, and this is what keeps that
            // true rather than accidental.
            if complete && self.machine_line.is_none() {
                self.machine_line = Some(line[at..].to_string());
            }
        }
        // The self-test verdict (milestone 268). The stage ratchets on a partial line, because a
        // substring match is monotone and more bytes cannot unmake it; the *text* is only captured
        // from a complete line, and so is the failure below, because a truncated verdict would
        // record a count that has not finished arriving.
        if let Some(at) = line.find(boot_ladder::SELF_TEST) {
            self.reach(Stage::SelfTest);
            if complete && self.self_test_line.is_none() {
                self.self_test_line = Some(line[at..].to_string());
            }
        }
        // The shell's banner, which is the top rung: userspace is up and a prompt is coming. See
        // `boot_ladder::PROMPT`'s own BUGS for why the banner rather than the `$ ` itself.
        if line.contains(boot_ladder::PROMPT) {
            self.reach(Stage::Prompt);
        }

        if self
            .board
            .relocation
            .iter()
            .any(|marker| line.contains(marker))
        {
            self.relocated = true;
        }
        // A captured-log marker rather than a live one since milestone 295: no kernel prints
        // `init/build` after `components/src/builder.rs` was retired on 2026-09-14. Kept because
        // the VisionFive 2 capture carries it; see [`BootProgress::userspace_ran`].
        if line.contains("init/build") {
            self.userspace_ran = true;
        }
        if let Some(at) = line.find(boot_ladder::TOUR) {
            self.reach(Stage::Tour);
            if complete && self.banner_line.is_none() {
                self.banner_line = Some(line[at..].to_string());
            }
        }
        // The soak (milestone 219). `soak-test: started` is `kernel/src/soak.rs`'s `START_MARKER`, and
        // the two agree by one of them being tested against the other's text rather than by both
        // being remembered. Ratcheting on the START line rather than on any `soak:` line is
        // deliberate: a `soak-test: FAILED` line reaches the failure arm below and should not also be
        // read as the workload having got going.
        if line.contains("soak-test: started") {
            self.reach(Stage::Soak);
        }
        if complete && line.contains("soak-test: t=") {
            self.observe_soak_beat(line);
        }
        // **The job-mix sweep** (milestone 324 part 2). Every marker is `crates/job_mix`'s rather
        // than a literal, the same move milestone 268 made when it put the boot ladder's markers in
        // `crates/boot_ladder`: the kernel prints these very constants, so this recogniser and the
        // thing it recognises cannot hold two copies of one contract. The census prefix
        // (`job-mix-census:`) is outside all of them, which is what keeps a census block from
        // reading as a sweep starting.
        if line.contains(job_mix::STARTED) {
            self.reach(Stage::Sweep);
        }
        if line.contains(job_mix::DONE) {
            self.reach(Stage::SweepDone);
        }
        // Complete lines only, for both. A stage ratchet is monotone so a partial may set it; a
        // captured number is not, and a truncated `ticks=` would record a figure that has not
        // finished arriving. Same rule as the self-test verdict above.
        if complete && let Some(at) = line.find(job_mix::SUBRUN) {
            self.observe_sweep_subrun(&line[at..]);
        }
        if complete && let Some(at) = line.find(job_mix::POINT) {
            self.observe_sweep_point(&line[at..]);
        }

        // Failures. Recorded once: the first thing that went wrong is the one worth reporting,
        // and everything after it is downstream.
        if self.failure.is_none() {
            // **The firmware's own refusals, from the board profile** (milestone 324 part 3).
            // First, because a firmware that gave up did so before the kernel ran and everything
            // after it is downstream. `reason_is_the_line_before` deliberately reads `last_line`
            // before the update at the bottom of this function, because U-Boot puts the refusal on
            // one line and its reason on the one before.
            if let Some(refusal) = self
                .board
                .refusals
                .iter()
                .find(|refusal| line.contains(refusal.marker))
            {
                self.failure = Some(Failure::FirmwareRefused {
                    diagnosis: refusal.diagnosis,
                    reason: if refusal.reason_is_the_line_before {
                        self.last_line.clone()
                    } else {
                        String::new()
                    },
                });
            } else if line.contains("MEASURED BOOT REFUSED") {
                self.failure = Some(Failure::MeasuredBootRefused);
            } else if complete && let Some(at) = line.find(job_mix::FAILED) {
                // Before the panic arm because the sweep's refusal is its own announcement and
                // carries a better reason than the halt that follows it.
                self.failure = Some(Failure::SweepFailed(
                    line[at + job_mix::FAILED.len()..].trim().to_string(),
                ));
            } else if complete && let Some(at) = line.find("[PANIC] ") {
                self.failure = Some(Failure::KernelPanic(
                    line[at + "[PANIC] ".len()..].to_string(),
                ));
            } else if complete
                && let Some(at) = line.find(boot_ladder::SELF_TEST)
                && let Some(failed) = line[at..].find(boot_ladder::SELF_TEST_FAILED)
            {
                // **Both markers, on one complete line.** `FAILED:` on its own is a word the boot
                // tour has used since milestone 267 for its preemption check, so matching it alone
                // would report that unrelated line as a self-test failure; and the verdict prefix
                // alone is every green boot. The names are what follows the token.
                let names = line[at + failed + boot_ladder::SELF_TEST_FAILED.len()..].trim();
                self.failure = Some(Failure::SelfTestFailed(names.to_string()));
            } else if complete
                && let Some(at) = line.find(boot_ladder::SELF_TEST)
                && let Some(total) = Self::verdict_total(&line[at + boot_ladder::SELF_TEST.len()..])
                && total != boot_ladder::SELF_TEST_CHECKS.len()
            {
                // **A green verdict over the wrong set is not green** (milestone 268). The kernel
                // counts against `boot_ladder::SELF_TEST_CHECKS`, so a total that differs is a
                // kernel built against a different list: an older one, or one architecture whose
                // set was cut and the count adjusted to hide it, which printed `4 of 4 passed`.
                self.failure = Some(Failure::SelfTestFailed(format!(
                    "the verdict counted {total} checks and boot_ladder lists {}",
                    boot_ladder::SELF_TEST_CHECKS.len()
                )));
            }
        }

        // Last, and only for a complete line: this is what the NEXT line may need, so recording a
        // partial here would hand the refusal a truncated reason.
        if complete && !line.trim().is_empty() {
            self.last_line = line.trim().to_string();
        }
    }

    /// The `Y` in a verdict's `X of Y passed`, given the text after `boot_ladder::SELF_TEST`.
    ///
    /// `None` for anything that is not that shape, so an unfamiliar verdict is left alone rather
    /// than reported as a count it never printed.
    fn verdict_total(tail: &str) -> Option<usize> {
        let mut words = tail.split_whitespace();
        words.next()?.parse::<usize>().ok()?;
        if words.next()? != "of" {
            return None;
        }
        words.next()?.trim_end_matches(',').parse().ok()
    }

    /// Pull the numbers out of one `soak-test: t=... beat=... rounds=...` line.
    ///
    /// Field-name-directed rather than positional, so adding a field to the kernel's heartbeat does
    /// not silently shift what this reads. A field that is missing or unparseable leaves the
    /// previous value in place rather than zeroing it, because a garbled line on a serial link is
    /// a lost measurement, not a measurement of zero.
    fn observe_soak_beat(&mut self, line: &str) {
        let field = |name: &str| field(line, name);
        let beat = self.soak.get_or_insert_with(SoakBeat::default);
        for (name, slot) in [
            ("t=", &mut beat.seconds),
            ("beat=", &mut beat.beat),
            ("rounds=", &mut beat.rounds),
            ("rate=", &mut beat.rate),
            ("refused=", &mut beat.refused),
            ("mismatch=", &mut beat.mismatches),
            ("stalled=", &mut beat.stalled),
            ("crossings=", &mut beat.crossings),
            ("remote=", &mut beat.remote),
            ("wakes=", &mut beat.wakes),
        ] {
            if let Some(v) = field(name) {
                *slot = v;
            }
        }
    }

    /// Pull the numbers out of one `job-mix-repeat: tasks=... repeat=... ticks=...` line.
    ///
    /// Field-name-directed, for [`Self::observe_soak_beat`]'s reason: a field added to the kernel's
    /// line must not silently shift what this reads. A line missing a field leaves the previous
    /// value in place rather than zeroing it, because a garbled line on a serial link is a lost
    /// measurement and not a measurement of zero.
    fn observe_sweep_subrun(&mut self, tail: &str) {
        let subrun = self.sweep_subrun.get_or_insert_with(SweepSubrun::default);
        for (name, slot) in [
            ("tasks=", &mut subrun.tasks),
            ("repeat=", &mut subrun.repeat),
            ("ticks=", &mut subrun.ticks),
        ] {
            if let Some(v) = field(tail, name) {
                *slot = v;
            }
        }
    }

    /// Pull the numbers out of one `job-mix: tasks=... jobs=... repeats=... ticks_min=...
    /// ticks_median=... ticks_max=... jpm_median=...` line.
    ///
    /// Field-name-directed, for [`Self::observe_soak_beat`]'s reason, and **that is not the same as
    /// safe**: this list held `ticks=` and `jpm=` for a day after the kernel stopped printing
    /// either, and nothing said so. The module's `BUGS` has it. A line missing a field leaves the
    /// previous value in place rather than zeroing it, because a garbled line on a serial link is a
    /// lost measurement and not a measurement of zero.
    ///
    /// The seven names are all distinct as substrings, which is what makes [`field`]'s `find` the
    /// right tool here: `ticks_min=`, `ticks_median=` and `ticks_max=` share a prefix and none is a
    /// prefix of another, so no search can land on a neighbour's digits.
    fn observe_sweep_point(&mut self, tail: &str) {
        let point = self.sweep_point.get_or_insert_with(SweepPoint::default);
        for (name, slot) in [
            ("tasks=", &mut point.tasks),
            ("jobs=", &mut point.jobs),
            ("repeats=", &mut point.repeats),
            ("ticks_min=", &mut point.ticks_min),
            ("ticks_median=", &mut point.ticks_median),
            ("ticks_max=", &mut point.ticks_max),
            ("jpm_median=", &mut point.jpm_median),
        ] {
            if let Some(v) = field(tail, name) {
                *slot = v;
            }
        }
    }

    fn reach(&mut self, stage: Stage) {
        if stage > self.reached {
            self.reached = stage;
        }
    }

    // `>` rather than `>=` above is deliberately not load-bearing, and that is worth recording
    // rather than leaving for the next reader to wonder about. `stage == self.reached` can only
    // hold two ways: same variant with no data, where reassigning changes nothing observable;
    // or two `Stage::Firmware` values whose rungs compare equal, which (per `board::Rung`'s own
    // `PartialEq`, depth alone) means same depth. Within one session every `Stage::Firmware` is
    // drawn from the one profile `BootProgress::new` was given, and
    // `every_profiles_depths_count_from_one_without_gaps` (in `board`) is what makes a depth
    // point at exactly one rung in that profile. So an "equal" `Stage::Firmware` is the *same*
    // `&'static Rung`, and reassigning it to itself is not observable either. `>=`'s extra
    // branch (firing when `stage == self.reached`) is therefore equivalent to `>`'s here, and a
    // mutant that makes this `>=` is expected to survive.
}

/// The decimal run after `name=` in `text`, if there is one.
///
/// Shared by the soak's heartbeat and the sweep's two lines, which is not a tidiness: all three are
/// `name=value` lines from the same console, and a second copy of this would be a second place for
/// a trailing `/s` or a comma to be got wrong.
pub(crate) fn field(text: &str, name: &str) -> Option<u64> {
    let at = text.find(name)?;
    let rest = &text[at + name.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Splits a byte stream into lines, and hands back the incomplete tail as well.
///
/// The tail is the reason this exists rather than a call to [`str::lines`]. U-Boot's `StarFive #`
/// prompt is printed with no newline after it, so a console tool that only recognises complete
/// lines waits forever at exactly the moment the board is waiting for *it*.
#[derive(Debug, Default)]
pub struct LineFeeder {
    pending: String,
}

impl LineFeeder {
    /// A fresh splitter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The bytes seen since the last line terminator, which have not been offered as a line.
    ///
    /// A caller that has stopped reading should hand this to [`BootProgress::observe_line`]: at
    /// the end of a session there is nothing more coming, so the tail is as complete as it will
    /// ever be. That is what catches a last line printed without a newline, which a panic followed
    /// by a halted machine can be.
    #[must_use]
    pub fn tail(&self) -> &str {
        &self.pending
    }

    /// Feed a chunk of bytes; returns every line it completed, plus the incomplete tail.
    ///
    /// Bytes are decoded lossily on purpose. A wrong baud rate produces bytes that are not UTF-8,
    /// and that is a case the tool must survive and report (runbook row two) rather than a case it
    /// may fail on.
    ///
    /// Both `\n` and `\r` end a line, because a serial console emits `\r\n` and because a firmware
    /// progress display emits bare `\r` to overwrite itself.
    pub fn feed(&mut self, bytes: &[u8]) -> Feeding {
        self.pending.push_str(&String::from_utf8_lossy(bytes));
        let mut lines = Vec::new();
        while let Some(at) = self.pending.find(['\n', '\r']) {
            let line = self.pending[..at].to_string();
            self.pending.drain(..=at);
            lines.push(line);
        }
        Feeding {
            lines,
            tail: self.pending.clone(),
        }
    }
}

/// What one chunk of bytes turned into: the lines it completed and the tail still in flight.
#[derive(Debug)]
pub struct Feeding {
    /// Lines that ended with `\n` or `\r`, in order.
    pub lines: Vec<String>,
    /// The bytes after the last line ending, which may still grow.
    pub tail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// radon's firmware rung by its `--until` word, which is what `Stage::Spl` and its three
    /// siblings used to be. Assertions read the same; the rung is now the profile's rather than
    /// this file's, which is milestone 324 part 3 in one line.
    fn rung(key: &str) -> Stage {
        Stage::Firmware(
            board::RADON
                .rung(key)
                .unwrap_or_else(|| panic!("radon has no {key} rung")),
        )
    }

    /// Offer text to a fresh radon ratchet the way the watcher does, tail included.
    fn run(text: &str) -> BootProgress {
        feed(&board::RADON, text)
    }

    /// [`run`], for a board that is not radon (milestone 324 part 3).
    fn feed(board: &'static board::Profile, text: &str) -> BootProgress {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::new(board);
        // One byte at a time, which is the worst case a real UART delivers and the case that
        // catches a recogniser depending on chunk boundaries.
        for byte in text.as_bytes() {
            let feeding = feeder.feed(&[*byte]);
            for line in &feeding.lines {
                progress.observe_line(line);
            }
            progress.observe_partial(&feeding.tail);
        }
        progress
    }

    /// **The real thing**: bytes off the wire on 2026-09-01, control characters and all, fed one
    /// byte at a time. Everything else in this file is a unit test; this is the only one that
    /// says the markers are the text a board prints.
    #[test]
    fn the_captured_boot_runs_the_whole_tour() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-manual-boot.log"
        ));
        assert_eq!(progress.reached(), Stage::Tour);
        assert_eq!(progress.failure(), None);
        assert!(progress.is_relocated());
        assert!(
            !progress.userspace_ran(),
            "this card carried no archive, and the tour says so"
        );
        assert_eq!(
            progress.banner_line(),
            Some("nife on RISC-V (rv64, S-mode, Sv39)")
        );
    }

    /// The third outcome, and the one no documentation in this tree described: U-Boot gives up
    /// **after** loading and relocating the image and **before** the kernel runs. It is not a
    /// hang, and reporting it as one would send somebody hunting a multicore bug in a kernel that
    /// never executed an instruction.
    #[test]
    fn the_captured_extlinux_failure_is_u_boot_giving_up() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-extlinux-refused.log"
        ));
        assert_eq!(progress.reached(), rung("uboot"));
        assert!(progress.is_relocated(), "the image did load and relocate");
        assert_eq!(
            progress.failure(),
            Some(&Failure::FirmwareRefused {
                diagnosis: "U-Boot gave up before the kernel ran and wants the board reset",
                reason: "Device tree not found or missing FDT support".to_string(),
            }),
            "the reason is the line before the ERROR, and a reader needs it"
        );
    }

    /// **The whole portable ladder, in order** (milestone 268), and this is the test that says the
    /// three rungs a boot climbs on every architecture are recognised in the order they arrive.
    ///
    /// The lines are the kernel's own markers plus the tails it actually prints, rather than
    /// invented text: a recogniser matching text no board ever prints fails in the direction that
    /// looks like success.
    #[test]
    fn the_ladder_climbs_banner_machine_self_test_prompt() {
        let mut progress = BootProgress::default();
        progress.observe_line("nife on aarch64 (EL1, MMU off: physical addresses until mmu::init)");
        assert_eq!(progress.reached(), Stage::Banner);
        progress.observe_line("nife machine: aarch64, 4 processor(s), 256 MiB, 100 Hz");
        assert_eq!(progress.reached(), Stage::Machine);
        progress.observe_line("nife self-test: 5 of 5 passed");
        assert_eq!(progress.reached(), Stage::SelfTest);
        assert_eq!(progress.failure(), None, "a green verdict is not a failure");
        progress
            .observe_line("nife capability shell. naming a resource in a command IS granting it.");
        assert_eq!(progress.reached(), Stage::Prompt);
    }

    /// **Every architecture reaches the verdict, and that is the point of the milestone.** The
    /// same three assertions against the three banners this tree prints, because finding 3 was
    /// exactly a marker that only one architecture's arm produced.
    #[test]
    fn every_architecture_reaches_the_self_test_verdict() {
        for (banner, machine) in [
            (
                "nife on aarch64 (EL1, MMU off: physical addresses until mmu::init)",
                "nife machine: aarch64, 4 processor(s), 256 MiB, 100 Hz",
            ),
            (
                "nife on RISC-V (rv64, S-mode, Sv39)",
                "nife machine: riscv64, 4 processor(s), 256 MiB, 100 Hz",
            ),
            (
                "nife on x86_64 (long mode, ring 0, 4-level paging)",
                "nife machine: x86_64, 1 processor(s), 254 MiB, 100 Hz",
            ),
        ] {
            let mut progress = BootProgress::default();
            progress.observe_line(banner);
            progress.observe_line(machine);
            progress.observe_line("nife self-test: 5 of 5 passed");
            assert_eq!(
                progress.reached(),
                Stage::SelfTest,
                "{banner} did not climb the ladder"
            );
            assert_eq!(progress.machine_line(), Some(machine));
        }
    }

    /// **The machine line is kept from its first arrival, not overwritten by a later one**, the
    /// same rule `banner_line` follows and for the same reason: `&&` rather than `||` in the
    /// guard that captures it.
    #[test]
    fn the_machine_line_is_kept_from_first_arrival_only() {
        let mut progress = BootProgress::default();
        progress.observe_line("nife machine: riscv64, 4 processor(s), 256 MiB, 100 Hz");
        progress.observe_line("nife machine: aarch64, 8 processor(s), 512 MiB, 200 Hz");
        assert_eq!(
            progress.machine_line(),
            Some("nife machine: riscv64, 4 processor(s), 256 MiB, 100 Hz"),
            "the first machine line is kept, not replaced by a second"
        );
    }

    /// **A red verdict is a failure and not a stage.** Milestone 268's item 5: a degraded board on
    /// the bench must not read as a good one, and this is the assertion that says so.
    #[test]
    fn a_failed_verdict_is_announced_with_the_names() {
        let mut progress = BootProgress::default();
        progress.observe_line("nife machine: riscv64, 4 processor(s), 256 MiB, 100 Hz");
        progress.observe_line("nife self-test: 4 of 5 passed, 1 FAILED: exceptions");
        assert_eq!(
            progress.failure(),
            Some(&Failure::SelfTestFailed("exceptions".to_string())),
        );
        // And the stage still ratchets: the verdict *did* arrive, which is a different fact from
        // whether it was green, and a report that lost it would say the board never self-tested.
        assert_eq!(progress.reached(), Stage::SelfTest);
        assert_eq!(
            progress.self_test_line(),
            Some("nife self-test: 4 of 5 passed, 1 FAILED: exceptions"),
        );
    }

    /// **A green verdict over a set that is not `boot_ladder::SELF_TEST_CHECKS` is a failure.**
    /// Milestone 268's parity hole, measured before it was closed: `x86_64` with `scheduler` cut and
    /// the count set to four printed exactly this line, and `boot-check` passed it.
    #[test]
    fn a_green_verdict_over_the_wrong_set_is_a_failure() {
        let mut progress = BootProgress::default();
        progress.observe_line("nife self-test: 4 of 4 passed");
        assert_eq!(
            progress.failure(),
            Some(&Failure::SelfTestFailed(format!(
                "the verdict counted 4 checks and boot_ladder lists {}",
                boot_ladder::SELF_TEST_CHECKS.len()
            ))),
        );

        let mut right = BootProgress::default();
        right.observe_line(&format!(
            "nife self-test: {n} of {n} passed",
            n = boot_ladder::SELF_TEST_CHECKS.len()
        ));
        assert_eq!(right.failure(), None, "the listed total must stay green");
    }

    /// More than one failed check, because the verdict names them all and a reader needs all of
    /// them: fixing the first and re-running to discover the second is the loop this avoids.
    #[test]
    fn a_failed_verdict_carries_every_name() {
        let mut progress = BootProgress::default();
        progress.observe_line("nife self-test: 3 of 5 passed, 2 FAILED: mapping scheduler");
        assert_eq!(
            progress.failure(),
            Some(&Failure::SelfTestFailed("mapping scheduler".to_string())),
        );
    }

    /// **`FAILED:` on its own is not a self-test failure**, and this is the regression that says
    /// so. The boot tour has printed that token since milestone 267 for its preemption check, so a
    /// recogniser keying on the word alone would report every such line as a broken self-test.
    #[test]
    fn the_tours_own_failed_token_is_not_a_self_test_failure() {
        let mut progress = BootProgress::default();
        progress.observe_line("  FAILED: a spinner did not run, or nothing was preempted.");
        assert_eq!(progress.failure(), None);
    }

    /// A green verdict's counts are kept, because a reader comparing two runs needs to see both ran
    /// the same set. "0 of 0 passed" used to be a *passing* verdict on a broken build, visible only
    /// if a person read the numbers; since milestone 268 checks the total against
    /// `boot_ladder::SELF_TEST_CHECKS`, it is a failure, and its raw line is still kept.
    #[test]
    fn a_green_verdict_keeps_its_counts() {
        let green = format!(
            "nife self-test: {n} of {n} passed",
            n = boot_ladder::SELF_TEST_CHECKS.len()
        );
        let mut progress = BootProgress::default();
        progress.observe_line(&green);
        assert_eq!(progress.failure(), None);
        assert_eq!(progress.self_test_line(), Some(green.as_str()));

        // `0 of 0 passed` is the vacuous pass the counts exist to expose, and since the total is
        // checked against `boot_ladder::SELF_TEST_CHECKS` it is red rather than kept as green.
        let mut vacuous = BootProgress::default();
        vacuous.observe_line("nife self-test: 0 of 0 passed");
        assert!(matches!(
            vacuous.failure(),
            Some(Failure::SelfTestFailed(_))
        ));
        assert_eq!(
            vacuous.self_test_line(),
            Some("nife self-test: 0 of 0 passed"),
            "the raw line is kept on a red verdict too"
        );
    }

    /// A partial verdict ratchets the stage and captures nothing, which is `observe_partial`'s
    /// contract: a substring match is monotone, so more bytes cannot unmake the rung, but a
    /// truncated line would record a count that has not finished arriving.
    #[test]
    fn a_partial_verdict_ratchets_but_does_not_capture() {
        let mut progress = BootProgress::default();
        progress.observe_partial("nife self-test: 4 of 5 pas");
        assert_eq!(progress.reached(), Stage::SelfTest);
        assert_eq!(progress.self_test_line(), None);
        assert_eq!(
            progress.failure(),
            None,
            "the FAILED token has not arrived yet, and half a verdict is not a verdict"
        );
    }

    /// The ladder is an order, and these are the two comparisons the tools actually make.
    #[test]
    fn the_new_rungs_sit_where_the_milestone_put_them() {
        assert!(Stage::Machine > Stage::Banner);
        assert!(Stage::SelfTest > Stage::Machine);
        assert!(
            Stage::Tour > Stage::SelfTest,
            "the riscv tour runs after it"
        );
        assert!(Stage::Prompt > Stage::Tour, "a prompt is past any tour");
        assert!(Stage::Soak > Stage::Prompt, "a soak replaces the handoff");
    }

    /// The banner is not the end of the story, and the two captures differ by exactly this: both
    /// reach U-Boot, one reaches the banner and then the tour, the other never reaches either.
    #[test]
    fn the_tour_line_outranks_the_banner() {
        let mut progress = BootProgress::default();
        progress.observe_line("nife on RISC-V (rv64, S-mode, Sv39)");
        assert_eq!(progress.reached(), Stage::Banner);
        progress.observe_line("nife: the capability core runs on RISC-V.");
        assert_eq!(progress.reached(), Stage::Tour);
    }

    /// The trust boundary refusing, captured rather than imagined. Note where it got to: past the
    /// banner and well into the tour, which is what makes "reached the banner" a useless test for
    /// whether a boot worked.
    #[test]
    fn the_captured_refusal_gets_past_the_banner_before_refusing() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-measured-boot-refused.log"
        ));
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(progress.failure(), Some(&Failure::MeasuredBootRefused));
        assert!(
            !progress.userspace_ran(),
            "it halted rather than handing the archive to init"
        );
        // The wording is load-bearing: this is the gate working, and a message that read like a
        // crash would send somebody debugging the boot mechanism instead of their build.
        let said = progress.failure().unwrap().describe();
        assert!(said.contains("the gate working"));
        assert!(said.contains("script/board-image"));
    }

    /// The other successful capture, and the difference between the two: this one had an archive
    /// on the card, so `init` built a child and a userspace driver came up.
    #[test]
    fn the_captured_userspace_boot_ran_a_child() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-userspace.log"
        ));
        assert_eq!(progress.reached(), Stage::Tour);
        assert_eq!(progress.failure(), None);
        assert!(progress.userspace_ran());
    }

    /// **The soak stage and its numbers, from a real QEMU riscv64 run** (milestone 219). Fed one
    /// byte at a time by `run`, which is the case that catches a field parser depending on chunk
    /// boundaries.
    #[test]
    fn the_captured_soak_reaches_the_soak_stage_and_its_last_beat_is_read() {
        // A pre-297 capture, respelled at read time and not on disk; see
        // `crate::respell_pre_297_markers` for why the file is left as the machine printed it.
        let progress = run(&crate::respell_pre_297_markers(include_str!(
            "../tests/fixtures/captured/qemu-2026-09-01-riscv64-soak.log"
        )));
        assert_eq!(progress.reached(), Stage::Soak);
        assert!(
            progress.reached() > Stage::Tour,
            "a soak is a stage past the tour, which is what re-arms the quiet check"
        );
        assert_eq!(progress.failure(), None);
        let beat = progress.soak().expect("the heartbeats must be parsed");
        assert_eq!(beat.beat, 5);
        assert_eq!(beat.seconds, 25);
        assert_eq!(beat.rounds, 595_432);
        assert_eq!(beat.rate, 24_160);
        assert_eq!(beat.refused, 0);
        assert_eq!(beat.crossings, 21);
        // The fixture is a pre-221 capture and has no `wakes=` field, so the parser leaves it at
        // zero. That is the case the summary keys on to tell "nothing made it cross" apart from
        // "something did and it still did not".
        assert_eq!(beat.wakes, 0);
    }

    /// **The same parse against the marker vocabulary the kernel prints today** (milestone 297).
    ///
    /// `qemu-2026-09-14-riscv64-soak-test.log` is `script/soak-test --arch riscv64 --for 30s`, taken
    /// unedited on the day the rename landed. It exists so that the live spelling is proved against
    /// a machine rather than only against text this project wrote, which is the standard
    /// `tests/fixtures/README.md` holds every other marker to. It is also post-221, so it carries
    /// the `wakes=` field the capture above predates.
    #[test]
    fn the_renamed_markers_are_read_from_a_real_run_of_the_renamed_command() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/qemu-2026-09-14-riscv64-soak-test.log"
        ));
        assert_eq!(progress.reached(), Stage::Soak);
        assert_eq!(progress.failure(), None);
        let beat = progress.soak().expect("the heartbeats must be parsed");
        assert_eq!(beat.beat, 5);
        assert_eq!(beat.seconds, 25);
        assert_eq!(beat.rounds, 274_744);
        assert_eq!(beat.rate, 11_014);
        assert_eq!(beat.refused, 0);
        assert_eq!(beat.mismatches, 0);
        assert_eq!(beat.stalled, 0);
        // The tick route is live in this build, so unlike the pre-221 capture above this one both
        // wakes and crosses. Neither number is a result: see notes/soak.md on what a soak rate is
        // worth under emulation.
        assert_eq!(beat.wakes, 10_032);
        assert_eq!(beat.crossings, 2_717);
    }

    /// **A soak failure is a failure, not a stage.** The kernel prints `soak-test: FAILED` and then
    /// panics, and it is the panic the recogniser names, carrying the reason; the `FAILED` line
    /// must not be mistaken for a heartbeat or for the workload starting.
    #[test]
    fn a_soak_failure_is_reported_as_the_panic_it_becomes() {
        let progress = run(concat!(
            "soak-test: started 4 groups of one responder and 3 callers (20 user threads)\n",
            "soak-test: t=5s beat=1 rounds=100 rate=20/s workers=20 refused=1 mismatch=0 stalled=0\n",
            "soak-test: FAILED at t=5s beat=1: the wake gate refused a wake\n",
            "[PANIC] soak failed at t=5s beat=1\n",
        ));
        assert_eq!(progress.reached(), Stage::Soak);
        assert_eq!(
            progress.failure(),
            Some(&Failure::KernelPanic(
                "soak failed at t=5s beat=1".to_string()
            ))
        );
        assert_eq!(progress.soak().map(|b| b.refused), Some(1));
    }

    /// **The placement census must be invisible to this recogniser** (milestone 240).
    ///
    /// `kernel/src/soak.rs` prints a block of `soak-test-census:` lines at soak start and again whenever
    /// the arrangement changes. The two agree by that prefix being outside both substrings this
    /// file matches on, and that is an argument until something checks it: these lines are verbatim
    /// from an aarch64 QEMU run on 2026-09-03, interleaved exactly as the kernel emits them.
    ///
    /// Three claims. The census does not advance the stage on its own, the start line still does,
    /// and a beat carrying the new `drifted=` field still parses field for field, `t=` included,
    /// which is the one a census token could plausibly have collided with.
    #[test]
    fn the_placement_census_changes_nothing_the_recogniser_reads() {
        let progress = run(concat!(
            "soak-test-census: core=0 threads=10 R0 C0 C0 C0 G0 R2 C2 C2 C2 G3\n",
            "soak-test: started 4 groups of one responder, 3 callers, 1 grinder and 1 tick waiter \
             (24 user threads) on 4 online core(s), beating every 5s\n",
            "soak-test-census: where the kernel placed each worker at spawn: R=responder, C=caller, \
             G=grinder, W=tick waiter, and the number after each letter is its group\n",
            "soak-test-census: core=1 threads=7 W0 W1 W2 R3 C3 C3 C3\n",
            "soak-test-census: core=2 threads=3 G1 G2 W3\n",
            "soak-test: t=10s beat=2 rounds=314048 rate=35576/s wakes=3747 wakerate=386/s workers=24 \
             refused=0 mismatch=0 stalled=0 drifted=0 crossings=1849 remote=2165 steals=4 \
             deferred=6\n",
        ));
        assert_eq!(progress.reached(), Stage::Soak);
        assert_eq!(progress.failure(), None);
        let beat = progress.soak().expect("the heartbeat must still parse");
        assert_eq!(beat.seconds, 10);
        assert_eq!(beat.beat, 2);
        assert_eq!(beat.rounds, 314_048);
        assert_eq!(beat.rate, 35_576);
        assert_eq!(beat.crossings, 1849);
        assert_eq!(beat.remote, 2165);
    }

    /// A census line that arrives BEFORE anything else must not reach the soak stage by itself.
    ///
    /// Separate from the test above because that one proves the start line still works, and this
    /// one proves the census does not stand in for it: a log truncated to census lines has not seen
    /// a soak start, and reporting one would make `board_console` claim a stage from an artefact.
    #[test]
    fn a_census_alone_does_not_reach_the_soak_stage() {
        let progress = run("soak-test-census: core=0 threads=6 R0 C0 C0 C0 G0 W0\n");
        assert_eq!(progress.reached(), Stage::Cold);
        assert_eq!(progress.soak(), None);
    }

    #[test]
    fn a_stale_card_is_named_at_u_boot() {
        let progress = run(include_str!(
            "../tests/fixtures/synthetic/vf2-bad-magic.log"
        ));
        assert_eq!(progress.reached(), rung("uboot"));
        assert_eq!(
            progress.failure(),
            Some(&Failure::FirmwareRefused {
                diagnosis: "U-Boot rejected the image header (Bad Linux RISCV Image magic!)",
                reason: String::new(),
            })
        );
    }

    #[test]
    fn a_panic_carries_its_message() {
        let progress = run(
            "Starting kernel ...\n\nnife on RISC-V (rv64, S-mode, Sv39)\n[PANIC] hart 3 took a load fault\n",
        );
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(
            progress.failure(),
            Some(&Failure::KernelPanic(
                "hart 3 took a load fault".to_string()
            ))
        );
    }

    /// The distinction the whole recogniser turns on, and the one a `contains("U-Boot")` gets
    /// wrong: SPL's banner must not be read as U-Boot proper, or a board that dies in SPL is
    /// reported two stages further along than it got.
    #[test]
    fn spl_is_not_u_boot_proper() {
        let progress = run("U-Boot SPL 2021.10 (Feb 12 2023 - 20:24:34 +0800)\n");
        assert_eq!(progress.reached(), rung("spl"));
    }

    #[test]
    fn u_boot_proper_is_recognised_by_its_version() {
        let progress =
            run("U-Boot 2021.10 (Feb 12 2023 - 20:24:34 +0800), Build: jenkins-github\n");
        assert_eq!(progress.reached(), rung("uboot"));
    }

    /// The prompt has no newline after it. A feeder that only reported complete lines would sit
    /// there while U-Boot sat waiting for a command.
    #[test]
    fn the_prompt_is_seen_without_a_newline() {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::default();
        let feeding = feeder.feed(b"Hit any key to stop autoboot:  0 \nStarFive # ");
        for line in &feeding.lines {
            progress.observe_line(line);
        }
        progress.observe_partial(&feeding.tail);
        assert_eq!(progress.reached(), rung("uboot"));
        assert_eq!(feeding.tail, "StarFive # ");
    }

    /// A wrong baud rate is bytes, not text. The tool must keep going and log them.
    #[test]
    fn garbage_bytes_do_not_stop_the_feeder() {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::default();
        let feeding = feeder.feed(&[0xff, 0xfe, 0x80, b'\n', 0xc0]);
        for line in &feeding.lines {
            progress.observe_line(line);
        }
        progress.observe_partial(&feeding.tail);
        assert_eq!(progress.reached(), Stage::Cold);
        assert_eq!(feeding.lines.len(), 1);
    }

    /// A marker split across two reads is the normal case at 115200, not an edge one.
    #[test]
    fn a_marker_split_across_chunks_is_still_seen() {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::default();
        for chunk in [&b"Starting ke"[..], &b"rnel ...\n"[..]] {
            let feeding = feeder.feed(chunk);
            for line in &feeding.lines {
                progress.observe_line(line);
            }
            progress.observe_partial(&feeding.tail);
        }
        assert_eq!(progress.reached(), rung("handoff"));
    }

    /// The bug the byte-at-a-time feeding found, kept as its own test because reasoning did not
    /// find it: mid-word, `U-Boot ` reads as U-Boot proper, and a payload captured from a tail is
    /// captured truncated.
    #[test]
    fn a_partial_line_does_not_settle_a_word_or_capture_text() {
        let mut progress = BootProgress::default();
        progress.observe_partial("U-Boot ");
        assert_eq!(progress.reached(), Stage::Cold);
        progress.observe_partial("U-Boot SPL 2021.10 (Feb");
        assert_eq!(progress.reached(), rung("spl"));

        let mut progress = BootProgress::default();
        progress.observe_partial("nife on ");
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(progress.banner_line(), None);
        progress.observe_line("nife on RISC-V (rv64, S-mode, Sv39)");
        assert_eq!(
            progress.banner_line(),
            Some("nife on RISC-V (rv64, S-mode, Sv39)")
        );

        let mut progress = BootProgress::default();
        progress.observe_partial("[PANIC] hart 3 took");
        assert_eq!(progress.failure(), None);
        progress.observe_line("[PANIC] hart 3 took a load fault");
        assert_eq!(
            progress.failure(),
            Some(&Failure::KernelPanic(
                "hart 3 took a load fault".to_string()
            ))
        );
    }

    #[test]
    fn the_ratchet_never_goes_backwards() {
        let mut progress = BootProgress::default();
        progress.observe_line("Starting kernel ...");
        progress.observe_line("U-Boot SPL 2021.10");
        assert_eq!(progress.reached(), rung("handoff"));
    }

    /// A carriage return alone ends a line: firmware progress indicators use it to overwrite.
    #[test]
    fn a_bare_carriage_return_ends_a_line() {
        let mut feeder = LineFeeder::new();
        let feeding =
            feeder.feed(b"Hit any key to stop autoboot:  2 \rHit any key to stop autoboot:  1 \r");
        assert_eq!(feeding.lines.len(), 2);
        assert_eq!(feeding.tail, "");
    }

    /// **The sweep, from a real run of `--features job_mix`** (milestone 324 part 2), fed one byte
    /// at a time. This is the test that says the markers are the text a kernel prints: the fixture
    /// is `helpers/qemu-runner-aarch64.sh` on 2026-09-19, unedited, CRLF and all.
    ///
    /// **And it is the test that says the *fields* are, which it did not say before.** The capture
    /// it read until 2026-09-19 was taken from a pre-milestone-168 kernel, so it agreed with a
    /// parser that had stopped agreeing with the kernel; both were green and both were wrong. This
    /// one was taken from the merged tree, which is the whole of why it was retaken. See the
    /// module's `BUGS`.
    #[test]
    fn the_captured_sweep_runs_to_its_done_line() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/qemu-2026-09-19-aarch64-job-mix-medians.log"
        ));
        assert_eq!(progress.reached(), Stage::SweepDone);
        assert_eq!(progress.failure(), None);
        // The whole boot ladder is underneath it, which is what makes the sweep a stage past the
        // tour rather than a thing beside it.
        assert_eq!(
            progress.self_test_line(),
            Some("nife self-test: 5 of 5 passed")
        );
        let point = progress.sweep_point().expect("the points must be parsed");
        assert_eq!(
            *point,
            SweepPoint {
                tasks: 32,
                jobs: 4096,
                repeats: 21,
                ticks_min: 228_108_401,
                ticks_median: 233_958_411,
                ticks_max: 249_234_771,
                jpm_median: 65_652,
            },
            "the last point is the top of job_mix::TASK_SWEEP"
        );
        assert_eq!(
            u64::from(u32::try_from(job_mix::MAX_TASKS).expect("the sweep tops out in a u32")),
            point.tasks,
            "the last point is job_mix::MAX_TASKS, and the two agree by the kernel reading it"
        );
        assert_eq!(point.jobs, point.tasks * job_mix::JOBS_PER_TASK);
        // **The field the parse is most likely to be silently wrong about.** Every other number
        // here would survive a parser that read nothing, because `Default` is zero and a stale
        // expectation is also a number; this one is `job_mix::REPEATS` and the kernel printed it,
        // so the two disagreeing means one of them moved.
        assert_eq!(
            point.repeats,
            job_mix::REPEATS as u64,
            "the kernel prints job_mix::REPEATS and this build reads the same constant"
        );
        assert!(
            point.ticks_min <= point.ticks_median && point.ticks_median <= point.ticks_max,
            "the three ticks fields are a sorted spread, and reading them out of order is how a \
             near-miss parse would look"
        );
        let subrun = progress.sweep_subrun().expect("the subruns must be parsed");
        assert_eq!(
            *subrun,
            SweepSubrun {
                tasks: 32,
                repeat: 20,
                ticks: 233_743_670,
            },
            "the last subrun is the last repeat of the last point, and it is NOT the median one"
        );
        // These numbers are a draw rather than a result: TCG models no cache, and milestone 240's
        // census says the placement lottery decides throughput by up to fifteenfold on real
        // silicon. See notes/job-mix.md. What is asserted here is the parse, not the figure.
    }

    /// **The whole of part 2, in one pair of assertions**: a sweep that stops partway is a
    /// different stage from one that finished, where before this milestone both ended as the clock
    /// running out and shared an exit status.
    ///
    /// The truncation is the real capture cut after the third point, which is the honest stand-in
    /// for a wedge: nothing was invented, and the bytes before the cut are bytes a machine printed.
    #[test]
    fn a_sweep_cut_off_partway_has_not_reached_its_done_line() {
        let whole =
            include_str!("../tests/fixtures/captured/qemu-2026-09-19-aarch64-job-mix-medians.log");
        let cut = whole
            .find("job-mix: tasks=8")
            .expect("the capture has a fourth point to cut before");
        let progress = run(&whole[..cut]);
        assert_eq!(progress.reached(), Stage::Sweep);
        assert!(
            progress.reached() < Stage::SweepDone,
            "a wedged sweep must not read as a finished one"
        );
        assert_eq!(progress.failure(), None, "a wedge announces nothing");
        assert_eq!(
            progress.sweep_point().map(|p| p.tasks),
            Some(4),
            "three points printed and the third was four tasks"
        );
    }

    /// The census must be invisible to the recogniser, the same claim
    /// `the_placement_census_changes_nothing_the_recogniser_reads` makes for the soak and for the
    /// same reason: `job_mix::CENSUS` is outside `job_mix::STARTED` and `job_mix::POINT`, and that
    /// is an argument until something checks it. The lines are verbatim from the capture.
    #[test]
    fn a_job_mix_census_alone_reaches_nothing() {
        let progress = run(concat!(
            "job-mix-census: where the kernel placed each thread at spawn: S=echo server, T=task\n",
            "job-mix-census: core=0 threads=7 T3 T8 T14 T15 T22 T24 T29\n",
            "job-mix-census: core=2 threads=10 S0 T2 T4 T9 T11 T17 T18 T20 T27 T28\n",
        ));
        assert_eq!(progress.reached(), Stage::Cold);
        assert_eq!(progress.sweep_point(), None);
        assert_eq!(progress.failure(), None);
    }

    /// **A refused sweep is a failure and not a stage**, milestone 268's item 5 applied to the
    /// workload: a kernel that would not start the sweep must not read like one that ran it.
    ///
    /// The line is built from `job_mix::FAILED` rather than written out, so this test cannot
    /// disagree with the kernel about the prefix; the tail is `kernel/src/job_mix.rs`'s own.
    #[test]
    fn a_refused_sweep_is_announced_with_its_reason() {
        let progress = run(&format!(
            "{}no 'job_mix_task' program in the initrd archive; nothing to run\n",
            job_mix::FAILED
        ));
        assert_eq!(
            progress.failure(),
            Some(&Failure::SweepFailed(
                "no 'job_mix_task' program in the initrd archive; nothing to run".to_string()
            ))
        );
        assert_eq!(
            progress.reached(),
            Stage::Cold,
            "it refused before the sweep started"
        );
        let said = progress.failure().expect("it failed").describe();
        assert!(
            said.contains("No point of the sweep was measured"),
            "an empty log reads like a wedge until the report says nothing ran"
        );
    }

    /// The two sweep rungs are an order, and these are the comparisons the tools make.
    #[test]
    fn the_sweep_rungs_sit_above_the_boot_ladder() {
        assert!(Stage::Sweep > Stage::Prompt, "a sweep replaces the handoff");
        assert!(
            Stage::SweepDone > Stage::Sweep,
            "finishing outranks starting, which is the whole of part 2"
        );
    }

    /// A partial sweep line ratchets and captures nothing, which is `observe_partial`'s contract.
    /// The tail is offered again as it grows, so a counter here would double-count and a truncated
    /// `ticks=` would record a figure that has not finished arriving.
    #[test]
    fn a_partial_sweep_line_ratchets_but_does_not_capture() {
        let mut progress = BootProgress::default();
        progress.observe_partial("job-mix: started 32 tasks and 2 ser");
        assert_eq!(progress.reached(), Stage::Sweep);
        let mut progress = BootProgress::default();
        progress.observe_partial("job-mix: tasks=32 jobs=4096 repeats=21 ticks_min=2281");
        assert_eq!(
            progress.sweep_point(),
            None,
            "half a point is not a point, and 2281 is not 228108401"
        );
    }

    /// **The board profile's whole claim, checked against radon's own capture** (milestone 324 part
    /// 3): the firmware prologue is the profile's and the rest of the ladder is not.
    ///
    /// The same bytes, read twice. With radon's profile the boot climbs SPL, OpenSBI, U-Boot and
    /// the handoff on the way to the tour. With xenon's, whose prologue is empty, it reaches
    /// exactly the same tour and never reports a firmware rung, because none of those lines are
    /// xenon's to claim. A hard-coded prologue cannot tell those two readings apart, which is what
    /// the ruling meant by *the profile is the firmware prologue only*.
    #[test]
    fn the_same_capture_climbs_radons_prologue_and_not_xenons() {
        let log = include_str!("../tests/fixtures/captured/vf2-2026-09-01-manual-boot.log");

        let as_radon = feed(&board::RADON, log);
        assert_eq!(as_radon.reached(), Stage::Tour);
        assert!(as_radon.is_relocated(), "Moving Image from is U-Boot's");

        let as_xenon = feed(&board::XENON, log);
        assert_eq!(
            as_xenon.reached(),
            Stage::Tour,
            "the kernel's own ladder is shared and is read the same either way"
        );
        assert!(
            !as_xenon.is_relocated(),
            "a relocation note belongs to the firmware that printed it"
        );
        assert_eq!(as_xenon.board().name, "xenon");
    }

    /// **xenon's prologue is empty because its capture is**, which is part 3's premise rather than
    /// an omission. Milestone 324's block says a `--replay` of this file reports the banner, the
    /// machine line, the five-of-five verdict and the measured-boot refusal; this is that claim as
    /// an assertion, read through the profile that says xenon has no firmware rungs at all.
    ///
    /// The file is `bench/` rather than `tests/fixtures/` on purpose: it is calef's bench record of
    /// first light on that machine, and a copy here would be a second one to keep in step.
    #[test]
    fn xenons_capture_needs_no_prologue_to_be_read_correctly() {
        let progress = feed(
            &board::XENON,
            include_str!("../../../bench/xenon-2026-09-17/first-light-095500.log"),
        );
        assert_eq!(
            progress.banner_line(),
            Some("nife on x86_64 (long mode, ring 0, 4-level paging)")
        );
        assert_eq!(
            progress.machine_line(),
            Some("nife machine: x86_64, 4 processor(s), 17119 MiB, 100 Hz")
        );
        assert_eq!(
            progress.self_test_line(),
            Some("nife self-test: 5 of 5 passed")
        );
        assert_eq!(progress.failure(), Some(&Failure::MeasuredBootRefused));
        assert_eq!(
            progress.reached(),
            Stage::SelfTest,
            "it halted at the trust boundary, after the verdict and before any handoff"
        );
    }

    /// `--until spl` against a board with no SPL is a request that can never be satisfied, and
    /// answering it with a refusal beats watching for two minutes and reporting that time ran out.
    /// `xtask` is where the refusal is printed; this is the lookup it rests on.
    #[test]
    fn a_firmware_rung_is_only_offered_by_a_board_that_has_one() {
        assert!(board::RADON.rung("spl").is_some());
        assert!(board::XENON.rung("spl").is_none());
    }

    /// Every stage's label, which every report reads through [`fmt::Display`] rather than
    /// through this function directly, so nothing else in the suite pins the exact wording down.
    #[test]
    fn every_stage_has_its_own_label() {
        assert_eq!(Stage::Cold.label(), "nothing recognisable");
        assert_eq!(Stage::Banner.label(), "kernel banner");
        assert_eq!(Stage::Machine.label(), "machine described");
        assert_eq!(Stage::SelfTest.label(), "self-test verdict");
        assert_eq!(Stage::Tour.label(), "boot tour complete");
        assert_eq!(Stage::Prompt.label(), "shell prompt");
        assert_eq!(Stage::Soak.label(), "soak running");
        assert_eq!(Stage::Sweep.label(), "job-mix sweep running");
        assert_eq!(Stage::SweepDone.label(), "job-mix sweep complete");
        let rung = board::RADON.rung("spl").expect("radon has an spl rung");
        assert_eq!(Stage::Firmware(rung).label(), "U-Boot SPL");
    }

    /// `Display` writes the label into the formatter; a mutant that turned this into a no-op
    /// would still return `Ok`, and only reading what actually landed in the string catches it.
    #[test]
    fn displaying_a_stage_writes_its_label() {
        assert_eq!(Stage::Tour.to_string(), "boot tour complete");
    }

    /// `Failure::describe`'s one match guard, both ways: with a reason, the diagnosis and the
    /// reason both appear; without one, only the diagnosis does.
    #[test]
    fn firmware_refused_describes_with_and_without_a_reason() {
        let with_reason = Failure::FirmwareRefused {
            diagnosis: "U-Boot gave up",
            reason: "the line before it".to_string(),
        };
        assert_eq!(with_reason.describe(), "U-Boot gave up: the line before it");

        let without_reason = Failure::FirmwareRefused {
            diagnosis: "U-Boot gave up",
            reason: String::new(),
        };
        assert_eq!(without_reason.describe(), "U-Boot gave up");
    }

    /// [`LineFeeder::tail`] reads back exactly the bytes still pending, which is the only thing
    /// this accessor promises.
    #[test]
    fn line_feeder_tail_reads_back_the_pending_bytes() {
        let mut feeder = LineFeeder::new();
        let feeding = feeder.feed(b"a partial line, no terminator yet");
        assert_eq!(feeding.tail, "a partial line, no terminator yet");
        assert_eq!(feeder.tail(), "a partial line, no terminator yet");
    }
}
