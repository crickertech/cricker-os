//! **The words the capability demonstrations agree on**, and nothing else.
//!
//! Milestones 7 through 19 each proved one property of the capability model with a pair of user
//! programs and an assertion in `kernel/src/user`. Until milestone 291 every one of those programs
//! was a *role* of `fixtures/src/hello.rs`, so a value two of them shared was a constant in one
//! file and the comment "one binary, so one constant serves both roles" was true. Splitting the
//! roles into programs made that sentence false, and AGENTS.md rule 7 is what it becomes: anything
//! two binaries must agree on is a crate.
//!
//! Everything here but [`process_composition`]'s verdict bits is a **sentinel**: a value chosen
//! only to be recognised. None of them is a wire format, and changing one costs a rebuild rather
//! than a migration, which is why they can sit together in one crate instead of one crate each.
//!
//! # Examples
//!
//! The frame-delegation pair, with the page and the two processes stood in for. The producer writes
//! the sentinel into a page it owns and delegates a read-only view; the consumer maps the same
//! physical page and checks the value it reads. Recognising it is how the consumer knows the memory
//! is genuinely *shared* rather than a second frame that happens to be zeroed, which is the whole
//! claim `kernel::user::page_frame_service` wires and asserts.
//!
//! ```
//! use capability_witness_protocol::PAGE_FRAME_SENTINEL;
//!
//! // The producer, in its own address space.
//! let mut page = [0u64; 512];
//! page[0] = PAGE_FRAME_SENTINEL;
//!
//! // The consumer, through its own read-only mapping of the same physical frame.
//! let shared: &[u64] = &page;
//! assert_eq!(shared[0], PAGE_FRAME_SENTINEL, "the page was not shared");
//!
//! // And a frame that was merely retyped, never written through, does not look like one.
//! let fresh = [0u64; 512];
//! assert_ne!(fresh[0], PAGE_FRAME_SENTINEL);
//! ```
//!
//! # Bugs
//!
//! Nothing checks that a reader of one of these constants is one of the two parties that agreed on
//! it. A third program could depend on this crate and read `USED_WORD` for an unrelated purpose,
//! and the next change to that constant would break it silently. The tree's other `*_protocol`
//! crates have the same property and it has never bitten; recorded here rather than defended.
//! Name: ratified 2026-09-18 (calef, replacing the provisional `capability_demo_protocol` coined by
//! milestone 291's lane). Refused `capability_demo_protocol`, `demo_words`, `capability_demo`, and
//! folding these into `crates/abi`; the argument for each is below.
//!
//! **The ruling is the same one as `address_space_witness` (`process_composition_witness` since
//! §185 (what carries the claim that userspace composes a process from an authority you can count on
//! one hand), 2026-10-05), made the same day**: a thing here is named for what it proves rather than for
//! the occasion of its existence. Every value in this crate is a **witness** in the sense this tree
//! already uses the word: DECISIONS §31 (the foreign-language seam: C holds no capabilities and makes no syscalls)'s *"two witness pages answering two different questions"*,
//! and `unwritable_clock_witness`. A sentinel chosen only to be recognised by a second program is
//! exactly a witness value, so the word describes the contents rather than the milestone that
//! produced them.
//!
//! **Refused `capability_demo_protocol`**, this crate's own coinage, which the maintainer
//! recommended ratifying on the grounds that these really are demonstration fixtures and a name
//! hiding that would be worse. The objection that carried: **`demo` says why the code exists
//! rather than what the thing is**, and it sits close to AGENTS.md's second failure mode, a generic
//! word that could name almost anything in an operating system. It also dates the crate to an
//! occasion, and the values outlive the occasion.
//!
//! Refused `demo_words` ("words" names the representation rather than the agreement, and the
//! suffix is the shape a reader of this tree already recognises). Refused folding these into
//! `crates/abi` (abi is the kernel's syscall surface, which every program depends on, and three
//! fixture constants have no business widening it). Refused `capability_demo` unsuffixed (it would
//! read as the demo itself rather than as what the demos agree on, and there is no single demo to
//! be).
//!
//! The suffixed shape is AGENTS.md rule 7: what two compilation units must agree on has exactly one
//! definition, in a crate rather than a `#[path]` module.
//!
//! **Performed 2026-09-18**, in 27 occurrences across 16 files: the directory, the package name,
//! three `Cargo.toml` dependency entries, the `use` sites in `fixtures/` and `kernel/`, and the
//! live pointers in `design/naming.md` and one `PROPOSED` proposal. Two occurrences of the old name
//! were deliberately left standing in
//! `design/roadmap/430-refusals-written-where-the-tool-cannot-read-them.md`, where they are a
//! dated account of a measurement taken on this crate under the name it had that day.
//!
//! The suffix became `_protocol` at milestone 265, three weeks after this block was written and
//! together with the crates it cites; the `_proto` spellings above are what they were called when
//! this name was argued, and are left standing because the argument was about that spelling.

#![no_std]

/// The word the frame producer writes into a page it owns, and the consumer reads back through its
/// own mapping of the same physical page.
///
/// The two are separate processes with separate address spaces, so reading this value is the whole
/// evidence that one physical page is under both mappings: nothing else would put this bit pattern
/// in a freshly retyped frame.
pub const PAGE_FRAME_SENTINEL: u64 = 0xF00D_CAFE_D00D_1234;

/// The word the delegation receiver sends back through the capability it was *delegated*, so that
/// whoever holds the other end can confirm a capability minted by one process carries real
/// authority when a different process invokes it.
///
/// `system_tests/src/user/delegation_service.rs` re-exports this; it kept its own copy with a "must
/// match" comment beside it until milestone 291, which is the duplicate this crate removes.
pub const USED_WORD: u64 = 0x5A;

/// The word the cycle-counter reader reports when it read the counter without being killed for it.
///
/// **The word is the grant's result.** An ungranted read of `PMCCNTR_EL0` or the `cycle` CSR
/// traps, and this kernel turns that into a fault that ends the thread, so a program that gets as
/// far as sending anything is a program the grant reached. The two counter reads ride in words 1
/// and 2, and the kernel's test checks they moved forward only where the kernel itself says the
/// counter runs (milestone 74; the reasoning is on the test).
pub const CYCLE_COUNTER_WORD: u64 = 0xC1C1E;

/// **The verdict `process_composition_witness` reports**, one bit per step, so a failure says which
/// step stopped (milestone 404 (composing a process from two capabilities is proved for two verbs
/// and no more), building §185 (what carries the claim that userspace composes a process from an
/// authority you can count on one hand)).
///
/// Not a sentinel like the words above: a bit layout the fixture writes and the kernel's test reads,
/// which is the same rule 7 reason to live here. It was the literal `0b111` on both sides while the
/// fixture stopped at milestone 19b (run a real workload)'s three steps.
///
/// Name: provisional (the §185 build lane, 2026-10-05), with every constant in it.
pub mod process_composition {
    /// The child's ELF was found in the archive by name and parsed.
    pub const CHILD_FOUND: u64 = 1 << 0;
    /// An address space was retyped from the witness's budget and the child laid into it: segments,
    /// stack, a thread control block and its one capability.
    pub const SPACE_BUILT: u64 = 1 << 1;
    /// A frame of the witness's own was mapped into that space at a fresh address.
    pub const FRAME_MAPPED: u64 = 1 << 2;
    /// Mapping the same address again was refused: break-before-make holds in a space a process built.
    pub const DOUBLE_MAP_REFUSED: u64 = 1 << 3;
    /// The thread was configured at the child's entry, in that space, and started.
    pub const STARTED: u64 = 1 << 4;
    /// The child's word arrived on the rendezvous the witness minted for it, and it was the input
    /// squared: the thread ran, in the space it was built in, with the capability it was given.
    pub const CHILD_ANSWERED: u64 = 1 << 5;
    /// Every step.
    pub const WHOLE: u64 =
        CHILD_FOUND | SPACE_BUILT | FRAME_MAPPED | DOUBLE_MAP_REFUSED | STARTED | CHILD_ANSWERED;
}
