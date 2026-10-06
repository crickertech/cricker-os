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
//! | 2 | [`CHILD_REPORT_SLOT`] | `WRITE`, `GRANT` | handed to each job as its slot 0, when placed; a durable session places none |
//! | 3 | [`DEATHS_SLOT`] | `READ`, `GRANT` | each job's supervision endpoint, and what corpses are reaped through |
//! | 4 | [`CLOCK_SLOT`] | `READ`, `GRANT`, optional | the clock page (§43 (reading the clock is a page)), also mapped read-only at [`CLOCK_VA`] |
//! | 5 | [`ACTIVATION_SLOT`] | `WRITE` | the store's `activation/` directory, read-only; store mode only |
//! | 6 | [`PACKAGES_SLOT`] | `WRITE` | the store's `packages/` directory, read-only; store mode only |
//!
//! Slot 4 is the one optional grant. With it, `Held::clock` is true: calendar entries can keep a
//! time of day, and a job whose manifest declares a clock gets the page too, read-only, at its own
//! slot 1 and [`CLOCK_VA`], as the progenitor gives `date` one. Without it, every calendar line is
//! `Unbacked::WallClock`. The spawn site must map the page before the timetable starts, and the
//! timetable probes the slot at `_start` for `swish`'s reason: later, a region it split could land
//! there.
//!
//! **Store mode** (milestone 152 (durable delegation), Fork 8 ruled D by calef on 2026-09-27, on
//! #1377) is a timetable holding [`ACTIVATION_SLOT`]. It is handed no archive (`a1` is 0): each job's
//! program is what a bare word runs at the prompt, the live activation generation's entry for the
//! name, resolved when a document is registered and checked again at every fire. The two slots are
//! endpoints to caretakers that serve those directories read-only, over a file-service channel of
//! the timetable's own whose page the spawn site maps at [`STORE_PAGE_VA`]. A registrar is
//! required. They were 4 and 5 until the clock took slot 4 on `main` (milestone 129 (scheduled
//! execution)); moved at the merge of 2026-10-03 (UTC). A durable session grants no clock yet, so
//! its calendar lines are `Unbacked::WallClock` (`components/src/user_timetable_keeper.rs`'s BUGS).
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
//! its exit code at [`crate::registration::EXIT`]. Slot 0 may be left empty, and so may slot 2: a
//! timetable that holds no report endpoint hands its jobs none (`crate::Held::report`). A durable
//! session leaves it empty, which is Fork 6 C of milestone 152 (durable delegation), ruled 2026-09-27
//! on #1377.
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

/// The clock page's slot, when the timetable is granted one.
pub const CLOCK_SLOT: u64 = 4;
/// Where the clock page is mapped, read-only: in the timetable, and in each job that declares a
/// clock. The address `date` reads it at, `system_initializer`'s `CHILD_CLOCK_VA`.
pub const CLOCK_VA: u64 = 0x00c0_0000;
/// Store mode's `activation/` directory: an endpoint to a caretaker serving it read-only.
pub const ACTIVATION_SLOT: u64 = 5;
/// Store mode's `packages/` directory, likewise.
pub const PACKAGES_SLOT: u64 = 6;
/// **Where a store-mode timetable finds its file-service channel's page**, mapped read/write by the
/// spawn site. The page is shared with the two caretakers and the file service, and with nothing
/// else: a channel of its own, because a job fires while its user may be using the store at the
/// prompt, and two clients staging bytes in one page overwrite each other (milestone 599 (a frame
/// per filesystem client channel)).
pub const STORE_PAGE_VA: u64 = 0x0610_0000;
/// **The largest program a store-mode timetable loads**, in bytes: its staging buffer. A job's
/// segments, stack and tables must fit an instance's 48 pages (192 KiB), and this bounds the file,
/// which carries a symbol table the instance never maps.
///
/// **Raised 64 -> 128 KiB on 2026-10-02 (UTC), a correction.** This used to say 64 KiB was never
/// the binding limit. The machine said otherwise: the suite's installed fixture,
/// `least_authority_demo`'s bytes as the archive packs them (`--strip-debug`, so the symbol table
/// stays), is 71,664 bytes on aarch64 and 12,184 on riscv64. Store mode refused it on aarch64 as
/// `registration::STATUS_NO_IMAGE`, so `login`'s start-up pass ended corinne's re-derived session
/// at once and `a_durable_session_is_re_derived_at_start_up_unless_suspended` failed there while
/// riscv64 passed. 128 KiB is still under an instance, and costs 16 more pages of `.bss`
/// (`user_timetable_keeper.rs`'s `TIMETABLE_REGION_PAGES`).
pub const STAGING_BYTES: usize = 128 << 10;

/// Which start argument carries the fire count.
pub const ARG_FIRES: usize = 0;
/// Which start argument carries the archive's length.
pub const ARG_ARCHIVE_LEN: usize = 1;
/// Which start argument carries the registration page's address.
pub const ARG_REGISTRATION_PAGE: usize = 2;

/// Stack pages a timetable needs. Its working set is the plan, a kilobyte per entry, and a
/// replacement holds two plans at once; eight pages died with a stack overflow in 2026-08.
///
/// **Raised 32 -> 48 on 2026-09-30**, when the store-mode registration chain overflowed 32 pages on
/// both ISAs after `main` grew `grant_plan`'s planning path underneath this branch (milestone 205
/// (how a foreign program is told what to do)'s designate and stage planning). The debug-build
/// frames, measured with `-Z emit-stack-sizes`:
/// `replace_if_asked` 48.0 KiB, `Registry::register` 21.2 KiB, `_start` 20.0 KiB,
/// `plan_against_with` 13.4 KiB, the resolve closures 11.2 and 10.0 KiB, `register_installed`
/// 9.8 KiB, `admit_installed` 5.6 KiB, and the store's `resolve_installed` -> `load_current` ->
/// `store::load` another 11.0 KiB, which chains to about 125 KiB against the 128 KiB 32 pages gave.
/// 48 pages is that with roughly half again; the growth was `main`'s, not this crate's, and the
/// bound is a size rather than an assertion, so raising it is the fix rather than a workaround.
pub const STACK_PAGES: u64 = 48;

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
/// It holds the store and no registration page, which store mode requires.
pub const E_STORE: u64 = 0xE305;
