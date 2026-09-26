//! **The std runtime contract**: where a `std` program on nife finds each authority it was given.
//!
//! Name: provisional. Introduced 2026-09-25 by milestone 595 (provisional), in which the shell runs
//! a `std` program. It needed the progenitor to build a child in this layout and found the numbers
//! written three times: in the std PAL's `rt.rs`, and twice in the kernel test harness. The
//! `_protocol` suffix follows `environment_protocol` and `clock_protocol`, the contracts the same
//! PAL already reads; the stem says which runtime. Expect calef to rename it.
//!
//! A native nife program is handed its capabilities in the order its spawner lists them, and
//! reads them by position. A `std` program cannot work that way, because the code reading the
//! slots is the standard library, written once for every program: `File::open` has to know where
//! the directory is without the program saying. So a `std` program's slots are **fixed by
//! number**, and an empty slot is how std learns it was not given that authority. `std::fs` on a
//! program with slot 4 empty answers `Unsupported`, which is what "no ambient filesystem" looks
//! like from inside.
//!
//! ```text
//!   slot 0  untyped     the heap's budget            always
//!   slot 1  endpoint    stdout and stderr            always
//!   slot 2  endpoint    the network stack            only if given the network
//!   slot 3  untyped     socket frames' budget        only if given the network
//!   slot 4  endpoint    one directory                only if given a directory; page at FS_PAGE
//!   slot 5  page frame  the wall clock, READ         only if given a clock; page at CLOCK_PAGE
//!   slot 6  endpoint    the entropy service          only if given randomness
//!   slot 7  page frame  TZ, LANG and TERM, READ      only if given configuration; page at CONFIG_PAGE
//! ```
//!
//! What each slot means to std, and what std does when it is empty, is `rt.rs`'s documentation,
//! beside the code that reads them. This crate is the numbers, so the PAL, the progenitor and the
//! kernel test harness cannot disagree about them.
//!
//! **Generated verbatim into the PAL** by `cargo xtask std-src` (as `sys/pal/nife/runtimeproto.rs`),
//! the same discipline as `abi` and every other contract the PAL reads, so this file must stay a
//! valid non-root module: no crate attributes survive the copy, and neither does the test module.
//!
//! # BUGS
//!
//! - **The image and the stack are not in here**, and they are part of the layout. A std program is
//!   linked at `0x40_0000` like every other, and its stack grows down from `0x50_0000`, the
//!   loader's one address for every child (`supervision_protocol::CHILD_STACK_VA`). So a std image
//!   over about 1 MiB meets its own stack. That is milestone 206 (a program image has under 896
//!   KiB), not yet built. DECISIONS §171 (where a program image starts) was ruled on 2026-09-26:
//!   one written address-space map, with these constants derived from it.

#![cfg_attr(not(test), no_std)]

/// The untyped budget the global allocator maps heap pages from, lazily, at [`HEAP_BASE`].
pub const MEMORY_REGION_SLOT: u64 = 0;
/// The endpoint `stdout` and `stderr` both send on, in `byte_sink_protocol`'s framing.
pub const STDOUT_SLOT: u64 = 1;
/// The network stack's client endpoint, which `std::net` speaks `socket_protocol` over. The name
/// means the stack, not a call stack.
pub const STACK_SLOT: u64 = 2;
/// The untyped budget `std::net` mints each socket's shared frame from.
pub const NET_MEMORY_REGION_SLOT: u64 = 3;
/// The file service endpoint that **is** the directory capability: every name `std::fs` sends is
/// resolved under the one directory it reaches. Its shared page is at [`FS_PAGE`].
pub const FS_DIR_SLOT: u64 = 4;
/// A `READ` page frame capability naming the wall-clock page, mapped read-only at [`CLOCK_PAGE`].
pub const CLOCK_SLOT: u64 = 5;
/// The entropy service's request endpoint, `WRITE`: the right to ask for bytes.
pub const ENTROPY_SLOT: u64 = 6;
/// A `READ` page frame capability naming the inert-configuration page, mapped read-only at
/// [`CONFIG_PAGE`].
pub const CONFIG_SLOT: u64 = 7;

/// How many slots the contract fixes. A loader places a std program's capabilities at these slot
/// numbers and nowhere else; the reserved fault slot is last in the table and far above them.
pub const SLOTS: u64 = 8;

/// Where the loader maps the page a std program shares with its file service: one file block,
/// carrying a name out on `OPEN` and file bytes both ways on `READ` and `WRITE`.
pub const FS_PAGE: u64 = 0x1100_0000;
/// Where the loader maps the wall-clock page, read-only (`clock_protocol`'s layout).
pub const CLOCK_PAGE: u64 = 0x1200_0000;
/// Where the loader maps the inert-configuration page, read-only (`environment_protocol`'s layout).
pub const CONFIG_PAGE: u64 = 0x1300_0000;

/// Where the heap starts: clear of the image (`0x40_0000`), the stack below `0x50_0000`, the net
/// PAL's per-socket frames (`0x1000_0000` upward), the three pages above, and the initrd window at
/// `0x2000_0000`. Same value as `user_mode_runtime::heap::DEFAULT_BASE`.
pub const HEAP_BASE: u64 = 0x4000_0000;
/// The heap's growth cap. Only a bound on the address range: the budget in
/// [`MEMORY_REGION_SLOT`] is the real, per-program limit.
pub const HEAP_MAX: u64 = 256 * 1024 * 1024;

/// **Stack pages a loader maps for a std program**, where a native child of the progenitor gets
/// twelve. std's startup, its formatting machinery and its collections use far more stack than a
/// hand-written `no_std` program, and the failure mode of too little is a fault in code nobody
/// here wrote. Thirty-two is what the kernel test harness has mapped under every std program since
/// milestone 27 (Rust `std` on the native ABI), plus the one page its own `run` maps, so it is the number every std program on
/// nife has been proven under, rather than a measurement of any one of them.
///
/// Name: provisional.
pub const STACK_PAGES: u64 = 32;

// The stack grows down from `0x50_0000` and the image starts at `0x40_0000`, so the stack may not
// take that whole megabyte: a stack sized into the image would map over code. At compile time, so
// the PAL this file is generated into checks it too.
const _: () = assert!(STACK_PAGES > 0 && STACK_PAGES * 4096 < 0x50_0000 - 0x40_0000);

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: u64 = 4096;

    #[test]
    fn every_slot_is_distinct_and_inside_the_fixed_range() {
        let slots = [
            MEMORY_REGION_SLOT,
            STDOUT_SLOT,
            STACK_SLOT,
            NET_MEMORY_REGION_SLOT,
            FS_DIR_SLOT,
            CLOCK_SLOT,
            ENTROPY_SLOT,
            CONFIG_SLOT,
        ];
        for (i, a) in slots.iter().enumerate() {
            assert!(
                *a < SLOTS,
                "slot {a} is outside the {SLOTS} the contract fixes"
            );
            for b in &slots[i + 1..] {
                assert_ne!(a, b, "two authorities share slot {a}");
            }
        }
        assert_eq!(slots.len() as u64, SLOTS);
    }

    /// The three shared pages must not land on each other, on the heap, or inside the window the
    /// image and stack occupy: a collision there is not a compile error, it is two authorities
    /// aliasing one page in a child.
    #[test]
    fn the_shared_pages_are_page_aligned_and_clear_of_each_other_and_the_heap() {
        let pages = [FS_PAGE, CLOCK_PAGE, CONFIG_PAGE];
        for (i, a) in pages.iter().enumerate() {
            assert_eq!(a % PAGE, 0, "{a:#x} is not page aligned");
            assert!(
                *a >= 0x50_0000,
                "{a:#x} is inside the image and stack window"
            );
            assert!(*a + PAGE <= HEAP_BASE, "{a:#x} overlaps the heap");
            for b in &pages[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert_eq!(HEAP_BASE % PAGE, 0);
        assert_eq!(HEAP_MAX % PAGE, 0);
    }
}
