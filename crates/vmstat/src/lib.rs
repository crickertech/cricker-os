//! **`vmstat`: the machine's counters since boot** (milestone 126 (the `procps` package), DECISIONS §225 (`free` sees the
//! machine and your share)).
//!
//! The program's logic, lifted out so it runs on the host; `components/src/vmstat.rs` reads the
//! machine statistics page and the ambient clock and writes this. One report, which is upstream
//! `vmstat`'s first line: averages since boot.
//!
//! # The columns nife has a subject for, and the ones it does not
//!
//! | upstream | here | why |
//! |---|---|---|
//! | `r` | `r` | runnable threads over every core, sampled at each core's last tick |
//! | `b` | absent | no thread here sleeps uninterruptibly on I/O: the drivers are userspace programs |
//! | `swpd`, `si`, `so` | absent | nife refuses paging out for now (calef, 2026-09-26, pull request #1356), so a zero would say swap exists and is empty |
//! | `free` | `free` | the machine statistics page, KiB |
//! | `buff`, `cache` | `total` instead | the kernel keeps no buffer or page cache to report |
//! | `bi`, `bo` | absent | block I/O happens in userspace drivers the kernel does not count |
//! | `in`, `cs` | `in`, `cs` | per second since boot |
//! | `us`, `sy` | `busy` | the tick knows what ran, not which privilege level it interrupted |
//! | `id` | `id` | idle-thread ticks |
//! | `wa`, `st` | absent | no I/O wait and no hypervisor steal accounting |
//!
//! # EXAMPLES
//!
//! ```text
//! $ vmstat
//! procs ------memory (KiB)------ --system-- -cpu-
//!     r       free      total     in     cs busy  id
//!     1     110592     131072    104     35   12  88
//! ```
//!
//! # BUGS
//!
//! - No interval and no count: this kernel has no timed wait (milestone 106 (a wait that ends on either the interrupt or the deadline)), so a repeating
//!   `vmstat` would be a yield-spin that counted itself as the busiest thing on the machine, the
//!   finding that cut `watch`.
//! - `busy` is `us` and `sy` together; see the table.
//! - Each figure is a word read a moment apart from the others, not one snapshot.
//!
//! Name: provisional, milestone 126's `free` lane, 2026-09-26: upstream `procps`'s.

#![cfg_attr(not(test), no_std)]

use machine_statistics_protocol::Snapshot;

/// The two header lines and the one report line, or nothing if `machine` is `None`, since a
/// `vmstat` that cannot see the machine has nothing to tabulate. The caller says why on its
/// second stream.
pub fn write_report(machine: Option<&Snapshot>, uptime_nanos: u64, out: &mut dyn FnMut(&[u8])) {
    let Some(s) = machine else { return };
    let secs = (uptime_nanos / 1_000_000_000).max(1);
    let (busy, idle) = (s.busy_ticks(), s.idle_ticks());
    let ticks = busy + idle;
    let busy_pct = (busy * 100).checked_div(ticks).unwrap_or(0);
    let idle_pct = if ticks == 0 { 0 } else { 100 - busy_pct };

    out(b"procs ------memory (KiB)------ --system-- -cpu-\n");
    out(b"    r       free      total     in     cs busy  id\n");
    for (v, width) in [
        (s.runnable(), 5),
        (s.free_bytes() / 1024, 11),
        (s.total_bytes() / 1024, 11),
        (s.interrupts() / secs, 7),
        (s.context_switches() / secs, 7),
        (busy_pct, 5),
        (idle_pct, 4),
    ] {
        write_right(v, width, out);
    }
    out(b"\n");
}

/// The one complaint `vmstat` can have, for the second stream.
pub fn write_diagnostics(granted: bool, machine: Option<&Snapshot>, out: &mut dyn FnMut(&[u8])) {
    if machine.is_some() {
        return;
    }
    if granted {
        out(b"vmstat: the machine statistics page is not one this program recognizes\n");
    } else {
        out(b"vmstat: the machine's owner has not granted the machine statistics page\n");
    }
}

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
    use machine_statistics_protocol::{WORDS, build_header, word};

    use super::*;

    fn page() -> Snapshot {
        let mut w = [0u64; WORDS];
        w[..word::LINE].copy_from_slice(&build_header(4096, 100));
        w[word::TOTAL_FRAMES] = 32768;
        w[word::FREE_FRAMES] = 27648;
        for (cpu, busy, idle, intr, cs, r) in [(0, 10, 90, 520, 150, 1), (2, 14, 86, 520, 200, 0)] {
            let at = word::cpu(cpu);
            w[at + word::ONLINE] = 1;
            w[at + word::BUSY_TICKS] = busy;
            w[at + word::IDLE_TICKS] = idle;
            w[at + word::INTERRUPTS] = intr;
            w[at + word::CONTEXT_SWITCHES] = cs;
            w[at + word::RUNNABLE] = r;
        }
        Snapshot::from_words(&w).unwrap()
    }

    fn shown(m: Option<&Snapshot>, nanos: u64) -> String {
        let mut v = Vec::new();
        write_report(m, nanos, &mut |b| v.extend_from_slice(b));
        String::from_utf8(v).unwrap()
    }

    #[test]
    fn the_report_is_the_header_and_one_line_of_averages_since_boot() {
        assert_eq!(
            shown(Some(&page()), 10_000_000_000),
            "procs ------memory (KiB)------ --system-- -cpu-\n\
             \x20   r       free      total     in     cs busy  id\n\
             \x20   1     110592     131072    104     35   12  88\n"
        );
    }

    /// The first second of a boot must not divide by zero, and must not print a rate a thousand
    /// times too large either: under a second counts as one.
    #[test]
    fn a_boot_under_a_second_old_counts_as_one_second() {
        let line = shown(Some(&page()), 400_000_000);
        assert!(line.contains("   1040    350"), "{line}");
    }

    #[test]
    fn no_machine_page_prints_nothing_and_says_which_kind_of_nothing() {
        assert_eq!(shown(None, 0), "");
        let mut d = Vec::new();
        write_diagnostics(false, None, &mut |b| d.extend_from_slice(b));
        assert!(
            String::from_utf8(d)
                .unwrap()
                .contains("owner has not granted")
        );
    }
}
