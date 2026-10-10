//! **A hostile second client of a network stack** (milestone 649 (every client of a network stack
//! shares its socket numbers); names here provisional).
//!
//! Milestone 800 (a non-Anthropic model attacks the confinement claim)'s fourth outsider pass
//! booted the escape this program used to perform, on PR #1798: when a socket was a small number
//! every client of one stack shared, a second client could attach its own page at the number
//! another client was about to use, and that client's traffic then ran through the squatter's page,
//! both directions. Since §255 (each socket is its own capability) a socket is a capability the
//! stack mints and names by its kernel-stamped badge, so there is no number left to squat. This
//! program is the same squatter, rewritten against the contract as it now stands, trying every way
//! it has of reaching a socket somebody else opened while that socket is open.
//!
//! It is endowed like any stack client (report endpoint, `WRITE` on the stack's front door, an
//! untyped budget: the three slots `socket_test_client` gets), and it:
//!
//! 1. attaches its own page through the **front door**, the move that used to land on the victim's
//!    socket number;
//! 2. asks the front door for every operation that names a socket (`SENDTO`, `SEND`, `CONNECT`,
//!    `ACCEPT`, `CLOSE`, `RECEIVE`), each of which must be refused, because the front door names
//!    none;
//! 3. tries to mint a socket's capability itself with `BADGE`, which its front door has no `GRANT`
//!    for;
//! 4. opens a socket of its own and tries a kernel `RECEIVE` and `RECEIVE_CAP` on its capability,
//!    refused as minted without `READ`; re-badges it, closes it, and tries the closed one (nothing);
//! 5. arms a valid TFTP read request in its own page, reports which of those were refused, and
//!    watches its page for the victim's traffic, bounded.
//!
//! `system_tests/src/user/net_confinement_tests.rs` runs it while an honest TFTP client holds a
//! socket open, and asserts that every attempt was refused, that the honest client's exchange went
//! through untouched, and that nothing of it reached this program's page.
//!
//! It rides in the `net_stack` binary beside `socket_test_client` for the same reason that client
//! does: the initrd directory holds at most 15 files on aarch64. That makes the `net_stack` image
//! the booted system runs carry a hostile client nothing in the boot ever starts, which is an
//! exception to keeping test roles out of shipped images, recorded here so the next hostile fixture
//! does not extend it unmarked.
//!
//! Name: provisional, milestone 649's lane, 2026-10-07 (UTC), keeping the file name milestone 800's
//! lane gave the program it replaces.
//!
//! # Capability contract
//! - slot 0: the report endpoint (WRITE)
//! - slot 1: the stack's front door (WRITE), exactly what any client holds
//! - slot 2: an untyped budget, to mint and map its page

#![allow(missing_docs)]

use abi::rights;
use socket_protocol::*;
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{
    badge, call, call_receiving, cap_delete, exit, map_page_frame, now, receive, receive_cap,
    retype_page_frame, send, send_cap, yield_now,
};

use crate::socket_test_client::TFTP_NAME;

pub const ROLE: u64 = 8;

const REPORT: u64 = 0;
const STACK: u64 = 1;
const MEMORY_REGION: u64 = 2;

/// Word 0 of the first report: the attempts are made and the watch begins. Word 1 is the bitmask
/// of attempts that were refused (below).
pub const RPT_ARMED: u64 = 1;
/// Word 0 of the final report: did the victim's traffic land in this program's page?
pub const RPT_LEAKED: u64 = 1;
pub const RPT_QUIET: u64 = 0;

// Word 2 of that report (`recv_refused`) sets one bit per kernel-IPC probe on the squatter's own
// socket capability that was refused: bit 0 a plain `RECEIVE`, bit 1 a `RECEIVE_CAP`. Both set is
// the held world (the minted socket capability carries no `READ`, so neither can drain the stack).
//
// Word 1 of that report sets one bit per attempt that was refused, ten in all. In order: the six
// socket operations on the front door (`SENDTO`, `SEND`, `CONNECT`, `ACCEPT`, `CLOSE`, `RECEIVE`);
// minting a socket with `BADGE` on the front door; re-badging its own socket; its own socket after
// `CLOSE` (`RECEIVE`); and the closed socket asked to open another.

const PAGE_FRAME_VA: u64 = address_space_map::pair_page(0x0000_0000_00A0_0000);

/// slirp's gateway hosts the built-in TFTP server; the literals `socket_test_client` dials, so the
/// bytes armed here are the request the victim itself would send.
const GW_IP: [u8; 4] = [10, 0, 2, 2];
const TFTP_PORT: u16 = 69;

/// How long to watch the page, in seconds. The victim's exchange completes well inside this once
/// it is told to go; the bound ends this program in the world the test asserts.
const WATCH_SECS: u64 = 5;

// SAFETY: `run` maps this page writable at PAGE_FRAME_VA before any read or write below.
static WINDOW: MappedWindow = unsafe { MappedWindow::new(PAGE_FRAME_VA, PAGE) };

/// **The squat, against capabilities.** See the module doc for the five steps.
pub fn run() -> ! {
    let frame = retype_page_frame(MEMORY_REGION);
    if frame < 0 || !map_page_frame(frame as u64, PAGE_FRAME_VA, true, MEMORY_REGION) {
        send(REPORT, 0, 0xE001, 0);
        exit();
    }
    let frame = frame as u64;

    // 1. The old squat: a page through the front door. The stack drops it, and no reply could say
    //    so, so the proof is step 5's watch.
    let _ = send_cap(
        STACK,
        frame,
        rights::READ | rights::WRITE,
        OPERATION_ATTACH_PAGE_FRAME,
    );

    // 2. Every operation that names a socket, on the front door. `RECEIVE` after `CLOSE`: on an
    //    open socket a `RECEIVE` waits out the stack's bounded wait for a datagram, so if the front
    //    door did reach the victim's socket, asking it first would turn a refusal the test can name
    //    into a report that never came.
    let mut refused = 0u64;
    let operations = [
        OPERATION_SENDTO,
        OPERATION_SEND,
        OPERATION_CONNECT,
        OPERATION_ACCEPT,
        OPERATION_CLOSE,
        OPERATION_RECEIVE,
    ];
    for (bit, &operation) in operations.iter().enumerate() {
        if call(STACK, operation, 0).0 == REP_ERR {
            refused |= 1 << bit;
        }
    }

    // 3. Mint a socket's capability: the front door has no `GRANT`.
    if badge(STACK, SOCKET_BADGE | 1) < 0 {
        refused |= 1 << 6;
    }

    // 4. A socket of our own: re-badging it is refused (a badge is set once), and once closed it
    //    reaches nothing, not even the front door's power to open.
    //
    //    The two kernel-IPC probes are new ground, risk 7's fifth outsider pass (2026-10-10 UTC).
    //    The capability the stack mints for a socket is a copy of the stack's own serve endpoint,
    //    carrying the socket's badge (`socket_protocol::stack_slots::MINT`). If that copy carried
    //    `READ`, a kernel plain `RECEIVE` or `RECEIVE_CAP` on it would dequeue the stack's own
    //    incoming queue, every other client's request to the stack, which is the capture class
    //    milestone 649 closed reached by IPC rather than by a squatted page. The mint omits `READ`
    //    (it is `WRITE | GRANT`), and the kernel refuses a receive on a `READ`-less endpoint
    //    (`kernel/src/syscall.rs`, `RECEIVE`/`RECEIVE_CAP`), so both return a negative error at
    //    once rather than blocking. A permissive kernel would instead block here, which the test
    //    reads as the squatter never arming.
    let mut recv_refused = 0u64;
    if let (REP_OK, _, Some(own)) = call_receiving(STACK, OPERATION_OPEN_UDP, 0) {
        if (receive(own).0 as i64) < 0 {
            recv_refused |= 1 << 0;
        }
        if (receive_cap(own).0 as i64) < 0 {
            recv_refused |= 1 << 1;
        }
        if badge(own, SOCKET_BADGE | 2) < 0 {
            refused |= 1 << 7;
        }
        let _ = call(own, OPERATION_CLOSE, 0);
        if call(own, OPERATION_RECEIVE, 0).0 == REP_ERR {
            refused |= 1 << 8;
        }
        match call_receiving(own, OPERATION_OPEN_UDP, 0) {
            (REP_ERR, _, None) => refused |= 1 << 9,
            (_, _, Some(opened)) => cap_delete(opened),
            _ => {}
        }
        cap_delete(own);
    }

    // 5. Arm a valid request in our page and watch it for the victim's reply.
    let mut p = OFF_PAYLOAD;
    let mut put = |v: u8| {
        WINDOW.w8(p, v);
        p += 1;
    };
    put(0x00);
    put(0x01); // TFTP RRQ
    for &c in TFTP_NAME {
        put(c);
    }
    put(0x00);
    for &c in b"octet" {
        put(c);
    }
    put(0x00);
    for (i, &b) in GW_IP.iter().enumerate() {
        WINDOW.w8(OFF_DST_IP + i as u64, b);
    }
    WINDOW.w16(OFF_DST_PORT, TFTP_PORT);
    WINDOW.w16(OFF_LEN, 0);

    send(REPORT, RPT_ARMED, refused, recv_refused);

    let deadline = now() + WATCH_SECS * user_mode_runtime::cntfrq();
    while now() < deadline && WINDOW.r16(OFF_LEN) == 0 {
        yield_now();
    }
    let len = WINDOW.r16(OFF_LEN);
    let opcode = ((WINDOW.r8(OFF_PAYLOAD) as u16) << 8) | WINDOW.r8(OFF_PAYLOAD + 1) as u16;
    send(
        REPORT,
        if len != 0 { RPT_LEAKED } else { RPT_QUIET },
        (opcode as u64) << 32 | len as u64,
        0,
    );
    exit();
}
