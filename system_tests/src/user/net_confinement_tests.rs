//! **A client of a network stack reaches the sockets it was handed and no others** (milestone 649
//! (every client of a network stack shares its socket numbers), §255 (each socket is its own
//! capability); names here provisional).
//!
//! Until milestone 649 a socket was a small number every client of one `net_stack` shared, so a
//! second client could send on, read, close or attach a page to another client's socket. Milestone
//! 800 (a non-Anthropic model attacks the confinement claim)'s fourth outsider pass booted the
//! sharpest form on all three ISAs, on PR #1798: a squatter that attached its own page at the
//! number a TFTP client was about to use captured that client's traffic, both directions. That
//! test was pinned red and opt-in there. These two are its replacement, in the default suite:
//!
//! - [`a_squatter_at_a_shared_stack_endpoint_cannot_capture_the_clients_traffic`] runs the
//!   squatter (`components/src/socket_squatter.rs`, `net_stack`'s role 8) while an honest client
//!   holds a socket open, and asserts that every way it has to reach that socket is refused, that
//!   the honest exchange still completes, and that nothing of it lands in the squatter's page.
//! - [`a_socket_moves_by_its_capability_and_a_closed_one_reaches_nothing`] hands a socket from one
//!   program to another that holds no front door to the stack at all, has the second use it, and
//!   checks that once it is closed the first program's copy reaches nothing.
//!
//! All three architectures, through the `e1000e` every runner attaches, the route
//! `name_resolver_tests` takes. Each test starts its own stack and gives every region back.

use super::*;
use crate::cap::Rights;
use crate::sched::RendezvousId;
use crate::user::holding::Holding;

/// `net_stack`'s entry roles (`components/src/socket_test_client.rs`, and `socket_squatter::ROLE`).
/// Spelled here rather than imported, the convention `tests.rs` keeps for the same numbers.
const NET_ROLE_SQUATTER: u64 = 8;
const NET_TEST_SOCKET_GIVE: u64 = 9;
const NET_TEST_SOCKET_TAKE: u64 = 10;
const NET_TEST_UDP_TFTP_HELD: u64 = 11;
/// `socket_test_client`'s success word and its `READY`.
const NET_CLIENT_OK: u64 = 1;
const NET_CLIENT_READY: u64 = 0x5EAD;

/// `socket_squatter`'s report words: armed, and the final verdict's quiet world.
const SQUATTER_ARMED: u64 = 1;
const SQUATTER_QUIET: u64 = 0;
/// Every one of the squatter's ten attempts refused (its header lists them in bit order).
const SQUATTER_ALL_REFUSED: u64 = (1 << 10) - 1;

/// Budget pages per spawned client: one frame plus its page tables, `spawn_stack_client`'s number.
const BUDGET_PAGES: u64 = 16;
/// Extra stack pages per client: `spawn_stack_client`'s margin.
const STACK_PAGES: u64 = 6;

/// A capability to the endpoint `ep` with `rights`, and one to the memory region `region`.
type Granted = crate::cap::Cap;
fn endpoint(ep: RendezvousId, rights: Rights) -> Granted {
    crate::cap::rendezvous_cap(ep, rights)
}
fn budget(region: u64) -> Granted {
    crate::cap::memory_region_cap(region)
}

/// Bounded by wall clock, `ntp_tests`' discipline: a lost wakeup fails loudly instead of hanging.
fn wait_for(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + 15 * crate::arch::timer::frequency();
    while crate::arch::timer::now() < deadline {
        if cond() {
            return true;
        }
        crate::sched::yield_now();
    }
    cond()
}

fn net_stack_image() -> &'static [u8] {
    program("net_stack").expect("no net_stack program in the initrd archive")
}

/// Start `net_stack` over the `e1000e` and drain its lease. `None` when no `e1000e` is attached.
fn start_stack() -> Option<(Holding, RendezvousId)> {
    match crate::user::e1000e_service::start_net_server(
        net_stack_image(),
        socket_protocol::NO_LISTEN_GRANT,
    ) {
        Ok(w) => {
            // Drain the lease so the server reaches its serve loop before any client asks.
            let [_lease, ..] = crate::sched::ipc_receive(w.report);
            Some((w.held, w.stack))
        }
        Err(crate::user::e1000e_service::Absent::NoController) => None,
        Err(why) => panic!("the e1000e NIC is on the bus and its bring-up refused: {why:?}"),
    }
}

/// Spawn one `net_stack` role: slot 0 its report endpoint, slot 1 `WRITE` on the stack's front door
/// (`None` leaves it empty, for a program handed a socket and nothing else), slot 2 an untyped
/// budget, then `extra` in slots 3 onward. `spawn_stack_client`'s shape, spelled here because that
/// one is private to the virtio wiring and these tests spawn several clients of one stack.
fn spawn_client(
    arg0: u64,
    stack: Option<RendezvousId>,
    extra: &[Granted],
    held: &mut Holding,
) -> RendezvousId {
    let eps = crate::memory_region::create(2).expect("no endpoint region for the client");
    let report = crate::sched::create_rendezvous_from(eps).expect("no client report endpoint");
    let budget_region =
        crate::memory_region::create(BUDGET_PAGES).expect("no untyped for the client");
    let stack_region = crate::memory_region::create(STACK_PAGES).expect("no stack region");
    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; STACK_PAGES as usize];
    for (k, m) in maps.iter_mut().enumerate() {
        m.phys = crate::memory_region::retype_page(stack_region).expect("no frame for the client");
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
    }
    let mut grants = [endpoint(report, Rights::WRITE); 6];
    if let Some(stack) = stack {
        grants[1] = endpoint(stack, Rights::WRITE);
    }
    grants[2] = budget(budget_region);
    grants[3..3 + extra.len()].copy_from_slice(extra);
    let n = 3 + extra.len();
    let image = net_stack_image();
    let tid = crate::sched::spawn(move || {
        // With no front door, slot 1 stays empty: every other capability is placed at its name, and
        // `run` grants nothing more.
        let in_order: &[Granted] = if stack.is_some() {
            &grants[..n]
        } else {
            for (slot, &granted) in grants[..n].iter().enumerate() {
                if slot != 1 {
                    crate::sched::grant_at(slot as u64, granted)
                        .expect("a client slot was already occupied");
                }
            }
            &[]
        };
        run(
            image,
            Spawn {
                arg0,
                arg1: 0,
                arg2: 0,
                grants: in_order,
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the net client");
    held.add_thread(tid);
    held.add_region(eps);
    held.add_region_after_death(budget_region);
    held.add_region_after_death(stack_region);
    report
}

/// A report, or a failure that names what is still blocked.
fn next_report(report: RendezvousId, what: &str) -> [u64; 5] {
    assert!(
        wait_for(|| crate::sched::rendezvous_waiting_senders(report) > 0),
        "the {what} never reported: it, the stack or the peer is blocked",
    );
    crate::sched::ipc_receive(report)
}

/// **A second client of the stack cannot reach a socket another client opened.**
///
/// The honest TFTP client opens its socket, attaches its page, says so, and waits. While that
/// socket is open, the squatter, holding exactly what an honest client holds, tries everything it
/// has: its page through the front door (the move that captured the traffic before milestone 649),
/// each socket operation on the front door, minting a socket's capability itself, re-badging its
/// own, and its own socket after closing it. Every one must be refused. Then the honest client runs
/// its exchange, which must complete, and nothing of it may reach the squatter's page.
///
/// The name is the one milestone 800 (a non-Anthropic model attacks the confinement claim)'s pinned
/// red test carried on PR #1798, kept so the record of
/// that escape leads here.
///
/// Falsification: replayable `system_tests/falsifications/user.net_confinement_tests.a_squatter_at_a_shared_stack_endpoint_cannot_capture_the_clients_traffic.patch`
#[test_case]
fn a_squatter_at_a_shared_stack_endpoint_cannot_capture_the_clients_traffic() {
    let Some((mut held, stack)) = start_stack() else {
        crate::testing::skip!("no e1000e NIC attached");
    };
    let go_region = crate::memory_region::create(1).expect("no region for the go endpoint");
    let go = crate::sched::create_rendezvous_from(go_region).expect("no go endpoint");

    // The victim first: its socket is open, with its page, before the squatter starts.
    let client = spawn_client(
        NET_TEST_UDP_TFTP_HELD,
        Some(stack),
        &[endpoint(go, Rights::READ)],
        &mut held,
    );
    let ready = next_report(client, "TFTP client");
    assert_eq!(
        ready[0], NET_CLIENT_READY,
        "the client could not open its socket: word {:#x}",
        ready[0]
    );

    let squatter = spawn_client(NET_ROLE_SQUATTER, Some(stack), &[], &mut held);
    let armed = next_report(squatter, "squatter");
    assert_eq!(
        armed[0], SQUATTER_ARMED,
        "the squatter did not arm: word {:#x}",
        armed[0]
    );
    assert_eq!(
        armed[1],
        SQUATTER_ALL_REFUSED,
        "CONFINEMENT ESCAPE: of the squatter's ten attempts on a socket it was not handed, these \
         were NOT refused: {:#012b} (bits: SENDTO, SEND, CONNECT, ACCEPT, CLOSE, RECEIVE on the \
         front door; minting with BADGE; re-badging its own; its closed socket; opening through \
         its closed socket)",
        !armed[1] & SQUATTER_ALL_REFUSED,
    );

    // Now the honest exchange, while the squatter watches its page.
    crate::sched::ipc_send(go, [1, 0, 0]);
    let verdict = next_report(client, "TFTP client");
    assert_eq!(
        verdict[0], NET_CLIENT_OK,
        "the honest client's exchange failed while the squatter held the same stack: {:#x}",
        verdict[0]
    );
    let saw = next_report(squatter, "squatter (final)");
    assert_eq!(
        saw[0], SQUATTER_QUIET,
        "CONFINEMENT ESCAPE: the client's traffic reached the squatter's page (opcode and length \
         {:#x}). A second client of the stack, holding nothing but its front door, read another \
         client's exchange.",
        saw[1],
    );
    held.release_or_fail("the squatter net test's clients");
    crate::sched::reclaim_region(go_region).expect("the go endpoint's region did not come back");
}

/// **A socket moves by passing its capability, and a closed socket's capability reaches nothing.**
///
/// The giver opens a UDP socket and sends it, `WRITE` only, to the taker, which holds no front door
/// to the stack at all: its slot 1 is empty. The taker attaches its own page and runs the TFTP
/// round trip through the socket it was handed, then closes it. The giver kept its own copy, and
/// once the taker has closed the socket that copy must reach nothing.
///
/// Falsification: replayable `system_tests/falsifications/user.net_confinement_tests.a_socket_moves_by_its_capability_and_a_closed_one_reaches_nothing.patch`
#[test_case]
fn a_socket_moves_by_its_capability_and_a_closed_one_reaches_nothing() {
    let Some((mut held, stack)) = start_stack() else {
        crate::testing::skip!("no e1000e NIC attached");
    };
    let region = crate::memory_region::create(2).expect("no region for the hand-off endpoints");
    let handoff = crate::sched::create_rendezvous_from(region).expect("no hand-off endpoint");
    let go = crate::sched::create_rendezvous_from(region).expect("no go endpoint");

    let giver = spawn_client(
        NET_TEST_SOCKET_GIVE,
        Some(stack),
        &[endpoint(handoff, Rights::WRITE), endpoint(go, Rights::READ)],
        &mut held,
    );
    let taker = spawn_client(
        NET_TEST_SOCKET_TAKE,
        None,
        &[endpoint(handoff, Rights::READ)],
        &mut held,
    );

    let ready = next_report(giver, "giver");
    assert_eq!(
        ready[0], NET_CLIENT_READY,
        "the giver could not open or send its socket: word {:#x}",
        ready[0]
    );
    let used = next_report(taker, "taker");
    assert_eq!(
        used[0], NET_CLIENT_OK,
        "a program handed a socket, and no front door, could not use it: word {:#x}",
        used[0]
    );

    // The taker closed the socket on its way out; now the giver's copy.
    crate::sched::ipc_send(go, [1, 0, 0]);
    let stale = next_report(giver, "giver (stale copy)");
    assert_eq!(
        stale[0], NET_CLIENT_OK,
        "a copy of a closed socket's capability still reached a socket: word {:#x}",
        stale[0]
    );
    held.release_or_fail("the socket hand-off test's clients");
    crate::sched::reclaim_region(region).expect("the hand-off endpoints' region did not come back");
}
