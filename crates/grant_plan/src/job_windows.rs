//! **The file service's client windows, as the progenitor hands them to jobs** (milestone 599 (a
//! frame per filesystem client channel), milestone 685 (a job is finished when its memory is
//! back)).
//!
//! The progenitor gives every directory-granted job a window of its own: a page of the file
//! service's pool and the service's endpoint badged with the window's number. A window shared by
//! two live jobs is the shared-channel race the pool exists to close, and worse once the window is
//! bound (confinement claim 24 in `notes/confinement-claims.md`): the older job's badge resolves to
//! the newer job's directory. So a window must not go to a new job while its last holder lives.
//!
//! This is the pool's bookkeeping alone, pure logic so the rule is tested on the host. The
//! progenitor (`crates/system_initializer`) owns the capabilities and the binding.
//!
//! Name: provisional (milestone 685's lane).
//!
//! # BUGS
//!
//! - **Today's pool rotates without looking.** [`Windows::take`] hands out the next window round
//!   robin, whether or not the job that last held it has been reaped, because the progenitor is
//!   never told when a job dies. This module records the holder and ignores it; milestone 685's
//!   "reaped" message is what lets it stop ignoring it.

/// The most windows a pool can track. The file service's pool has seven
/// (`login_protocol::DURABLE_WINDOW` is the last), so this is room, not a target.
pub const MAX_WINDOWS: usize = 16;

/// **A job's label**: the badge the progenitor puts on the job's supervision capability, which the
/// kernel hands back beside the job's death message (DECISIONS §148 (resolves by asking the kernel), milestone 105 (the two forks)) and
/// `job_undertaker` forwards in its "reaped" message. Never `0`, which is "unlabelled".
pub type Label = u64;

/// **Which window the next directory-granted job gets, and who holds each one.**
pub struct Windows {
    /// The first window this pool hands out.
    first: u64,
    /// One past the last.
    end: u64,
    /// Where the rotation looks first.
    next: u64,
    /// `holder[w]` is the label of the job window `w` was last handed to and that has not been
    /// reaped, or `0` when the window is free.
    holder: [Label; MAX_WINDOWS],
    /// Bit `w` set means window `w`'s badge is bound to a grant and must be unbound before reuse
    /// (milestone 606 (a directory walk costs what it does on Linux), ruling D).
    bound: u64,
}

impl Windows {
    /// A pool handing out windows `first..end`.
    ///
    /// # Panics
    ///
    /// If the range is empty or reaches past [`MAX_WINDOWS`]: a pool that could hand out nothing,
    /// or a window this table cannot record, is a wiring mistake, caught where the pool is made.
    #[must_use]
    pub const fn new(first: u64, end: u64) -> Self {
        assert!(first < end && end <= MAX_WINDOWS as u64);
        Self {
            first,
            end,
            next: first,
            holder: [0; MAX_WINDOWS],
            bound: 0,
        }
    }

    /// **The window job `label` gets**, recorded as its holder.
    pub fn take(&mut self, label: Label) -> Option<u64> {
        let w = self.next;
        self.next = if w + 1 < self.end { w + 1 } else { self.first };
        self.holder[w as usize] = label;
        Some(w)
    }

    /// **Job `label` was reaped**: the window it held, if any, is free. Returns that window.
    pub fn reaped(&mut self, label: Label) -> Option<u64> {
        if label == 0 {
            return None;
        }
        let w = (self.first..self.end).find(|&w| self.holder[w as usize] == label)?;
        self.holder[w as usize] = 0;
        Some(w)
    }

    /// **Window `w` was taken and no job came to hold it** (the build or the start failed), so it is
    /// free now rather than at a reap that will never come.
    pub fn release(&mut self, w: u64) {
        if (self.first..self.end).contains(&w) {
            self.holder[w as usize] = 0;
        }
    }

    /// Whether window `w`'s badge is bound to a grant.
    #[must_use]
    pub const fn is_bound(&self, w: u64) -> bool {
        self.bound & (1 << w) != 0
    }

    /// Record whether window `w`'s badge is bound.
    pub fn set_bound(&mut self, w: u64, bound: bool) {
        if bound {
            self.bound |= 1 << w;
        } else {
            self.bound &= !(1 << w);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;

    /// The file service's pool as the progenitor makes it: windows 1 to 6 (0 is the boot's
    /// long-lived clients', 7 is `login`'s durable sessions').
    fn pool() -> Windows {
        Windows::new(1, 7)
    }

    /// **`take` never hands out a window whose last holder is unreaped** (confinement claim 24's
    /// fix, milestone 685). Six jobs hold all six windows and none has been reaped, so a seventh
    /// gets nothing; once one is reaped, its window and only its window comes back.
    #[test]
    fn take_never_hands_out_a_window_whose_last_holder_is_unreaped() {
        let mut p = pool();
        let held: Vec<u64> = (1..=6)
            .map(|job| p.take(job).expect("six windows for six jobs"))
            .collect();
        assert_eq!(
            p.take(7),
            None,
            "a seventh job got a window while all six holders were unreaped"
        );
        assert_eq!(
            p.reaped(3),
            Some(held[2]),
            "job 3's window did not come back"
        );
        assert_eq!(
            p.take(8),
            Some(held[2]),
            "the reaped job's window was not handed out again"
        );
        assert_eq!(p.take(9), None, "a window came back that no reap released");
    }

    /// **Rotation still spreads reuse, and skips the windows still held.** Jobs 1 and 2 hold the
    /// first two windows, job 1 is reaped, and four more jobs come: none of them may get job 2's.
    #[test]
    fn take_skips_a_held_window_on_its_way_round() {
        let mut p = pool();
        let a = p.take(1).unwrap();
        let b = p.take(2).unwrap();
        p.reaped(1);
        for job in 3..=6 {
            let w = p.take(job).expect("a free window remained");
            assert_ne!(w, b, "job {job} got the window job 2 still holds");
        }
        let w = p.take(7).expect("job 1's window is free");
        assert_eq!(w, a);
    }

    /// **A window whose job never started is free at once**, and a reap of a label nobody holds, or
    /// of `0`, frees nothing.
    #[test]
    fn release_frees_and_a_stranger_reap_does_not() {
        let mut p = Windows::new(1, 2);
        let w = p.take(1).unwrap();
        assert_eq!(p.reaped(0), None);
        assert_eq!(p.reaped(99), None);
        assert_eq!(p.take(2), None, "the one window is still held by job 1");
        p.release(w);
        assert_eq!(p.take(2), Some(w));
    }
}
