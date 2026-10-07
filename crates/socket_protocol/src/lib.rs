#![cfg_attr(not(test), no_std)]
//! The socket contract wire format, shared by the net server (`net_stack`) and its clients (milestone
//! 30, piece 3 phase B; DECISIONS §25).
//!
//! **Each socket is its own capability** (§255 (each socket is its own capability), milestone 649
//! (every client of a network stack shares its socket numbers)). A process holds a **front door**,
//! a `Stack` endpoint capability that can make sockets and do nothing else, and one capability per
//! socket it has open. Every operation is one message:
//!
//! - `OPEN_UDP`, `OPEN_TCP`, `BIND_UDP` and `LISTEN` are `CALL`s on the front door. A success is
//!   answered by `REPLY_CAPABILITY` with the new socket's capability, which the client's `CALL` finds
//!   in `x2` (`user_mode_runtime::call_receiving`).
//! - `ATTACH_PAGE_FRAME` is a `SEND_CAP` on a socket's capability: it carries the socket's shared
//!   frame and gets no reply.
//! - every other operation is a `CALL` on a socket's capability, the opcode alone in the request
//!   word, one argument in the second word, and a reply word back. `ACCEPT`, on a listener's
//!   capability, is answered with the connection's capability as an open is.
//!
//! **The stack names a socket by the kernel-stamped badge on the capability a request arrived
//! on**, never by a number the client wrote. It mints each socket's capability from its own
//! endpoint with a badge in the [`SOCKET_BADGE`] range, from a counter it never rewinds, and drops
//! the badge at `CLOSE`, so any copy of a closed socket's capability reaches nothing. Passing a
//! socket to another program is passing its capability. Before milestone 649 a socket was a small
//! integer every client of one stack shared, so one client could send on, read, close or redirect
//! another's sockets; that finding, and the attack that booted it, are in milestone 649's block.
//!
//! **`PageFrame` layout, pinned.** One data region, reused per operation, NOT a split TX/RX ring. The
//! phase-one contract is one *synchronous* exchange per `CALL` (the client blocks in the CALL while
//! the server drives the network), so a request's payload and its reply never coexist in the frame
//! and a single region is sufficient and simpler. A split TX/RX ring becomes necessary only with
//! asynchronous or streaming sockets, which the concurrency model defers (notes/net.md).
//!
//! ```text
//!   +0x000  u8[4]  dst_ip      destination address, octets (SENDTO / CONNECT);
//!                              source address on a UDP RECEIVE reply
//!   +0x004  u16    dst_port    destination port, little-endian (SENDTO / CONNECT);
//!                              source port on a UDP RECEIVE reply
//!   +0x006  u16    len         payload length, in for SEND*/out for RECEIVE
//!   +0x008  ...    payload     up to DATA_MAX bytes
//! ```
//!
//! **A UDP `RECEIVE` reply carries the datagram's source endpoint** in the `dst_ip`/`dst_port`
//! fields. Those fields are request state that the reply never needed until a server-shaped
//! client existed: a UDP responder must see who asked, both to answer unicast and because mDNS's
//! semantics turn on the querier's source port (RFC 6762 §6.7, legacy unicast). The fields are
//! dead space on a reply, so the source rides there with no format change; a client that only
//! ever speaks to one peer can keep ignoring them, at the cost of re-writing the destination
//! before its next `SENDTO`. On a TCP `RECEIVE` they are untouched: the peer is fixed by the
//! connection and already known to whoever made or accepted it.
//!
//! # The inbound half: a listener is not a connection (milestone 107)
//!
//! `LISTEN` and `ACCEPT` add the direction the contract did not have. The shape is **not** POSIX's,
//! where a listening socket and an accepted one are both file descriptors and the only difference
//! is which calls happen to work on them. Here they are two objects because they are two
//! authorities:
//!
//! - A **listener** is the authority to *receive connections on a port*. It is port-scoped, it is
//!   exclusive (two programs cannot both hold port 80), and **it carries no shared frame at all**,
//!   because no bytes ever cross on it. That last part is not a rule anyone imposed; it falls out
//!   of §25's decision that the frame is the real granted resource. A listener has nothing to grant.
//! - A **connection** is the authority to *speak with one peer*. It is 4-tuple-scoped, and it is
//!   exactly the object `CONNECT` already produced, reached the other way round.
//!
//! So `ACCEPT` is made on the listener's capability and answered with the connection's, which the
//! client then attaches a frame to, exactly as it does to a socket `OPEN_TCP` made. This is the tree's existing habit of splitting authority by what a
//! holder can *do* rather than by what it names: `PageFrame` versus `DeviceFrame`, `WRITE` versus
//! `GRANT` on the same object.
//!
//! # Who binds the port: the listen grant
//!
//! Outbound needs no permission beyond the `Stack` capability, because an ephemeral local port is
//! allocated by the server and contended by nobody. **Inbound is different in kind:** a listening
//! port is a claim on an exclusive name in a shared namespace, which is the same thing that makes a
//! directory a capability rather than a path. A shell that could hand out "listen on 80" would be
//! handing out something categorically larger than "here is a connection".
//!
//! So the port is not the client's to choose freely. A net server is spawned with a **listen
//! grant**, an inclusive port range packed by [`listen_grant`], and refuses `LISTEN` outside it with
//! [`LISTEN_DENIED`]. [`NO_LISTEN_GRANT`] (the default, and what every outbound-only test spawns
//! with) means the server accepts nothing from anywhere: **inbound authority is granted, never
//! assumed.**
//!
//! **A fixed UDP port is the same claim, and gets the same answer** (milestone 55's mDNS stack
//! half). `OPEN_UDP` allocates an ephemeral local port, contended by nobody; an mDNS responder
//! must bind 5353, which is an exclusive name in the same shared namespace a TCP listen port is.
//! So `BIND_UDP` is checked against a **UDP bind grant**, a second inclusive range packed by
//! [`udp_bind_grant`] into the *same spawn word* (the listen grant lives in the low 32 bits, this
//! one in the high 32; the two compose with `|`), and it answers with `LISTEN`'s own vocabulary,
//! because the three outcomes are properties of claiming a port, not of TCP: granted, refused as
//! a matter of authority, or held by somebody else. The zero word still means no inbound
//! authority of either kind.
//!
//! **BUGS / limits, named here because this is where a reader meets the feature.** The grant's
//! granularity is the *stack*, not the client: every holder of a front door to one stack shares its
//! grant, because the grant is a spawn argument of the stack. A per-client grant would ride on a
//! badged front door, which the [`SOCKET_BADGE`] split leaves room for and nothing builds yet. The
//! backlog is **one connection deep** per listener (see `net_stack`'s `OPERATION_ACCEPT`, which re-arms
//! immediately), so a second connection arriving while a first is un-accepted is refused by TCP
//! rather than queued.
//! # Examples
//!
//! The claim worth checking in code is that **the two inbound authorities are independent**. They
//! ride in one spawn word and compose with `|`, which is exactly the shape that invites a reader to
//! assume one implies the other. It does not: an mDNS responder granted UDP 5353 cannot listen on
//! TCP 5353, and a web server granted TCP 80 cannot bind UDP 80.
//!
//! ```
//! use socket_protocol::{
//!     NO_LISTEN_GRANT, grant_allows, listen_grant, udp_bind_grant, udp_grant_allows,
//! };
//!
//! // What an mDNS responder is spawned with: one UDP port, no TCP listen authority at all.
//! let responder = udp_bind_grant(5353, 5353);
//! assert!(udp_grant_allows(responder, 5353));
//! assert!(!grant_allows(responder, 5353)); // the same number, and not the same authority
//!
//! // What a file server is spawned with: SMB's port, and nothing on the UDP side.
//! let smb = listen_grant(445, 445);
//! assert!(grant_allows(smb, 445));
//! assert!(!udp_grant_allows(smb, 445));
//!
//! // Both at once is one word, and neither half leaks into the other.
//! let both = smb | responder;
//! assert!(grant_allows(both, 445) && udp_grant_allows(both, 5353));
//! assert!(!grant_allows(both, 5353) && !udp_grant_allows(both, 445));
//!
//! // The default, and what every outbound-only client is spawned with: nothing inbound.
//! assert!(!grant_allows(NO_LISTEN_GRANT, 445));
//! assert!(!udp_grant_allows(NO_LISTEN_GRANT, 5353));
//! ```
//!
//! A grant is also **safe to get wrong at the spawn site**, which is why the degenerate cases
//! collapse to one value a reader can recognise rather than to a wide range:
//!
//! ```
//! use socket_protocol::{NO_LISTEN_GRANT, listen_grant};
//!
//! assert_eq!(listen_grant(0, 65535), NO_LISTEN_GRANT); // port 0 is not a port
//! assert_eq!(listen_grant(8080, 80), NO_LISTEN_GRANT); // an inverted range grants nothing
//! ```
//!
//! And the badge split is the whole of how a request is read: a badge in the socket range names a
//! socket, and any other badge is a front door, which names none.
//!
//! ```
//! use socket_protocol::{SOCKET_BADGE, names_a_socket};
//!
//! assert!(names_a_socket(SOCKET_BADGE | 1));
//! assert!(!names_a_socket(0)); // the unbadged front door a spawner hands out
//! assert!(!names_a_socket(7)); // a front door a spawner badged for its own reasons
//! ```
//!
//! Name: ratified 2026-08-01 (calef, the naming tenet), which names `socket_proto` among the
//! standard terms that are already right. It graduated from a module inside `net_stack` to a crate
//! on 2026-07-31 under rule 7, taking the spelling the suffix rule already required. That suffix
//! rule changed at milestone 265, on calef's ruling of 2026-09-05 that `_proto` is a truncation and
//! is equally short for `prototype`, so the crate is `socket_protocol` now; `socket`, the half the 2026-08-01
//! ruling was about, is untouched.

/// Operations. The opcode is the whole request word (milestone 649: it carried a socket id beside
/// it until a socket became a capability).
///
/// `SEND_CAP` on a socket's capability: delegate that socket's shared frame.
pub const OPERATION_ATTACH_PAGE_FRAME: u64 = 1;
/// `CALL` on the front door: create a UDP socket and bind an ephemeral local port. Answered
/// [`REP_OK`] with the socket's capability.
pub const OPERATION_OPEN_UDP: u64 = 2;
/// `CALL` on the front door: create a TCP socket. Answered [`REP_OK`] with the socket's capability.
pub const OPERATION_OPEN_TCP: u64 = 3;
/// `CALL`: UDP send; dst in the frame header, payload in the frame.
pub const OPERATION_SENDTO: u64 = 4;
/// `CALL`: block until a datagram/segment arrives, write it to the frame.
pub const OPERATION_RECEIVE: u64 = 5;
/// `CALL`: TCP connect to the frame's dst; reply the outcome.
pub const OPERATION_CONNECT: u64 = 6;
/// `CALL`: TCP send; payload in the frame.
pub const OPERATION_SEND: u64 = 7;
/// `CALL`: close the socket and drop its frame mapping.
pub const OPERATION_CLOSE: u64 = 8;
/// `CALL(front door, OPERATION_LISTEN, port)`: start listening on `port`. Replies one of the
/// [`LISTEN_GRANTED`] outcomes, and a granted one carries the listener's capability. Names
/// provisional (milestone 107 (the socket contract learns to accept)).
pub const OPERATION_LISTEN: u64 = 9;
/// `CALL(listener, OPERATION_ACCEPT, 0)`: block until a connection arrives on this listener, then
/// answer [`REP_OK`] carrying the connection's capability, or [`REP_ERR`]. The listener keeps
/// listening, and the connection wants a frame attached before it carries bytes.
pub const OPERATION_ACCEPT: u64 = 10;
/// `CALL(front door, OPERATION_BIND_UDP, port)`: create a UDP socket bound to the **fixed**
/// `port`, subject to the stack's [`udp_bind_grant`]. Replies the [`LISTEN_GRANTED`] vocabulary,
/// which is the port-claim vocabulary rather than a TCP one, and a granted one carries the socket's
/// capability. Name provisional (milestone 55 (Time Machine: SMB3 with Apple's extensions, and
/// mDNS)'s mDNS stack half).
pub const OPERATION_BIND_UDP: u64 = 11;

/// **The badge bit that marks a socket's capability** (§255 (each socket is its own capability);
/// name provisional). The stack mints every socket's badge with this bit set, so a request whose
/// badge has it names a socket and any other badge is a front door. That split is what stops a
/// closed socket's capability from acting as a front door: its badge is in the socket range and no
/// longer in the stack's table, so it reaches nothing at all.
pub const SOCKET_BADGE: u64 = 1 << 63;

/// Does a request that arrived with `badge` name a socket? `false` means it came through a front
/// door. Name provisional.
pub const fn names_a_socket(badge: u64) -> bool {
    badge & SOCKET_BADGE != 0
}

/// Reply words. Non-negative is success (RECEIVE returns the length here); the connect outcomes are
/// their own small vocabulary so the client can tell "refused" from "connected".
pub const REP_OK: u64 = 0;
/// `OPERATION_CONNECT` succeeded.
pub const CONNECT_ESTABLISHED: u64 = 0;
/// The peer sent RST, or the connect otherwise failed or closed.
pub const CONNECT_REFUSED: u64 = 1;
/// A failure sentinel (a capability that names no live socket, a bad operation, or a server-side
/// timeout). High bit set so
/// it is never mistaken for a length or a connect outcome.
pub const REP_ERR: u64 = 1 << 32;

/// `LISTEN`'s outcomes, its own small vocabulary for the same reason `CONNECT` has one: a client
/// that cannot tell "I was never granted this port" from "somebody else holds it" cannot report
/// anything useful, and the two call for opposite responses (ask for authority, versus pick another
/// port).
pub const LISTEN_GRANTED: u64 = 0;
/// The port is outside this stack's [`listen_grant`]. **The capability refusal**: no retry, no
/// other port in range will help unless the caller was granted it.
pub const LISTEN_DENIED: u64 = 1;
/// The port is inside the grant but another socket already listens there. Exclusivity is the whole
/// reason a port is a grantable thing; this is what enforcing it looks like from the client side.
pub const LISTEN_IN_USE: u64 = 2;

/// No inbound authority at all, and the default a net server is spawned with.
pub const NO_LISTEN_GRANT: u64 = 0;

/// Pack an inclusive listen-port range into the one word a net server is spawned with. `lo == 0` is
/// [`NO_LISTEN_GRANT`] whatever `hi` says, because port 0 is not a port and a grant that starts at
/// "no port" grants nothing; that keeps the "no grant" case a single value a reader can recognise.
pub const fn listen_grant(lo: u16, hi: u16) -> u64 {
    if lo == 0 || hi < lo {
        return NO_LISTEN_GRANT;
    }
    (lo as u64) | ((hi as u64) << 16)
}

/// May a holder of a stack carrying `grant` listen on `port`? The whole authority check, in one
/// place both the server and its tests read, so a server-side widening cannot pass a test that
/// keeps its own copy of the range.
pub const fn grant_allows(grant: u64, port: u16) -> bool {
    let lo = (grant & 0xffff) as u16;
    let hi = ((grant >> 16) & 0xffff) as u16;
    lo != 0 && port >= lo && port <= hi
}

/// Pack an inclusive **UDP bind** port range into the high half of the spawn word (milestone 55's
/// mDNS stack half; the [`listen_grant`] holds the low half, and the two compose with `|`). The
/// same degenerate cases as the listen grant, for the same reasons: `lo == 0` grants nothing, and
/// an inverted range is a spawn-site mistake whose safe reading is "grants nothing".
pub const fn udp_bind_grant(lo: u16, hi: u16) -> u64 {
    if lo == 0 || hi < lo {
        return NO_LISTEN_GRANT;
    }
    ((lo as u64) << 32) | ((hi as u64) << 48)
}

/// May a holder of a stack carrying `grant` bind a UDP socket to the fixed `port`? [`grant_allows`]
/// for the word's UDP half; the two halves are independent authorities, so granting a TCP listen
/// range grants no UDP port and the reverse.
pub const fn udp_grant_allows(grant: u64, port: u16) -> bool {
    let lo = ((grant >> 32) & 0xffff) as u16;
    let hi = ((grant >> 48) & 0xffff) as u16;
    lo != 0 && port >= lo && port <= hi
}

/// **Where a network stack finds the two capabilities §255 (each socket is its own capability)
/// added to its spawn**, which its spawners (the progenitor, and the kernel's test harness) and the
/// stack itself must agree on, so they are here under rule 7 rather than spelled per program. Names
/// provisional.
pub mod stack_slots {
    /// The stack's own endpoint again, `WRITE | GRANT`: what each socket's capability is minted
    /// from with `BADGE`, which keeps the source's rights. Not the `READ` copy the stack serves on,
    /// because a socket minted from that could take other clients' requests.
    pub const MINT: u64 = 7;
    /// The stack's own address space, `WRITE`: so `CLOSE` can `UNMAP` a socket's page before its
    /// table entry takes another client's.
    pub const OWN_SPACE: u64 = 8;
}

/// `PageFrame` header offsets.
pub const OFF_DST_IP: u64 = 0x000;
/// The destination port, right after the destination IP.
pub const OFF_DST_PORT: u64 = 0x004;
/// The payload length.
pub const OFF_LEN: u64 = 0x006;
/// Where the payload bytes start.
pub const OFF_PAYLOAD: u64 = 0x008;

/// The largest payload the frame carries (a 4 KiB frame minus the header).
pub const DATA_MAX: usize = 4096 - OFF_PAYLOAD as usize;

/// **The inbound gate's shared facts**, in one place because four programs have to agree on them
/// and three of them are separate binaries: `socket_test_client` (the hand-written client),
/// `std_exerciser` (the `std::net` client, in its own workspace), and the kernel test that decides
/// which ports each stack is granted. The same discipline as `filesystem_protocol::fixture`, and rule 7's
/// reason verbatim: a port number two binaries agree on is a crate, never a literal repeated per
/// call site.
///
/// **What is deliberately NOT here is the host's copy**, and that is the interesting half.
/// `xtask`'s prober spells `IN_MSG` and `OUT_MSG` again as its own literals, exactly as the
/// pinned [MS-NLMP] vectors beside the credential tests are spelled twice: the gate's claim is that
/// two independently-written sides agree, and a shared constant would let one edit move both. The
/// port is spelled a third time in `helpers/qemu-runner-*.sh`, where a `hostfwd` names the guest
/// side, because a shell script cannot read a Rust crate. A drift in either is loud (the prober
/// reports "the guest served 0 of N"), which is the honest cost of the split.
pub mod fixture {
    /// The port the inbound gate listens on, and the guest side of the runners' `hostfwd`. Both
    /// ISA legs use the same number because they run one after the other and never hold it at once.
    pub const LISTEN_PORT: u16 = 7778;
    /// A port deliberately **outside** every listen grant the tests hand out, so asking for it
    /// proves the grant *refuses* rather than that nothing happened to bind. It is the whole of the
    /// authority claim: a client that could bind this bound a port nothing granted.
    pub const DENIED_PORT: u16 = 8080;

    /// What the host sends in, and what the guest answers with. Different strings on purpose: an
    /// echo would pass even if the guest were only reflecting the host's own bytes, and the point
    /// of the gate is that the guest *composed* an answer to a connection it did not make.
    pub const IN_MSG: &[u8] = b"nife-in!";
    /// What the guest answers with, composed rather than echoed (see the field-group docs).
    pub const OUT_MSG: &[u8] = b"nife-out!";

    /// How many connections one listener serves before the client stops. **Two, and the second is
    /// the load-bearing one**: a listener that accepts exactly one connection would pass a
    /// one-round gate and is precisely what a file server cannot use.
    pub const ROUNDS: usize = 2;

    /// **The runners' TCP echo peer**: a `guestfwd` inside QEMU's user-mode network that pipes
    /// every connection to `10.0.2.9:7777` into a fresh `/bin/cat`
    /// (`helpers/qemu-runner-aarch64.sh`, and its riscv64 twin). Two binaries connect to it, the
    /// kernel harness's `socket_test_client` and the prompt's `network_echo_client` (milestone 590
    /// (the booted system starts its network stack)), which is what put it here rather than in
    /// each of them. The runners spell it a
    /// third time for [`LISTEN_PORT`]'s reason.
    pub const ECHO_PEER_IP: [u8; 4] = [10, 0, 2, 9];
    /// The echo peer's port; see [`ECHO_PEER_IP`].
    pub const ECHO_PEER_PORT: u16 = 7777;

    /// **The runners' package source**: a `guestfwd` that runs `helpers/package-http-peer` once
    /// per connection to `10.0.2.9:8080`, serving packages over HTTP/1.0 (milestone 198 (a package
    /// manager) rung 3a). Two binaries dial it, the kernel harness's `socket_test_client` and the
    /// progenitor's `package install <name>`, which is what put it here (rule 7). Only reachable
    /// inside a QEMU runner: a booted system has no other package source yet, and that is
    /// notes/packages.md's BUGS rather than a property of this address.
    pub const PACKAGE_PEER_IP: [u8; 4] = [10, 0, 2, 9];
    /// The package source's port; see [`PACKAGE_PEER_IP`]. The same number as [`DENIED_PORT`], and
    /// unrelated to it: that is a port a *guest* may not listen on, this is one on a *host* the
    /// guest dials.
    pub const PACKAGE_PEER_PORT: u16 = 8080;
    /// The `Host` header a request to the package source carries: [`PACKAGE_PEER_IP`], spelled.
    pub const PACKAGE_PEER_HOST: &str = "10.0.2.9";
}

/// **The lease report** `net_stack` sends its spawner once DHCP completes, word by word. Provisional
/// (2026-10-04, milestone 384 (in a capability system the resolver is a grant)), as is the name:
/// §248 (the name resolver is its own confined program) gave the nameserver DHCP hands out a
/// consumer, the resolver's spawner, so it rides the report's second word, which was zero.
///
/// - word 0: the leased IPv4 address, big-endian in the low 32 bits, unchanged since milestone 30
///   (the network stack as a confined component)
/// - word 1: the first DNS server the lease named, the same encoding, or [`lease::NO_NAMESERVER`]
/// - word 2: zero, and reserved
///
/// A spawner that ignores word 1 is unaffected, which is why this did not need a new message.
///
/// ```
/// use socket_protocol::lease;
///
/// let word = lease::ipv4_word([10, 0, 2, 3]);
/// assert_eq!(lease::word_ipv4(word), Some([10, 0, 2, 3]));
/// assert_eq!(lease::word_ipv4(lease::NO_NAMESERVER), None);
/// ```
pub mod lease {
    /// Word 1 when the lease named no DNS server. 0.0.0.0 is never a nameserver, so it cannot
    /// collide with one.
    pub const NO_NAMESERVER: u64 = 0;

    /// An IPv4 address as a report word: big-endian in the low 32 bits.
    pub const fn ipv4_word(octets: [u8; 4]) -> u64 {
        u32::from_be_bytes(octets) as u64
    }

    /// A report word back to an address, or `None` for [`NO_NAMESERVER`] or a word with any of the
    /// high 32 bits set, which no encoder here produces.
    pub const fn word_ipv4(word: u64) -> Option<[u8; 4]> {
        if word == NO_NAMESERVER || word > u32::MAX as u64 {
            return None;
        }
        Some((word as u32).to_be_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lease_word_round_trips_and_refuses_what_no_encoder_writes() {
        assert_eq!(lease::ipv4_word([10, 0, 2, 3]), 0x0a00_0203);
        assert_eq!(lease::word_ipv4(0x0a00_0203), Some([10, 0, 2, 3]));
        assert_eq!(lease::word_ipv4(lease::NO_NAMESERVER), None);
        assert_eq!(lease::word_ipv4(1 << 32), None);
        assert_eq!(lease::word_ipv4(u32::MAX as u64), Some([255; 4]));
    }

    /// Every opcode the contract defines, in one place. It was written out three times before
    /// milestone 107 added two more, and a list repeated per property is a list that will one day
    /// be missing an entry in exactly the property that would have caught it.
    const OPERATIONS: &[u64] = &[
        OPERATION_ATTACH_PAGE_FRAME,
        OPERATION_OPEN_UDP,
        OPERATION_OPEN_TCP,
        OPERATION_SENDTO,
        OPERATION_RECEIVE,
        OPERATION_CONNECT,
        OPERATION_SEND,
        OPERATION_CLOSE,
        OPERATION_LISTEN,
        OPERATION_ACCEPT,
        OPERATION_BIND_UDP,
    ];

    /// Opcodes are distinct from each other and from zero, and stay small. The request word is the
    /// opcode alone now, so a wide opcode costs nothing on the wire, but zero is what an empty
    /// word reads as and must never mean an operation.
    #[test]
    fn every_opcode_is_nonzero_and_small() {
        for &operation in OPERATIONS {
            assert!(operation != 0 && operation <= 0xff, "opcode {operation}");
        }
    }

    /// Opcodes are distinct. Two sharing a number is one operation silently performing another.
    #[test]
    fn opcodes_are_distinct() {
        for (i, a) in OPERATIONS.iter().enumerate() {
            for b in &OPERATIONS[i + 1..] {
                assert_ne!(a, b, "two opcodes share a number");
            }
        }
    }

    /// **The socket range and the front door never overlap.** A badge either names a socket or is
    /// a front door; if one value could be both, a closed socket's capability would open new
    /// sockets, or a front door would reach someone's socket.
    #[test]
    fn a_badge_is_a_socket_or_a_front_door_and_never_both() {
        assert!(!names_a_socket(0), "the unbadged front door names a socket");
        for b in [1u64, 7, 1 << 62, (1 << 63) - 1] {
            assert!(!names_a_socket(b), "front door badge {b:#x} names a socket");
        }
        for b in [SOCKET_BADGE, SOCKET_BADGE | 1, u64::MAX] {
            assert!(
                names_a_socket(b),
                "socket badge {b:#x} reads as a front door"
            );
        }
    }

    /// The shared frame's header fields do not overlap, and the payload starts after all of them.
    /// An overlap here means a destination port written over an address, which would send bytes to
    /// the wrong host rather than failing.
    #[test]
    fn the_frame_header_fields_do_not_overlap() {
        let mut offs: [u64; 3] = [OFF_DST_IP, OFF_DST_PORT, OFF_LEN];
        offs.sort_unstable();
        for w in offs.windows(2) {
            assert_ne!(w[0], w[1], "two header fields share an offset");
        }
        for o in offs {
            assert!(o < OFF_PAYLOAD, "header field {o} runs into the payload");
        }
    }

    /// The payload is exactly the rest of the frame: the header, then bytes to the last one on the
    /// page. Larger and a full-size send writes past the end of the frame the client was granted;
    /// smaller and part of a granted page is unreachable, which nothing anywhere would report.
    #[test]
    fn a_full_payload_fills_the_shared_frame_exactly() {
        assert_eq!(
            OFF_PAYLOAD as usize + DATA_MAX,
            4096,
            "the payload is not the whole frame minus its header"
        );
    }

    /// The connect outcomes are distinguishable from each other and from the generic reply codes,
    /// which is the whole reason they have their own vocabulary.
    #[test]
    fn connect_outcomes_are_their_own_vocabulary() {
        assert_ne!(CONNECT_ESTABLISHED, CONNECT_REFUSED);
        assert_ne!(CONNECT_ESTABLISHED, REP_ERR);
        assert_ne!(CONNECT_REFUSED, REP_ERR);
    }

    /// The same for `LISTEN`'s three outcomes, and for the same reason: a client that cannot tell
    /// a refusal of *authority* from a collision on a port cannot do the right thing about either.
    #[test]
    fn listen_outcomes_are_their_own_vocabulary() {
        let outcomes = [LISTEN_GRANTED, LISTEN_DENIED, LISTEN_IN_USE, REP_ERR];
        for (i, a) in outcomes.iter().enumerate() {
            for b in &outcomes[i + 1..] {
                assert_ne!(a, b, "two listen outcomes share a value");
            }
        }
    }

    /// **A grant admits exactly its range and nothing else.** This is the authority check itself,
    /// so an off-by-one here is a server binding a port it was never granted, which no other test
    /// in the tree would see: the on-device gate asks for one allowed port and one denied one.
    #[test]
    fn a_listen_grant_admits_its_range_and_refuses_everything_outside_it() {
        let g = listen_grant(7778, 7780);
        assert!(!grant_allows(g, 7777), "one below the range was allowed");
        for p in 7778..=7780 {
            assert!(grant_allows(g, p), "port {p} inside the range was refused");
        }
        assert!(!grant_allows(g, 7781), "one above the range was allowed");
        assert!(
            !grant_allows(g, 80),
            "a well-known port outside was allowed"
        );

        // A single-port grant is the common case (one server, one port) and the tightest one.
        let one = listen_grant(80, 80);
        assert!(grant_allows(one, 80));
        assert!(!grant_allows(one, 81));
    }

    /// **No grant means no port.** The default a server is spawned with must admit nothing, or
    /// "inbound authority is granted, never assumed" is a sentence in a doc comment and not a
    /// property of the system. Port 0 is refused under every grant, including one that names it,
    /// because it is not a port anything can listen on.
    #[test]
    fn the_default_grant_admits_no_port_at_all() {
        for p in [0u16, 1, 80, 7778, 65535] {
            assert!(
                !grant_allows(NO_LISTEN_GRANT, p),
                "port {p} was allowed with no grant"
            );
        }
        assert_eq!(listen_grant(0, 65535), NO_LISTEN_GRANT);
        assert!(!grant_allows(listen_grant(0, 65535), 7778));
        assert!(!grant_allows(listen_grant(1, 65535), 0));
        // An inverted range is a mistake at the spawn site, and the safe reading of a mistake in a
        // grant is "grants nothing", never "grants the ports between them in the other order".
        assert_eq!(listen_grant(7780, 7778), NO_LISTEN_GRANT);
    }

    /// **The UDP bind grant admits exactly its range**, the twin of the listen-grant test above
    /// and for the same reason: this is the authority check itself, and the on-device gate asks
    /// for one allowed port and one denied one, so an off-by-one here is a server binding a UDP
    /// port it was never granted with no other test to see it.
    #[test]
    fn a_udp_bind_grant_admits_its_range_and_refuses_everything_outside_it() {
        let g = udp_bind_grant(5353, 5355);
        assert!(
            !udp_grant_allows(g, 5352),
            "one below the range was allowed"
        );
        for p in 5353..=5355 {
            assert!(
                udp_grant_allows(g, p),
                "port {p} inside the range was refused"
            );
        }
        assert!(
            !udp_grant_allows(g, 5356),
            "one above the range was allowed"
        );

        // The single-port grant is the one milestone 55 actually spawns (mDNS's 5353).
        let one = udp_bind_grant(5353, 5353);
        assert!(udp_grant_allows(one, 5353));
        assert!(!udp_grant_allows(one, 5354));

        // The degenerate cases read as "grants nothing", exactly as the listen grant's do.
        assert_eq!(udp_bind_grant(0, 65535), NO_LISTEN_GRANT);
        assert_eq!(udp_bind_grant(5355, 5353), NO_LISTEN_GRANT);
        for p in [0u16, 1, 5353, 65535] {
            assert!(
                !udp_grant_allows(NO_LISTEN_GRANT, p),
                "port {p} allowed with no grant"
            );
        }
    }

    /// **The two grants share one word and neither leaks into the other.** They are different
    /// authorities (receive TCP connections on a port, versus claim a fixed UDP port), so a spawn
    /// that grants one must not have granted the other, and the composed word must answer each
    /// question from its own half only. A packing overlap here is a stack granting mDNS's port to
    /// anything that was granted a TCP listen range, silently.
    #[test]
    fn the_listen_and_udp_bind_grants_compose_without_leaking() {
        let listen_only = listen_grant(7778, 7778);
        assert!(grant_allows(listen_only, 7778));
        assert!(
            !udp_grant_allows(listen_only, 7778),
            "a listen grant granted a UDP bind"
        );

        let udp_only = udp_bind_grant(5353, 5353);
        assert!(udp_grant_allows(udp_only, 5353));
        assert!(
            !grant_allows(udp_only, 5353),
            "a UDP bind grant granted a TCP listen"
        );

        let both = listen_grant(7778, 7778) | udp_bind_grant(5353, 5353);
        assert!(grant_allows(both, 7778));
        assert!(udp_grant_allows(both, 5353));
        assert!(
            !grant_allows(both, 5353),
            "the UDP half answered a listen question"
        );
        assert!(
            !udp_grant_allows(both, 7778),
            "the listen half answered a UDP question"
        );

        // The top of the port space survives the high half's packing.
        let top = udp_bind_grant(65535, 65535);
        assert!(udp_grant_allows(top, 65535));
        assert!(!udp_grant_allows(top, 65534));
    }

    /// **The inbound fixture's own claim, checked**: the port the gate is granted is inside the
    /// grant the tests hand out and the port it probes is outside it. Both device legs spend
    /// minutes proving the refusal on real hardware-shaped paths, and neither of them would notice
    /// if somebody edited [`fixture::DENIED_PORT`] to a number inside the range: the client would
    /// bind it, print that it bound it, and every assertion would still pass. This is the check
    /// that a "denied" port is denied.
    #[test]
    fn the_inbound_fixture_probes_a_port_no_test_grants() {
        let g = listen_grant(fixture::LISTEN_PORT, fixture::LISTEN_PORT);
        assert!(grant_allows(g, fixture::LISTEN_PORT));
        assert!(
            !grant_allows(g, fixture::DENIED_PORT),
            "the port the gate calls denied is inside the grant it is handed",
        );
        // And the wider range the combined inbound boot hands out (the SMB port rides beside the
        // listen port) must not reach it either.
        let wide = listen_grant(fixture::LISTEN_PORT, fixture::LISTEN_PORT + 1);
        assert!(!grant_allows(wide, fixture::DENIED_PORT));
        // The default admits neither, which is what "granted, never assumed" means for the fixture.
        assert!(!grant_allows(NO_LISTEN_GRANT, fixture::LISTEN_PORT));
    }

    /// A grant round-trips through the one word a spawn argument carries, up to the top of the
    /// port space. It rides in a `u64` beside nothing else, so the only way to lose a bit is to
    /// pack it wrong.
    #[test]
    fn a_grant_survives_the_word_it_is_spawned_with() {
        for (lo, hi) in [
            (1u16, 65535u16),
            (49152, 65535),
            (65535, 65535),
            (7778, 7778),
        ] {
            let g = listen_grant(lo, hi);
            assert!(grant_allows(g, lo), "the low end was lost for {lo}..={hi}");
            assert!(grant_allows(g, hi), "the high end was lost for {lo}..={hi}");
        }
    }
}
