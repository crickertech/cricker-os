//! **The FS-service client** (milestone 32 phase 2): opens a file through a granted directory
//! capability and proves the whole stack end to end.
//!
//! This is the program a milestone-31 shell will one day be. It holds an endpoint to the FS server
//! and nothing that names the server, the block server, or the disk. That endpoint IS its directory
//! capability: it can open names the server resolves under the bound directory, and it can open
//! nothing else, because there is no global namespace to reach into. It reads the `motd` the image
//! ships with, then writes a pattern to `scratch` and reads it back, and reports the `motd` head
//! plus a success sentinel. Any failed check panics, which faults, which fails the waiting test.
//!
//! # Capability contract (notes/fs-server.md, notes/abi.md §4)
//! - **slot 0**: the file-service endpoint, `WRITE` (this is the directory capability; the client
//!   `CALL`s here).
//! - **slot 1**: the report endpoint, `WRITE`.
//! - **[`FILE_VA`]**: the base of the channel shared with the FS server (a name out, file bytes
//!   both ways). `filesystem_protocol::fs::TRANSFER_PAGES` pages, all of them mapped by this client's wiring,
//!   because the throughput role asks for the whole of it in one request.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `fsclient`. Refused `fsclient`
//! (squished) and `fs_client` (any program that wants files is an FS client, so that name belongs
//! to the real thing; it also predicts "a client of the FS service" rather than "the program the
//! kernel spawns to prove the FS contract holds").

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use filesystem_protocol::{dir, fixture, fs, grant, xattr};
use grant_plan::nav::{TwoRoots, Which};
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{call, exit, now, recv, send};

/// The file-service endpoint: the client's whole authority to the filesystem. Naming a file over it
/// is a request the server resolves under the one directory this endpoint is bound to.
const FILE: u64 = 0;
/// Where the client reports success (WRITE).
const REPORT: u64 = 1;
/// The base of the client's mapping of the channel it shares with the FS server.
///
/// **It is `filesystem_protocol::fs::TRANSFER_MAX` bytes wide, not one page** (milestone 138 step 3), and the
/// kernel's wiring maps every page of it here (`kernel/src/user/fs_service.rs`, `map_channel`). A
/// client may not ask for more than it mapped, and this one maps all of it, which is what lets the
/// throughput role measure the contract's own ceiling rather than a page.
const FILE_VA: u64 = 0x0000_0000_0060_0000;

// SAFETY: the kernel's wiring maps every page of the fs::TRANSFER_MAX-byte channel at FILE_VA
// before this program runs (milestone 139 round 2; see `user_mode_runtime::mapped_window`, which is what
// collapsed the five hand-rolled read_volatile/write_volatile loops below into bounds-checked
// calls).
const WINDOW: MappedWindow = unsafe { MappedWindow::new(FILE_VA, fs::TRANSFER_MAX as u64) };

/// A failed check is a fault, not a wrong answer: panic, and the handler traps.
fn check(ok: bool) {
    if !ok {
        panic!();
    }
}

/// Copy `bytes` into the shared page (a name to open, or data to write).
fn put_page(bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        WINDOW.w8(i as u64, b);
    }
}

/// Copy `bytes` into the shared page at `off`, for the one request that carries two payloads: a
/// rename's source and destination names sit back to back.
fn put_page_at(off: usize, bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        WINDOW.w8((off + i) as u64, b);
    }
}

/// Read `n` bytes out of the shared page (a completed read landed there).
fn get_page(n: usize, out: &mut [u8]) {
    for (i, b) in out.iter_mut().take(n).enumerate() {
        *b = WINDOW.r8(i as u64);
    }
}

/// Open a name under the bound directory; returns the handle. A negative reply (no such file, or a
/// forged request) is REPORTED, not trapped, so the server's reason survives the failure.
fn open(name: &str) -> u64 {
    put_page(name.as_bytes());
    let (r0, _) = call(FILE, fs::req(fs::OPEN, 0, name.len() as u64), 0);
    if (r0 as i64) < 0 {
        fail(STAGE_OPEN, r0);
    }
    r0
}

/// Stage tags for [`fail`], so a reported failure says *which* request the server refused.
const STAGE_OPEN: u64 = 1;
const STAGE_READ: u64 = 2;
const STAGE_WRITE: u64 = 3;

/// **Report a refused request instead of faulting.** A `check` failure traps, and a trapped client
/// tells the waiting test only that something went wrong; the server's *reason* dies with it. This
/// sends it instead, so the reason reaches the kernel test's assertion, which prints the value it
/// compared against `SUCCESS`.
///
/// `w0` is the raw reply word, `w1` is `0xBADD_0000 | stage << 12 | errno`. The errno is recovered
/// with `filesystem_protocol::reply_errno`'s rule (a negative reply is a negated errno). Note the known
/// reply-space overlap (notes/std/fs.md): the kernel's own `invoke` errors are -1..-8, so a small value
/// here is ambiguous between "the server returned this errno" and "the IPC itself failed". The raw
/// word travels in `w0` precisely so that ambiguity is visible rather than hidden.
fn fail(stage: u64, r0: u64) -> ! {
    let errno = if (r0 as i64) < 0 {
        (-(r0 as i64)) as u64
    } else {
        0
    };
    send(REPORT, r0, 0xBADD_0000 | (stage << 12) | errno, 0);
    exit();
}

/// Write `data` to `handle` at `offset`; checks the server accepted the whole slice.
fn write(handle: u64, offset: u64, data: &[u8]) {
    put_page(data);
    let (r0, _) = call(FILE, fs::req(fs::WRITE, handle, data.len() as u64), offset);
    if (r0 as i64) < 0 {
        fail(STAGE_WRITE, r0);
    }
    check(r0 as usize == data.len());
}

/// How long each repeat-write payload is: exactly the fixture pattern's length, so every write in
/// this test (including the final one, which restores the fixture) replaces the file's contents
/// completely. There is no truncate verb, so a shorter write would leave the previous tail behind and
/// the gate's post-run check would see a mixture.
const REPEAT_LEN: usize = fixture::WRITE_PATTERN.len();

/// The payload for repeat-write pass `n`: tagged with the pass, position-dependent after that, and
/// [`REPEAT_LEN`] bytes like every other write here, so a stale or shifted read cannot match. The
/// twin of `redoxfs_server`'s host-side `repeat_write_payload`.
fn repeat_payload(pass: u8) -> [u8; REPEAT_LEN] {
    let mut p = [0u8; REPEAT_LEN];
    p[..8].copy_from_slice(b"CRKRPT__");
    p[7] = b'0' + pass;
    let mut i = 8;
    while i < REPEAT_LEN {
        p[i] = (i as u8).wrapping_mul(31) ^ pass;
        i += 1;
    }
    p
}

/// Read up to `n` bytes from `handle` at `offset` into the shared page; returns the count.
fn read(handle: u64, offset: u64, n: usize) -> usize {
    let (r0, _) = call(FILE, fs::req(fs::READ, handle, n as u64), offset);
    if (r0 as i64) < 0 {
        fail(STAGE_READ, r0);
    }
    r0 as usize
}

/// Iterations for the timed read loop (`ROLE_BENCH`). Warmup pays the `OPEN`; the timed loop then
/// reads the same block over and over. Self-timed, reported home like
/// `os_primitives_benchmarker`'s primitives.
///
/// **"Warm, not disk latency" is what this comment used to claim, and it was wrong when it was
/// written.** Through milestone 138 step 1 there was no warm: `IpcDisk` was a bare `Disk` with no
/// cache around it, so re-reading one block re-read it from the device every time, and the ~204 us
/// this reported was the device round trip. kernel/src/bench.rs and notes/benchmarks.md were
/// corrected on 2026-08-04 and this copy of the claim survived; milestone 38 found it by measuring
/// a *sequential* read against it and getting the same per-block cost, which is exactly what a path
/// with no cache does.
///
/// **It is warm now, and this time the word is accurate.** Step 2 wrapped `IpcDisk` in
/// `redoxfs_server::CachedDisk`, which answers a repeated single-block read (the tree walk this loop's
/// every request repeats, since `motd` lives inline in its node and rides entirely on that walk)
/// from memory. Measured (`sh bench/cache-slots-sweep.sh`): 210,490 ns to 9,474, 22.2x. This role's
/// own report still reads correctly as "the FS-server contract's cost above a raw endpoint call",
/// only warm rather than cold now; `motd` is 69 bytes and lives inline in its node, so this reads
/// the node rather than a record.
const BENCH_ITERS: u64 = 2000;
const BENCH_WARMUP: u64 = 64;

/// Roles, chosen by `arg0` at START (mirrors the kernel side). 0 is the end-to-end proof the test
/// spawns; 1 is the benchmark loop the `--real --smp` bench spawns; 2 is the attacker that a
/// milestone-31 per-file grant is measured against. One binary, six entries.
const ROLE_PROOF: u64 = 0;
const ROLE_BENCH: u64 = 1;
const ROLE_ATTACKER: u64 = 2;
/// Milestone 37: drive the writes the FS server is killed in the middle of.
const ROLE_CRASH_DRIVER: u64 = 3;
/// Milestone 37: read the file back through the FS server that mounted the crashed disk.
const ROLE_CRASH_VERIFY: u64 = 4;
/// Milestone 47: the attacker against a per-*directory* grant. `a1` is a run index, which is the
/// only thing it is told: everything else it has to find out by trying.
const ROLE_DIR_ATTACKER: u64 = 5;
/// Milestone 61: the attribute witness behind a **name-set** grant, the third caretaker. Told
/// nothing; it tries a name the set carries and a name it does not, and reports what got through.
const ROLE_SET_ATTRS: u64 = 6;
/// Milestone 38: sequential and random read/write throughput through the FS server, the four
/// phases `filesystem_protocol::fixture::throughput` names. The `--real --smp` bench boot spawns it after
/// [`ROLE_BENCH`], on the service that role already wired. The number lives in `filesystem_protocol` because
/// the kernel passes it and this program matches on it, which is rule 7's case exactly.
const ROLE_THROUGHPUT: u64 = fixture::throughput::ROLE;
/// Milestone 154: the confined program that holds **two** directory capabilities at once, one at
/// capability table slot 0 and one at slot 1 (`kernel/src/user/fs_service.rs`'s `start_granted_two_dirs`
/// convention). It
/// proves the roadmap block's own deliverable: `/a/x` and `/b/y` both resolve to the grant their
/// label names, `/a/../b` is refused before it ever reaches a caretaker, and neither caretaker's
/// tree is reachable through the other's endpoint.
const ROLE_TWO_DIR: u64 = 10;
/// Milestone 152's first piece, this lane's own demonstration writer: `MKDIR` one identity's home
/// subtree (DECISIONS §117), write its `schedule` file (§122's format, `timetable::parse`'s own
/// dialect unchanged) inside it, and record that identity in the manifest §125 proposes at the
/// store's own root, so `session_reviver`'s read-at-boot path has something to find. The identity
/// and the document are both `schedule_store::fixture` constants, matching every other seed role in
/// this file's own fixed-fixture shape: this role's job is proving the write path
/// works, not exercising a real per-user registration flow (#387, out of this lane's scope).
const ROLE_SCHEDULE_SEED: u64 = 11;
/// Milestone 152's write-path witness, in the seed/verify shape milestone 54's SMB roles used
/// (notes/smb.md): read back
/// what [`ROLE_SCHEDULE_SEED`] wrote, through a **fresh** descent and fresh handles, and confirm the
/// schedule file and the manifest both hold exactly those bytes. Independent of whether
/// `session_reviver`'s own read path agrees, so a store holding the wrong bytes and a re-deriver
/// that misreads the right ones can never be mistaken for each other.
const ROLE_SCHEDULE_VERIFY: u64 = 12;
/// Milestone 599 (a frame per filesystem client channel), provisional: the shared-frame witness's
/// **victim**. Stages its own name, hands off to the attacker over the sync endpoint, and only then
/// calls `OPEN`, so the name the server reads is whatever the attacker left in the page. Reports
/// which file it actually got. See [`share_victim`] and `kernel/src/user/fs_shared_page_tests.rs`.
const ROLE_SHARE_VICTIM: u64 = 13;
/// Milestone 599 (provisional): the shared-frame witness's **attacker**. Holds the same file frame
/// read-write as the victim, waits for the victim to stage its name, overwrites it with a different
/// (same-length) name, and signals back. It never calls the FS server; sharing the frame is the
/// whole of its power. See [`share_attacker`].
const ROLE_SHARE_ATTACKER: u64 = 14;

/// The sync endpoint the two shared-frame witness roles hand off over (slot 2 by the wiring in
/// `fs_service::start_shared_frame_witness`). Both hold it `READ|WRITE`, so each can `SEND` and
/// `RECV`; the roles strictly alternate, so the endpoint is never ambiguous about direction.
const SYNC: u64 = 2;
/// The victim's word to the attacker: "my name is staged in the page".
const SHARE_STAGED: u64 = 0x0599_57A6;
/// The attacker's word back to the victim: "I have overwritten the staged name; call now".
const SHARE_OVERWROTE: u64 = 0x0599_09E2;

#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, a1: u64, _a2: u64) -> ! {
    match role {
        ROLE_BENCH => bench(),
        ROLE_ATTACKER => attacker(a1 != 0),
        ROLE_CRASH_DRIVER => crash_driver(),
        ROLE_CRASH_VERIFY => crash_verify(),
        ROLE_DIR_ATTACKER => dir_attacker(a1),
        ROLE_SET_ATTRS => set_attrs(),
        ROLE_THROUGHPUT => throughput(),
        ROLE_TWO_DIR => two_dir(),
        ROLE_SCHEDULE_SEED => schedule_seed(),
        ROLE_SCHEDULE_VERIFY => schedule_verify(),
        ROLE_SHARE_VICTIM => share_victim(a1),
        ROLE_SHARE_ATTACKER => share_attacker(),
        ROLE_PROOF => proof(),
        _ => proof(),
    }
}

/// **The shared-frame witness's victim** (milestone 599, provisional).
///
/// It stages its own name, and would open exactly that name were it the only writer of the page.
/// The attacker holds the same frame, so between this client staging its name and the server
/// reading it, the attacker rewrites the page. This role forces that ordering deterministically
/// with a two-message handshake rather than relying on a race, because the property under test is
/// not "does the race fire" but "can a second holder of the frame change the bytes the server
/// resolves for this client's call". The server has no way to tell whose bytes it read.
///
/// The report's first word is the verdict: [`fixture::SHARED_SUBSTITUTED`] when the body read back
/// is the attacker's file, [`fixture::SHARED_ISOLATED`] when it is this client's own (what
/// milestone 599's fix makes true), or [`fixture::SHARED_UNEXPECTED`] on any other outcome.
///
/// `window` is this client's channel window (`a1`): the victim **mints its own badged endpoint** for
/// it with `abi::rendezvous::BADGE`, which is what exercises the mint syscall end to end (the
/// attacker gets a pre-badged endpoint from the wiring instead, so the test covers both shapes). It
/// then calls through the badged slot, so the server reads the victim's own window.
fn share_victim(window: u64) -> ! {
    let victim = fixture::SHARED_VICTIM_NAME.as_bytes();
    // Mint a badged view of the FS endpoint naming our window, and call through it from here on.
    let r = user_mode_runtime::badge(FILE, window);
    if r < 0 {
        send(REPORT, fixture::SHARED_UNEXPECTED, (-r) as u64, 0);
        exit();
    }
    let ep = r as u64;
    // Stage our own name, then tell the attacker; the SEND blocks until the attacker RECVs it.
    put_page(victim);
    send(SYNC, SHARE_STAGED, 0, 0);
    // Block until the attacker has written its own window. Only then do we call. Before the fix the
    // attacker shared our frame and this is where the substitution landed; now it cannot reach it.
    let _ = recv(SYNC);
    // The length travels in the register (we choose our own name's length); the name bytes travel
    // in our window, which only we and the server map.
    let (r0, _) = call(ep, fs::req(fs::OPEN, 0, victim.len() as u64), 0);
    if (r0 as i64) < 0 {
        send(REPORT, fixture::SHARED_UNEXPECTED, (-(r0 as i64)) as u64, 0);
        exit();
    }
    let handle = r0;
    let (n, _) = call(ep, fs::req(fs::READ, handle, 8), 0);
    if (n as i64) < 0 {
        send(REPORT, fixture::SHARED_UNEXPECTED, (-(n as i64)) as u64, 0);
        exit();
    }
    let mut head = [0u8; 8];
    get_page(n as usize, &mut head);
    let got = &head[..];
    let verdict = if got == &fixture::SHARED_VICTIM_BODY[..8] {
        fixture::SHARED_ISOLATED
    } else if got == &fixture::SHARED_USURPER_BODY[..8] {
        fixture::SHARED_SUBSTITUTED
    } else {
        fixture::SHARED_UNEXPECTED
    };
    send(REPORT, verdict, 0, 0);
    exit();
}

/// **The shared-frame witness's attacker** (milestone 599, provisional).
///
/// It maps the same file frame the victim and the FS server share, and nothing else that names the
/// filesystem: it never calls the server. It waits for the victim's "staged" word, overwrites the
/// staged name with its own (same-length) name, and signals back. That is the whole of finding 1's
/// "a runnable third party": a second holder of the frame, writing it while the victim is parked in
/// its call.
fn share_attacker() -> ! {
    let _ = recv(SYNC);
    put_page(fixture::SHARED_USURPER_NAME.as_bytes());
    send(SYNC, SHARE_OVERWROTE, 0, 0);
    exit();
}

/// Stage tags [`schedule_seed`] adds to [`fail`]'s vocabulary, beyond [`STAGE_OPEN`]/
/// [`STAGE_READ`]/[`STAGE_WRITE`].
const STAGE_MKDIR: u64 = 4;
const STAGE_OPENDIR: u64 = 5;
const STAGE_CREATE: u64 = 6;

/// `EEXIST`: the standard value (matching `redoxfs`/`syscall`'s own, which `redoxfs_server` returns and
/// nothing in `filesystem_protocol` re-exports under a name), the same constant
/// `identity_provisioner.rs` defines locally for the identical reason.
const EEXIST: i32 = 17;

/// [`ROLE_SCHEDULE_SEED`]'s own body. See that constant's doc for what this proves and why it is a
/// fixed fixture rather than a caller-driven write.
/// Scratch page buffers for milestone 152's schedule-store roles ([`ROLE_SCHEDULE_SEED`],
/// [`ROLE_SCHEDULE_VERIFY`]), in `.bss` rather than on the stack: this program's stack is small
/// (`fs_service::spawn_fs_client`'s default, no extra pages granted to either role), the same
/// reason the SMB adapter's own buffers were `static mut` rather than local, and
/// `filesystem_protocol::PAGE` bytes does not fit it (this crashed under `script/test`'s aarch64 run
/// before this fix landed: a data abort at the stack's guard page, from `schedule_verify` alone
/// putting two page-sized arrays on the stack at once). `schedule_verify` needs both live
/// simultaneously to compare them; `schedule_seed` uses only [`page_buf_a`].
static mut PAGE_BUF_A: [u8; filesystem_protocol::PAGE] = [0; filesystem_protocol::PAGE];
static mut PAGE_BUF_B: [u8; filesystem_protocol::PAGE] = [0; filesystem_protocol::PAGE];

/// One thread per address space (DECISIONS §33), so there is no concurrent access; the previous
/// role's use of this buffer, if any, is done before `_start` ever picks a role to run.
fn page_buf_a() -> &'static mut [u8; filesystem_protocol::PAGE] {
    let p = &raw mut PAGE_BUF_A;
    // SAFETY: see above.
    unsafe { &mut *p }
}
/// Same reasoning as [`page_buf_a`].
fn page_buf_b() -> &'static mut [u8; filesystem_protocol::PAGE] {
    let p = &raw mut PAGE_BUF_B;
    // SAFETY: see `page_buf_a`.
    unsafe { &mut *p }
}

fn schedule_seed() -> ! {
    let identity = schedule_store::fixture::DEMO_IDENTITY.as_bytes();

    // `MKDIR` the identity's own subtree (DECISIONS §117), tolerant of `EEXIST`: a prior test in
    // this suite's one continuous boot may already have created it (or a previous run of this same
    // role, if this test is ever re-entered), and `identity_provisioner.rs`'s own module docs record
    // why that is recovery rather than failure, not a special case invented for this role.
    put_page(identity);
    let (mk, _) = call(FILE, fs::req(fs::MKDIR, 0, identity.len() as u64), dir::ALL);
    if (mk as i64) < 0 && filesystem_protocol::reply_errno(mk as i64) != Some(EEXIST) {
        fail(STAGE_MKDIR, mk);
    }

    // Descend into it and write the schedule file.
    put_page(identity);
    let (dh, _) = call(
        FILE,
        fs::req(fs::OPENDIR, 0, identity.len() as u64),
        dir::ALL,
    );
    if (dh as i64) < 0 {
        fail(STAGE_OPENDIR, dh);
    }
    let sh = create_or_truncate(dh, schedule_store::SCHEDULE_FILE_NAME.as_bytes());
    let doc = schedule_store::fixture::DEMO_SCHEDULE_DOC.as_bytes();
    write(sh, 0, doc);
    let (size, _) = call(FILE, fs::req(fs::FSTAT, sh, 0), 0);
    check(size as usize == doc.len());
    let (c, _) = call(FILE, fs::req(fs::CLOSE, sh, 0), 0);
    check((c as i64) >= 0);
    let (cd, _) = call(FILE, fs::req(fs::CLOSE, dh, 0), 0);
    check((cd as i64) >= 0);

    // Record the identity in the manifest at the store's own root (DECISIONS §125), so
    // `session_reviver` has a hard-wired set to read without ever enumerating.
    let buf = page_buf_a();
    let n = schedule_store::render_manifest(&[identity], buf)
        .expect("one short identity name always fits the manifest buffer");
    let mh = create_or_truncate(0, schedule_store::MANIFEST_FILE_NAME.as_bytes());
    write(mh, 0, &buf[..n]);
    let (msize, _) = call(FILE, fs::req(fs::FSTAT, mh, 0), 0);
    check(msize as usize == n);
    let (mc, _) = call(FILE, fs::req(fs::CLOSE, mh, 0), 0);
    check((mc as i64) >= 0);

    send(REPORT, fixture::SUCCESS, 0, 0);
    exit();
}

/// `CREATE` a name under `dir_handle`, or open and `TRUNCATE` it if it is already there, matching
/// The seed idiom generalized to an arbitrary directory handle rather than only [`fs::ROOT`].
fn create_or_truncate(dir_handle: u64, name: &[u8]) -> u64 {
    put_page(name);
    let (r0, _) = call(FILE, fs::req(fs::CREATE, dir_handle, name.len() as u64), 0);
    if (r0 as i64) >= 0 {
        return r0;
    }
    put_page(name);
    let (r0b, _) = call(FILE, fs::req(fs::OPEN, dir_handle, name.len() as u64), 0);
    if (r0b as i64) < 0 {
        fail(STAGE_CREATE, r0b);
    }
    let (t, _) = call(FILE, fs::req(fs::TRUNCATE, r0b, 0), 0);
    check((t as i64) >= 0);
    r0b
}

/// [`open`]'s own logic, generalized to an arbitrary directory handle rather than only
/// [`fs::ROOT`] (`open` never needs a subtree; every other caller of it stays as it was).
fn open_at(dir_handle: u64, name: &[u8]) -> u64 {
    put_page(name);
    let (r0, _) = call(FILE, fs::req(fs::OPEN, dir_handle, name.len() as u64), 0);
    if (r0 as i64) < 0 {
        fail(STAGE_OPEN, r0);
    }
    r0
}

/// A report code distinct from [`fixture::SUCCESS`]: the schedule file's bytes did not match what
/// [`ROLE_SCHEDULE_SEED`] wrote. Word 1 of the report is how many bytes this role actually read.
const RPT_SCHEDULE_MISMATCH: u64 = 0xBAD5_0001;
/// As [`RPT_SCHEDULE_MISMATCH`], for the manifest instead of the schedule file.
const RPT_MANIFEST_MISMATCH: u64 = 0xBAD5_0002;

/// [`ROLE_SCHEDULE_VERIFY`]'s own body. See that constant's doc for what this proves.
fn schedule_verify() -> ! {
    let identity = schedule_store::fixture::DEMO_IDENTITY.as_bytes();
    let expected_doc = schedule_store::fixture::DEMO_SCHEDULE_DOC.as_bytes();

    put_page(identity);
    let (dh, _) = call(
        FILE,
        fs::req(fs::OPENDIR, 0, identity.len() as u64),
        dir::ALL,
    );
    if (dh as i64) < 0 {
        fail(STAGE_OPENDIR, dh);
    }
    let sh = open_at(dh, schedule_store::SCHEDULE_FILE_NAME.as_bytes());
    // One byte past what is expected, so a file that is *longer* than expected (a bug that wrote
    // extra bytes) is visible as a mismatch rather than silently truncated to a match.
    let n = read(
        sh,
        0,
        (expected_doc.len() + 1).min(filesystem_protocol::PAGE),
    );
    let got = page_buf_a();
    get_page(n, got);
    let (c, _) = call(FILE, fs::req(fs::CLOSE, sh, 0), 0);
    check((c as i64) >= 0);
    let (cd, _) = call(FILE, fs::req(fs::CLOSE, dh, 0), 0);
    check((cd as i64) >= 0);
    if &got[..n] != expected_doc {
        send(REPORT, RPT_SCHEDULE_MISMATCH, n as u64, 0);
        exit();
    }

    // `page_buf_a` (holding the schedule file's bytes) has already been compared and is done being
    // read; reused here for the manifest's own expected rendering, since the two are never live at
    // once. `page_buf_b` holds what was actually read back, so both are live together for the
    // comparison below, which is why this role needs two page-sized `.bss` buffers rather than one.
    let expected_manifest = page_buf_a();
    let mn = schedule_store::render_manifest(&[identity], expected_manifest)
        .expect("one short identity name always fits the manifest buffer");
    let mh = open(schedule_store::MANIFEST_FILE_NAME);
    let got_n = read(mh, 0, (mn + 1).min(filesystem_protocol::PAGE));
    let got_manifest = page_buf_b();
    get_page(got_n, got_manifest);
    let (mc, _) = call(FILE, fs::req(fs::CLOSE, mh, 0), 0);
    check((mc as i64) >= 0);
    if got_manifest[..got_n] != expected_manifest[..mn] {
        send(REPORT, RPT_MANIFEST_MISMATCH, got_n as u64, 0);
        exit();
    }

    send(REPORT, fixture::SUCCESS, 0, 0);
    exit();
}

/// **The crash driver** (milestone 37): write one payload and get an acknowledgement, then write the
/// next one into a server that has been told to die partway through it.
///
/// It reports after the acknowledged write, because that report is the thing the property depends
/// on: "payload A was acknowledged" is what makes losing it a failure rather than a permitted
/// outcome. Then it issues the second write and **never returns from it**, because the server dies
/// inside the request and blocking IPC has no "your server is gone" reply.
///
/// That permanent block is not an oversight, it is the open item DECISIONS §27 and notes/fs-server.md
/// already record: a client of a dead server waits forever, and §26's fault endpoint wired into a
/// supervision tree (milestone 23) is the mechanism that would turn it into a message. This test
/// exhibits it rather than working around it; the thread is `Blocked`, so it costs no CPU, and the
/// kernel test does not wait on this client for anything after the first report.
fn crash_driver() -> ! {
    use fixture::crash;
    let h = open(crash::NAME);

    // Write A and read it straight back. "The server accepted my write" and "my write landed" are
    // different claims, and the second is the one that has to be true before the kill means anything.
    write(h, 0, crash::A);
    let mut buf = [0u8; 128];
    let n = read(h, 0, crash::A.len());
    check(n == crash::A.len());
    get_page(n, &mut buf);
    check(&buf[..n] == crash::A);
    send(REPORT, fixture::SUCCESS, crash::SAW_A, 0);

    // And now the one the server dies inside. This call never returns.
    write(h, 0, crash::B);
    // Unreachable in the test's configuration; if the injector failed to fire, say so rather than
    // exiting quietly and leaving the kernel test to time out with no explanation.
    send(REPORT, fixture::SUCCESS, crash::SAW_B, 0);
    exit();
}

/// **The crash verifier** (milestone 37): a fresh client of a fresh FS server that mounted the disk
/// the killed one left behind. It reports which of the two payloads the file holds.
///
/// It classifies rather than asserts, for the reason milestone 31's attacker reports a bitmap: the
/// interesting failure is not "it did not read back", it is "it read back something that was never
/// written", and a client that panicked on a mismatch would tell the kernel test nothing about
/// which.
fn crash_verify() -> ! {
    use fixture::crash;
    let h = open(crash::NAME);
    let mut buf = [0u8; 256];
    let n = read(h, 0, crash::A.len().max(crash::B.len()) + 8);
    get_page(n, &mut buf);
    let saw = if n == crash::A.len() && &buf[..n] == crash::A {
        crash::SAW_A
    } else if n == crash::B.len() && &buf[..n] == crash::B {
        crash::SAW_B
    } else {
        crash::SAW_NEITHER
    };
    // The length rides home in the first word: a partial write shows up as a number rather than as
    // a bare verdict, which is the difference between "this failed" and "this is what it did".
    send(REPORT, n as u64, saw, 0);
    exit();
}

/// **The attacker against a per-file grant** (milestone 31 phase 2). Spawned holding a *narrowed*
/// endpoint: not the directory capability, but the file caretaker's, which designates exactly one
/// file read-only. Its job is to try everything that would make that sentence false, and to report
/// which attempts got through as a bitmap (`fixture::escape`).
///
/// **It is its own negative control, which is why it reports a bitmap and not a pass.** Run against
/// a read-only grant, every bit must be clear. Run against a read/write grant of the same shape,
/// `WROTE` and `TRUNCATED` must be **set** and everything else clear. Without that second run the
/// first proves very little: a caretaker that refused every request would pass it, and so would a
/// grant that reached nothing at all.
///
/// **What makes the attempts real.** Each one is against something that exists and that the process
/// one hop up the chain can genuinely reach: the neighbour file is on the image, one directory
/// entry away, and the caretaker could open it on any request it liked. Milestone 33's attacker was
/// handed a real neighbouring client's address rather than a fictional one for exactly this reason,
/// and milestone 36 used two witnesses for the reason the paragraph above gives.
///
/// `writable` also selects *which* file is granted, and that is a fixture constraint rather than a
/// design one: the writable run damages what it is given, so it is given `scratch` (whose contents
/// the gate's post-run host check pins, and which this restores as its last write) rather than
/// `motd`, which two other tests compare byte for byte.
fn attacker(writable: bool) -> ! {
    use fixture::escape;
    let mut verdict = 0u64;
    let mut buf = [0u8; 128];

    let (granted, neighbour) = if writable {
        (fixture::SCRATCH_NAME, fixture::MOTD_NAME)
    } else {
        (fixture::MOTD_NAME, fixture::SCRATCH_NAME)
    };

    // The control: the one file this capability designates must open, and read back what is on it.
    //
    // The read-only run compares against `MOTD`, which nothing ever writes. The writable run cannot
    // and must not: the suite runs its tests in **alphabetical order**, so this program runs before
    // the two clients that put a known pattern in `scratch`, and asserting on those bytes here would
    // be asserting on a history this test did not write. That is the exact failure DECISIONS §27 was
    // corrected four times over, met again from the other side. So the writable run's control is that
    // the file reads back the number of bytes `FSTAT` says it has, and, further down, that the bytes
    // it writes are the bytes it reads back.
    put_page(granted.as_bytes());
    let (h, _) = call(FILE, fs::req(fs::OPEN, 0, granted.len() as u64), 0);
    if (h as i64) < 0 {
        verdict |= escape::GRANTED_READ_FAILED;
    } else {
        let (size, _) = call(FILE, fs::req(fs::FSTAT, h, 0), 0);
        let want = if writable {
            size as usize
        } else {
            fixture::MOTD.len()
        };
        let (n, _) = call(FILE, fs::req(fs::READ, h, want as u64), 0);
        if (size as i64) < 0 || (n as i64) < 0 || n as usize != want || want > buf.len() {
            verdict |= escape::GRANTED_READ_FAILED;
        } else {
            get_page(n as usize, &mut buf);
            if !writable && &buf[..n as usize] != fixture::MOTD {
                verdict |= escape::GRANTED_READ_FAILED;
            }
        }
    }

    // 1. A second file, by name. It exists, it sits in the same directory, and the caretaker could
    //    open it. This capability names one file, so this must find nothing.
    put_page(neighbour.as_bytes());
    let (r, _) = call(FILE, fs::req(fs::OPEN, 0, neighbour.len() as u64), 0);
    if (r as i64) >= 0 {
        verdict |= escape::SECOND_FILE;
    }

    // 2. A write, against the file it IS allowed to read. Refusing a write to a file it cannot even
    //    name would prove nothing; this is the sharp case. When it is *supposed* to succeed, the
    //    bytes are read straight back: "the server accepted my write" and "my write landed" are
    //    different claims, and only the second one makes this a control for the refusal above.
    let probe = probe_payload();
    put_page(&probe[..PROBE_LEN]);
    let (w, _) = call(FILE, fs::req(fs::WRITE, grant::HANDLE, PROBE_LEN as u64), 0);
    if (w as i64) >= 0 {
        verdict |= escape::WROTE;
        let (rb, _) = call(FILE, fs::req(fs::READ, grant::HANDLE, PROBE_LEN as u64), 0);
        get_page(PROBE_LEN, &mut buf);
        if rb as usize != PROBE_LEN || buf[..PROBE_LEN] != probe[..PROBE_LEN] {
            verdict |= escape::GRANTED_READ_FAILED;
        }
    }

    // 3. Truncation is a write that carries no bytes, so a guard that only covered WRITE would miss
    //    it, and truncating to zero destroys a file just as thoroughly.
    let (t, _) = call(FILE, fs::req(fs::TRUNCATE, grant::HANDLE, 0), 0);
    if (t as i64) >= 0 {
        verdict |= escape::TRUNCATED;
    }

    // 4. Creating a new name: the way to get a second file without opening one that exists. Refused
    //    in both directions, because a file capability is not a directory.
    put_page(b"made-by-attacker");
    let (c, _) = call(FILE, fs::req(fs::CREATE, 0, 16), 0);
    if (c as i64) >= 0 {
        verdict |= escape::CREATED;
    }

    // 5. Handle guessing. The caretaker minted one handle and the FS server's own handle for the
    //    file is a different number this process never saw, so spraying numbers is probing a table
    //    it is not addressing. Every miss must be refused by the same check.
    let mut guess = 1u64;
    while guess < 8 {
        let (g, _) = call(FILE, fs::req(fs::READ, guess, 8), 0);
        if (g as i64) >= 0 {
            verdict |= escape::FORGED_HANDLE;
        }
        guess += 1;
    }

    // 6. **Extended attributes** (milestone 61). Before it, all three caretakers answered
    //    `EOPNOTSUPP` to all four verbs, so a program behind a per-file grant could not reach its
    //    own file's attributes. That was uniform and honest and still a capability the confined
    //    program should have had: an attribute is part of what a file *is*, so a capability that may
    //    read the file may read what is attached to it, and one that may not write it may not change
    //    them.
    //
    //    Three probes, and the middle one is the sharp one.
    //
    //    A listing works through **either** direction, because it is a read. Zero attributes is a
    //    successful listing (`r0` = 0), so this distinguishes "forwarded and there were none" from
    //    "refused", which the old behaviour could not.
    let (l, _) = call(FILE, fs::req(fs::LISTXATTR, grant::HANDLE, 0), 0);
    if (l as i64) < 0 {
        verdict |= escape::GRANTED_ATTRS_FAILED;
    }

    //    A get of an attribute that is not set must answer `ENODATA`, **not** `EOPNOTSUPP`. That is
    //    the difference between a request that reached the store and one the caretaker refused, and
    //    it is the only probe here that a read-only grant can make about a verb that reads.
    put_page(fixture::attrs::NAME);
    let (g0, _) = call(
        FILE,
        fs::req(
            fs::GETXATTR,
            grant::HANDLE,
            fixture::attrs::NAME.len() as u64,
        ),
        0,
    );
    if filesystem_protocol::reply_errno(g0 as i64) == Some(xattr::ENOTSUP) {
        verdict |= escape::GRANTED_ATTRS_FAILED;
    }

    //    A set: accepted only with the write direction, which is `WROTE`'s and `TRUNCATED`'s rule
    //    applied to the third way of changing a file. When it is accepted it is read straight back
    //    with its **type code**, because "the server took my attribute" and "my attribute is on the
    //    file" are different claims, and then removed, so the run leaves the image as it found it.
    put_page(fixture::attrs::NAME);
    put_page_at(fixture::attrs::NAME.len(), fixture::attrs::VALUE);
    let (s, _) = call(
        FILE,
        fs::req(
            fs::SETXATTR,
            grant::HANDLE,
            fixture::attrs::NAME.len() as u64,
        ),
        xattr::spec(fixture::attrs::KIND, fixture::attrs::VALUE.len() as u64),
    );
    if (s as i64) >= 0 {
        verdict |= escape::WROTE_ATTR;
        put_page(fixture::attrs::NAME);
        let (g, _) = call(
            FILE,
            fs::req(
                fs::GETXATTR,
                grant::HANDLE,
                fixture::attrs::NAME.len() as u64,
            ),
            0,
        );
        get_page(xattr::reply_value_len(g as i64), &mut buf);
        if (g as i64) < 0
            || xattr::reply_kind(g as i64) != fixture::attrs::KIND
            || xattr::reply_value_len(g as i64) != fixture::attrs::VALUE.len()
            || &buf[..fixture::attrs::VALUE.len()] != fixture::attrs::VALUE
        {
            verdict |= escape::GRANTED_ATTRS_FAILED;
        }
        put_page(fixture::attrs::NAME);
        let (rm, _) = call(
            FILE,
            fs::req(
                fs::REMOVEXATTR,
                grant::HANDLE,
                fixture::attrs::NAME.len() as u64,
            ),
            0,
        );
        if (rm as i64) < 0 {
            verdict |= escape::GRANTED_ATTRS_FAILED;
        }
    } else if writable {
        // A writable grant that cannot set one is the control for the refusal: without it, a
        // caretaker that refused every attribute request would pass the read-only run.
        verdict |= escape::GRANTED_ATTRS_FAILED;
    }

    // Leave the fixture behind, as the LAST write, on the run that was allowed to damage it. The gate
    // reopens the image with the host tool afterwards and compares `scratch` byte for byte, so an
    // attacker that walked away from a truncated file would fail a check three steps downstream and
    // look like a filesystem bug. That exact coupling is what DECISIONS §27 was corrected four times
    // over, so it is paid off here rather than left to be rediscovered.
    if writable {
        let pat = fixture::WRITE_PATTERN;
        put_page(pat);
        let (rw, _) = call(FILE, fs::req(fs::WRITE, grant::HANDLE, pat.len() as u64), 0);
        let (rr, _) = call(FILE, fs::req(fs::READ, grant::HANDLE, pat.len() as u64), 0);
        get_page(pat.len(), &mut buf);
        if (rw as i64) < 0 || rr as usize != pat.len() || &buf[..pat.len()] != pat {
            verdict |= escape::GRANTED_READ_FAILED;
        }
    }

    send(REPORT, fixture::VERDICT, verdict, 0);
    exit();
}

/// **The attacker against a per-directory grant** (milestone 47). Spawned holding an endpoint to a
/// `fs_subtree_caretaker`, which is a capability to one subtree with one rights set. Its job is to
/// make "it cannot reach its parent or a sibling, and it carries no right it was not given" false.
///
/// **It is told nothing about its own grant** beyond a run index (which only keeps the names it
/// creates distinct across the runs that share one image). That is deliberate: it attempts every
/// verb and reports a bitmap of what got through, and the kernel test asserts the *exact* expected
/// set for each configuration. So the specification lives in the test rather than in the program
/// being tested, and three runs of the same code are each other's controls: a caretaker that
/// refused everything fails the wide run, and a caretaker that allowed everything fails the narrow
/// one.
///
/// **What makes the attempts real.** `motd` is in the parent and `other/secret` is in a sibling, and
/// both are on the image, one directory entry from the caretaker, which could open either on any
/// request it liked. Milestone 33's attacker was handed a real neighbour's address for exactly this
/// reason.
///
/// It never writes to `sub/inner` or anything outside `sub`, so the post-run host check can pin
/// those bytes from outside the guest and catch an escape this program failed to report.
fn dir_attacker(run: u64) -> ! {
    use fixture::{dirscape as esc, tree};
    let mut v = 0u64;
    let mut buf = [0u8; 256];

    // 1. The parent. `motd` is in the granted directory's parent and is not in the granted
    //    directory, so this must find nothing. Not a refusal: in this scope there is no such name.
    if dir_open(fs::ROOT, fixture::MOTD_NAME) >= 0 {
        v |= esc::REACHED_PARENT;
    }
    // 2. The sibling, by both routes: descending into it and naming the file inside it.
    if dir_descend(fs::ROOT, tree::OTHER, dir::ALL) >= 0 || dir_open(fs::ROOT, tree::SECRET) >= 0 {
        v |= esc::REACHED_SIBLING;
    }
    // 3. `..`, which is not a component this contract accepts, at any rights.
    if dir_descend(fs::ROOT, "..", dir::ALL) >= 0 || dir_open(fs::ROOT, "..") >= 0 {
        v |= esc::WALKED_UP;
    }

    // 4. The control: the file inside the grant. A capability that reaches nothing is trivially
    //    unescapable, so without this bit every refusal above proves nothing.
    let own = dir_open(fs::ROOT, tree::INNER);
    if own >= 0 {
        v |= esc::OPENED_ITS_OWN;
        let (n, _) = call(
            FILE,
            fs::req(fs::READ, own as u64, tree::INNER_BODY.len() as u64),
            0,
        );
        get_page(n as usize, &mut buf);
        if (n as i64) < 0
            || n as usize != tree::INNER_BODY.len()
            || &buf[..n as usize] != tree::INNER_BODY
        {
            v |= esc::GRANTED_ACCESS_FAILED;
        }

        // 4b. **Its attributes** (milestone 61). Reading them needs what reading the file needs, so
        //     every configuration that got here should manage it; writing one needs `dir::WRITE`,
        //     and the caretaker checks nothing, so the refusal comes from the FS server acting on
        //     the rights the caretaker's one `OPENDIR` left on this handle.
        //
        //     The attribute is set on `inner` and then removed, so the post-run host check still
        //     sees the file it pins, byte for byte, with nothing attached.
        let (l, _) = call(FILE, fs::req(fs::LISTXATTR, own as u64, 0), 0);
        if (l as i64) < 0 {
            v |= esc::GRANTED_ACCESS_FAILED;
        } else {
            v |= esc::READ_ATTRS;
        }

        put_page(fixture::attrs::NAME);
        put_page_at(fixture::attrs::NAME.len(), fixture::attrs::VALUE);
        let (s, _) = call(
            FILE,
            fs::req(fs::SETXATTR, own as u64, fixture::attrs::NAME.len() as u64),
            xattr::spec(fixture::attrs::KIND, fixture::attrs::VALUE.len() as u64),
        );
        if (s as i64) >= 0 {
            put_page(fixture::attrs::NAME);
            let (g, _) = call(
                FILE,
                fs::req(fs::GETXATTR, own as u64, fixture::attrs::NAME.len() as u64),
                0,
            );
            get_page(xattr::reply_value_len(g as i64), &mut buf);
            if (g as i64) >= 0
                && xattr::reply_kind(g as i64) == fixture::attrs::KIND
                && xattr::reply_value_len(g as i64) == fixture::attrs::VALUE.len()
                && buf[..fixture::attrs::VALUE.len()] == *fixture::attrs::VALUE
            {
                v |= esc::SET_AN_ATTR;
            } else {
                v |= esc::GRANTED_ACCESS_FAILED;
            }
            put_page(fixture::attrs::NAME);
            let (rm, _) = call(
                FILE,
                fs::req(
                    fs::REMOVEXATTR,
                    own as u64,
                    fixture::attrs::NAME.len() as u64,
                ),
                0,
            );
            if (rm as i64) < 0 {
                v |= esc::GRANTED_ACCESS_FAILED;
            }
        }
    }

    // 5. Enumeration, and what it is allowed to contain. A listing is a rendering of authority, so
    //    a name from outside the grant appearing in one is an escape even though nothing was opened.
    let (n, _) = call(FILE, fs::req(fs::READDIR, fs::ROOT, 0), 0);
    if (n as i64) >= 0 {
        v |= esc::ENUMERATED;
        // Clamped to the local buffer: the page is sixteen times larger, and indexing past the
        // buffer with a length the server chose would be a panic this program cannot report.
        let n = (n as usize).min(buf.len());
        get_page(n, &mut buf);
        for (name, _) in filesystem_protocol::dirent::iter(&buf[..n]) {
            for stranger in [
                fixture::MOTD_NAME,
                fixture::SCRATCH_NAME,
                tree::OTHER,
                tree::SECRET,
            ] {
                if name == stranger.as_bytes() {
                    v |= esc::ENUMERATED_A_STRANGER;
                }
            }
        }
    }

    // 6. Making a name inside the grant, and 7. making a directory inside it. Both need CREATE;
    //    `mkdir` needs DESCEND too, because a directory you could not have walked into would be a
    //    way to mint a capability out of a right that was withheld.
    let made = run_name(tree::MADE, run);
    let created = dir_create(fs::ROOT, made_str(&made));
    if created >= 0 {
        v |= esc::CREATED;
        // 8. A write, to what it just made, read straight back: "the server accepted my write" and
        //    "my write landed" are different claims. Never to `inner`, which the host check pins.
        put_page(tree::MADE_BODY);
        let (w, _) = call(
            FILE,
            fs::req(fs::WRITE, created as u64, tree::MADE_BODY.len() as u64),
            0,
        );
        if (w as i64) >= 0 {
            v |= esc::WROTE;
            let (rb, _) = call(
                FILE,
                fs::req(fs::READ, created as u64, tree::MADE_BODY.len() as u64),
                0,
            );
            get_page(tree::MADE_BODY.len(), &mut buf);
            if rb as usize != tree::MADE_BODY.len()
                || &buf[..tree::MADE_BODY.len()] != tree::MADE_BODY
            {
                v |= esc::GRANTED_ACCESS_FAILED;
            }
        }
    }
    let made_dir = run_name(tree::MADE_DIR, run);
    if dir_mkdir(fs::ROOT, made_str(&made_dir), dir::READ) >= 0 {
        v |= esc::MADE_A_DIR;
    }

    // 9. Descending inside the grant, which is allowed exactly when DESCEND was granted. Asking for
    //    DESCEND alone, so this probe is about reaching the child rather than about its rights.
    let child = dir_descend(fs::ROOT, tree::DEEPER, dir::DESCEND);
    if child >= 0 {
        v |= esc::DESCENDED;
    }

    // 10. **Widening, the question this milestone exists to answer**, probed two ways.
    //
    //   (a) A child asked for NOTHING must be able to do nothing. If a zero-rights directory
    //       capability opens, lists or creates, rights are not being carried on the handle at all.
    let nothing = dir_descend(fs::ROOT, tree::DEEPER, 0);
    if nothing >= 0 {
        let listed = call(FILE, fs::req(fs::READDIR, nothing as u64, 0), 0).0 as i64;
        if dir_open(nothing as u64, tree::LEAF) >= 0
            || dir_create(nothing as u64, "smuggled") >= 0
            || listed >= 0
        {
            v |= esc::WIDENED;
        }
    }
    //   (b) A child cannot carry what the parent did not. If creating in the grant was refused, the
    //       grant has no CREATE, so a descent that asks for CREATE must be refused too.
    if created < 0 && dir_descend(fs::ROOT, tree::DEEPER, dir::ALL) >= 0 {
        v |= esc::WIDENED;
    }

    // 11. **Renaming, which is the only verb on this wire that consults `REMOVE`.** It moves the
    //     name it made itself, never `inner`, so a run that is allowed to do this damages only its
    //     own leavings and the post-run host check can still pin the fixture. The attempt is made
    //     whatever the grant is: the server checks the rights before it resolves anything, so a
    //     capability with no `REMOVE` gets the same refusal whether or not the source is there,
    //     and this program is not told which case it is in.
    let moved = run_name(tree::MOVED, run);
    if dir_rename(fs::ROOT, made_str(&made), fs::ROOT, made_str(&moved)) >= 0 {
        v |= esc::RENAMED;
    }

    // 12. Handle guessing, past anything the caretaker could have minted for it. The caretaker
    //     numbers its client's handles in its own space, so a number it never issued is refused by
    //     one check.
    let mut guess = 9u64;
    while guess < 16 {
        if call(FILE, fs::req(fs::READ, guess, 8), 0).0 as i64 >= 0
            || call(FILE, fs::req(fs::READDIR, guess, 0), 0).0 as i64 >= 0
        {
            v |= esc::FORGED_HANDLE;
        }
        guess += 1;
    }

    // Nothing at all worked: report it, so a capability that reaches nothing cannot pass as a
    // capability that is perfectly confined.
    if v & (esc::OPENED_ITS_OWN | esc::ENUMERATED | esc::DESCENDED | esc::CREATED) == 0 {
        v |= esc::GRANTED_ACCESS_FAILED;
    }

    send(REPORT, fixture::VERDICT, v, 0);
    exit();
}

/// **The two-directory witness** (milestone 154,
/// design/roadmap/154-multi-directory-namespace.md).
///
/// It holds two `fs_subtree_caretaker`s at once, one at capability table slot 0 and one at slot 1
/// (`kernel/src/user/fs_service.rs`'s `start_granted_two_dirs` convention, which the kernel test wiring this role
/// follows). It is told nothing about either grant beyond that ordering; the kernel test wires
/// slot 0 to the fixture's `sub` (labeled `a`) and slot 1 to `other` (labeled `b`), and this
/// program only ever learns that through the labels it was compiled to use.
///
/// It proves the roadmap block's own deliverable, in its own words:
/// - `/a/inner` and `/b/secret` (the milestone's `/a/x` and `/b/y`) both resolve, through
///   [`grant_plan::nav::TwoRoots`]'s pure resolution and then a real `OPEN` on the caretaker the
///   resolution named.
/// - `/a/../b` is refused **before any request is sent**: `TwoRoots::resolve` returns an error,
///   the same way a single grant's `..` at its own root already does, so the witness never even
///   asks slot 1's caretaker for anything.
/// - **Neither caretaker's tree is reachable through the other's endpoint**, checked directly by
///   asking slot 0 for the name that only exists in grant B's subtree, and slot 1 for the name
///   that only exists in grant A's, which is the structural finding notes/dir-capability.md
///   already argues (the endpoint is the boundary), witnessed here with two live caretakers
///   rather than inferred from one.
fn two_dir() -> ! {
    use filesystem_protocol::fixture::twodir as t;
    use fixture::tree;

    /// Grant A's endpoint: `fs_service::start_granted_two_dirs`' slot 0.
    const FILE_A: u64 = 0;
    /// Grant B's endpoint: slot 1.
    const FILE_B: u64 = 1;
    /// Where this role reports, one slot past the two directory endpoints.
    const REPORT_SLOT: u64 = 2;

    let mut v = 0u64;
    // The labels are this witness's own choice, independent of what the grants are named on
    // disk (`sub`, `other`): a namespace label is assigned by whoever composes the namespace,
    // not by the directory it points at, and using different words for the two makes that plain.
    let ns = TwoRoots::new(b"a", b"b");

    // `/a/inner`: resolves to grant A, one component deep, and opens through slot 0.
    match ns.resolve(b"/a/inner") {
        Ok((Which::A, cwd)) if cwd.depth() == 1 => {
            if dir_open_on(FILE_A, fs::ROOT, cwd.component(0)) >= 0 {
                v |= t::OPENED_A;
            } else {
                v |= t::GRANTED_ACCESS_FAILED;
            }
        }
        _ => v |= t::GRANTED_ACCESS_FAILED,
    }

    // `/b/secret`: the same claim, through the second grant and slot 1.
    match ns.resolve(b"/b/secret") {
        Ok((Which::B, cwd)) if cwd.depth() == 1 => {
            if dir_open_on(FILE_B, fs::ROOT, cwd.component(0)) >= 0 {
                v |= t::OPENED_B;
            } else {
                v |= t::GRANTED_ACCESS_FAILED;
            }
        }
        _ => v |= t::GRANTED_ACCESS_FAILED,
    }

    // The negative control: a path that ascends out of grant A and into grant B. Resolution
    // itself must refuse it, so nothing below sends a request over it at all.
    if ns.resolve(b"/a/../b").is_ok() {
        v |= t::DOT_DOT_CROSSED_MOUNTS;
    }

    // And from the wire's own side, independent of `TwoRoots` entirely: grant A's caretaker
    // holds nothing that names grant B's tree, and the reverse. `secret` is real, on the image,
    // and one directory entry away from the caretaker one hop up; a capability to `sub` reaching
    // it would be an escape whether or not any namespace logic tried to stop it.
    if dir_open_on(FILE_A, fs::ROOT, tree::SECRET.as_bytes()) >= 0 {
        v |= t::REACHED_B_VIA_A;
    }
    if dir_open_on(FILE_B, fs::ROOT, tree::INNER.as_bytes()) >= 0 {
        v |= t::REACHED_A_VIA_B;
    }

    send(REPORT_SLOT, fixture::VERDICT, v, 0);
    exit();
}

/// `OPEN` a name under a directory handle, over a chosen endpoint slot: [`two_dir`]'s own
/// helper, because every other role in this file talks to exactly one caretaker at slot
/// [`FILE`] and this is the one role that holds two.
fn dir_open_on(ep: u64, parent: u64, name: &[u8]) -> i64 {
    put_page(name);
    call(ep, fs::req(fs::OPEN, parent, name.len() as u64), 0).0 as i64
}

/// **The attribute witness behind a name-set grant** (milestone 61, the third caretaker).
///
/// `fs_nameset_caretaker` narrows a directory capability to a designated *set* of names, and it is
/// the one caretaker that inspects a name on every request. Milestone 61 taught it four verbs whose
/// operand is an **attribute** name rather than a directory name, and that distinction is the whole
/// risk: filtering an attribute name against the set would refuse a program its own file's
/// attributes, and *not* filtering a directory name would be an escape. This asks both questions.
///
/// It is told nothing about its grant. The kernel test wires it over the `sub` fixture with a set of
/// exactly one name, so `inner` is designated and `deeper`, one entry away in the same directory, is
/// not. `rm`'s witness already asks the naming question with `REMOVE` alone; this asks it with
/// `READ`, because closing an attribute gap by opening a naming one would be a poor trade.
///
/// The attribute is removed before it reports, so the file the gate's post-run host check pins is
/// left byte for byte as it was found, with nothing attached.
fn set_attrs() -> ! {
    use fixture::{dirscape as esc, tree};
    let mut v = 0u64;
    let mut buf = [0u8; 128];

    // The name the set designates. Everything below hangs off this, so its failure is the control.
    let own = dir_open(fs::ROOT, tree::INNER);
    if own < 0 {
        v |= esc::GRANTED_ACCESS_FAILED;
    } else {
        v |= esc::OPENED_ITS_OWN;

        // A listing: a read, so it needs what reading the file needs and nothing more. Zero
        // attributes is a successful listing, which is what tells "forwarded and there were none"
        // apart from "the caretaker refused it".
        let (l, _) = call(FILE, fs::req(fs::LISTXATTR, own as u64, 0), 0);
        if (l as i64) < 0 {
            v |= esc::GRANTED_ACCESS_FAILED;
        } else {
            v |= esc::READ_ATTRS;
        }

        // A set, read straight back with its type code, then removed. The attribute's name is not a
        // name in the directory, so a caretaker that filtered it against the set would refuse this
        // and the bit would stay clear.
        put_page(fixture::attrs::NAME);
        put_page_at(fixture::attrs::NAME.len(), fixture::attrs::VALUE);
        let (s, _) = call(
            FILE,
            fs::req(fs::SETXATTR, own as u64, fixture::attrs::NAME.len() as u64),
            xattr::spec(fixture::attrs::KIND, fixture::attrs::VALUE.len() as u64),
        );
        if (s as i64) >= 0 {
            put_page(fixture::attrs::NAME);
            let (g, _) = call(
                FILE,
                fs::req(fs::GETXATTR, own as u64, fixture::attrs::NAME.len() as u64),
                0,
            );
            get_page(xattr::reply_value_len(g as i64), &mut buf);
            if (g as i64) >= 0
                && xattr::reply_kind(g as i64) == fixture::attrs::KIND
                && xattr::reply_value_len(g as i64) == fixture::attrs::VALUE.len()
                && buf[..fixture::attrs::VALUE.len()] == *fixture::attrs::VALUE
            {
                v |= esc::SET_AN_ATTR;
            } else {
                v |= esc::GRANTED_ACCESS_FAILED;
            }
            put_page(fixture::attrs::NAME);
            let (rm, _) = call(
                FILE,
                fs::req(
                    fs::REMOVEXATTR,
                    own as u64,
                    fixture::attrs::NAME.len() as u64,
                ),
                0,
            );
            if (rm as i64) < 0 {
                v |= esc::GRANTED_ACCESS_FAILED;
            }
        } else {
            v |= esc::GRANTED_ACCESS_FAILED;
        }
    }

    // The naming question, by both routes: a file the set does not carry, and a directory it does
    // not carry. Both are really in the granted directory and the caretaker one hop up could open
    // either on any request it liked, so each refusal is a fact about the set.
    if dir_open(fs::ROOT, tree::DEEPER) >= 0 || dir_descend(fs::ROOT, tree::DEEPER, dir::READ) >= 0
    {
        v |= esc::REACHED_AN_UNMATCHED_NAME;
    }

    send(REPORT, fixture::VERDICT, v, 0);
    exit();
}

/// A name with the run index appended, so three attacker runs sharing one image do not collide on
/// `EEXIST` and read it as a refusal. Fixed-size because this program has no allocator.
fn run_name(base: &str, run: u64) -> ([u8; 16], usize) {
    let mut out = [0u8; 16];
    let n = base.len().min(15);
    out[..n].copy_from_slice(&base.as_bytes()[..n]);
    out[n] = b'0' + (run % 10) as u8;
    (out, n + 1)
}

/// The `&str` inside a [`run_name`]. Its bytes are ASCII by construction, so the conversion cannot
/// fail; `unwrap_or` keeps the panic-free discipline anyway.
fn made_str(name: &([u8; 16], usize)) -> &str {
    core::str::from_utf8(&name.0[..name.1]).unwrap_or("made")
}

/// `OPEN` a name under a directory handle; the raw reply, so a refusal is a value rather than a trap.
fn dir_open(parent: u64, name: &str) -> i64 {
    put_page(name.as_bytes());
    call(FILE, fs::req(fs::OPEN, parent, name.len() as u64), 0).0 as i64
}

/// `CREATE` a name under a directory handle.
fn dir_create(parent: u64, name: &str) -> i64 {
    put_page(name.as_bytes());
    call(FILE, fs::req(fs::CREATE, parent, name.len() as u64), 0).0 as i64
}

/// `OPENDIR`: descend, asking for `rights`. The requested rights ride in the second word.
fn dir_descend(parent: u64, name: &str, rights: u64) -> i64 {
    put_page(name.as_bytes());
    call(
        FILE,
        fs::req(fs::OPENDIR, parent, name.len() as u64),
        rights,
    )
    .0 as i64
}

/// `MKDIR`: the same shape as [`dir_descend`], with creation.
fn dir_mkdir(parent: u64, name: &str, rights: u64) -> i64 {
    put_page(name.as_bytes());
    call(FILE, fs::req(fs::MKDIR, parent, name.len() as u64), rights).0 as i64
}

/// `UNLINK` a name under a directory handle; the raw reply, so a name that was not there is a value
/// rather than a trap. The witness uses it to clean up after itself and to provoke a node's death.
fn dir_unlink(parent: u64, name: &str) -> i64 {
    put_page(name.as_bytes());
    call(FILE, fs::req(fs::UNLINK, parent, name.len() as u64), 0).0 as i64
}

/// `RENAME`: two directories and two names, so both names go into the page back to back (source
/// first) and the destination's handle and length ride in the second word.
fn dir_rename(src_dir: u64, src: &str, dst_dir: u64, dst: &str) -> i64 {
    put_page(src.as_bytes());
    put_page_at(src.len(), dst.as_bytes());
    call(
        FILE,
        fs::req(fs::RENAME, src_dir, src.len() as u64),
        fs::rename_dst(dst_dir, dst.len() as u64),
    )
    .0 as i64
}

// --- The extended-attribute witness (milestone 57) ---

/// Fill `len` bytes of the shared page at `off` with `b`. The oversize probe writes a value larger
/// than any local this program could afford, so it goes straight to the page.
fn fill_page(off: usize, len: usize, b: u8) {
    for i in 0..len {
        WINDOW.w8((off + i) as u64, b);
    }
}

/// `SETXATTR`: the only verb here with two payloads, so the name and the value go into the page back
/// to back and the value's length and type code ride packed in the second word.
fn xattr_set(handle: u64, name: &[u8], kind: u32, value: &[u8]) -> i64 {
    put_page(name);
    put_page_at(name.len(), value);
    call(
        FILE,
        fs::req(fs::SETXATTR, handle, name.len() as u64),
        xattr::spec(kind, value.len() as u64),
    )
    .0 as i64
}

/// `GETXATTR`: the name goes out on the page and the value comes back on it, so the reply's packed
/// kind and length are all this returns; the bytes are read with [`get_page`].
fn xattr_get(handle: u64, name: &[u8]) -> i64 {
    put_page(name);
    call(FILE, fs::req(fs::GETXATTR, handle, name.len() as u64), 0).0 as i64
}

/// `LISTXATTR`: no name, no cursor. The reply is how many bytes of records landed in the page.
fn xattr_list(handle: u64) -> i64 {
    call(FILE, fs::req(fs::LISTXATTR, handle, 0), 0).0 as i64
}

/// `REMOVEXATTR`: [`xattr_get`]'s shape, and the reply is 0 or an error.
fn xattr_remove(handle: u64, name: &[u8]) -> i64 {
    put_page(name);
    call(FILE, fs::req(fs::REMOVEXATTR, handle, name.len() as u64), 0).0 as i64
}

/// **The extended-attribute witness** (milestone 57): everything the layer claims, attempted against
/// the real FS server over the real contract, reported as a bitmap the kernel test asserts exactly.
///
/// It is a bitmap rather than a chain of `check`s for the reason every other witness in this file is
/// one: a trapped client tells the waiting test only that something went wrong, and the interesting
/// failure here is *which* claim broke. A layer that dropped type codes, a store keyed on a path
/// instead of a node, and a purge that never ran each fail a different bit.
///
/// It cleans up after itself. Its attribute is removed from `scratch` and the two files it makes are
/// unlinked, so the image it leaves behind differs from the one it found by the store directory
/// alone, which the host tool is expected to show (notes/host-recovery.md).
fn attr_witness(scratch: u64) -> u64 {
    use fixture::attrs;
    let mut v = 0u64;
    let mut buf = [0u8; 512];

    // 1. Set an attribute and read it back: its bytes AND its type code. The kind is the half that
    //    catches a layer which stored the value and threw the type away, which is the thing that
    //    would foreclose indexing later without anyone noticing.
    if xattr_set(scratch, attrs::NAME, attrs::KIND, attrs::VALUE) >= 0 {
        let r = xattr_get(scratch, attrs::NAME);
        if r >= 0
            && xattr::reply_kind(r) == attrs::KIND
            && xattr::reply_value_len(r) == attrs::VALUE.len()
        {
            get_page(attrs::VALUE.len(), &mut buf);
            if &buf[..attrs::VALUE.len()] == attrs::VALUE {
                v |= attrs::SET_AND_READ_BACK;
            }
        }
    } else {
        v |= attrs::ATTRS_FAILED;
    }

    // 2. The listing names it, and names nothing else. A store keyed on the wrong node would show up
    //    here as somebody else's attributes rather than as a missing one.
    let n = xattr_list(scratch);
    if n > 0 {
        get_page(n as usize, &mut buf);
        let mut count = 0;
        let mut saw = false;
        for name in xattr::list::iter(&buf[..(n as usize).min(buf.len())]) {
            count += 1;
            if name == attrs::NAME {
                saw = true;
            }
        }
        if saw && count == 1 {
            v |= attrs::LISTED;
        }
    }

    // 3. A value past the ceiling is refused with its own word, not stored short. The probe is wider
    //    than any local here, so it is written straight onto the page.
    let big = b"user.toobig";
    put_page(big);
    fill_page(big.len(), xattr::MAX_VALUE + 1, b'Z');
    let r = call(
        FILE,
        fs::req(fs::SETXATTR, scratch, big.len() as u64),
        xattr::spec(xattr::RAW, (xattr::MAX_VALUE + 1) as u64),
    )
    .0 as i64;
    if filesystem_protocol::reply_errno(r) == Some(xattr::E2BIG) {
        v |= attrs::OVERSIZE_REFUSED;
    }

    // 4. Removing it takes it away, and reading it afterwards is ENODATA. Without this the round
    //    trip above is equally true of a store that never forgets anything.
    if xattr_remove(scratch, attrs::NAME) >= 0
        && filesystem_protocol::reply_errno(xattr_get(scratch, attrs::NAME)) == Some(xattr::ENODATA)
    {
        v |= attrs::GONE_AFTER_REMOVE;
    }

    // 5. **The headline: an attribute follows its file across a rename.** The two names are unlinked
    //    first, because `NIFE_KEEP_REDOXFS` makes a second boot meet the first boot's leftovers
    //    and an `EEXIST` here would read as a broken layer.
    let _ = dir_unlink(fs::ROOT, attrs::PROBE);
    let _ = dir_unlink(fs::ROOT, attrs::PROBE_MOVED);
    let probe = dir_create(fs::ROOT, attrs::PROBE);
    if probe >= 0
        && xattr_set(probe as u64, attrs::NAME, attrs::KIND, attrs::VALUE) >= 0
        && dir_rename(fs::ROOT, attrs::PROBE, fs::ROOT, attrs::PROBE_MOVED) >= 0
    {
        // Opened at its NEW name, through a handle minted after the rename, so nothing carried over
        // from the handle that set it.
        let moved = dir_open(fs::ROOT, attrs::PROBE_MOVED);
        let r = if moved >= 0 {
            xattr_get(moved as u64, attrs::NAME)
        } else {
            -1
        };
        if r >= 0 && xattr::reply_value_len(r) == attrs::VALUE.len() {
            get_page(attrs::VALUE.len(), &mut buf);
            if &buf[..attrs::VALUE.len()] == attrs::VALUE {
                v |= attrs::SURVIVED_RENAME;
            }
        }
    } else {
        v |= attrs::ATTRS_FAILED;
    }

    // 6. And the file's death takes its attributes with it: unlink the name, make it again, and the
    //    new file has nothing on it. The node id is very likely reused, which is the case that makes
    //    a leaked blob a correctness bug rather than wasted space.
    if dir_unlink(fs::ROOT, attrs::PROBE_MOVED) >= 0 {
        let fresh = dir_create(fs::ROOT, attrs::PROBE_MOVED);
        if fresh >= 0 && xattr_list(fresh as u64) == 0 {
            v |= attrs::GONE_AFTER_UNLINK;
        }
        let _ = dir_unlink(fs::ROOT, attrs::PROBE_MOVED);
    }

    // 7. The store is not in the namespace: it cannot be opened, created, or descended into.
    if dir_open(fs::ROOT, xattr::STORE_DIR) < 0
        && dir_create(fs::ROOT, xattr::STORE_DIR) < 0
        && dir_descend(fs::ROOT, xattr::STORE_DIR, dir::ALL) < 0
    {
        v |= attrs::STORE_UNNAMEABLE;
    }

    // 8. And a full enumeration does not name it. Driven to the end with a cursor, because a listing
    //    that stopped early would be a clean report about the part it happened to read.
    let (mut cursor, mut entries, mut clean, mut finished) = (0u64, 0u64, true, false);
    for _ in 0..64 {
        let n = call(FILE, fs::req(fs::READDIR, fs::ROOT, 0), cursor).0 as i64;
        if n <= 0 {
            finished = n == 0;
            break;
        }
        get_page(n as usize, &mut buf);
        let mut seen = 0u64;
        for (name, _) in filesystem_protocol::dirent::iter(&buf[..(n as usize).min(buf.len())]) {
            if name == xattr::STORE_DIR.as_bytes() {
                clean = false;
            }
            seen += 1;
        }
        if seen == 0 {
            break; // a cursor that cannot advance is a failure, not a clean listing
        }
        cursor += seen;
        entries += seen;
    }
    if clean && finished && entries > 0 {
        v |= attrs::STORE_UNLISTED;
    }

    v
}

/// How many bytes the attacker's write probe carries. A fixed length rather than the file's, because
/// the file's length is history this program did not write (see the control's note).
const PROBE_LEN: usize = 32;

/// A recognisable payload for the write probe: if a read-only grant ever let one through, the bytes
/// left on the disk say who put them there.
fn probe_payload() -> [u8; PROBE_LEN] {
    let mut p = [b'!'; PROBE_LEN];
    p[..16].copy_from_slice(b"CLOBBERED-BY-ATK");
    p
}

/// **The FS-server read benchmark** (the userspace-server tax, DECISIONS §32). Open `motd` once, then
/// time a loop of reads of its first block and report `[ticks, iters]` on the report endpoint, the
/// `os_primitives_benchmarker` shape. The kernel's bench boot prints it as `fs_read`; against the bare `ipc_rtt_el0`
/// round trip, the difference is what the FS-server contract costs above a raw endpoint call. It is a
/// `--real`-only magnitude (the mount is device-driven, not deterministic under icount); see
/// notes/benchmarks.md and kernel/src/bench.rs.
fn bench() -> ! {
    let handle = open(fixture::MOTD_NAME);
    let block = fixture::MOTD.len();
    for _ in 0..BENCH_WARMUP {
        let _ = read(handle, 0, block);
    }
    let start = now();
    for _ in 0..BENCH_ITERS {
        let _ = read(handle, 0, block);
    }
    let ticks = now().wrapping_sub(start);
    // slot 1 is REPORT; carry ticks and the iteration count, the bench line's two fields.
    send(REPORT, ticks, BENCH_ITERS, 0);
    exit();
}

/// **The filesystem throughput benchmark** (milestone 38, DECISIONS §34 condition 2). Six phases
/// over one file, each self-timed and reported home as `[ticks, transfers, phase]`: sequential
/// write, sequential read, random read, a record-aligned read, random write, and the client's own
/// payload cost. The bench boot turns those into the `fs_seq_write` / `fs_seq_read` /
/// `fs_rand_read` / `fs_record_read` / `fs_rand_write` / `fs_payload_fill` lines and into MiB/s.
///
/// **What one transfer is, and why that is the whole story.** One `filesystem_protocol` request moves at most
/// `fs::TRANSFER_MAX` bytes, because the payload travels through the region the client shares with
/// the server. That was one page until milestone 138 step 3 and is sixteen now, so a phase here is
/// `UNIT` per request and the throughput is the request rate times `UNIT`. A Linux program reading
/// the same file would use a 64 KiB or 1 MiB buffer and pay one syscall for it; **that difference
/// is a property of this protocol, not of the measurement**, and it is still the first thing
/// notes/benchmarks.md says next to the numbers. It got smaller; it did not go away.
///
/// **The bytes moved per phase are held constant** (`tp::TOTAL`, 1 MiB) rather than the transfer
/// count, so a run at one page and a run at sixteen move the same file and their MiB/s figures are
/// a before and an after rather than two unrelated numbers.
///
/// **Why the write phase comes first.** It creates the file, so it is the only phase that pays
/// allocation, and the read phases have something laid out by the server under test rather than by
/// the host tool. It is also the reason there is no separate "create" number.
///
/// **What a write here promises, which decides what it compares against.** Every `filesystem_protocol` write
/// goes through one RedoxFS transaction that commits to the header ring before the reply, so the
/// filesystem's own state is durable per request the way `O_DSYNC` makes ext4's; but no device
/// flush is issued unless a client asks for one (`filesystem_protocol::fs::SYNC`), so the bytes sit where
/// `O_DIRECT` alone leaves them. This sits **between** Linux's two, and notes/benchmarks.md prints
/// both rather than picking one.
///
/// **The offsets are drawn from a fixed seed**, so "random" means the same sequence on every run
/// and two runs are comparable. They are page-aligned and inside the file, so a random read never
/// hits EOF and a random write never extends the file: the random phases measure placement, not
/// growth.
///
/// A read phase's timed loop issues nothing but the `CALL`: the bytes are never copied out, because
/// a client that shares a page with its server can use them where they land, which is one copy
/// fewer than a Unix `read(2)`. A write phase's loop repaints the page first, and has to; see the
/// comment on the payload below.
fn throughput() -> ! {
    use fixture::throughput as tp;

    let handle = throughput_file();

    // **Every write gets a fresh, incompressible page, and that is not fussiness.** RedoxFS stores
    // a file in records (8 KiB since milestone 138 step 1, 128 KiB before it), compresses each with
    // lz4 when it is larger than one block, and skips a write outright when the record it would
    // produce is byte-identical to the one already there. So a benchmark that sends one constant
    // page measures the short circuit on every repeat, and one that sends several copies of an
    // incompressible page into a single record measures lz4 finding the copies. The first draft
    // of this bench did both, and reported random writes as fast as random reads because none of
    // them wrote anything. See notes/benchmarks.md.
    let mut payload = 0x9E37_79B9_7F4A_7C15_u64;

    // What that costs, measured on its own so a reader can take it back out of the write phases,
    // which pay it inside their window because a client that writes data has to produce the data.
    let start = now();
    for _ in 0..tp::BLOCKS {
        paint_page(&mut payload);
    }
    let ticks = now().wrapping_sub(start);
    send(REPORT, ticks, tp::BLOCKS, tp::PAYLOAD_FILL);

    // --- Phase 1: sequential write. This is also the file's creation. ---
    // **No warmup here, on purpose.** Every other phase warms up to keep a cold first touch out of
    // the window; this phase's whole subject is the cold path, because writing a file that did not
    // exist is what allocates its blocks. Warming it would mean overwriting the first few blocks
    // before timing them, which measures a different operation than the rest of the loop.
    let start = now();
    for k in 0..tp::BLOCKS {
        paint_page(&mut payload);
        write_unit(handle, k * tp::UNIT as u64);
    }
    let ticks = now().wrapping_sub(start);
    // The file is exactly as long as the phase claims to have written it. Without this a server
    // that answered every write with a short count would still produce a throughput number.
    let (size, _) = call(FILE, fs::req(fs::FSTAT, handle, 0), 0);
    check(size == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::SEQ_WRITE);

    // --- Phase 2: sequential read of what phase 1 left behind. ---
    for _ in 0..tp::WARMUP {
        check(read(handle, 0, tp::UNIT) == tp::UNIT);
    }
    let start = now();
    let mut got = 0u64;
    for k in 0..tp::BLOCKS {
        got += read(handle, k * tp::UNIT as u64, tp::UNIT) as u64;
    }
    let ticks = now().wrapping_sub(start);
    check(got == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::SEQ_READ);

    // --- Phase 3: random read, page-aligned, inside the file. ---
    let mut rng = 0x38F5_7042_1DEA_u64;
    for _ in 0..tp::WARMUP {
        check(read(handle, next_offset(&mut rng), tp::UNIT) == tp::UNIT);
    }
    let start = now();
    let mut got = 0u64;
    for _ in 0..tp::BLOCKS {
        got += read(handle, next_offset(&mut rng), tp::UNIT) as u64;
    }
    let ticks = now().wrapping_sub(start);
    check(got == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::RAND_READ);

    // --- Phase 5, out of order on purpose: the cheapest 4 KiB read there is. ---
    // Record-aligned, so `read_record` fetches one block instead of averaging over a record's
    // worth. It runs before the second write phase so that it reads a file no write has touched
    // since the read phases above, and its gap to the sequential read is the record geometry with
    // everything else held identical. See `fixture::throughput::RECORD_READ`.
    let records = (tp::BLOCKS * tp::UNIT as u64) / tp::RECORD;
    for k in 0..tp::WARMUP {
        check(read(handle, (k % records) * tp::RECORD, tp::UNIT) == tp::UNIT);
    }
    let start = now();
    let mut got = 0u64;
    for k in 0..tp::BLOCKS {
        got += read(handle, (k % records) * tp::RECORD, tp::UNIT) as u64;
    }
    let ticks = now().wrapping_sub(start);
    check(got == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::RECORD_READ);

    // --- Phase 4: random write, in place, over blocks that already exist. The offset stream runs
    // on from phase 3 rather than restarting, so no timed write lands on a block the warmup just
    // touched. ---
    for _ in 0..tp::WARMUP {
        paint_page(&mut payload);
        write_unit(handle, next_offset(&mut rng));
    }
    let start = now();
    for _ in 0..tp::BLOCKS {
        paint_page(&mut payload);
        write_unit(handle, next_offset(&mut rng));
    }
    let ticks = now().wrapping_sub(start);
    // The file did not grow: a random write that appended would be measuring allocation.
    let (size, _) = call(FILE, fs::req(fs::FSTAT, handle, 0), 0);
    check(size == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::RAND_WRITE);

    exit();
}

/// Open the throughput file, creating it if this is the first run against this image and emptying
/// it if it is not. `CREATE` answers `EEXIST` rather than opening (`filesystem_protocol`'s note says why), so
/// the fallback is an explicit open-and-truncate.
fn throughput_file() -> u64 {
    let name = fixture::THROUGHPUT_NAME;
    put_page(name.as_bytes());
    let (r0, _) = call(FILE, fs::req(fs::CREATE, 0, name.len() as u64), 0);
    if (r0 as i64) >= 0 {
        return r0;
    }
    let h = open(name);
    let (t, _) = call(FILE, fs::req(fs::TRUNCATE, h, 0), 0);
    check((t as i64) >= 0);
    h
}

/// One timed write: a page of the shared page's current contents at `offset`. No `put_page`, so the
/// measured window holds the request and not a 4 KiB byte-at-a-time fill.
fn write_unit(handle: u64, offset: u64) {
    let unit = fixture::throughput::UNIT;
    let (r0, _) = call(FILE, fs::req(fs::WRITE, handle, unit as u64), offset);
    if (r0 as i64) < 0 {
        fail(STAGE_WRITE, r0);
    }
    check(r0 as usize == unit);
}

/// **Fill the whole shared page with fresh, incompressible bytes**, advancing `state`. Eight bytes
/// at a time, so it is 512 stores rather than 4096; the cost is reported as its own row
/// (`fs_payload_fill`) because the write phases pay it inside their timed window.
fn paint_page(state: &mut u64) {
    let mut i = 0;
    while i < fixture::throughput::UNIT {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *state = x;
        WINDOW.write::<u64>(i as u64, x);
        i += 8;
    }
}

/// The next page-aligned offset inside the throughput file, from a xorshift64 the seed makes
/// reproducible. A benchmark whose access pattern changes between runs cannot be compared with
/// itself, let alone with another system.
fn next_offset(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x % fixture::throughput::BLOCKS) * fixture::throughput::UNIT as u64
}

fn proof() -> ! {
    // Read the motd the image ships with, through a handle the server minted for us. This proves
    // the whole read path end to end: a real RedoxFS image we did not write, mounted by a confined
    // FS server over blk IPC, its files opened by name under a granted directory capability and read
    // by a client that names nothing else in the system.
    //
    let motd = open(fixture::MOTD_NAME);
    let n = read(motd, 0, fixture::MOTD.len());
    check(n == fixture::MOTD.len());
    let mut buf = [0u8; 128];
    get_page(n, &mut buf);
    check(&buf[..n] == fixture::MOTD);
    let mut head = [0u8; 8];
    head.copy_from_slice(&buf[..8]);

    // **Repeat writes, in one run.** A first write to a pristine block always worked; a write to a
    // block the image already carries is the case that loops, and the gate could not see it because
    // `mkredoxfs` rewrites the target to a placeholder before every run, making every gated write a
    // first write. Writing the same block three times here depends on nothing left over from a
    // previous invocation, so the bug cannot hide behind the harness again.
    let scratch = open(fixture::SCRATCH_NAME);
    let mut pass = 1u8;
    while pass <= 3 {
        let payload = repeat_payload(pass);
        write(scratch, 0, &payload);
        let m = read(scratch, 0, payload.len());
        check(m == payload.len());
        get_page(m, &mut buf);
        check(buf[..m] == payload[..]);
        pass += 1;
    }

    // Leave the fixture pattern behind as the LAST write, so the gate's post-run host-tool check
    // (`redoxfs_check_after_run`, which reopens the image with the pinned engine and compares
    // `scratch` against `WRITE_PATTERN`) still validates a documented value. Every write above is the
    // same length as this one, so this replaces them completely.
    write(scratch, 0, fixture::WRITE_PATTERN);
    let f = read(scratch, 0, fixture::WRITE_PATTERN.len());
    check(f == fixture::WRITE_PATTERN.len());
    get_page(f, &mut buf);
    check(&buf[..f] == fixture::WRITE_PATTERN);

    // **Extended attributes** (milestone 57), on the same server over the same endpoint, so the
    // whole three-process stack does not have to be booted a second time to prove a fourth verb
    // family. It runs after the fixture restore above because attributes do not touch a file's
    // contents, so the gate's post-run byte comparison is unaffected either way and this ordering
    // keeps the restore the last thing that writes bytes.
    let attrs = attr_witness(scratch);

    // Report the motd head, the success sentinel, and the attribute witness's bitmap; the kernel
    // asserts all three, and the third against an exact expected set.
    send(REPORT, u64::from_le_bytes(head), fixture::SUCCESS, attrs);
    exit();
}

user_mode_runtime::panic_handler!();
