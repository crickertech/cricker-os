//! **An NTP server and the network under it, in one process** (milestone 51, split out of the
//! client's binary by milestone 290; notes/ntp.md).
//!
//! The peer `components/src/network_time_client.rs` is tested against. It holds `READ` on the
//! endpoint the client holds `WRITE` on, and it speaks the same socket contract
//! (`crates/socket_protocol/src/lib.rs`) `net_stack` does, so the client cannot tell it apart from the
//! real stack.
//!
//! # What the substitution is worth, and why it survives being a separate binary
//!
//! **The client's network path is one endpoint capability.** Substituting the peer at that
//! capability boundary runs the client's **real, unmodified code**: there is no test hook, no
//! conditional, and nothing in the client that knows which of the two it is talking to. That
//! property comes from the boundary, not from co-location, which is why milestone 290 could lift
//! this out of the client's binary and change nothing about what it proves.
//!
//! What the pair proves: the socket-contract glue (minting a frame, delegating it, the destination
//! header, `SENDTO`/`RECV` framing), that the 48 bytes on the wire are a well-formed NTPv4 client
//! packet addressed to port 123, that the nonce is unpredictable, that a reply failing
//! `Query::accept` moves nothing, and that an accepted sample becomes a *proposal* the clock
//! service judges.
//!
//! What it does not prove: that smoltcp, UDP, IPv4 and the NIC carry those bytes. Milestone 30's
//! socket-contract tests prove that path with a real datagram. Nor does it prove anything about a
//! real internet time server: nothing in QEMU's slirp answers UDP 123, and pointing the gate at a
//! public server would make it depend on somebody else's network.
//!
//! # The wiring, and the one report
//!
//! `a0` selects an [`srv`] variant, `a1` is the wall-clock nanoseconds it claims to have. **The
//! kernel supplies the claimed time** because this process holds no clock capability of its own,
//! which keeps every test's expectation an exact number rather than a window.
//!
//! Three slots and no more: the report endpoint, `READ` on the socket endpoint, and an untyped
//! budget to map the client's frame. It holds no entropy capability and no propose endpoint, so a
//! reader can see from the endowment alone that it cannot reach the clock.
//!
//! It reports **once**, after the first request it sees, and that `send` blocks until the test
//! drains it: a report per request would deadlock the exchange the moment nobody was draining, and
//! one is all the assertions need.
//!
//! Name: ratified 2026-09-14 (calef, milestone 290), his own words, lifted out of the `ntp` binary's
//! `ROLE_SERVER`. **The directory is half the name**: milestone 175 drew the line between
//! `components/` (what a distribution ships because somebody wants its function) and `fixtures/`
//! (what exists to exercise the system), and a test server sitting in `components/` was the defect
//! that line exists to prevent. `network_time` carries the stem calef ruled on 2026-09-13 for
//! `ntp_proto`, so this name was already spelled the way milestone 265 would spell the crate
//! (`network_time_protocol`, landed 2026-09-14). Refused
//! `ntp_test_server`: the acronym rule set 2026-09-05 asks whether the expansion teaches, and
//! network time does where `pci` does not, which is the same ruling that moved the crate's stem.
//! Refused `fake_stack` and `stub_net_stack`, which name what it *stands in for* rather than what
//! it is, and which understate it: it is a real server answering a real contract, and the only
//! thing false about it is that no datagram leaves the machine. The old header's claim that being
//! one binary with the client kept "the initrd's directory small" is an argument from
//! implementation convenience, which AGENTS.md ranks below everything else; it did not survive.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use abi::rendezvous;
use network_time_protocol::{Packet, Short, Timestamp, leap, mode};
// The socket contract, verbatim from the file `net_stack` compiles, so this server and the client
// cannot drift from the real server's idea of the wire format. The TCP half is dead here and is
// allowed rather than trimmed: the value of compiling the *same file* net_stack does is that the
// two cannot drift, and a per-consumer subset would throw that away.
#[allow(dead_code)]
use socket_protocol::*;
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{cap_delete, map_page_frame, recv_cap, reply, send};

// =================================================================================================
// The slots, and the one word this program reports.
//
// The report vocabulary is one numbering space across all three programs milestone 290 split the
// `ntp` binary into, and each declares only the words it sends. The numbers did not move in that
// split, so a boot log from before it still reads.
// =================================================================================================

/// Slot 0: the report endpoint (WRITE).
const REPORT: u64 = 0;
/// Slot 1: the socket contract's endpoint. This server holds `READ`; the client is given `WRITE`.
const STACK: u64 = 1;
/// Slot 2: an untyped budget, to map the client's shared frame and pay for the page tables.
const MEMORY_REGION: u64 = 2;

/// This server saw its first request. `w1` is the request's transmit field (the nonce), `w2` is
/// `(dst_port << 32) | (version << 8) | mode`.
pub const RPT_SERVED: u64 = 7;

/// Which reply this server sends.
pub mod srv {
    /// A correct, acceptable reply at the claimed time.
    pub const GOOD: u64 = 0;
    /// The origin field is the nonce with its low bit flipped: the reply of an off-path attacker
    /// who guessed wrong, and the check the whole of plain NTP's spoofing resistance rests on.
    pub const BAD_ORIGIN: u64 = 1;
    /// Stratum 0 with the kiss code `RATE`. An instruction to back off, not a time.
    pub const KISS_OF_DEATH: u64 = 2;
    /// Twenty bytes: something that is not an NTP packet arriving on our socket.
    pub const SHORT: u64 = 3;
}

/// This server's turnaround in its reply: T3 - T2. Small but not zero, so the delay arithmetic is
/// exercised on a value that has to be subtracted rather than one that cannot go wrong. Far under
/// the client's own round trip, which is two IPC exchanges.
const SERVER_TURNAROUND_NANOS: u64 = 1_000;

/// Where this server maps the client's shared frame in its own address space. The client picks its
/// own, and the two do not have to agree: what they share is the frame's *layout*, which is
/// `socket_protocol`'s.
const PAGE_FRAME_VA: u64 = address_space_map::pair_page(0x0000_0000_00A0_0000);

// SAFETY: the `OP_ATTACH_PAGE_FRAME` arm below maps one page read/write at PAGE_FRAME_VA, and it is
// the first request the client makes, before any of `WINDOW`'s accessors are reached (milestone 139).
const WINDOW: MappedWindow = unsafe { MappedWindow::new(PAGE_FRAME_VA, PAGE) };

/// `a0` selects an [`srv`] variant, `a1` is the wall-clock time in nanoseconds this server claims.
#[unsafe(no_mangle)]
pub extern "C" fn _start(variant: u64, claimed_nanos: u64) -> ! {
    server(variant, claimed_nanos)
}

/// Serve the socket contract on [`STACK`], answering each request with an NTP reply built from
/// `variant` at `claimed_nanos`. See the module docs for what this does and does not prove.
fn server(variant: u64, claimed_nanos: u64) -> ! {
    let mut pending = [0u8; network_time_protocol::PACKET_LEN];
    let mut pending_len = 0usize;
    let mut reported = false;

    loop {
        let (w0, cap, w1) = recv_cap(STACK);
        match req_op(w0) {
            // A SEND_CAP: the client's shared frame, which we map for ourselves and then drop the
            // capability for, because the mapping outlives it. No reply; nobody is waiting.
            OP_ATTACH_PAGE_FRAME => {
                map_page_frame(cap, PAGE_FRAME_VA, true, MEMORY_REGION);
                cap_delete(cap);
            }
            OP_OPEN_UDP | OP_OPEN_TCP => {
                reply(cap, REP_OK, 0);
            }
            OP_SENDTO => {
                // The length the client declared, which is how the real server learns it too.
                let mut wire = [0u8; network_time_protocol::PACKET_LEN];
                let n = read_payload(
                    (w1 as usize).min(network_time_protocol::PACKET_LEN),
                    &mut wire,
                );
                let request = Packet::parse(&wire[..n]).unwrap_or_default();
                pending_len = build_reply(&request, variant, claimed_nanos, &mut pending);
                reply(cap, REP_OK, 0);
                if !reported {
                    reported = true;
                    let dst_port = r16le(PAGE_FRAME_VA + OFF_DST_PORT) as u64;
                    send(
                        REPORT,
                        RPT_SERVED,
                        request.transmit.bits(),
                        (dst_port << 32) | ((request.version as u64) << 8) | request.mode as u64,
                    );
                }
            }
            OP_RECV => {
                write_payload(&pending[..pending_len]);
                w16le(PAGE_FRAME_VA + OFF_LEN, pending_len as u16);
                reply(cap, pending_len as u64, 0);
            }
            OP_CLOSE => {
                reply(cap, REP_OK, 0);
            }
            _ => {
                if cap != rendezvous::NO_CAP {
                    reply(cap, REP_ERR, 0);
                }
            }
        }
    }
}

/// Build the reply for `variant`. Returns how many bytes of `out` are the reply.
fn build_reply(request: &Packet, variant: u64, claimed_nanos: u64, out: &mut [u8]) -> usize {
    let t2 = stamp(claimed_nanos).unwrap_or(Timestamp::ZERO);
    let t3 = stamp(claimed_nanos + SERVER_TURNAROUND_NANOS).unwrap_or(Timestamp::ZERO);

    let mut p = Packet {
        leap: leap::NONE,
        version: network_time_protocol::VERSION,
        mode: mode::SERVER,
        stratum: 2,
        poll: request.poll,
        precision: -20,
        root_delay: Short(0x0000_1000),      // ~62 ms
        root_dispersion: Short(0x0000_2000), // ~125 ms
        reference_id: *b"TEST",
        reference: t2,
        // A server echoes the client's transmit field into origin and interprets it no further.
        // That is what makes a random nonce free, and it is what BAD_ORIGIN below breaks.
        origin: request.transmit,
        receive: t2,
        transmit: t3,
    };

    match variant {
        srv::BAD_ORIGIN => {
            p.origin = Timestamp::from_bits(request.transmit.bits() ^ 1);
        }
        srv::KISS_OF_DEATH => {
            p.stratum = 0;
            p.reference_id = *b"RATE";
        }
        srv::SHORT => {
            let bytes = p.to_bytes();
            out[..20].copy_from_slice(&bytes[..20]);
            return 20;
        }
        _ => {}
    }

    out[..network_time_protocol::PACKET_LEN].copy_from_slice(&p.to_bytes());
    network_time_protocol::PACKET_LEN
}

/// Unix nanoseconds to an NTP timestamp. `None` outside the crate's representable window. The
/// client carries the same four lines against its own clock; this one is over a number the kernel
/// handed it, so a claimed time far outside the era becomes `Timestamp::ZERO` and the client's
/// `Query::accept` refuses it, which is the behaviour a test wiring an absurd time wants.
fn stamp(unix_nanos: u64) -> Option<Timestamp> {
    Timestamp::from_unix(
        unix_nanos / clock_protocol::NANOS_PER_SEC,
        (unix_nanos % clock_protocol::NANOS_PER_SEC) as u32,
    )
}

// =================================================================================================
// The shared frame. Absolute-VA volatile access through `WINDOW` (milestone 139), the same
// abstraction socket_test_client, network_time_client, keyboard_driver, entropy and net_transport share.
// `va` is always `PAGE_FRAME_VA + <an offset constant>`, so subtracting PAGE_FRAME_VA recovers the offset
// `WINDOW` bounds-checks against.
// =================================================================================================

fn r8(va: u64) -> u8 {
    WINDOW.r8(va - PAGE_FRAME_VA)
}
fn w8(va: u64, v: u8) {
    WINDOW.w8(va - PAGE_FRAME_VA, v);
}
fn r16le(va: u64) -> u16 {
    WINDOW.r16(va - PAGE_FRAME_VA)
}
fn w16le(va: u64, v: u16) {
    WINDOW.w16(va - PAGE_FRAME_VA, v);
}

fn write_payload(bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
    }
}

fn read_payload(n: usize, out: &mut [u8]) -> usize {
    let n = n.min(out.len());
    for (i, b) in out[..n].iter_mut().enumerate() {
        *b = r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64);
    }
    n
}

user_mode_runtime::panic_handler!();
