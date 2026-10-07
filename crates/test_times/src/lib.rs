//! **What each kernel test cost, as the lines the suite prints and `xtask` reads** (milestone 807
//! (the kernel suite reports what each test cost), §254 (a gate prints what each item cost), Fork 1).
//!
//! The kernel's test runner already stamps the timer before every test, because the per-test
//! ceiling needs a start time. Until milestone 807 it printed the elapsed figure only at 5 s and
//! over and threw the rest away. Now it keeps every figure and, after the suite's other reports and
//! before `test result:`, prints a block:
//!
//! ```text
//! time per test, in milliseconds:
//! time 3 kernel::memory::tests::frames_are_zeroed
//! time 10512 system_tests::timetable_tests::a_calendar_entry_keeps_time_by_its_granted_clock
//! ```
//!
//! The console is the record's channel because it is the only one aarch64, riscv64 and x86_64
//! share: x86_64 exits through `isa-debug-exit`, which has no file I/O, so semihosting a file out
//! would break §19 (architectural parity is a tenet). That is why the grammar lives here. Two
//! programs agree on it, the kernel that prints it and `xtask` that parses it, and rule 7 says such
//! an agreement is a crate.
//!
//! # The grammar
//!
//! A line is `time`, one space, the elapsed milliseconds in decimal, one space, the test's full
//! path as `core::any::type_name` gives it. No other spacing is accepted, and a path may not
//! contain whitespace. A trailing `\r` is tolerated because a serial transcript may carry one.
//!
//! The block opens with [`HEADING`] on a line of its own. The heading is not in §254's wording,
//! which rules on the line; it is there so a reader can tell "this image printed no record" from
//! "this image ran no tests", and so a test whose own output happens to begin with `time ` is never
//! read as a record line. `xtask` reads record lines only after the heading.
//!
//! Milliseconds because whole seconds round most of this suite to zero (§254). A `u64` of
//! milliseconds outlasts any suite.
//!
//! # EXAMPLES
//!
//! ```
//! use test_times::{Line, parse};
//!
//! let line = Line { milliseconds: 42, path: "kernel::sched::tests::a_thread_runs" };
//! let mut printed = String::new();
//! core::fmt::write(&mut printed, format_args!("{line}")).unwrap();
//! assert_eq!(printed, "time 42 kernel::sched::tests::a_thread_runs");
//! assert_eq!(parse(&printed), Ok(line));
//! ```
//!
//! # BUGS
//!
//! - **The figure is guest time.** Under TCG the guest's counter follows the host's clock, so a
//!   slow runner reads as a slow test. `xtask`'s cross-check bounds the guest clock against the
//!   host's, not against a fair machine.
//! - **The heading is a wording, not a rung of [`boot_ladder`]'s kind**, and nothing else keys on
//!   it. Changing it is a change to two programs at once, which this crate makes a compile error
//!   rather than a silent drift, since both take it from here.
//!
//! [`boot_ladder`]: ../boot_ladder/index.html
//!
//! Name: provisional, minted 2026-10-07 (UTC) by milestone 807's lane. §254 left the name to a
//! separate ratification. A plain plural noun for what it holds: each test's time.
//! Refused: `test_time_record`, the first draft, because `script/lint` reads a name ending in `d`
//! as a claim about what the thing is (§39 (a component is named for what it is)) and an allow-list entry was more
//! surface than a provisional name deserves; `test_timing`, which reads as a mechanism (a timer) rather than the record the two
//! programs exchange; `test_cost`, which is the milestone's word but would claim more than elapsed
//! time (the frame ledger is also a per-test cost, and it is not here).

#![no_std]

use core::fmt;

/// The line that opens the record block. Printed once per image, after the suite's other reports.
pub const HEADING: &str = "time per test, in milliseconds:";

/// What every record line starts with, its trailing space included so a match is on a boundary.
pub const PREFIX: &str = "time ";

/// One test's record: how long it ran, and which test it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line<'a> {
    /// The elapsed time from the stamp the runner takes before the test body to the one after it.
    pub milliseconds: u64,
    /// The test's full path, `core::any::type_name` of the test function.
    pub path: &'a str,
}

impl fmt::Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{PREFIX}{} {}", self.milliseconds, self.path)
    }
}

/// Why a line is not a record line. Each names the first thing that was wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Malformed {
    /// The line does not start with [`PREFIX`].
    NoPrefix,
    /// The milliseconds field is empty, is not all decimal digits, or does not fit a `u64`.
    BadMilliseconds,
    /// There is no test path after the milliseconds, or it is separated by something other than
    /// exactly one space.
    NoPath,
    /// The path contains whitespace, so it is not one `type_name`.
    SpaceInPath,
}

/// Read one record line.
///
/// # Errors
///
/// [`Malformed`] names the first thing wrong. A line that is not a record line at all is
/// [`Malformed::NoPrefix`], so a caller scanning a block can tell "not mine" from "mine but bad".
pub fn parse(line: &str) -> Result<Line<'_>, Malformed> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let rest = line.strip_prefix(PREFIX).ok_or(Malformed::NoPrefix)?;
    let (digits, path) = rest.split_once(' ').ok_or(Malformed::NoPath)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Malformed::BadMilliseconds);
    }
    let milliseconds = digits.parse().map_err(|_| Malformed::BadMilliseconds)?;
    if path.is_empty() || path.starts_with(' ') {
        return Err(Malformed::NoPath);
    }
    if path.chars().any(char::is_whitespace) {
        return Err(Malformed::SpaceInPath);
    }
    Ok(Line { milliseconds, path })
}

/// Counter ticks to whole milliseconds, rounding down.
///
/// In 128 bits because the product can outgrow 64: a 900 s ceiling on a 3 GHz TSC is 2.7e12 ticks,
/// times 1000 is 2.7e15, which fits, but nothing here should depend on a ceiling staying small. A
/// zero frequency (a timer that never calibrated) gives zero rather than a division trap, and
/// `xtask`'s cross-check is what notices a clock that lies.
#[must_use]
pub fn milliseconds(ticks: u64, frequency: u64) -> u64 {
    if frequency == 0 {
        return 0;
    }
    let ms = u128::from(ticks) * 1000 / u128::from(frequency);
    u64::try_from(ms).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::string::String;

    use super::*;

    fn printed(line: Line<'_>) -> String {
        let mut s = String::new();
        fmt::write(&mut s, format_args!("{line}")).unwrap();
        s
    }

    #[test]
    fn a_printed_line_parses_back_to_itself() {
        let line = Line {
            milliseconds: 10_512,
            path: "system_tests::timetable_tests::a_calendar_entry",
        };
        assert_eq!(parse(&printed(line)), Ok(line));
        let zero = Line {
            milliseconds: 0,
            path: "kernel::a",
        };
        assert_eq!(parse(&printed(zero)), Ok(zero));
    }

    #[test]
    fn a_serial_carriage_return_is_tolerated() {
        assert_eq!(
            parse("time 7 kernel::x\r"),
            Ok(Line {
                milliseconds: 7,
                path: "kernel::x"
            })
        );
    }

    #[test]
    fn malformed_lines_are_refused_with_the_first_fault() {
        assert_eq!(parse("test kernel::x ... ok"), Err(Malformed::NoPrefix));
        assert_eq!(
            parse("time per test, in milliseconds:"),
            Err(Malformed::BadMilliseconds)
        );
        assert_eq!(parse("time 12"), Err(Malformed::NoPath));
        assert_eq!(parse("time  12 kernel::x"), Err(Malformed::BadMilliseconds));
        assert_eq!(parse("time 12  kernel::x"), Err(Malformed::NoPath));
        assert_eq!(parse("time 12 "), Err(Malformed::NoPath));
        assert_eq!(parse("time -1 kernel::x"), Err(Malformed::BadMilliseconds));
        assert_eq!(parse("time +1 kernel::x"), Err(Malformed::BadMilliseconds));
        assert_eq!(parse("time 1.5 kernel::x"), Err(Malformed::BadMilliseconds));
        assert_eq!(
            parse("time 99999999999999999999 kernel::x"),
            Err(Malformed::BadMilliseconds)
        );
        assert_eq!(parse("time 3 kernel::x ok"), Err(Malformed::SpaceInPath));
        assert_eq!(parse("time 3 kernel::x\tok"), Err(Malformed::SpaceInPath));
    }

    #[test]
    fn the_heading_is_not_a_record_line() {
        // xtask reads record lines only after the heading; the heading itself must never parse.
        assert!(parse(HEADING).is_err());
    }

    #[test]
    fn ticks_convert_to_milliseconds_without_overflow() {
        assert_eq!(milliseconds(62_500_000, 62_500_000), 1000);
        assert_eq!(milliseconds(9_999_999, 10_000_000_000), 0);
        assert_eq!(milliseconds(15, 10_000), 1);
        assert_eq!(milliseconds(u64::MAX, 1), u64::MAX);
        assert_eq!(milliseconds(u64::MAX, 3_000_000_000), u64::MAX / 3_000_000);
        assert_eq!(milliseconds(5, 0), 0);
    }
}
