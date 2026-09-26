//! **`slabtop`: where this prompt's job budget went, by kind of kernel object** (milestone 126 (the `procps` package),
//! DECISIONS §225 (`free` sees the machine and your share)).
//!
//! Upstream `slabtop` ranks the kernel's slab caches by the memory each holds. This kernel has no
//! slab: milestone 14 (kernel objects from untyped) removed its heap, and a kernel object is carved
//! from a region its holder owns. So the question §225 kept is the same one asked of a budget
//! instead of a kernel: of the pages this prompt's jobs have spent, how many went to threads, to
//! address spaces, to rendezvous objects, and to plain frames. The figures are
//! `MemoryRegion::USAGE` on a view of the budget narrowed to `ENUMERATE`, summed by the kernel over
//! every job region carved from it.
//!
//! The program's logic lives here so it runs on the host; `components/src/slabtop.rs` asks the
//! kernel seven questions and writes this.
//!
//! # EXAMPLES
//!
//! ```text
//! $ slabtop
//! job budget: 80 of 512 pages spent
//!    PAGES      KIB  SPENT ON
//!       60      240  frames
//!        4       16  threads
//!        4       16  address spaces
//!        2        8  rendezvous
//! ```
//!
//! # BUGS
//!
//! - The counts say where pages went, not what is alive: a region's pages are spent until the
//!   region is reclaimed, so a job that ended and was reaped leaves nothing, and a thread that died
//!   inside a live job still counts.
//! - It does not refresh, for `vmstat`'s reason (milestone 106 (a wait that ends on either the interrupt or the deadline)).
//! - It counts `slabtop` itself, which is one of the jobs.
//! - The breakdown need not sum to the pages spent: a job region's unspent pages are spent from the
//!   budget's point of view and on nothing from the job's.
//!
//! Name: provisional, milestone 126's `free` lane, 2026-09-26, and flagged harder than `free`'s:
//! the upstream name promises a kernel-wide cache view, and this is one budget's breakdown.

#![cfg_attr(not(test), no_std)]

/// What `MemoryRegion::USAGE` answered for each record, in pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spending {
    /// `SIZE`.
    pub pages: u64,
    /// `COMMITTED`.
    pub committed: u64,
    /// `FRAMES`, over the subtree.
    pub frames: u64,
    /// `THREADS`, over the subtree.
    pub threads: u64,
    /// `ADDRESS_SPACES`, over the subtree.
    pub address_spaces: u64,
    /// `RENDEZVOUS`, over the subtree.
    pub rendezvous: u64,
}

/// The summary and the table, largest first. Ties keep the order above, so the output is
/// deterministic for a test and for a person comparing two runs.
pub fn write_report(s: &Spending, out: &mut dyn FnMut(&[u8])) {
    out(b"job budget: ");
    write_right(s.committed, 0, out);
    out(b" of ");
    write_right(s.pages, 0, out);
    out(b" pages spent\n");
    out(b"   PAGES      KIB  SPENT ON\n");
    let mut rows: [(u64, &[u8]); 4] = [
        (s.frames, b"frames"),
        (s.threads, b"threads"),
        (s.address_spaces, b"address spaces"),
        (s.rendezvous, b"rendezvous"),
    ];
    // Stable, and four rows: an insertion sort says so more plainly than a dependency would.
    for i in 1..rows.len() {
        let mut j = i;
        while j > 0 && rows[j - 1].0 < rows[j].0 {
            rows.swap(j - 1, j);
            j -= 1;
        }
    }
    for (pages, what) in rows {
        write_right(pages, 8, out);
        write_right(pages * (page_frames::FRAME_SIZE / 1024), 9, out);
        out(b"  ");
        out(what);
        out(b"\n");
    }
}

/// A `u64` right-aligned in `width` columns; `0` means no padding.
fn write_right(v: u64, width: usize, out: &mut dyn FnMut(&[u8])) {
    let mut buf = [b' '; 24];
    let mut i = buf.len();
    let mut v = v;
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let start = buf.len().saturating_sub(width.max(buf.len() - i));
    out(&buf[start..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(s: &Spending) -> String {
        let mut v = Vec::new();
        write_report(s, &mut |b| v.extend_from_slice(b));
        String::from_utf8(v).unwrap()
    }

    #[test]
    fn the_table_is_ranked_largest_first_with_the_budget_above_it() {
        let s = Spending {
            pages: 512,
            committed: 80,
            frames: 60,
            threads: 4,
            address_spaces: 4,
            rendezvous: 2,
        };
        assert_eq!(
            shown(&s),
            "job budget: 80 of 512 pages spent\n\
             \x20  PAGES      KIB  SPENT ON\n\
             \x20     60      240  frames\n\
             \x20      4       16  threads\n\
             \x20      4       16  address spaces\n\
             \x20      2        8  rendezvous\n"
        );
    }

    #[test]
    fn a_kind_with_nothing_spent_on_it_is_still_a_row() {
        let out = shown(&Spending::default());
        assert!(out.contains("       0        0  rendezvous"), "{out}");
    }
}
