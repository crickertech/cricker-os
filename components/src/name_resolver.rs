//! **The name resolver** (milestone 384 (in a capability system the resolver is a grant); §248
//! (the name resolver is its own confined program)).
//!
//! Turns a host name into IPv4 addresses for a client that holds a grant to ask, and for nobody
//! else. It is a client of `net_stack` like any other and a server to its own clients; the
//! contract between them is `crates/name_resolution_protocol`, and the wire format it speaks to a
//! name server is `crates/domain_name_system`, whose `Query::accept` decides what is believed.
//!
//! ```text
//!   spawner ──GRANT (badge 0)──►┌───────────────┐──Stack (socket contract)──► net_stack ──► name server
//!   client ──RESOLVE (badge B)─►│ name_resolver │
//!     (its page holds the name, └──────┬────────┘
//!      and gets the addresses)         └──entropy──► the transaction id
//! ```
//!
//! # What it holds
//!
//! | slot | what | rights |
//! |---|---|---|
//! | 0 | its own endpoint: the spawner's grants and its clients' requests arrive here | `READ` |
//! | 1 | `net_stack`'s `Stack` endpoint, its whole network authority | `WRITE` |
//! | 2 | an untyped budget: the socket page, and the page tables for its clients' pages | |
//! | 3 | the entropy service's endpoint, for transaction ids | `WRITE` |
//!
//! And three start words ([`name_resolution_protocol::endowment`]): the name server's address, its
//! port and the transport, and the socket id to use on the stack. Which server to ask is the
//! spawner's decision, read from the DHCP lease (`socket_protocol::lease`), the way the network time
//! client is told its server rather than choosing one.
//!
//! # What a client can and cannot make it do
//!
//! A client holds a badged copy of slot 0 and nothing else of this process. **The name is judged
//! against the zone its badge was granted before anything is sent**, so a client granted
//! `nife.test` that asks about `example.com` gets [`status::DENIED`] and the network sees nothing.
//! It cannot widen its grant: only a message with badge 0 is a grant, and only the spawner holds an
//! unbadged capability. It cannot read another client's answer: each badge's page is mapped in its
//! own window, and the name is copied out of the page before it is parsed, so a client rewriting
//! its page mid-request changes nothing but its own reply.
//!
//! On Unix every process can read `/etc/resolv.conf` and call `getaddrinfo`, and the first sign
//! that a program looked up a host it should not have is a packet capture.
//!
//! # How it asks
//!
//! Over UDP, with a transaction id drawn from the entropy service for every query, and again over
//! TCP when the reply comes back truncated; or over TCP alone when the spawner says the server
//! speaks nothing else (the QEMU runners' peer, which slirp reaches only over TCP). A UDP datagram
//! from any address but the server's, or one `Query::accept` refuses as not ours (another id,
//! another question, not a response, not DNS), is dropped and the resolver keeps listening, so a
//! forger's packet arriving first does not end a real exchange. **With no entropy it asks
//! nothing** and answers [`status::NO_ENTROPY`], for the reason the network time client refuses: a
//! guessable id is what an off-path forger needs, and a resolver that quietly fell back to one would be
//! worse than one that says it cannot.
//!
//! # EXAMPLES
//!
//! `system_tests/src/user/name_resolver_tests.rs` plays the spawner on all three architectures:
//!
//! ```text
//! start net_stack over the e1000e NIC; start name_resolver on it, told 10.0.2.9:53 over TCP
//! GRANT badge 0x384 the zone "nife.test"; spawn name_resolver_test_client with that badge
//! client: packages.nife.test  -> OK, 10.0.2.9 (then a TCP echo to that address)
//! client: mirror.nife.test    -> OK, 10.0.2.9, through a compressed CNAME
//! client: forged.nife.test    -> REFUSED (IdMismatch)
//! client: example.com         -> DENIED, and no query was sent
//! ```
//!
//! # BUGS
//!
//! - **One request at a time.** A client's `CALL` holds this thread for the whole exchange, and
//!   `net_stack` itself serves one request at a time, so a UDP query nobody answers stalls every
//!   client of both for `net_stack`'s 15-second receive bound, three times.
//! - **No cache.** Every resolve is a query, whatever the last answer's TTL said. The TTL is handed
//!   back so a client can cache, which is where the knowledge of how long it needs an address is.
//! - **IPv4 only, no DNSSEC, no EDNS(0)**: `domain_name_system`'s BUGS, inherited.
//! - **One name server.** A second, tried when the first does not answer, wants a second endowment
//!   word and a reason to need it.
//! - **One socket id**, given at spawn, because `net_stack`'s socket numbers are shared by all its
//!   clients (milestone 649 (every client of a network stack shares its socket numbers)). The
//!   spawner picks one no other client of that stack uses.
//! - **A page attached before its badge was granted is deleted**, so a client must be spawned after
//!   the spawner's grant, which is the order a spawner works in anyway.
//! - The UDP path is proved on the host only up to the bytes (`domain_name_system`'s tests); the
//!   gating boot test asks over TCP, because slirp forwards UDP to no process a test owns.
//!
//! Name: provisional 2026-10-06 (UTC), milestone 384's lane. §248 calls it "the resolver", and
//! "name resolver" is what the act is called wherever DNS is described (RFC 1034, section 5). Refused
//! `dns_resolver` under §154 (the acronym test) and `resolver` alone, which says nothing about what
//! is resolved in a tree where capabilities are resolved too.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates: one lint
// policy)'s ratchet tracks (§107 (`missing_docs` moves to `workspace.lints.rust`)).
#![allow(missing_docs)]
#![no_main]

use abi::rights;
use domain_name_system::{Answer, Name, Query, Reject, TcpReply};
use name_resolution_protocol::{
    self as protocol, Grants, MAX_ADDRESSES, NAME_TEXT_MAX, OFF_ADDRESSES, OFF_NAME, Outcome,
    endowment, status,
};
use socket_protocol as socket;
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{
    Delivered, call, cap_delete, map_page_frame, receive_request, reply, retype_page_frame,
    send_cap,
};

/// Slot 0: where grants and requests arrive.
const SERVICE: u64 = 0;
/// Slot 1: the socket contract's endpoint.
const STACK: u64 = 1;
/// Slot 2: the untyped budget.
const MEMORY_REGION: u64 = 2;
/// Slot 3: the entropy service.
const ENTROPY: u64 = 3;

/// Where the socket page is mapped. A private pair page: nobody else knows this address.
const SOCKET_VA: u64 = address_space_map::pair_page(0x0000_0000_00A0_0000);
/// Where client window 0 is mapped; window `i` is `i` pages above it.
const CLIENT_VA: u64 = address_space_map::pair_page(0x0000_0000_00B0_0000);
/// The last window, checked against the map at compile time like the first.
const _: u64 = address_space_map::pair_page(CLIENT_VA + (protocol::GRANTS_MAX as u64 - 1) * PAGE);

// SAFETY: `attach_socket` maps one page read/write at SOCKET_VA before any accessor runs, and every
// accessor is reached only after it succeeded (`socket_ready`).
const SOCKET: MappedWindow = unsafe { MappedWindow::new(SOCKET_VA, PAGE) };

/// How many UDP queries one resolve sends before it gives up, a fresh id each. Three, the number
/// the network time client and the socket test client settled on for a lossy path.
const ATTEMPTS: u32 = 3;
/// How many datagrams one attempt reads looking for its answer, so a flood of forgeries ends the
/// attempt rather than holding the resolver forever.
const DATAGRAMS_PER_ATTEMPT: u32 = 8;

/// The largest reply over TCP this resolver reads. RFC 7766 allows 65,535 bytes; a host-name lookup
/// with eight addresses is a few hundred, and a reply past this is refused as malformed rather than
/// truncated, because over TCP there is nothing left to fall back to.
const TCP_REPLY_MAX: usize = 2048;

/// The network step that failed, in [`status::NETWORK`]'s detail byte.
mod step {
    pub const NO_SOCKET_PAGE: u8 = 1;
    pub const OPEN: u8 = 2;
    pub const SEND: u8 = 3;
    pub const CONNECT: u8 = 4;
}

/// Everything too large for a one-page stack, in `.bss`. One thread, so one reference.
struct State {
    grants: Grants,
    attached: [bool; protocol::GRANTS_MAX],
    request: [u8; 2 + domain_name_system::UDP_MESSAGE_MAX],
    reply: [u8; TCP_REPLY_MAX],
    chunk: [u8; socket::DATA_MAX],
}

static mut STATE: State = State {
    grants: Grants::new(),
    attached: [false; protocol::GRANTS_MAX],
    request: [0; 2 + domain_name_system::UDP_MESSAGE_MAX],
    reply: [0; TCP_REPLY_MAX],
    chunk: [0; socket::DATA_MAX],
};

/// Where and how to ask, from the start words.
#[derive(Clone, Copy)]
struct Server {
    ip: [u8; 4],
    port: u16,
    tcp_only: bool,
    sid: u64,
}

/// Why an exchange produced no answer: a reply was refused, or something else is wrong.
enum Fail {
    Rejected(Reject),
    Status(Outcome),
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(server: u64, port_and_transport: u64, sid: u64) -> ! {
    // SAFETY: one thread, and `_start` is the only code that names `STATE`, once, here.
    let state = unsafe { &mut *core::ptr::addr_of_mut!(STATE) };
    let (port, transport) = endowment::port_and_transport_fields(port_and_transport);
    let server = Server {
        ip: endowment::server_octets(server),
        port,
        tcp_only: transport == endowment::TRANSPORT_TCP,
        sid,
    };
    // A failure here is answered on every request rather than by exiting: a resolver that vanished
    // would leave its clients blocked in `CALL` with nothing to say why.
    let socket_ready = attach_socket(sid);
    loop {
        serve(state, &server, socket_ready);
    }
}

fn serve(state: &mut State, server: &Server, socket_ready: bool) {
    let r = receive_request(SERVICE);
    if r.badge == 0 {
        // The spawner's unbadged capability, and nothing else, can say this. A control `CALL` gets
        // an answer so a spawner that called is not left blocked; a delegation is deleted.
        let control = state.grants.control(r.w0, r.w1);
        if let Some(to) = r.delivered.into_reply() {
            let refused = matches!(control, protocol::Control::Refused(_));
            let _ = reply(to, refused as u64, 0);
        }
        return;
    }
    // A badge wider than a grant's 32 bits names no grant; refusing it here keeps a truncation from
    // aliasing it onto one that does.
    let badge = u32::try_from(r.badge).unwrap_or(0);
    match protocol::operation(r.w0) {
        protocol::OPERATION_ATTACH_PAGE_FRAME => match r.delivered {
            Delivered::Delegation(frame) => attach_client(state, badge, frame),
            other => {
                if let Some(to) = other.into_reply() {
                    let _ = reply(to, Outcome::of(status::BAD_REQUEST).word(), 0);
                }
            }
        },
        protocol::OPERATION_RESOLVE => {
            let Some(to) = r.delivered.into_reply() else {
                return;
            };
            let (outcome, ttl) = if socket_ready {
                resolve(state, server, badge, r.w1)
            } else {
                (network(step::NO_SOCKET_PAGE), 0)
            };
            let _ = reply(to, outcome.word(), ttl);
        }
        _ => {
            if let Some(to) = r.delivered.into_reply() {
                let _ = reply(to, Outcome::of(status::BAD_REQUEST).word(), 0);
            }
        }
    }
}

/// Map a client's page in its grant's window, or delete it: no grant, or a page already attached.
fn attach_client(state: &mut State, badge: u32, frame: u64) {
    match state.grants.granted(badge) {
        Some(i) if !state.attached[i] => {
            if map_page_frame(frame, window_va(i), true, MEMORY_REGION) {
                state.attached[i] = true;
            } else {
                cap_delete(frame);
            }
        }
        _ => cap_delete(frame),
    }
}

fn window_va(i: usize) -> u64 {
    CLIENT_VA + i as u64 * PAGE
}

/// One resolve, start to finish: the outcome word and the TTL.
fn resolve(state: &mut State, server: &Server, badge: u32, len: u64) -> (Outcome, u64) {
    let Some(window) = state.grants.granted(badge) else {
        return (Outcome::of(status::DENIED), 0);
    };
    if !state.attached[window] {
        return (Outcome::of(status::NOT_ATTACHED), 0);
    }
    // SAFETY: `attach_client` mapped a page read/write at this window's address before setting
    // `attached`, and no window is ever unmapped.
    let page = unsafe { MappedWindow::new(window_va(window), PAGE) };

    // Copied out before it is read, so what is judged and what is asked are the same bytes.
    let len = len as usize;
    if len == 0 || len > NAME_TEXT_MAX + 1 {
        return (Outcome::of(status::BAD_NAME), 0);
    }
    let mut text = [0u8; NAME_TEXT_MAX + 1];
    for (i, b) in text[..len].iter_mut().enumerate() {
        *b = page.r8((OFF_NAME + i) as u64);
    }
    let Some(name) = core::str::from_utf8(&text[..len])
        .ok()
        .and_then(|t| Name::from_dotted(t).ok())
    else {
        return (Outcome::of(status::BAD_NAME), 0);
    };

    // **The grant, before the network.** Nothing below this line runs for a name the badge may not
    // ask about.
    if state.grants.check(badge, &name).is_err() {
        return (Outcome::of(status::DENIED), 0);
    }

    let answer = match ask(state, server, name) {
        Ok(answer) => answer,
        Err(Fail::Status(outcome)) => return (outcome, 0),
        Err(Fail::Rejected(reject)) => return (refused(reject), 0),
    };
    let addresses = answer.addresses();
    for (k, a) in addresses.iter().enumerate().take(MAX_ADDRESSES) {
        for (j, &b) in a.iter().enumerate() {
            page.w8((OFF_ADDRESSES + 4 * k + j) as u64, b);
        }
    }
    (
        Outcome {
            status: status::OK,
            detail: 0,
            count: addresses.len() as u8,
        },
        answer.ttl() as u64,
    )
}

/// A refused reply as the client sees it: the two refusals a client acts on have their own status,
/// and the rest say which check refused.
fn refused(reject: Reject) -> Outcome {
    match reject {
        Reject::NoSuchName => Outcome::of(status::NO_SUCH_NAME),
        Reject::ServerError(rcode) => Outcome {
            status: status::SERVER_ERROR,
            detail: rcode,
            count: 0,
        },
        other => Outcome {
            status: status::REFUSED,
            detail: protocol::reject_code(other),
            count: 0,
        },
    }
}

fn network(step: u8) -> Outcome {
    Outcome {
        status: status::NETWORK,
        detail: step,
        count: 0,
    }
}

/// Ask the server about `name`: over TCP alone, or over UDP and then TCP when the UDP reply was
/// truncated. A fresh id for each query.
fn ask(state: &mut State, server: &Server, name: Name) -> Result<Answer, Fail> {
    let query = Query::for_name(transaction_id()?, name);
    if server.tcp_only {
        return over_tcp(state, server, &query);
    }
    match over_udp(state, server, &query) {
        Err(Fail::Rejected(Reject::Truncated)) => {
            let query = Query::for_name(transaction_id()?, name);
            over_tcp(state, server, &query)
        }
        other => other,
    }
}

/// Sixteen unguessable bits, or the refusal to ask at all.
fn transaction_id() -> Result<u16, Fail> {
    let no_entropy = Fail::Status(Outcome::of(status::NO_ENTROPY));
    let (r0, r1) = call(ENTROPY, entropy_protocol::req(entropy_protocol::GET, 2), 0);
    let n = entropy_protocol::delivered(r0).ok_or(no_entropy)?;
    if n < 2 {
        return Err(Fail::Status(Outcome::of(status::NO_ENTROPY)));
    }
    let mut bytes = [0u8; 2];
    entropy_protocol::take(n, r1, &mut bytes);
    Ok(u16::from_le_bytes(bytes))
}

fn over_udp(state: &mut State, server: &Server, query: &Query) -> Result<Answer, Fail> {
    let sid = server.sid;
    if call(STACK, socket::req(socket::OPERATION_OPEN_UDP, sid), 0).0 != socket::REP_OK {
        return Err(Fail::Status(network(step::OPEN)));
    }
    let result = exchange_udp(state, server, query);
    let _ = call(STACK, socket::req(socket::OPERATION_CLOSE, sid), 0);
    result
}

fn exchange_udp(state: &mut State, server: &Server, query: &Query) -> Result<Answer, Fail> {
    let n = query
        .request(&mut state.request)
        .map_err(|e| Fail::Rejected(Reject::Malformed(e)))?;
    // The last refusal of a reply that was not ours, reported only if nothing better arrives.
    let mut ignored = None;
    for _ in 0..ATTEMPTS {
        set_destination(server);
        write_payload(&state.request[..n]);
        if call(
            STACK,
            socket::req(socket::OPERATION_SENDTO, server.sid),
            n as u64,
        )
        .0 != socket::REP_OK
        {
            return Err(Fail::Status(network(step::SEND)));
        }
        for _ in 0..DATAGRAMS_PER_ATTEMPT {
            let (got, _) = call(STACK, socket::req(socket::OPERATION_RECEIVE, server.sid), 0);
            if got == socket::REP_ERR || got == 0 || got > socket::DATA_MAX as u64 {
                break; // nothing came: the next attempt
            }
            // A UDP RECEIVE reply carries the datagram's source in the destination fields
            // (socket_protocol's layout). Anything not from the server is not an answer.
            if source() != (server.ip, server.port) {
                continue;
            }
            let got = got as usize;
            read_payload(&mut state.chunk[..got]);
            match query.accept(&state.chunk[..got]) {
                Ok(answer) => return Ok(answer),
                // Not a reply to this query: a forger's, or a late one to an earlier id. Keep
                // listening; the real answer may be behind it.
                Err(
                    reject @ (Reject::IdMismatch
                    | Reject::QuestionMismatch
                    | Reject::NotAResponse
                    | Reject::Malformed(_)),
                ) => ignored = Some(reject),
                Err(reject) => return Err(Fail::Rejected(reject)),
            }
        }
    }
    Err(match ignored {
        Some(reject) => Fail::Rejected(reject),
        None => Fail::Status(Outcome::of(status::NO_ANSWER)),
    })
}

fn over_tcp(state: &mut State, server: &Server, query: &Query) -> Result<Answer, Fail> {
    let sid = server.sid;
    if call(STACK, socket::req(socket::OPERATION_OPEN_TCP, sid), 0).0 != socket::REP_OK {
        return Err(Fail::Status(network(step::OPEN)));
    }
    let result = exchange_tcp(state, server, query);
    let _ = call(STACK, socket::req(socket::OPERATION_CLOSE, sid), 0);
    result
}

fn exchange_tcp(state: &mut State, server: &Server, query: &Query) -> Result<Answer, Fail> {
    set_destination(server);
    if call(STACK, socket::req(socket::OPERATION_CONNECT, server.sid), 0).0
        != socket::CONNECT_ESTABLISHED
    {
        return Err(Fail::Status(network(step::CONNECT)));
    }
    let n = query
        .request_tcp(&mut state.request)
        .map_err(|e| Fail::Rejected(Reject::Malformed(e)))?;
    write_payload(&state.request[..n]);
    if call(
        STACK,
        socket::req(socket::OPERATION_SEND, server.sid),
        n as u64,
    )
    .0 != n as u64
    {
        return Err(Fail::Status(network(step::SEND)));
    }
    let mut framed = TcpReply::new(&mut state.reply);
    loop {
        let (got, _) = call(STACK, socket::req(socket::OPERATION_RECEIVE, server.sid), 0);
        if got == 0 || got > socket::DATA_MAX as u64 {
            // The server closed, or the stack gave up, before the reply was whole.
            return Err(Fail::Status(Outcome::of(status::NO_ANSWER)));
        }
        let got = got as usize;
        read_payload(&mut state.chunk[..got]);
        match framed.feed(&state.chunk[..got]) {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => return Err(Fail::Rejected(Reject::Malformed(e))),
        }
    }
    let message = framed.message().ok_or(Fail::Rejected(Reject::Malformed(
        domain_name_system::Error::Truncated,
    )))?;
    query.accept(message).map_err(Fail::Rejected)
}

/// Mint the socket page from the budget, map it, and delegate it to the stack at `sid`. `false`
/// at any step, and the resolver then answers every request with the step that failed.
fn attach_socket(sid: u64) -> bool {
    let frame = retype_page_frame(MEMORY_REGION);
    if frame < 0 {
        return false;
    }
    let frame = frame as u64;
    if !map_page_frame(frame, SOCKET_VA, true, MEMORY_REGION) {
        return false;
    }
    send_cap(
        STACK,
        frame,
        rights::READ | rights::WRITE,
        socket::req(socket::OPERATION_ATTACH_PAGE_FRAME, sid),
    ) >= 0
}

fn set_destination(server: &Server) {
    for (i, &b) in server.ip.iter().enumerate() {
        SOCKET.w8(socket::OFF_DST_IP + i as u64, b);
    }
    SOCKET.w16(socket::OFF_DST_PORT, server.port);
}

/// The source of the datagram a UDP `RECEIVE` just delivered.
fn source() -> ([u8; 4], u16) {
    let mut ip = [0u8; 4];
    for (i, b) in ip.iter_mut().enumerate() {
        *b = SOCKET.r8(socket::OFF_DST_IP + i as u64);
    }
    (ip, SOCKET.r16(socket::OFF_DST_PORT))
}

fn write_payload(bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        SOCKET.w8(socket::OFF_PAYLOAD + i as u64, b);
    }
}

fn read_payload(out: &mut [u8]) {
    for (i, b) in out.iter_mut().enumerate() {
        *b = SOCKET.r8(socket::OFF_PAYLOAD + i as u64);
    }
}

user_mode_runtime::panic_handler!();
