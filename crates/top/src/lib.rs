//! **`top`: the same domain `ps` lists, ordered by what it is costing** (milestone 282 (a thread's CPU time, and the `top` it makes possible),
//! DECISIONS §150 (how does a thread's CPU time reach userspace?), notes/process-view.md).
//!
//! This is the program's logic, lifted out so it runs on the host in milliseconds;
//! `components/src/top.rs` is the syscalls and the two output streams and nothing else. The crate
//! and the program share a name because they are one thing split at the IO boundary, the same pair
//! `ps`, `coremark` and `line_editor` already are.
//!
//! # The name had to be earned, and this is what earned it
//!
//! `top` means *the top N by resource consumption*: it is defined by ranking, and until there was
//! something to rank by the name would have overclaimed. `ps`'s table was `TID` and `STATE`, two
//! columns with no order to put them in, which is why milestone 126 (who else is running, and who is allowed to ask) shipped `ps` and
//! deliberately did not ship this. `abi::survey::record::CPU_TIME` is what changed, and it is a
//! **measurement** rather than an estimate: §150 refused wall-clock age (a thread that slept five
//! minutes and one that ran five minutes read identically) and refused userspace sampling (a thread
//! that runs between two samples is invisible), on the ground that a name like this one must not be
//! attached to a number that does not mean what the name says.
//!
//! # What this holds that `crates/ps` does not
//!
//! Almost nothing, and that is the honest answer rather than a modest one. The walk is
//! [`ps::collect`], the second walk that fills the CPU column is
//! [`ps::Survey::join_cpu_time`], the ordering is [`ps::Survey::rank_by_cpu_time`] and the table is
//! [`ps::Survey::write_report`]. What lives here is the **summary line**, which is the one thing a
//! ranked view needs and a listing does not: how many threads there are in total, what they are
//! doing, and how long the machine has been counting. Unix's `top` opens with the same three facts
//! for the same reason, which is that a ranked table shows you the head of a distribution and tells
//! you nothing about its size.
//!
//! **Whether this should be a program at all is an architect's**, and the argument is live rather
//! than settled. Milestone 281 (`watch` holds exactly what `ps` holds) deleted `watch` on the
//! finding that *two programs are two programs when they hold different authority*, and `top` holds
//! exactly `ps`'s three slots. What is different here is the question asked rather than the
//! authority held: `ps` answers *what exists*, in the kernel's own order, and `top` answers *what
//! is consuming*, in order of consumption. Folding it back into `ps` as a flag is a day's work and
//! stays available; see this crate's `BUGS`.
//!
//! # EXAMPLES
//!
//! ```text
//! $ top
//! up 00:01:12, 4 threads: 1 running, 1 ready, 1 blocked, 1 dead
//!          TID  STATE     TIME(ms)
//!            9  running        310
//! 4294967302  dead             150
//!            5  blocked         20
//!            7  ready            0
//! ```
//!
//! And the refusal, which reads exactly as `ps`'s does, because it is `ps`'s:
//!
//! ```text
//! $ top        top: this process holds no process-domain capability
//! ```
//!
//! The summary line is the host-testable part, and it needs no kernel:
//!
//! ```
//! use ps::Row;
//!
//! let rows = [
//!     Row { tid: 5, state: abi::survey::RUNNING, cpu_millis: Some(310) },
//!     Row { tid: 9, state: abi::survey::DEAD, cpu_millis: Some(10) },
//! ];
//! let mut text = Vec::new();
//! top::write_summary(&rows, 72_000_000_000, &mut |b| text.extend_from_slice(b));
//! let line = String::from_utf8(text).unwrap();
//! assert!(line.starts_with("up 00:01:12, 2 threads: "), "{line}");
//! assert!(line.contains("1 running"), "{line}");
//! ```
//!
//! # BUGS
//!
//! - **There is no live refresh, and that is a decision rather than a gap.** The obvious `top`
//!   redraws on an interval, and this kernel has **no timed wait**: the interval `watch` used was a
//!   yield-spin over the ambient monotonic counter, which burns a whole core for the length of the
//!   wait. That was merely wasteful in a program showing two columns of text. It is *corrupting*
//!   here, because the spin would be charged to this program's own thread by the very counter the
//!   table ranks on, and `top` would truthfully report itself as the busiest thread on the machine.
//!   A refresh arrives when a timed wait does (milestone 106 (a wait that ends on either the interrupt or the deadline)'s fork), not before.
//! - **No `%CPU` column**, for the same root cause. A percentage is a ratio of CPU time to elapsed
//!   time over an *interval*, so it needs two samples, and one invocation of this program is one
//!   sample. What is printed instead is the cumulative figure, which is Unix `top`'s own `TIME+`
//!   column and is the honest thing a single sample supports. A cumulative percentage against
//!   uptime was considered and refused: a thread born a second ago and one born at boot would be
//!   divided by the same denominator, which is the wall-clock-age error §150 refused, moved into the
//!   printer.
//! - **`top` cannot be asked for the top *N*.** `ArgSpec` (`crates/grant_plan`) is `Required` or
//!   `Forbidden` with nothing between, so a program with an optional integer cannot be declared, and
//!   a `top` that *required* one could not be typed bare. Every row is printed, ranked. This is the
//!   same boundary limitation `crates/pgrep`'s `BUGS` records for its missing pattern.
//! - **The summary counts what the survey returned, not what the machine holds.** A domain is the
//!   supervision subtree the caller was endowed, so "4 threads" means four in *this* domain; the
//!   idle threads, the kernel's own threads and every other domain are not in it and must not be.
//!   That is the difference from Unix's `top`, whose header counts the machine because `/proc` is
//!   ambient.
//! - **Ties are broken by tid, so an idle machine ranks in slot order.** With every figure at zero
//!   the "ranking" is no ranking at all, which is truthful and can read as a bug to somebody who
//!   typed `top` on a quiet system.
//!
//! Name: provisional. `top` is the name every reader already knows from outside this project, which
//! the naming tenet calls the best name available for a standard term, and it is the name DECISIONS
//! §150 (how does a thread's CPU time reach userspace?) and milestone 281 (`watch` holds exactly what `ps` holds) both anticipated for this program by name. It is
//! provisional because calef has not ruled on it and because the prior question is whether this is a
//! program or a flag on `ps`; see the module docs above.

#![cfg_attr(not(test), no_std)]

/// **The line above the table**: how long the machine has been counting, how many threads the
/// domain holds, and what they are doing.
///
/// Ends with a newline, and is written to the **output** stream rather than to diagnostics: it is
/// part of the answer, not a complaint about it. A caller that was refused prints neither this nor
/// the table (`ps::Survey::write_report` already declines), which keeps `top > out.txt` empty on a
/// refusal exactly as `ps > out.txt` is.
///
/// `uptime_nanos` is `user_mode_runtime::monotonic_nanos()`, the ambient counter every process
/// holds unconditionally (`kernel/src/arch/*/timer.rs`'s documented exception to DECISIONS §10 (process model: capability-based, microkernel)'s
/// no-ambient-authority rule). It is a parameter rather than a call so that this function is a pure
/// one and its output is a value a host test can assert on.
///
/// The state counts are in the order a reader scans for: what is on a CPU, what wants one, what is
/// waiting, what is over. A zero count is printed rather than elided, because a row missing from a
/// status line reads as a status line that did not check.
pub fn write_summary(rows: &[ps::Row], uptime_nanos: u64, out: &mut dyn FnMut(&[u8])) {
    // `uptime::format` is the same formatter `uptime` prints, reused rather than restated so the
    // two programs cannot come to disagree about what "up" means. It ends in a newline and this
    // line does not end here, so the newline is dropped.
    let up = uptime::format(uptime_nanos);
    let up = up.as_bytes();
    out(up.strip_suffix(b"\n").unwrap_or(up));
    out(b", ");
    write_u64(rows.len() as u64, out);
    out(b" threads: ");

    let mut first = true;
    for state in [
        abi::survey::RUNNING,
        abi::survey::READY,
        abi::survey::BLOCKED,
        abi::survey::DEAD,
    ] {
        if !first {
            out(b", ");
        }
        first = false;
        write_u64(rows.iter().filter(|r| r.state == state).count() as u64, out);
        out(b" ");
        out(ps::state_name(state).as_bytes());
    }
    out(b"\n");
}

/// A `u64` in decimal, no padding. The summary line is prose rather than a table, so its numbers
/// are not in columns.
fn write_u64(v: u64, out: &mut dyn FnMut(&[u8])) {
    let mut buf = [0u8; 20];
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
    out(&buf[i..]);
}

#[cfg(test)]
mod tests {
    use ps::Row;

    fn row(tid: u64, state: u64, ms: u64) -> Row {
        Row {
            tid,
            state,
            cpu_millis: Some(ms),
        }
    }

    fn shown(rows: &[Row], nanos: u64) -> String {
        let mut v = Vec::new();
        super::write_summary(rows, nanos, &mut |b| v.extend_from_slice(b));
        String::from_utf8(v).unwrap()
    }

    /// The whole line, asserted verbatim once. Every other test here checks one property; this one
    /// checks the shape, because a status line is read by a person and its punctuation is the
    /// difference between a sentence and a debug dump.
    #[test]
    fn the_summary_reads_as_a_sentence() {
        let rows = [
            row(3, abi::survey::RUNNING, 300),
            row(5, abi::survey::BLOCKED, 20),
            row(7, abi::survey::DEAD, 40),
            row(9, abi::survey::READY, 0),
        ];
        assert_eq!(
            shown(&rows, 72_000_000_000),
            "up 00:01:12, 4 threads: 1 running, 1 ready, 1 blocked, 1 dead\n",
        );
    }

    /// **A zero is printed rather than elided.** A status line that drops its empty categories
    /// reads as one that did not look, and the reader cannot tell "no corpses" from "corpses were
    /// not counted".
    #[test]
    fn a_state_with_nobody_in_it_still_reports_zero() {
        let line = shown(&[row(3, abi::survey::RUNNING, 10)], 0);
        assert!(line.contains("0 dead"), "{line}");
        assert!(line.contains("0 blocked"), "{line}");
    }

    /// An empty domain still produces a well-formed line. `ps` says "this domain holds no
    /// processes" on diagnostics and prints no table; this line is the output stream's half, and a
    /// caller that prints it is printing a truthful zero rather than nothing.
    #[test]
    fn an_empty_domain_summarises_to_zero_of_everything() {
        assert_eq!(
            shown(&[], 0),
            "up 00:00:00, 0 threads: 0 running, 0 ready, 0 blocked, 0 dead\n",
        );
    }

    /// The counts are of the survey, and a state this program does not know (a kernel newer than
    /// it) is counted in none of the four rather than miscounted into one of them. The total still
    /// names it, which is the pair that makes the discrepancy visible instead of silent.
    #[test]
    fn an_unknown_state_is_in_the_total_and_in_no_category() {
        let line = shown(&[row(3, 99, 0)], 0);
        assert!(line.contains("1 threads: "), "{line}");
        assert!(
            line.contains("0 running, 0 ready, 0 blocked, 0 dead"),
            "{line}"
        );
    }
}
