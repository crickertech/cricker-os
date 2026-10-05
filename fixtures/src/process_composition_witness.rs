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
//! the double map was refused. `kernel::user::tests` asserts `0b111` and prints the bit meanings on
//! failure.
//!
//! Name: ratified 2026-10-05 (calef, §185 (what carries the claim that userspace composes a process
//! from an authority you can count on one hand), replacing `address_space_witness`, which he
//! ratified 2026-09-18 ruling on the provisional fixture names of milestone 291 (thirty-one
//! programs wearing one name)). Coined by milestone 291's lane, from `hello`'s
//! `ADDRESS_SPACE_BUILDER` role (number 19), whose role constant lowercased was the first name.
//! Refused `address_space_builder` and `builder`; the argument for each is below.
//!
//! **Why it was renamed.** §185 extended this fixture to run a thread in the space it builds, and
//! `address_space_witness` then named one step of what it proves rather than the claim. calef
//! prefers a program that does one thing well, and accepted the extension under this name on that
//! ground: *"The one thing it does well is test."* (his full words are in §185's ruling). The one
//! thing is the claim it witnesses, that a process can be composed from two capabilities; building
//! the space is a step of that, not a second job.
//!
//! **The ruling underneath is that a fixture is named for what it proves, not for what it does.**
//! This tree already had one of those in `unwritable_clock_witness`, and this file's own prose
//! reached for the same word before anyone ruled: *what this witnesses is that a process can
//! construct one at all*. **Refused `address_space_builder`**, which the maintainer recommended on
//! the grounds that building *is* the claim here, since 19b asked whether a process can construct a
//! space at all, so the action and the proof coincide. calef ruled the other way: everything in
//! `fixtures/` proves something, and naming them for the proof is the scheme rather than the
//! exception.
//!
//! **That makes a scheme question live for the fixtures named for their action** (`image_self_
//! checker`, `allocator_exerciser`, `os_primitives_benchmarker`). They are already on `script/names
//! --unratified`, so they will reach calef on their own; nothing here rules on them, and this block
//! should not be read as having done so.
//!
//! **The reason the first name gave for its qualifier had expired.** It said the qualifier existed
//! because the name was *"deliberately not `builder`, which is already a program in every archive
//! (milestone 20's richer-initrd demo)"*. That demo belonged to milestone 20 (a portable HAL,
//! proven on a second architecture). Milestone 295 (retire `components/src/builder.rs`) retired
//! that program on 2026-09-14, on calef's own ruling, so the collision the qualifier was avoiding
//! no longer exists. The refusal survives on a different rule: `builder` alone is a **generic word
//! that could name almost anything in an operating system**, which is AGENTS.md's second naming
//! failure mode.
//!
//! **Performed twice.** The first rename (`address_space_builder` to `address_space_witness`,
//! 2026-09-18) touched 44 occurrences in 18 files and left the old name in five
//! `bench/radon-2026-09-16/jobmix-boot*.log` transcripts, the `BUILT` blocks of milestones 291, 295
//! and 158, a captured `script/names --unratified` listing, and the refusals proposal's dated list.
//! The second (2026-10-05, the §185 build lane) moved the binary, this file, the archive entry, the
//! test wiring and the live notes. It kept `address_space_witness` in what is an account under the
//! name it was written with: §185 itself, the `BUILT` blocks of milestones 295 and 430,
//! `design/naming/provenance.md` and `design/naming/rename-traps.md`, the capability witness
//! protocol's `Name:` block (which records a ruling made the same day under that name), and
//! `notes/dependency-census/dependencies.tsv`, whose evidence column quotes roadmap text as it read
//! on its blame date. Milestone 295's path was corrected, since a path is navigation. `hello`'s
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
