//! **Everything a network time client holds, and the clock page is still not reachable**
//! (milestone 51, split out of the client's binary by milestone 290; notes/ntp.md).
//!
//! Spawned with `components/src/network_time_client.rs`'s five slots and told the address at which
//! a process holding the *set* authority maps the clock page. It reports the address, writes there,
//! and faults, because its address space has no mapping of that frame at any address. **The
//! boundary is the mapping, not the layout**, and knowing where to look buys nothing.
//!
//! This is the claim Unix cannot make. `ntpd` runs as root: there is no address in a Unix system
//! its `settimeofday` cannot reach. `an_ntp_client_holds_no_writable_clock_page` in
//! `system_tests/src/user/ntp_tests.rs` is the assertion; this program is the thing it asserts about.
//!
//! # Why this can be its own binary, and what actually keeps it honest
//!
//! Until milestone 290 this was a role of the client's binary, and the client's header said the
//! proof rested on it being *the same binary*. **That argument is false, and it is worth recording
//! because it was believed.** The fault is caused by the **capability set**: any process holding
//! that endowment faults at that address whatever code it runs, and no amount of shared machine
//! code would make a stale capability list fault.
//!
//! What keeps this witness honest is that it and the client are endowed by **one function**,
//! `spawn_with_client_endowment` in `kernel/src/user/ntp_service.rs`, which takes the image as a
//! parameter. A sixth slot added to the client is a sixth slot this program gets, and there is no
//! second capability list anywhere to forget to update. **If you are ever about to write one, stop**:
//! a hand-maintained second copy of a fact the wiring already holds is how a security claim decays
//! without breaking a build, which is milestone 117's `swish` `caps` bug.
//!
//! # Reaching the second report is the failure
//!
//! It reports [`RPT_PROBING`] **before** the write. Anything after that means the write did not
//! fault, so the test asserts three things at once: that the user-fault counter rose, that the
//! faulting address was exactly the one it aimed at, and that **no second report is waiting**.
//!
//! The fault is a **translation** fault rather than a permission fault, because this process holds
//! no mapping of the clock page at all. The test deliberately does not assert the kind: the claim
//! is about the address a client aimed at and was stopped at, and pinning the kind would make the
//! test fail if the page were ever mapped read-only, which would be a *weaker* system passing a
//! stricter-looking assertion.
//!
//! # BUGS
//!
//! - **This proves an absence, so it can only ever be a witness to one wiring.** It shows that the
//!   client's endowment does not reach the clock page at the address the setter uses. It does not,
//!   and cannot, show there is no other page in the system that would let a client move time; that
//!   would be a proof over the whole capability graph, which nothing here performs.
//! - **It writes `clock_protocol::state::SET` because that is what a real attempt would write**, and
//!   nothing checks the value, since the write never lands. If the confinement ever broke, the
//!   clock page would be left holding a plausible state word rather than an obvious sentinel, and
//!   the test would catch it through `clock.page()` changing rather than through the value.
//!
//! Name: ratified 2026-09-14 (calef, milestone 290), ruled on being offered it against
//! `clock_page_write_attempt`. The register is this tree's existing fixture names
//! (`first_attempt_crasher`, `swap_boundary_witness`, `kernel_test_subject`), which say what the
//! fixture **proves** rather than what it does: what it does is one volatile store, and a reader
//! meeting `unwritable_clock_witness` in a boot log learns the property under test instead of the
//! instruction. Refused `clock_page_write_attempt` for that reason, and because an attempt is not a
//! thing, which is the noun rule one step further in. Refused keeping it as `ROLE_PROBE_CLOCK`
//! inside the client, on the "same binary" argument corrected above: the argument that looked
//! load-bearing was not, and the shape it justified put a test-only branch inside a component the
//! client's own header claimed had none. `network_time` is deliberately absent from this name: it
//! is not a time program, it is a witness to a mapping, and the endowment it borrows is named at
//! its one call site.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, send};

/// Slot 0: the report endpoint (WRITE). **The only slot this program uses**, and it is handed the
/// client's other four so that what it fails to reach, it fails to reach as a fully endowed network
/// time client rather than as a stripped-down one.
const REPORT: u64 = 0;

/// About to write the clock page at `w1`. The next thing this process does is fault; anything after
/// this report means it did not. One numbering space with the other two programs milestone 290 split
/// the `ntp` binary into, and the number did not move in that split.
pub const RPT_PROBING: u64 = 6;

/// `a0` is the address to write: where a process holding the *set* authority maps the clock page.
#[unsafe(no_mangle)]
pub extern "C" fn _start(va: u64) -> ! {
    send(REPORT, RPT_PROBING, va, 0);
    // SAFETY: deliberately not safe. This is the assertion: the write must fault. If it does not,
    // the process survives to send the report below, and the test fails on that.
    unsafe {
        core::ptr::write_volatile(va as *mut u64, clock_protocol::state::SET);
    }
    send(REPORT, RPT_PROBING, va, 1);
    exit()
}

user_mode_runtime::panic_handler!();
