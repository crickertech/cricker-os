//! **The raw-keystroke primitive** (milestone 169): `OP_RAWMODE` and `OP_READRAW`, proved against
//! a real `line_editor` process the way this tree proves everything, with a client that asks for
//! raw mode, sends real keystrokes, and gets them back unbuffered.
//!
//! `raw_mode_service::start` plays both the input driver and the application on one `TERM`
//! endpoint (the contract already allows this: DECISIONS §21's endpoint-only naming means nobody
//! on the other end can tell who is calling) and a fake console that never reads its own shared
//! page, so a test can sentinel-fill that page and check it untouched: the same witness-page
//! discipline `c_seam`'s confiner tests use, applied to "nothing was echoed" instead of "nothing
//! outside a grant changed".

use raw_mode_service as svc;

use super::*;
use crate::sched;

const OP_ON: u64 = 1;
const OP_OFF: u64 = 0;

/// Sentinel byte a test fills a shared page with before an exchange it expects to leave that page
/// untouched. Not `0`, so a page that was merely zeroed (e.g. by `line_editor`'s own boot state)
/// cannot be mistaken for a page a test actually observed staying put.
const SENTINEL: u8 = 0xaa;

fn fill(phys: u64, byte: u8, len: usize) {
    let base = mmu::phys_to_virt(phys);
    for i in 0..len {
        // SAFETY: `phys` is a frame this test allocated and is the only writer while the
        // exchange under test runs.
        unsafe { core::ptr::write_volatile((base + i as u64) as *mut u8, byte) };
    }
}

fn read(phys: u64, len: usize) -> [u8; 8] {
    let base = mmu::phys_to_virt(phys);
    let mut out = [0u8; 8];
    for (i, b) in out.iter_mut().enumerate().take(len) {
        // SAFETY: same frame, read after the exchange's reply has already come back, so any
        // write `line_editor` was going to make already happened (the reply is ordered after it:
        // `con.flush()` runs before `reply` in every arm that touches `Con`).
        *b = unsafe { core::ptr::read_volatile((base + i as u64) as *const u8) };
    }
    out
}

fn rawmode(term: sched::RendezvousId, on: bool) {
    let w0 = line_editor::proto::req(
        line_editor::proto::OP_RAWMODE,
        if on { OP_ON } else { OP_OFF },
    );
    let r = sched::ipc_call(term, [w0, 0]);
    assert_eq!(r[0], 0, "OP_RAWMODE did not reply 0");
}

/// Send `bytes` (at most 8) as the input driver would, packed exactly as `OP_BYTES` requires.
fn send_bytes(term: sched::RendezvousId, bytes: &[u8]) -> [u64; 3] {
    assert!(bytes.len() <= 8);
    let mut w1 = 0u64;
    for (i, &b) in bytes.iter().enumerate() {
        w1 |= (b as u64) << (8 * i);
    }
    let w0 = line_editor::proto::req(line_editor::proto::OP_BYTES, bytes.len() as u64);
    sched::ipc_call(term, [w0, w1])
}

fn read_raw(term: sched::RendezvousId) -> (usize, [u8; 8]) {
    let w0 = line_editor::proto::req(line_editor::proto::OP_READRAW, 0);
    let r = sched::ipc_call(term, [w0, 0]);
    (r[0] as usize, r[1].to_le_bytes())
}

/// **Raw mode suppresses echo, and `OP_READRAW` hands back exactly what was sent, unbuffered.**
///
/// This is the milestone's central claim, and it is proven both ways so it cannot be vacuous.
/// The negative half (raw mode: the console page is untouched) would also pass if `line_editor`
/// simply never echoed anything, ever, so the positive control (canonical mode: the same bytes
/// through the same page DO land there) is what proves the sentinel methodology actually detects
/// an echo when there is one.
#[test_case]
fn raw_mode_suppresses_echo_and_delivers_bytes_unbuffered() {
    let (w, held) = svc::start();

    rawmode(w.term, true);
    fill(w.console_phys, SENTINEL, 8);
    let r = send_bytes(w.term, b"hi");
    assert_eq!(
        r[0], 0,
        "a raw OP_BYTES must reply 0, like the canonical path"
    );
    assert_eq!(
        &read(w.console_phys, 2)[..2],
        &[SENTINEL, SENTINEL],
        "raw mode must not echo: the console page moved",
    );

    let (n, bytes) = read_raw(w.term);
    assert_eq!(n, 2, "OP_READRAW returned the wrong byte count");
    assert_eq!(
        &bytes[..2],
        b"hi",
        "OP_READRAW did not return exactly what was sent"
    );

    // The positive control: same bytes, canonical mode, same page. If this did not move, the
    // negative check above would have been meaningless.
    rawmode(w.term, false);
    fill(w.console_phys, SENTINEL, 8);
    send_bytes(w.term, b"hi");
    assert_eq!(
        &read(w.console_phys, 2)[..2],
        b"hi",
        "canonical mode must still echo: the sentinel methodology itself is broken if this fails",
    );
    held.release_or_fail("raw_mode_service");
}

/// **Raw mode delivers control bytes literally; it does not interpret them as editing commands.**
///
/// `^C` is the sharpest witness available: in the line discipline it is intercepted, discards the
/// line, prints `"^C\r\n"`, and bumps `OP_INTRCOUNT`'s counter (DECISIONS §24). None of that may
/// happen in raw mode: the byte `0x03` must reach the application exactly as `kilo` needs it to
/// (real kilo binds its own quit key, not SIGINT, because termios raw mode disables `ISIG`).
#[test_case]
fn raw_mode_delivers_control_bytes_literally() {
    let (w, held) = svc::start();
    rawmode(w.term, true);
    fill(w.console_phys, SENTINEL, 8);

    send_bytes(w.term, &[0x03]);
    assert_eq!(
        read(w.console_phys, 8)[0],
        SENTINEL,
        "^C must not print anything in raw mode",
    );
    let (n, bytes) = read_raw(w.term);
    assert_eq!(n, 1);
    assert_eq!(
        bytes[0], 0x03,
        "^C must arrive as a literal byte in raw mode"
    );

    let intr_w0 = line_editor::proto::req(line_editor::proto::OP_INTRCOUNT, 0);
    let r = sched::ipc_call(w.term, [intr_w0, 0]);
    assert_eq!(
        r[0], 0,
        "raw mode's ^C must not bump the line discipline's interrupt counter"
    );

    // An escape sequence a VT arrow key sends is three more bytes the line discipline would
    // otherwise consume whole and echo nothing for; raw mode must hand back all three raw.
    fill(w.console_phys, SENTINEL, 8);
    send_bytes(w.term, b"\x1b[D");
    assert_eq!(
        read(w.console_phys, 8)[0],
        SENTINEL,
        "no echo for an escape sequence either"
    );
    let (n, bytes) = read_raw(w.term);
    assert_eq!(n, 3);
    assert_eq!(
        &bytes[..3],
        b"\x1b[D",
        "the escape sequence must arrive byte for byte, unparsed"
    );
    held.release_or_fail("raw_mode_service");
}

/// **The two input models refuse each other.** `OP_READLINE` while raw mode is on, and
/// `OP_READRAW` while it is off, are both protocol violations refused with `BAD_REQUEST`, the
/// exact refusal a second concurrent `OP_READLINE` already gets: a client that mixed them up
/// fails fast rather than hanging on a reply that will never come.
#[test_case]
fn the_two_input_models_refuse_each_other() {
    let (w, held) = svc::start();

    let readraw_w0 = line_editor::proto::req(line_editor::proto::OP_READRAW, 0);
    let r = sched::ipc_call(w.term, [readraw_w0, 0]);
    assert_eq!(
        r[0],
        line_editor::proto::BAD_REQUEST,
        "OP_READRAW while raw mode is off must be refused",
    );

    rawmode(w.term, true);
    let readline_w0 = line_editor::proto::req(line_editor::proto::OP_READLINE, 0);
    let r = sched::ipc_call(w.term, [readline_w0, 0]);
    assert_eq!(
        r[0],
        line_editor::proto::BAD_REQUEST,
        "OP_READLINE while raw mode is on must be refused",
    );
    held.release_or_fail("raw_mode_service");
}

/// **A read parked before any byte arrives is still answered correctly once one does.**
///
/// This drives the actual parking path `OP_READLINE` already relies on (`deliver`'s twin,
/// `deliver_raw`): a second thread calls `OP_READRAW` first, with the raw queue empty, and blocks
/// in the kernel exactly as a `CALL` with no waiting server does. The main thread then sends the
/// byte. Both orderings the scheduler could actually choose (the reader's call reaching
/// `line_editor` before or after the byte does) produce the same correct report, so this test is
/// not racy on its assertion; the bounded wait before sending is only to bias real runs toward
/// exercising the parked case rather than the immediate one, which the two tests above already
/// cover.
#[test_case]
fn a_raw_read_parked_before_data_arrives_still_gets_it() {
    let (w, held) = svc::start();
    rawmode(w.term, true);

    let report = sched::create_rendezvous();
    let term = w.term;
    sched::spawn(move || {
        let (n, bytes) = read_raw(term);
        sched::ipc_send(report, [n as u64, u64::from_le_bytes(bytes), 0]);
    })
    .expect("could not spawn the raw reader");

    // Bias toward the parked interleaving: give the reader thread real wall-clock time to reach
    // and block in its CALL before this thread sends anything. Not load-bearing for correctness
    // (see the doc comment above), only for which code path a given run actually exercises.
    let deadline = crate::arch::timer::now() + crate::arch::timer::frequency() / 20; // 50ms
    while crate::arch::timer::now() < deadline {
        sched::yield_now();
    }

    send_bytes(w.term, &[0x42]);

    let [n, packed, ..] = sched::ipc_recv(report);
    assert_eq!(n, 1, "the parked reader did not get exactly one byte");
    assert_eq!(
        packed.to_le_bytes()[0],
        0x42,
        "the parked reader got the wrong byte"
    );
    held.release_or_fail("raw_mode_service");
}

/// **Switching mode abandons the line in progress, in both directions**, so a session can never
/// resume half a line typed under the mode it just left.
///
/// Entering raw mode with an `OP_READLINE` parked must fail that read rather than hang it
/// forever (raw mode would never generate the `Event::Line` it is waiting for). Leaving raw mode
/// with an `OP_READRAW` parked must fail that one the same way.
#[test_case]
fn switching_mode_abandons_a_parked_read_of_the_other_kind() {
    let (w, held) = svc::start();

    // A parked OP_READLINE, abandoned by entering raw mode.
    let report = sched::create_rendezvous();
    let term = w.term;
    sched::spawn(move || {
        let w0 = line_editor::proto::req(line_editor::proto::OP_READLINE, 0);
        let r = sched::ipc_call(term, [w0, 0]);
        sched::ipc_send(report, [r[0], r[1], 0]);
    })
    .expect("could not spawn the readline waiter");
    let deadline = crate::arch::timer::now() + crate::arch::timer::frequency() / 20;
    while crate::arch::timer::now() < deadline {
        sched::yield_now();
    }
    rawmode(w.term, true);
    let [r0, ..] = sched::ipc_recv(report);
    assert_eq!(
        r0,
        line_editor::proto::BAD_REQUEST,
        "entering raw mode must fail a parked OP_READLINE rather than hang it",
    );

    // A parked OP_READRAW, abandoned by leaving raw mode.
    let report2 = sched::create_rendezvous();
    let term2 = w.term;
    sched::spawn(move || {
        let (n, bytes) = read_raw(term2);
        sched::ipc_send(report2, [n as u64, u64::from_le_bytes(bytes), 0]);
    })
    .expect("could not spawn the readraw waiter");
    let deadline = crate::arch::timer::now() + crate::arch::timer::frequency() / 20;
    while crate::arch::timer::now() < deadline {
        sched::yield_now();
    }
    rawmode(w.term, false);
    let [n2, ..] = sched::ipc_recv(report2);
    assert_eq!(
        n2,
        line_editor::proto::BAD_REQUEST,
        "leaving raw mode must fail a parked OP_READRAW rather than hang it",
    );
    held.release_or_fail("raw_mode_service");
}

/// **`OP_WRITE` is not raw-mode-gated, and `OP_READLINE` works normally once raw mode is off
/// again.** `kilo` depends on the first (it redraws the whole screen with `OP_WRITE` on every
/// keystroke, in raw mode, the entire time it runs); the second is the check that raw mode is a
/// mode switch and not a one-way door: a terminal that left it broken behind would fail every
/// program written before this milestone.
#[test_case]
fn op_write_ignores_raw_mode_and_op_readline_survives_a_round_trip() {
    let (w, held) = svc::start();

    rawmode(w.term, true);
    let text = b"kilo redraws through OP_WRITE while raw mode is on";
    let base = mmu::phys_to_virt(w.app_out_phys);
    for (i, &b) in text.iter().enumerate() {
        // SAFETY: `app_out_phys` is the frame `line_editor` maps read-only at its own APP_OUT_VA;
        // this test is the only writer, and no request naming it is in flight yet.
        unsafe { core::ptr::write_volatile((base + i as u64) as *mut u8, b) };
    }
    let w0 = line_editor::proto::req(line_editor::proto::OP_WRITE, text.len() as u64);
    let r = sched::ipc_call(w.term, [w0, 0]);
    assert_eq!(
        r[0],
        text.len() as u64,
        "OP_WRITE must work while raw mode is on"
    );
    assert_eq!(
        &read(w.console_phys, 8)[..4],
        b"kilo",
        "the written text must reach the console page exactly as line mode would print it",
    );

    // Leaving raw mode restores OP_READLINE. Type a line's worth of raw bytes first (as the input
    // driver would, before anyone asked to read a line -- exactly the type-ahead case) and confirm
    // it comes back through OP_READLINE once a read is posted, which only works if turning raw
    // mode off actually re-enabled the line discipline rather than leaving OP_BYTES stuck.
    rawmode(w.term, false);
    send_bytes(w.term, b"hi\r");
    let w0 = line_editor::proto::req(line_editor::proto::OP_READLINE, 0);
    let r = sched::ipc_call(w.term, [w0, 0]);
    assert_eq!(
        r[0], 2,
        "OP_READLINE must return the type-ahead line's length"
    );
    let base = mmu::phys_to_virt(w.app_in_phys);
    let got: [u8; 2] = core::array::from_fn(|i| {
        // SAFETY: `app_in_phys` is the frame `line_editor` maps read/write at its own APP_IN_VA,
        // and the OP_READLINE reply above is ordered after `line_editor` wrote it there.
        unsafe { core::ptr::read_volatile((base + i as u64) as *const u8) }
    });
    assert_eq!(
        &got, b"hi",
        "OP_READLINE did not deliver the line line_editor assembled"
    );
    held.release_or_fail("raw_mode_service");
}
