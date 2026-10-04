//! **Drains `login`'s audit trail so the service never blocks on it** (milestone 49's terminal
//! update, wiring the login stack into the real interactive boot).
//!
//! `components/src/login.rs` sends one [`login_protocol::ATTRIBUTED`] message per successful login on its
//! `AUDIT` endpoint, and that send is a plain, blocking rendezvous
//! (`crates/inter_process_communication`'s own model): it does not return until something receives
//! it. Nothing in a real interactive boot was reading that endpoint before this program existed, so
//! `login`'s very first successful login would have parked its whole thread inside that `send`
//! forever, unable to reclaim the connection it just served or accept the next one. This process
//! is the receiver, on `job_undertaker`'s own pattern: one endpoint capability, `READ`, and nothing
//! else.
//!
//! # What it does with a message once it has one
//!
//! Discards it. This is the honest, considered choice for this slice, not an oversight: printing
//! the record (who logged in, in order) would need a `WRITE` view of the terminal, and handing that
//! out to a *third* process at exactly the moment milestone 49's own terminal update is building
//! careful single-holder semantics for the interactive terminal (`components/src/login.rs`'s "The
//! terminal: single-session, deny cleanly") is a real complication this program's whole reason for
//! existing (unblocking `login`) does not need. See BUGS.
//!
//! # Capability contract
//!
//! - slot [`AUDIT`]: `READ` on `login`'s own `AUDIT` endpoint. Nothing else: this process cannot
//!   build anything, allocate a page, or reach any other component's memory.
//!
//! # BUGS
//!
//! **The audit trail this program drains is not surfaced anywhere.** DECISIONS §109 names two
//! properties: a server establishing a channel, and (separately) a server logging which channel a
//! later request arrived on. `login.rs`'s own BUGS already notes it only builds the first half; this
//! program is what makes the first half *safe to leave running* at a real boot, not a consumer of
//! the record it drains. A deployment that wants the attribution trail actually read (to a log file,
//! to the console, anywhere) needs a different, real consumer in this program's place, which is
//! follow-on work and not attempted here.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist), and performed 2026-09-24,
//! replacing `audit_sink`. Minted 2026-08-27 for the boot-wiring update of milestone 49 (users and
//! attribution) as `audit_sink`, a provisional name. Refused `audit_sink` (half the name was honest
//! and half was not: an audit trail that is discarded is not an audit trail, and a reader meeting
//! it in a process listing would reasonably conclude the system records logins somewhere; nothing
//! does). Refused `login_audit_discarder` (what the program does today and the first
//! recommendation, but it names the disposal rather than the role and would itself need renaming
//! the moment the records are kept). Refused `login_audit_sink` (it fixes whose audit it is and
//! keeps the promise that something is audited). Refused `audit_drain` (the same defect again, and
//! "drain" is spent in this tree on the merge-drain helper).
//!
//! `login_audit_receiver` survives the day the records are kept, and `login_audit_recorder` is the
//! honest successor. calef, ruling it: *"We would probably rename to `login_audit_recorder` when it
//! does record the login audit."* That is the condition written down rather than left to whoever
//! notices, the same shape §71 (a limitation is promoted when it stops being a fact) asks of a
//! `BUGS` entry that names what would promote it. Whoever gives this program a `WRITE` view and
//! makes it keep records performs that rename in the same change.
//!
//! Why the old name went, in more words. "Sink" is already this tree's word for the end of a stream
//! nobody reads further (`byte_sink_protocol`, `terminal_sink_caretaker`), and that is exactly this
//! program's role. The fault was the `audit` half, which is the fault `flaky` was renamed for the
//! same day: borrowed recognition the program contradicts. The structural-versus-current test this
//! ruling set is in `design/naming/vocabulary-rulings.md`.
//!
//! The first form of this block (2026-09-13 to 2026-09-24) wrote the four refusals as one run-on
//! paragraph, so the parse also recorded `login_audit_receiver` and `login_audit_recorder` as
//! refused. Neither was: one won and the other is the named successor. Rewritten with one refusal
//! per sentence at the rename, which moved the tree-wide refusal count by those two and by the one
//! `audit_sink` adds.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::receive;

/// `login`'s own `AUDIT` endpoint, `READ`.
const AUDIT: u64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    loop {
        receive(AUDIT);
    }
}

user_mode_runtime::panic_handler!();
