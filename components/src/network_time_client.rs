//! **The network time client** (milestone 51, split out of one binary by milestone 290;
//! DECISIONS §43 and §44, notes/ntp.md).
//!
//! The component that turns `network_time_protocol`'s wire format into an actual clock correction. It holds a
//! **network capability** and a capability to **propose** a time. It does not hold the clock.
//!
//! Its test server is `fixtures/src/network_time_test_server.rs` and the witness that proves the
//! missing slot is `fixtures/src/unwritable_clock_witness.rs`. Until milestone 290 all three were
//! roles of this one binary, dispatched on `arg0`.
//!
//! # The endowment is the argument
//!
//! ```text
//!   entropy ──an endpoint──►┌──────────────┐──an endpoint──► net_stack ──► the network
//!   (8 random bytes,        │  this client │ (the socket contract, UDP 123)
//!    the nonce)             └──────┬───────┘
//!                                  │ an endpoint: PROPOSE
//!                          ┌───────▼────────┐
//!                          │ clock service  │──writes──► the clock page
//!                          └────────────────┘   (this process holds NO mapping of it,
//!                                                read-only or otherwise)
//! ```
//!
//! Five slots, and the interesting one is the slot that is missing. There is no clock page here,
//! writable or not, so "set the time" is not an operation this process can express: the only thing
//! it can do with the wall clock is ask, and `clock_protocol::policy` answers. **A compromised NTP
//! client can lie inside the service's bounds and can do nothing else.** In Unix `ntpd` runs as root
//! and may set the clock to anything at all. `an_ntp_client_holds_no_writable_clock_page` in
//! `kernel/src/user/ntp_tests.rs` proves the claim the way the machine proves things: a process
//! given **these five slots and nothing else**, plus the address at which a *setter* maps the clock
//! page, writes there and dies of a fault.
//!
//! **What keeps that proof honest is the endowment, not the binary.** Until milestone 290 the
//! witness was a role of this file, and this header said the proof rested on it being *the same
//! binary*. It does not, and the correction is worth having because it was believed: the fault is
//! caused by the capability set, so any process holding this endowment faults at that address
//! whatever code it runs. What stops the witness drifting is that both processes are endowed by
//! **one function** (`spawn_with_client_endowment` in `kernel/src/user/ntp_service.rs`), which takes
//! the image as a parameter. A sixth slot added to the client is a sixth slot the witness gets, and
//! there is no second capability list anywhere to forget to update. See milestone 290's block.
//!
//! **There is no test-only branch in this program**, and since milestone 290 that sentence needs no
//! qualification. It used to sit on the same screen as `ROLE_PROBE_CLOCK`, a test-only branch in
//! this binary, which is the contradiction 290 was minted to end.
//!
//! # The nonce comes from the entropy service, or the client stops
//!
//! `Query::with_nonce` exists because a server never interprets the client's transmit timestamp; it
//! copies it into the origin field and does nothing else. So 64 random bits on the wire, with the
//! true send time kept locally, take an off-path attacker from about twelve bits of guesswork to
//! sixty-four (notes/ntp.md).
//!
//! That is worth exactly as much as the bits are unguessable, and until 2026-07-30 the only source
//! here was splitmix64 seeded off the virtual counter, which notes/entropy.md calls "predictable to
//! anyone who could guess boot-relative time". So the nonce is drawn from the entropy service
//! (DECISIONS §44), and a client with no entropy capability **refuses to send anything**: it reports
//! [`RPT_NO_ENTROPY`] and exits. Falling back to the weak stream would be §42's silent degradation
//! in the one place where the whole point of the value is that nobody can predict it, and it is the
//! same call `SystemRng` makes when it panics rather than degrading.
//!
//! # One shot, because there is no timed wait
//!
//! The kernel's syscall surface is `EXIT`, `YIELD`, `INVOKE`, `CAP_DELETE`. There is no sleep, no
//! timeout, and no deadline anywhere in it (the milestone 51 block's open fork), so a poll interval
//! is a yield-spin: a thread that stays runnable for the whole interval and costs scheduler work in
//! proportion to it. At NTP's ordinary 64-second poll that is not a service anybody should ship, so
//! this is a **one-shot synchroniser**: up to [`ATTEMPTS`] requests a few milliseconds apart, one
//! proposal, exit. A long-running client is the timed-wait fork's to build, not a workaround to
//! invent here.
//!
//! **A kiss-o'-death is not retried.** Stratum 0 is an instruction rather than a time (`RATE` means
//! back off, `DENY` means go away), and a client that retries into one is the abusive client the
//! packet exists to stop.
//!
//! # What the pair proves, and what it does not
//!
//! The test server is **both an NTP server and the network**: it holds `READ` on the endpoint this
//! client was given `WRITE` on, and it speaks the same socket contract
//! (`crates/socket_protocol/src/lib.rs`) `net_stack` does. The client cannot tell, and that is the
//! point rather than a convenience: its network path is one endpoint capability, so substituting
//! the peer at that boundary runs the client's **real, unmodified code**. That property comes from
//! the capability boundary and not from co-location, which is exactly why splitting the two
//! binaries in milestone 290 cost nothing.
//!
//! What that proves: the socket-contract glue (minting a frame, delegating it, the destination
//! header, `SENDTO`/`RECV` framing), that the 48 bytes on the wire are a well-formed NTPv4 client
//! packet addressed to port 123, that the nonce is unpredictable, that a reply failing
//! `Query::accept` moves nothing, and that an accepted sample becomes a *proposal* the clock service
//! judges.
//!
//! What it does not prove: that smoltcp, UDP, IPv4 and the NIC carry those bytes. Milestone 30's
//! socket-contract tests prove that path with a real datagram, and milestone 51 deliberately did not
//! re-prove it. Nor does it prove anything about a real internet time server: nothing in QEMU's
//! slirp answers UDP 123, and pointing the gate at a public server would make it depend on somebody
//! else's network. The honest summary is that the client is proven against a server we wrote, over a
//! network we wrote, and the parts we did not write are proven elsewhere.
//!
//! Name: ratified 2026-09-14 (calef, milestone 290), his own words, replacing `ntp`. The old binary
//! was three programs dispatched on `arg0` and the header justified that shape as "how every other
//! multi-part program here is packed and keeps the initrd's directory small", which is an argument
//! from implementation convenience and AGENTS.md ranks it below everything else. Refused keeping
//! `ntp` for the typed-command latitude: nothing types this name, it is loaded from the archive by
//! the kernel's wiring, so the latitude that produced `mdr` does not reach it. `network_time`
//! carries the stem calef ruled on 2026-09-13 for `ntp_proto`, so these three names were already
//! spelled the way milestone 265 would spell the crate, and 265 did not have to rename them. That also
//! **overtakes 265's "the `ntp` program stays `ntp`" exception**, which was written when there was
//! one program to keep the short name: after 290 there is no `ntp` program, the client's name is
//! expanded, and the pair `network_time_protocol`/`network_time_client` does not disagree.
//! `client` rather than `synchroniser` because a client is what NTP's own mode field calls it, and
//! the reader who knows the protocol meets the word it uses.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use abi::rights;
use clock_protocol::propose;
use network_time_protocol::{Query, Reject, Timestamp};
// The socket contract, verbatim from the file `net_stack` compiles, so this client and the test
// server cannot drift from the real server's idea of the wire format.
// An NTP client speaks the UDP half of the contract and never the TCP half, so the rest of the
// file is dead here. Allowed rather than trimmed: the value of compiling the *same file* net_stack
// does is that the two cannot drift, and a per-consumer subset would throw that away.
#[allow(dead_code)]
use socket_protocol::*;
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{
    call, cntfrq, exit, map_page_frame, now, retype_page_frame, send, send_cap, yield_now,
};

// =================================================================================================
// The slots, and the words this program reports.
//
// The report vocabulary is one numbering space across all three programs milestone 290 split this
// file into, and each declares only the words it sends. The numbers did not move in that split, so
// a boot log from before it still reads. `kernel/src/user/ntp_service.rs`'s `rpt` module is the
// whole vocabulary in one place, as the kernel side of a wiring always is here.
// =================================================================================================

/// Slot 0: the report endpoint (WRITE).
const REPORT: u64 = 0;
/// Slot 1: the socket contract's endpoint. **This client's whole network authority.** The client
/// holds `WRITE`, the test server `READ`.
const STACK: u64 = 1;
/// Slot 2: an untyped budget, to mint and map the one shared frame.
const MEMORY_REGION: u64 = 2;
/// Slot 3: the clock service's **propose** endpoint (WRITE). Not a page, and not a way to set
/// anything.
const PROPOSE: u64 = 3;
/// Slot 4: the entropy service's endpoint (WRITE): "you may obtain randomness", naming no device.
const ENTROPY: u64 = 4;

/// A sample was accepted and proposed. `w1` is the `clock_protocol::status` the service answered with,
/// `w2` the nanoseconds proposed.
pub const RPT_SYNCED: u64 = 1;
/// A reply arrived and `Query::accept` refused it. `w1` is a [`reject_code`], `w2` the number of
/// requests sent. **Nothing was proposed.**
pub const RPT_REJECTED: u64 = 2;
/// No reply after [`ATTEMPTS`] requests. `w1` is the number sent.
pub const RPT_NO_REPLY: u64 = 3;
/// **No unguessable bits, so no request.** `w1` is the entropy service's reply word (or the kernel
/// error from a `CALL` on an empty slot, which is what "no capability" looks like).
pub const RPT_NO_ENTROPY: u64 = 4;
/// The socket contract refused us. `w1` names the step, so a wiring mistake is not a mystery.
pub const RPT_NET_ERROR: u64 = 5;
/// The local wall clock is outside the range an NTP timestamp can hold, so no exchange is possible.
pub const RPT_BAD_LOCAL_TIME: u64 = 8;

/// How many requests one synchronisation makes before giving up. Three is a client's ordinary
/// behaviour on a lossy path, not a widened timeout; the DNS check in `socket_test_client` settled on the same
/// number for the same reason.
pub const ATTEMPTS: u32 = 3;

/// The gap between attempts, as a yield-spin, because there is no timed wait (see the module docs).
/// Milliseconds rather than NTP's 64 seconds: the interval a real client wants is precisely the
/// thing this kernel cannot yet express without keeping a thread runnable for the whole of it.
const RETRY_GAP_NANOS: u64 = 2 * 1_000_000;

/// Where this client maps the shared socket frame in its own address space. Above the program's
/// segments; the same address `socket_test_client` uses, and each address space is its own. The
/// test server picks its own, and the two do not have to agree: what they share is the frame's
/// *layout*, which is `socket_protocol`'s.
const PAGE_FRAME_VA: u64 = address_space_map::pair_page(0x0000_0000_00A0_0000);

// SAFETY: this program's own `PageFrame::MAP` (in `attach_page_frame`, before the frame is touched)
// mapped one page read/write at PAGE_FRAME_VA before any of `WINDOW`'s accessors are called
// (milestone 139).
const WINDOW: MappedWindow = unsafe { MappedWindow::new(PAGE_FRAME_VA, PAGE) };

/// The one socket id this client uses.
const SID: u64 = 0;

/// `a0` is the server's IPv4 packed big-endian, `a1` its UDP port.
#[unsafe(no_mangle)]
pub extern "C" fn _start(server_ip: u64, server_port: u64) -> ! {
    client(server_ip, server_port)
}

/// Report and stop. One-shot, so it **exits** rather than parking: a role left spinning on a run
/// queue starves later tests on the same core, which is the finding `socket_test_client`'s `done` records.
fn done(w0: u64, w1: u64, w2: u64) -> ! {
    send(REPORT, w0, w1, w2);
    exit();
}

fn client(server_ip: u64, server_port: u64) -> ! {
    // The wall clock, read the only way this process can read it: by asking the service. A reader
    // would hold the clock page and do two loads and an add, but the whole point of this process is
    // that it holds no mapping of that page, so `propose::STATE` is its channel. It costs one round
    // trip, and it is taken once: the ambient monotonic counter carries the time from here.
    let (_state, wall0) = call(PROPOSE, propose::req(propose::STATE), 0);
    let local = LocalClock {
        wall0,
        mono0: monotonic_nanos(),
    };

    // **The nonce before the network.** A client that cannot obtain unguessable bits has nothing to
    // send, so it finds that out before it exercises any network authority at all, rather than
    // reaching the same refusal one socket later.
    let mut nonce = match nonce_bits() {
        Ok(bits) => bits,
        Err(word) => done(RPT_NO_ENTROPY, word, 0),
    };

    attach_page_frame();
    if call(STACK, req(OP_OPEN_UDP, SID), 0).0 != REP_OK {
        done(RPT_NET_ERROR, 2, 0);
    }

    let mut sent = 0u32;
    let mut last_reject = 0u64;
    while sent < ATTEMPTS {
        // A fresh nonce every attempt after the first. Reusing one would let a reply to an earlier
        // request answer a later one, which is the stale-reply half of the origin check's job.
        if sent > 0 {
            nonce = match nonce_bits() {
                Ok(bits) => bits,
                Err(word) => done(RPT_NO_ENTROPY, word, 0),
            };
        }

        // T1 as late as it can honestly be taken. The bytes on the wire do not depend on it (a
        // request carries the nonce, not the send time), so everything that can be done first is,
        // and what remains between this reading and the datagram leaving is one IPC. What is left
        // over lands in the measured delay, which is the same path asymmetry NTP already lives with.
        write_dst(server_ip, server_port);
        let Some(t1) = stamp(local.now()) else {
            done(RPT_BAD_LOCAL_TIME, local.now(), 0);
        };
        let query = Query::with_nonce(t1, Timestamp::from_bits(nonce));
        write_payload(&query.request());
        sent += 1;
        if call(
            STACK,
            req(OP_SENDTO, SID),
            network_time_protocol::PACKET_LEN as u64,
        )
        .0 != REP_OK
        {
            done(RPT_NET_ERROR, 3, sent as u64);
        }

        let (n, _) = call(STACK, req(OP_RECV, SID), 0);
        if n == REP_ERR || n == 0 {
            poll_gap();
            continue;
        }

        // T4, read as close to the arrival as this side can manage: the RECV reply is the first
        // instruction after the bytes landed.
        let Some(t4) = stamp(local.now()) else {
            done(RPT_BAD_LOCAL_TIME, local.now(), 0);
        };
        let mut wire = [0u8; network_time_protocol::PACKET_LEN];
        let n = read_payload(n as usize, &mut wire);

        match query.accept(&wire[..n], t4) {
            Ok(sample) => {
                // The correction, applied to the local clock as it is now rather than as it was at
                // T4: the proposal is a claim about the present, and the service will compare it
                // against the present.
                let corrected = local.now() as i128 + sample.offset.nanos();
                let proposed = corrected.clamp(0, u64::MAX as i128) as u64;
                let (status, _wall_after) = call(PROPOSE, propose::req(propose::PROPOSE), proposed);
                let _ = call(STACK, req(OP_CLOSE, SID), 0);
                done(RPT_SYNCED, status, proposed);
            }
            Err(reject) => {
                last_reject = reject_code(reject);
                // A kiss-o'-death is an instruction, and the instruction is to stop.
                if matches!(reject, Reject::KissOfDeath(_)) {
                    break;
                }
                poll_gap();
            }
        }
    }

    let _ = call(STACK, req(OP_CLOSE, SID), 0);
    if last_reject != 0 {
        done(RPT_REJECTED, last_reject, sent as u64);
    }
    done(RPT_NO_REPLY, sent as u64, 0)
}

/// **Eight unguessable bytes, or an error word that says why not.**
///
/// `entropy_protocol::delivered` is what makes the two failures distinguishable with no probe: a byte
/// count is `0..=8`, and every error a `CALL` can return is one of the kernel's small negatives,
/// which read as enormous `u64`s. So `Err(0)` is "the service has no entropy" and `Err(huge)` is
/// "there is no entropy service in slot 4". Both stop the client; only the report tells them apart.
fn nonce_bits() -> Result<u64, u64> {
    let (r0, r1) = call(ENTROPY, entropy_protocol::req(entropy_protocol::GET, 8), 0);
    let Some(n) = entropy_protocol::delivered(r0) else {
        return Err(r0);
    };
    if n < 8 {
        return Err(r0);
    }
    let mut bytes = [0u8; 8];
    entropy_protocol::take(n, r1, &mut bytes);
    Ok(u64::from_le_bytes(bytes))
}

/// The local wall clock, anchored once against the monotonic counter.
///
/// `wall0` is what the clock service said at `mono0`; everything after that is counter arithmetic,
/// which is ambient and costs one instruction. When the service says the clock is `UNKNOWN` it
/// answers 0, and the arithmetic still works out: T1 and T4 are then measured from 1970, the offset
/// comes out as the whole distance to the server's clock, and the proposal lands on the server's
/// time. That is the bootstrap case, and it is the case a machine with no RTC is in.
struct LocalClock {
    wall0: u64,
    mono0: u64,
}

impl LocalClock {
    fn now(&self) -> u64 {
        self.wall0
            .saturating_add(monotonic_nanos().saturating_sub(self.mono0))
    }
}

/// Monotonic nanoseconds since boot, from the ambient counter.
///
/// Whole seconds and a remainder, not `ticks * 1e9`: at 62.5 MHz the naive product overflows a
/// `u64` about five minutes into a boot, and the clock built on it jumps backwards by six centuries.
/// The clock service's copy of this arithmetic carries the same note.
fn monotonic_nanos() -> u64 {
    let freq = cntfrq();
    let ticks = now();
    let secs = ticks / freq;
    let rem = ticks % freq;
    secs * clock_protocol::NANOS_PER_SEC + rem * clock_protocol::NANOS_PER_SEC / freq
}

/// Unix nanoseconds to an NTP timestamp. `None` outside the crate's representable window, which the
/// clock service's 2100 ceiling keeps us well inside; it is checked rather than assumed because the
/// alternative to checking is a timestamp 136 years out.
fn stamp(unix_nanos: u64) -> Option<Timestamp> {
    Timestamp::from_unix(
        unix_nanos / clock_protocol::NANOS_PER_SEC,
        (unix_nanos % clock_protocol::NANOS_PER_SEC) as u32,
    )
}

/// The poll interval, as the only thing this kernel can express: a yield-spin. See the module docs;
/// keeping a thread runnable for the whole interval is why a continuously polling client waits on
/// the timed-wait fork rather than being written around it.
fn poll_gap() {
    let deadline = monotonic_nanos() + RETRY_GAP_NANOS;
    while monotonic_nanos() < deadline {
        yield_now();
    }
}

/// A stable small integer for each `Reject`, so a test (and one day a log) can name which check
/// refused a packet. The enum is the security story of unauthenticated NTP; collapsing it to a bool
/// at the first opportunity would throw that away.
fn reject_code(r: Reject) -> u64 {
    match r {
        Reject::Length(_) => 1,
        Reject::Version(_) => 2,
        Reject::Mode(_) => 3,
        Reject::KissOfDeath(_) => 4,
        Reject::Stratum(_) => 5,
        Reject::Unsynchronised => 6,
        Reject::OriginMismatch => 7,
        Reject::TransmitZero => 8,
        Reject::ServerOrderReversed => 9,
        Reject::ClientOrderReversed => 10,
        Reject::NegativeDelay => 11,
        Reject::RootDistance => 12,
    }
}

/// Mint one frame from our own budget, map it, and delegate it to the socket contract's server.
/// Exactly what `socket_test_client` does, because it is exactly the same contract.
fn attach_page_frame() {
    // RETYPE answers with the new frame capability's slot, or a negative error.
    let frame = retype_page_frame(MEMORY_REGION);
    if frame < 0 {
        done(RPT_NET_ERROR, 0, 0);
    }
    let frame = frame as u64;
    // Map it writable; the page tables come from our untyped.
    if !map_page_frame(frame, PAGE_FRAME_VA, true, MEMORY_REGION) {
        done(RPT_NET_ERROR, 1, 0);
    }
    // Delegate it, narrowed to read/write, with the ATTACH request.
    if send_cap(
        STACK,
        frame,
        rights::READ | rights::WRITE,
        req(OP_ATTACH_PAGE_FRAME, SID),
    ) < 0
    {
        done(RPT_NET_ERROR, 1, 0);
    }
}

// =================================================================================================
// The shared frame. Absolute-VA volatile access through `WINDOW` (milestone 139), the same
// abstraction socket_test_client, network_time_test_server, keyboard_driver, entropy and net_transport share.
// `va` is always `PAGE_FRAME_VA + <an offset constant>`, so subtracting PAGE_FRAME_VA recovers the offset
// `WINDOW` bounds-checks against.
// =================================================================================================

fn r8(va: u64) -> u8 {
    WINDOW.r8(va - PAGE_FRAME_VA)
}
fn w8(va: u64, v: u8) {
    WINDOW.w8(va - PAGE_FRAME_VA, v);
}
fn w16le(va: u64, v: u16) {
    WINDOW.w16(va - PAGE_FRAME_VA, v);
}

/// Write the destination header: the address and port the *wiring* named, not one this program
/// chose. Which server to ask is an endowment, the same way which network to use is.
fn write_dst(ip: u64, port: u64) {
    for i in 0..4 {
        w8(PAGE_FRAME_VA + OFF_DST_IP + i, (ip >> (24 - 8 * i)) as u8);
    }
    w16le(PAGE_FRAME_VA + OFF_DST_PORT, port as u16);
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
