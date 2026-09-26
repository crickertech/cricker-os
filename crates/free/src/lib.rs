//! **`free`: the machine's memory, and this prompt's share of it** (milestone 126 (the `procps` package), DECISIONS §225
//! (`free` sees the machine and your share)).
//!
//! The program's logic, lifted out so it runs on the host; `components/src/free.rs` is the two
//! reads and the two sinks. Two lines, and they are §225's two answers:
//!
//! - `Mem:` is the machine, from the machine statistics page (`crates/machine_statistics_protocol`),
//!   which the owner grants every login by default and can withhold.
//! - `Yours:` is this prompt's job budget, the region every job it runs is carved from, read
//!   through `MemoryRegion::USAGE` on a view narrowed to `ENUMERATE`. It is the limit that binds a
//!   program started here, which the machine line is not.
//!
//! Upstream `free` prints only the first line, and inside a container that line describes memory
//! the caller can never have; LXCFS exists to patch that (§225's prior art). Printing both, labelled,
//! answers the two readers at once.
//!
//! # EXAMPLES
//!
//! ```text
//! $ free
//!               total        used        free
//! Mem:         131072       20480      110592
//! Yours:         2048         320        1728
//! ```
//!
//! Figures are KiB, upstream's default unit. A prompt whose owner withheld the machine page prints
//! the `Yours:` line alone and says why on its second stream:
//!
//! ```text
//! $ free       free: the machine's owner has not granted the machine statistics page
//! ```
//!
//! # BUGS
//!
//! - No `shared`, `buff/cache` or `available` columns: the kernel has no page cache and no shared
//!   memory accounting to report, so `available` would only repeat `free`.
//! - No `Swap:` line, and not a line of zeroes. nife refuses paging out for now (calef,
//!   2026-09-26, the refusal in pull request #1356), and a zero would say swap exists and is
//!   empty. If the refusal is ever lifted, §225 says the line takes the same two-row shape.
//! - `Yours:` counts `free` itself, since it is one of the jobs carved from the budget, the way `ps`
//!   lists itself.
//! - The two lines are read at slightly different moments, and neither is a snapshot of the other.
//!
//! Name: provisional, milestone 126's `free` lane, 2026-09-26: upstream `procps`'s name for the
//! program a reader types to ask this.

#![cfg_attr(not(test), no_std)]

use machine_statistics_protocol::Snapshot;

/// This prompt's job budget, as `MemoryRegion::USAGE` answered: pages held, and pages spent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Share {
    /// `abi::usage::SIZE`.
    pub pages: u64,
    /// `abi::usage::COMMITTED`.
    pub committed: u64,
}

/// Why there is no `Mem:` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineRefusal {
    /// No capability at the slot: the owner withheld it.
    Withheld,
    /// A capability, and a page without the magic.
    Unrecognized,
}

/// What the machine read found: the page, or why not.
pub type Machine<'a> = Result<&'a Snapshot, MachineRefusal>;

/// Why there is no `Yours:` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShareRefusal {
    /// No view of the budget was granted.
    NotHeld,
    /// The kernel refused the question, with this error code.
    Refused(i64),
}

/// **The complaints, for the second stream**, one line each. Written before the report, under
/// DECISIONS §67 (a program's second stream is a declaration)'s order.
pub fn write_diagnostics(
    machine: Machine<'_>,
    share: Result<Share, ShareRefusal>,
    out: &mut dyn FnMut(&[u8]),
) {
    match machine {
        Ok(_) => {}
        Err(MachineRefusal::Withheld) => {
            out(b"free: the machine's owner has not granted the machine statistics page\n");
        }
        Err(MachineRefusal::Unrecognized) => {
            out(b"free: the machine statistics page is not one this program recognizes\n");
        }
    }
    match share {
        Ok(_) => {}
        Err(ShareRefusal::NotHeld) => out(b"free: this process holds no view of a job budget\n"),
        Err(ShareRefusal::Refused(code)) => {
            out(b"free: the kernel refused to say what the job budget spent (error ");
            write_signed(code, out);
            out(b")\n");
        }
    }
}

/// **The table**: a header and whichever of the two lines could be read. Nothing at all when
/// neither could, so `free > out.txt` on a machine that shows it nothing is an empty file rather
/// than a header over no rows.
pub fn write_report(
    machine: Machine<'_>,
    share: Result<Share, ShareRefusal>,
    out: &mut dyn FnMut(&[u8]),
) {
    let seen = machine.ok();
    if seen.is_none() && share.is_err() {
        return;
    }
    out(b"              total        used        free\n");
    if let Some(s) = seen {
        let total = s.total_bytes() / 1024;
        let free = s.free_bytes() / 1024;
        row(b"Mem:  ", total, total.saturating_sub(free), free, out);
    }
    if let Ok(sh) = share {
        let kib = page_frames::FRAME_SIZE / 1024;
        let total = sh.pages * kib;
        let used = sh.committed.min(sh.pages) * kib;
        row(b"Yours:", total, used, total - used, out);
    }
}

fn row(label: &[u8], total: u64, used: u64, free: u64, out: &mut dyn FnMut(&[u8])) {
    out(label);
    for v in [total, used, free] {
        write_right(v, 12, out);
    }
    out(b"\n");
}

/// A `u64` right-aligned in `width` columns.
pub fn write_right(v: u64, width: usize, out: &mut dyn FnMut(&[u8])) {
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

fn write_signed(v: i64, out: &mut dyn FnMut(&[u8])) {
    if v < 0 {
        out(b"-");
    }
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    let mut n = v.unsigned_abs();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out(&buf[i..]);
}

#[cfg(test)]
mod tests {
    use machine_statistics_protocol::{WORDS, build_header, word};

    use super::*;

    fn machine(total: u64, free: u64) -> Snapshot {
        let mut w = [0u64; WORDS];
        w[..word::LINE].copy_from_slice(&build_header(4096, 100));
        w[word::TOTAL_FRAMES] = total;
        w[word::FREE_FRAMES] = free;
        Snapshot::from_words(&w).unwrap()
    }

    fn shown(m: Machine<'_>, s: Result<Share, ShareRefusal>) -> (String, String) {
        let (mut out, mut diag) = (Vec::new(), Vec::new());
        write_diagnostics(m, s, &mut |b| diag.extend_from_slice(b));
        write_report(m, s, &mut |b| out.extend_from_slice(b));
        (
            String::from_utf8(out).unwrap(),
            String::from_utf8(diag).unwrap(),
        )
    }

    #[test]
    fn both_lines_read_as_upstream_free_does_plus_yours() {
        let (out, diag) = shown(
            Ok(&machine(32768, 27648)),
            Ok(Share {
                pages: 512,
                committed: 80,
            }),
        );
        assert_eq!(
            out,
            "              total        used        free\n\
             Mem:        131072       20480      110592\n\
             Yours:        2048         320        1728\n"
        );
        assert_eq!(diag, "");
    }

    /// §225's switch, seen from the reader: a withheld page is said, never printed as zeroes.
    #[test]
    fn a_withheld_machine_page_is_a_sentence_and_not_a_machine_of_zero_bytes() {
        let (out, diag) = shown(
            Err(MachineRefusal::Withheld),
            Ok(Share {
                pages: 512,
                committed: 80,
            }),
        );
        assert!(!out.contains("Mem:"), "{out}");
        assert!(out.contains("Yours:"), "{out}");
        assert!(diag.contains("owner has not granted"), "{diag}");
    }

    #[test]
    fn nothing_readable_prints_no_table_at_all() {
        let (out, diag) = shown(
            Err(MachineRefusal::Unrecognized),
            Err(ShareRefusal::Refused(-3)),
        );
        assert_eq!(out, "");
        assert!(diag.contains("not one this program recognizes"), "{diag}");
        assert!(diag.contains("(error -3)"), "{diag}");
    }

    #[test]
    fn a_budget_reporting_more_spent_than_held_does_not_underflow() {
        let (out, _) = shown(
            Err(MachineRefusal::Withheld),
            Ok(Share {
                pages: 4,
                committed: 9,
            }),
        );
        assert!(
            out.contains("Yours:          16          16           0"),
            "{out}"
        );
    }
}
