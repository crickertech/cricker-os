//! The interrupt heeder: a long-running job that heeds the cooperative interrupt (DECISIONS §24).
//!
//! It works forever, one small unit at a time, and between units it reads a single word in a page
//! it shares with the shell: the interrupt flag. When the shell sets it (on the first `^C`), this
//! program cleans up, records that it stopped gracefully, and exits. This is the cooperative tier
//! of DECISIONS §24 made visible: the first `^C` asks the job to stop, and a job that listens does.
//!
//! Why a shared word and not an endpoint: a running computation cannot poll an endpoint (there is
//! no non-blocking receive) and cannot block on one without stalling the work the user wants to
//! interrupt. So the signal is memory this program reads with a plain load between units. See
//! `grant_plan::job_page_frame` and notes/grant-expression.md.
//!
//! # What it holds
//!
//! - the shared job frame, mapped read/write at [`JOB_PAGE_FRAME_VA`] (init maps it; the shell
//!   holds the other view). It reads [`job_page_frame::INTERRUPT`] and writes the rest. No
//!   capabilities: it touches only this page and exits. Its whole authority is one shared page.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist). Refused `heeder` (it never
//! said what it heeds, and the answer, §24's cooperative interrupt flag, was not in the name). Its
//! counterpart `spinner` became `interrupt_ignorer` under the same ruling, so the pair still reads
//! as a pair and both names parse the same way. Performed by milestone 175, which moved this file
//! into `fixtures/` in the same change rather than moving it twice.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use grant_plan::job_page_frame;
use user_mode_runtime::exit;
use user_mode_runtime::mapped_window::{self, MappedWindow};

/// Where init maps the shared job frame in the child's address space. Must match the shell/init
/// wiring. A pair page on the address-space map (`crates/address_space_map`).
const JOB_PAGE_FRAME_VA: usize = 0x0030_0000;

// SAFETY: init mapped one page read/write at JOB_PAGE_FRAME_VA before this program runs (milestone 139
// round 2; see `user_mode_runtime::mapped_window`, which is what collapsed the hand-rolled read_volatile/
// write_volatile pair below).
const WINDOW: MappedWindow =
    unsafe { MappedWindow::new(JOB_PAGE_FRAME_VA as u64, mapped_window::PAGE) };

fn load(off: usize) -> u64 {
    WINDOW.read(off as u64)
}

fn store(off: usize, v: u64) {
    WINDOW.write(off as u64, v);
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, _x1: u64, _x2: u64) -> ! {
    let mut beats: u64 = 0;
    loop {
        // One unit of work. The heartbeat is liveness the shell can show; the spin makes a unit
        // take long enough that the interrupt is noticed promptly, not after a million iterations.
        beats = beats.wrapping_add(1);
        store(job_page_frame::HEARTBEAT, beats);
        for _ in 0..4096 {
            core::hint::spin_loop();
        }
        // Notice the cooperative interrupt between units.
        if load(job_page_frame::INTERRUPT) != 0 {
            store(job_page_frame::STATUS, job_page_frame::STATUS_INTERRUPTED);
            store(job_page_frame::DONE, 1); // tell the shell we are stopping cleanly, then leave
            exit();
        }
    }
}

user_mode_runtime::panic_handler!();
