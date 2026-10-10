//! **The std runtime contract**: where a `std` program on nife finds each authority it was given.
//!
//! Name: provisional, from milestone 595 (provisional), 2026-09-25. The `_protocol` suffix follows
//! `environment_protocol` and `clock_protocol`, which the same PAL reads; the stem says which runtime.
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
//!   slot 8  page frame  the argv, READ               only if its line had words; page at ARGS_PAGE
//!   slot 9  endpoint    a name resolver, badged      only if given names to resolve; page at RESOLVER_PAGE
//!   slot 10 process     its own process, with BIND   only if it may spawn threads (milestone 812)
//!   slot 11 thread      its own first thread, WRITE  with slot 10
//!   slot 12 space       its own address space, READ  for futex waits; without it locks yield
//! ```
//!
//! What each slot means to std, and what std does when it is empty, is `rt.rs`'s documentation.
//! This crate is the numbers, so the PAL, the progenitor and the kernel test harness cannot disagree.
//!
//! **Generated verbatim into the PAL** by `cargo xtask std-src` (as `sys/pal/nife/runtimeproto.rs`),
//! the same discipline as `abi` and every other contract the PAL reads, so this file must stay a
//! valid non-root module: no crate attributes survive the copy, and neither does the test module.
//!
//! # Where the addresses come from
//!
//! **Every address below is a row of `crates/address_space_map`** (milestone 206 (a program image has under 896 KiB), DECISIONS §171 (where a program image starts)
//! option D), restated as a number rather than named, because this file is copied into std's PAL
//! where there is no crate to name. The test module (which does not travel) pins each one to its
//! band, the shape `byte_sink_protocol` uses for `abi`. The image and the stack are the map's own
//! rows, `IMAGE_BASE` and `STACK_TOP_PAGE`; a std program is linked at the one and its stack grows
//! down from the other like every other program's.
//!
//! # BUGS
//!
//! - **The numbers are restated, not derived.** A change to the map that moves the heap or the
//!   runtime windows fails this crate's host tests rather than failing to compile, which is rung
//!   two where the rest of the tree is on rung one.

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
/// A `READ` page frame capability naming the argument page, mapped read-only at [`ARGS_PAGE`]
/// (milestone 205 (how a foreign program is told what to do), DECISIONS §170 (how a foreign program is told what to do); the layout is
/// `argument_protocol`'s). Plain bytes that carry no authority. Provisional, with the layout: where
/// the page sits is one of the things the ruling on that layout fixes.
pub const ARGS_SLOT: u64 = 8;

/// A badged endpoint to the name resolver (milestone 384 (in a capability system the resolver is a
/// grant), §252 (a resolver grant is one zone per client badge)), which `std::net`'s `lookup_host`
/// asks. The badge carries the zone the spawner granted, so what `ToSocketAddrs` can resolve is
/// that zone and nothing else. Provisional, 2026-10-09 (UTC), milestone 801 (packages over the
/// internet).
pub const RESOLVER_SLOT: u64 = 9;

/// **This program's own process, with `BIND`** (milestone 812 (`std::thread::spawn` runs real
/// threads in one address space)): what `std::thread::spawn` joins a new thread to. Only if the
/// program may have threads; without it, a spawn answers `Unsupported`.
pub const PROCESS_SLOT: u64 = 10;

/// **This program's own first thread, with `WRITE`** (milestone 812): what the main thread sets its
/// own thread pointer through (`ThreadControlBlock::SET_THREAD_POINTER`), which `x86_64` can do no
/// other way. Given with [`PROCESS_SLOT`].
pub const THREAD_SLOT: u64 = 11;

/// **This program's own address space, with `READ`** (milestone 812): the capability futex `WAIT`
/// and `WAKE` are invoked on, so `std`'s locks sleep rather than spin. Without it they yield.
pub const SPACE_SLOT: u64 = 12;

/// How many slots the contract fixes. A loader places a std program's capabilities at these slot
/// numbers and nowhere else; the reserved fault slot is last in the table and far above them.
pub const SLOTS: u64 = 13;

/// Where the loader maps the page a std program shares with its file service: one file block,
/// carrying a name out on `OPEN` and file bytes both ways on `READ` and `WRITE`.
pub const FS_PAGE: u64 = 0x1100_0000;
/// Where the loader maps the wall-clock page, read-only (`clock_protocol`'s layout).
pub const CLOCK_PAGE: u64 = 0x1200_0000;
/// Where the loader maps the inert-configuration page, read-only (`environment_protocol`'s layout).
pub const CONFIG_PAGE: u64 = 0x1300_0000;
/// Where the loader maps the argument page, read-only (`argument_protocol`'s layout). One page
/// above the configuration page, following its pattern. Provisional, like [`ARGS_SLOT`].
pub const ARGS_PAGE: u64 = 0x1400_0000;

/// Where std maps the page it lends the resolver: the program's own page, minted from the socket
/// frames' budget (slot 3) the first time a name is looked up, so a resolver grant is only usable
/// beside the network. Provisional, like [`RESOLVER_SLOT`].
pub const RESOLVER_PAGE: u64 = 0x1500_0000;

/// Where the heap starts: the start of the address-space map's heap band, and the same value as
/// `user_mode_runtime::heap::DEFAULT_BASE`.
pub const HEAP_BASE: u64 = 0x4000_0000;
/// The heap's growth cap: the heap band's whole width. Only a bound on the address range: the
/// budget in [`MEMORY_REGION_SLOT`] is the real, per-program limit.
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

// The stack band holds 4,080 pages less a guard (`address_space_map::MAX_STACK_PAGES`); the test
// below pins the exact figure. This line keeps a nonsense value out of the PAL, which cannot see
// the map.
const _: () = assert!(STACK_PAGES > 0 && STACK_PAGES <= 4079);

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
            ARGS_SLOT,
            RESOLVER_SLOT,
            PROCESS_SLOT,
            THREAD_SLOT,
            SPACE_SLOT,
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

    /// The shared pages must not land on each other, on the heap, or inside the window the
    /// image and stack occupy: a collision there is not a compile error, it is two authorities
    /// aliasing one page in a child.
    #[test]
    fn the_shared_pages_are_page_aligned_and_clear_of_each_other_and_the_heap() {
        let pages = [FS_PAGE, CLOCK_PAGE, CONFIG_PAGE, ARGS_PAGE, RESOLVER_PAGE];
        for (i, a) in pages.iter().enumerate() {
            assert_eq!(a % PAGE, 0, "{a:#x} is not page aligned");
            assert!(
                address_space_map::RUNTIME_WINDOWS.holds(*a, *a + PAGE),
                "{a:#x} is not in the address-space map's runtime windows"
            );
            for b in &pages[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert_eq!(HEAP_BASE % PAGE, 0);
        assert_eq!(HEAP_MAX % PAGE, 0);
    }

    /// **The restated numbers are the map's**, which is the whole of what stands between this file
    /// and a PAL that disagrees with the loader. See the crate's `BUGS`.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_heap_and_the_stack_are_the_address_space_maps() {
        assert_eq!(HEAP_BASE, address_space_map::HEAP.start);
        assert_eq!(HEAP_MAX, address_space_map::HEAP.bytes());
        assert!(STACK_PAGES <= address_space_map::MAX_STACK_PAGES);
        assert_eq!(address_space_map::MAX_STACK_PAGES, 4079);
    }
}
