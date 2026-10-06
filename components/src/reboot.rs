//! **`reboot`**: restart the machine (milestone 805 (`reboot` at the prompt), DECISIONS §251
//! (restarting the machine is a kernel object the progenitor hands out)).
//!
//! The whole program is: say what the filesystem flush answered, then invoke the reboot object.
//! On success the second step never returns, and the next thing on the console is the firmware.
//!
//! # The capability table
//!
//! | slot | what | why it is that and not wider |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the flush report goes |
//! | 8 | the diagnostics sink, `WRITE` | where a refusal goes, so `>` cannot swallow it |
//! | 13 | the reboot object, `WRITE` | the right to invoke it, and no `GRANT` to hand it on |
//!
//! And one word that is data, not authority: what `filesystem_protocol::fs::SYNC` answered when the
//! progenitor flushed before starting this program, in the third start register
//! ([`grant_plan::REBOOT_SYNC_REGISTER`]).
//!
//! # Why the progenitor flushes, not this program
//!
//! §251's clause 3 says the flush comes before the reset, and the kernel does none of it. A program
//! that sent `SYNC` itself would need a filesystem capability carrying `dir::WRITE`, which is the
//! right to open and truncate files by name, all to make one request that touches no file. The
//! progenitor already holds the file service and already flushes it after an install, so it flushes
//! once more, at this program's spawn, and hands the answer over. Nothing here can reach a file.
//!
//! # Why the report is sent twice
//!
//! The report goes to the shell, which forwards it to the terminal, and the reset that follows
//! stops everything. A `SEND` on a rendezvous returns when the receiver has taken the message, not
//! when it has printed it, so the end-of-stream after the report is the barrier: the shell takes it
//! only once it has finished with the report. Without that the report raced the reset and lost.
//!
//! # BUGS
//!
//! - **A firmware that accepts the reset and hangs cannot be told from a slow one**, from inside
//!   the machine. radon did exactly that on 2026-09-04, and milestone 592 (radon's cold reboot dies
//!   in OpenSBI's PMIC write) holds the fix, which has not run on the board. On radon today this
//!   program prints its report and the board stops.
//! - **A refusal does not exit non-zero**, because no program in this system reports an exit
//!   status (`crates/swish`'s `Status` says so). The refusal is a sentence on the second stream.
//! - **The flush is not fenced against background jobs.** Writes a concurrently running job makes
//!   between the progenitor's `SYNC` and the reset are not covered. The shell runs `reboot` in the
//!   foreground, so this is a job somebody started earlier and left writing.
//! - **Devices are not quiesced.** A DMA transfer in flight is cut off; after `SYNC` the block
//!   servers are idle and nothing else writes to persistent storage.
//!
//! Name: provisional (milestone 805). A verb, which design/naming.md passes only as a term of art.

#![no_std]
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, is_granted, send};

const REPORT: u64 = 0;
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;
const REBOOT_SLOT: u64 = grant_plan::REBOOT_SLOT;

/// `EOPNOTSUPP`: the device offers no flush, which `fs::SYNC` passes through on purpose.
const EOPNOTSUPP: i64 = 95;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, a2: u64) -> ! {
    // `grant_plan::REBOOT_SYNC_REGISTER` is 2: the progenitor's `SYNC` answer, as a signed word.
    let synced = a2 as i64;
    let has_diag = is_granted(DIAG_SLOT);
    let complain = |text: &[u8]| write_on(if has_diag { DIAG_SLOT } else { REPORT }, text);

    if !is_granted(REBOOT_SLOT) {
        complain(b"reboot: no reboot capability was granted; the machine keeps running\n");
        finish(has_diag);
    }

    let mut line = Line::new();
    if synced > 0 {
        line.push(b"reboot: filesystem flushed (device flushes completed since boot: ");
        line.number(synced as u64);
        line.push(b"); restarting\n");
    } else if synced == 0 {
        line.push(b"reboot: no writable filesystem is attached, so nothing to flush; restarting\n");
    } else if synced == -EOPNOTSUPP {
        line.push(b"reboot: this device cannot flush on demand (EOPNOTSUPP), so every write it acknowledged is as durable as it makes it; restarting\n");
    } else {
        complain(b"reboot: the filesystem flush failed, so the machine was not restarted (errno ");
        let mut n = Line::new();
        n.number(synced.unsigned_abs());
        n.push(b")\n");
        complain(n.bytes());
        finish(has_diag);
    }
    write_on(REPORT, line.bytes());
    // The barrier: the shell takes end-of-stream only after it has passed the line on.
    send(REPORT, byte_sink_protocol::eof(), 0, 0);

    let answer = user_mode_runtime::reboot(REBOOT_SLOT);
    let mut n = Line::new();
    n.push(b"reboot: the firmware refused every reset route (abi error ");
    n.number(answer.unsigned_abs());
    n.push(b"); the kernel console above says which; the machine keeps running\n");
    write_on(if has_diag { DIAG_SLOT } else { REPORT }, n.bytes());
    if has_diag {
        send(DIAG_SLOT, byte_sink_protocol::eof(), 0, 0);
    }
    exit();
}

/// End both streams and exit, for the two paths that never reached the reset.
fn finish(has_diag: bool) -> ! {
    if has_diag {
        send(DIAG_SLOT, byte_sink_protocol::eof(), 0, 0);
    }
    send(REPORT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

/// A line built without a heap: the program has none.
struct Line {
    buf: [u8; 192],
    len: usize,
}

impl Line {
    fn new() -> Self {
        Line {
            buf: [0; 192],
            len: 0,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        let n = bytes.len().min(self.buf.len() - self.len);
        self.buf[self.len..self.len + n].copy_from_slice(&bytes[..n]);
        self.len += n;
    }

    fn number(&mut self, mut v: u64) {
        let mut digits = [0u8; 20];
        let mut i = digits.len();
        loop {
            i -= 1;
            digits[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        self.push(&digits[i..]);
    }

    fn bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
