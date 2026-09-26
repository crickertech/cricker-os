//! **How a timetable is spawned: its slots, its arguments, and how its document arrives**
//! (milestone 129 (scheduled execution), §222 (who holds a user's schedule)).
//!
//! Two kinds of spawner start a timetable: the boot-time test, which uses the compiled-in
//! document, and a durable session (milestone 152 (durable delegation)), which registers its
//! user's schedule. Both read this module rather than a copy of it (AGENTS.md rule 7).
//!
//! # Slots
//!
//! | slot | constant | rights | what |
//! |---|---|---|---|
//! | 0 | [`OUT_SLOT`] | `WRITE` | the plan and the summary, `byte_sink_protocol` bytes; compiled-in mode only |
//! | 1 | [`BUDGET_SLOT`] | `WRITE` | the untyped every instance is split from |
//! | 2 | [`CHILD_REPORT_SLOT`] | `WRITE`, `GRANT` | handed to each job as its slot 0 |
//! | 3 | [`DEATHS_SLOT`] | `READ`, `GRANT` | each job's supervision endpoint, and what corpses are reaped through |
//!
//! Nothing else. In particular never the run-unvouched capability
//! (`grant_plan::spawnproto::RUN_UNVOUCHED_SLOT`): a timetable holding it runs nothing and exits
//! with [`E_UNVOUCHED`], because a scheduled job must stay within reach of §220 (signed builds, and
//! trusting a key is scoped).
//!
//! # Arguments
//!
//! - `a0` ([`ARG_FIRES`]): how many fires before the timetable reports and exits; `0` is forever.
//! - `a1` ([`ARG_ARCHIVE_LEN`]): the length of the archive mapped read-only at
//!   `user_mode_runtime::initrd::INITRD_VA`, holding the programs its jobs may run.
//! - `a2` ([`ARG_REGISTRATION_PAGE`]): where a writable registration page is mapped, or `0`.
//!
//! # How the document arrives
//!
//! With `a2 == 0` the document is the compiled-in `components/timetable.conf`, and everything the
//! timetable says goes down [`OUT_SLOT`].
//!
//! With a page, the timetable starts with an empty document and the registrar sends the first one
//! with `REPLACE` (see [`crate::registration`]), typically the user's stored schedule. It is then
//! **silent on [`OUT_SLOT`]**: a session supervising it is blocked on supervision and cannot drain
//! a stream, and a `SEND` nobody takes would stop the timetable. Everything a registrar needs is in
//! the page: each reply's status, verdicts and printed plan, and, once the timetable has stopped,
//! its exit code at [`crate::registration::EXIT`]. Slot 0 may be left empty.
//!
//! Name: provisional, minted 2026-09-26 (UTC) by milestone 129's lane, for this module and every
//! constant here. Naming is calef's.

/// The output endpoint's slot.
pub const OUT_SLOT: u64 = 0;
/// The budget's slot.
pub const BUDGET_SLOT: u64 = 1;
/// The child report endpoint's slot.
pub const CHILD_REPORT_SLOT: u64 = 2;
/// The supervision endpoint's slot.
pub const DEATHS_SLOT: u64 = 3;

/// Which start argument carries the fire count.
pub const ARG_FIRES: usize = 0;
/// Which start argument carries the archive's length.
pub const ARG_ARCHIVE_LEN: usize = 1;
/// Which start argument carries the registration page's address.
pub const ARG_REGISTRATION_PAGE: usize = 2;

/// Stack pages a timetable needs. Its working set is the plan, a kilobyte per entry, and a
/// replacement holds two plans at once; eight pages died with a stack overflow in 2026-08.
pub const STACK_PAGES: u64 = 32;

/// Exit codes, the verdict word on [`OUT_SLOT`] or the page's exit word. A clean finish is `0`.
///
/// The document did not parse; the low byte is the line number.
pub const E_CONFIG: u64 = 0xE300;
/// The archive did not parse.
pub const E_ARCHIVE: u64 = 0xE301;
/// A program an admitted entry names is not in the archive.
pub const E_IMAGE: u64 = 0xE302;
/// The budget cannot back even one instance.
pub const E_BUDGET: u64 = 0xE303;
/// It was handed the run-unvouched capability, so it ran nothing.
pub const E_UNVOUCHED: u64 = 0xE304;
