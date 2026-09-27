//! **The TSC calibration's estimator, checked where it is a decision rather than a device.**
//!
//! `arch::x86_64::timer` is nearly all `in8` and `out8` against the 8254, which nothing but a real
//! boot can exercise. The one part that is neither is the rule that decides when enough windows
//! have been timed, and it is the part that was wrong: the module shipped for months taking
//! exactly one window and dividing by exactly ten milliseconds, which is an estimator with an
//! unbounded one-sided error and no test could have said so, because there was no estimator to
//! test.
//!
//! So these are in their own file rather than in `timer.rs`, and they come in two kinds. The
//! **estimator** tests feed the stopping rule sequences written by hand, including the ones a real
//! host is too polite to produce, and assert what it decides. The **boot** tests are the two
//! assertions that need a machine: that the windows were actually timed, and that the number the
//! rest of the kernel reads is the smallest of them.
//!
//! **They all run under QEMU rather than on the host**, which is not where this tree would
//! normally put pure logic. `timer.rs` compiles only for `x86_64-unknown-none`, so an aarch64
//! development machine's `cargo test` cannot reach the stopping rule at all without moving it into
//! a crate, and there is no second consumer to justify one (AGENTS.md's rule 7 is about what two
//! binaries must agree on). The marginal cost is nil because `script/test --arch x86_64` boots
//! this kernel anyway. If the rule ever gains a second caller, it belongs in
//! `crates/counter_frequency_protocol`, which already holds `is_plausible` and is the same kind of
//! thing.
//!
//! See design/roadmap/571-the-x86-boot-calibrates-once-and-can-be-wrong-by-4x.md (the x86 boot
//! calibrates the TSC once, and can be wrong by 4x) for the measured distributions that chose the
//! cap and the tolerance.
//!
//! # BUGS
//!
//! - **Nothing here can prove the calibration is accurate**, only that the estimator is the one
//!   intended. Accuracy needs an independent clock, which is `arch::x86_64::tsc_probe` behind the
//!   off-by-default `tsc_probe` feature, and it costs thirty-five seconds of wall clock per boot,
//!   which is why it is an instrument rather than a test. notes/tsc-under-tcg.md is its owner.
//! - **The boot tests cannot fail on a machine that reports `CPUID` leaf 0x15**, because the
//!   stored rate then comes from the part rather than from these windows. No machine this runs on
//!   today does (TCG does not offer the leaf), so the skip has never been taken; it is written as
//!   a skip rather than an assertion so that the day it is taken, the suite says so instead of
//!   going red for the wrong reason.

#[cfg(any(test, feature = "system_tests"))]
mod estimator {
    //! The stopping rule, fed sequences by hand. No device access, so nothing here can flake.

    use crate::arch::x86_64::timer::calibration_has_converged;

    /// A clean 1 GHz window, in raw PIT-window TSC delta units (10 ms of a 1 GHz counter).
    const CLEAN: u64 = 10_000_000;

    /// **One window is never enough**, which is the defect this replaced stated as a test. The
    /// rule needs two samples to agree before it can conclude anything, so a single sample, however
    /// plausible it looks, cannot end the calibration.
    #[test_case]
    fn one_window_never_converges() {
        assert!(!calibration_has_converged(&[]));
        assert!(!calibration_has_converged(&[CLEAN]));
    }

    /// **Two windows that agree end it**, which is the common case and the reason the mean cost is
    /// three or four windows rather than the cap.
    #[test_case]
    fn two_agreeing_windows_converge() {
        assert!(calibration_has_converged(&[CLEAN, CLEAN]));
        // One part in 2000 apart: well inside the tolerance.
        assert!(calibration_has_converged(&[CLEAN, CLEAN + CLEAN / 2000]));
    }

    /// **Two windows that disagree do not**, and the case worth pinning is the one the old code
    /// got wrong: a wildly inflated window must not be allowed to satisfy anything. A descheduled
    /// window is inflated by tens of per cent, nowhere near one part in a thousand.
    #[test_case]
    fn a_descheduled_window_does_not_satisfy_the_rule() {
        assert!(!calibration_has_converged(&[CLEAN, CLEAN * 4]));
        assert!(!calibration_has_converged(&[CLEAN * 4, CLEAN]));
        // Two separate deschedulings that happen to be close to each other are still not
        // agreement with the *minimum*, which is what the rule compares against.
        assert!(!calibration_has_converged(&[CLEAN, CLEAN * 3, CLEAN * 3]));
    }

    /// **The agreeing pair need not be adjacent.** This is why the rule compares against the
    /// running minimum rather than against the previous sample, and a version that compared
    /// neighbours would keep timing windows here for no reason.
    #[test_case]
    fn the_agreeing_pair_need_not_be_adjacent() {
        assert!(calibration_has_converged(&[CLEAN, CLEAN * 5, CLEAN]));
        assert!(!calibration_has_converged(&[CLEAN, CLEAN * 5]));
    }

    /// **Agreement is judged against the smallest sample, not the largest**, so a sequence that
    /// converges late is not fooled by two large windows that happen to be similar. The truth is
    /// at or below the minimum; a pair agreeing high says only that the host was busy twice.
    #[test_case]
    fn agreement_is_judged_against_the_smallest() {
        // 4x and 4.001x agree with each other but not with the 1x minimum.
        let nearly_equal_but_wrong = CLEAN * 4 + CLEAN / 200;
        assert!(!calibration_has_converged(&[
            CLEAN,
            CLEAN * 4,
            nearly_equal_but_wrong
        ]));
    }
}

#[cfg(any(test, feature = "system_tests"))]
mod boot {
    //! The tests that need a machine, run by `script/test --arch x86_64`.

    use crate::arch::timer;

    /// **The calibration took more than one window and fewer than the cap**, which is the whole
    /// shape of the fix in one assertion.
    ///
    /// Two is the floor because the stopping rule needs two windows to agree before it can stop,
    /// so a boot that reports one timed window is a boot where the loop did not run. The cap is
    /// the ceiling by construction. Reaching the cap is not a failure (a host fighting hard enough
    /// will), so this asserts the range rather than a number: a test that demanded three would go
    /// red on a loaded CI runner for doing exactly the right thing.
    #[test_case]
    fn the_calibration_timed_several_windows() {
        let calibration = timer::calibration();
        let windows = calibration.windows();

        assert!(
            windows.len() >= 2,
            "the stopping rule cannot fire before two windows, so {} means the loop did not run",
            windows.len()
        );
        assert!(
            windows.iter().all(|&hz| hz != 0),
            "a timed window with a zero rate is a PIT that never counted",
        );
    }

    /// **The rate the kernel reads is the smallest window, not the first, the last or the mean.**
    ///
    /// This is the assertion the old code would have failed, and it is worth stating as an
    /// identity rather than a bound: every window is an upper bound on the truth, so anything
    /// other than the minimum is knowingly reporting a rate that is too high.
    #[test_case]
    fn the_stored_rate_is_the_smallest_window() {
        if timer::frequency_source() != "PIT calibration" {
            // The part stated its own rate; these windows were timed for the APIC timer and not
            // used for the TSC. See this file's BUGS.
            return;
        }

        let calibration = timer::calibration();
        let smallest = calibration
            .windows()
            .iter()
            .copied()
            .min()
            .expect("the test above proves there is at least one");

        assert_eq!(
            timer::frequency(),
            smallest,
            "the stored rate must be the best window; worst was {}",
            calibration.worst(),
        );
        assert!(
            calibration.worst() >= timer::frequency(),
            "the worst window cannot be below the best one",
        );
    }
}
