//! **The net server: smoltcp as a userspace TCP/IP stack over the confined NIC** (milestone 30,
//! piece 3).
//!
//! The networking form of the userspace-reuse thesis: the kernel confines the NIC's DMA (pieces 1
//! and 2), and a real, reused TCP/IP stack (smoltcp, not hand-built) runs it entirely at EL0. The
//! kernel knows nothing about TCP, UDP, or DHCP; it owns only the DMA confinement.
//!
//! `net_stack` brings the NIC up, runs DHCP to completion (reporting the lease), then serves a
//! capability-shaped socket contract on a `Stack` endpoint (DECISIONS §25, notes/net.md,
//! `crates/socket_protocol/src/lib.rs`): a socket is a capability this server mints and names by its
//! badge (§255 (each socket is its own capability)), per-connection bytes cross in a shared frame
//! the client delegates, and every operation is one message. Phase one is single-threaded and
//! synchronous, one exchange per request; the server blocks on the `Stack` endpoint between
//! requests and drives the network inside handling one.
//!
//! # Capability contract
//! - slot 0: the report endpoint (WRITE) for the DHCP lease
//! - slot 1: the NIC interrupt (WAIT / ACK)
//! - slot 2: the confined `Virtio` transport
//! - slot 3: an untyped budget, for the heap and for mapping clients' shared frames
//! - slot 4: the `Stack` endpoint (READ), where clients' requests arrive
//! - slot 5: the retransmit notification, bound to this thread (absent from the progenitor's spawn)
//! - slot 6: the retransmit timer (likewise)
//! - slot 7: the `Stack` endpoint again (WRITE | GRANT), which each socket is minted from
//! - slot 8: this process's own address space (WRITE), to unmap a closed socket's page
//! - arg1: the DMA page's physical address
//! - arg2: the **grant word**: the TCP listen range (milestone 107) in its low half and the
//!   fixed-UDP bind range (milestone 55's stack half) in its high half, both packed by
//!   `socket_protocol`. Zero, the default, means no port anywhere in either protocol: a stack serves
//!   inbound connections, or claims a fixed UDP port, only when whoever spawned it said which.
//!
//! Name: ratified 2026-07-30 (calef, DECISIONS §39, landed by milestone 46), replacing `netd`, and
//! respelled from `netstack` on 2026-08-01 (milestone 63) because `net` is already this tree's word
//! and the two halves are separate concepts. Refused `netd`, the name DECISIONS §39 was written
//! about: it holds five explicit capabilities, cannot name its own callers, is supervised, and can
//! be reaped by something that lacks the authority to build it, which is about as far from the
//! model that suffix claims as a long-running process gets.
//!
//! # BUGS
//!
//! **A socket id's window is whichever frame arrived at that id first, and `ATTACH` cannot say
//! no** (milestone 800 (a non-Anthropic model attacks the confinement claim), fourth outsider pass,
//! 2026-10-06 UTC). `ATTACH` is a `SEND_CAP`, so it carries no Reply, and the id namespace is a raw
//! client word with no per-caller scope. Two programs holding `WRITE` on one `Stack` endpoint is
//! shipped wiring (`name_resolver_tests` grants one endpoint to a resolver and its client), so a
//! second client can pre-attach its own frame at the first's socket id: the victim's attach fails
//! silently, its socket binds the squatter's window, its `SENDTO` transmits the squatter's bytes
//! and the reply lands in the squatter's frame. Both directions, booted red on all three
//! architectures by the opt-in `net_confinement_tests::
//! a_squatter_at_a_shared_stack_endpoint_cannot_capture_the_clients_traffic` while red. The
//! sibling that does it right is `name_resolver`: windows keyed by the badge's grant index, a
//! second attach refused. Its refusals are silent too, and that is safe there because the
//! scoping makes every attach failure the caller's own. Whether this server's refusals should
//! also become answerable is a separate, smaller call, still open. The fix shape itself is ruled:
//! per-caller windows keyed by the badge, this precedent's exact shape, refusals silent (calef,
//! 2026-10-06, PR #1798). Ruled with it, tree-wide: per-caller scoping is a written rule for every
//! multi-client window server, and the fix lane audits the remaining ones, `system_log`'s reader
//! windows first (not read this pass).
//!
//! **No host fuzz target reaches this dispatch** (proposal #1592 part a, rank 2). The request match
//! and every helper under it live in this EL0 binary and take `virtio_net_transport::VirtioNet`,
//! the clock and two blocking waits directly, so nothing builds for the host. What a host part
//! would cost, measured by reading on 2026-10-04 (UTC): about 630 lines moved into a sans-IO crate
//! over smoltcp's `phy::Device`, with the clock and the waits behind an edge trait. See
//! `notes/fuzzing-the-services.md`.
//!
//! **One client can take every socket.** The table is [`SOCKETS`] entries shared by every client
//! of the stack, and nothing bounds how many one client holds, so a client that opens sixteen and
//! keeps them starves the rest of the network. Before §255 (each socket is its own capability) the
//! same was true of the six shared socket numbers. A per-client quota wants the client named, which
//! a badged front door per client would do; nothing builds that yet. Recorded by milestone 649
//! (every client of a network stack shares its socket numbers).
//!
//! **A spawn without slot 8 cannot reuse a socket's page.** With no address space to `UNMAP` from,
//! a closed socket's page stays mapped, and its table entry refuses every later frame rather than
//! show the next socket the last one's page, so after sixteen sockets with pages the stack carries
//! no more bytes. Every spawner in the tree grants slot 8.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

extern crate alloc;

use alloc::vec;

use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::{dhcpv4, tcp, udp};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address};
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{
    Delivered, Reply, cap_delete, cntfrq, irq_wait, map_page_frame, notification_poll, now,
    receive_request, reply, reply_capability, send, sleep_until, timer_arm, timer_cancel,
};

#[path = "designware_ethernet_transport.rs"]
mod designware_ethernet_transport;
#[path = "e1000e_transport.rs"]
mod e1000e_transport;
#[path = "virtio_net_transport.rs"]
mod virtio_net_transport;
// The socket-contract client rides in this same binary (dispatched by the entry role), because the
// initrd directory holds at most 15 files; see components/src/socket_test_client.rs.
#[path = "socket_test_client.rs"]
mod socket_test_client;
// The hostile second client milestone 649's confinement test runs (role 8), here for the same
// 15-file reason; its header records why that is an exception.
#[path = "socket_squatter.rs"]
mod socket_squatter;
use socket_protocol::*;

const REPORT: u64 = 0;
const IRQ: u64 = 1;
const MEMORY_REGION: u64 = 3;
const STACK: u64 = 4;
/// The lease word an `e1000e` server sends instead of an address when its spawn arguments were
/// not a handoff the kernel could have made. Outside every private /24 a test accepts.
const BAD_HANDOFF: u64 = 0xDEAD_0001;
/// The notification bound to this thread, which the retransmit timer signals (milestone 106 (a wait
/// that ends on either the interrupt or the deadline)). The spawner makes and binds it.
const WAKE: u64 = 5;
/// The retransmit timer: armed for smoltcp's next deadline while this server waits for a frame.
const RETRANSMIT: u64 = 6;
/// Our own endpoint again, `WRITE | GRANT`, and our own address space (§255 (each socket is its own
/// capability)); `socket_protocol::stack_slots` says what each is for.
const MINT: u64 = socket_protocol::stack_slots::MINT;
const OWN_SPACE: u64 = socket_protocol::stack_slots::OWN_SPACE;

/// **How many sockets are open at once, across every client** (§255). A constant of this server's,
/// no longer of the wire: a client never names an entry. Sixteen, from the six the wire allowed,
/// because the table is now shared by every client of the stack rather than divided among them by
/// convention. A socket's real cost, its buffers and its mapped page, is paid only while it is open.
const SOCKETS: usize = 16;

/// The heap smoltcp allocates against, capped under the granted budget.
///
/// **96 pages, down from 128** (milestone 107), and the two numbers are a pair: the spawn service
/// grants `NET_SERVER_BUDGET_PAGES = 128`, and the difference is what pays for the heap's page
/// tables and for mapping clients' shared frames. The budget came down because ten `net_stack`
/// regions are held for the whole boot and never reclaimed, and the aarch64 suite ran out of memory
/// when an eleventh net test arrived; see `kernel/src/user/virtio_service.rs` for the measurement.
/// If either number moves, both must.
const HEAP_MAX: u64 = 96 * 4096;

#[global_allocator]
static HEAP: user_mode_runtime::heap::MemoryRegionHeap =
    user_mode_runtime::heap::MemoryRegionHeap::new();

/// Our MAC. Locally administered; slirp routes DHCP regardless.
const MAC: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

/// Where the shared frame of the socket in table entry `entry` is mapped in `net_stack`'s address
/// space. Above the DMA page (`0x90_0000`) and well below the heap (1 GiB).
fn socket_va(entry: usize) -> u64 {
    0x0000_0000_00A0_0000 + entry as u64 * 0x1000
}

/// smoltcp's clock, from the monotonic counter, in milliseconds.
fn instant() -> Instant {
    let ms = (now() as u128 * 1000 / cntfrq() as u128) as i64;
    Instant::from_millis(ms)
}

/// One open socket: its smoltcp handle, what kind it is, the window onto its shared frame (if it
/// has one), the local port it holds, and the badge its capability carries.
///
/// A **listener** holds a smoltcp socket parked in `Listen` state, has no shared frame (`window ==
/// None`, and it never needs one, because no bytes cross on a listener), and its port is the one it
/// was granted rather than one the allocator handed out.
#[derive(Clone, Copy)]
struct Sock {
    handle: SocketHandle,
    kind: Kind,
    window: Option<MappedWindow>,
    local_port: u16,
    badge: u64,
}

/// What a socket is, which decides what its capability may ask.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Udp,
    Tcp,
    Listener,
}

/// The private ephemeral-port range `net_stack` allocates local ports from, and a **rotating** allocator
/// over it. The local port must be independent of the socket id: deriving it from the id (the
/// original bug, found by the `std::net` PAL) means reopening a just-closed id reuses the exact port,
/// and a TCP connect on a 4-tuple whose slirp flow has not yet cleared stalls in the bounded poll
/// forever. Advancing monotonically hands out a fresh port each open, so a closed connection's port
/// is not reused until the whole range has cycled; ports a live socket still holds are skipped
/// outright. See notes/net/the-outbound-gates.md.
const EPHEMERAL_LO: u16 = 49152;
const EPHEMERAL_HI: u16 = 65535;

struct PortAllocator {
    next: u16,
}

impl PortAllocator {
    fn new() -> Self {
        Self { next: EPHEMERAL_LO }
    }

    /// The next free ephemeral port not currently held by a live socket. Bounded by the range size,
    /// so it always terminates; with only `SOCKETS` sockets ever live, it never exhausts.
    fn alloc(&mut self, table: &Table) -> u16 {
        for _ in 0..=(EPHEMERAL_HI - EPHEMERAL_LO) {
            let port = self.next;
            self.next = if self.next == EPHEMERAL_HI {
                EPHEMERAL_LO
            } else {
                self.next + 1
            };
            if !table.port_held(port) {
                return port;
            }
        }
        self.next
    }
}

/// The largest datagram/segment buffered per socket.
const SOCK_BUF: usize = 2048;

/// Entry role 0 is the net server; any other role runs the socket-contract client (the same
/// binary, so the initrd stays under its 15-file directory limit).
///
/// `a2` is the server's **grant word**, both port authorities in one spawn argument: the TCP
/// listen range in the low half (milestone 107, `socket_protocol::listen_grant`) and the fixed-UDP
/// bind range in the high half (milestone 55's stack half, `socket_protocol::udp_bind_grant`).
/// Whoever spawns the server decides both; a server spawned with zero (`NO_LISTEN_GRANT`, which
/// every outbound-only test uses) refuses every `LISTEN` and every `BIND_UDP`. The client half
/// ignores it.
#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, direct_memory_access_phys: u64, a2: u64) -> ! {
    if role == 0 {
        let dev = Nic::Virtio(virtio_net_transport::VirtioNet::bring_up(
            direct_memory_access_phys,
        ));
        server(dev, a2)
    } else if e1000e::Handoff::is_role(role) {
        // The `e1000e` server (milestone 494 (a driver for the network card a PC actually has)):
        // the role word carries the tag and the MAC address, the second word the DMA region.
        let Some(handoff) = e1000e::Handoff::unpack(role, direct_memory_access_phys) else {
            send(REPORT, BAD_HANDOFF, 0, 0);
            user_mode_runtime::exit();
        };
        server(
            Nic::Gigabit(e1000e_transport::GigabitNic::bring_up(handoff)),
            a2,
        )
    } else if designware_ethernet::Handoff::is_role(role) {
        // radon's Ethernet port (milestone 53 (the board's own peripherals: network and storage on
        // real silicon)), the same two words in the `e1000e` server's shape.
        let Some(handoff) = designware_ethernet::Handoff::unpack(role, direct_memory_access_phys)
        else {
            send(REPORT, BAD_HANDOFF, 0, 0);
            user_mode_runtime::exit();
        };
        server(
            Nic::DesignWare(designware_ethernet_transport::DesignWareNic::bring_up(
                handoff,
            )),
            a2,
        )
    } else if role == socket_squatter::ROLE {
        socket_squatter::run()
    } else {
        // The client's second word is its own (only the package exchange reads it); the DMA page is
        // the server's.
        socket_test_client::run(role, direct_memory_access_phys)
    }
}

/// The net server: bring the NIC up, run DHCP, then serve the socket contract.
fn server(mut dev: Nic, grant_word: u64) -> ! {
    HEAP.init(
        MEMORY_REGION,
        user_mode_runtime::heap::DEFAULT_BASE,
        HEAP_MAX,
    );

    let mut config = Config::new(HardwareAddress::Ethernet(EthernetAddress(dev.mac())));
    config.random_seed = now();
    let mut iface = Interface::new(config, &mut dev, instant());
    let mut sockets = SocketSet::new(vec![]);

    // --- DHCP: bring the interface up, then report the lease (the phase-A test asserts it). ---
    let dhcp = sockets.add(dhcpv4::Socket::new());
    loop {
        iface.poll(instant(), &mut dev, &mut sockets);
        if let Some(dhcpv4::Event::Configured(cfg)) = sockets.get_mut::<dhcpv4::Socket>(dhcp).poll()
        {
            iface.update_ip_addrs(|addrs| {
                let _ = addrs.push(IpCidr::Ipv4(cfg.address));
            });
            if let Some(router) = cfg.router {
                let _ = iface.routes_mut().add_default_ipv4_route(router);
            }
            // The lease report's layout is `socket_protocol::lease`: the address, then the first
            // DNS server the lease named, which a resolver's spawner hands on (§248 (the name
            // resolver is its own confined program)).
            let address = socket_protocol::lease::ipv4_word(cfg.address.address().octets());
            let nameserver = cfg
                .dns_servers
                .first()
                .map_or(socket_protocol::lease::NO_NAMESERVER, |a| {
                    socket_protocol::lease::ipv4_word(a.octets())
                });
            send(REPORT, address, nameserver, 0);
            break;
        }
        // Same discipline as service_until: block on the interrupt only when smoltcp has no timer
        // pending, so a dropped DHCP OFFER is retried by smoltcp's own DISCOVER retransmit rather
        // than deadlocking on an interrupt that will not come.
        wait_for_nic(&mut iface, &mut dev, &mut sockets);
    }

    // No multicast group is joined here. Milestone 55 joined mDNS's 224.0.0.251 at this point, after
    // DHCP so the IGMP report carried a real source address, and milestone 298 removed the join on
    // 2026-09-15 with the responder that was its only reason (notes/mdns.md). A group is interface
    // state rather than socket state, so a future multicast client puts a join back here and
    // smoltcp's `multicast` feature back in components/Cargo.toml.

    // --- Serve the socket contract. One synchronous exchange per request. ---
    let mut table = Table::new();
    let mut ports = PortAllocator::new();
    loop {
        let req = receive_request(STACK);
        let (operation, arg, badge) = (req.w0, req.w1, req.badge);

        // **The one operation that takes a delegation, and every other operation takes a Reply**
        // (milestone 706 (a CALL server can tell a Reply from a delegation), DECISIONS §245 (a
        // `CALL` server tells a Reply from a delegation)). `ATTACH` is a SEND_CAP whose capability
        // is the socket's frame, on that socket's own capability; anything else is a CALL, and a
        // delegation sent with it is deleted here rather than answered into.
        let to = match req.delivered {
            Delivered::Delegation(frame) if operation == OPERATION_ATTACH_PAGE_FRAME => {
                table.attach(badge, frame);
                continue;
            }
            other => other.into_reply(),
        };
        // Nobody to answer: a plain SEND, an ATTACH that carried no frame, or a delegation (now
        // deleted) on an operation that wants a CALL.
        let Some(to) = to else {
            continue;
        };

        // **The front door makes sockets and does nothing else** (§255 (each socket is its own
        // capability)). A badge outside the socket range is a front door, whoever badged it.
        if !names_a_socket(badge) {
            let opened = match operation {
                OPERATION_OPEN_UDP => open_udp(&mut sockets, &mut table, &mut ports),
                OPERATION_OPEN_TCP => open_tcp(&mut sockets, &mut table, &mut ports),
                OPERATION_LISTEN => tcp_listen(&mut sockets, &mut table, arg, grant_word),
                OPERATION_BIND_UDP => udp_bind(&mut sockets, &mut table, arg, grant_word),
                _ => Err(REP_ERR),
            };
            match opened {
                Ok((entry, word)) => table.hand_over(&mut sockets, to, entry, word),
                Err(word) => {
                    reply(to, word, 0);
                }
            }
            continue;
        }

        // A socket's capability. A badge the table does not hold is a closed socket's, or one
        // nobody minted: it reaches nothing.
        let Some(entry) = table.find(badge) else {
            reply(to, REP_ERR, 0);
            continue;
        };
        let rep = match operation {
            OPERATION_SENDTO => udp_sendto(
                &mut iface,
                &mut dev,
                &mut sockets,
                &table,
                entry,
                arg as usize,
            ),
            OPERATION_RECEIVE => sock_receive(&mut iface, &mut dev, &mut sockets, &table, entry),
            OPERATION_CONNECT => tcp_connect(&mut iface, &mut dev, &mut sockets, &table, entry),
            OPERATION_SEND => tcp_send(
                &mut iface,
                &mut dev,
                &mut sockets,
                &table,
                entry,
                arg as usize,
            ),
            OPERATION_CLOSE => {
                table.close(&mut iface, &mut dev, &mut sockets, entry);
                REP_OK
            }
            OPERATION_ACCEPT => {
                match tcp_accept(&mut iface, &mut dev, &mut sockets, &mut table, entry) {
                    Ok(connection) => {
                        table.hand_over(&mut sockets, to, connection, REP_OK);
                        continue;
                    }
                    Err(word) => word,
                }
            }
            // A CALL naming ATTACH carries no frame; anything else is not an operation.
            _ => REP_ERR,
        };
        reply(to, rep, 0);
    }
}

/// **The open sockets of every client, keyed by the badge on each one's capability** (§255 (each
/// socket is its own capability)). An entry is a place in this table and a window VA, never a
/// name any client sees.
struct Table {
    socks: [Option<Sock>; SOCKETS],
    /// Whether entry `i`'s window VA has a page mapped. A closed socket's page is unmapped, and an
    /// entry whose page could not be unmapped never takes another frame, so no socket ever reads
    /// through a page a previous holder of its entry attached.
    mapped: [bool; SOCKETS],
    /// The next badge to mint. Starts in the socket range and only counts up, so no badge is ever
    /// minted twice and a stale copy can never come to name a later socket.
    next_badge: u64,
}

impl Table {
    fn new() -> Self {
        Self {
            socks: [None; SOCKETS],
            mapped: [false; SOCKETS],
            next_badge: SOCKET_BADGE | 1,
        }
    }

    /// The entry holding the socket `badge` names, if one is open.
    fn find(&self, badge: u64) -> Option<usize> {
        self.socks
            .iter()
            .position(|s| s.is_some_and(|s| s.badge == badge))
    }

    /// A free entry, if the table has one.
    fn free(&self) -> Option<usize> {
        self.socks.iter().position(Option::is_none)
    }

    /// Fill `entry` with a socket over `handle` and give it a fresh badge.
    fn install(&mut self, entry: usize, handle: SocketHandle, kind: Kind, local_port: u16) {
        let badge = self.next_badge;
        self.next_badge += 1;
        self.socks[entry] = Some(Sock {
            handle,
            kind,
            window: None,
            local_port,
            badge,
        });
    }

    /// **Answer `to` with `word` and the socket at `entry`'s capability**, minted here from the
    /// `WRITE | GRANT` copy of our own endpoint with the socket's badge. If either step fails the
    /// socket is taken down again and the caller hears `REP_ERR`: a socket nobody holds a
    /// capability for is one nobody could ever close.
    fn hand_over(&mut self, sockets: &mut SocketSet, to: Reply, entry: usize, word: u64) {
        let badge = self.socks[entry].map_or(0, |s| s.badge);
        let minted = user_mode_runtime::badge(MINT, badge);
        if minted < 0 {
            self.drop_entry(sockets, entry);
            reply(to, REP_ERR, 0);
            return;
        }
        let minted = minted as u64;
        match reply_capability(to, word, 0, minted) {
            Ok(true) => {}
            // Answered, but the copy did not land: the client's table was full. Nobody holds the
            // socket, so it goes now rather than fill an entry for good.
            Ok(false) => self.drop_entry(sockets, entry),
            Err((to, _)) => {
                self.drop_entry(sockets, entry);
                reply(to, REP_ERR, 0);
            }
        }
        // Ours goes either way: the client holds its copy now, or there is none to hold.
        cap_delete(minted);
    }

    /// `OPERATION_ATTACH_PAGE_FRAME`: map `frame` writable at the window of the socket `badge`
    /// names, paid for from `net_stack`'s untyped, and drop the capability (the mapping outlives
    /// it). A badge that names no open socket, or names a listener, which carries no bytes, gets
    /// nothing, and the frame is dropped. A second attach replaces the first page.
    fn attach(&mut self, badge: u64, frame: u64) {
        let entry = self
            .find(badge)
            .filter(|&e| self.socks[e].is_some_and(|s| s.kind != Kind::Listener));
        if let Some(entry) = entry
            && self.unmap(entry)
        {
            let va = socket_va(entry);
            if map_page_frame(frame, va, true, MEMORY_REGION) {
                self.mapped[entry] = true;
                if let Some(sk) = self.socks[entry].as_mut() {
                    // SAFETY: the MAP above just mapped one page read/write at `va`, and it stays
                    // mapped until `unmap` takes it back, which clears this window first.
                    sk.window = Some(unsafe { MappedWindow::new(va, PAGE) });
                }
            }
        }
        cap_delete(frame);
    }

    /// Take entry `entry`'s page out of our address space, if it has one. `false` if a page is
    /// there and could not be unmapped (a stack spawned without its own address space), and the
    /// entry then keeps the page and takes no other.
    fn unmap(&mut self, entry: usize) -> bool {
        if let Some(sk) = self.socks[entry].as_mut() {
            sk.window = None;
        }
        if !self.mapped[entry] {
            return true;
        }
        // SAFETY: the syscall; the kernel checks the capability, the right and the address.
        let gone = unsafe {
            user_mode_runtime::invoke(OWN_SPACE, abi::address_space::UNMAP, socket_va(entry), 0, 0)
        } == 0;
        if gone {
            self.mapped[entry] = false;
        }
        gone
    }

    /// **`CLOSE`: end the socket and unbind its badge.** Every copy of its capability, wherever it
    /// went, reaches nothing after this, and its page leaves our address space.
    fn close(
        &mut self,
        iface: &mut Interface,
        dev: &mut Nic,
        sockets: &mut SocketSet,
        entry: usize,
    ) {
        let Some(sk) = self.socks[entry] else { return };
        match sk.kind {
            Kind::Tcp | Kind::Listener => {
                let handle = sk.handle;
                sockets.get_mut::<tcp::Socket>(handle).close();
                if sk.kind == Kind::Tcp {
                    // Drain the close handshake so the peer (and slirp's flow for it) sees a
                    // clean teardown before we drop the socket. Dropping a socket mid-close
                    // leaves the peer's connection half-open, which on the guestfwd echo peer
                    // blocks the next connection to it. Both FINs exchanged (Closed or TimeWait)
                    // is enough; waiting for full Closed would linger in TimeWait's timer.
                    // Bounded, so a peer that never finishes still returns.
                    service_until(iface, dev, sockets, |s| {
                        matches!(
                            s.get_mut::<tcp::Socket>(handle).state(),
                            tcp::State::Closed | tcp::State::TimeWait
                        )
                    });
                }
            }
            Kind::Udp => {
                sockets.get_mut::<udp::Socket>(sk.handle).close();
                iface.poll(instant(), dev, sockets);
            }
        }
        self.drop_entry(sockets, entry);
    }

    /// Forget entry `entry`: its badge, its smoltcp socket, and its page.
    fn drop_entry(&mut self, sockets: &mut SocketSet, entry: usize) {
        if let Some(sk) = self.socks[entry] {
            sockets.remove(sk.handle);
        }
        let _ = self.unmap(entry);
        self.socks[entry] = None;
    }

    /// Whether any open socket holds local `port`.
    fn port_held(&self, port: u16) -> bool {
        self.socks.iter().flatten().any(|s| s.local_port == port)
    }
}

/// `OPERATION_OPEN_UDP`: a UDP socket on an ephemeral port, in a free entry.
fn open_udp(
    sockets: &mut SocketSet,
    table: &mut Table,
    ports: &mut PortAllocator,
) -> Result<(usize, u64), u64> {
    let entry = table.free().ok_or(REP_ERR)?;
    let handle = sockets.add(udp_socket());
    let local_port = ports.alloc(table);
    let _ = sockets.get_mut::<udp::Socket>(handle).bind(local_port);
    table.install(entry, handle, Kind::Udp, local_port);
    Ok((entry, REP_OK))
}

/// `OPERATION_OPEN_TCP`: an unconnected TCP socket with an ephemeral port, in a free entry.
fn open_tcp(
    sockets: &mut SocketSet,
    table: &mut Table,
    ports: &mut PortAllocator,
) -> Result<(usize, u64), u64> {
    let entry = table.free().ok_or(REP_ERR)?;
    let handle = sockets.add(tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; SOCK_BUF]),
        tcp::SocketBuffer::new(vec![0u8; SOCK_BUF]),
    ));
    let local_port = ports.alloc(table);
    table.install(entry, handle, Kind::Tcp, local_port);
    Ok((entry, REP_OK))
}

fn udp_socket() -> udp::Socket<'static> {
    udp::Socket::new(
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 8], vec![0u8; SOCK_BUF]),
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 8], vec![0u8; SOCK_BUF]),
    )
}

/// Read the destination (octets + little-endian port) from a shared frame's header.
fn read_dst(window: MappedWindow) -> IpEndpoint {
    let ip = Ipv4Address::new(
        window.r8(OFF_DST_IP),
        window.r8(OFF_DST_IP + 1),
        window.r8(OFF_DST_IP + 2),
        window.r8(OFF_DST_IP + 3),
    );
    let port = window.r16(OFF_DST_PORT);
    IpEndpoint::new(IpAddress::Ipv4(ip), port)
}

/// **Wait for the NIC to need attention again, without stalling smoltcp's own timers.**
///
/// The obvious loop, "block on the NIC interrupt, service on each wakeup," has a hole that SMP timing
/// on riscv exposed and that a lone core hid: smoltcp drives TCP retransmits, delayed ACKs, and DNS
/// timeouts from *its* clock, advanced only when we call `poll`. If we block on the interrupt alone
/// and the exchange is waiting on one of those timers (a segment we must retransmit because its ACK
/// was dropped), no RX interrupt is coming (the peer is waiting for that retransmit), so the block
/// never returns and the whole pipeline deadlocks with every core idle. aarch64 happened never to
/// drop a segment and so never hit it; riscv under the SMP scatter did. See notes/net/the-outbound-gates.md.
///
/// So: ask smoltcp when it next needs to run. When it has **no** timer pending (`poll_delay` is
/// `None`), block on the interrupt, which is the common, efficient case (0% CPU until a frame
/// arrives), and correct, because with nothing of our own to send we are purely waiting on the peer,
/// whose own retransmit will wake us. When it **does** have a timer pending, do not block: yield and
/// let the caller re-`poll`, so the timer actually fires. That confines the busy interval to the
/// short retransmit window, not the whole exchange.
fn wait_for_nic(iface: &mut Interface, dev: &mut Nic, sockets: &mut SocketSet) {
    match dev {
        Nic::Gigabit(nic) => return poll_without_interrupt(iface, nic.frame_waiting(), sockets),
        Nic::DesignWare(nic) => {
            return poll_without_interrupt(iface, nic.frame_waiting(), sockets);
        }
        Nic::Virtio(_) => {}
    }
    let Some(delay) = iface.poll_delay(instant(), sockets) else {
        irq_wait(IRQ);
        dev.ack_irq();
        return;
    };
    // A smoltcp timer is due in `delay`: wait for a frame **or** that deadline, whichever comes
    // first (milestone 106 (a wait that ends on either the interrupt or the deadline)). The
    // notification in `WAKE` is bound to this thread, so `Irq::WAIT` ends on either and the hart is
    // idle in between. Until 2026-09-26 this yielded and re-polled instead, spinning a hart through
    // every retransmit backoff (`notes/timed-wait.md`, section 5).
    let micros = delay.total_micros();
    let ticks = abi::timer::counter_ticks_for(
        micros / 1_000_000,
        ((micros % 1_000_000) * 1000) as u32,
        cntfrq(),
    );
    if timer_arm(RETRANSMIT, now().saturating_add(ticks), WAKE, 1) == 0 {
        irq_wait(IRQ); // 1: a frame; BOUND: the deadline
        // Whichever ended the wait, leave nothing behind: disarm, and clear a fire that raced the
        // frame, so a stale deadline never ends the next `receive_cap(STACK)` as a bound delivery.
        if timer_cancel(RETRANSMIT) == 0 {
            notification_poll(WAKE);
        }
    } else {
        // No timer granted (a spawner that predates milestone 106): the old yield, so the caller
        // re-polls and smoltcp's timer still fires, at a hart's cost.
        user_mode_runtime::yield_now();
    }
    dev.ack_irq();
}

/// **A polled NIC's wait, which is a sleep** (milestone 494 (a driver for the network card a PC
/// actually has)): the `e1000e` server's, and since milestone 53 (the board's own peripherals:
/// network and storage on real silicon) radon's Ethernet port's, both 1 ms. Neither holds an
/// interrupt (`kernel/src/user/e1000e_service.rs` says why), so the wait ends at smoltcp's own
/// deadline or after `e1000e_transport::POLL_MS`, whichever is sooner, and the caller looks at the
/// ring again. A frame already waiting skips the sleep. The timer and notification are the same
/// pair the virtio server's retransmit wait uses (slots 5 and 6).
fn poll_without_interrupt(iface: &mut Interface, frame_waiting: bool, sockets: &mut SocketSet) {
    if frame_waiting {
        return;
    }
    let poll_micros = e1000e_transport::POLL_MS * 1000;
    let micros = iface
        .poll_delay(instant(), sockets)
        .map_or(poll_micros, |d| d.total_micros().min(poll_micros));
    if micros == 0 {
        return;
    }
    let ticks = abi::timer::counter_ticks_for(0, (micros * 1000) as u32, cntfrq());
    if sleep_until(RETRANSMIT, WAKE, now().saturating_add(ticks)) < 0 {
        user_mode_runtime::yield_now();
    }
}

/// **The NIC this server drives**: virtio-net (milestone 30 (the network stack as a confined
/// component)) or the `e1000e` family (milestone 494). One enum rather than a generic parameter,
/// so the dispatch below is compiled once.
enum Nic {
    Virtio(virtio_net_transport::VirtioNet),
    Gigabit(e1000e_transport::GigabitNic),
    DesignWare(designware_ethernet_transport::DesignWareNic),
}

impl Nic {
    fn mac(&self) -> [u8; 6] {
        match self {
            Nic::Virtio(_) => MAC,
            Nic::Gigabit(n) => n.mac(),
            Nic::DesignWare(n) => n.mac(),
        }
    }
    fn ack_irq(&self) {
        if let Nic::Virtio(v) = self {
            v.ack_irq();
        }
    }
    fn rx_take(&mut self) -> Option<alloc::vec::Vec<u8>> {
        match self {
            Nic::Virtio(v) => v.rx_take(),
            Nic::Gigabit(n) => n.rx_take(),
            Nic::DesignWare(n) => n.rx_take(),
        }
    }
    fn tx_send(&mut self, frame: &[u8]) {
        match self {
            Nic::Virtio(v) => v.tx_send(frame),
            Nic::Gigabit(n) => n.tx_send(frame),
            Nic::DesignWare(n) => n.tx_send(frame),
        }
    }
}

impl smoltcp::phy::Device for Nic {
    type RxToken<'a> = NicRxToken;
    type TxToken<'a> = NicTxToken;

    fn receive(&mut self, _timestamp: Instant) -> Option<(NicRxToken, NicTxToken)> {
        let frame = self.rx_take()?;
        Some((NicRxToken { frame }, NicTxToken { dev: self }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<NicTxToken> {
        Some(NicTxToken { dev: self })
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        let mut caps = smoltcp::phy::DeviceCapabilities::default();
        caps.medium = smoltcp::phy::Medium::Ethernet;
        caps.max_transmission_unit = match self {
            Nic::Virtio(_) => virtio_net_transport::MTU,
            Nic::Gigabit(_) => e1000e::MAX_FRAME,
            Nic::DesignWare(_) => designware_ethernet::MAX_FRAME,
        };
        caps
    }
}

struct NicRxToken {
    frame: alloc::vec::Vec<u8>,
}

impl smoltcp::phy::RxToken for NicRxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        f(&self.frame)
    }
}

struct NicTxToken {
    dev: *mut Nic,
}

impl smoltcp::phy::TxToken for NicTxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buf = vec![0u8; len];
        let r = f(&mut buf);
        // SAFETY: single-threaded; the device outlives this token, and the `&mut self` borrow that
        // produced the token has ended (the token is an owned value). `virtio_net_transport`'s own
        // token made the same argument before this enum took its place.
        unsafe {
            (*self.dev).tx_send(&buf);
        }
        r
    }
}

/// Drive the poll loop until `cond` holds, servicing the NIC on each wakeup. Bounded so a stuck
/// exchange returns rather than spinning forever; a genuinely lost packet still relies on the
/// QEMU-level timeout, the disk driver's discipline.
fn service_until(
    iface: &mut Interface,
    dev: &mut Nic,
    sockets: &mut SocketSet,
    mut cond: impl FnMut(&mut SocketSet) -> bool,
) -> bool {
    let start = instant().total_millis();
    loop {
        iface.poll(instant(), dev, sockets);
        if cond(sockets) {
            return true;
        }
        // A stuck exchange still returns rather than blocking a hart forever; 15 s is far beyond any
        // slirp round trip yet well under the 60 s watchdog. The disk driver's bounded discipline.
        if instant().total_millis() - start > 15_000 {
            iface.poll(instant(), dev, sockets);
            return cond(sockets);
        }
        wait_for_nic(iface, dev, sockets);
    }
}

fn udp_sendto(
    iface: &mut Interface,
    dev: &mut Nic,
    sockets: &mut SocketSet,
    table: &Table,
    entry: usize,
    len: usize,
) -> u64 {
    let Some(sk) = table.socks[entry] else {
        return REP_ERR;
    };
    if sk.kind != Kind::Udp || len > DATA_MAX {
        return REP_ERR;
    }
    let Some(window) = sk.window else {
        return REP_ERR;
    };
    let dst = read_dst(window);
    let mut buf = vec![0u8; len];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = window.r8(OFF_PAYLOAD + i as u64);
    }
    if sockets
        .get_mut::<udp::Socket>(sk.handle)
        .send_slice(&buf, dst)
        .is_err()
    {
        return REP_ERR;
    }
    iface.poll(instant(), dev, sockets); // push the datagram out
    REP_OK
}

fn sock_receive(
    iface: &mut Interface,
    dev: &mut Nic,
    sockets: &mut SocketSet,
    table: &Table,
    entry: usize,
) -> u64 {
    let Some(sk) = table.socks[entry] else {
        return REP_ERR;
    };
    let Some(window) = sk.window else {
        return REP_ERR;
    };
    let handle = sk.handle;
    let is_tcp = sk.kind == Kind::Tcp;
    let ready = if is_tcp {
        service_until(iface, dev, sockets, |s| {
            s.get_mut::<tcp::Socket>(handle).can_recv()
        })
    } else {
        service_until(iface, dev, sockets, |s| {
            s.get_mut::<udp::Socket>(handle).can_recv()
        })
    };
    if !ready {
        return REP_ERR;
    }

    let mut buf = [0u8; DATA_MAX];
    let n = if is_tcp {
        sockets
            .get_mut::<tcp::Socket>(handle)
            .recv_slice(&mut buf)
            .unwrap_or(0)
    } else {
        match sockets.get_mut::<udp::Socket>(handle).recv_slice(&mut buf) {
            Ok((n, meta)) => {
                // The datagram's source endpoint rides back in the frame's dst fields, which are
                // dead space on a reply (socket_protocol's layout note). A responder needs it to answer
                // whoever asked (TFTP's transfer ids use it today, and mDNS's legacy-unicast rule
                // did until milestone 298), and this used to be discarded with `.map(|(n, _)| n)`.
                let IpAddress::Ipv4(src) = meta.endpoint.addr;
                for (i, &b) in src.octets().iter().enumerate() {
                    window.w8(OFF_DST_IP + i as u64, b);
                }
                window.w16(OFF_DST_PORT, meta.endpoint.port);
                n
            }
            Err(_) => 0,
        }
    };
    for (i, &b) in buf[..n].iter().enumerate() {
        window.w8(OFF_PAYLOAD + i as u64, b);
    }
    window.w16(OFF_LEN, n as u16);
    n as u64
}

fn tcp_connect(
    iface: &mut Interface,
    dev: &mut Nic,
    sockets: &mut SocketSet,
    table: &Table,
    entry: usize,
) -> u64 {
    let Some(sk) = table.socks[entry] else {
        return REP_ERR;
    };
    if sk.kind != Kind::Tcp {
        return REP_ERR;
    }
    let Some(window) = sk.window else {
        return REP_ERR;
    };
    let dst = read_dst(window);
    let handle = sk.handle;
    let local = sk.local_port;
    {
        let cx = iface.context();
        if sockets
            .get_mut::<tcp::Socket>(handle)
            .connect(cx, dst, local)
            .is_err()
        {
            return REP_ERR;
        }
    }
    // Drive until the connection settles: established, or back to a closed state (a RST from an
    // unused port, the deterministic refusal outcome).
    service_until(iface, dev, sockets, |s| {
        let st = s.get_mut::<tcp::Socket>(handle).state();
        st == tcp::State::Established || st == tcp::State::Closed
    });
    match sockets.get_mut::<tcp::Socket>(handle).state() {
        tcp::State::Established => CONNECT_ESTABLISHED,
        _ => CONNECT_REFUSED,
    }
}

/// Park a fresh TCP socket in `Listen` state on `port`. `None` if smoltcp refuses the port (only
/// port 0 does that), in which case the socket is removed rather than left in the set as a
/// `Closed` socket nothing owns.
fn arm_listener(sockets: &mut SocketSet, port: u16) -> Option<SocketHandle> {
    let s = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; SOCK_BUF]),
        tcp::SocketBuffer::new(vec![0u8; SOCK_BUF]),
    );
    let handle = sockets.add(s);
    if sockets.get_mut::<tcp::Socket>(handle).listen(port).is_err() {
        sockets.remove(handle);
        return None;
    }
    Some(handle)
}

/// **`LISTEN`: claim a port, if this stack was granted it** (milestone 107).
///
/// Three refusals, and they are deliberately distinguishable (`socket_protocol`): outside the grant is
/// a refusal of *authority* and no retry will fix it; already-listening is a collision on an
/// exclusive name and another port would work; `REP_ERR` is a full table. A granted listen is
/// answered with the listener's own capability (§255 (each socket is its own capability)).
///
/// A listener gets no shared frame. That is the contract's claim that a listener and a connection
/// are different objects, made concrete: there is nothing to map, because nothing is carried.
fn tcp_listen(
    sockets: &mut SocketSet,
    table: &mut Table,
    port_word: u64,
    grant: u64,
) -> Result<(usize, u64), u64> {
    // Not truncated to 16 bits: a request for 65536 is a request for a port no grant can name, and
    // silently turning it into port 0 would answer a question nobody asked.
    if port_word > u16::MAX as u64 {
        return Err(LISTEN_DENIED);
    }
    let port = port_word as u16;
    if !grant_allows(grant, port) {
        return Err(LISTEN_DENIED);
    }
    if table
        .socks
        .iter()
        .flatten()
        .any(|s| s.kind == Kind::Listener && s.local_port == port)
    {
        return Err(LISTEN_IN_USE);
    }
    let entry = table.free().ok_or(REP_ERR)?;
    let handle = arm_listener(sockets, port).ok_or(REP_ERR)?;
    table.install(entry, handle, Kind::Listener, port);
    Ok((entry, LISTEN_GRANTED))
}

/// **`ACCEPT`: block until a peer connects, then hand the connection over as its own socket.**
///
/// The listener keeps its badge and its port; what moves is the smoltcp socket that just completed
/// a handshake, which becomes a *connection* in a free entry with a badge of its own, and the
/// listener is immediately re-armed with a fresh socket parked on the same port. That re-arm is the
/// difference between a server that accepts a connection and a server that accepts **one**
/// connection, and it happens before this function returns, so the client can be busy serving the
/// first exchange while the second handshake completes underneath it in the poll loop.
///
/// The connection is a new capability rather than the listener's own, which is the contract
/// enforcing itself: POSIX would let a listening descriptor become the connection in place, and
/// that conflation is exactly what makes "give this program port 80" and "give this program this
/// connection" the same kind of grant there.
///
/// **BUGS.** The backlog is one connection deep, because smoltcp has no accept queue: a second peer
/// arriving in the window between a handshake completing and this call re-arming gets a RST rather
/// than a wait. Bounded by `service_until`'s 15 s, so an `ACCEPT` nobody ever connects to returns
/// `REP_ERR` instead of holding the server forever.
fn tcp_accept(
    iface: &mut Interface,
    dev: &mut Nic,
    sockets: &mut SocketSet,
    table: &mut Table,
    listening: usize,
) -> Result<usize, u64> {
    let Some(listener) = table.socks[listening] else {
        return Err(REP_ERR);
    };
    if listener.kind != Kind::Listener {
        return Err(REP_ERR);
    }
    // Room for the connection first: a handshake taken off the listener with nowhere to put it
    // would be a peer accepted and then dropped.
    let target = table.free().ok_or(REP_ERR)?;
    let handle = listener.handle;
    let port = listener.local_port;

    // `Listen` means no SYN yet and `SynReceived` means the handshake is in flight; any other state
    // means it either landed or was aborted, and both of those end the wait.
    service_until(iface, dev, sockets, |s| {
        !matches!(
            s.get_mut::<tcp::Socket>(handle).state(),
            tcp::State::Listen | tcp::State::SynReceived
        )
    });
    let landed = !matches!(
        sockets.get_mut::<tcp::Socket>(handle).state(),
        tcp::State::Listen | tcp::State::SynReceived | tcp::State::Closed
    );

    // Re-arm whatever happened, including the aborted case: the listener's authority did not expire
    // because one peer sent a RST, and a client that gets `REP_ERR` should be able to accept again.
    match arm_listener(sockets, port) {
        Some(fresh) => {
            table.socks[listening] = Some(Sock {
                handle: fresh,
                ..listener
            });
        }
        None => {
            // The listener is gone; its badge goes with it, so its capability reaches nothing.
            table.socks[listening] = None;
            sockets.remove(handle);
            return Err(REP_ERR);
        }
    }
    if !landed {
        sockets.remove(handle);
        return Err(REP_ERR);
    }
    table.install(target, handle, Kind::Tcp, port);
    Ok(target)
}

/// **`BIND_UDP`: claim a fixed UDP port, if this stack was granted it** (milestone 55's mDNS stack
/// half). `tcp_listen`'s refusals, verbatim, because they are properties of claiming an exclusive
/// port and not of TCP: outside the grant is a refusal of *authority* (`LISTEN_DENIED`), and a port
/// some live socket already holds is a collision (`LISTEN_IN_USE`, and the check spans *all*
/// sockets, so a fixed bind cannot silently shadow an ephemeral port a live socket was allocated).
/// Unlike a listener the socket carries bytes, so its holder attaches a frame to it exactly as to
/// one `OPERATION_OPEN_UDP` made.
fn udp_bind(
    sockets: &mut SocketSet,
    table: &mut Table,
    port_word: u64,
    grant: u64,
) -> Result<(usize, u64), u64> {
    // Not truncated to 16 bits, tcp_listen's reasoning: a request for 65536 is a request for a
    // port no grant can name, and silently taking it as port 0 answers a question nobody asked.
    if port_word > u16::MAX as u64 {
        return Err(LISTEN_DENIED);
    }
    let port = port_word as u16;
    if !udp_grant_allows(grant, port) {
        return Err(LISTEN_DENIED);
    }
    if table.port_held(port) {
        return Err(LISTEN_IN_USE);
    }
    let entry = table.free().ok_or(REP_ERR)?;
    let handle = sockets.add(udp_socket());
    if sockets.get_mut::<udp::Socket>(handle).bind(port).is_err() {
        // Only port 0 is refused, and the grant already excludes it; kept so a surprise is a
        // clean error rather than a socket nothing owns parked in the set.
        sockets.remove(handle);
        return Err(REP_ERR);
    }
    table.install(entry, handle, Kind::Udp, port);
    Ok((entry, LISTEN_GRANTED))
}

fn tcp_send(
    iface: &mut Interface,
    dev: &mut Nic,
    sockets: &mut SocketSet,
    table: &Table,
    entry: usize,
    len: usize,
) -> u64 {
    let Some(sk) = table.socks[entry] else {
        return REP_ERR;
    };
    if sk.kind != Kind::Tcp || len > DATA_MAX {
        return REP_ERR;
    }
    let Some(window) = sk.window else {
        return REP_ERR;
    };
    let mut buf = vec![0u8; len];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = window.r8(OFF_PAYLOAD + i as u64);
    }
    let sent = sockets
        .get_mut::<tcp::Socket>(sk.handle)
        .send_slice(&buf)
        .unwrap_or(0);
    iface.poll(instant(), dev, sockets);
    sent as u64
}

user_mode_runtime::panic_handler!();
