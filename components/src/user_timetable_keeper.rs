//! **`user_timetable_keeper`: what keeps a user's schedule running after they disconnect**
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
//!    budget for the jobs it fires, and two endpoints. It is handed no programs: a scheduled job
//!    runs what the live activation generation names (Fork 8 ruled D by calef on 2026-09-27, on
//!    #1377), so the timetable is handed the two read-only store caretakers `login` built
//!    ([`ACTIVATION`], [`PACKAGES`]) and their channel's page ([`STORE_PAGE`]), which makes it a
//!    store-mode timetable (`timetable::contract`). The registration page `login` made ([`PAGE`])
//!    is mapped into it at [`TIMETABLE_PAGE_VA`], so the timetable starts empty and silent and
//!    waits for a `REPLACE`.
//! 2. Says it is ready on [`READY`], once. `login` is blocked waiting for exactly that word.
//! 3. Blocks on `e`, its one endpoint, for the rest of its life. The timetable's death arrives
//!    there, because `e` is its supervision endpoint. No job report does: calef ruled Fork 6 C on
//!    2026-09-27 (UTC, #1377), so a scheduled job holds no report endpoint and writes its output
//!    through whatever its entry grants.
//! 4. When the timetable is gone, gives [`BUDGET`]'s contents back and exits. The page stays, in
//!    `login`'s region, so `login` can read why the timetable stopped before it reclaims this
//!    process.
//!
//! **Only the timetable's death arrives.** The reap still decides rather than the event word, which
//! is cheap and keeps this process right if something else ever holds `e`.
//!
//! # Capability contract (`login_protocol::user_timetable_keeper`)
//!
//! - slot [`READY`]: `WRITE`. One readiness word or a failure word, and, at the end,
//!   `login_protocol::user_timetable_keeper::STOPPED`.
//! - slot [`BUDGET`]: `WRITE | GRANT`. The timetable, its jobs, and both endpoints are built from
//!   it; `GRANT` because the timetable is handed a split of it.
//! - slot [`PAGE`]: `WRITE`. The registration page, retyped by `login` from this process's own
//!   construction region.
//! - slots [`ACTIVATION`] and [`PACKAGES`]: `WRITE`. Endpoints to the caretakers serving the
//!   store's `activation/` and `packages/` read-only, handed on to the timetable.
//! - slot [`STORE_PAGE`]: `WRITE`. The durable window's page the caretakers stage through, mapped
//!   into the timetable and never here.
//! - `a0`: the length of `timetable`'s image, copied in at `login_protocol::user_timetable_keeper::TIMETABLE_VA`.
//!
//! Name: ratified 2026-10-06 (calef, "`user_timetable_keeper` ratified.", in conversation),
//! replacing the provisional `session` (the maintainer's suggestion, shipped 2026-09-26). It builds
//! a user's timetable and holds it alive after logout. `session` collided with every other OS's
//! login session and with the login sessions PR #1769 proposes. Refused `supervisor`, because it
//! does not restart a faulted timetable (BUGS), and `caretaker`, because it serves no access. A
//! keeper holds something alive and does not restart it; if this ever restarts the timetable, it
//! is renamed `user_timetable_supervisor`.
//!
//! # BUGS
//!
//! - **Nothing restarts a timetable that faults.** This process exits, and the schedule stays on
//!   disk for the next `SCHEDULE` or the boot-time re-deriver.
//! - **The timetable's image is copied twice**, once into this process and once into the
//!   timetable, because `supervision_protocol::build_child` can hand a child data only by copying a
//!   blob or mapping a frame this process holds a capability to, and `login` hands it bytes rather
//!   than frames.
//! - **A durable timetable holds no clock**, so every calendar line in a durable schedule is
//!   `timetable::Unbacked::WallClock` and never fires, and no durable job gets the clock page.
//!   Milestone 129 (scheduled execution) gave the timetable an optional clock at
//!   `timetable::contract::CLOCK_SLOT` while this program was being built; `login` holds no clock
//!   to pass down. Found at the merge of 2026-10-03 (UTC).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates) tracks
// (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)): each `[[bin]]` is its own
// crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use login_protocol::user_timetable_keeper as contract;
use supervision_protocol::{
    ChildEndowment, Retention, build_child, memory_region_destroy, memory_region_split,
    retype_obj_from as retype_obj, start_child,
};
use timetable::contract as tt;
use user_mode_runtime::{cap_delete, exit, reap, receive_fault, send, yield_now};

/// The readiness endpoint, `WRITE`.
const READY: u64 = contract::READY_SLOT;
/// Everything this process builds, `WRITE`.
const BUDGET: u64 = contract::BUDGET_SLOT;
/// The registration page, `WRITE`.
const PAGE: u64 = contract::PAGE_SLOT;
/// The store's `activation/`, read-only, `WRITE`.
const ACTIVATION: u64 = contract::ACTIVATION_SLOT;
/// The store's `packages/`, read-only, `WRITE`.
const PACKAGES: u64 = contract::PACKAGES_SLOT;
/// The store channel's page, `WRITE`.
const STORE_PAGE: u64 = contract::STORE_PAGE_SLOT;

/// Where the timetable finds its registration page. Any address clear of its program, its stack
/// and the archive at `user_mode_runtime::initrd::INITRD_VA` would do; this is the one
/// `kernel/src/user/timetable_tests.rs` already uses for the same job.
const TIMETABLE_PAGE_VA: u64 = 0x0600_0000;

/// The timetable's own construction: its segments (which since Fork 8 D include a
/// `timetable::contract::STAGING_BYTES` buffer, 32 pages), its `timetable::contract::STACK_PAGES`
/// stack and its tables. Measured against the aarch64 debug build on 2026-09-26 at 224 with room to
/// spare, before the buffer; see this program's own test for what fails if it is short.
///
/// **Raised 240 -> 256 on 2026-09-30** for the stack's 32 -> 48 (see `timetable::contract`), whose
/// halves are both here: the region carries the stack it grew and the buffer beside it. The
/// constraint below still holds. (It said "with 128 to spare"; the arithmetic was always 16.)
///
/// **Raised 256 -> 272 on 2026-10-02 (UTC)** for the staging buffer's 64 -> 128 KiB (see
/// `timetable::contract::STAGING_BYTES` for the program that did not fit), 16 pages of `.bss`.
const TIMETABLE_REGION_PAGES: u64 = 272;
/// The budget the timetable fires jobs from: two 48-page instances and the loader's scratch.
const JOB_BUDGET_PAGES: u64 = 128;
// What this process splits must fit what `login` gives it, with the two endpoints' pages beside.
const _: () = assert!(
    TIMETABLE_REGION_PAGES + JOB_BUDGET_PAGES
        < login_protocol::durable::USER_TIMETABLE_KEEPER_BUDGET_PAGES
);

/// How many times to retry a reap or a destroy that finds something still standing on its region.
/// The same net `components/src/timetable.rs` keeps, for the same reason.
const ATTEMPTS: usize = 1024;

#[unsafe(no_mangle)]
pub extern "C" fn _start(image_len: u64, _a1: u64, _a2: u64) -> ! {
    // SAFETY: `login` copies `image_len` bytes to `TIMETABLE_VA` before this process runs
    // (`login_protocol::user_timetable_keeper`'s contract), and nothing writes them afterwards.
    let image = unsafe {
        core::slice::from_raw_parts(contract::TIMETABLE_VA as *const u8, image_len as usize)
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
                    tt::DEATHS_SLOT,
                    deaths,
                    abi::rights::READ | abi::rights::GRANT,
                ),
                (tt::ACTIVATION_SLOT, ACTIVATION, abi::rights::WRITE),
                (tt::PACKAGES_SLOT, PACKAGES, abi::rights::WRITE),
            ],
            maps: &[
                (TIMETABLE_PAGE_VA, PAGE, abi::address_space::MAP_RW),
                (tt::STORE_PAGE_VA, STORE_PAGE, abi::address_space::MAP_RW),
            ],
            fault: Some(e),
            stack_pages: tt::STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    );
    let Ok(child) = built else { fail(8) };
    // Forever (`a0 == 0`): the timetable stops when its document is emptied, not after a count.
    // No archive (`a1 == 0`): it is in store mode.
    if !start_child(child, 0, 0, TIMETABLE_PAGE_VA) {
        fail(9)
    }
    // The timetable holds its own copies now. `jobs` is kept: it is a child of `BUDGET`, and
    // `BUDGET` cannot come down until it does.
    cap_delete(deaths);
    cap_delete(PAGE);
    cap_delete(ACTIVATION);
    cap_delete(PACKAGES);
    cap_delete(STORE_PAGE);

    send(READY, contract::READY, 0, 0);

    // Only the kernel sends here: the timetable's death, since no job holds this endpoint (Fork 6
    // C). The reap still decides, so a message that is not the timetable's death is skipped rather
    // than believed.
    loop {
        let (event, tid, ..) = receive_fault(e);
        let death = event == abi::fault::EVENT_EXIT || event == abi::fault::EVENT_FAULT;
        if death && reaped(e, tid) {
            break;
        }
    }

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
