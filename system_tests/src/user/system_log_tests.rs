//! **The system log service, end to end** (milestone 613 (a system log service: the in-memory
//! half)).
//!
//! The kernel plays two parts here, the same substitution the sink tests make: the **spawner**,
//! which holds the one unbadged intake capability and registers what each badge means, and the
//! **reader**, which `SEND`s a read through its own badge and waits on its notification. The two
//! writers are real user programs (`sink_transcript_writer`), each holding nothing but a badged
//! copy of the intake endpoint and a report endpoint, so the badge reaches the service through the
//! syscall path every writer will use.

use byte_sink_protocol::fixture;
use system_log_protocol::{control, read};

use super::*;
use crate::cap::{Rights, notification_cap, rendezvous_cap, rendezvous_cap_badged};
use crate::sched::RendezvousId;

/// The two writers' badges and users. Distinct users, so a reader that confused the two would put
/// the wrong name on a line rather than the right name on both.
const WRITERS: [(u32, &[u8]); 2] = [(1, b"alice"), (2, b"bob")];
/// The system reader's badge, answered in window 0.
const SYSTEM_READER: u32 = 9;
/// The per-user reader's badge, alice's, answered in window 1.
const ALICE_READER: u32 = 8;

/// Play the spawner: one control word, through the unbadged capability (a kernel `SEND` carries
/// badge 0).
fn control(intake: RendezvousId, w: (u64, u64, u64)) {
    crate::sched::ipc_send(intake, [w.0, w.1, w.2]);
}

/// Register `badge` as `program` run by `user` (empty for none).
fn register(intake: RendezvousId, badge: u32, program: &[u8], user: &[u8]) {
    for (field, name) in [(control::PROGRAM, program), (control::USER, user)] {
        for m in control::name_messages(badge, field, name)
            .into_iter()
            .flatten()
        {
            control(intake, m);
        }
    }
}

/// Spawn `sink_transcript_writer` holding only a badged copy of the intake endpoint (slot 0) and
/// its report endpoint (slot 1). It writes the transcript once and reports how its last `SEND`
/// classified.
fn spawn_writer(image: &'static [u8], intake: RendezvousId, badge: u32, report: RendezvousId) {
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 1,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap_badged(intake, Rights::WRITE, badge as u64), // slot 0
                    rendezvous_cap(report, Rights::WRITE),                      // slot 1
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn a log writer");
}

/// The most lines a test reads back from one window.
const MAX_LINES: usize = 8;

/// Play a reader: ask for everything from `cursor` through `badge`, wait for the service to say the
/// window is filled, and return the next cursor and the window's lines.
fn read_window(
    intake: RendezvousId,
    badge: u32,
    done: crate::sched::NotificationId,
    window_phys: u64,
    cursor: u64,
) -> (u64, [&'static str; MAX_LINES], usize) {
    crate::sched::ipc_send_badged(intake, [read::request(), cursor, 0], badge as u64);
    crate::sched::notification_wait(done).expect("the reader's notification died");
    // SAFETY: a frame this test allocated and owns until the end of the test, named through the
    // direct map. The service wrote it before signalling, and the signal is the ordering point.
    let window: &'static [u8] = unsafe {
        core::slice::from_raw_parts(
            mmu::phys_to_virt(window_phys) as *const u8,
            read::WINDOW_BYTES,
        )
    };
    let (next, len, status) = read::header(window);
    assert_eq!(
        status,
        read::status::OK,
        "the service refused a registered reader"
    );
    let text = core::str::from_utf8(&window[read::DATA..read::DATA + len])
        .expect("the service wrote a line that is not UTF-8");
    let mut lines = [""; MAX_LINES];
    let mut n = 0;
    for line in text.lines() {
        assert!(n < MAX_LINES, "more lines than this test wrote: {text}");
        lines[n] = line;
        n += 1;
    }
    (next, lines, n)
}

/// **Two differently badged writers, one ring, and each line attributed to the writer that wrote
/// it**, read back by the system reader; then the per-user read, which sees only alice's.
///
/// What would make this pass wrongly, and why it cannot: both writers print the same transcript,
/// so the text cannot tell them apart and only the stamp can. The stamp's `source` is the badge
/// the kernel read off each writer's capability, and `user` is what the spawner registered for that
/// badge. A service that read the badge from anywhere else (or a kernel that dropped it on a plain
/// `SEND`, which it did before this milestone) would stamp both lines with badge 0 and no user.
#[test_case]
fn two_badged_writers_are_attributed_by_the_badge_and_a_per_user_read_filters() {
    let Some(service) = program("system_log") else {
        crate::testing::skip!("no system_log program in this archive");
    };
    let writer =
        program("sink_transcript_writer").expect("no sink_transcript_writer in the initrd archive");

    let region = crate::memory_region::create(4).expect("no region for the log's objects");
    let intake = crate::sched::create_rendezvous_from(region).expect("no intake rendezvous");
    let report = crate::sched::create_rendezvous_from(region).expect("no report rendezvous");
    let system_done = crate::sched::create_notification_from(region).expect("no notification");
    let alice_done = crate::sched::create_notification_from(region).expect("no notification");
    let windows = [
        crate::memory::alloc_zeroed().expect("no frame for window 0"),
        crate::memory::alloc_zeroed().expect("no frame for window 1"),
    ];
    let phys = [windows[0].addr(), windows[1].addr()];

    let tid = crate::sched::spawn(move || {
        run(
            service,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(intake, Rights::READ), // slot 0: the intake
                    notification_cap(system_done, Rights::WRITE), // slot 1: window 0's reader
                    notification_cap(alice_done, Rights::WRITE), // slot 2: window 1's reader
                ],
                maps: &[
                    Mapping {
                        va: read::WINDOW_VA,
                        phys: phys[0],
                        flags: Flags::user_data(),
                    },
                    Mapping {
                        va: read::WINDOW_VA + read::WINDOW_BYTES as u64,
                        phys: phys[1],
                        flags: Flags::user_data(),
                    },
                ],
            },
        )
    })
    .expect("could not spawn the system log service");

    // The spawner's half, before any writer holds a capability: so a writer's first line is
    // already attributed.
    for (badge, user) in WRITERS {
        register(intake, badge, b"sink_transcript_writer", user);
    }
    register(intake, SYSTEM_READER, b"kernel_reader", b"");
    control(
        intake,
        (
            control::word(control::OP_READER, control::SCOPE_SYSTEM, 0, SYSTEM_READER),
            0,
            0,
        ),
    );
    register(intake, ALICE_READER, b"kernel_reader", b"alice");
    control(
        intake,
        (
            control::word(control::OP_READER, control::SCOPE_USER, 1, ALICE_READER),
            0,
            0,
        ),
    );

    for (badge, _) in WRITERS {
        spawn_writer(writer, intake, badge, report);
    }
    // Each writer reports after its last `SEND` returned, which is after the service received it;
    // the service handles one message before it receives the next, so by the time both reports
    // are in, every writer byte is in the ring ahead of the read below.
    for _ in WRITERS {
        let [class, total, ..] = crate::sched::ipc_recv(report);
        assert_eq!(
            class,
            fixture::code(byte_sink_protocol::Sent::Ok),
            "a writer's log sink refused it"
        );
        assert_eq!(total as usize, fixture::TRANSCRIPT.len());
    }

    let text = core::str::from_utf8(fixture::TRANSCRIPT)
        .unwrap()
        .trim_end_matches('\n');
    // The comparison below is against the raw text, which holds only while it needs no escaping.
    assert!(
        !text.contains(['"', '\\']),
        "the transcript needs JSON escaping now"
    );
    let (next, all, n) = read_window(intake, SYSTEM_READER, system_done, phys[0], 0);
    let all = &all[..n];
    assert_eq!(n, 2, "the system reader saw {all:?}");
    assert_eq!(next, 2);
    // Both writers printed the same text, so only the stamp tells their lines apart.
    for stamp in [
        r#""source":1,"program":"sink_transcript_writer","user":"alice","severity":"info","msg":""#,
        r#""source":2,"program":"sink_transcript_writer","user":"bob","severity":"info","msg":""#,
    ] {
        let line = all.iter().find(|l| l.contains(stamp));
        let line = line.unwrap_or_else(|| panic!("no line stamped {stamp} in {all:?}"));
        let msg = &line[line.find(stamp).unwrap() + stamp.len()..];
        assert_eq!(msg.strip_suffix("\"}"), Some(text), "{line}");
    }

    let (_, alice, n) = read_window(intake, ALICE_READER, alice_done, phys[1], 0);
    let alice = &alice[..n];
    assert_eq!(n, 1, "alice's read saw {alice:?}");
    assert!(alice[0].contains(r#""user":"alice""#), "{alice:?}");

    // Tear down: reclaiming the region destroys the intake, the service's receive answers `Gone`,
    // and it exits. Only then are its windows free to give back.
    assert!(
        wait_for(|| crate::sched::reclaim_region(region).is_ok()),
        "the log's region would not reclaim"
    );
    assert!(
        wait_for(|| !crate::sched::is_thread_present(tid)),
        "the system log service did not exit when its intake was destroyed"
    );
    for w in windows {
        crate::memory::free(w);
    }
}
