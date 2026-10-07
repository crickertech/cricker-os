//! **`ripgrep`, unmodified, from crates.io** (milestone 121; `design/fatal-risks/README.md` risk 1).
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
//! than a lane's. `rg` is never part of the base image either: calef ruled on 2026-10-07 (UTC) that
//! it is installed with `jig` from its own package. See notes/ripgrep-on-nife.md.
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
/// capability it was handed, and, granted no argument page, hears no arguments.**
///
/// Every layer below the search works, and none of it was written for `ripgrep`: the loader maps a
/// multi-megabyte ELF, std grows a heap one page at a time, `std::env::current_dir` answers `/`
/// because this process holds a directory, and output reaches the one endpoint it was granted.
///
/// This was the whole result until milestone 205 (how a foreign program is told what to do) gave a
/// program an argv. It stays as the honest-absence case: slot 8 empty is no arguments at all, not
/// even `argv[0]`, so `ripgrep` prints its own "requires at least one pattern" and leaves. The
/// searches below give it the page.
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
// The walk, without `ripgrep`: the parts of milestone 121 (`ripgrep` on nife: enumeration as a
// capability) that run in every archive.
// ===========================================================================================
//
// These run `std_exerciser`, which is in every archive on every ISA, so CI proves them where it
// cannot prove the `rg` tests below. What they prove does not depend on who wrote the walker: the
// walk is `walk_pricing::walk`, plain `std::fs` in the shape `walkdir` and `ignore` walk, and the
// grant is a caretaker narrowing `fixture::walk::ROOT`, the same grant `rg` holds below.

/// The caretaker's ELF, in every archive since milestone 47 (navigation and naming).
fn caretaker_image() -> &'static [u8] {
    program("fs_subtree_caretaker").expect("no fs_subtree_caretaker program in the initrd archive")
}

/// Run `std_exerciser` holding milestone 121's priced tree with `rights`, and return its whole
/// transcript. `None` when there is nothing to run it against, which the caller skips on.
fn walk_with(rights: u64, out: &mut [u8]) -> Option<usize> {
    let image = program("std_exerciser").expect("no std_exerciser program in the initrd archive");
    run_confined(image, "std_exerciser (walk)", rights, None, out).map(|(len, _)| len)
}

/// **Run `image` holding the priced tree with `rights`, told `line` if given**, and return its
/// transcript's length and how many of its 256 heap pages it spent. The program must exit on its
/// own and must not trap. Everything it held comes back before this returns.
fn run_confined(
    image: &'static [u8],
    who: &str,
    rights: u64,
    line: Option<&str>,
    out: &mut [u8],
) -> Option<(usize, u64)> {
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let faults_before = USER_FAULTS.load(Ordering::Relaxed);
    let spawned = fs_service::start_std_narrowed(
        block_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        caretaker_image(),
        image,
        filesystem_protocol::fixture::walk::ROOT,
        rights,
        line.map(str::as_bytes),
    )?;
    let len = super::std_tests::drain_sink(spawned.report, out, who);

    // The program returns from `main` or calls `exit`, and one that printed a whole transcript and
    // then trapped would look identical from here without these two.
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(spawned.thread)),
        "{who} never left: it is neither exited nor faulted",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults_before,
        "{who} trapped instead of exiting",
    );
    // Pages the heap's region retyped, which only grows: the program's high-water mark.
    let spent = crate::memory_region::usage(spawned.heap).map_or(0, |(spent, _)| spent);
    // **Give everything back: the heap, the stack, the argument page and the caretaker.** The
    // suite's frame ledger has always carried exactly one `std_exerciser` (`fs_service::start_std`'s
    // reasoning). These are more, and the first version of the walk test kept its caretakers: the
    // aarch64 suite then ran out of page frames three tests later, in
    // `std_net_runs_over_the_socket_contract`.
    assert!(
        spawned.release(),
        "{who}'s caretaker outlived its holding: a service this test cannot give back",
    );
    Some((len, spent))
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
        "walk split",
    ] {
        assert!(
            text.contains(figure),
            "the priced walk printed no `{figure}` line"
        );
    }
}

// ===========================================================================================
// The same walk through a grant the FS server enforces itself (milestone 606 (a directory walk
// costs what it does on Linux), calef's ruling D, 2026-09-27). No caretaker: the program holds
// the FS server's endpoint with a badge the kernel bound to the subtree, and `subtree_scope`
// decides what the badge reaches.
// ===========================================================================================

/// **The priced walk through a bound grant visits exactly what the caretaker grant visits.** The
/// counts are the fixture's, so a bound grant that leaked a sibling into the walk, or hid part of
/// the tree, fails here. The timings print beside the caretaker's for the comparison.
#[test_case]
fn a_walk_through_a_bound_grant_is_priced() {
    use filesystem_protocol::dir;
    use filesystem_protocol::fixture::walk as tree;
    skip_without_the_walk!();
    let Some(spawned) = fs_service::start_std_bound(
        block_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        program("std_exerciser").expect("no std_exerciser program in the initrd archive"),
        tree::ROOT,
        dir::ENUMERATE | dir::READ | dir::DESCEND,
    ) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    let mut got = [0u8; 2048];
    let len = super::std_tests::drain_sink(spawned.report, &mut got, "std_exerciser (bound walk)");
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(spawned.thread)),
        "std_exerciser never left the bound walk",
    );
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    the walk through a bound grant:\n{text}");
    let mut want = Line {
        buf: [0; 128],
        len: 0,
    };
    core::fmt::Write::write_fmt(
        &mut want,
        format_args!(
            "walk granted with enumerate\nwalk visited {} entries, {} files, {} bytes, {} components\n",
            tree::WALK_ENTRIES,
            tree::WALK_FILES,
            tree::WALK_BYTES,
            tree::WALK_COMPONENTS,
        ),
    )
    .expect("the expected line fits");
    let want = core::str::from_utf8(&want.buf[..want.len]).expect("ASCII");
    assert!(
        text.starts_with(want),
        "the bound walk did not visit exactly the fixture: wanted it to begin {want:?}",
    );
    assert!(
        spawned.release(),
        "the FS server refused to take the bound grant back"
    );
}

/// **A bound badge reaches its subtree and nothing else, and loses it on `UNBIND`.** Spoken for
/// the badge by the kernel, so each refusal is the FS server's own and not a PAL's: a name outside
/// the subtree is not there, a handle the kernel opened is not the badge's, the badge cannot bind,
/// and after the grant is taken back even its own root is gone.
#[test_case]
fn a_bound_grant_reaches_nothing_outside_it() {
    use filesystem_protocol::fixture::walk as tree;
    use filesystem_protocol::{dir, fs};
    skip_without_the_walk!();
    let Some(grant) = fs_service::bind_subtree(
        block_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        tree::ROOT,
        dir::ENUMERATE | dir::READ | dir::DESCEND,
    ) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    // A request's name is read from the window its badge names: the grant's own for the badge,
    // window 0 for the kernel's unbadged calls.
    let stage_in = |base: u64, name: &str| {
        // SAFETY: a window frame the file service owns; no program holds either in this test,
        // and the suite runs one test at a time.
        unsafe {
            core::ptr::copy_nonoverlapping(
                name.as_ptr(),
                crate::arch::mmu::phys_to_virt(base) as *mut u8,
                name.len(),
            );
        }
        name.len() as u64
    };
    let stage = |name: &str| stage_in(grant.phys, name);
    let call = |w0: u64, w1: u64, badge: u64| {
        crate::sched::ipc_call_badged(grant.file_ep, [w0, w1], badge)[0] as i64
    };
    let b = grant.window;

    let inside = "narrow/n000";
    let len = stage(inside);
    let h = call(fs::req(fs::OPEN, fs::ROOT, len), dir::READ, b);
    assert!(
        h >= 0,
        "a file inside the bound subtree would not open: {h}"
    );
    assert_eq!(call(fs::req(fs::CLOSE, h as u64, 0), 0, b), 0);

    let len = stage(filesystem_protocol::fixture::MOTD_NAME);
    assert_eq!(
        call(fs::req(fs::OPEN, fs::ROOT, len), dir::READ, b),
        -2,
        "the image root's motd is not in the subtree, so it is not there (ENOENT)",
    );
    let len0 = stage_in(grant.shared, filesystem_protocol::fixture::MOTD_NAME);
    let theirs = call(fs::req(fs::OPEN, fs::ROOT, len0), dir::READ, 0);
    assert!(
        theirs >= 0,
        "the kernel's unbound call could not open motd: {theirs}"
    );
    assert_eq!(
        call(fs::req(fs::FSTAT, theirs as u64, 0), 0, b),
        -9,
        "a handle the badge did not mint is not its to use (EBADF)",
    );
    let _ = call(fs::req(fs::CLOSE, theirs as u64, 0), 0, 0);
    assert_eq!(
        call(fs::req(fs::BIND, fs::ROOT, 0), b + 1, b),
        -1,
        "a bound badge cannot bind another (EPERM)",
    );
    assert_eq!(
        call(fs::req(fs::CLOSE, fs::ROOT, 0), 0, b),
        -22,
        "a bound badge cannot close its grant's root (EINVAL)",
    );

    assert_eq!(
        call(fs::req(fs::UNBIND, 0, 0), b, 0),
        0,
        "the kernel takes the grant back"
    );
    let len = stage(inside);
    assert_eq!(
        call(fs::req(fs::OPEN, fs::ROOT, len), dir::READ, b),
        -9,
        "after UNBIND the badge reaches nothing, not the whole image (EBADF)",
    );
    fs_service::release_window_after_test(b);
}

// ===========================================================================================
// `rg` itself: milestone 121's three points, since milestone 205 (how a foreign program is told
// what to do) gave a foreign program an argv and milestone 206 (a program image has under 896 KiB)
// gave it room for its image. The grant is the walk tests' caretaker over
// `fixture::walk::ROOT`; the words are `fixture::walk::RG_SEARCH`, made into an argv by
// `grant_plan::argv`, the shell's own function, so `rg` hears what `swish` would send for that
// line. This is the kernel harness, not the prompt: `rg` is in this archive only because somebody
// ran `helpers/build-ripgrep.sh`, and it is never part of the base image (calef, 2026-10-07: it is
// installed by `jig`, from its own package).
// ===========================================================================================

/// Skip when this archive has no `rg` or this boot no file service.
macro_rules! skip_without_rg {
    () => {
        if program("rg").is_none() {
            crate::testing::skip!(NO_RIPGREP);
        }
        if fs_service::fs_server_image().is_none() {
            crate::testing::skip!(fs_service::NO_FS_SERVER);
        }
    };
}

/// One `rg` run over the priced tree, its transcript in `out`. Prints the transcript's tail and
/// the heap it spent, which is the figure the 256-page budget is judged on.
fn rg_with(rights: u64, line: &str, out: &mut [u8]) -> Option<usize> {
    let image = program("rg").expect("no rg program in the initrd archive");
    let (len, spent) = run_confined(image, "rg", rights, Some(line), out)?;
    let text = core::str::from_utf8(&out[..len]).unwrap_or("<not utf-8>");
    crate::println!("    `{line}` printed {len} bytes and spent {spent} heap pages; it ended:");
    // The stats block and, for a refusal, the error above it: the last dozen lines. A line of
    // ripgrep's own that says files were "skipped" is left out, because the runner reads that word
    // in a test's output as a test that skipped itself and returned: milestone 214 (a test that
    // prints "skipping" and returns is counted as passed).
    let lines = text.lines().count();
    for l in text.lines().skip(lines.saturating_sub(12)) {
        if !l.contains("skip") {
            crate::println!("    | {l}");
        }
    }
    Some(len)
}

/// `"{n} {what}\n"`, without an allocator.
fn count_line(n: usize, what: &str) -> Line {
    use core::fmt::Write;
    let mut l = Line {
        buf: [0; 128],
        len: 0,
    };
    write!(l, "\n{n} {what}\n").expect("the line fits");
    l
}

impl Line {
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).expect("ASCII")
    }
}

/// **Unmodified `ripgrep` searches a tree it was granted, and finds exactly what is there.**
///
/// The first half of the point of milestone 121: a stranger's forty-crate search tool, told a
/// pattern, walks a directory capability carrying `ENUMERATE | READ | DESCEND` and searches every
/// file in it. The counts come from `rg --stats` and are asserted against the fixture's own
/// constants, so a walk that skipped a directory, a read that came back short, or a grant that
/// leaked a sibling into the listing fails on a number: every file searched, every byte of every
/// file searched, and one match in each file that holds the walk's small body. One match line is
/// asserted whole, to prove `rg` printed matches and not only a summary.
///
/// Falsification: unfalsified. A replay needs `rg` in the archive, and no gate builds it (the
/// archive takes it only after `helpers/build-ripgrep.sh`), so a replayed patch would skip here.
#[test_case]
fn ripgrep_searches_the_tree_it_was_granted() {
    use filesystem_protocol::dir;
    use filesystem_protocol::fixture::walk as tree;
    skip_without_rg!();
    let mut got = [0u8; 8192];
    let Some(len) = rg_with(
        dir::ENUMERATE | dir::READ | dir::DESCEND,
        tree::RG_SEARCH,
        &mut got,
    ) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    assert!(
        text.contains("narrow/n000:nife walk entry\n"),
        "rg printed no match line for narrow/n000",
    );
    for (n, what) in [
        (tree::RG_MATCHES, "matches"),
        (tree::RG_MATCHES, "files contained matches"),
        (tree::WALK_FILES, "files searched"),
        (tree::WALK_BYTES, "bytes searched"),
    ] {
        let want = count_line(n, what);
        assert!(
            text.contains(want.as_str()),
            "rg's stats did not say {:?}",
            want.as_str(),
        );
    }
    assert!(text.ends_with(" seconds\n"), "rg did not finish its stats");
}

/// **`rg` through a grant lacking `ENUMERATE` is refused, and says so, rather than finding nothing.**
///
/// The second half, and the one milestone 121 calls load-bearing: the same `rg`, the same words,
/// the same tree, and one right withheld. A search that silently reports zero matches because it
/// could not look is the worst failure a search tool can have. Here `rg` names the refusal (the
/// `EPERM` §47 chose over an empty listing, carried through the caretaker, std's `read_dir` and
/// `ignore`'s walker, and worded by the PAL as "this directory capability does not carry the right
/// that verb needs"), says it searched nothing, and prints no match.
///
/// The control is [`ripgrep_reaches_a_named_file_without_enumerate`]: under the same grant, a
/// search naming a file still finds its line, so this refusal is about enumeration alone.
///
/// Falsification: unfalsified. A replay needs `rg` in the archive, and no gate builds it (the
/// archive takes it only after `helpers/build-ripgrep.sh`), so a replayed patch would skip here.
#[test_case]
fn ripgrep_without_enumerate_is_refused_rather_than_empty() {
    use filesystem_protocol::dir;
    use filesystem_protocol::fixture::walk as tree;
    skip_without_rg!();
    let mut got = [0u8; 8192];
    let Some(len) = rg_with(dir::READ | dir::DESCEND, tree::RG_SEARCH, &mut got) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    assert!(
        text.contains("does not carry the right"),
        "rg did not name the refusal: a walk it could not make read as something else",
    );
    assert!(
        !text.contains(":nife walk entry"),
        "rg printed a match through a grant that cannot list",
    );
    assert!(
        text.contains(count_line(0, "files searched").as_str()),
        "rg searched files through a grant that cannot list",
    );
}

/// **The refusal's control: a named file needs no `ENUMERATE`.** `READ | DESCEND`, the grant the
/// refusal above holds, and `rg` told a path two components down. It finds the one line, so the
/// grant reaches the tree and only listing is withheld.
///
/// The line is the one a person would type, without `--no-mmap` or `--threads 1`, and that is a
/// second finding. A single named file is exactly where `ripgrep` reaches for a memory map, and
/// nife has none: `memmap2` compiles its stub, the map fails, and `ripgrep` reads instead. So
/// `--no-mmap` is a measurement pin and never a requirement.
///
/// Falsification: unfalsified. A replay needs `rg` in the archive, and no gate builds it (the
/// archive takes it only after `helpers/build-ripgrep.sh`), so a replayed patch would skip here.
#[test_case]
fn ripgrep_reaches_a_named_file_without_enumerate() {
    use filesystem_protocol::dir;
    use filesystem_protocol::fixture::walk as tree;
    skip_without_rg!();
    let mut got = [0u8; 1024];
    let Some(len) = rg_with(dir::READ | dir::DESCEND, tree::RG_NAMED, &mut got) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    assert_eq!(
        core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>"),
        "nife walk entry\n",
        "rg did not find the line in the file it was named",
    );
}
