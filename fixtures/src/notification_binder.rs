//! **A thread that binds a notification to itself and waits on a message or a signal at once**
//! (milestone 151 (notification objects), DECISIONS §101 (notification objects)), driven through the real syscall boundary.
//!
//! Slots: 0 a memory region to mint from, 1 a `ThreadControlBlock` capability naming *this* thread
//! (the kernel test mints it, because a running thread otherwise holds none to itself), 2 an
//! endpoint to receive on, 3 a report endpoint.
//!
//! Every check that can be made from inside is a `check`, which traps: a failed check must be
//! indistinguishable from a broken program. What only the other side can see (a wake that arrived
//! while this thread was blocked, and a sender's attempt to forge a notification) is reported on
//! slot 3 as `(w0, w1, tag)`, where `tag` is 1 for a notification and 0 for a message, and the
//! kernel test asserts it.
//!
//! Name: provisional (milestone 151's lane, 2026-09-26).

#![no_std]
// Program entry points, not the crates/ library surface the ratchet of milestone 68 (code-quality gates) tracks
// (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)): each `[[bin]]` is its own crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{
    Received, exit, notification_bind, notification_poll, notification_signal, notification_wait,
    recv_bound, retype_object, send, send_cap,
};

const MEMORY_REGION: u64 = 0;
const MYSELF: u64 = 1;
const ENDPOINT: u64 = 2;
const REPORT: u64 = 3;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    let n = retype_object(MEMORY_REGION, abi::objtype::NOTIFICATION);
    check(n >= 0);
    let n = n as u64;

    // Two signals before anyone looks are one word carrying both, and a poll takes all of it.
    check(notification_signal(n, 0b01) == 0);
    check(notification_signal(n, 0b10) == 0);
    check(notification_poll(n) == 0b11);
    check(notification_poll(n) == 0);
    // A signal of no bits changes nothing.
    check(notification_signal(n, 0) == 0);
    check(notification_poll(n) == 0);
    // A wait with a word pending returns it without blocking.
    check(notification_signal(n, 0b100) == 0);
    check(notification_wait(n) == 0b100);

    // Bind to ourselves, once. A second bind of the same pair is refused: a notification binds one
    // thread and a thread binds one notification.
    check(notification_bind(n, MYSELF) == 0);
    check(notification_bind(n, MYSELF) == abi::Error::NotPermitted as i64);
    // The wrong kind of capability in the thread slot is `WrongObject`, not a silent success.
    check(notification_bind(n, ENDPOINT) == abi::Error::WrongObject as i64);

    // A signal counted while we are not receiving ends our next receive at once.
    check(notification_signal(n, 0b1000) == 0);
    check(recv_bound(ENDPOINT) == Received::Notification(0b1000));

    // Hand the kernel test a signal-only view of the notification, then block receiving twice.
    check(send_cap(REPORT, n, abi::rights::WRITE, 0) == 0);
    for _ in 0..2 {
        match recv_bound(ENDPOINT) {
            Received::Notification(word) => {
                send(REPORT, abi::notification::BOUND, word, 1);
            }
            Received::Message(w0, w1, _) => {
                send(REPORT, w0, w1, 0);
            }
        }
    }
    exit()
}

/// The only way this program can say "no": a trap, which the kernel treats as a fault.
fn check(ok: bool) {
    if !ok {
        user_mode_runtime::trap()
    }
}

user_mode_runtime::panic_handler!();
