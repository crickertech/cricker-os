//! **The session process: what keeps a user's schedule running after they disconnect**
//! (milestone 152 (durable delegation), S1 and L2 of 2026-09-26, §222 (who holds a user's
//! schedule)).
//!
//! `login` builds one of these when an authenticated identity asks for its schedule
//! (`login_protocol::SCHEDULE`). It is built out of a region split from that user's own session
//! budget, and so is the timetable it builds. Both are therefore live descendants of the budget,
//! which is the whole of what keeps the session alive once the client leaves: DECISIONS §16 (object
//! revocation) refuses `MemoryRegion::DESTROY` on a parent with a live child. Nothing new holds the
//! session up. `kernel::user::login_tests::a_login_session_with_pending_work_refuses_logout_until_the_work_is_gone`
//! proves that rule on a login budget.
//!
//! # What it does
//!
//! 1. Builds `timetable` from the image `login` copied in, out of [`BUDGET`]: its own region, a
//!    budget for the jobs it fires, and two endpoints. It is handed the jobs archive `login` copied
//!    in, which holds exactly the programs a scheduled job may run. The registration page `login` made
//!    ([`PAGE`]) is mapped into it at [`TIMETABLE_PAGE_VA`], so the timetable starts empty and
//!    silent and waits for a `REPLACE` (`timetable::contract`).
//! 2. Says it is ready on [`READY`], once. `login` is blocked waiting for exactly that word.
//! 3. Blocks on `e`, its one endpoint, for the rest of its life. The timetable's death arrives
//!    there, because `e` is its supervision endpoint, and so does every scheduled job's report,
//!    because `e` is also the endpoint the timetable hands each job as its report slot. A process
//!    has one wait point, and this is how one reader serves both.
//! 4. When the timetable is gone, gives [`BUDGET`]'s contents back and exits. The page stays, in
//!    `login`'s region, so `login` can read why the timetable stopped before it reclaims this
//!    process.
//!
//! **Telling a death from a report.** A death is five words from the kernel with an event of
//! `abi::fault::EVENT_EXIT` or `EVENT_FAULT`. A job can send those same numbers, so the event alone
//! decides nothing: the reap decides. Only the timetable is supervised by `e`, so `REAP` on the
//! tid a job claims answers `NotSupervised` or `StillAlive`, and the message is treated as a report.
//!
//! # Capability contract (`login_protocol::session`)
//!
//! - slot [`READY`]: `WRITE`. One readiness word or a failure word, and, at the end,
//!   `login_protocol::session::STOPPED`.
//! - slot [`BUDGET`]: `WRITE | GRANT`. The timetable, its jobs, and both endpoints are built from
//!   it; `GRANT` because the timetable is handed a split of it.
//! - slot [`PAGE`]: `WRITE`. The registration page, retyped by `login` from this process's own
//!   construction region.
//! - `a0`: the length of `timetable`'s image, copied in at `login_protocol::session::TIMETABLE_VA`.
//! - `a1`: the length of the jobs archive, copied in at `login_protocol::session::JOBS_VA`.
//!
//! Name: provisional. calef ruled S1 on 2026-09-26 and gave no name; the maintainer suggested
//! `session`, and this lane shipped it on 2026-09-26. It names what the process is for a user
//! rather than what it does, which is the question the naming conventions ask first.
//!
//! # BUGS
//!
//! - **A scheduled job's report is received and dropped.** Question 6 in
//!   `notes/durable-delegation.md` asks where it should go once nobody is attached; until that is
//!   answered, this process is the only reader there is, and it reads so that no job blocks for
//!   ever on its report.
//! - **Nothing restarts a timetable that faults.** The session exits, and the schedule stays on
//!   disk for the next `SCHEDULE` or the boot-time re-deriver.
//! - **The jobs archive is copied twice**, once into this process and once into the timetable, because
//!   `supervision_protocol::build_child` can hand a child data only by copying a blob or mapping a
//!   frame this process holds a capability to, and `login` hands it bytes rather than frames.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates) tracks
// (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)): each `[[bin]]` is its own
// crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use login_protocol::session as contract;
use supervision_protocol::{
    ChildEndowment, Retention, build_child, memory_region_destroy, memory_region_split,
    retype_obj_from as retype_obj, start_child,
};
use timetable::contract as tt;
use user_mode_runtime::{cap_delete, exit, reap, recv_fault, send, yield_now};

/// The readiness endpoint, `WRITE`.
const READY: u64 = contract::READY_SLOT;
/// Everything this process builds, `WRITE`.
const BUDGET: u64 = contract::BUDGET_SLOT;
/// The registration page, `WRITE`.
const PAGE: u64 = contract::PAGE_SLOT;

/// Where the timetable finds its registration page. Any address clear of its program, its stack
/// and the archive at `user_mode_runtime::initrd::INITRD_VA` would do; this is the one
/// `kernel/src/user/timetable_tests.rs` already uses for the same job.
const TIMETABLE_PAGE_VA: u64 = 0x0600_0000;

/// The timetable's own construction: its segments, the archive copied into it, its
/// `timetable::contract::STACK_PAGES` stack and its tables. Measured against the aarch64 debug
/// build on 2026-09-26 with room to spare; see this program's own test for what fails if it is short.
const TIMETABLE_REGION_PAGES: u64 = 224;
/// The budget the timetable fires jobs from: two 48-page instances and the loader's scratch.
const JOB_BUDGET_PAGES: u64 = 128;

/// How many times to retry a reap or a destroy that finds something still standing on its region.
/// The same net `components/src/timetable.rs` keeps, for the same reason.
const ATTEMPTS: usize = 1024;

#[unsafe(no_mangle)]
pub extern "C" fn _start(image_len: u64, jobs_len: u64, _a2: u64) -> ! {
    // SAFETY: `login` copies `image_len` bytes to `TIMETABLE_VA` and `jobs_len` bytes to `JOBS_VA`
    // before this process runs (`login_protocol::session`'s contract), and nothing writes them
    // afterwards.
    let (image, jobs_archive) = unsafe {
        (
            core::slice::from_raw_parts(contract::TIMETABLE_VA as *const u8, image_len as usize),
            core::slice::from_raw_parts(contract::JOBS_VA as *const u8, jobs_len as usize),
        )
    };
    let Ok(elf) = elf::Elf::parse(image) else {
        fail(3)
    };

    // The one endpoint this process ever waits on, and the timetable's supervision endpoint.
    let Ok(e) = retype_obj(BUDGET, abi::objtype::RENDEZVOUS) else {
        fail(4)
    };
    // The endpoint the timetable reaps its own jobs through. Not read here.
    let Ok(deaths) = retype_obj(BUDGET, abi::objtype::RENDEZVOUS) else {
        fail(5)
    };
    let Ok(jobs) = memory_region_split(BUDGET, JOB_BUDGET_PAGES) else {
        fail(6)
    };
    let Ok(region) = memory_region_split(BUDGET, TIMETABLE_REGION_PAGES) else {
        fail(7)
    };

    let built = build_child(
        BUDGET,
        region,
        &elf,
        &ChildEndowment {
            placed: &[
                (tt::BUDGET_SLOT, jobs, abi::rights::WRITE),
                (
                    tt::CHILD_REPORT_SLOT,
                    e,
                    abi::rights::WRITE | abi::rights::GRANT,
                ),
                (
                    tt::DEATHS_SLOT,
                    deaths,
                    abi::rights::READ | abi::rights::GRANT,
                ),
            ],
            maps: &[(TIMETABLE_PAGE_VA, PAGE, abi::address_space::MAP_RW)],
            blobs: &[(user_mode_runtime::initrd::INITRD_VA, jobs_archive)],
            fault: Some(e),
            stack_pages: tt::STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    );
    let Ok(child) = built else { fail(8) };
    // Forever (`a0 == 0`): the timetable stops when its document is emptied, not after a count.
    if !start_child(child, 0, jobs_len, TIMETABLE_PAGE_VA) {
        fail(9)
    }
    // The timetable holds its own copies now. `jobs` is kept: it is a child of `BUDGET`, and
    // `BUDGET` cannot come down until it does.
    cap_delete(deaths);
    cap_delete(PAGE);

    send(READY, contract::READY, 0, 0);

    let mut reports = 0u64;
    loop {
        let (event, tid, ..) = recv_fault(e);
        let death = event == abi::fault::EVENT_EXIT || event == abi::fault::EVENT_FAULT;
        if death && reaped(e, tid) {
            break;
        }
        // A job's report, or a job pretending to be a death. Either way nothing more is owed.
        reports = reports.wrapping_add(1);
    }
    let _ = reports;

    // The reap took the timetable's own region with it. It drained its jobs before it stopped, so
    // `jobs` is empty; then `BUDGET` has no children and comes down too, taking both endpoints.
    // What is left of this session is the region `login` built this process from, which `login`
    // reclaims once it reads the timetable's exit word. `region`'s name is stale by now.
    let _ = region;
    destroy(jobs);
    destroy(BUDGET);
    // Say so, and wait to be taken: `login` reclaims this process only after it has this word, so
    // its region is never destroyed under the two destroys above.
    send(READY, contract::STOPPED, 0, 0);
    exit()
}

/// `REAP` the thread a death message names, retrying while its region still holds something that
/// can run. `false` if this endpoint does not supervise it, which is how a forged death reads.
fn reaped(e: u64, tid: u64) -> bool {
    for _ in 0..ATTEMPTS {
        let r = reap(e, tid);
        if r == 0 {
            return true;
        }
        if r == abi::Error::NotSupervised as i64 || r == abi::Error::StillAlive as i64 {
            return false;
        }
        yield_now();
    }
    false
}

/// `MemoryRegion::DESTROY`, retried while something in the region can still run.
fn destroy(r: u64) {
    for _ in 0..ATTEMPTS {
        if memory_region_destroy(r) {
            return;
        }
        yield_now();
    }
}

/// Report a failure to `login`, which is waiting for exactly one word, and stop.
fn fail(step: u64) -> ! {
    send(READY, contract::FAILED | step, 0, 0);
    exit()
}

user_mode_runtime::panic_handler!();
