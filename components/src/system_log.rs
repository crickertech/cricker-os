//! **The system log service** (milestone 613 (a system log service: the in-memory half), for §242
//! (a system log)): one place the programs on a machine append to, and that readers read from.
//!
//! Everything it decides is `crates/system_log`, host-tested; this file is the receive loop and
//! the three syscalls around it. One thread, one wait point:
//!
//! 1. `RECV` on the intake endpoint, which returns the three words a sender passed and the badge
//!    the kernel read off the capability it sent through (milestone 613's amendment to §230
//!    (badged endpoint capabilities): a plain `SEND` carries its badge too).
//! 2. Hand them to [`system_log::Log::handle`]: a writer's bytes become stamped lines in the ring;
//!    a badge-0 word is the spawner registering a badge; a reader's request names a window.
//! 3. For a read, fill that reader's window and `SIGNAL` its notification. Signalling never blocks,
//!    so a reader that stops reading cannot hold this thread, and a writer never waits on a reader.
//!
//! # Capability contract
//!
//! | slot | what | rights |
//! |---|---|---|
//! | 0 | the intake endpoint: writers, readers and the spawner all `SEND` here | `READ` |
//! | 1 to [`WINDOWS`] | reader *n*'s notification, signalled when its window is filled | `WRITE` |
//!
//! Window *n* (0-based) is the page at `read::WINDOW_VA + n * 4096`, mapped read-write, which the
//! same spawner maps into reader *n*. A window past the count is never written, and a missing
//! notification slot is a reader nobody can wake: its window still holds the answer.
//!
//! The spawner holds the only unbadged `WRITE` capability to slot 0's endpoint, and mints a badged
//! copy (`abi::rendezvous::BADGE`) for each writer and reader. That is what makes attribution
//! unforgeable: every writer is somebody's badge, and only badge 0 may say whose.
//!
//! # EXAMPLES
//!
//! `system_tests/src/user/system_log_tests.rs` plays the spawner and the reader:
//!
//! ```text
//! register writer badge 1 as program "sink_transcript_writer", user "alice"
//! register writer badge 2 as program "sink_transcript_writer", user "bob"
//! register badge 9 as the system reader, window 0
//! spawn two writers, each holding only its badged capability; they print the transcript
//! SEND (OP_READ, cursor 0) through badge 9, wait on window 0's notification, read the window
//! ```
//!
//! # BUGS
//!
//! - **Nothing starts it at boot yet.** The progenitor does not spawn this service or mint log
//!   badges for its children; milestone 342 (the kernel and the `console` server drive one UART
//!   from two address spaces) is the first customer that needs it running, and wires it then.
//! - **It does not yet forward kernel lines to the console.** §242 makes this service the one
//!   writer of kernel output on the console, but the kernel's ring and the drain that would feed it
//!   are milestone 342's. [`system_log::Log::ingest`] is the entry point that work will use.
//! - **Four readers at most** (`system_log_protocol::read::WINDOWS`), because each is a page and
//!   a notification the spawner provides at spawn.
//! - **Its stack depth is measured only on the paths the QEMU test drives.** A program the kernel
//!   spawns gets one 4 KiB stack page, and this one is built unoptimized in a debug image. The
//!   first version overflowed it by moving 250-byte lines by value; `crates/system_log` now lends
//!   them. The crowded-writer path (a ninth writer mid-line) runs on the host only.
//!
//! Name: provisional (milestone 613's lane, 2026-10-02 UTC). §242 calls it "the log service"; the
//! crate and program share `system_log` so a reader finds both with one grep.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates: one
// lint policy)'s ratchet tracks (§107 (`missing_docs` moves to `workspace.lints.rust`)): each
// `[[bin]]` is its own crate root with one `_start`, and documenting an OS-facing ABI entry point
// is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use system_log::{Handled, Log};
use system_log_protocol::read;
use user_mode_runtime::{exit, monotonic_nanos, notification_signal, recv_badged};

/// The intake endpoint, `READ`.
const INTAKE: u64 = 0;

/// How many reader windows the service maps, from the contract both sides link.
const WINDOWS: u64 = read::WINDOWS as u64;

/// The first reader window, checked against the address-space map at compile time.
const WINDOW_VA: u64 = address_space_map::pair_page(read::WINDOW_VA);

/// The whole service's state, in `.bss` rather than on the one-page stack: the 64 KiB ring is
/// sixteen times the stack.
static mut LOG: Log = Log::new();

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    // SAFETY: this program has one thread, and `_start` is the only code that names `LOG`, once,
    // here; every later use goes through this one reference.
    let log = unsafe { &mut *core::ptr::addr_of_mut!(LOG) };
    loop {
        let (w0, w1, w2, badge) = recv_badged(INTAKE);
        // The intake endpoint was destroyed (its owner reclaimed the region it lived in): this
        // service has nothing left to serve. Only badge 0 can mean it, because the kernel writes
        // `Gone` with no badge, while a writer forging the word arrives with its own nonzero one.
        if badge == 0 && w0 as i64 == abi::Error::Gone as i64 {
            exit();
        }
        let Handled::Read { window, cursor } = log.handle(badge, w0, w1, w2, monotonic_nanos())
        else {
            continue;
        };
        if window as u64 >= WINDOWS {
            continue;
        }
        let va = WINDOW_VA + window as u64 * read::WINDOW_BYTES as u64;
        // SAFETY: windows 0 to WINDOWS-1 are pages the spawner mapped read-write at these
        // addresses before this thread started (the capability contract above). A reader shares
        // the page, so nothing here trusts what it reads from it; it only writes.
        let page = unsafe { core::slice::from_raw_parts_mut(va as *mut u8, read::WINDOW_BYTES) };
        log.fill(badge, cursor, page);
        // A refused signal (a slot the spawner left empty) is a reader nobody can wake; the
        // window holds the answer for whenever it next looks.
        let _ = notification_signal(1 + window as u64, 1);
    }
}

user_mode_runtime::panic_handler!();
