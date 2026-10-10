//! `std_threads_tests`: **milestone 812 (`std::thread::spawn` runs real threads in one address
//! space)'s exit test**, on all three architectures (§19 (architectural parity is a tenet)).
//!
//! The `std_threads` program is ordinary `std` Rust: four threads, an `AtomicU64` and a
//! `Mutex<u64>` a million increments each, a `thread_local!` per thread, and
//! `available_parallelism`. It runs here the way a loader that gives a program threads builds it:
//! a process created around its space, its first thread joined to that process, and the runtime's
//! fixed slots filled in the process's shared table, the three new ones included (its own process
//! with `BIND`, its own first thread, its own space). Then its transcript is the verdict.
//!
//! # BUGS
//!
//! - **The rayon half of the block's exit test is not here.** It needs `rayon` as a dependency of
//!   a test program, which is calef's to rule (§46 (thin primitives or whole subsystems)); it is
//!   asked on pull request #1892.

use super::*;
use crate::cap::Rights;
use crate::sched;

/// The program's ELF bytes, if this archive carries it (it is built by `cargo xtask std-exerciser`,
/// which `script/test` runs first).
fn std_threads_image() -> Option<&'static [u8]> {
    program("std_threads")
}

/// **Run a `std` program as a process, the way a loader that gives it threads builds it**, and hand
/// back its whole stdout. Its space is loaded from `image`, a process is created around it, its
/// first thread joins that process, and the runtime's fixed slots go in the process's shared table,
/// the three milestone 812 added included: its own process with `BIND`, its own first thread and its
/// own space. Then the program runs to its end, and the test checks that its process ended with
/// every thread reaped and its space gone, and gives every region back.
fn run_as_process(image: &'static [u8], what: &str, out: &mut [u8]) -> usize {
    let (mut space, entry) = load(image, 0).expect("could not load the program");

    // The deep stack a std program's main thread needs, from a region this test gives back.
    let stack_region =
        crate::memory_region::create(std_runtime_protocol::STACK_PAGES).expect("no stack region");
    for k in 1..std_runtime_protocol::STACK_PAGES {
        let frame = crate::memory_region::retype_page(stack_region).expect("no stack frame");
        space
            .map_physical(
                USER_STACK_VA - k * FRAME_SIZE,
                frame,
                Flags::user_data(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .expect("could not map the program's stack");
    }
    let name = readopt_user_address_space(space).expect("register the program's space");

    let process_region = crate::memory_region::create(4).expect("no process region");
    let pid = sched::create_process_from(process_region, name).expect("no process");
    let tid = sched::create_thread_control_block(process_region).expect("no tcb");
    let page = sched::configure_thread_control_block_joining(tid, entry, USER_STACK_TOP, pid, 0)
        .expect("the program's first thread could not join its process");
    assert_eq!(
        page,
        current_cpu_protocol::PAGE_VA,
        "a process's first member reads the page every thread read before"
    );

    let report_region = crate::memory_region::create(1).expect("no report region");
    let report = sched::create_rendezvous_from(report_region).expect("no report rendezvous");
    let budget = crate::memory_region::create(super::std_service::BUDGET_PAGES)
        .expect("no heap budget for the program");
    for (slot, cap) in [
        (
            std_runtime_protocol::MEMORY_REGION_SLOT,
            crate::cap::memory_region_cap(budget),
        ),
        (
            std_runtime_protocol::STDOUT_SLOT,
            crate::cap::rendezvous_cap(report, Rights::WRITE),
        ),
        (
            std_runtime_protocol::PROCESS_SLOT,
            crate::cap::process_capability(pid, Rights::BIND),
        ),
        (
            std_runtime_protocol::THREAD_SLOT,
            crate::cap::thread_control_block_cap(tid, Rights::WRITE),
        ),
        (
            std_runtime_protocol::SPACE_SLOT,
            crate::cap::address_space_cap(name, Rights::READ),
        ),
    ] {
        sched::thread_control_block_insert_cap(tid, cap, Some(slot))
            .expect("could not fill a runtime slot in the process's table");
    }
    sched::start_thread_control_block(tid, [0; 3]).expect("start the program");

    let len = drain_until_end(report, pid, out, what);
    assert!(
        super::wait_for(|| !sched::is_thread_present(tid)),
        "{what}'s main thread never left"
    );
    assert!(
        super::wait_for(|| sched::process_state(pid).is_some_and(|(m, ended)| m == 0 && ended)),
        "{what}'s process did not end with all its threads reaped",
    );
    assert!(
        user_address_space_root(name).is_none(),
        "{what}'s space outlived its process"
    );
    for region in [budget, report_region, process_region, stack_region] {
        sched::reclaim_region(region).expect("a region the program used did not come back");
    }
    len
}

/// `std_tests::drain_sink`, except that a process that ends without its end-of-output fails the
/// test rather than leaving it blocked in `RECEIVE` forever. A thread's fault ends its whole
/// process, and a dead program sends no end-of-output.
fn drain_until_end(
    report: sched::RendezvousId,
    pid: sched::ProcessId,
    out: &mut [u8],
    what: &str,
) -> usize {
    let mut len = 0usize;
    loop {
        while sched::rendezvous_waiting_senders(report) == 0 {
            if sched::process_state(pid).is_none_or(|(_, ended)| ended) {
                panic!(
                    "{what} ended before its output did, after: {}",
                    core::str::from_utf8(&out[..len]).unwrap_or("<not utf-8>")
                );
            }
            sched::yield_now();
        }
        let words = sched::ipc_receive(report);
        let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(words[0], words[1], words[2], &mut chunk) {
            byte_sink_protocol::Msg::Bytes(n) => {
                assert!(
                    len + n <= out.len(),
                    "{what}: wrote more than the buffer holds"
                );
                out[len..len + n].copy_from_slice(&chunk[..n]);
                len += n;
            }
            byte_sink_protocol::Msg::Eof => return len,
            byte_sink_protocol::Msg::Malformed => panic!("{what}: a sink message out of contract"),
        }
    }
}

/// **Four `std` threads share an atomic, a mutex and an address space, and each keeps its own
/// thread-local**, which is the exit test of milestone 812's block.
///
/// The transcript says: both counters read four million, every thread read back its own
/// thread-local and the main thread's is untouched, the shared state was there after the first
/// thread exited, and `available_parallelism` is the online count (§269 (how threads share a
/// process) fork 7). After the program ends, its process has ended with every member reaped.
///
/// Falsification: replayable `system_tests/falsifications/user.std_threads_tests.four_std_threads_share_a_mutex_an_atomic_and_a_space.patch`
#[test_case]
fn four_std_threads_share_a_mutex_an_atomic_and_a_space() {
    let Some(image) = std_threads_image() else {
        crate::testing::skip!(super::std_service::NO_STD_EXERCISER);
    };
    let mut got = [0u8; 512];
    let len = run_as_process(image, "std_threads", &mut got);
    let online = crate::smp::online_harts_mask().count_ones();
    let mut want = [0u8; 512];
    let want_len = {
        use core::fmt::Write;
        struct Buf<'a>(&'a mut [u8], usize);
        impl Write for Buf<'_> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let end = self.1 + s.len();
                self.0
                    .get_mut(self.1..end)
                    .ok_or(core::fmt::Error)?
                    .copy_from_slice(s.as_bytes());
                self.1 = end;
                Ok(())
            }
        }
        let mut b = Buf(&mut want, 0);
        write!(
            b,
            "threads 4\natomic 4000000\nmutex 4000000\nthread-locals each its own\n\
             main thread-local 0\nspace outlived the first exit true\nparallelism {online}\n"
        )
        .expect("the expected transcript fits");
        b.1
    };
    assert_eq!(
        core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>"),
        core::str::from_utf8(&want[..want_len]).unwrap_or("<not utf-8>"),
        "std_threads's transcript",
    );
}

/// **The `std` heap's one lock, measured under four threads** (the block's item 12: "measure it under
/// four threads before replacing it"). `std_heap_contention` does the same allocations with one
/// thread and then four and prints both times; this prints them to the log and checks only that both
/// arrived. Under QEMU the numbers are a ratio, not a speed. `notes/std/threads.md` records them.
///
/// Falsification: unfalsified. It is a measurement, and it asserts nothing a defect could break
/// except that the program ran to its end, which `four_std_threads_share_a_mutex_an_atomic_and_a_space`
/// already falsifies.
#[test_case]
fn the_std_heap_is_measured_under_four_threads() {
    let Some(image) = program("std_heap_contention") else {
        crate::testing::skip!(super::std_service::NO_STD_EXERCISER);
    };
    let mut got = [0u8; 256];
    let len = run_as_process(image, "std_heap_contention", &mut got);
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    for line in text.lines() {
        crate::println!("    (std heap under threads: {line})");
    }
    for line in ["one thread us ", "four threads us "] {
        assert!(
            text.lines().any(|l| l
                .strip_prefix(line)
                .is_some_and(|n| n.parse::<u64>().is_ok())),
            "std_heap_contention printed no `{line}` figure: {text}",
        );
    }
}
