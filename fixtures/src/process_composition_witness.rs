//! **A process composed from two capabilities, run in the space it was built in** (milestone 19b
//! (run a real workload), extended by §185 (what carries the claim that userspace composes a process
//! from an authority you can count on one hand), as milestone 404 (composing a process from two
//! capabilities is proved for two verbs and no more)).
//!
//! Holds exactly two capabilities, a memory region (slot 0) and a report line (slot 1), which is
//! what the retired `builder` held. From those and nothing else it:
//!
//! 1. reads `least_authority_demo` out of the archive by name and parses its ELF,
//! 2. mints a rendezvous out of its own memory, for the child to answer on,
//! 3. builds the child through `supervision_protocol::build_child_space`, the loader every composer
//!    in this tree shares: an address space retyped from the budget, each segment laid down W^X, a
//!    stack, a thread control block, and one capability (`WRITE` on that rendezvous) in its slot 0,
//! 4. maps a frame of its own into the same space and maps it again, which milestone 19b's claim
//!    needs refused (break-before-make holds in a space a process built),
//! 5. configures the thread at the child's entry in that space and starts it with an input,
//! 6. and receives the child's answer, the input squared, on the rendezvous it minted.
//!
//! The kernel allocates nothing on the way: every object is retyped out of the memory region it was
//! handed. The verdict is one word, a bit per step, laid out in
//! `capability_witness_protocol::process_composition`; the kernel's test asserts the whole word and
//! prints the bit meanings on failure.
//!
//! **Reading the archive is not a third capability.** The kernel maps the archive read-only at
//! `user_mode_runtime::initrd::INITRD_VA` and passes its length in `x1`, which is what it did for
//! `builder` and does for the progenitor. That mapping lets this program read bytes; it names no
//! kernel object and can do nothing with the system. A reader who counts it as authority anyway
//! should know it is there.
//!
//! **Milestone 19b's "nothing runs in the space it built" reading no longer holds.** It held while
//! threads were 19c's object, and §185 chose to extend this fixture rather than add a second one. The
//! account of 19b says it as it was.
//!
//! # BUGS
//!
//! - **A child that never answers hangs this program**, because the receive in step 6 has no
//!   deadline and the child is not supervised. The kernel's test waits with one instead, so the
//!   failure is a test that says the child never answered rather than a watchdog dump; the witness
//!   itself stays parked until the test kernel exits. Supervising the child (`fault` in the
//!   endowment) would cost a third capability or a second minted object, and the claim is about the
//!   floor.
//! - **The child is loaded unmeasured**, as `builder` loaded it: nothing here consults
//!   `measured_boot::PROGRAM_MEASUREMENTS`. notes/trusted-init.md lists the demo loaders that do not.
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

use capability_witness_protocol::process_composition::{
    CHILD_ANSWERED, CHILD_FOUND, DOUBLE_MAP_REFUSED, FRAME_MAPPED, SPACE_BUILT, STARTED,
};
use supervision_protocol::{
    ChildEndowment, Retention, build_child_space, configure_child, start_child,
};
use user_mode_runtime::{exit, map_into, receive, retype_object, retype_page_frame, send};

const MEMORY_REGION: u64 = 0;
const REPORT: u64 = 1;

/// The child this composes: one capability in slot 0, squares the input in `x1`, sends it there.
const CHILD: &str = "least_authority_demo";
const INPUT: u64 = 9;

/// Where the witness maps a frame of its own into the child's space for milestone 19b's probe: in
/// the pair band, so nothing the loader lays down can be there already.
const VA: u64 = address_space_map::pair_page(0x0040_0000);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, initrd_len: u64, _arg2: u64) -> ! {
    send(REPORT, compose(initrd_len), 0, 0);
    exit()
}

/// Every step, returning the verdict bits it reached. Stops at the first step that fails, so the
/// word says where.
fn compose(initrd_len: u64) -> u64 {
    // SAFETY: forwarded from user_mode_runtime::initrd::initrd_bytes's own contract: the kernel
    // mapped the archive at INITRD_VA and passed its length in x1.
    let archive = unsafe { user_mode_runtime::initrd::initrd_bytes(initrd_len) };
    let Some(elf) = nifefs::Fs::parse(archive)
        .ok()
        .and_then(|fs| fs.read(CHILD))
        .and_then(|bytes| elf::Elf::parse(bytes).ok())
    else {
        return 0;
    };
    let mut verdict = CHILD_FOUND;

    let answer = retype_object(MEMORY_REGION, abi::objtype::RENDEZVOUS);
    if answer < 0 {
        return verdict;
    }
    let answer = answer as u64;
    let caps = [(answer, abi::rights::WRITE)];
    let endow = ChildEndowment {
        caps: &caps,
        ..ChildEndowment::new(Retention::Nothing)
    };
    // One budget pays for both the child and our own scratch mappings: there is only the one, and
    // nothing here destroys it while the child lives.
    let Ok((child, aspace)) = build_child_space(MEMORY_REGION, MEMORY_REGION, &elf, &endow) else {
        return verdict;
    };
    verdict |= SPACE_BUILT;

    let frame = retype_page_frame(MEMORY_REGION);
    if frame >= 0 {
        let frame = frame as u64;
        if map_into(aspace, VA, frame, abi::address_space::MAP_RW) == 0 {
            verdict |= FRAME_MAPPED;
        }
        if map_into(aspace, VA, frame, abi::address_space::MAP_RW) < 0 {
            verdict |= DOUBLE_MAP_REFUSED;
        }
    }

    // CONFIGURE consumes the address space capability; START, with `Retention::Nothing`, our
    // thread control block capability. After this the child is reachable only through what it holds.
    if configure_child(child.tcb, aspace, elf.entry()).is_err() || !start_child(child, 0, INPUT, 0)
    {
        return verdict;
    }
    verdict |= STARTED;

    let (word, _, _) = receive(answer);
    if word == INPUT * INPUT {
        verdict |= CHILD_ANSWERED;
    }
    verdict
}

user_mode_runtime::panic_handler!();
