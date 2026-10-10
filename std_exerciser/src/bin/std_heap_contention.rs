//! **`std_heap_contention`: how the `std` heap's spinlock holds up under four threads** (milestone 812
//! (`std::thread::spawn` runs real threads in one address space), the block's item 12: "measure it
//! under four threads before replacing it").
//!
//! The same work twice: `ALLOCATIONS` small allocations and frees, done by one thread, then split
//! across four. It prints the counter ticks each took, and the test that runs it
//! (`system_tests::user::std_threads_tests`) prints them; `notes/std/threads.md` records what they
//! were. A number under QEMU's emulation is a ratio to read, not a speed: the heap is the one lock,
//! so four threads on four cores finishing in no less time than one thread is contention.

use std::time::Instant;

const ALLOCATIONS: usize = 200_000;

fn churn(n: usize) {
    for i in 0..n {
        let b = Box::new([i as u8; 48]);
        std::hint::black_box(&b);
    }
}

fn timed(threads: usize) -> u128 {
    let start = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|_| std::thread::spawn(move || churn(ALLOCATIONS / threads)))
        .collect();
    for h in handles {
        h.join().expect("a churning thread panicked");
    }
    start.elapsed().as_micros()
}

fn main() {
    let one = timed(1);
    let four = timed(4);
    println!("heap allocations {ALLOCATIONS}");
    println!("one thread us {one}");
    println!("four threads us {four}");
}
