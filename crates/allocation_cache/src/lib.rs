//! **`allocation_cache`**: a bounded stash of freed blocks in front of the shared heap, so most
//! allocations never take the heap's lock. The algorithm only, like `user_mode_heap`: it never locks,
//! maps or reads a CPU id, and its owner decides who touches it. `std` gives each thread one, and
//! `cargo xtask std-src` generates this file into the overlay verbatim.
//!
//! **One thread owns a cache, never a CPU.** Without `rseq`, a thread preempted mid-pop hands its
//! core, and a per-CPU list, to another thread, so a per-CPU list needs an atomic on every operation.
//! A per-thread cache needs none. `notes/std/allocation.md` has the argument and the measurement.
//!
//! A class is an effective size on `user_mode_heap`'s grid ([`GRID`]), up to [`LARGEST`]. Only
//! requests aligned to at most the grid are cached. The heap knows a block's size from the size
//! alone, so blocks of a class are fungible: any one serves, and is freed under, any such request.
//!
//! # Examples
//!
//! ```
//! use allocation_cache::{Cache, class};
//! use core::ptr::NonNull;
//!
//! let mut cache = Cache::new();
//! let c = class(48, 8).unwrap(); // a `[u8; 48]`
//! assert_eq!((class(16, 64), class(257, 8)), (None, None)); // over-aligned, too big
//! let mut block = [0u128; 3]; // 48 grid-aligned bytes, as the heap would hand out
//! let p = NonNull::from(&mut block).cast::<u8>();
//! // SAFETY: `p` is 48 writable, grid-aligned bytes nobody else uses while the cache holds them.
//! assert!(unsafe { cache.put(c, p) }.is_ok());
//! assert_eq!((cache.take(c), cache.take(c)), (Some(p), None)); // then the caller goes to the heap
//! ```
//!
//! # BUGS
//!
//! - A cached block cannot coalesce until it is spilled: up to [`MOST_CACHED_BYTES`] (34,816) per
//!   cache, about half a mebibyte for sixteen threads. A thread's cache is spilled at its end.
//! - [`DEPTH`] (glibc's `tcache` count) and [`LARGEST`] are chosen, not measured per workload.
//!
//! Name: provisional, minted 2026-10-10 (UTC) by the lane for milestone 561 (a per-CPU
//! allocator is what the current-CPU page was for), for what it holds rather than its owner.
//! calef has not ruled on it.

#![no_std]

use core::ptr::NonNull;

/// The allocation grid, restated from `user_mode_heap::MIN_ALIGN` because the two files become
/// sibling modules inside `std` and may not depend on each other. A host test holds them equal.
pub const GRID: usize = 16;

/// The largest effective size a cache holds.
pub const LARGEST: usize = 256;

/// How many size classes there are: one per grid step up to [`LARGEST`].
pub const CLASSES: usize = LARGEST / GRID;

/// How many blocks of one class a cache holds before a free spills.
pub const DEPTH: usize = 16;

/// How many blocks a refill takes from the heap, and a spill gives back, under one acquisition of
/// its lock: half the depth, so a thread that only allocates or only frees takes the lock once per
/// eight blocks, and one that alternates at the boundary does not take it on every call.
pub const BATCH: usize = DEPTH / 2;

/// The most memory one full cache keeps out of the heap.
pub const MOST_CACHED_BYTES: usize = DEPTH * GRID * CLASSES * (CLASSES + 1) / 2;

/// The class of a request of `size` bytes aligned to `align`, or `None` when it is not cached.
pub const fn class(size: usize, align: usize) -> Option<usize> {
    if align > GRID {
        return None;
    }
    // Rounded exactly as `user_mode_heap::effective_size` rounds, zero included.
    let effective = if size == 0 {
        GRID
    } else {
        size.next_multiple_of(GRID)
    };
    if effective > LARGEST {
        return None;
    }
    Some(effective / GRID - 1)
}

/// The effective size of a class's blocks.
pub const fn class_bytes(class: usize) -> usize {
    (class + 1) * GRID
}

/// A cached block's first word, linking it to the next block of its class.
struct Free {
    next: *mut Free,
}

const _: () = assert!(core::mem::size_of::<Free>() <= GRID);

/// **One owner's stash**: a stack of freed blocks per class, each at most [`DEPTH`] deep.
///
/// `const` so it can sit inside a statically made structure (`std`'s per-thread block). It holds raw
/// pointers into memory it does not own, so it is neither `Send` nor `Sync`: its owner keeps it
/// where one thread reaches it, and `std` keeps it in that thread's own block.
pub struct Cache {
    heads: [*mut Free; CLASSES],
    counts: [u8; CLASSES],
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    /// An empty cache.
    pub const fn new() -> Self {
        Self {
            heads: [core::ptr::null_mut(); CLASSES],
            counts: [0; CLASSES],
        }
    }

    /// How many blocks of `class` the cache holds.
    pub fn count(&self, class: usize) -> usize {
        usize::from(self.counts[class])
    }

    /// The bytes the cache is keeping out of the heap.
    pub fn cached_bytes(&self) -> usize {
        (0..CLASSES).map(|c| self.count(c) * class_bytes(c)).sum()
    }

    /// A cached block of `class`, or `None` when there is none and the caller must go to the heap.
    pub fn take(&mut self, class: usize) -> Option<NonNull<u8>> {
        let head = NonNull::new(self.heads[class])?;
        // SAFETY: `head` is a block this cache holds (`put`'s contract), whose first word `put`
        // wrote as a `Free`.
        self.heads[class] = unsafe { (*head.as_ptr()).next };
        self.counts[class] -= 1;
        Some(head.cast())
    }

    /// Keep `block` as a free block of `class`. Hands it back as `Err` when the class is full, and
    /// the caller should [`spill`](Self::spill) and try again, or free it to the heap.
    ///
    /// # Safety
    ///
    /// `block` is [`class_bytes`]`(class)` bytes, writable, aligned to [`GRID`], and from now on
    /// used by nothing but this cache, until [`take`](Self::take) or a spill hands it back.
    pub unsafe fn put(&mut self, class: usize, block: NonNull<u8>) -> Result<(), NonNull<u8>> {
        if self.count(class) >= DEPTH {
            return Err(block);
        }
        // The caller promises `GRID` alignment, which is at least a `Free`'s.
        #[allow(clippy::cast_ptr_alignment)]
        let node = block.as_ptr().cast::<Free>();
        // SAFETY: the caller's contract: the block is ours, writable and big and aligned enough.
        unsafe {
            node.write(Free {
                next: self.heads[class],
            });
        };
        self.heads[class] = node;
        self.counts[class] += 1;
        Ok(())
    }

    /// Hand up to [`BATCH`] blocks of `class` to `give`, which returns them to the heap. Call it
    /// holding the heap's lock, once, rather than locking per block.
    pub fn spill(&mut self, class: usize, mut give: impl FnMut(NonNull<u8>)) {
        for _ in 0..BATCH {
            match self.take(class) {
                Some(b) => give(b),
                None => break,
            }
        }
    }

    /// Fill `class` with up to [`BATCH`] blocks from `get`, which allocates them from the heap and
    /// answers `None` when it has none. Call it holding the heap's lock. Returns how many it took.
    ///
    /// # Safety
    ///
    /// Every block `get` answers meets [`put`](Self::put)'s contract for `class`.
    pub unsafe fn refill(
        &mut self,
        class: usize,
        mut get: impl FnMut() -> Option<NonNull<u8>>,
    ) -> usize {
        let mut taken = 0;
        while taken < BATCH && self.count(class) < DEPTH {
            let Some(b) = get() else { break };
            // SAFETY: the caller's contract for what `get` answers; the class has room (checked).
            let kept = unsafe { self.put(class, b) };
            debug_assert!(kept.is_ok());
            taken += 1;
        }
        taken
    }

    /// Hand every block to `give` with its class, leaving the cache empty: a thread's end.
    pub fn drain(&mut self, mut give: impl FnMut(usize, NonNull<u8>)) {
        for c in 0..CLASSES {
            while let Some(b) = self.take(c) {
                give(c, b);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::alloc::Layout;
    use std::collections::HashSet;
    use std::vec::Vec;

    use super::*;

    #[test]
    fn the_grid_is_the_heaps() {
        assert_eq!(GRID, user_mode_heap::MIN_ALIGN);
        for size in 0..=LARGEST + GRID {
            for align in [1, 2, 4, 8, 16] {
                let l = Layout::from_size_align(size, align).unwrap();
                match class(size, align) {
                    Some(c) => assert_eq!(class_bytes(c), user_mode_heap::effective_size(l)),
                    None => assert!(user_mode_heap::effective_size(l) > LARGEST),
                }
            }
        }
    }

    #[test]
    fn the_bound_is_what_a_full_cache_holds() {
        let mut arena: Vec<u128> = std::vec![0; MOST_CACHED_BYTES / GRID];
        let base = arena.as_mut_ptr().cast::<u8>();
        let mut cache = Cache::new();
        let mut off = 0;
        for c in 0..CLASSES {
            for _ in 0..DEPTH {
                // SAFETY: disjoint, grid-aligned slices of `arena`, which outlives `cache`.
                let b = unsafe { NonNull::new_unchecked(base.add(off)) };
                // SAFETY: as above; each slice is handed to the cache once.
                assert!(unsafe { cache.put(c, b) }.is_ok());
                off += class_bytes(c);
            }
            let spare = NonNull::new(base).unwrap();
            // SAFETY: refused before the block is touched, so aliasing it is harmless.
            assert_eq!(unsafe { cache.put(c, spare) }, Err(spare));
        }
        assert_eq!(off, MOST_CACHED_BYTES);
        assert_eq!(cache.cached_bytes(), MOST_CACHED_BYTES);
        let mut n = 0;
        cache.drain(|_, _| n += 1);
        assert_eq!(n, CLASSES * DEPTH);
        assert_eq!(cache.cached_bytes(), 0);
    }

    /// The test that matters: a cache in front of a real `user_mode_heap`, driven by a random
    /// mix of allocations and frees with refills and spills, never hands out a block twice and gives
    /// every byte back, coalesced, at the end.
    #[test]
    fn a_cache_in_front_of_the_heap_never_hands_out_a_block_twice() {
        const ARENA: usize = 64 * 1024;
        let mut arena: Vec<u128> = std::vec![0; ARENA / GRID];
        let mut heap = user_mode_heap::Heap::new();
        // SAFETY: `arena` is ours, grid-aligned, and outlives `heap`.
        unsafe { heap.add_region(arena.as_mut_ptr().cast(), ARENA) };
        let mut cache = Cache::new();
        let mut live: Vec<(NonNull<u8>, Layout)> = Vec::new();
        let mut seen = HashSet::new();
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        // Miri interprets about a thousand times slower; a tenth of the walk still crosses every
        // refill and spill boundary many times over.
        let steps = if cfg!(miri) { 2_000 } else { 20_000 };
        for _ in 0..steps {
            let r = next();
            if r % 3 != 0 || live.is_empty() {
                let size = (r >> 8) as usize % (LARGEST + 1);
                let l = Layout::from_size_align(size, 8).unwrap();
                let c = class(size, 8).unwrap();
                let p = match cache.take(c) {
                    Some(p) => p,
                    None => {
                        // SAFETY: the heap's blocks are its class's size and grid-aligned.
                        unsafe {
                            cache.refill(c, || {
                                heap.alloc(Layout::from_size_align(class_bytes(c), 8).unwrap())
                            })
                        };
                        match cache.take(c) {
                            Some(p) => p,
                            None => continue, // the arena is full; fine
                        }
                    }
                };
                assert!(seen.insert(p), "a block was handed out while live");
                // SAFETY: a live block of at least `size` bytes; writing all of it would trip Miri
                // on any overlap with another live block or with a cache link.
                unsafe { core::ptr::write_bytes(p.as_ptr(), 0xa5, size) };
                live.push((p, l));
            } else {
                let (p, l) = live.swap_remove((r >> 8) as usize % live.len());
                seen.remove(&p);
                let c = class(l.size(), l.align()).unwrap();
                // SAFETY: `p` is live, ours, of class `c`.
                if let Err(p) = unsafe { cache.put(c, p) } {
                    // SAFETY: each spilled block was cached as class `c`, so `class_bytes(c)` is
                    // what it owns in the heap.
                    cache.spill(c, |b| unsafe {
                        heap.dealloc(b, Layout::from_size_align(class_bytes(c), 8).unwrap());
                    });
                    // SAFETY: as the first `put`; the spill made room.
                    assert!(unsafe { cache.put(c, p) }.is_ok());
                }
            }
        }
        for (p, l) in live.drain(..) {
            // SAFETY: live blocks from this heap, freed once, under their own layouts.
            unsafe { heap.dealloc(p, l) };
        }
        // SAFETY: as above, with the class size every cached block was given back under.
        cache.drain(|c, b| unsafe {
            heap.dealloc(b, Layout::from_size_align(class_bytes(c), 8).unwrap());
        });
        assert_eq!(heap.free_bytes(), ARENA);
        assert_eq!(heap.block_count(), 1, "everything given back coalesces");
    }
}
