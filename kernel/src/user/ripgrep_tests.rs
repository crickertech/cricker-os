//! **`ripgrep`, unmodified, from crates.io** (milestone 121; `design/fatal-risks.md` risk 1).
//!
//! Risk 1 is *"only software written for nife runs on nife"*, and it is the most dangerous of the
//! nine because it is structural: optimization cannot fix "nothing runs here". `ripgrep` is the
//! decisive experiment because it is not a toy. It has forty transitive crates, it walks a
//! filesystem, and it is written for a world with threads, a command line, and `mmap`.
//!
//! **The program here is not ours and is not patched.** `helpers/build-ripgrep.sh` downloads the
//! published `ripgrep` crate and builds it with a target spec and three link arguments; there is no
//! overlay, no vendored copy and no fork. Everything this test observes is therefore a fact about
//! the platform rather than about our port of a program.
//!
//! **It skips when the archive has no `rg`**, which is every ordinary build and all of CI, because
//! making the gate fetch a crates.io dependency tree is DECISIONS §46's decision and calef's rather
//! than a lane's. See notes/ripgrep-on-nife.md.
//!
//! **All three ISAs run it**, which is DECISIONS §19 rather than thoroughness: a capability ships on
//! every supported architecture or a scope note records the gap and the plan. `helpers/build-ripgrep.sh`
//! builds for `aarch64-unknown-nife`, `riscv64-unknown-nife` and (since milestone 184)
//! `x86_64-unknown-nife` in one pass, and one test body serves all three because nothing it asserts
//! is architecture-specific. **It runs on all three since milestone 303**, which gave `q35` a RedoxFS
//! image the FS service can find (the block lookup spans virtio-mmio and virtio-pci now), and the
//! x86_64 transcript is byte-identical to the other two.

use super::*;

/// The reason this test gives when nobody built `rg`.
const NO_RIPGREP: &str = "no rg in this archive: build it with helpers/build-ripgrep.sh, which \
                          fetches the published ripgrep crate from crates.io (milestone 121)";

/// **The block server's ELF**, one program in every archive since milestone 291. This was two
/// `cfg` arms (a role of `hello` on aarch64, the dedicated `block_driver` elsewhere) until that
/// milestone packed `block_driver` on aarch64 too; `fs_service::blk_server_image` carries the
/// reason.
fn block_server_image() -> &'static [u8] {
    program("block_driver").expect("no block_driver program in the initrd archive")
}

/// **Somebody else's forty-crate application loads, runs, reaches a real filesystem through a
/// capability it was handed, and then cannot be told what to search for.**
///
/// Every layer below the last clause works, and none of it was written for `ripgrep`. A multi-megabyte
/// ELF the loader maps (4.7 MB on aarch64, 10.7 MB on riscv64); a heap std grows one page at a time under `regex`'s and `ignore`'s allocation
/// patterns; `std::env::current_dir` answering `/` because this process holds a directory; output
/// through the one endpoint it was granted. `ripgrep` did all of that without a line of nife in it.
///
/// **What stops it is that the nife ABI has no argument vector.** `std::env::args()` resolves to
/// std's `sys/args/unsupported.rs`, which yields an empty iterator, because a program is entered
/// with three registers and a capability table (notes/abi.md) and there is nowhere for a command
/// line to live. So `ripgrep` parses zero arguments, finds no pattern, and prints its own usage
/// text. It is not refusing and it has not failed: it was never asked anything.
///
/// **It does not hit DECISIONS §105 at all**, which is the result this test was expected to
/// produce and did not. `ripgrep` asks `std::thread::available_parallelism()` how much parallelism
/// it has, nife's PAL answers `1` honestly, and `ripgrep` picks its own single-threaded walker and
/// searcher. `thread::spawn` is never reached. See notes/ripgrep-on-nife.md.
#[test_case]
fn unmodified_ripgrep_runs_and_has_no_arguments_to_run_on() {
    if program("rg").is_none() {
        crate::testing::skip!(NO_RIPGREP);
    }
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let image = program("rg").expect("no rg program in the initrd archive");
    let faults_before = USER_FAULTS.load(Ordering::Relaxed);
    let Some(rg) = fs_service::start_std_full(
        block_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        image,
    ) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    super::std_tests::assert_fs_service_ready(rg.readiness);

    let mut got = [0u8; 8192];
    let len = super::std_tests::drain_sink(rg.report, &mut got, "rg");
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    rg printed {len} bytes:\n{text}");

    // `ripgrep`'s own usage text, which is what it prints when it is given nothing. Asserting on
    // its words rather than on the whole block, because the block is a stranger's copy and pinning
    // it byte for byte would make a `ripgrep` release a failure here.
    assert!(
        text.contains("ripgrep"),
        "rg printed something that is not ripgrep's own output",
    );
    assert!(
        !text.contains("current working directory"),
        "rg could not name its own directory: the FS grant did not reach it",
    );

    // The exit, on `std_tests`' reasoning: `ripgrep`'s `main` ends in `std::process::exit`, and a
    // program that printed a perfect transcript and then trapped would look identical from here
    // without this.
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(rg.thread)),
        "rg never left: it is neither exited nor faulted",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults_before,
        "rg trapped instead of exiting",
    );

    // **Give the 256-page heap back** (`user::holding`'s reasoning). This program is in the archive
    // only when somebody ran `helpers/build-ripgrep.sh`, so a permanent charge here would make the
    // suite's frame ledger fail for exactly the person running the experiment and pass for everyone
    // else. The thread is already gone, so one call is enough.
    let _ = crate::sched::reclaim_region(rg.heap);
}

// ===========================================================================================
// The walk, without `ripgrep` (the parts of milestone 121 (`ripgrep` on nife: enumeration as a
// capability) that need no argument vector).
// ===========================================================================================
//
// `rg` cannot be told a pattern until §170 (how a foreign program is told what to do) is built as
// milestone 205 (how a foreign program is told what to do), and cannot start from the prompt until
// milestone 595 (the shell runs a `std` program) lands, so these two run `std_exerciser` instead, which is in every
// archive on every ISA. What they prove does not depend on who wrote the walker: the walk is
// `walk_pricing::walk`, plain `std::fs` in the shape `walkdir` and `ignore` walk, and the grant is
// a caretaker narrowing `fixture::walk::ROOT`, which is the capability the confined `rg pattern
// src/` will hold. When `rg` can be told what to do, it meets exactly these two answers.

/// The caretaker's ELF, in every archive since milestone 47 (navigation and naming).
fn caretaker_image() -> &'static [u8] {
    program("fs_subtree_caretaker").expect("no fs_subtree_caretaker program in the initrd archive")
}

/// Run `std_exerciser` holding milestone 121's priced tree with `rights`, and return its whole
/// transcript. `None` when there is nothing to run it against, which the caller skips on.
fn walk_with(rights: u64, out: &mut [u8]) -> Option<usize> {
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let faults_before = USER_FAULTS.load(Ordering::Relaxed);
    let spawned = fs_service::start_std_narrowed(
        block_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        caretaker_image(),
        program("std_exerciser").expect("no std_exerciser program in the initrd archive"),
        filesystem_protocol::fixture::walk::ROOT,
        rights,
    )?;
    let len = super::std_tests::drain_sink(spawned.report, out, "std_exerciser (walk)");

    // `std_exerciser` returns from `main`, and a program that printed a whole transcript and then
    // trapped would look identical from here without these two.
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(spawned.thread)),
        "std_exerciser never left the walk: it is neither exited nor faulted",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults_before,
        "std_exerciser trapped during the walk instead of exiting",
    );
    // **Give everything back: the heap, the stack and the caretaker.** The suite's frame ledger has
    // always carried exactly one `std_exerciser` (`fs_service::start_std`'s reasoning). These are
    // two more, and the first version of this test kept its caretakers: the aarch64 suite then ran
    // out of page frames three tests later, in `std_net_runs_over_the_socket_contract`.
    assert!(
        spawned.release(),
        "the walk's caretaker outlived its holding: a service this test cannot give back",
    );
    Some(len)
}

/// Skip when this archive or this boot cannot run the walk.
macro_rules! skip_without_the_walk {
    () => {
        if fs_service::fs_server_image().is_none() {
            crate::testing::skip!(fs_service::NO_FS_SERVER);
        }
        if super::std_service::std_exerciser_image().is_none() {
            crate::testing::skip!(super::std_service::NO_STD_EXERCISER);
        }
    };
}

/// **A walk through a grant lacking `ENUMERATE` is refused, and never comes back empty.**
///
/// The negative half milestone 121 calls load-bearing: the same tree, the same walker, and one right
/// withheld. A search that silently finds no matches because it could not look is the worst
/// failure a search tool can have, and `filesystem_protocol` chose `EPERM` over an empty listing
/// for exactly this reason, in §47 (a directory capability carries six rights). This proves that
/// choice survives every layer above it: the caretaker, `std`'s `read_dir`, and a stranger-shaped
/// recursive walker that
/// propagates the error rather than skipping the directory.
///
/// The transcript's last line is the control. A capability that reached nothing would refuse
/// every listing too, so the program also opens a file it can name, two levels down, and the
/// refusals are about enumeration alone.
#[test_case]
fn a_walk_without_enumerate_is_refused_rather_than_empty() {
    use filesystem_protocol::dir;
    skip_without_the_walk!();
    let mut got = [0u8; 1024];
    let Some(len) = walk_with(dir::READ | dir::DESCEND, &mut got) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    assert_eq!(
        text,
        "walk granted without enumerate\n\
         read_dir refused\n\
         read_dir below refused\n\
         walk refused, not empty\n\
         named file opened\n",
        "the walk without ENUMERATE printed the wrong transcript",
    );
}

/// A `core::fmt::Write` over a fixed buffer, for building the one line this test compares against
/// without an allocator.
struct Line {
    buf: [u8; 128],
    len: usize,
}

impl core::fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len + s.len();
        self.buf
            .get_mut(self.len..end)
            .ok_or(core::fmt::Error)?
            .copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// **The walk milestone 121 prices, through the capability the confined `rg` will hold.**
///
/// `ENUMERATE | READ | DESCEND` over one subtree, behind a caretaker, walked recursively by
/// `walk_pricing::walk` and then priced: per path component, per directory entry, per KiB read,
/// and whole. The counts are asserted, because they are a fact about the tree and the walk: a
/// walker that skipped a directory, or a grant that leaked a sibling into the listing, changes
/// them. The timings are printed and not asserted, because under TCG they are the emulator's time;
/// notes/walk-pricing.md records what they mean and which run to believe.
#[test_case]
fn a_walk_through_a_confined_grant_is_priced() {
    use core::fmt::Write;

    use filesystem_protocol::dir;
    use filesystem_protocol::fixture::walk as tree;
    skip_without_the_walk!();
    let mut got = [0u8; 2048];
    let Some(len) = walk_with(dir::ENUMERATE | dir::READ | dir::DESCEND, &mut got) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    the priced walk:\n{text}");

    let mut want = Line {
        buf: [0; 128],
        len: 0,
    };
    write!(
        want,
        "walk granted with enumerate\nwalk visited {} entries, {} files, {} bytes, {} components\n",
        tree::WALK_ENTRIES,
        tree::WALK_FILES,
        tree::WALK_BYTES,
        tree::WALK_COMPONENTS,
    )
    .expect("the expected line fits");
    let want = core::str::from_utf8(&want.buf[..want.len]).expect("ASCII");
    assert!(
        text.starts_with(want),
        "the priced walk did not visit exactly the fixture: wanted it to begin {want:?}",
    );
    for figure in [
        "walk per component",
        "walk per entry",
        "walk per KiB",
        "walk whole",
    ] {
        assert!(
            text.contains(figure),
            "the priced walk printed no `{figure}` line"
        );
    }
}
