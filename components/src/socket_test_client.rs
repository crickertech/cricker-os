//! A client of the net server's socket contract (milestone 30, piece 3 phase B).
//!
//! It exercises the capability-shaped contract from the outside: mint a shared frame from its own
//! untyped budget, delegate it to the net server, and drive real network exchanges through socket
//! ids on the `Stack` endpoint, no ambient network anywhere. It holds a capability to the stack or
//! it does not; here it was granted one.
//!
//! The exchanges, selected by the entry role, all against QEMU user-mode networking with zero host
//! setup:
//!   - `TEST_UDP_TFTP`: a UDP request/response round trip against **slirp's own built-in TFTP
//!     server** (10.0.2.2:69), which libslirp answers itself with no host network involved. This is
//!     the gating UDP test: deterministic and offline, the UDP twin of the guestfwd echo peer.
//!   - `TEST_NAME_RESOLUTION` (milestone 384 (in a capability system the resolver is a grant)):
//!     two exchanges through `domain_name_system`, reported as two words. **The first gates**: six
//!     queries over TCP to the runners' name server peer (10.0.2.9:53, `helpers/name-server-peer`),
//!     two of which must resolve and four of which are lies or failures the resolver must refuse
//!     for the right reason, then a TCP connection to the address a name resolved to. **The second
//!     does not**: a real query for `example.com` over UDP to 10.0.2.3, which is not a resolver;
//!     libslirp NATs anything sent there to the *host's* configured nameserver
//!     (`get_dns_addr_libresolv`), so it depends on the developer's DNS at that instant and a host
//!     that does not answer reports `NO_ANSWER`. A reply that arrives and is not ours still fails,
//!     because that would be our bug. See notes/net/the-outbound-gates.md and
//!     notes/name-resolution.md.
//!   - `TEST_TCP_ECHO`: a full TCP round trip to slirp's guestfwd echo peer (10.0.2.9:7777 -> a
//!     `/bin/cat`): connect (handshake), send, receive the echo, close (teardown).
//!   - `TEST_TCP_ACCEPT`: **the inbound half** (milestone 107), and the only exchange here that is
//!     not the guest as a client. A port outside the stack's listen grant is refused as a matter of
//!     authority, the granted one binds and is exclusive, and then a *host* process connects to it
//!     through QEMU's `hostfwd` twice, which proves the listener re-arms.
//!     The same spawn then carries **the UDP bind grant's refusals** (milestone 55), because a
//!     second net server does not fit the aarch64 boot (the memory receipt in notes/net/memory-and-reclamation.md; that
//!     lane re-measured it: an eleventh spawn died as `Unmappable(OutOfPageFrames)` in an unrelated
//!     later test). A fixed port outside the grant is refused as authority, a granted one binds
//!     and is exclusive, which incidentally proves the two grant halves compose in one word on the
//!     machine. Multicast traffic rode here first as marker payloads and then as a separate
//!     multicast DNS responder client; the responder was retired on 2026-09-15 (milestone 298,
//!     notes/mdns.md) and nothing exercises multicast now.
//!
//! On success it reports `OK`; any failure reports a stage code, so the kernel test fails loudly
//! with a hint rather than hanging.
//!
//! This is a **module of the `net_stack` binary** (dispatched by its entry role), not a separate binary,
//! because the initrd archive's directory holds at most 15 files; folding the client in keeps the
//! entry count under that ceiling (see xtask `initrd_aarch64`).
//!
//! # Capability contract (when entered as the client)
//! - slot 0: the report endpoint (WRITE)
//! - slot 1: the `Stack` endpoint (WRITE)
//! - slot 2: an untyped budget (to mint and map the shared frame)
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `netcli`. Refused `netcli` (squished)
//! and `socket_client`, which belongs to the real clients milestone 54 will need. This file is a
//! single-consumer `#[path]` module rather than a `[[bin]]`.

use core::sync::atomic::{AtomicU64, Ordering};

use abi::rights;
use domain_name_system::{Query, Reject, TcpReply};
use socket_protocol::*;
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{
    call, call_receiving, cap_delete, exit, map_page_frame, now, receive, receive_cap,
    retype_page_frame, send, send_cap,
};

const REPORT: u64 = 0;
const STACK: u64 = 1;
const MEMORY_REGION: u64 = 2;

/// Test selectors (the entry role), and the success word the kernel test asserts.
pub const TEST_NAME_RESOLUTION: u64 = 1;
pub const TEST_TCP_ECHO: u64 = 2;
pub const TEST_TCP_REOPEN: u64 = 3;
pub const TEST_UDP_TFTP: u64 = 4;
pub const TEST_TCP_ACCEPT: u64 = 5;
pub const TEST_HTTP_PACKAGE: u64 = 6;
/// **Drain a TCP stream from a peer named at spawn, and time it** (milestone 494 (a driver for the
/// network card a PC actually has)'s bench boot). The second start word is the peer, packed
/// `ipv4 << 16 | port`. Not a gate: its peer is a host on a real LAN, or a slirp `guestfwd` command
/// in the rehearsal, and what it reports is a number rather than a verdict.
pub const TEST_TCP_DRAIN: u64 = 7;
/// **The socket-capability roles** (§255 (each socket is its own capability), milestone 649 (every
/// client of a network stack shares its socket numbers)), driven by `net_confinement_tests`. Role
/// 8 is `socket_squatter`'s, in its own file. Names provisional.
///
/// [`TEST_SOCKET_GIVE`] opens a UDP socket and hands it to another program over slot 3;
/// [`TEST_SOCKET_TAKE`], which holds no front door to the stack at all, receives it on slot 3 and
/// runs the TFTP round trip through it; [`TEST_UDP_TFTP_HELD`] is the TFTP client with its socket
/// open and waiting, so another client can try to reach it while it exists.
pub const TEST_SOCKET_GIVE: u64 = 9;
pub const TEST_SOCKET_TAKE: u64 = 10;
pub const TEST_UDP_TFTP_HELD: u64 = 11;
/// The first word a waiting role reports before it waits.
pub const READY: u64 = 0x5EAD;
/// Slot 3 of the three roles above: the hand-off endpoint (`WRITE` to the giver, `READ` to the
/// taker), or the go signal the held TFTP client waits on (`READ`).
const HANDOFF: u64 = 3;
/// Slot 4 of [`TEST_SOCKET_GIVE`]: the go signal it waits on (`READ`).
const GO: u64 = 4;
const OK: u64 = 1;
/// Reported when an exchange could not be completed **for an environmental reason** rather than a
/// defect in our stack: today only the real-DNS half of `TEST_NAME_RESOLUTION`, whose upstream is
/// the host's resolver. The
/// kernel test prints and skips on this instead of failing, so the gate never depends on the
/// developer's network. Distinct from `OK` and from every `0xE0xx` protocol failure.
const NO_ANSWER: u64 = 2;

/// Where the client maps its shared frame.
const PAGE_FRAME_VA: u64 = address_space_map::pair_page(0x0000_0000_00A0_0000);

/// The window onto that frame (milestone 139; see `user_mode_runtime::mapped_window`). A `static`, not a
/// `const`, for the same reason the type's own doc names as its second valid case: the range is
/// only actually mapped once `attach_page_frame`'s `PageFrame::MAP` succeeds, and every `r8`/`w8`/`r16le`/
/// `w16le` call in this file happens after that, during the protocol exchanges `attach_page_frame` is
/// called to set up.
// SAFETY: `attach_page_frame` maps this frame writable at PAGE_FRAME_VA before any read or write below runs.
static WINDOW: MappedWindow = unsafe { MappedWindow::new(PAGE_FRAME_VA, PAGE) };

/// slirp's guest-visible nameserver address (a NAT to the *host's* resolver, not a resolver), the
/// gateway that hosts slirp's own TFTP server, and the guestfwd echo peer the runners attach.
const DNS_IP: [u8; 4] = [10, 0, 2, 3];
const DNS_PORT: u16 = 53;
const GW_IP: [u8; 4] = [10, 0, 2, 2];
const TFTP_PORT: u16 = 69;
const ECHO_IP: [u8; 4] = socket_protocol::fixture::ECHO_PEER_IP;
const ECHO_PORT: u16 = socket_protocol::fixture::ECHO_PEER_PORT;

const DNS_TXID: u16 = 0x1234;

/// **The runners' name server** (milestone 384), a `guestfwd` that runs `helpers/name-server-peer`
/// once per connection, the package source's address on DNS's port. Local rather than in
/// `socket_protocol::fixture` because this is the only program that dials it; the peer spells the
/// zone below again in Python, so the two sides are written independently.
const NAME_SERVER_IP: [u8; 4] = [10, 0, 2, 9];
/// The transaction id for the name server exchanges. Fixed, because over TCP to a peer the test
/// owns the id defends against nothing; a resolver that queries over UDP draws it from the entropy
/// service, which this client is not granted.
const NAME_SERVER_TXID: u16 = 0x384;

/// What the peer's zone must produce. Two names resolve (one through a CNAME the peer compresses
/// the way a recursive server does), and four are refused, each by a different check in
/// `Query::accept`: a name that does not exist, the right answer under the wrong id, an address for
/// a name nobody asked about, and a compression pointer to itself.
enum Expect {
    Address([u8; 4]),
    Refused(Reject),
}

const NAME_SERVER_CASES: [(&str, Expect); 6] = [
    ("packages.nife.test", Expect::Address(NAME_SERVER_IP)),
    ("mirror.nife.test", Expect::Address(NAME_SERVER_IP)),
    ("nosuch.nife.test", Expect::Refused(Reject::NoSuchName)),
    ("forged.nife.test", Expect::Refused(Reject::IdMismatch)),
    ("poisoned.nife.test", Expect::Refused(Reject::NoAddress)),
    (
        "loop.nife.test",
        Expect::Refused(Reject::Malformed(domain_name_system::Error::PointerForward)),
    ),
];

/// **The inbound half** (milestone 107). The guest listens on `LISTEN_PORT`; the host reaches it
/// because the runners add a QEMU `hostfwd` from a host port to this one, the mirror of the
/// `guestfwd` the outbound gate uses. `DENIED_PORT` is deliberately *outside* the listen grant the
/// spawn service hands this stack, so asking for it proves the grant refuses rather than that
/// nothing happened to bind.
///
/// Both come from `socket_protocol::fixture` since milestone 64, when `std_exerciser` became a second
/// binary that has to agree with this one about which port is granted and which is not. Rule 7:
/// what two binaries agree on is a crate.
const LISTEN_PORT: u64 = socket_protocol::fixture::LISTEN_PORT as u64;
const DENIED_PORT: u64 = socket_protocol::fixture::DENIED_PORT as u64;
/// Which of [`SOCKETS`] an exchange means. Most exchanges use one socket at a time, [`SOCKET`].
/// The inbound gate holds two, because they are two objects: the listener never carries a byte and
/// never gets a frame, and the connection is where the frame is. Keeping them apart is the
/// contract, not a convenience. These name places in this client, never anything on the wire.
const SOCKET: usize = 0;
const LISTENER: usize = 0;
const CONNECTION: usize = 1;
/// What the host sends in and what the guest answers with. Different strings on purpose: an echo
/// would pass even if the guest were somehow reflecting the host's own bytes, and the point of this
/// gate is that the guest *composed* an answer to a connection it did not make. Shared with
/// `std_exerciser`'s inbound half through `socket_protocol::fixture`; `xtask`'s prober deliberately
/// keeps its own literals, so the two sides of the exchange are written independently.
const IN_MSG: &[u8] = socket_protocol::fixture::IN_MSG;
const OUT_MSG: &[u8] = socket_protocol::fixture::OUT_MSG;

/// The fixture the runners put in slirp's TFTP directory, and its exact contents. Both sides are
/// fixed so the round trip is asserted byte for byte (see helpers/qemu-runner-*.sh).
pub(crate) const TFTP_NAME: &[u8] = b"nife";
const TFTP_BODY: &[u8] = b"nife-tftp!";

/// **The UDP bind grant's refusals** (milestone 55). `UDP_DENIED_PORT` is deliberately outside the
/// range the kernel test grants this spawn, so asking for it proves the grant *refuses* rather than
/// that nothing happened to bind; `UDP_GRANTED_PORT` is inside it.
///
/// It is 5354 rather than 5353 because the multicast DNS responder held 5353 as a second client of
/// this same stack until milestone 298 retired it on 2026-09-15 (notes/mdns.md). The numbers must
/// match the kernel test's `NET_UDP_GRANT_*` and mean nothing else.
const UDP_DENIED_PORT: u64 = 4444;
const UDP_GRANTED_PORT: u64 = 5354;

/// How many times the real-DNS check sends its query before giving up. A DNS client retries; UDP has
/// no retransmit of its own and the measured single-query loss to a real resolver was ~2.5%, so one
/// attempt made an environment-dependent test look like a code defect. Three attempts is ordinary
/// resolver behaviour, not a widened timeout.
const DNS_ATTEMPTS: u32 = 3;

// `va` is an absolute address, `PAGE_FRAME_VA + <some offset>`, computed at every call site; `WINDOW`
// wants an offset, so these subtract the base back out and let its own bounds check (milestone
// 139) stand in for the hand-written "va is inside the frame" this used to assert only in prose.
fn w8(va: u64, v: u8) {
    WINDOW.w8(va - PAGE_FRAME_VA, v);
}
fn w16le(va: u64, v: u16) {
    WINDOW.w16(va - PAGE_FRAME_VA, v);
}
fn r8(va: u64) -> u8 {
    WINDOW.r8(va - PAGE_FRAME_VA)
}
fn r16le(va: u64) -> u16 {
    WINDOW.r16(va - PAGE_FRAME_VA)
}

/// [`TEST_UDP_TFTP_HELD`]: open the socket, say so, and wait for the go before using it. The wait
/// is the window in which `socket_squatter` tries every way it has to reach this socket.
fn udp_tftp_held() -> ! {
    attach_page_frame();
    if open(OPERATION_OPEN_UDP, SOCKET, 0) != REP_OK {
        done(0xE040);
    }
    send(REPORT, READY, 0, 0);
    let _ = receive(HANDOFF);
    tftp_on_socket()
}

/// [`TEST_SOCKET_GIVE`]: open a UDP socket and send it to the taker, keeping our own copy. Then wait
/// for the go on slot 4, which the test sends once the taker has used the socket and closed it, and
/// report whether our copy still reaches anything: `OK` when it reaches nothing, which is what a
/// close is for. The probe is a zero-length `SENDTO`, which an open socket answers `REP_OK` at once
/// and a closed one `REP_ERR`, so it cannot be mistaken for a receive that timed out.
fn socket_give() -> ! {
    let (word, _, socket) = call_receiving(STACK, OPERATION_OPEN_UDP, 0);
    let Some(socket) = socket.filter(|_| word == REP_OK) else {
        done(0xE0B0);
    };
    // `WRITE` alone: what the taker needs to use it. Our copy keeps `GRANT`; the taker's does not.
    if send_cap(HANDOFF, socket, rights::WRITE, 0) < 0 {
        done(0xE0B1);
    }
    send(REPORT, READY, 0, 0);
    let _ = receive(GO);
    let (stale, _) = call(socket, OPERATION_SENDTO, 0);
    done(if stale == REP_ERR { OK } else { 0xE0B2 });
}

/// [`TEST_SOCKET_TAKE`]: take a socket from slot 3, attach our own page to it, and run the TFTP round
/// trip through it, with no front door to the stack at all. Then check that after our `CLOSE` the
/// capability reaches nothing. Reports `OK`, or a stage code.
fn socket_take() -> ! {
    let (_, socket, _) = receive_cap(HANDOFF);
    if socket == abi::rendezvous::NO_CAP {
        done(0xE0C0);
    }
    attach_page_frame();
    keep(SOCKET, socket, true);
    tftp_on_socket()
}

/// The source endpoint a UDP RECEIVE reply left in the frame header (`socket_protocol`'s layout note).
fn receive_source() -> ([u8; 4], u16) {
    let mut ip = [0u8; 4];
    for (i, b) in ip.iter_mut().enumerate() {
        *b = r8(PAGE_FRAME_VA + OFF_DST_IP + i as u64);
    }
    (ip, r16le(PAGE_FRAME_VA + OFF_DST_PORT))
}

/// Set the shared frame's destination header.
fn set_dst(ip: [u8; 4], port: u16) {
    for (i, &b) in ip.iter().enumerate() {
        w8(PAGE_FRAME_VA + OFF_DST_IP + i as u64, b);
    }
    w16le(PAGE_FRAME_VA + OFF_DST_PORT, port);
}

/// Report `code` and stop.
fn done(code: u64) -> ! {
    send(REPORT, code, 0, 0);
    // Exit so the kernel reaps this one-shot client rather than leaving it spinning on a run queue
    // forever. Leaked net-client spinners accumulate across the socket-contract tests and starve the
    // later std_net test on core 0 (the same test-thread-starvation finding that made the driver
    // roles exit; nothing balances threads across cores yet, DECISIONS Open design ideas). A
    // one-shot role must exit, not spin.
    exit();
}

/// **The capabilities of this client's open sockets** (§255 (each socket is its own capability)):
/// what `OPEN`, `LISTEN`, `BIND_UDP` and `ACCEPT` handed back, by the place the exchange keeps them
/// in. [`NO_SOCKET`] where none is open. Statics because the exchanges are free functions and the
/// client is one thread.
static SOCKETS: [AtomicU64; 2] = [AtomicU64::new(NO_SOCKET), AtomicU64::new(NO_SOCKET)];
const NO_SOCKET: u64 = u64::MAX;
/// The slot of our shared frame, once [`attach_page_frame`] has minted it. Every socket that carries
/// bytes gets a copy of it, so the client keeps one page and one mapping however many it opens.
static FRAME: AtomicU64 = AtomicU64::new(NO_SOCKET);

/// **Make a socket through the front door** and keep its capability in `which`: `OPEN_TCP`,
/// `OPEN_UDP`, `LISTEN` or `BIND_UDP`, with `arg` the port for the last two. Returns the reply word.
/// A socket that carries bytes is handed a copy of our frame straight away. A success that arrived
/// without its capability reads as `REP_ERR`, because a socket this client cannot name is no use.
fn open(operation: u64, which: usize, arg: u64) -> u64 {
    let (word, _, socket) = call_receiving(STACK, operation, arg);
    // `REP_OK` and `LISTEN_GRANTED` are both 0: every success here carries a capability.
    let Some(socket) = socket else {
        return if word == REP_OK { REP_ERR } else { word };
    };
    keep(which, socket, operation != OPERATION_LISTEN);
    word
}

/// `ACCEPT` on the listener in `listener`, keeping the connection it hands back in `connection`.
fn accept(listener: usize, connection: usize) -> u64 {
    let (word, _, socket) = call_receiving(held(listener), OPERATION_ACCEPT, 0);
    let Some(socket) = socket else {
        return REP_ERR;
    };
    keep(connection, socket, true);
    word
}

/// File `socket` as `which` and, if it carries bytes, give it a copy of our frame. A listener
/// carries none and gets none.
fn keep(which: usize, socket: u64, carries_bytes: bool) {
    SOCKETS[which].store(socket, Ordering::Relaxed);
    let frame = FRAME.load(Ordering::Relaxed);
    if carries_bytes
        && frame != NO_SOCKET
        && send_cap(
            socket,
            frame,
            rights::READ | rights::WRITE,
            OPERATION_ATTACH_PAGE_FRAME,
        ) < 0
    {
        done(0xE003);
    }
}

/// The capability kept as `which`.
fn held(which: usize) -> u64 {
    SOCKETS[which].load(Ordering::Relaxed)
}

/// One `CALL` on the socket kept as `which`.
fn on(which: usize, operation: u64, arg: u64) -> (u64, u64) {
    call(held(which), operation, arg)
}

/// `CLOSE` the socket kept as `which`, and drop our capability to it.
fn close(which: usize) -> (u64, u64) {
    let answer = on(which, OPERATION_CLOSE, 0);
    cap_delete(held(which));
    SOCKETS[which].store(NO_SOCKET, Ordering::Relaxed);
    answer
}

/// Mint a frame from our untyped and map it writable. Each socket opened after this is handed a
/// copy of it ([`keep`]).
fn attach_page_frame() {
    // RETYPE returns the new frame capability's slot, or a negative error.
    let frame = retype_page_frame(MEMORY_REGION);
    if frame < 0 {
        done(0xE001);
    }
    let frame = frame as u64;
    // Map it writable; page tables come from our untyped.
    if !map_page_frame(frame, PAGE_FRAME_VA, true, MEMORY_REGION) {
        done(0xE002);
    }
    FRAME.store(frame, Ordering::Relaxed);
}

/// Write a byte at `*at` and advance it.
fn put8(v: u8, at: &mut u64) {
    w8(*at, v);
    *at += 1;
}

/// **`TEST_NAME_RESOLUTION`**: the gating exchange against the runners' name server, then the
/// non-gating one against the host's resolver, reported together as `(gating, real)`. One role and
/// one `net_stack` for both, because every stack a test starts holds a virtio slot for the rest of
/// the boot and the table is at its ceiling (`MAX_DEVICES` in kernel/src/virtio.rs).
fn name_resolution() -> ! {
    attach_page_frame();
    let gating = against_the_name_server();
    let real = against_the_host_resolver();
    send(REPORT, gating, real, 0);
    exit();
}

/// Each name in [`NAME_SERVER_CASES`] over its own TCP connection, every verdict checked, and then
/// an echo through the address `packages.nife.test` resolved to: a name the guest did not know the
/// address of, carried to a connection. `OK`, or `0xE1` with the case in the next nibble and the
/// stage in the last.
fn against_the_name_server() -> u64 {
    let mut resolved = None;
    for (case, (host, expect)) in NAME_SERVER_CASES.iter().enumerate() {
        let code = |stage: u64| 0xE100 | ((case as u64) << 4) | stage;
        let Ok(query) = Query::new(NAME_SERVER_TXID, host) else {
            return code(0x0);
        };
        let verdict = match ask_over_tcp(&query) {
            Ok(verdict) => verdict,
            Err(stage) => return code(stage),
        };
        match (expect, verdict) {
            (Expect::Address(want), Ok(answer)) if answer.addresses() == [*want] => {
                resolved.get_or_insert(answer.addresses()[0]);
            }
            (Expect::Refused(want), Err(got)) if *want == got => {}
            _ => return code(0xF), // the wrong verdict: the lie was believed, or the truth refused
        }
    }
    match resolved {
        Some(address) => echo_at(address),
        None => 0xE1F0,
    }
}

/// One query on socket 0 over TCP: the reply's verdict, or the stage that stopped the exchange.
fn ask_over_tcp(query: &Query) -> Result<Result<domain_name_system::Answer, Reject>, u64> {
    if open(OPERATION_OPEN_TCP, SOCKET, 0) != REP_OK {
        return Err(0x1);
    }
    let verdict = exchange_over_tcp(query);
    let _ = close(SOCKET);
    verdict
}

fn exchange_over_tcp(query: &Query) -> Result<Result<domain_name_system::Answer, Reject>, u64> {
    set_dst(NAME_SERVER_IP, domain_name_system::PORT);
    if on(SOCKET, OPERATION_CONNECT, 0).0 != CONNECT_ESTABLISHED {
        return Err(0x2);
    }
    let mut request = [0u8; 2 + domain_name_system::UDP_MESSAGE_MAX];
    let n = query.request_tcp(&mut request).map_err(|_| 0x3u64)?;
    for (i, &b) in request[..n].iter().enumerate() {
        w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
    }
    if on(SOCKET, OPERATION_SEND, n as u64).0 != n as u64 {
        return Err(0x4);
    }
    let mut buf = [0u8; 2 + domain_name_system::UDP_MESSAGE_MAX];
    let mut reply = TcpReply::new(&mut buf);
    let mut chunk = [0u8; DATA_MAX];
    loop {
        let (got, _) = on(SOCKET, OPERATION_RECEIVE, 0);
        if got == 0 || got > DATA_MAX as u64 {
            return Err(0x5); // the peer went away (or the stack failed) before the reply was whole
        }
        let got = got as usize;
        for (i, b) in chunk[..got].iter_mut().enumerate() {
            *b = r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64);
        }
        match reply.feed(&chunk[..got]) {
            Ok(true) => break,
            Ok(false) => {}
            Err(_) => return Err(0x6), // longer than announced, or than a reply here may be
        }
    }
    let message = reply.message().ok_or(0x7u64)?;
    Ok(query.accept(message))
}

/// Connect to `address` on the echo peer's port and see one payload come back: the resolved address
/// is one a connection can be made to, which is what resolving it was for.
fn echo_at(address: [u8; 4]) -> u64 {
    const MSG: &[u8] = b"nife-by-name";
    if open(OPERATION_OPEN_TCP, SOCKET, 0) != REP_OK {
        return 0xE1F1;
    }
    set_dst(address, ECHO_PORT);
    let mut code = OK;
    if on(SOCKET, OPERATION_CONNECT, 0).0 != CONNECT_ESTABLISHED {
        code = 0xE1F2;
    } else {
        for (i, &b) in MSG.iter().enumerate() {
            w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
        }
        if on(SOCKET, OPERATION_SEND, MSG.len() as u64).0 != MSG.len() as u64 {
            code = 0xE1F3;
        } else if on(SOCKET, OPERATION_RECEIVE, 0).0 != MSG.len() as u64
            || MSG
                .iter()
                .enumerate()
                .any(|(i, &b)| r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64) != b)
        {
            code = 0xE1F4;
        }
    }
    let _ = close(SOCKET);
    code
}

/// **Real DNS resolution, and therefore NOT a gate.** The query goes to 10.0.2.3, which libslirp
/// NATs to the *host's* nameserver, so whether it is answered is a fact about the developer's
/// machine. Retries like any resolver client, then reports `NO_ANSWER` if the host never answered,
/// which the kernel test turns into a loud skip. A server that answers with a failure (SERVFAIL from
/// an offline host, NXDOMAIN from a filtering one) is environmental too. A reply that is not ours,
/// or that `Query::accept` finds malformed, or one for `example.com` with no address in it, still
/// fails: that would be a defect here, not in the network.
fn against_the_host_resolver() -> u64 {
    if open(OPERATION_OPEN_UDP, SOCKET, 0) != REP_OK {
        return 0xE010;
    }
    let code = ask_the_host_resolver();
    let _ = close(SOCKET);
    code
}

fn ask_the_host_resolver() -> u64 {
    let Ok(query) = Query::new(DNS_TXID, "example.com") else {
        return 0xE012;
    };
    let mut request = [0u8; domain_name_system::UDP_MESSAGE_MAX];
    let Ok(n) = query.request(&mut request) else {
        return 0xE012;
    };
    let mut reply = [0u8; DATA_MAX];
    for _ in 0..DNS_ATTEMPTS {
        for (i, &b) in request[..n].iter().enumerate() {
            w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
        }
        set_dst(DNS_IP, DNS_PORT);
        if on(SOCKET, OPERATION_SENDTO, n as u64).0 != REP_OK {
            return 0xE011;
        }
        let (rlen, _) = on(SOCKET, OPERATION_RECEIVE, 0);
        if rlen == REP_ERR || rlen == 0 || rlen > DATA_MAX as u64 {
            continue;
        }
        let rlen = rlen as usize;
        for (i, b) in reply[..rlen].iter_mut().enumerate() {
            *b = r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64);
        }
        return match query.accept(&reply[..rlen]) {
            Ok(_) => OK,
            Err(Reject::NoSuchName | Reject::ServerError(_) | Reject::Truncated) => NO_ANSWER,
            Err(Reject::IdMismatch) => 0xE013,
            Err(Reject::NotAResponse) => 0xE014,
            Err(_) => 0xE015,
        };
    }
    // The host's resolver never answered. Environmental, not ours.
    NO_ANSWER
}

/// **The gating UDP test: a round trip against slirp's own TFTP server.** libslirp implements TFTP
/// internally (enabled by `tftp=` on the netdev), so this request and its reply never leave the
/// emulator: no host resolver, no internet, no packet that can be dropped by somebody else's router.
/// It proves exactly what the DNS test was there to prove about *our* code, and nothing about the
/// host: a client holding only a `Stack` endpoint and a shared frame can open a UDP socket,
/// send a datagram to a chosen address, and read the reply back through the same frame.
///
/// Send a read request (opcode 1, `octet` mode) for the fixture the runners planted, and require the
/// first data packet back: opcode 3, block 1, and the fixture's bytes exactly.
fn udp_tftp() -> ! {
    attach_page_frame();
    if open(OPERATION_OPEN_UDP, SOCKET, 0) != REP_OK {
        done(0xE040);
    }
    tftp_on_socket()
}

/// The TFTP round trip on the UDP socket kept as [`SOCKET`], whose frame is attached, then `CLOSE`
/// it and report.
fn tftp_on_socket() -> ! {
    // RRQ: { u16 opcode = 1 } filename 0 "octet" 0
    let mut p = PAGE_FRAME_VA + OFF_PAYLOAD;
    put8(0x00, &mut p);
    put8(0x01, &mut p);
    for &c in TFTP_NAME {
        put8(c, &mut p);
    }
    put8(0x00, &mut p);
    for &c in b"octet" {
        put8(c, &mut p);
    }
    put8(0x00, &mut p);
    let qlen = p - (PAGE_FRAME_VA + OFF_PAYLOAD);

    set_dst(GW_IP, TFTP_PORT);
    if on(SOCKET, OPERATION_SENDTO, qlen).0 != REP_OK {
        done(0xE041);
    }

    // DATA: { u16 opcode = 3 }{ u16 block = 1 } body. The fixture is one short block, so the whole
    // file arrives in this first packet and no ACK/continuation is needed.
    let (rlen, _) = on(SOCKET, OPERATION_RECEIVE, 0);
    if rlen == REP_ERR || rlen < 4 + TFTP_BODY.len() as u64 {
        done(0xE042);
    }
    let opcode = ((r8(PAGE_FRAME_VA + OFF_PAYLOAD) as u16) << 8)
        | r8(PAGE_FRAME_VA + OFF_PAYLOAD + 1) as u16;
    let block = ((r8(PAGE_FRAME_VA + OFF_PAYLOAD + 2) as u16) << 8)
        | r8(PAGE_FRAME_VA + OFF_PAYLOAD + 3) as u16;
    if opcode != 3 {
        done(0xE043); // an ERROR packet (opcode 5) means the fixture is missing: see the runners
    }
    if block != 1 {
        done(0xE044);
    }
    for (i, &b) in TFTP_BODY.iter().enumerate() {
        if r8(PAGE_FRAME_VA + OFF_PAYLOAD + 4 + i as u64) != b {
            done(0xE045); // the bytes came back changed
        }
    }

    // The RECEIVE reply now carries the DATA packet's source endpoint in the frame header (milestone
    // 55's stack half), and this is the slirp-provable check of it: the DATA came from the gateway,
    // from a real port. The port is deliberately not pinned to 69: TFTP's own protocol has the
    // server answer from a transfer-id port of its choosing (RFC 1350 §4), so asserting 69 would
    // pin a libslirp implementation detail.
    let (src_ip, src_port) = receive_source();
    if src_ip != GW_IP {
        done(0xE046); // the reported source is not the server that answered
    }
    if src_port == 0 {
        done(0xE047); // no source port arrived at all
    }

    // ACK block 1, which ends the transfer properly: { u16 opcode = 4 }{ u16 block = 1 }. The fixture
    // is one short block, so this is the last packet of the exchange. Without it the server would sit
    // retransmitting its DATA at a socket we are about to close, which is rude to the next test that
    // brings this NIC up even though libslirp eventually gives up on its own.
    //
    // Addressed to the DATA's reported source rather than to :69, which is what TFTP's TID scheme
    // asks for and is the first real consumer of the source endpoint: replying to the querier is
    // exactly the move any UDP responder makes.
    let mut a = PAGE_FRAME_VA + OFF_PAYLOAD;
    put8(0x00, &mut a);
    put8(0x04, &mut a);
    put8(0x00, &mut a);
    put8(0x01, &mut a);
    set_dst(src_ip, src_port);
    let _ = on(SOCKET, OPERATION_SENDTO, a - (PAGE_FRAME_VA + OFF_PAYLOAD));

    let _ = close(SOCKET);
    done(OK);
}

fn tcp_echo() -> ! {
    const MSG: &[u8] = b"nife-net!";

    attach_page_frame();
    if open(OPERATION_OPEN_TCP, SOCKET, 0) != REP_OK {
        done(0xE020);
    }

    set_dst(ECHO_IP, ECHO_PORT);
    let (outcome, _) = on(SOCKET, OPERATION_CONNECT, 0);
    if outcome != CONNECT_ESTABLISHED {
        done(0xE021); // handshake did not complete (refused/reset)
    }

    for (i, &b) in MSG.iter().enumerate() {
        w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
    }
    let (sent, _) = on(SOCKET, OPERATION_SEND, MSG.len() as u64);
    if sent != MSG.len() as u64 {
        done(0xE022);
    }

    let (rlen, _) = on(SOCKET, OPERATION_RECEIVE, 0);
    if rlen != MSG.len() as u64 {
        done(0xE023); // the echo did not come back whole
    }
    for (i, &b) in MSG.iter().enumerate() {
        if r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64) != b {
            done(0xE024); // the echoed bytes differ
        }
    }

    let _ = close(SOCKET);
    done(OK);
}

/// **Regression: closing a socket and opening the next one is safe.** Open a TCP socket, connect to
/// the echo peer, close it, then open another and connect again. Before `net_stack` assigned
/// ephemeral local ports independent of the socket's place in its table, the reopen reused the exact
/// local port, and the second connect on a 4-tuple whose slirp flow had not yet cleared stalled
/// `net_stack`'s bounded poll forever (found by the `std::net` PAL,
/// notes/net/the-outbound-gates.md). With the rotating allocator the reopen gets a fresh port, so
/// both connects complete. Since §255 (each socket is its own capability) the second socket also
/// lands in the table entry the first one left, with a new badge, so this exercises the entry's
/// page being taken back and attached again.
fn tcp_reopen() -> ! {
    attach_page_frame();
    set_dst(ECHO_IP, ECHO_PORT);

    // The first connection.
    if open(OPERATION_OPEN_TCP, SOCKET, 0) != REP_OK {
        done(0xE030);
    }
    if on(SOCKET, OPERATION_CONNECT, 0).0 != CONNECT_ESTABLISHED {
        done(0xE031);
    }
    let _ = close(SOCKET);

    // A second socket, in the entry the first left, and connect again. This is the exact path that
    // hung before the port fix.
    if open(OPERATION_OPEN_TCP, SOCKET, 0) != REP_OK {
        done(0xE032);
    }
    if on(SOCKET, OPERATION_CONNECT, 0).0 != CONNECT_ESTABLISHED {
        done(0xE033);
    }
    let _ = close(SOCKET);

    done(OK);
}

/// **The inbound gate: a granted port, and the guest connected TO through it, twice** (milestone
/// 107).
///
/// Everything else in this file is the guest as a client. Here it is the server: listen on a port it
/// was granted, accept a connection a *host* process opened through QEMU's `hostfwd`, read what
/// arrived, answer it, and then do the whole thing again on the same listener. The second round is
/// the load-bearing one: a listener that can accept exactly one connection is a listener a file
/// server cannot use, and nothing but a second accept proves the re-arm.
///
/// **The grant checks ride in the same exchange rather than in a test of their own, and that is the
/// machine's call, not a preference.** A second net server costs a 192-page untyped region that is
/// never reclaimed (nothing unregisters a transport or reaps `net_stack`), and the aarch64 test boot
/// has no room for one: with two, a later test asking for 128 contiguous pages found 137 free frames
/// and no run that long. So one spawn proves both halves, with distinct stage codes standing in for
/// the separate test names.
///
/// The frame is attached to the *connection* and never to the listener, and it is attached only
/// after the listener is bound. That ordering is the two-object split made visible: the whole grant
/// half runs with no shared frame anywhere, because a listener carries no bytes.
fn tcp_accept_inbound() -> ! {
    // A port outside the grant is refused as a matter of AUTHORITY, which is a different answer from
    // "somebody has it" and calls for a different response from a client.
    match open(OPERATION_LISTEN, LISTENER, DENIED_PORT) {
        LISTEN_DENIED => {}
        LISTEN_GRANTED => done(0xE050), // bound a port nothing granted: the whole point, lost
        LISTEN_IN_USE => done(0xE051),
        _ => done(0xE052),
    }

    // The granted one binds, and this listener is the one the rest of the exchange accepts on.
    match open(OPERATION_LISTEN, LISTENER, LISTEN_PORT) {
        LISTEN_GRANTED => {}
        LISTEN_DENIED => done(0xE053), // the spawn service granted the wrong range
        LISTEN_IN_USE => done(0xE054),
        _ => done(0xE055),
    }

    // And it is exclusive, which is the property that makes a port grantable rather than merely a
    // number. Asking again must collide.
    match open(OPERATION_LISTEN, CONNECTION, LISTEN_PORT) {
        LISTEN_IN_USE => {}
        LISTEN_GRANTED => done(0xE056), // two listeners on one port
        _ => done(0xE057),
    }

    // Only now a frame, and only for the connection.
    attach_page_frame();

    // Two connections in a row, each with its own stage codes so a failure names which one.
    serve_one_inbound(0xE060);
    serve_one_inbound(0xE070);

    let _ = close(LISTENER);

    // The UDP bind half rides in this same spawn (milestone 55's stack half), because a second net
    // server does not fit the aarch64 boot: the spawn is ~154 frames nothing ever reclaims, and
    // this lane measured the eleventh one dying as `Unmappable(OutOfPageFrames)` in an unrelated later
    // test, the exact failure notes/net/memory-and-reclamation.md's memory receipt predicted.
    // Milestone 107 (the socket contract learns to accept) folded its grant half for the same
    // reason; the stage codes stand in for the separate test's name.
    udp_bind_half();
    done(OK);
}

/// Accept one inbound connection, check what the host sent, answer it, and close. Reports through
/// `done` on any failure, so `base` distinguishes the first connection from the second.
fn serve_one_inbound(base: u64) {
    if accept(LISTENER, CONNECTION) != REP_OK {
        done(base); // nobody connected within the server's bounded wait
    }

    let (rlen, _) = on(CONNECTION, OPERATION_RECEIVE, 0);
    if rlen != IN_MSG.len() as u64 {
        done(base + 1);
    }
    for (i, &b) in IN_MSG.iter().enumerate() {
        if r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64) != b {
            done(base + 2); // something connected and said something else
        }
    }

    for (i, &b) in OUT_MSG.iter().enumerate() {
        w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
    }
    let (sent, _) = on(CONNECTION, OPERATION_SEND, OUT_MSG.len() as u64);
    if sent != OUT_MSG.len() as u64 {
        done(base + 3);
    }

    if close(CONNECTION).0 != REP_OK {
        done(base + 4);
    }
}

/// **A fixed UDP port is an authority, and this is the half that proves the refusals** (milestone
/// 55; runs inside the accept test's spawn, see `tcp_accept_inbound`).
///
/// - **A port outside the grant is `LISTEN_DENIED`**, which is the capability answer and a
///   different one from "somebody has it": no retry helps, and no other port will do unless the
///   spawn site granted it. Since this spawn's word also carries the TCP listen grant the accept
///   half just spent, the machine is exercising the *composed* word rather than one half alone.
/// - **A granted port binds**, and asking for it again collides, which is
///   the exclusivity that makes a port a grantable thing rather than a number.
///
/// **What is deliberately not here any more**: the marker-payload exchange with xtask's multicast
/// prober. It proved that a joined group receives, that a multicast `SENDTO` reaches the wire, and
/// that a datagram's source endpoint rides back on `RECEIVE`. The multicast DNS responder took over
/// the first two with real DNS messages, and milestone 298 retired it and the prober on 2026-09-15
/// (notes/mdns.md), so **nothing proves multicast now**. The third is still proved, by `udp_tftp`.
fn udp_bind_half() {
    match open(OPERATION_BIND_UDP, LISTENER, UDP_DENIED_PORT) {
        LISTEN_DENIED => {}
        LISTEN_GRANTED => done(0xE080), // bound a port nothing granted: the whole point, lost
        _ => done(0xE081),
    }

    match open(OPERATION_BIND_UDP, CONNECTION, UDP_GRANTED_PORT) {
        LISTEN_GRANTED => {}
        LISTEN_DENIED => done(0xE082), // the spawn granted the wrong range
        _ => done(0xE083),
    }

    // Exclusive, the property that makes a fixed port grantable rather than merely a number.
    match open(OPERATION_BIND_UDP, LISTENER, UDP_GRANTED_PORT) {
        LISTEN_IN_USE => {}
        LISTEN_GRANTED => done(0xE084), // two sockets on one fixed port
        _ => done(0xE085),
    }

    let _ = close(CONNECTION);
}

/// Where the spawner maps the image's package catalogue (`package_archive::CATALOGUE`) for
/// [`TEST_HTTP_PACKAGE`], read-only. The kernel test and this file must agree; see
/// `kernel/src/user/virtio_service.rs`'s `NET_CLIENT_CATALOGUE_VA`.
const CATALOGUE_VA: u64 = address_space_map::pair_page(0x0000_0000_00C0_0000);
/// Reported in a word of its own when the fetched bytes did not match the catalogue's digest, which
/// is the refusal the tampered fetch must produce. Distinct from `OK` and from every stage failure,
/// so a test asserting a refusal cannot be satisfied by a broken fetch.
pub const DIGEST_REFUSED: u64 = 3;
// The package source the runners put at 10.0.2.9:8080 (`helpers/package-http-peer`).
use socket_protocol::fixture::{
    PACKAGE_PEER_HOST, PACKAGE_PEER_IP as PACKAGE_IP, PACKAGE_PEER_PORT as PACKAGE_PORT,
};

/// **Rung 3a's fetch and verify** (milestone 198 (a package manager)): `GET` a package from a host
/// over plain HTTP, hash it as it arrives, and accept it only if the digest is the one the image's
/// own catalogue names. **Twice, in one spawn**: the genuine package, which must be accepted, then
/// the peer's copy with one byte flipped, which must be refused. The two verdicts go back as the
/// report's first and second words. One spawn rather than two tests because every `net_stack` a
/// test starts takes a virtio slot for the rest of the boot, and the table was full (see
/// `MAX_DEVICES` in `kernel/src/virtio.rs`); milestone 107 (the socket contract learns to accept)
/// made the same trade the same way.
///
/// The catalogue is what makes plain HTTP enough. It is an archive entry packed above the
/// measurement table, so the kernel's trust root vouches for it, and the digest it carries never
/// crossed the network (DECISIONS §195 (a reviewed recipe vouches for a package)). The body is never
/// held: each read goes through `http_response` and into the hash, so a package costs this client
/// one page of socket frame and the hash state. **That is also this exchange's limit**: it proves
/// the bytes, and installing them is `notes/packages.md`'s "Where this stops".
///
/// `len` is the catalogue's length; the spawner maps it read-only at [`CATALOGUE_VA`].
fn http_package(len: u64) -> ! {
    // SAFETY: the spawner maps `len` bytes of catalogue at CATALOGUE_VA, read-only, for the life of
    // this client, and nothing writes them.
    let catalogue = unsafe { core::slice::from_raw_parts(CATALOGUE_VA as *const u8, len as usize) };
    let Ok(catalogue) = core::str::from_utf8(catalogue) else {
        done(0xE090);
    };
    #[cfg(target_arch = "aarch64")]
    const STEM: &str = "uptime-0.1.0-aarch64";
    #[cfg(target_arch = "riscv64")]
    const STEM: &str = "uptime-0.1.0-riscv64";
    #[cfg(target_arch = "x86_64")]
    const STEM: &str = "uptime-0.1.0-x86_64";
    let Some(expected) = measured_boot::expected_in_manifest(catalogue, STEM) else {
        done(0xE091); // the image vouches for no such package, so nothing fetched could be run
    };

    attach_page_frame();
    let genuine = fetch_and_verify(STEM, false, &expected);
    let tampered = fetch_and_verify(STEM, true, &expected);
    send(REPORT, genuine, tampered, 0);
    exit();
}

/// One fetch on socket 0: `OK`, [`DIGEST_REFUSED`], or the stage code that stopped it.
fn fetch_and_verify(stem: &str, tampered: bool, expected: &measured_boot::Digest) -> u64 {
    if open(OPERATION_OPEN_TCP, SOCKET, 0) != REP_OK {
        return 0xE092;
    }
    let code = exchange(stem, tampered, expected);
    let _ = close(SOCKET);
    code
}

fn exchange(stem: &str, tampered: bool, expected: &measured_boot::Digest) -> u64 {
    set_dst(PACKAGE_IP, PACKAGE_PORT);
    if on(SOCKET, OPERATION_CONNECT, 0).0 != CONNECT_ESTABLISHED {
        return 0xE093;
    }

    // `/tampered/` asks the peer for the same file with one byte flipped.
    let mut path = [0u8; 64];
    let mut at = 0;
    for part in [if tampered { "/tampered/" } else { "/" }, stem, ".nifepkg"] {
        path[at..at + part.len()].copy_from_slice(part.as_bytes());
        at += part.len();
    }
    let path = core::str::from_utf8(&path[..at]).unwrap_or("/");
    let mut request = [0u8; 128];
    let Some(n) = http_response::get_request(PACKAGE_PEER_HOST, path, &mut request) else {
        return 0xE094;
    };
    for (i, &b) in request[..n].iter().enumerate() {
        w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
    }
    if on(SOCKET, OPERATION_SEND, n as u64).0 != n as u64 {
        return 0xE095;
    }

    let mut response = http_response::Response::new();
    let mut hash = measured_boot::Sha256::new();
    let mut chunk = [0u8; DATA_MAX];
    while !response.is_complete() {
        let (got, _) = on(SOCKET, OPERATION_RECEIVE, 0);
        if got == 0 || got > DATA_MAX as u64 {
            return 0xE096; // the peer went away (or the stack failed) before the body was whole
        }
        let got = got as usize;
        for (i, b) in chunk[..got].iter_mut().enumerate() {
            *b = r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64);
        }
        let Ok(body) = response.feed(&chunk[..got]) else {
            return 0xE097; // not an HTTP response this client accepts
        };
        if response.status().is_some_and(|s| s != 200) {
            return 0xE098;
        }
        // A package is bounded by `u32` (package_archive's BUGS); refuse a larger claim before
        // reading a byte of it.
        if response
            .content_length()
            .is_some_and(|l| l > u64::from(u32::MAX))
        {
            return 0xE099;
        }
        hash.update(body);
    }
    if hash.finalize() == *expected {
        OK
    } else {
        DIGEST_REFUSED
    }
}

/// [`TEST_TCP_DRAIN`]: connect to `peer` (`ipv4 << 16 | port`), read until the stream ends, and
/// report `(OK or a stage code, bytes, counter ticks from the handshake completing to the last
/// byte)`.
///
/// "Ends" is the peer closing or `net_stack`'s 15-second receive bound, whichever comes first,
/// and neither is counted: the clock stops at the last byte that arrived, so the tail of waiting
/// for a close that never comes does not dilute the rate. It starts when the handshake completes,
/// so the first byte's round trip is in the figure, which understates a rate by one RTT.
fn tcp_drain(peer: u64) -> ! {
    let ip = ((peer >> 16) as u32).to_be_bytes();
    let port = peer as u16;
    attach_page_frame();
    if open(OPERATION_OPEN_TCP, SOCKET, 0) != REP_OK {
        done(0xE0A0);
    }
    set_dst(ip, port);
    if on(SOCKET, OPERATION_CONNECT, 0).0 != CONNECT_ESTABLISHED {
        done(0xE0A1); // nobody listening at the peer, or no route to it
    }
    let mut bytes: u64 = 0;
    let first = now();
    let mut last = first;
    loop {
        let (got, _) = on(SOCKET, OPERATION_RECEIVE, 0);
        if got == 0 || got > DATA_MAX as u64 {
            break;
        }
        last = now();
        bytes += got;
    }
    let _ = close(SOCKET);
    send(REPORT, OK, bytes, last.saturating_sub(first));
    exit();
}

/// Run the selected client exchange. Entered from `net_stack`'s `_start` when the entry role is
/// nonzero; `arg` is the second start word, which [`TEST_HTTP_PACKAGE`] and [`TEST_TCP_DRAIN`]
/// read.
pub fn run(test: u64, arg: u64) -> ! {
    match test {
        TEST_HTTP_PACKAGE => http_package(arg),
        TEST_NAME_RESOLUTION => name_resolution(),
        TEST_UDP_TFTP => udp_tftp(),
        TEST_TCP_ECHO => tcp_echo(),
        TEST_TCP_REOPEN => tcp_reopen(),
        TEST_TCP_ACCEPT => tcp_accept_inbound(),
        TEST_TCP_DRAIN => tcp_drain(arg),
        TEST_SOCKET_GIVE => socket_give(),
        TEST_SOCKET_TAKE => socket_take(),
        TEST_UDP_TFTP_HELD => udp_tftp_held(),
        _ => done(0xE0FF),
    }
}
