//! **Building another address space from EL0**, milestone 19b.
//!
//! Holds a memory region (slot 0) and a report line (slot 1). It retypes part of its own memory
//! into an address space, retypes a frame, maps the frame into the space it built, and proves the
//! kernel keeps the rules there too: the same virtual address twice is refused.
//!
//! Nothing can run in the built space (threads are 19c's object). What this witnesses is that a
//! process can *construct* one at all, out of memory it was handed, with the kernel allocating
//! nothing.
//!
//! The verdict is three bits: bit 0 the space was retyped, bit 1 the frame mapped into it, bit 2
//! the double map was refused. `kernel::user::tests` asserts `0b111` and prints the bit meanings
//! on failure.
//!
//! Name: ratified 2026-09-18 (calef, ruling on milestone 291's provisional fixture names), and
//! performed the same day. Coined by milestone 291's lane, from `hello`'s `ADDRESS_SPACE_BUILDER`
//! role (number 19), whose role constant lowercased was the name. Refused `address_space_builder`
//! and `builder`; the argument for each is below.
//!
//! **The ruling is that a fixture is named for what it proves, not for what it does.** This tree
//! already had one of those in `unwritable_clock_witness`, and this file's own prose reached for
//! the same word before anyone ruled: *what this witnesses is that a process can construct one at
//! all*. **Refused `address_space_builder`**, which the maintainer recommended on the grounds that
//! building *is* the claim here, since 19b asked whether a process can construct a space at all,
//! so the action and the proof coincide. calef ruled the other way: everything in `fixtures/`
//! proves something, and naming them for the proof is the scheme rather than the exception.
//!
//! **That makes a scheme question live for the fixtures named for their action** (`image_self_
//! checker`, `allocator_exerciser`, `os_primitives_benchmarker`). They are already on
//! `script/names --unratified`, so they will reach calef on their own; nothing here rules on them,
//! and this block should not be read as having done so.
//!
//! **The reason this name used to give had expired.** It said the qualifier existed because the
//! name was *"deliberately not `builder`, which is already a program in every archive (milestone
//! 20's richer-initrd demo)"*. Milestone 295 retired that program on 2026-09-14, on calef's own
//! ruling, so the collision the qualifier was avoiding no longer exists. The qualifier survives on
//! a different rule: `builder` alone is a **generic word that could name almost anything in an
//! operating system**, which is AGENTS.md's second naming failure mode. Recorded because a
//! justification that quietly stops being true is worse than one that was never written.
//!
//! **Performed** on 2026-09-18, across 44 occurrences in 18 files. What kept the old name, and
//! why: **five** `bench/radon-2026-09-16/jobmix-boot*.log` transcripts, which are machine output
//! and are evidence (the count was written as three here before the sweep enumerated them, which
//! is the reason the procedure says to enumerate rather than recall); the `BUILT` blocks in
//! milestones 291, 295 and 158, which are accounts under the name they were written under; a
//! captured `script/names --unratified` listing in `design/naming.md`; and the dated list of ten
//! corrected blocks in the `refusals-written-where-the-tool-cannot-read-them` proposal. `hello`'s
//! `ADDRESS_SPACE_BUILDER` role constant is a different name and was not touched.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, map_into, retype_object, retype_page_frame, send};

const MEMORY_REGION: u64 = 0;
const REPORT: u64 = 1;
const VA: u64 = address_space_map::pair_page(0x0040_0000);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    let aspace = retype_object(MEMORY_REGION, abi::objtype::ADDRESS_SPACE);
    let mut verdict = 0u64;
    if aspace >= 0 {
        verdict |= 1; // built a space out of our own pages
        let frame = retype_page_frame(MEMORY_REGION);
        if frame >= 0 {
            let mapped = map_into(aspace as u64, VA, frame as u64, 1);
            if mapped == 0 {
                verdict |= 2; // mapped our frame into the space we built
            }
            let again = map_into(aspace as u64, VA, frame as u64, 1);
            if again < 0 {
                verdict |= 4; // the same va twice was refused: break-before-make holds there too
            }
        }
    }
    send(REPORT, verdict, 0, 0);
    exit()
}

user_mode_runtime::panic_handler!();
