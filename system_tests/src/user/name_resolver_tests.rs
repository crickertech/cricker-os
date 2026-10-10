//! **The name resolver, end to end, and the grant that bounds what a client may ask** (milestone 384
//! (in a capability system the resolver is a grant); §248 (the name resolver is its own confined
//! program)).
//!
//! The kernel plays the spawner: it starts `net_stack` over the `e1000e` NIC, starts
//! `name_resolver` as a client of that stack, grants one badge the zone `nife.test`, and spawns
//! `name_resolver_test_client` holding nothing of the resolver but that badged capability. The
//! client asks about every name in `name_resolution_protocol::fixture::CASES` and reports each
//! reply, and this file judges them. The name server is the runners' `helpers/name-server-peer` at
//! 10.0.2.9:53, over TCP, because slirp forwards nothing else to a host process.
//!
//! **One module for all three architectures**, over the `e1000e` because every runner attaches one
//! (milestone 494 (a driver for the network card a PC actually has)), which is what makes this the
//! first name resolution the x86_64 leg has run.
//!
//! What a pass is not: a claim about the UDP path. The gate asks over TCP, the resolver's UDP
//! exchange is the same `Query` bytes and the same `accept`, and that is proved on the host in
//! `crates/domain_name_system`; a UDP name server a test owned would have to bind a host port.

use name_resolution_protocol::fixture::{self, CASES};
use name_resolution_protocol::{Outcome, endowment, grant_messages, status};

use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap, rendezvous_cap_badged};
use crate::sched::RendezvousId;
use crate::user::holding::Holding;

/// The badge the granted client's capability carries, and the one the second client's carries,
/// which nobody grants anything.
const GRANTED: u32 = 0x384;
const UNGRANTED: u32 = 0x385;

/// Budget pages: each program mints at most two pages and pays for their page tables, and the
/// resolver maps up to `GRANTS_MAX` client pages besides.
const BUDGET_PAGES: u64 = 16;
/// Extra stack pages for the client, the margin `network_time_client` gets.
const CLIENT_STACK_PAGES: u64 = 3;
/// Extra stack pages for the resolver. Measured rather than chosen: with three, the debug build
/// faulted below its stack on its first resolve (2026-10-06 UTC, aarch64), because an unoptimized
/// `Query::accept` and the `Name`s around it keep several 256-byte copies live at once.
const RESOLVER_STACK_PAGES: u64 = 8;
/// The larger of the two, which sizes the one array `stack_pages` fills.
const STACK_PAGES_MAX: usize = RESOLVER_STACK_PAGES as usize;

/// Bounded by wall clock, as `ntp_tests`' is, so a client that never reports fails with a sentence
/// rather than the 60 s watchdog. Twenty seconds, because one case is a TCP connection to a host
/// process slirp starts, and the slowest leg is riscv64 under TCG.
fn wait_for(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + 20 * crate::arch::timer::frequency();
    while crate::arch::timer::now() < deadline {
        if cond() {
            return true;
        }
        crate::sched::yield_now();
    }
    cond()
}

fn image(name: &str) -> &'static [u8] {
    program(name).unwrap_or_else(|| panic!("no {name} program in the initrd archive"))
}

/// The entropy service's request endpoint, from whichever source this machine has: virtio-rng on
/// the two `virt` boards, the instruction on x86_64's `q35`. The first `ensure` is handed the
/// service's readiness report and must drain it, or the service parks in its own startup
/// (`ntp_tests`' `machine_has_no_entropy` has the story).
pub(super) fn entropy() -> Option<RendezvousId> {
    let image = image("entropy");
    for bus in [
        entropy_service::Bus::Mmio,
        entropy_service::Bus::Instruction,
    ] {
        if let Some(w) = entropy_service::ensure(image, bus) {
            if let Some(report) = w.wait_for_ready() {
                assert_eq!(
                    report[0],
                    entropy_protocol::READY,
                    "the entropy service did not come up (it reported {:#x})",
                    report[0],
                );
            }
            return Some(w.request);
        }
    }
    None
}

/// `pages` zeroed pages below the stack top, from a region `held` hands back once the program is
/// dead. The array is the largest size; only the first `pages` entries are mappings.
fn stack_pages(pages: u64, held: &mut Holding) -> [Mapping; STACK_PAGES_MAX] {
    let region = crate::memory_region::create(pages).expect("no stack region");
    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; STACK_PAGES_MAX];
    for (k, m) in maps.iter_mut().take(pages as usize).enumerate() {
        m.phys = crate::memory_region::retype_page(region).expect("no stack page");
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
    }
    held.add_region_after_death(region);
    maps
}

/// **Start the resolver on `stack`**, told to ask `fixture::SERVER` on port 53 over TCP. Returns its
/// endpoint, from a region of its own in `held`: the resolver blocks there between requests, and
/// reclaiming that region is what wakes it to die at release.
pub(super) fn start_resolver(
    stack: RendezvousId,
    entropy: RendezvousId,
    held: &mut Holding,
) -> RendezvousId {
    let region = crate::memory_region::create(1).expect("no endpoint region for the resolver");
    let service = crate::sched::create_rendezvous_from(region).expect("no resolver endpoint");
    let budget = crate::memory_region::create(BUDGET_PAGES).expect("no untyped for the resolver");
    let maps = stack_pages(RESOLVER_STACK_PAGES, held);
    let image = image("name_resolver");
    let tid = crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: endowment::server(fixture::SERVER),
                arg1: endowment::port_and_transport(
                    domain_name_system::PORT,
                    endowment::TRANSPORT_TCP,
                ),
                // Unused since the resolver's sockets became capabilities (§255 (each socket is its
                // own capability)).
                arg2: 0,
                grants: &[
                    rendezvous_cap(service, Rights::READ), // slot 0: grants and requests
                    rendezvous_cap(stack, Rights::WRITE),  // slot 1: the network
                    memory_region_cap(budget),             // slot 2: the pages
                    rendezvous_cap(entropy, Rights::WRITE), // slot 3: transaction ids
                ],
                maps: &maps[..RESOLVER_STACK_PAGES as usize],
            },
        )
    })
    .expect("could not spawn name_resolver");
    held.add_thread(tid);
    held.add_region(region);
    held.add_region_after_death(budget);
    service
}

/// Play the spawner's half of a grant: the messages, through the unbadged capability (a kernel
/// `SEND` carries badge 0).
pub(super) fn grant(service: RendezvousId, badge: u32, zone: &str) {
    for (w0, w1) in grant_messages(badge, zone.as_bytes()).expect("a grantable zone") {
        crate::sched::ipc_send(service, [w0, w1, 0]);
    }
}

/// Spawn the test client holding `badge` on the resolver and, if `stack` is given, the network.
/// Returns its report endpoint.
fn start_client(
    service: RendezvousId,
    badge: u32,
    stack: Option<RendezvousId>,
    held: &mut Holding,
) -> RendezvousId {
    let region = crate::memory_region::create(1).expect("no endpoint region for the client");
    let report = crate::sched::create_rendezvous_from(region).expect("no client report endpoint");
    let budget = crate::memory_region::create(BUDGET_PAGES).expect("no untyped for the client");
    let maps = stack_pages(CLIENT_STACK_PAGES, held);
    let image = image("name_resolver_test_client");
    let tid = crate::sched::spawn(move || {
        let grants = [
            rendezvous_cap(report, Rights::WRITE), // slot 0
            rendezvous_cap_badged(service, Rights::WRITE, badge as u64), // slot 1
            memory_region_cap(budget),             // slot 2
            rendezvous_cap(stack.unwrap_or(report), Rights::WRITE), // slot 3
        ];
        run(
            image,
            Spawn {
                arg0: if stack.is_some() {
                    fixture::WITH_NETWORK
                } else {
                    0
                },
                arg1: 0,
                arg2: 0,
                // No stack, no slot 3: the client is then a program that cannot reach the network
                // at all, rather than one holding a capability it was told not to use.
                grants: if stack.is_some() {
                    &grants[..]
                } else {
                    &grants[..3]
                },
                maps: &maps[..CLIENT_STACK_PAGES as usize],
            },
        )
    })
    .expect("could not spawn name_resolver_test_client");
    held.add_thread(tid);
    held.add_region(region);
    held.add_region_after_death(budget);
    report
}

/// The client's next report, or a failure that says which case it was stuck on.
fn next_report(report: RendezvousId, expecting: usize) -> [u64; 3] {
    assert!(
        wait_for(|| crate::sched::rendezvous_waiting_senders(report) > 0),
        "the test client never reported case {expecting}: it, the resolver or the stack is blocked",
    );
    let m = crate::sched::ipc_receive(report);
    assert_ne!(
        m[0],
        u64::MAX,
        "the test client could not attach its page to the resolver"
    );
    assert_eq!(
        m[0], expecting as u64,
        "the test client reported out of order"
    );
    [m[0], m[1], m[2]]
}

/// **A client granted `nife.test` gets exactly what the zone's name server says, refuses every lie
/// for its own reason, is denied every name outside the zone without a query, and connects to what
/// it resolved; a client with no grant gets nothing.**
///
/// The first half is milestone 384's gate: the resolver is a confined program holding a stack
/// endpoint and an entropy endpoint, the client holds a badged endpoint to it and a stack endpoint,
/// and the badge's zone decides which names the client may ask about. `example.com` and
/// `evilnife.test` come back `DENIED`, where a resolver that asked would have got the peer's
/// `REFUSED` (a `SERVER_ERROR`), which is how this test tells "never asked" from "asked and lost".
///
/// The second half is the confinement claim on its own: the same program, with a badge nobody
/// granted, gets `DENIED` for every name, including the ones the first client resolved.
///
/// Falsification: replayable `system_tests/falsifications/user.name_resolver_tests.a_granted_client_resolves_inside_its_zone_and_nothing_outside_it.patch`
#[test_case]
fn a_granted_client_resolves_inside_its_zone_and_nothing_outside_it() {
    let Some(entropy) = entropy() else {
        crate::testing::skip!(
            "no entropy source on this machine, and the resolver asks nothing without one"
        );
    };
    let mut w = match e1000e_service::start_net_server(
        image("net_stack"),
        socket_protocol::NO_LISTEN_GRANT,
    ) {
        Ok(w) => w,
        Err(e1000e_service::Absent::NoController) => {
            crate::testing::skip!("no e1000e NIC attached");
        }
        Err(why) => panic!("the e1000e NIC is on the bus and its bring-up refused: {why:?}"),
    };
    // The lease, drained so `net_stack` enters its serve loop. Its second word is the name server
    // DHCP named (slirp's 10.0.2.3, which forwards to the host's resolver); the resolver here is
    // pointed at the runners' own peer instead, because that is the zone the cases are written
    // against. Which server to ask is the spawner's decision either way.
    let [_lease, nameserver, ..] = crate::sched::ipc_receive(w.report);
    assert_eq!(
        socket_protocol::lease::word_ipv4(nameserver),
        Some([10, 0, 2, 3]),
        "the lease did not name slirp's name server, so a spawner would have nothing to hand on",
    );

    let service = start_resolver(w.stack, entropy, &mut w.held);
    grant(service, GRANTED, fixture::ZONE);
    let report = start_client(service, GRANTED, Some(w.stack), &mut w.held);

    for (i, case) in CASES.iter().enumerate() {
        let [_, word, address] = next_report(report, i);
        let got = Outcome::from_word(word)
            .unwrap_or_else(|| panic!("{}: the resolve call failed with {word:#x}", case.name));
        assert_eq!(
            (got.status, got.detail),
            (case.status, case.detail),
            "{}: the resolver answered status {} detail {} (name_resolution_protocol::status)",
            case.name,
            got.status,
            got.detail,
        );
        if case.status == status::OK {
            assert_eq!(got.count, 1, "{}: one address in the zone", case.name);
            assert_eq!(
                endowment::server_octets(address),
                fixture::SERVER,
                "{}: resolved to the wrong address",
                case.name,
            );
        }
    }
    let [_, echo, _] = next_report(report, CASES.len());
    assert_eq!(
        echo,
        fixture::ECHO_OK,
        "the client resolved packages.nife.test and could not connect to it (verdict {echo})",
    );

    let report = start_client(service, UNGRANTED, None, &mut w.held);
    for (i, case) in CASES.iter().enumerate() {
        let [_, word, _] = next_report(report, i);
        assert_eq!(
            Outcome::from_word(word).map(|o| o.status),
            Some(status::DENIED),
            "{}: a client with no grant was not denied (reply {word:#x})",
            case.name,
        );
    }
    let [_, echo, _] = next_report(report, CASES.len());
    assert_eq!(echo, fixture::ECHO_SKIPPED);

    w.held
        .release_or_fail("net_stack, name_resolver and its clients");
}

/// The badge `std_resolve`'s capability carries, granted the same zone as the native client's.
const GRANTED_STD: u32 = 0x801;

/// What `std_resolve` prints with a grant, in order. The addresses and kinds are std's own
/// spelling of the fixture's cases: the zone's address on the echo port, NXDOMAIN as `NotFound`,
/// and a name outside the zone as `PermissionDenied`, which only the grant can produce.
const STD_GRANTED: &[&str] = &[
    "std_resolve start",
    "lookup packages.nife.test: 10.0.2.9:7777",
    "lookup nosuch.nife.test: NotFound",
    "lookup example.com: PermissionDenied",
    "echo by name ok",
    "std_resolve done",
];

/// And with no resolver at `RESOLVER_SLOT`: every name is `Unsupported`, including the connect.
const STD_UNGRANTED: &[&str] = &[
    "std_resolve start",
    "lookup packages.nife.test: Unsupported",
    "lookup nosuch.nife.test: Unsupported",
    "lookup example.com: Unsupported",
    "echo by name: Unsupported",
    "std_resolve done",
];

/// Run `std_resolve` over `stack`, with `resolver` at its slot or none, and check its transcript.
fn std_resolve_prints(
    program_image: &'static [u8],
    stack: RendezvousId,
    resolver: Option<(RendezvousId, u32)>,
    want: &[&str],
) {
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let faults_before = USER_FAULTS.load(Ordering::Relaxed);
    let spawned = std_service::start_networked_resolving(
        program_image,
        image("clock"),
        image("entropy"),
        stack,
        resolver,
    );
    let run = &spawned.run;
    let mut got = [0u8; 1024];
    let len = super::std_tests::drain_sink(run.report, &mut got, "std_resolve");
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    std_resolve printed:\n{text}");
    let mut from = 0;
    for line in want {
        match text[from..].find(line) {
            Some(at) => from += at + line.len(),
            None => panic!("std_resolve never printed `{line}` (after what came before it)"),
        }
    }
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(run.thread)),
        "std_resolve never left",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults_before,
        "std_resolve trapped instead of exiting",
    );
    let _ = crate::sched::reclaim_region(spawned.frames);
    run.give_back("std_resolve");
}

/// **A `std` program's `ToSocketAddrs` resolves exactly the zone its resolver badge was granted,
/// and nothing at all without one** (milestone 801 (packages over the internet), item 3).
///
/// The same resolver, zone and peer as the test above, with the std PAL's `lookup_host` as the
/// client instead of a native one. With the badge, the zone's name resolves to the peer and a
/// connection by name echoes; NXDOMAIN reads as `NotFound`; `example.com` reads as
/// `PermissionDenied`, which only the grant can say, since the peer would have answered `REFUSED`.
/// Without it, the PAL finds the slot empty and answers `Unsupported` before minting a page.
///
/// Falsification: replayable `system_tests/falsifications/user.name_resolver_tests.a_std_program_resolves_its_granted_zone_and_nothing_without_a_grant.patch`
#[test_case]
fn a_std_program_resolves_its_granted_zone_and_nothing_without_a_grant() {
    let Some(std_resolve) = program("std_resolve") else {
        crate::testing::skip!(std_service::NO_STD_EXERCISER);
    };
    let Some(entropy) = entropy() else {
        crate::testing::skip!("no entropy source on this machine, and the resolver needs one");
    };
    let mut w = match e1000e_service::start_net_server(
        image("net_stack"),
        socket_protocol::NO_LISTEN_GRANT,
    ) {
        Ok(w) => w,
        Err(e1000e_service::Absent::NoController) => {
            crate::testing::skip!("no e1000e NIC attached");
        }
        Err(why) => panic!("the e1000e NIC is on the bus and its bring-up refused: {why:?}"),
    };
    let _ = crate::sched::ipc_receive(w.report);
    let service = start_resolver(w.stack, entropy, &mut w.held);
    grant(service, GRANTED_STD, fixture::ZONE);

    std_resolve_prints(
        std_resolve,
        w.stack,
        Some((service, GRANTED_STD)),
        STD_GRANTED,
    );
    std_resolve_prints(std_resolve, w.stack, None, STD_UNGRANTED);

    w.held
        .release_or_fail("net_stack and name_resolver, for std_resolve");
}
