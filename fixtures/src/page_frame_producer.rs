//! **The frame demo, the half that shares**, milestone 19.
//!
//! Retypes a page out of its own memory region into a `PageFrame` capability, maps it read/write,
//! writes [`capability_witness_protocol::PAGE_FRAME_SENTINEL`], and hands the consumer a READ-only view
//! of the *same physical page*. The kernel never copies the data and was never told these two
//! processes would share memory: they composed the sharing themselves out of a capability.
//!
//! The rendezvous that carries the delegation is also the synchronization edge that makes the
//! write visible to the consumer, which is why the sentinel is written before the `SEND_CAP` and
//! not after.
//!
//! The other half is `page_frame_consumer`; the wiring is `kernel/src/user/page_frame_service.rs`.
//!
//! Name: provisional (milestone 291). This was `hello`'s `PAGE_FRAME_PRODUCER` role, number 11,
//! and the lowercase of the role constant is the name. Refused `frame_sharer`: `PageFrame` is the
//! kernel object's own name, and shortening it here would make the program harder to connect to
//! the capability it holds.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, map_page_frame, retype_page_frame, send_cap};

const MEMORY_REGION: u64 = 0; // retype the frame and draw page tables from here
const CHANNEL: u64 = 1; // delegate the frame to the consumer over here
const PAGE_FRAME_VA: u64 = address_space_map::pair_page(0x0000_0000_00A0_0000);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    // Retype: a page out of our budget becomes a PageFrame capability we hold. Nothing is mapped
    // yet.
    let frame = retype_page_frame(MEMORY_REGION);
    check(frame >= 0);

    // Map it read/write; the page tables to reach PAGE_FRAME_VA come from the same region.
    check(map_page_frame(
        frame as u64,
        PAGE_FRAME_VA,
        true,
        MEMORY_REGION,
    ));

    // Write the sentinel the consumer will read back through its own mapping of this page.
    // SAFETY: PAGE_FRAME_VA is now a mapped, writable page in our address space.
    unsafe {
        core::ptr::write_volatile(
            PAGE_FRAME_VA as *mut u64,
            capability_witness_protocol::PAGE_FRAME_SENTINEL,
        );
    }

    // Delegate a READ-only view: drop WRITE and GRANT on the way over.
    send_cap(CHANNEL, frame as u64, abi::rights::READ, 0);

    exit()
}

/// The only way this program can say "no": a trap, which the kernel treats as a fault and kills us
/// for. A failed check must be indistinguishable from a broken program, because it is one.
fn check(ok: bool) {
    if !ok {
        user_mode_runtime::trap()
    }
}

user_mode_runtime::panic_handler!();
