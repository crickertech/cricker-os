//! **`std_rayon`: unmodified `rayon` on nife** (milestone 812 (`std::thread::spawn` runs real
//! threads in one address space)'s exit test, its second check).
//!
//! The global pool sizes itself from `available_parallelism`, so it is one worker per online core.
//! A parallel count over four million items must come to four million, the answer the four-thread
//! half of the exit test reaches by hand. `rayon::broadcast` then runs one closure on every worker,
//! which only returns once each of them has run, so the number of distinct workers it reports is
//! the number of threads that really ran. `system_tests/src/user/std_threads_tests.rs` checks the
//! transcript against the cores the kernel has online.
//!
//! `rayon` is a dependency of this package only (calef's ruling on pull request #1892,
//! 2026-10-10 UTC, under §46 (thin primitives or whole subsystems)); `deny.toml` refuses it
//! anywhere else.

use rayon::prelude::*;

const ITEMS: u64 = 4_000_000;

fn main() {
    let threads = rayon::current_num_threads();
    let sum: u64 = (0..ITEMS).into_par_iter().map(|_| 1u64).sum();
    let mut ran = rayon::broadcast(|ctx| ctx.index());
    ran.sort_unstable();
    ran.dedup();
    println!("rayon threads {threads}");
    println!("rayon sum {sum}");
    println!("rayon broadcast reached {}", ran.len());
}
