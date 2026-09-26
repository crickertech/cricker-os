use super::*;
use crate::cap::{Rights, memory_region_root_cap, page_frame_cap, rendezvous_cap};
use crate::sched::{self, RendezvousId};

/// Where the service maps its own request to the credential service. Must match `components/src/login.rs`.
const CRED_VA: u64 = 0x0000_0000_00e3_0000;

/// How many pages a spawned `login_test_client` run's own `memory_region_cap` (slot 3) holds:
/// enough for `page_frame::MAP`'s own page-table cost when it self-maps the frame `login`'s
/// `CONNECT` step delegates (milestone 49's channel-per-client update; `fixtures/src/login_test_client.rs`
/// mirrors this program's own post-auth `map_page_frame(fs_page_frame, FS_VA, true, budget)`, but a
/// run holds no budget yet at the point it must map its own connect channel). Margin over the one
/// page a fresh mapping ever strictly needs, on this file's own existing style for every other
/// region here.
///
/// **Reclaimed once the run is dead** ([`wait_client`], 2026-09-26 UTC). A run's scratch pays for
/// page tables in that run's own address space, so it is handed to
/// `kernel::user::holding::Holding::add_region_after_death` beside the run's thread id, and comes
/// back only after the thread is gone. Until then nothing reclaimed it, and every run held one
/// region and four frames for the rest of the boot. It cost two lanes a test on the same day: the
/// `milestone/198-owner-console` lane's two extra runs made `tests::std_net_runs_over_the_socket_contract`
/// fail to load a program for want of frames, and one more login test in milestone 152's lane took
/// the aarch64 suite to 253 of `memory_region::MAX_REGIONS` (256) live regions at `timetable_tests`,
/// so the timetable's `--mem` split failed and the test hung rather than failing.
const CLIENT_SCRATCH_UT_PAGES: u64 = 4;

/// Stack pages beyond the one page `run` maps. This process parses the initrd, parses an ELF, and
/// builds a child address space (`supervision_protocol::build_child`), which is deeper than
/// `root_supervisor`'s own 8-page stack covers; sized against `credentialer.rs`'s own lesson (its
/// Argon2id inner loop needed 16 pages where one was not close) rather than guessed from nothing.
const LOGIN_STACK_PAGES: u64 = 16;

/// **What a `login_test_client` run is handed** (milestone 293): a behaviour, and separately a
/// credential. Must match `fixtures/src/login_test_client.rs`, whose module docs carry the argument
/// for why these are two things and not eleven roles.
///
/// The credential halves are `credential_protocol::fixture`'s own indices, not named again here: a
/// third copy of `chris` is exactly what 293 removed.
pub const LOGIN: u64 = 0;
/// DECISIONS §117's per-identity subtree proof, writing the identity it was handed; see the same
/// file's module docs.
pub const WRITE_MARKER: u64 = 1;
/// Reads that marker back, in an independent channel.
pub const READ_MARKER: u64 = 2;
/// Logs in, then tears the session down with the fourth delegated capability and proves the
/// directory came down with it. See the same file's module docs.
pub const LOGOUT: u64 = 3;
/// Milestone 49's terminal update: logs in, proves the fifth delegated capability (the terminal)
/// works, tears the session down without freeing the terminal. See the same file's module docs.
pub const HOLD_TERMINAL: u64 = 4;
/// Sends `login_protocol::logout_word` on the front door directly, with no credential at all.
pub const FREE_TERMINAL: u64 = 5;
/// DECISIONS §219 (how the shell names an installed program to the spawner) gate D2: logs in and sends [`RUN_UNVOUCHED_MAGIC`] on the sixth delegated
/// capability. See the same file's module docs.
pub const PRESENT_RUN_UNVOUCHED: u64 = 6;
/// Milestone 152 (durable delegation): logs in, holds a pending-job child off the budget, proves the logout is refused
/// until the child is gone. See the same file's module docs. Provisional name.
pub const PENDING_WORK: u64 = 7;
/// [`PRESENT_RUN_UNVOUCHED`]'s proof-of-life word; must match the same file's
/// `RUN_UNVOUCHED_MAGIC`.
pub const RUN_UNVOUCHED_MAGIC: u64 = 0x_7e12_0000_0000_0002;

/// The report words `login_test_client` sends; must match the same file.
pub const RPT_OK: u64 = login_protocol::OK;
pub const RPT_DENIED: u64 = login_protocol::DENIED;
#[allow(dead_code)] // named for completeness with the pair above; nothing exercises it today
pub const RPT_MALFORMED: u64 = login_protocol::MALFORMED;
/// Milestone 49's terminal update: the terminal was already on loan.
pub const RPT_NO_TERMINAL: u64 = login_protocol::NO_TERMINAL;
/// [`FREE_TERMINAL`]'s own answer.
pub const RPT_LOGGED_OUT: u64 = login_protocol::LOGGED_OUT;

/// [`HOLD_TERMINAL`]'s proof-of-life word for the delegated terminal; must match the same file's
/// `TERM_MAGIC`.
pub const TERM_MAGIC: u64 = 0x_7e12_0000_0000_0001;

/// Bits of a successful report's second word; must match the same file.
pub const F_DIR_WORKS: u64 = 1 << 0;
pub const F_BUDGET_WORKS: u64 = 1 << 1;
pub const F_NOT_SHARED_SUBTREE: u64 = 1 << 2;
pub const F_MARKER_WRITTEN: u64 = 1 << 3;
pub const F_TEARDOWN_OK: u64 = 1 << 4;
pub const F_DEAD_AFTER_TEARDOWN: u64 = 1 << 5;
pub const F_BUDGET_TEARDOWN_OK: u64 = 1 << 6;
pub const F_BUDGET_DEAD_AFTER_TEARDOWN: u64 = 1 << 7;
/// Milestone 49's terminal update: the fifth delegated capability delivered [`TERM_MAGIC`] to a
/// real receiver. Set only by [`HOLD_TERMINAL`].
pub const F_TERM_WORKS: u64 = 1 << 8;
/// DECISIONS §219 limitation 2: the sixth capability arrived and `SEND_CAP` of it was refused.
pub const F_RUN_UNVOUCHED_NOT_GRANTABLE: u64 = 1 << 9;
/// The sixth capability delivered [`RUN_UNVOUCHED_MAGIC`]. Set only by [`PRESENT_RUN_UNVOUCHED`].
pub const F_RUN_UNVOUCHED_WORKS: u64 = 1 << 10;
/// `OK` announced a sixth capability. Set by every behaviour that was sent one.
pub const F_RUN_UNVOUCHED_ANNOUNCED: u64 = 1 << 11;
/// Milestone 152: the budget's `DESTROY` was refused `NotPermitted` while a pending-job child lived.
pub const F_LOGOUT_REFUSED_WHILE_PENDING: u64 = 1 << 12;
/// Milestone 152: the budget still retyped a page after that refusal.
pub const F_SESSION_SURVIVED_REFUSAL: u64 = 1 << 13;
/// Milestone 152: the pending-job child was destroyed.
pub const F_PENDING_WORK_DESTROYED: u64 = 1 << 14;

/// **[`LOGOUT`]'s third report word is microseconds, not an identity hint**: how long that
/// behaviour's `MemoryRegion::DESTROY` on the caretaker region waited for §16's armed kill to land.
/// Every other behaviour that fills the third word puts a [`login_protocol::identity_hint`] there; this one
/// has no identity to report and a number a red run needs. Must match
/// `fixtures/src/login_test_client.rs`'s `waited_micros`.
///
/// This is that client's own ceiling in the same units, so a failure message can say how close to
/// it the wait came, which is the difference between "the host was slow" and "the caretaker never
/// died". Must match its `DESTROY_WAIT_SECS`.
pub const DESTROY_WAIT_MICROS: u64 = 5 * 1_000_000;

/// A running login service and the endpoints that reach it.
pub struct Wiring {
    /// A client's login request, `WRITE`.
    pub request: RendezvousId,
    /// The verdict and, on success, five delegated capabilities, `READ`.
    pub result: RendezvousId,
    /// One [`login_protocol::ATTRIBUTED`] message per successful login, `READ`.
    pub audit: RendezvousId,
    /// **The stand-in terminal** (milestone 49's terminal update): this test harness holds no real
    /// terminal to grant, so it wires a bare rendezvous in its place, `READ`. A test can `ipc_recv`
    /// here to confirm a delegated `TERM_EP` copy actually names this object (real communication,
    /// not merely "a capability arrived"), the same "prove it works, not merely that it arrived"
    /// standard this file's own module doc already sets for the directory and the budget.
    pub term_ep: RendezvousId,
    /// **The stand-in run-unvouched endpoint** (DECISIONS §219 gate D2), `READ`: the progenitor's
    /// role, played by the harness. `login` holds it `WRITE | GRANT` at
    /// `grant_plan::spawnproto::RUN_UNVOUCHED_SLOT`, as the real boot places it, and a test
    /// `ipc_recv`s here to confirm the copy a session was handed names this object.
    pub run_unvouched: RendezvousId,
}

/// Copy `bytes` into fresh read-only pages at consecutive VAs from `base` (milestone 233).
///
/// The kernel-side twin of `supervision_protocol`'s `blobs`, which is how
/// `crates/system_initializer` hands the same two blobs to the same program. Read-only for the
/// same reason that one gives: a program image is data the child reads, and a child that could
/// rewrite the image it was handed could hand a different one on.
///
/// An empty `bytes` maps nothing at all, which is the case `login` is told about by a zero length
/// in its argument register rather than by a mapping it would have to probe.
fn map_blob(space: &mut AddressSpace, base: u64, bytes: &[u8]) {
    let mut off = 0usize;
    while off < bytes.len() {
        let page = space
            .map_new(base + off as u64, Flags::user_rodata())
            .expect("could not map a login blob");
        let n = core::cmp::min(FRAME_SIZE as usize, bytes.len() - off);
        page[..n].copy_from_slice(&bytes[off..off + n]);
        off += n;
    }
}

/// **Wire and spawn the login service.** It parses the initrd for `fs_subtree_caretaker`'s own
/// bytes and then blocks on [`Wiring::request`].
///
/// `verify` is the credential service's verify endpoint (milestone 56), already sealed: login never
/// provisions it and never could. `verify_page_frame` is the exact physical frame that instance maps at
/// its own `VERIFY_VA` (`credential_service::Wiring::verify_page_frame` on the instance `verify` came
/// from). `fs_ep`/`fs_page_frame` are the file service's root directory capability and the page its
/// clients share with it (`fs_service::root_directory`). `construction_pages` bounds how many
/// logins this instance can serve before every further one is answered [`login_protocol::DENIED`] (see
/// `components/src/login.rs`'s BUGS: nothing reclaims a caretaker's region in this slice).
///
/// **`verify_page_frame` is a parameter and not a lookup**, on purpose (milestone 155): a caller that
/// wired more than one credential service in the same boot (as that milestone's own suite does, for
/// a store still open to provision against) cannot ask a bare global "which one," because there is
/// no one answer. Taking the frame from the specific `Wiring` the caller already holds is correct
/// regardless of how many other instances exist or when they were wired.
pub fn start(
    image: &'static [u8],
    verify: RendezvousId,
    verify_page_frame: u64,
    fs_ep: RendezvousId,
    fs_page_frame: u64,
    construction_pages: u64,
) -> Wiring {
    let elf = Elf::parse(image).expect("login is not loadable");

    // **The two blobs `login`'s `_start` is started against** (milestone 233), read out of the same
    // archive this harness used to map wholesale.
    //
    // This changed so that there is **one** contract rather than two. `login` used to read the
    // initrd at `user_mode_runtime::initrd::INITRD_VA`, which this harness could hand it (it is the kernel;
    // it maps reserved RAM directly) and which `crates/system_initializer` could not (`build_child`
    // maps only pages the spawner holds a capability for, and nothing names the archive). So the
    // one path the suite exercised was the one the real boot never took, and `login` died at
    // `_start` on every interactive boot without a single test noticing. A harness that starts a
    // program differently from the way the system starts it is not testing that program.
    let archive = super::initrd().expect("no initrd");
    let fs = nifefs::Fs::parse(archive).expect("the initrd is not a nifefs archive");
    let caretaker = fs.read("fs_subtree_caretaker").unwrap_or(&[]);
    let measurements = fs.read(measured_boot::PROGRAM_MEASUREMENTS).unwrap_or(&[]);

    let content: u64 = elf
        .segments()
        .map(|seg| {
            let (s, e) = seg.page_range(FRAME_SIZE);
            (e - s) / FRAME_SIZE
        })
        .sum::<u64>()
        + 1 // CRED_VA
        + caretaker.len().div_ceil(FRAME_SIZE as usize) as u64
        + measurements.len().div_ceil(FRAME_SIZE as usize) as u64
        + LOGIN_STACK_PAGES
        + 8;
    let mut space = AddressSpace::new(content).expect("no memory for login");
    map_segments(&mut space, &elf).expect("could not lay out login");
    for k in 0..LOGIN_STACK_PAGES {
        space
            .map_new(USER_STACK_VA - k * FRAME_SIZE, Flags::user_data())
            .expect("could not map login's stack");
    }
    #[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
    map_timebase_page(&mut space).expect("could not map login's timebase page");
    map_blob(&mut space, login_protocol::CARETAKER_ELF_VA, caretaker);
    map_blob(
        &mut space,
        login_protocol::PROGRAM_MEASUREMENTS_VA,
        measurements,
    );
    // Milestone 49's channel-per-client update removed the front door's own shared staging page:
    // `CONNECT` (the only word the front door accepts) carries no page at all, and every actual
    // login's identity and secret now travel on a page `login`'s own `connect()` mints and maps at
    // runtime, private to the one client it was minted for. Only the credential-relay page below is
    // still wired here, statically, because it must be the exact frame `credential_service` itself
    // reads from.
    //
    // The credential-relay page: it must be the exact physical frame `credential_service` wired the
    // service's own `VERIFY_VA` to, because that is the only page the credential service ever reads
    // a request from. Taken from the caller's own `Wiring` (see this function's own doc) rather than
    // looked up.
    let cred_page = verify_page_frame;
    space
        .map_physical(
            CRED_VA,
            cred_page,
            Flags::user_data(),
            crate::revoke::PageMapSource::NoCapability,
        )
        .expect("could not map login's credential-relay page");

    let aspace = readopt_user_address_space(space).expect("register the login aspace");

    let request = sched::create_rendezvous();
    let result = sched::create_rendezvous();
    let audit = sched::create_rendezvous();
    // The stand-in terminal (milestone 49's terminal update); see `Wiring::term_ep`'s own doc.
    let term_ep = sched::create_rendezvous();
    let construction = crate::memory_region::create(construction_pages)
        .expect("no construction budget for the login service");

    let thread_control_block_region =
        crate::memory_region::create(2).expect("no tcb region for login");
    let tid =
        sched::create_thread_control_block(thread_control_block_region).expect("no tcb for login");

    // In `components/src/login.rs`'s own slot order: REQUEST, RESULT, VERIFY, FS_EP, FS_PAGE_FRAME,
    // CONSTRUCTION_UT, AUDIT, TERM_EP. Each `assert_eq!` inside `grant_in_order` is that file's
    // own doc read from the other side, the same discipline `authority_tests::spawn_tree` uses for
    // `root_supervisor`.
    //
    // Granted one at a time rather than collected into a `[(&str, Cap); 8]` first: milestone 49's
    // terminal update took that array to eight entries and `start`'s frame to 4288 bytes, over the
    // 4096-byte guard page `script/stack-frame-check` gates against, because an unoptimised build
    // materialises the whole table plus a temporary per element before the first insert ever runs.
    // The array only ever existed to pair each name with its slot index, and the counter below
    // pairs them just as tightly while nothing but one capability is ever live at once: 1632 bytes
    // on aarch64, 1648 on riscv64, measured the same way the gate measures.
    let mut next_slot = 0u64;
    let mut grant_in_order = |name: &str, cap: crate::cap::Cap| {
        let slot = sched::thread_control_block_insert_cap(tid, cap, None)
            .unwrap_or_else(|_| panic!("insert {name}"));
        assert_eq!(
            slot, next_slot,
            "login's {name} must land in slot {next_slot}"
        );
        next_slot += 1;
    };
    grant_in_order("request", rendezvous_cap(request, Rights::READ));
    grant_in_order(
        "result",
        rendezvous_cap(result, Rights::WRITE.union(Rights::GRANT)),
    );
    grant_in_order("verify", rendezvous_cap(verify, Rights::WRITE));
    grant_in_order(
        "fs_ep",
        rendezvous_cap(fs_ep, Rights::WRITE.union(Rights::GRANT)),
    );
    grant_in_order(
        "fs_page_frame",
        // GRANT as well as READ|WRITE: `components/src/login.rs` both maps this frame into every
        // caretaker it builds (`MAP_INTO`, which only checks WRITE) and delegates it directly
        // to every authenticated client (`SEND_CAP`, which needs GRANT on the capability being
        // sent). The second use is why this differs from `credential_service.rs`'s own frames,
        // which are never delegated onward.
        page_frame_cap(
            fs_page_frame,
            Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
        ),
    );
    grant_in_order("construction", memory_region_root_cap(construction));
    grant_in_order("audit", rendezvous_cap(audit, Rights::WRITE));
    grant_in_order(
        "term_ep",
        rendezvous_cap(term_ep, Rights::WRITE.union(Rights::GRANT)),
    );
    assert_eq!(next_slot, 8, "login must hold exactly eight capabilities");
    // **And the run-unvouched capability at its named slot** (DECISIONS §219 gate D2), where
    // `crates/system_initializer` places it in the real boot, so `login` meets one contract here
    // and there.
    let run_unvouched = sched::create_rendezvous();
    let landed = sched::thread_control_block_insert_cap(
        tid,
        rendezvous_cap(run_unvouched, Rights::WRITE.union(Rights::GRANT)),
        Some(grant_plan::spawnproto::RUN_UNVOUCHED_SLOT),
    )
    .expect("insert run_unvouched");
    assert_eq!(landed, grant_plan::spawnproto::RUN_UNVOUCHED_SLOT);

    sched::configure_thread_control_block(tid, elf.entry(), USER_STACK_TOP, aspace)
        .expect("configure");
    sched::start_thread_control_block(tid, [caretaker.len() as u64, measurements.len() as u64, 0])
        .expect("start");

    Wiring {
        request,
        result,
        audit,
        term_ep,
        run_unvouched,
    }
}

/// **Spawn one `login_test_client` run** against `w`, and return its report. Waits for it to
/// finish before returning: `spawn_client` followed by `wait_client` is the same pair, split, for a
/// caller that wants two (or more) runs genuinely in flight together (see those two functions'
/// own docs, and `kernel::user::login_tests` for the isolation proof that needs it).
pub fn client(
    image: &'static [u8],
    w: &Wiring,
    behaviour: u64,
    identity: u64,
    secret: u64,
) -> [u64; 5] {
    wait_client(spawn_client(image, w, behaviour, identity, secret))
}

/// **One `login_test_client` run in flight**: where its report arrives, and what [`wait_client`]
/// gives back once the run is dead (its thread, and the scratch region its page tables came from).
pub struct ClientRun {
    report: RendezvousId,
    tid: crate::thread::ThreadId,
    scratch: u64,
}

/// **Spawn one `login_test_client` run and return its report endpoint immediately**, without
/// waiting for it to run at all. Milestone 49's channel-per-client update is what makes this worth
/// having separately from [`client`]: two runs spawned this way before either is waited on reach
/// the front door on their own schedule, which is genuine concurrency at the front door rather than
/// the artificial kind a single call that spawns-then-waits could ever produce. Pair with
/// [`wait_client`].
///
/// **`identity` and `secret` are separate arguments on purpose** (milestone 293): they compose, so
/// the wrong-secret case is `(fixture::CHRIS, fixture::WRONG)` rather than an eleventh role whose
/// code could drift away from the honest one's. Both are `credential_protocol::fixture` indices.
pub fn spawn_client(
    image: &'static [u8],
    w: &Wiring,
    behaviour: u64,
    identity: u64,
    secret: u64,
) -> ClientRun {
    let report = sched::create_rendezvous();
    // A small, private scratch budget for this one run: milestone 49's channel-per-client update
    // means a run must map the page `login`'s `CONNECT` step delegates before it holds anything
    // else of its own (unlike the post-auth `budget`, `map_page_frame`'s own page-table cost has
    // nowhere else to come from at that point). Independent per run, the same reason
    // `login`'s own `CONSTRUCTION_UT` is never shared with a client: two runs racing to map their
    // own, unrelated pages must never be able to exhaust or interfere with each other's page tables.
    let scratch =
        crate::memory_region::create(CLIENT_SCRATCH_UT_PAGES).expect("no scratch region for a run");
    // Copied out of `w` rather than captured by reference: the spawned closure must be `'static`,
    // and an `RendezvousId` is a plain integer with nothing left to borrow once it is in hand.
    let (request, result) = (w.request, w.result);
    let tid = sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: behaviour,
                arg1: identity,
                arg2: secret,
                grants: &[
                    rendezvous_cap(request, Rights::WRITE),
                    rendezvous_cap(result, Rights::READ),
                    rendezvous_cap(report, Rights::WRITE),
                    memory_region_root_cap(scratch),
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn a login_test_client");
    ClientRun {
        report,
        tid,
        scratch,
    }
}

/// **Block for one run's report**, the other half of [`spawn_client`], then hand the run's scratch
/// back once its thread is gone. The report is the last thing a run sends before it exits, so the
/// wait is for an exit already under way.
pub fn wait_client(run: ClientRun) -> [u64; 5] {
    let report = sched::ipc_recv(run.report);
    let mut held = super::holding::Holding::new();
    held.add_thread(run.tid);
    held.add_region_after_death(run.scratch);
    held.release_or_fail("a login_test_client run");
    report
}
