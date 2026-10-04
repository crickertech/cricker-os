//! **The terminal is replaced live, and the person typing at it loses nothing** (milestone 23 (a
//! capability-routed component OS with live replacement); calef's ruling of 2026-09-27 that a
//! terminal supervisor holds the line editor).
//!
//! A real `terminal_supervisor` builds a real `line_editor` from its declaration, over a fake
//! console that acks every flush. The test plays the input driver and a line-mode reader on the
//! terminal endpoint, exactly as `raw_mode_service` does, and holds the supervisor's swap endpoint,
//! which on a real boot would be the installer's (milestone 198 (a package manager, and the trivial install), not built). What it proves is the
//! whole §209 (state handoff is an opaque blob over a granted frame, and it is optional) swap on a
//! component a person uses: a line half typed before the swap is finished after it, history typed
//! before it is recalled after it, and the reader parked across it is handed back and asks again.

use line_editor::component::supervisor as s;
use line_editor::proto;

use super::*;
use crate::cap::{Rights, page_frame_cap, rendezvous_cap};
use crate::sched;
use crate::user::holding::Holding;

const APP_OUT_LEN: u64 = 0;

const MAX_ELF_PAGES: usize = 64;

/// Stack pages below the one `run` maps. `run` gives a program one page, which a debug build of the
/// ELF loader and the supervision builder overflows; `swapper` meets the same need in its own
/// spawn helper with `INIT_STACK_PAGES`. These are ordinary `Spawn` windows placed below the stack.
const EXTRA_STACK_PAGES: usize = 7;

/// The supervisor's view of the `line_editor` image, one mapping per page. A static for `spawn`'s
/// 1 KiB capture limit; see its one use.
static mut ELF_MAPS: [Mapping; MAX_ELF_PAGES + EXTRA_STACK_PAGES] = [const {
    Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_rodata(),
    }
};
    MAX_ELF_PAGES + EXTRA_STACK_PAGES];

fn bytes_call(term: sched::RendezvousId, bytes: &[u8]) {
    let mut w1 = 0u64;
    for (i, &b) in bytes.iter().enumerate() {
        w1 |= (b as u64) << (8 * i);
    }
    sched::ipc_call(term, [proto::req(proto::OP_BYTES, bytes.len() as u64), w1]);
}

fn settle() {
    let deadline = crate::arch::timer::now() + crate::arch::timer::frequency() / 20;
    while crate::arch::timer::now() < deadline {
        sched::yield_now();
    }
}

fn line_at(phys: u64, len: usize) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (i, b) in out.iter_mut().take(len.min(8)).enumerate() {
        // SAFETY: the client input page, mapped into `line_editor` read/write; the read is ordered
        // after the reply that says the line is there.
        *b = unsafe { core::ptr::read_volatile((mmu::phys_to_virt(phys) + i as u64) as *const u8) };
    }
    out
}

/// A reader that reads lines until told to stop, reporting every reply, retries included.
fn spawn_reader(term: sched::RendezvousId, report: sched::RendezvousId, lines: usize) {
    sched::spawn(move || {
        let mut done = 0;
        while done < lines {
            let r = sched::ipc_call(term, [proto::req(proto::OP_READLINE, APP_OUT_LEN), 0]);
            sched::ipc_send(report, [r[0], r[1], 0]);
            if !proto::is_retry(r[0], r[1]) {
                done += 1;
            }
        }
    })
    .expect("could not spawn the reader");
}

#[test_case]
fn the_terminal_is_replaced_under_a_person_typing_and_nothing_they_typed_is_lost() {
    let supervisor_image =
        program("terminal_supervisor").expect("no terminal_supervisor in the initrd archive");
    let editor_image = program("line_editor").expect("no line_editor in the initrd archive");

    let ep_region = crate::memory_region::create(8).expect("no endpoint region");
    let term = sched::create_rendezvous_from(ep_region).expect("term");
    let conreq = sched::create_rendezvous_from(ep_region).expect("conreq");
    let conrep = sched::create_rendezvous_from(ep_region).expect("conrep");
    let swap = sched::create_rendezvous_from(ep_region).expect("swap");
    let budget = crate::memory_region::create(s::BUDGET_PAGES).expect("no supervisor budget");

    // Every frame the test hands out comes from one region, so reclaiming it at the end gives
    // them all back: `raw_mode_service`'s BUGS records what allocating them loose costs the suite.
    let frames = crate::memory_region::create((MAX_ELF_PAGES + EXTRA_STACK_PAGES) as u64 + 3)
        .expect("no frame region");
    let alloc = || crate::memory_region::retype_page(frames).expect("frame region exhausted");
    let (console_page, app_out, app_in) = (alloc(), alloc(), alloc());

    // The `line_editor` image, copied into fresh frames the supervisor maps at ELF_VA: the kernel's
    // stand-in for `system_initializer`'s `blobs` copy.
    let pages = editor_image.len().div_ceil(FRAME_SIZE as usize);
    assert!(
        pages <= MAX_ELF_PAGES,
        "line_editor's stripped image outgrew the test's mapping"
    );
    // The spawn closure below borrows this static because a closure that captured the array
    // itself would be over the 1 KiB `spawn` allows.
    let maps_p = &raw mut ELF_MAPS;
    // SAFETY: this test is the only user of `ELF_MAPS`, and it runs once.
    let maps: &'static mut [Mapping; MAX_ELF_PAGES + EXTRA_STACK_PAGES] = unsafe { &mut *maps_p };
    for (i, chunk) in editor_image.chunks(FRAME_SIZE as usize).enumerate() {
        let phys = alloc();
        // SAFETY: a frame just allocated for this copy and not yet mapped anywhere else.
        unsafe {
            core::ptr::copy_nonoverlapping(
                chunk.as_ptr(),
                mmu::phys_to_virt(phys) as *mut u8,
                chunk.len(),
            );
        }
        maps[i] = Mapping {
            va: s::ELF_VA + i as u64 * FRAME_SIZE,
            phys,
            flags: Flags::user_rodata(),
        };
    }

    for k in 0..EXTRA_STACK_PAGES {
        maps[pages + k] = Mapping {
            va: USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE,
            phys: alloc(),
            flags: Flags::user_data(),
        };
    }
    let windows = pages + EXTRA_STACK_PAGES;
    let maps_ro: &'static [Mapping; MAX_ELF_PAGES + EXTRA_STACK_PAGES] = maps;
    let console_tid = sched::spawn(move || {
        loop {
            sched::ipc_receive(conreq);
            sched::ipc_send(conrep, [0, 0, 0]);
        }
    })
    .expect("could not spawn the fake console");

    let elf_len = editor_image.len() as u64;
    let supervisor_tid = sched::spawn(move || {
        run(
            supervisor_image,
            Spawn {
                arg0: 0, // the console's sink shape
                arg1: elf_len,
                arg2: 7,
                grants: &[
                    crate::cap::memory_region_root_cap(budget), // BUDGET
                    rendezvous_cap(term, Rights::ALL),          // TERMINAL
                    rendezvous_cap(conreq, Rights::WRITE.union(Rights::GRANT)), // SINK
                    page_frame_cap(console_page, Rights::ALL),  // SINK_PAGE
                    page_frame_cap(app_out, Rights::ALL),       // CLIENT_OUT
                    page_frame_cap(app_in, Rights::ALL),        // CLIENT_IN
                    rendezvous_cap(conrep, Rights::READ.union(Rights::GRANT)), // SINK_REPLY
                    rendezvous_cap(swap, Rights::READ),         // the swap endpoint, slot 7
                ],
                maps: &maps_ro[..windows],
            },
        )
    })
    .expect("could not spawn terminal_supervisor");

    let mut held = Holding::new();
    held.add_thread(console_tid);
    held.add_thread(supervisor_tid);
    held.add_region(ep_region);
    held.add_region(frames);

    // A line typed and read before any swap, so there is history to carry.
    let report = sched::create_rendezvous();
    spawn_reader(term, report, 1);
    settle();
    bytes_call(term, b"first\r");
    let r = sched::ipc_receive(report);
    assert_eq!(
        r[0], 5,
        "the first line never arrived: the supervisor's line_editor is not serving"
    );

    // Half a line, with a reader parked on it, and then the swap.
    spawn_reader(term, report, 2);
    settle();
    bytes_call(term, b"ec");
    let swapped = sched::ipc_call(swap, [s::SWAP, 0]);
    assert_eq!(
        swapped[0],
        s::SWAPPED,
        "the swap did not commit (reply {:#x}, detail {}): the replacement refused the blob",
        swapped[0],
        swapped[1],
    );
    assert_eq!(
        swapped[1], 2,
        "the supervisor should have started exactly two instances"
    );
    let retried = sched::ipc_receive(report);
    assert!(
        proto::is_retry(retried[0], retried[1]),
        "the reader parked across the swap was not handed back with FLAG_RETRY",
    );

    // The replacement finishes the line the incumbent started.
    bytes_call(term, b"ho\r");
    let r = sched::ipc_receive(report);
    assert_eq!(
        r[0], 4,
        "the line typed across the swap came back {} bytes long",
        r[0]
    );
    assert_eq!(
        &line_at(app_in, 4)[..4],
        b"echo",
        "the half-typed line did not survive the swap"
    );

    // And recalls what the incumbent was told before it: Up twice is the pre-swap line.
    settle();
    bytes_call(term, b"\x1b[A\x1b[A\r");
    let r = sched::ipc_receive(report);
    assert_eq!(
        r[0], 5,
        "history typed before the swap is not recallable after it"
    );
    assert_eq!(&line_at(app_in, 5)[..5], b"first");

    // A second swap works too, which is the control endpoints alternating.
    let again = sched::ipc_call(swap, [s::SWAP, 0]);
    assert_eq!(
        (again[0], again[1]),
        (s::SWAPPED, 3),
        "a second swap did not commit"
    );

    // Retire the terminal so every instance's region comes home to the budget.
    let stopped = sched::ipc_call(swap, [s::STOP, 0]);
    assert_eq!(stopped[0], s::STOPPED);
    settle();
    held.release_or_fail("terminal_supervisor");
    sched::reclaim_region(budget)
        .expect("the supervisor's budget would not reclaim: an instance's region never came home");
}
