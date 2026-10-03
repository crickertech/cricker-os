//! **`ps`: what a process listing is when there is no `/proc`** (milestone 126,
//! notes/process-view.md).
//!
//! This is the program's logic, lifted out so it runs on the host in milliseconds; `components/src/ps.rs`
//! is the syscall and the two output streams and nothing else. The crate and the program share a
//! name because they are one thing split at the IO boundary, which is the convention `coremark`,
//! `line_editor` and `compositor` already follow.
//!
//! # What it does not do, which is the whole point
//!
//! `ps aux` on Linux reads `/proc`, which is **ambient**: any process gets it with no grant from
//! anyone, so the listing is every process on the machine, including the command lines with
//! secrets in `argv`. Nobody defends that; `hidepid` exists because enough people stopped wanting
//! it.
//!
//! Here the listing is a **capability**. What this crate is handed is a function that reads one
//! entry of a domain, and that function is backed by `abi::rendezvous::SURVEY` on a supervision
//! endpoint the program was endowed. It cannot widen the domain, cannot ask about a tid it was not
//! shown, and cannot discover that any other domain exists. There is no path here that reaches a
//! process this program's caller did not already have authority over, because the only input is the
//! reader itself.
//!
//! # Three answers, and telling them apart is the deliverable
//!
//! A monitor that reports nothing because it **could not look** is the worst failure this tool has
//! available: it reads exactly like a quiet machine. So the three cases are kept distinct all the
//! way to the terminal:
//!
//! - **A domain with processes in it.** A table on the output stream.
//! - **A domain that is empty.** No rows, and a line on the *diagnostics* stream saying the domain
//!   is empty. Not an error: the caller's authority was never in question.
//! - **A refusal.** No rows at all, and a line on diagnostics naming the reason. `filesystem_protocol` chose
//!   `EPERM` over an empty listing for this exact reason, and so does this.
//!
//! # Collect first, complain second, print third
//!
//! DECISIONS §67 gives a program a second output stream and one rule with it: everything it has to
//! complain about is said, and that stream closed, **before** it writes a byte of output. Its reader
//! is single-threaded and drains diagnostics to end-of-stream first, so a program that interleaved
//! the two would block in a rendezvous nobody is listening for.
//!
//! A survey cannot know its complaints up front: an endpoint can be destroyed halfway through a
//! walk. So [`collect`] takes the whole domain into a buffer first, and the caller then emits
//! [`Survey::write_diagnostics`] and [`Survey::write_report`] in that order.
//!
//! **The buffer is the caller's**, which is not ceremony. It was a `[Row; MAX_ROWS]` local until
//! `script/stack-frame-check` failed the build: two kilobytes of rows made `collect`'s frame 4,336
//! bytes, larger than the 4,096-byte guard page under every kernel thread stack, so one call could
//! move `sp` past the guard in a single step and write into a neighbouring thread's stack without
//! ever faulting. A caller-provided slice is the fix that gate recommends, and it is better anyway:
//! a program that sizes its own listing knows where the memory came from.
//!
//! Sizing it at [`MAX_ROWS`] makes truncation unreachable, because that is the kernel's whole thread
//! table. `ps` does exactly that. A shorter buffer is allowed and is **not silent**: the survey says
//! it was truncated, on diagnostics, for the same reason a refusal is not an empty list.
//!
//! # EXAMPLES
//!
//! At the prompt, in a domain holding two jobs and `ps` itself:
//!
//! ```text
//! $ ps
//!          TID  STATE     TIME(ms)
//!            5  blocked         20
//!            9  running        310
//! 4294967302  dead             150
//! ```
//!
//! The empty and the refused cases, which differ in a way `ps` on Linux has no way to express:
//!
//! ```text
//! $ ps                 ps: this domain holds no processes
//! $ ps                 ps: this endpoint does not carry the right to look
//! ```
//!
//! Driving it from a host test is the whole contract, and it needs no kernel:
//!
//! ```
//! use ps::{MAX_ROWS, Row, collect};
//!
//! // A domain of one running thread: cursor 0 yields it, cursor 1 says done.
//! let mut reader = |cursor: u64| match cursor {
//!     0 => (1, 7, abi::survey::RUNNING),
//!     _ => (abi::survey::DONE as i64, 0, 0),
//! };
//! let mut rows = [Row::default(); MAX_ROWS];
//! let survey = collect(&mut rows, &mut reader);
//! assert_eq!(survey.rows().len(), 1);
//! assert_eq!(survey.rows()[0].tid, 7);
//!
//! let mut text = Vec::new();
//! survey.write_report(&mut |b| text.extend_from_slice(b));
//! assert!(String::from_utf8(text).unwrap().contains("running"));
//! ```
//!
//! # BUGS
//!
//! - **A process has no name here, so there is no `CMD` column.** This system has `arg0` in `Spawn`
//!   and no display name at all, so the columns are the tid and the run state and that is
//!   everything. A name is information rather than authority, but a confined viewer may still not
//!   be entitled to it and there is no design for that today; a `CMD` column that appeared without
//!   one would be a leak wearing a familiar heading.
//! - **The tid is a generational name, so it is a large and ugly number** after any slot reuse
//!   (`(generation << 32) | slot`, `crates/slots`). It is printed as the one integer
//!   `abi::rendezvous::REAP` would accept, because splitting it into `gen:slot` would publish the
//!   shape of the kernel's thread table to every program that can run `ps`.
//! - **`ps` itself does not sort.** Rows come out in the kernel's slot order, which is neither
//!   creation order nor tid order once a slot has been reused. There is now something to sort by
//!   (milestone 282 (a thread's CPU time, and the `top` it makes possible)) and
//!   [`Survey::rank_by_cpu_time`] does it, but `ps` does not call it: ranking is what makes a
//!   listing a `top`, and `components/src/top.rs` is the program that does. `ps` has no flag for it
//!   because `ArgSpec` carries one integer and this program spends none, the same limit
//!   `crates/pgrep`'s `BUGS` names for its missing pattern.
//! - **The `TIME` column is two walks, not one.** `abi::rendezvous::SURVEY` answers one record per
//!   call, so a listing with both the state and the CPU time asks the kernel for each thread twice
//!   and joins on the tid ([`Survey::join_cpu_time`]). A thread that dies between the two walks has
//!   a row with no figure in it, printed as `-` rather than as a zero. DECISIONS §204 (how userspace asks where a thread runs) priced the
//!   extra syscall at about 45 microseconds per second for sixty-four threads.
//! - **The state is a snapshot per row, not per table.** See the `BUGS` section of
//!   notes/process-view.md: a row read early in the walk may be stale by the time the table prints.
//! - **No automated run spawns `ps` or `pgrep`.** This crate's own logic has host tests
//!   and `kernel::user::survey_tests` covers the syscall underneath, but both shipped
//!   programs are reachable only from an interactive prompt (`script/shell`): neither
//!   `script/test` nor `script/swish-check` types them. (It said "three" and named `watch` until
//!   milestone 281 deleted that program.) Found 2026-08-27 while raising
//!   [`MAX_ROWS`] with the kernel's thread ceiling, which left a static measurement
//!   (`-Z emit-stack-sizes` against `system_initializer::CHILD_STACK_PAGES`; the numbers are on
//!   [`MAX_ROWS`]) as the only check that the bigger stack-resident buffer still fits. That is a
//!   real check and it is not a run, so anything that only shows up when the program executes has
//!   nothing standing in its way.
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). `ps` is the name
//! every reader already knows from outside this project, which the naming tenet calls the best
//! name available and not one to spend a rename on. Sharing it with `components/src/ps.rs` is the
//! crate-and-program pair the same tenet describes: splitting them would hide the relationship, so
//! `coremark`, `line_editor` and `compositor` all keep one name across the two.

#![cfg_attr(not(test), no_std)]

/// The widest domain a survey can produce: the kernel's entire thread table (`MAX_THREADS` in
/// `kernel/src/sched.rs`).
///
/// **A buffer this size makes truncation unreachable rather than handled.** A `ps` holding the
/// widest grant this system can express sees every thread on the machine, so there is no "and N
/// more" the shipped program can ever print. If the kernel's table grows, this is the number that
/// moves with it, and a compile-time assertion in `kernel::user::survey_tests` is what keeps the
/// two in step rather than a reader.
///
/// **128 from milestone 126 until 2026-08-27**, when the kernel's ceiling doubled on a measured
/// peak (`sched::MAX_THREADS` carries the numbers) and this followed it, which is that assertion
/// working as designed. The raise is not free here, because a `[Row; MAX_ROWS]` is a stack
/// allocation in both shipped programs, so it was measured rather than waved through:
/// `-Z emit-stack-sizes` puts `_start` at 4,240 bytes in `ps` and 4,320 in `pgrep`, against the 12
/// pages (49,152 bytes) `system_initializer::CHILD_STACK_PAGES` maps under every child. Under 9% of
/// the stack each, and nothing else in these programs is deep: `collect` is 368 bytes and the next
/// frame down is smaller still.
///
/// **It was three programs and a worse number until milestone 281.** `watch` held *two* of these
/// buffers, one across its whole run and one declared inside its redraw loop, and measured 8,464
/// bytes: 17% of the stack, the worst of the three. It was deleted rather than fixed, and the
/// figures above are what is left.
pub const MAX_ROWS: usize = 256;

/// One line of the listing: a thread and what it is doing.
///
/// Two fields, because two facts is what a supervision domain honestly knows about its members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Row {
    /// The kernel-stamped thread id, the same name `abi::rendezvous::REAP` accepts.
    pub tid: u64,
    /// One of `abi::survey`'s state codes.
    pub state: u64,
    /// **Scheduled on-CPU time in milliseconds**, or `None` when nobody asked for it.
    ///
    /// `None` rather than `0`, and the difference is the whole reason this is an `Option`.
    /// [`collect`] performs one walk, for the state record, because that is all `pgrep` and a plain
    /// listing want; the figure arrives only if a caller then runs
    /// [`Survey::join_cpu_time`](Survey::join_cpu_time). A zero here would be a thread that has
    /// genuinely never been on a CPU at a tick, which is a different fact from "not asked", and a
    /// column that printed them the same way would be this crate's own `BUGS` section coming true.
    ///
    /// It also stays `None` for a row whose tid was in the first walk and not the second, which is
    /// a thread that died in between. See [`Survey::join_cpu_time`].
    pub cpu_millis: Option<u64>,
}

/// A finished walk of one domain: the rows, and whatever went wrong.
///
/// Built by [`collect`], which is the only constructor, so a `Survey` always describes a walk that
/// really happened.
pub struct Survey<'a> {
    /// Mutable because a survey is written to after it is collected: a second walk fills
    /// [`Row::cpu_millis`] in ([`Survey::join_cpu_time`]) and a ranking view reorders it
    /// ([`Survey::rank_by_cpu_time`]). The borrow is still the caller's buffer; nothing here
    /// allocates.
    rows: &'a mut [Row],
    /// The negated `abi::Error` that ended the walk, if one did. `Some` here means **the listing is
    /// not the domain**: it is however far the walk got, which is why the caller prints the reason
    /// rather than the partial table.
    refused: Option<i64>,
    /// The kernel handed back a cursor that did not advance. Not reachable from the shipped kernel
    /// and checked anyway, because a `collect` that could loop forever on a bad reader would not be
    /// a total function and could not be fuzzed or property-tested.
    stalled: bool,
    /// The buffer filled and the domain had more in it. Unreachable for a caller that sized its
    /// buffer at [`MAX_ROWS`], which `ps` does; reported rather than silent for one that did not.
    truncated: bool,
}

/// **Walk a domain to its end and keep what it says.**
///
/// `read(cursor)` is one `abi::rendezvous::SURVEY`: it answers `(next_cursor, tid, state)`, where a
/// negative first word is a refusal and `abi::survey::DONE` means the walk is over. Start at 0, feed
/// each `next_cursor` back; this function is that loop.
///
/// **It is deliberately a closure rather than a syscall**, which is what makes every case here
/// reachable from a host test: the empty domain, the refusal, and a domain of a hundred threads all
/// cost microseconds and no emulator.
///
/// The walk stops at the first refusal and reports it, rather than skipping the entry and carrying
/// on. A survey that silently dropped an entry it could not read would be the failure this whole
/// program exists to avoid, one row smaller.
pub fn collect<'a>(
    rows: &'a mut [Row],
    read: &mut dyn FnMut(u64) -> (i64, u64, u64),
) -> Survey<'a> {
    let mut n = 0usize;
    let mut refused = None;
    let mut stalled = false;
    let mut truncated = false;
    let mut cursor = 0u64;
    loop {
        let (next, tid, state) = read(cursor);
        if next < 0 {
            refused = Some(next);
            break;
        }
        let next = next as u64;
        if next == abi::survey::DONE {
            break;
        }
        // A cursor that did not advance would spin here forever. The kernel is inside the trusted
        // base and does not do this; the check exists so that this function terminates for *every*
        // reader, including a test's.
        if next <= cursor {
            stalled = true;
            break;
        }
        if n == rows.len() {
            truncated = true;
            break;
        }
        rows[n] = Row {
            tid,
            state,
            cpu_millis: None,
        };
        n += 1;
        cursor = next;
    }
    Survey {
        rows: &mut rows[..n],
        refused,
        stalled,
        truncated,
    }
}

impl Survey<'_> {
    /// The rows, in the order the kernel reported them unless [`rank_by_cpu_time`](Self::rank_by_cpu_time)
    /// has reordered them.
    pub fn rows(&self) -> &[Row] {
        self.rows
    }

    /// **Fill in [`Row::cpu_millis`] from a second walk of the same domain** (milestone 282 (a thread's CPU time, and the `top` it makes possible),
    /// `abi::survey::record::CPU_TIME`).
    ///
    /// `read(cursor)` is one `SURVEY` asking for the CPU-time record, exactly as the closure
    /// [`collect`] takes is one asking for the state record. The caller binds the record, not this
    /// function, because a program that wants a third fact tomorrow joins it the same way.
    ///
    /// **Two walks joined on the tid, rather than one walk carrying two facts**, which is the shape
    /// DECISIONS §204 (how userspace asks where a thread runs) chose when it made the record a selector: the cursor and the tid are the
    /// same whichever record is asked for, so only the third word moves, and a program that wants
    /// one fact pays nothing for the existence of the others. The cost is one extra syscall per
    /// thread, which §204 priced at about 45 microseconds per second for sixty-four threads at a
    /// one-second refresh.
    ///
    /// **A domain can change between the two walks**, and this is where that shows. A tid in the
    /// first walk and not the second is a thread that died in between: its row keeps `None` and the
    /// listing prints that it does not know, rather than borrowing a number from another row. A tid
    /// in the second walk and not the first is simply ignored, because there is no row to put it
    /// in. That is `readdir`'s bargain, the same one a single walk already takes between its own
    /// calls (see the `BUGS` section of notes/process-view.md), and it is stated here rather than
    /// hidden because the failure it prevents is a plausible wrong number.
    ///
    /// A refusal in the second walk is recorded exactly as one in the first is, so a caller that
    /// checks [`is_refused`](Self::is_refused) afterwards is told rather than shown a table with a
    /// blank column.
    pub fn join_cpu_time(&mut self, read: &mut dyn FnMut(u64) -> (i64, u64, u64)) {
        let mut cursor = 0u64;
        loop {
            let (next, tid, millis) = read(cursor);
            if next < 0 {
                self.refused = Some(next);
                return;
            }
            let next = next as u64;
            if next == abi::survey::DONE {
                return;
            }
            // The same non-advancing-cursor guard `collect` carries, and for the same reason: this
            // loop must terminate for every reader, including a test's.
            if next <= cursor {
                self.stalled = true;
                return;
            }
            if let Some(row) = self.rows.iter_mut().find(|r| r.tid == tid) {
                row.cpu_millis = Some(millis);
            }
            cursor = next;
        }
    }

    /// **Order the rows by CPU time, most first**, which is what makes a listing a `top`.
    ///
    /// Ties break on the tid, ascending, so the order is total and a redraw of an idle machine does
    /// not shuffle. A row with no figure (see [`Row::cpu_millis`]) sorts last, because "not known"
    /// is not "none used" and putting it at the head would be the strongest possible claim about
    /// the one row that has nothing to say.
    ///
    /// `sort_unstable_by` allocates nothing, which is why this can happen in a `no_std` program on
    /// a twelve-page stack.
    pub fn rank_by_cpu_time(&mut self) {
        self.rows.sort_unstable_by(|a, b| {
            b.cpu_millis
                .unwrap_or(0)
                .cmp(&a.cpu_millis.unwrap_or(0))
                .then(a.cpu_millis.is_none().cmp(&b.cpu_millis.is_none()))
                .then(a.tid.cmp(&b.tid))
        });
    }

    /// **Was this a refusal?** True when the domain could not be read at all or not to its end. A
    /// caller must not print an empty table for this: see the crate docs.
    pub fn is_refused(&self) -> bool {
        self.refused.is_some() || self.stalled
    }

    /// **Is this listing the whole domain?** False when the walk was refused, stalled, or ran out of
    /// buffer. A caller that prints the table anyway is printing something it cannot vouch for.
    pub fn is_complete(&self) -> bool {
        !self.is_refused() && !self.truncated
    }

    /// **What there is to complain about, as a clause a program name prefixes**, or `None` when the
    /// walk succeeded and found processes.
    ///
    /// Split out of [`write_diagnostics`](Survey::write_diagnostics) so that a *second* program over
    /// the same survey reuses these sentences instead of writing a parallel catalogue: `crates/pgrep`
    /// filters this listing and has to describe a refusal in the same words, or the two programs
    /// drift into disagreeing about what one refusal means. The clause carries no program name and no
    /// newline for exactly that reason.
    ///
    /// The order is the order [`write_diagnostics`](Survey::write_diagnostics) has always used, and
    /// the empty domain is last because it is the only one of the four that is **not** a failure.
    pub fn complaint(&self) -> Option<&'static str> {
        if self.stalled {
            return Some("the survey did not advance; the listing is incomplete");
        }
        if let Some(code) = self.refused {
            return Some(refusal(code));
        }
        if self.truncated {
            return Some("the listing buffer filled; this domain has more in it");
        }
        if self.rows.is_empty() {
            // **Not an error, and it must not read like one.** The caller held the domain and was
            // allowed to look; there was nothing in it. This is the sentence that distinguishes an
            // empty answer from a refused one, which is the distinction Linux's `ps` cannot draw.
            return Some("this domain holds no processes");
        }
        None
    }

    /// **Everything to complain about, said before a byte of output** (DECISIONS §67).
    ///
    /// Writes nothing at all when the walk succeeded and found processes, which is the common case
    /// and the one where a second stream should stay silent.
    pub fn write_diagnostics(&self, out: &mut dyn FnMut(&[u8])) {
        if let Some(clause) = self.complaint() {
            out(b"ps: ");
            out(clause.as_bytes());
            out(b"\n");
        }
    }

    /// The table itself, on the output stream. **Nothing at all on a refusal**, so a `ps > out.txt`
    /// that was refused leaves an empty file rather than a plausible-looking listing of nothing.
    pub fn write_report(&self, out: &mut dyn FnMut(&[u8])) {
        if !self.is_complete() || self.rows.is_empty() {
            return;
        }
        // The TIME column appears only when somebody asked for it (`Survey::join_cpu_time`), so a
        // `pgrep`-shaped caller that did not is not handed a column of dashes to explain.
        let timed = self.rows().iter().any(|r| r.cpu_millis.is_some());
        if timed {
            out(b"         TID  STATE     TIME(ms)\n");
        } else {
            out(b"         TID  STATE\n");
        }
        for row in self.rows() {
            write_thread_id(row.tid, out);
            out(b"  ");
            let name = state_name(row.state);
            out(name.as_bytes());
            if timed {
                // Pad the state to the width of the widest one so the figures line up; a column of
                // numbers that does not is a column a reader has to add up by hand.
                for _ in name.len()..STATE_WIDTH {
                    out(b" ");
                }
                out(b"  ");
                write_millis(row.cpu_millis, out);
            }
            out(b"\n");
        }
    }
}

/// What an `abi::survey` state code is called in the listing.
///
/// Lowercase, because these are states rather than headings and the tree's other programs do not
/// shout. An unknown code prints as `?` rather than panicking: this program is reading a number
/// from a kernel that may be newer than it is, and a listing with one honest `?` in it beats a
/// crash.
pub const fn state_name(state: u64) -> &'static str {
    match state {
        abi::survey::READY => "ready",
        abi::survey::RUNNING => "running",
        abi::survey::BLOCKED => "blocked",
        abi::survey::DEAD => "dead",
        _ => "?",
    }
}

/// What a negated `abi::Error` means to somebody who typed `ps`.
///
/// **Written as facts about authority, not as errno.** A refusal here must read like the capability
/// model, which is the same standard `grant_plan::Refusal::message` holds itself to: the reader
/// should learn what they do not hold, not which branch of a `match` they landed in.
pub fn refusal(code: i64) -> &'static str {
    match abi::Error::from_ret(code) {
        Some(abi::Error::NoSuchSlot) => "this process holds no process-domain capability",
        Some(abi::Error::NotPermitted) => {
            "this endpoint may be sent to, but not looked at: no READ on the domain"
        }
        Some(abi::Error::WrongObject) => {
            "the capability granted here is not a supervision endpoint"
        }
        Some(abi::Error::Gone) => "the domain this named has been destroyed",
        _ => "the domain could not be read",
    }
}

/// The width the `STATE` column is padded to: the longest name [`state_name`] can return
/// (`running`, seven characters), so the column after it starts in the same place on every line.
const STATE_WIDTH: usize = 7;

/// A CPU-time figure right-aligned in eight columns, or `-` when the row has none.
///
/// Eight columns holds 27 hours of CPU time before the column widens, and a figure that outgrows it
/// pushes the column rather than losing a digit, for the same reason [`write_thread_id`] does.
///
/// **`-` rather than `0` for an unknown figure**, which is the whole reason [`Row::cpu_millis`] is
/// an `Option`: a thread that was never on a CPU at a tick really does read `0`, and a reader who
/// cannot tell that from "this row was not asked about" has been handed the plausible wrong number
/// this tree keeps refusing to print.
fn write_millis(millis: Option<u64>, out: &mut dyn FnMut(&[u8])) {
    let Some(millis) = millis else {
        out(b"       -");
        return;
    };
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    let mut v = millis;
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    for _ in (buf.len() - i)..8 {
        out(b" ");
    }
    out(&buf[i..]);
}

/// A tid right-aligned in twelve columns, which is wide enough for a generational name whose
/// generation has moved (`(1 << 32) | 6` is ten digits). A wider one is not truncated, it just
/// pushes the column; losing digits from a name that `REAP` has to accept would be worse than an
/// uneven table.
fn write_thread_id(tid: u64, out: &mut dyn FnMut(&[u8])) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    let mut v = tid;
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let digits = buf.len() - i;
    for _ in digits..12 {
        out(b" ");
    }
    out(&buf[i..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader over a canned domain: `entries` in slot order, then done.
    fn domain(entries: &'static [(u64, u64)]) -> impl FnMut(u64) -> (i64, u64, u64) {
        move |cursor: u64| match entries.get(cursor as usize) {
            Some(&(tid, state)) => (cursor as i64 + 1, tid, state),
            None => (abi::survey::DONE as i64, 0, 0),
        }
    }

    fn shown(f: impl FnOnce(&mut dyn FnMut(&[u8]))) -> String {
        let mut v = Vec::new();
        f(&mut |b| v.extend_from_slice(b));
        String::from_utf8(v).unwrap()
    }

    #[test]
    fn a_domain_walks_to_its_end() {
        let mut rows = [Row::default(); MAX_ROWS];
        let s = collect(
            &mut rows,
            &mut domain(&[
                (3, abi::survey::RUNNING),
                (5, abi::survey::BLOCKED),
                (9, abi::survey::DEAD),
            ]),
        );
        assert!(!s.is_refused());
        assert_eq!(s.rows().len(), 3);
        assert_eq!(
            s.rows()[2],
            Row {
                tid: 9,
                state: abi::survey::DEAD,
                cpu_millis: None,
            }
        );
    }

    /// **The claim the whole program exists to make.** An empty domain and a refused one must not
    /// produce the same thing on any stream, because a monitor that reports nothing when it could
    /// not look reads exactly like a quiet machine.
    #[test]
    fn an_empty_domain_and_a_refusal_are_different_answers() {
        let mut rows_a = [Row::default(); MAX_ROWS];
        let empty = collect(&mut rows_a, &mut domain(&[]));
        let mut rows_b = [Row::default(); MAX_ROWS];
        let refused = collect(&mut rows_b, &mut |_| {
            (abi::Error::NotPermitted as i64, 0, 0)
        });

        assert!(!empty.is_refused(), "an empty domain is not a refusal");
        assert!(refused.is_refused());

        let empty_diag = shown(|o| empty.write_diagnostics(o));
        let refused_diag = shown(|o| refused.write_diagnostics(o));
        assert_ne!(empty_diag, refused_diag);
        assert!(empty_diag.contains("no processes"), "{empty_diag}");
        assert!(refused_diag.contains("not looked at"), "{refused_diag}");

        // And neither of them puts a single byte on the output stream, so a redirect of either
        // produces an empty file rather than a listing that says nothing happened.
        assert_eq!(shown(|o| empty.write_report(o)), "");
        assert_eq!(shown(|o| refused.write_report(o)), "");
    }

    /// A refusal arriving **mid-walk** is still a refusal, not a short table. Stopping and saying
    /// so is the point; a listing missing a row it could not read is the failure mode.
    #[test]
    fn a_refusal_halfway_through_discards_the_partial_table() {
        let mut calls = 0;
        let mut rows = [Row::default(); MAX_ROWS];
        let s = collect(&mut rows, &mut |cursor| {
            calls += 1;
            match cursor {
                0 => (1, 3, abi::survey::RUNNING),
                _ => (abi::Error::Gone as i64, 0, 0),
            }
        });
        assert!(s.is_refused());
        assert_eq!(
            s.rows().len(),
            1,
            "the rows it did read are kept for the record"
        );
        assert_eq!(
            shown(|o| s.write_report(o)),
            "",
            "but none of them are printed"
        );
        assert!(shown(|o| s.write_diagnostics(o)).contains("destroyed"));
    }

    /// **`collect` terminates for every reader**, including one that never advances its cursor. The
    /// shipped kernel cannot do this; the property is what makes this function safe to hand an
    /// arbitrary closure, which is what every test above does.
    #[test]
    fn a_cursor_that_does_not_advance_ends_the_walk() {
        let mut rows = [Row::default(); MAX_ROWS];
        let s = collect(&mut rows, &mut |_| (1, 4, abi::survey::READY));
        assert!(s.is_refused());
        assert!(shown(|o| s.write_diagnostics(o)).contains("did not advance"));
    }

    /// **A short buffer is not a short listing, it is a stated one.** The whole design refuses to
    /// let a monitor report less than it saw without saying so, and running out of room is one more
    /// way that could happen. `ps` cannot reach this, because it sizes its buffer at [`MAX_ROWS`];
    /// a future caller with a smaller one gets told rather than getting a plausible table.
    #[test]
    fn a_buffer_too_small_for_the_domain_says_so_and_prints_nothing() {
        let mut rows = [Row::default(); 2];
        let s = collect(&mut rows, &mut |cursor| {
            (cursor as i64 + 1, cursor + 100, abi::survey::READY)
        });
        assert_eq!(s.rows().len(), 2);
        assert!(
            !s.is_complete(),
            "a truncated listing claimed to be the whole domain"
        );
        assert!(!s.is_refused(), "running out of room is not a refusal");
        assert!(shown(|o| s.write_diagnostics(o)).contains("more in it"));
        assert_eq!(
            shown(|o| s.write_report(o)),
            "",
            "a partial table must not be printed"
        );
    }

    /// A domain as wide as the kernel's whole thread table fills the buffer exactly and needs no
    /// truncation case.
    #[test]
    fn the_widest_possible_domain_fits() {
        let mut rows = [Row::default(); MAX_ROWS];
        let s = collect(&mut rows, &mut |cursor| {
            if (cursor as usize) < MAX_ROWS {
                (cursor as i64 + 1, cursor + 100, abi::survey::READY)
            } else {
                (abi::survey::DONE as i64, 0, 0)
            }
        });
        assert!(!s.is_refused());
        assert_eq!(s.rows().len(), MAX_ROWS);
    }

    #[test]
    fn the_table_has_a_header_and_one_line_per_thread() {
        let mut rows = [Row::default(); MAX_ROWS];
        let s = collect(
            &mut rows,
            &mut domain(&[(3, abi::survey::RUNNING), (5, abi::survey::DEAD)]),
        );
        let out = shown(|o| s.write_report(o));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "header plus two rows: {out}");
        assert!(lines[0].contains("TID") && lines[0].contains("STATE"));
        assert!(lines[1].ends_with("3  running"), "{}", lines[1]);
        assert!(lines[2].ends_with("5  dead"), "{}", lines[2]);
        // Nothing to say when the answer is the table itself.
        assert_eq!(shown(|o| s.write_diagnostics(o)), "");
    }

    /// A generational tid is a ten-digit number and must survive the column intact: it is the name
    /// `REAP` accepts, so a truncated one would be a name that no longer works.
    #[test]
    fn a_wide_thread_id_keeps_every_digit() {
        let mut rows = [Row::default(); MAX_ROWS];
        let s = collect(
            &mut rows,
            &mut domain(&[((1u64 << 32) | 6, abi::survey::DEAD)]),
        );
        let out = shown(|o| s.write_report(o));
        assert!(out.contains("4294967302"), "{out}");
    }

    /// Every state code a kernel can report has its own name, and an unknown one prints as `?`.
    /// Only `running` and `dead` were pinned by the table tests, so deleting the `ready` or
    /// `blocked` arm left every test green.
    #[test]
    fn every_state_has_its_name() {
        assert_eq!(state_name(abi::survey::READY), "ready");
        assert_eq!(state_name(abi::survey::RUNNING), "running");
        assert_eq!(state_name(abi::survey::BLOCKED), "blocked");
        assert_eq!(state_name(abi::survey::DEAD), "dead");
        assert_eq!(state_name(u64::MAX), "?");
    }

    /// Each refusal a person can meet is explained in its own words. The two this pins were the
    /// ones whose arms could be deleted into the generic "could not be read" line unnoticed.
    #[test]
    fn each_refusal_is_explained_in_its_own_words() {
        let say = |e: abi::Error| refusal(e as i64);
        assert!(say(abi::Error::NoSuchSlot).contains("holds no process-domain capability"));
        assert!(say(abi::Error::WrongObject).contains("not a supervision endpoint"));
        assert!(say(abi::Error::NotPermitted).contains("no READ"));
        assert!(say(abi::Error::Gone).contains("destroyed"));
        assert!(refusal(0).contains("could not be read"));
    }

    /// The two right-aligned columns pad by the width of the digits written: a tid in twelve
    /// columns and a CPU figure in eight. A one-digit value cannot tell the digit count from its
    /// quotient or sum with the buffer offset, so these use several widths.
    #[test]
    fn the_columns_pad_to_their_widths() {
        let tid = |t| shown(|o| write_thread_id(t, o));
        assert_eq!(tid(7), "           7");
        assert_eq!(tid(1234), "        1234");
        assert_eq!(tid(123_456_789_012), "123456789012");
        assert_eq!(tid(1_234_567_890_123), "1234567890123");
        let ms = |m| shown(|o| write_millis(Some(m), o));
        assert_eq!(ms(0), "       0");
        assert_eq!(ms(12_345), "   12345");
        assert_eq!(ms(12_345_678), "12345678");
        assert_eq!(ms(123_456_789), "123456789");
        assert_eq!(shown(|o| write_millis(None, o)), "       -");
    }

    /// A CPU-time reader over a canned domain: `entries` in slot order, then done.
    fn times(entries: &'static [(u64, u64)]) -> impl FnMut(u64) -> (i64, u64, u64) {
        move |cursor: u64| match entries.get(cursor as usize) {
            Some(&(tid, millis)) => (cursor as i64 + 1, tid, millis),
            None => (abi::survey::DONE as i64, 0, 0),
        }
    }

    /// **The second walk fills the column, joined on the tid rather than on the position.**
    ///
    /// The two walks are given the domain in *different orders* on purpose. Joining on position
    /// would pass on a canned reader that returned them in the same order and would silently
    /// mis-attribute every figure the first time a slot was reused between two calls.
    #[test]
    fn a_second_walk_fills_the_time_column_by_tid() {
        let mut rows = [Row::default(); MAX_ROWS];
        let mut s = collect(
            &mut rows,
            &mut domain(&[(3, abi::survey::RUNNING), (5, abi::survey::BLOCKED)]),
        );
        s.join_cpu_time(&mut times(&[(5, 20), (3, 990)]));

        assert_eq!(s.rows()[0].tid, 3);
        assert_eq!(s.rows()[0].cpu_millis, Some(990));
        assert_eq!(s.rows()[1].cpu_millis, Some(20));
    }

    /// **A thread that died between the two walks has no figure, and that is printed as `-`.**
    ///
    /// The alternative is a zero, which reads as "this thread has used no CPU" and is a claim
    /// nobody made. It is the same distinction the empty domain and the refusal are kept apart by,
    /// one column over.
    #[test]
    fn a_thread_missing_from_the_second_walk_keeps_no_figure() {
        let mut rows = [Row::default(); MAX_ROWS];
        let mut s = collect(
            &mut rows,
            &mut domain(&[(3, abi::survey::RUNNING), (5, abi::survey::RUNNING)]),
        );
        s.join_cpu_time(&mut times(&[(3, 40)]));

        assert_eq!(s.rows()[1].cpu_millis, None);
        let out = shown(|o| s.write_report(o));
        assert!(out.lines().nth(2).unwrap().ends_with('-'), "{out}");
    }

    /// A refusal in the **second** walk is a refusal, exactly as one in the first is. A table with
    /// a blank column and no complaint would be the failure this crate exists to avoid, wearing a
    /// new hat.
    #[test]
    fn a_refusal_in_the_second_walk_is_still_a_refusal() {
        let mut rows = [Row::default(); MAX_ROWS];
        let mut s = collect(&mut rows, &mut domain(&[(3, abi::survey::RUNNING)]));
        assert!(!s.is_refused());
        s.join_cpu_time(&mut |_| (abi::Error::Gone as i64, 0, 0));
        assert!(s.is_refused());
        assert_eq!(shown(|o| s.write_report(o)), "");
        assert!(shown(|o| s.write_diagnostics(o)).contains("destroyed"));
    }

    /// A second walk whose cursor does not advance terminates, the same guard `collect` carries.
    #[test]
    fn a_second_walk_that_does_not_advance_ends() {
        let mut rows = [Row::default(); MAX_ROWS];
        let mut s = collect(&mut rows, &mut domain(&[(3, abi::survey::RUNNING)]));
        s.join_cpu_time(&mut |_| (1, 3, 10));
        assert!(s.is_refused());
    }

    /// **The ranking is what makes a listing a `top`**: most CPU first, ties on the tid, and a row
    /// with no figure last rather than first.
    #[test]
    fn ranking_puts_the_busiest_first_and_the_unknown_last() {
        let mut rows = [Row::default(); MAX_ROWS];
        let mut s = collect(
            &mut rows,
            &mut domain(&[
                (3, abi::survey::RUNNING),
                (5, abi::survey::RUNNING),
                (7, abi::survey::RUNNING),
                (9, abi::survey::RUNNING),
            ]),
        );
        // 9 gets no figure at all; 3 and 7 tie at 100.
        s.join_cpu_time(&mut times(&[(3, 100), (5, 900), (7, 100)]));
        s.rank_by_cpu_time();

        let order: Vec<u64> = s.rows().iter().map(|r| r.tid).collect();
        assert_eq!(order, vec![5, 3, 7, 9], "ranked: {order:?}");
    }

    /// The `TIME` column appears only when a caller asked for it, so a listing that ran one walk is
    /// not handed a column of dashes to explain.
    #[test]
    fn the_time_column_appears_only_when_it_was_asked_for() {
        let mut rows = [Row::default(); MAX_ROWS];
        let mut s = collect(&mut rows, &mut domain(&[(3, abi::survey::RUNNING)]));
        assert!(!shown(|o| s.write_report(o)).contains("TIME"));
        s.join_cpu_time(&mut times(&[(3, 250)]));
        let out = shown(|o| s.write_report(o));
        assert!(out.contains("TIME(ms)"), "{out}");
        assert!(out.lines().nth(1).unwrap().ends_with("250"), "{out}");
    }

    /// A state code from a kernel newer than this program prints as `?` rather than panicking or
    /// being silently dropped.
    #[test]
    fn an_unknown_state_is_shown_as_unknown() {
        assert_eq!(state_name(99), "?");
        let mut rows = [Row::default(); MAX_ROWS];
        let s = collect(&mut rows, &mut domain(&[(3, 99)]));
        assert!(shown(|o| s.write_report(o)).contains('?'));
    }

    /// Every refusal reads as a fact about authority. The catalogue is checked so the wording
    /// cannot drift into errno.
    #[test]
    fn every_refusal_names_what_is_not_held() {
        for code in [
            abi::Error::NoSuchSlot as i64,
            abi::Error::NotPermitted as i64,
            abi::Error::WrongObject as i64,
            abi::Error::Gone as i64,
            -99,
        ] {
            let m = refusal(code);
            assert!(!m.is_empty());
            assert!(
                m.chars().next().is_some_and(|c| c.is_lowercase()),
                "a refusal is a clause the program name prefixes: {m}",
            );
        }
    }
}
