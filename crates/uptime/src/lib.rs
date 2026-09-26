//! **`uptime`: how long the machine has been counting** (milestone 126,
//! design/roadmap/126-who-else-is-running.md).
//!
//! This is the program's whole logic, lifted out so it runs on the host in milliseconds;
//! `components/src/uptime.rs` is the syscall and nothing else. The crate and the program share a name,
//! the same split `ps`, `line_editor` and `compositor` already are.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist), for this crate and the
//! program together. `uptime` is upstream `procps`'s own name for the program this replaces
//! (`dpkg -L procps` lists `/usr/bin/uptime`), which the naming tenet calls the best name
//! available for a standard term a reader already knows.
//!
//! **The narrowing was put to calef and did not change the answer.** This prints only elapsed
//! time, none of upstream's load average or logged-in-user count (see `BUGS`). That is a
//! narrowing rather than a contradiction: what `uptime` names is how long the machine has been up,
//! and this reports exactly that. Contrast `watch`, cut the same day, whose borrowed name promised
//! an arbitrary command line the capability model will not grant. A borrowed word is the best name
//! available when it is true and the worst when it is not.
//!
//! # Where the number comes from, and why it needed no new capability
//!
//! `components/src/uptime.rs` reads `user_mode_runtime::monotonic_nanos`, the same ambient counter `date` reads
//! to compute the wall clock and `os_primitives_benchmarker` reads to time itself. It is granted to
//! **every** EL0 program, unconditionally, by `kernel/src/arch/*/timer.rs`'s `init` (`CNTKCTL_EL1`'s
//! `EL0VCTEN` bit on aarch64, the RISC-V and `x86_64` equivalents), which documents the grant as **a
//! deliberate, eyes-open exception to DECISIONS §10's no-ambient-authority rule**: a monotonic
//! counter grants no authority to *affect* anything, only to observe the passage of time, and every
//! OS that offers userspace self-timing accepts the same side channel. Since that exception already
//! exists and already covers every process, `uptime` needed no manifest field, no new capability,
//! and no wiring beyond what `least_authority_demo` already has: the program that answers with
//! nothing but a
//! number turned out to need nothing but the counter.
//!
//! This is the one member of milestone 126's "machine-wide statistics" row (`free`, `uptime`,
//! `vmstat`) that turned out to be pure wiring rather than a design fork. `free` and `vmstat` read
//! physical memory accounting the kernel keeps for itself and has no path to userspace yet
//! (`kernel/src/memory.rs`'s `stats()`); see notes/process-view/what-is-left.md, section 3, for
//! why that is a different body of work and the fork it waits on.
//!
//! # EXAMPLES
//!
//! ```
//! use uptime::format;
//!
//! assert_eq!(format(0).as_bytes(), b"up 00:00:00\n");
//! assert_eq!(format(90 * 1_000_000_000).as_bytes(), b"up 00:01:30\n");
//! assert_eq!(format(90_000 * 1_000_000_000).as_bytes(), b"up 1 day, 01:00:00\n");
//! ```
//!
//! # BUGS
//!
//! - **No load average, no logged-in-user count.** Upstream `uptime` prints both; neither concept
//!   exists here. There is no scheduler-load figure this kernel maintains (`sched.rs` tracks
//!   preemptions and per-core run queues, not a decaying load average), and "logged in" presumes a
//!   login registry this system does not keep (every shell is a program somebody spawned, not a
//!   session in a table). Printing "up" alone rather than inventing placeholders for either is
//!   DECISIONS §42's no-silent-degradation rule applied to features rather than to values: a zero
//!   would read as a real answer.
//! - **The counter's zero predates the kernel's own init by an unmeasured amount.** `CNTVCT_EL0`
//!   (and its RISC-V/`x86_64` equivalents) is a free-running hardware counter set by firmware or the
//!   VMM at reset, not by this kernel; `date`'s own module docs call it "ambient and untouched" for
//!   the same reason. So "up" here means "since the counter started counting," which is at or
//!   before kernel entry, never after. Under QEMU the gap is small and unmeasured; on real hardware
//!   it is whatever the boot ROM and firmware took. Every OS that reports uptime from a free-running
//!   counter carries the same caveat; Linux's is `CLOCK_BOOTTIME`, deliberately not the raw TSC, and
//!   this program has no equivalent kernel-boot epoch to read.
//! - **One-second resolution**, `date`'s own limitation, for the same reason: `monotonic_nanos`
//!   is exact, but there is no sub-second field in the printed format to spend it on.

#![cfg_attr(not(test), no_std)]

const NANOS_PER_SEC: u64 = 1_000_000_000;
const SECS_PER_MIN: u64 = 60;
const SECS_PER_HOUR: u64 = 60 * SECS_PER_MIN;
const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;

/// The longest line [`format`] can produce: `u64::MAX` nanoseconds is about 584 years, and
/// `"up 213503982 days, 23:59:59\n"` is 29 bytes. Rounded up with room to spare.
pub const MAX_LEN: usize = 40;

/// A formatted line, owned inline: no allocator on this target.
pub struct Formatted {
    buf: [u8; MAX_LEN],
    len: usize,
}

impl Formatted {
    /// The formatted line, newline included, with no trailing garbage.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

/// Turn elapsed nanoseconds on the ambient monotonic counter into `up [D day[s], ]HH:MM:SS\n`.
///
/// Sub-day durations print no day count at all (`up 00:01:30\n`), rather than `up 0 days, ...`:
/// the zero case is the common one at a demo machine's uptime and Unix's own `uptime` elides it
/// too. Singular/plural is the one piece of English this program commits to, because "1 days" is
/// the detail that makes a status line read as generated rather than written.
pub fn format(nanos: u64) -> Formatted {
    let total_secs = nanos / NANOS_PER_SEC;
    let days = total_secs / SECS_PER_DAY;
    let hours = (total_secs % SECS_PER_DAY) / SECS_PER_HOUR;
    let mins = (total_secs % SECS_PER_HOUR) / SECS_PER_MIN;
    let secs = total_secs % SECS_PER_MIN;

    let mut w = Writer {
        buf: [0u8; MAX_LEN],
        len: 0,
    };
    w.push_bytes(b"up ");
    if days > 0 {
        w.push_u64(days);
        w.push_bytes(if days == 1 { b" day, " } else { b" days, " });
    }
    w.push_2digit(hours);
    w.push_bytes(b":");
    w.push_2digit(mins);
    w.push_bytes(b":");
    w.push_2digit(secs);
    w.push_bytes(b"\n");

    Formatted {
        buf: w.buf,
        len: w.len,
    }
}

/// A fixed-size append cursor, the same shape `date`'s and `wc`'s hand-rolled buffers use: no
/// allocator on this target, so every formatter here is one.
struct Writer {
    buf: [u8; MAX_LEN],
    len: usize,
}

impl Writer {
    fn push_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.len < self.buf.len() {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
    }

    /// Decimal, no leading zeros, no leading sign (every value this program formats is unsigned).
    fn push_u64(&mut self, mut v: u64) {
        let mut digits = [0u8; 20];
        let mut i = digits.len();
        loop {
            i -= 1;
            digits[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        self.push_bytes(&digits[i..]);
    }

    /// Zero-padded to exactly two digits. Every caller here passes a value already reduced modulo
    /// 24 or 60, so `v < 100` always holds; a wider value would print its low two digits rather
    /// than panic, matching this crate's no-panic-on-bad-input posture elsewhere.
    fn push_2digit(&mut self, v: u64) {
        let v = v % 100;
        self.push_bytes(&[b'0' + (v / 10) as u8, b'0' + (v % 10) as u8]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_up_zero() {
        assert_eq!(format(0).as_bytes(), b"up 00:00:00\n");
    }

    #[test]
    fn seconds_minutes_hours_all_render() {
        assert_eq!(format(5 * NANOS_PER_SEC).as_bytes(), b"up 00:00:05\n");
        assert_eq!(format(90 * NANOS_PER_SEC).as_bytes(), b"up 00:01:30\n");
        assert_eq!(format(3661 * NANOS_PER_SEC).as_bytes(), b"up 01:01:01\n");
    }

    #[test]
    fn a_day_rolls_over_and_pluralises() {
        // One full day plus one hour: singular "day".
        assert_eq!(
            format((SECS_PER_DAY + SECS_PER_HOUR) * NANOS_PER_SEC).as_bytes(),
            b"up 1 day, 01:00:00\n"
        );
        // Two days: plural.
        assert_eq!(
            format(2 * SECS_PER_DAY * NANOS_PER_SEC).as_bytes(),
            b"up 2 days, 00:00:00\n"
        );
    }

    #[test]
    fn sub_second_time_truncates_rather_than_rounds() {
        // 999,999,999 ns is still "0 seconds elapsed", not "1": this is a floor, matching how
        // date's own second-resolution read behaves.
        assert_eq!(format(NANOS_PER_SEC - 1).as_bytes(), b"up 00:00:00\n");
    }

    #[test]
    fn hours_and_minutes_never_reach_24_or_60_in_the_printed_field() {
        // 23 hours, 59 minutes, 59 seconds: the boundary just before a day rolls over.
        let secs = 23 * SECS_PER_HOUR + 59 * SECS_PER_MIN + 59;
        assert_eq!(format(secs * NANOS_PER_SEC).as_bytes(), b"up 23:59:59\n");
    }
}
