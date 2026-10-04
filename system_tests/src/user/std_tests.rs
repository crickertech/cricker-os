use super::*;

/// Reassemble a std program's stdout off its endpoint until the writer says the stream is over,
/// and compare byte for byte.
///
/// The framing is the sink contract's (`crates/byte_sink_protocol`, milestone 50), which for bytes is
/// bit-for-bit the framing the PAL used before that contract existed: `w0` is the count, `w1`
/// and `w2` are the bytes, little-endian. `SEND` blocks until a receiver takes it, so the
/// program is somewhere between its last `println!` and `SYS_EXIT` when the bytes land.
///
/// **It reads to end of stream rather than to a byte count, and that is a strengthening.**
/// Stopping at `want.len()` proved the right bytes came out and said nothing about whether more
/// were coming; draining to `OPERATION_EOF` proves the program printed exactly this and then finished,
/// and it exercises the end-of-stream announcement std's `cleanup` now makes, which is what a
/// pipe's reader will depend on.
///
/// Shared by every std test on both ISAs (the arch-gated test modules reach it here), so all of
/// them assert the same way and a drift in one is a diff in one place.
pub(super) fn assert_std_transcript(report: crate::sched::RendezvousId, want: &[u8], what: &str) {
    let mut got = [0u8; 768];
    let len = drain_sink(report, &mut got, what);
    assert_eq!(
        &got[..len],
        want,
        "{what}: stdout did not match the transcript"
    );
}

/// Reassemble a sink-contract stream into `out` until end of stream; returns its length.
///
/// The one decoder for every sink in the suite, on purpose. The indifference test's whole claim
/// is that two destinations produce the same bytes, and it would be a much weaker claim if each
/// arm were decoded by its own code.
/// # BUGS
///
/// **A program that aborts is never heard, and this hid a bug for a day.** Found by
/// milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets).
/// This loop ends only on the sink's end-of-stream marker, which the runtime's `cleanup` sends after `main` returns. A program that
/// panics under `panic = "abort"` traps instead, so every byte it printed, **including the panic
/// message**, is sent to this endpoint and never read: the caller blocks until the watchdog fires
/// and the transcript reads as empty. That is indistinguishable from a program that never started,
/// and it was read as exactly that.
///
/// The workaround a program can apply itself is a panic hook that prints and then calls
/// `process::exit`, which is what `cryptography_exerciser` does. A fix here would need a
/// non-blocking receive or a pending count by rendezvous id, neither of which the scheduler
/// exposes today.
pub(super) fn drain_sink(ep: crate::sched::RendezvousId, out: &mut [u8], what: &str) -> usize {
    let mut len = 0usize;
    loop {
        let words = crate::sched::ipc_receive(ep);
        let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(words[0], words[1], words[2], &mut chunk) {
            byte_sink_protocol::Msg::Bytes(n) => {
                for &b in &chunk[..n] {
                    assert!(len < out.len(), "{what}: wrote more than the buffer holds");
                    out[len] = b;
                    len += 1;
                }
            }
            byte_sink_protocol::Msg::Eof => return len,
            byte_sink_protocol::Msg::Malformed => {
                panic!(
                    "{what}: a sink message the contract does not define: {:#x}",
                    words[0]
                )
            }
        }
    }
}

/// Consume the FS service's two readiness sentinels, if this caller is the one that wired it.
///
/// One boot has one FS service (the block server owns the device), so the hand-written client's
/// test and the `std::fs` test share it, and only the first of them to run gets the sentinels.
/// Asserting on them where they exist is what separates a hang in the mount from one in the
/// serve path.
/// **Kill the filesystem mid-transaction on a real device, then mount what is left of it**
/// (milestone 37, DECISIONS §34 condition 1). Shared by both ISA test modules, so the property
/// and its wording are asserted identically on each (rule 5, §19).
///
/// Six steps, each of which has to happen for the next one to mean anything:
///
/// 1. a block server brings up the crash test's own disk (nothing else in the boot touches it);
/// 2. an FS server mounts it, so the image was sound before this test damaged it;
/// 3. the driver writes payload A, reads it straight back, and reports: **acknowledged**;
/// 4. the driver's second write walks into the injector, which tears a block at 2048 bytes and
///    traps with the transaction's commit unwritten. The `CUT` word is how we know the kill was
///    the injector's rather than something else having gone wrong;
/// 5. a **different FS-server process** mounts the same disk through the same block server. Its
///    readiness sentinel is the consistency result: `Server::open` refuses an image it cannot
///    make sense of, so arriving at all means the filesystem is intact;
/// 6. a fresh client reads the file and says which payload is in it.
///
/// **The assertion is the property, not an outcome.** Either payload is a pass; what fails is a
/// mixture, a length nobody wrote, or the pre-boot contents (which would mean an acknowledged
/// write had vanished). Pinning the answer to "A" would be pinning a detail of when RedoxFS
/// happens to write its commit, and the claim is not about that.
pub(super) fn assert_a_kill_mid_transaction_recovers(
    blk_image: &'static [u8],
    redoxfs_server_image: &'static [u8],
    client_image: &'static [u8],
) {
    use filesystem_protocol::fixture::{READY, SUCCESS, crash};
    // The caller has already established the disk is there (`fs_service::is_crash_disk_present`)
    // and skipped its own `#[test_case]` if it is not. This `expect` is that guard restated where
    // it can fail loudly: a `None` here means the disk went away between the two calls, which is a
    // machine fault rather than an absent fixture, and it must not be reported as a skip.
    let run = fs_service::start_crash(blk_image, redoxfs_server_image, client_image)
        .expect("the crash disk was present a moment ago and the block server still refused it");
    assert_eq!(
        crate::sched::ipc_receive(run.blk_ready)[0],
        READY,
        "the block server did not bring the crash-test disk up",
    );
    assert_eq!(
        crate::sched::ipc_receive(run.fs_ready)[0],
        READY,
        "the FS server did not open the crash-test image, so there was nothing to crash",
    );

    let [w0, w1, ..] = crate::sched::ipc_receive(run.driver_report);
    assert_eq!(
        (w0, w1),
        (SUCCESS, crash::SAW_A),
        "the driver's first write was not acknowledged and read back, so nothing that follows \
         is a statement about an acknowledged write",
    );

    assert_eq!(
        crate::sched::ipc_receive(run.fs_ready)[0],
        crash::CUT,
        "the FS server did not die inside the injector: whatever killed it, it was not this \
         test, and the recovery below would be measuring the wrong thing",
    );

    let (ready, report) = fs_service::recover_crash(redoxfs_server_image, client_image);
    assert_eq!(
        crate::sched::ipc_receive(ready)[0],
        READY,
        "the recovery mount FAILED: a fresh FS server could not open the image the killed one \
         left behind, so a torn write mid-transaction cost the whole filesystem",
    );

    let [len, saw, ..] = crate::sched::ipc_receive(report);
    assert!(
        saw == crash::SAW_A || saw == crash::SAW_B,
        "after a kill mid-transaction the file held {len} bytes that were neither payload \
         whole: a write is either wholly present or wholly absent, and this was neither",
    );
    crate::println!(
        "    (crash recovery: the file holds payload {}, {len} bytes, whole)",
        if saw == crash::SAW_A { "A" } else { "B" },
    );
}

/// **The extended-attribute witness's verdict** (milestone 57), asserted as an exact set so that
/// a client which could do nothing and one which could do everything both fail.
///
/// One copy, here beside the other shared FS assertions, because the two ISA legs assert the
/// same thing and the useful part is naming *which* claim broke rather than printing a number
/// the reader has to decode. Each missing bit is one sentence about the layer.
pub(super) fn assert_attrs(attrs: u64) {
    use filesystem_protocol::fixture::attrs as a;
    if attrs == a::EXPECTED {
        return;
    }
    for (bit, what) in [
        (
            a::SET_AND_READ_BACK,
            "an attribute set and read back with its type code",
        ),
        (a::LISTED, "the listing named it and nothing else"),
        (a::SURVIVED_RENAME, "it followed its file across a rename"),
        (a::GONE_AFTER_REMOVE, "removing it made it ENODATA"),
        (
            a::GONE_AFTER_UNLINK,
            "a file remade at the same name inherited nothing",
        ),
        (
            a::OVERSIZE_REFUSED,
            "an over-long value was refused with E2BIG",
        ),
        (
            a::STORE_UNNAMEABLE,
            "the attribute store could not be named",
        ),
        (a::STORE_UNLISTED, "and did not appear in an enumeration"),
    ] {
        if attrs & bit == 0 {
            crate::println!("    MISSING: {what}");
        }
    }
    if attrs & a::ATTRS_FAILED != 0 {
        crate::println!("    the witness reported that something it should do failed");
    }
    panic!(
        "the extended-attribute witness reported {attrs:#x}, expected {:#x}",
        a::EXPECTED,
    );
}

pub(super) fn assert_fs_service_ready(readiness: fs_service::Readiness) {
    // One copy, in `fs_service`, because draining these is **sequencing** and not only an
    // assertion: each server is parked inside its own blocking announcement until somebody
    // receives it, so nothing it serves can be answered first. The caretakers depend on that.
    fs_service::wait_for_service(readiness);
}

/// Build the exact bytes `std_exerciser` prints when it is granted a directory capability, into
/// `buf`; returns the length. Not a `const` because the motd's contents are spliced in from the
/// shared fixture, and that is the load-bearing part: those bytes came off the RedoxFS image,
/// through the FS server, through `std::fs`, and out the stdout endpoint.
pub(super) fn std_fs_expected(buf: &mut [u8; 768]) -> usize {
    // The lengths spelled out below are the motd's; if the fixture changes, fail here rather
    // than in a byte comparison nobody can read.
    assert_eq!(
        filesystem_protocol::fixture::MOTD.len(),
        70,
        "the motd fixture changed; the expected transcript's lengths must change with it",
    );
    let mut n = 0;
    for part in [
        b"std fs on nife\n".as_slice(),
        filesystem_protocol::fixture::MOTD,
        b"read_to_string 70\nmetadata len 70\n".as_slice(),
        // Milestone 122 changed the third of these. `sub/motd` used to be refused for being
        // nested, which is no longer a category; what replaces it is `sub/../other/secret`, a real
        // file in a real sibling directory that this process CAN reach by naming `other/secret`,
        // refused through the `..` route. The refusal moved from "a path with two components" to
        // "there is no ascent", which is the property that was always the point.
        // Milestone 47's namespace half changed the first of these, on 2026-08-18. `/etc/passwd`
        // used to be refused for having a leading slash, and a leading slash now names this
        // process's own root, so the line that replaces it is a *positive*: `/motd` and `motd`
        // opened the same file, and `current_dir` named the root they share. The refusal that
        // carries the claim moved to `/../motd`, which asks for the level above the only root
        // there is.
        b"absolute is my root\n".as_slice(),
        b"absolute dotdot refused\ndotdot refused\ninner dotdot refused\n".as_slice(),
        b"missing not found\n".as_slice(),
        // Milestone 31 phase 2: `create unsupported` became `write create ok`, plus the two
        // refusals that prove CREATE did not widen what a client can reach.
        b"write create ok\ncreate_new refused\n".as_slice(),
        b"create refused absolute\ncreate refused dotdot\n".as_slice(),
        b"write readback ok\n".as_slice(),
        // Milestone 64: the namespace verbs. Every one of these was `Unsupported` in the PAL
        // while the FS server had been dispatching the verb behind it since milestones 47 and 48,
        // so these nine lines are a binding proven rather than a contract widened.
        b"mkdir ok\nread_dir ok\nread_dir descend ok\n".as_slice(),
        b"unlink refused a directory\nrmdir refused a file\n".as_slice(),
        b"rename ok\nunlink ok\nis_dir ok\nrmdir ok\n".as_slice(),
        // Milestone 64's second pass: `File::set_len` (`TRUNCATE` with a size, both directions) and
        // `fs::copy` (no verb, and none needed). Same shape of finding as the nine lines above, one
        // milestone later: the refusal outlived the reason for it.
        b"set_len ok\ncopy ok\n".as_slice(),
        // Milestone 64's last pass: file times, on the verbs milestone 47's `touch` added
        // (DECISIONS §112). The first line is the motd's host-stamped mtime landing in the
        // wall-clock window; the next three are a write moving a made file's mtime forward, a
        // set-by-name round trip in whole seconds, and a directory's mtime read both by name and
        // through a listing entry. The
        // last two are refusals: through an open handle (the contract asks by name), and an access
        // time, the granted directory itself and an empty set on a missing name.
        b"mtime from the image ok\nwrite moves mtime ok\nset_times ok\ndir mtime ok\n".as_slice(),
        b"handle mtime refused\nset_times refusals ok\n".as_slice(),
        // **Milestone 122: descent.** The first three lines are a nested path resolving, a second
        // descent below it, and the pair that was actually broken: list a subdirectory, then open
        // every file the listing named through the `path()` the listing handed back. That pair is
        // what every filesystem walker is built out of, and one level down it used to fail.
        b"descend read ok\ndescend twice ok\nwalk entry ok\n".as_slice(),
        // A path may only walk *through* directories, and a held directory is bound exactly as the
        // granted one is. `dir handle ok` is `std::fs::Dir`, std's own `openat`-shaped API, running
        // on the capability it already is: nife used to get std's generic fallback, which stores a
        // `canonicalize`d path and therefore failed at its first call.
        b"through a file refused\ndir handle ok\n".as_slice(),
        // `rename` across two directories is one atomic message, because `RENAME` carries both
        // handles and both are now walked. `remove_dir_all` needed no nife code at all: std's
        // generic recursion is written in terms of `read_dir` and `remove_file` on paths it
        // composes itself, so it started working the moment those paths resolved.
        b"rename across ok\nremove_dir_all ok\n".as_slice(),
        b"fs ok\n".as_slice(),
    ] {
        buf[n..n + part.len()].copy_from_slice(part);
        n += part.len();
    }
    n
}

/// The exact bytes `std_exerciser` prints, in order. `println!` is line-buffered and every line
/// ends in `\n`, so the whole transcript is flushed by the time the program exits. Pinned here
/// so a drift in std's behaviour, the PAL, or the demo is a loud diff rather than a mystery.
/// `os nife` proves `std::env::consts::OS` resolves through the patched `env_consts`; the
/// two `unsupported` lines prove `fs`/`net` refuse honestly rather than pretend.
pub(super) const EXPECTED: &[u8] = b"hello from std on nife\n\
    os nife\n\
    args []\n\
    vec sum 149985000\n\
    string len 500\n\
    map lookup 1369\n\
    fs honestly unsupported\n\
    net honestly unsupported\n\
    instant monotonic ok\n\
    wall clock ok\n\
    entropy ok\n\
    config seeded\n\
    env ok\n\
    paths ok\n\
    exiting through process::exit\n";

/// A whole Rust `std` program runs on the native ABI and its output is exactly right.
///
/// Granted a heap, a stdout endpoint, and a clock, so both `fs` and `net` refuse: the two
/// `unsupported` lines in the transcript are "no ambient filesystem" and "no ambient network"
/// felt from inside std, on a binary that also runs both for real when it is granted them.
///
/// The `wall clock ok` line is milestone 51's half: `SystemTime::now()` reached a real time,
/// through a read-only page and the ambient counter, and the program asserted it was inside the
/// same sanity window the clock service applies. Before that milestone the same call returned
/// 1970 plus uptime and would have passed any test that only checked it did not crash.
///
/// The `config seeded` line is milestone 47's environment-variable fork (DECISIONS §111): this
/// process is granted an inert-configuration page unconditionally
/// (`std_service::start_on`/`CONFIG_SLOT`/`CONFIG_PAGE_STD`), assembled once from
/// `environment_protocol::PageBuilder` before the program exists, and `pal::nife::init` reads `TZ`, `LANG`
/// and `TERM` off it into `std::env`'s table before `main` runs. The line proves the whole path:
/// the kernel assembled and validated the page, mapped it read-only, the std PAL's probe found
/// the granted capability, and the values it read back are the exact ones the kernel wrote. The
/// `env ok` line just below depends on it: `vars().count()` is asserted at 3 rather than 0
/// precisely because this line's three keys are already there.
///
/// The `env ok` line is milestone 64's, and it is a third correction of the same shape, found by a
/// different method. `std::env::vars()` used to **abort the process**: nife had no `sys::env`
/// backend, so it fell through to the unsupported one whose `env()` is a `panic!`. Nothing in a
/// build failure list showed it, because it compiled. The line asserts three things at once: that
/// the call returns, that the listing holds exactly what this process's grants seeded and nothing
/// more (three keys from the config page above, not zero: milestone 47 gave the count a nonzero
/// honest answer where it used to be an always-empty one), and that `set_var` takes, since that
/// is what `set_var` means on every other platform.
///
/// The `paths ok` line is the same finding a third time, and it is why this test grew rather than
/// a new one appearing. `std::env::temp_dir()`, `std::env::split_paths()` and `std::process::id()`
/// were all `panic!` in the fallbacks nife had no arm in, so each of them **killed the program**
/// while compiling perfectly; `tempfile::NamedTempFile::new()` reached the first of them before it
/// ever got to its own "operation not supported" arm, which is not what the measurement note said
/// it did. The line asserts the two that now answer (`temp_dir` is the granted directory and
/// `TMPDIR` steers it; a path list survives a join and a split) and, just as deliberately, the
/// three that still refuse (`current_dir`, `current_exe`, `home_dir`), so that a later lane cannot
/// quietly turn a refusal into a fabricated answer.
///
/// The `entropy ok` line is milestone 56's half, and it is the same shape of correction:
/// `std::random` reached a virtio-rng device, through one endpoint that names no device, and
/// the program asserted two draws differ. Before that milestone the same call returned
/// splitmix64 seeded from boot-relative time, which would also have passed any test that only
/// checked it did not crash.
///
/// The `exiting through process::exit` line, and the two assertions after the transcript, are
/// milestone 64's fourth pass and the same shape a fourth time. `std::process::exit` fell through
/// to `crate::intrinsics::abort()`, so a program ending the way almost every CLI-shaped `main`
/// ends executed a trap and was reported to its supervisor as a crash. It was invisible to the
/// three earlier passes for a reason worth keeping: `sys/exit.rs` is not a `sys/<module>/mod.rs`
/// backend, so "read every module the PAL falls through" does not reach it, and `_start` calls
/// the PAL's own exit directly, so the *usual* way a program ends never went near the broken one.
/// `cargo xtask std-aborts` is the mechanism that replaces that reading.
#[test_case]
fn a_whole_std_program_runs_on_the_native_abi() {
    if std_service::std_exerciser_image().is_none() {
        crate::testing::skip!(std_service::NO_STD_EXERCISER);
    }
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let image = program("std_exerciser").expect("no std_exerciser program in the initrd archive");
    let clock = program("clock").expect("no clock program in the initrd archive");
    let entropy = program("entropy").expect("no entropy program in the initrd archive");
    let faults_before = USER_FAULTS.load(Ordering::Relaxed);
    let (report, tid) = std_service::start(image, clock, entropy);
    assert_std_transcript(report, EXPECTED, "std_exerciser");

    // **The exit is part of the transcript's claim, and it was not being checked** (milestone 64,
    // fourth pass). The last line the program prints is the one before `std::process::exit(0)`,
    // whose `_ =>` arm in `sys/exit.rs` was `crate::intrinsics::abort()`: a `brk`, which the kernel
    // takes as a fault. So this program used to print a perfect transcript and then die as a crash,
    // and every byte above passed while it did.
    //
    // Waiting for the thread to be *gone* rather than for a timeout is what makes this cheap and
    // race-free. The transcript ends inside `rt::cleanup`, which runs before the process leaves, so
    // reading the last byte proves nothing about how it left; both a clean exit and a fault reap an
    // unsupervised thread, so the departure is the synchronisation and the fault counter is the
    // verdict.
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(tid)),
        "the std program never left: it is neither exited nor faulted",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults_before,
        "std::process::exit trapped instead of exiting: a clean exit reported as a crash",
    );
}
