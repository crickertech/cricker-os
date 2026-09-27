//! **`printenv`**: print the inert-configuration page (milestone 47's environment-variable fork,
//! DECISIONS §111, notes/env-config.md).
//!
//! The whole program is: read a page, print what it carries. It holds two capabilities and
//! neither of them can change anything, exactly `date`'s shape one manifest field over
//! ([`grant_plan::Manifest::config`] rather than [`grant_plan::Manifest::clock`]).
//!
//! # It reads. There is no way to set anything from here.
//!
//! The page this program maps is `READ`-only, and that rung is the whole of why there is no
//! `printenv -s TZ=...` here: the authority this process lacks is a page permission rather than a
//! check somewhere in this file. Nothing about what a shell hands its own children can be changed
//! by a program spawned with a config grant, which is the same shape `date` already demonstrates
//! for the clock.
//!
//! # Three keys, three closed domains, and a state the type carries honestly
//!
//! `environment_protocol::ConfigPage` answers `Option<&str>` per key: `Some(value)` when the page is
//! valid and carries that key, `None` when the page is valid but that key was never set, and
//! (indistinguishably from the outside, by design) `None` when nobody assembled a page into this
//! frame at all. This program tells the second case apart from the first two the same way `date`
//! tells "no clock service" from "no clock capability": by checking whether it holds the
//! capability at all before it prints anything about what the page says.
//!
//! # Arguments: none
//!
//! There is no argv on this ABI (notes/abi.md), and this program needs none: `printenv` with no
//! arguments is the only spelling, matching Unix's own `printenv(1)`/`env(1)` with nothing to
//! filter by.
//!
//! # EXAMPLES
//!
//! ```text
//! printenv                  TZ=UTC
//!                            LANG=C
//!                            TERM=dumb
//! printenv  (a key unset)   TZ=UTC
//!                            LANG (unset)
//!                            TERM=dumb
//! printenv  (no grant)      printenv: no configuration was granted
//! ```
//!
//! # BUGS
//!
//! - **No filtering.** Unix's `printenv NAME` prints one variable; this always prints all three,
//!   because `ArgSpec` has no position yet (the same gap `date`'s module docs name for a format
//!   selector) and three lines cost nothing to read in full.
//! - **The three keys are hardcoded.** Nothing here enumerates the page; a fourth validated domain
//!   would need a fourth line written by hand. `environment_protocol::PAGE_BYTES` is small and fixed,
//!   so this is a property of the contract rather than an oversight, but a reader expecting a
//!   generic env dump should know the shape is closed.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist). Introduced 2026-08-26
//! alongside `grant_plan::Manifest::config`. Unix's own name for exactly this (`printenv(1)`), a
//! term of art already right per this tree's naming convention for standard terms.
//!
//! **It passes the test `watch` failed the same day**, which is the reason to keep a borrowed word
//! rather than merely the reason to like it. A borrowed name is the best available when it is true
//! and the worst when it is not: upstream `watch` re-runs an arbitrary command and this tree's
//! `watch` never could, so the recognition was false. Upstream `printenv` with no arguments prints
//! the whole environment, which is exactly this program, and the section above records that
//! checked rather than assumed.
//!
//! Refused `env`, which names `env(1)`, a program that also *sets* variables and *runs* a command;
//! neither is possible here and the name would promise both. Refused `config` and `environment` as
//! generic words that could name almost anything in an operating system, and `environment_protocol`
//! already spends the stem on the wire format.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and documenting an
// OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use environment_protocol::ConfigPage;
use user_mode_runtime::{exit, invoke, send};

/// Slot 0: where the output goes. An endpoint with `WRITE`, and the same 16-bytes-per-message
/// framing the std PAL's stdout uses (`w0` = the byte count, `w1`|`w2` = the bytes, little-endian).
const REPORT: u64 = 0;

/// Slot 1: the config page's `PageFrame` capability, with `READ` and nothing else. Its *presence*
/// is what [`config_page`] probes for; its rights are what stop this program setting anything.
const CONFIG_SLOT: u64 = 1;

/// Where the wiring maps the config page, read-only. Must match
/// `crates/system_initializer`'s `CHILD_CONFIG_VA`.
const CONFIG_VA: u64 = address_space_map::pair_page(0x00e0_0000);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let Some(page) = config_page() else {
        line(b"printenv: no configuration was granted");
        send(REPORT, byte_sink_protocol::eof(), 0, 0);
        exit();
    };

    key_line(&page, b"TZ", page.tz());
    key_line(&page, b"LANG", page.lang());
    key_line(&page, b"TERM", page.term());

    send(REPORT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

/// One `KEY=value` line when the domain holds a value, `KEY (unset)` when the page is valid and
/// simply does not carry it. `_page` is unused directly but keeps this a method-shaped call at
/// every site, so a reader sees the three lines are the same operation three times rather than
/// three hand-written formats that happen to agree today.
fn key_line(_page: &ConfigPage, key: &[u8], value: Option<&str>) {
    let mut buf = [0u8; 96];
    let mut n = 0;
    push(&mut buf, &mut n, key);
    match value {
        Some(v) => {
            push(&mut buf, &mut n, b"=");
            push(&mut buf, &mut n, v.as_bytes());
        }
        None => push(&mut buf, &mut n, b" (unset)"),
    }
    line(&buf[..n]);
}

/// **Append what fits and silently drop the rest**, advancing `*n` by what was taken.
///
/// The truncation is deliberate and is this program's whole answer to a value it did not write:
/// `TZ`, `LANG` and `TERM` come out of a page `system_initializer` filled, and a line that will not
/// fit is a short line rather than a fault. What must never happen is the other thing, which is
/// this writing past the array; the proof at the bottom of this file is that claim, stated for
/// every starting offset including ones no caller here produces.
///
/// The guard is per byte rather than per call on purpose: a length check up front would have to
/// know `bytes.len()` and `*n` are both trustworthy, and one of them comes from the page.
fn push(buf: &mut [u8; 96], n: &mut usize, bytes: &[u8]) {
    for &b in bytes {
        if *n < buf.len() {
            buf[*n] = b;
            *n += 1;
        }
    }
}

/// Whether a capability is in `slot`, without touching whatever it names.
///
/// [`config_page`]'s probe, `date`'s `is_granted()` lifted verbatim: invoke a method number no
/// object type defines, so the call can only be refused, and read *which* refusal came back. An
/// empty slot answers `NoSuchSlot`; a real object answers `BadMethod`, which is a refusal from
/// something that exists.
fn is_granted(slot: u64) -> bool {
    /// A method number no object type defines, so the invocation can only ever be refused.
    const NO_SUCH_METHOD: u64 = 0xffff;
    // SAFETY: a syscall that cannot succeed; the kernel validates the slot before the method.
    let r = unsafe { invoke(slot, NO_SUCH_METHOD, 0, 0, 0) };
    r != abi::Error::NoSuchSlot as i64
}

/// The config page, or `None` when this process was granted no configuration capability at all.
///
/// The probe has to answer **without touching the page**, for `date`'s reason (a different binary,
/// the same shape): a process granted nothing has nothing mapped at [`CONFIG_VA`], and a read
/// there would fault instead of answering.
fn config_page() -> Option<ConfigPage> {
    if !is_granted(CONFIG_SLOT) {
        return None;
    }
    // SAFETY: the wiring maps the config page read-only at CONFIG_VA alongside the capability the
    // probe just found, and nothing unmaps it. Without the capability we never build the pointer.
    Some(unsafe { ConfigPage::new(CONFIG_VA) })
}

/// Send `bytes` and a newline to the output endpoint, 16 bytes per message. `date`'s `line`/
/// `line_on` collapsed into one, because this program has only the one destination.
fn line(bytes: &[u8]) {
    let mut out = [0u8; 96];
    let n = bytes.len().min(out.len() - 1);
    out[..n].copy_from_slice(&bytes[..n]);
    out[n] = b'\n';
    for chunk in out[..n + 1].chunks(16) {
        let mut w1 = 0u64;
        let mut w2 = 0u64;
        for (i, &b) in chunk.iter().enumerate() {
            if i < 8 {
                w1 |= (b as u64) << (8 * i);
            } else {
                w2 |= (b as u64) << (8 * (i - 8));
            }
        }
        send(REPORT, chunk.len() as u64, w1, w2);
    }
}

// Kani links `std`, which already defines `panic_impl`, and a second definition is a duplicate lang
// item that does not compile. Milestone 193 took exactly this `cfg` on `kernel/src/panic.rs` for
// exactly this reason. What it costs is item 3 of notes/user-proofs.md's stub list: nothing proved
// below says anything about what this program does after a panic.
#[cfg(not(kani))]
user_mode_runtime::panic_handler!();

/// **What the prover can see of `printenv`, and what it cannot** (milestone 197).
///
/// Read notes/user-proofs.md before adding a harness here; it enumerates the stub boundary, and the
/// hazard of proving a program like this one is that a stub reads as coverage. The short version:
/// the boundary is **hard rather than soft**, because every capability this program holds is
/// reached through `user_mode_runtime`, whose calls are `asm!`, and Kani refuses an unsupported construct
/// instead of proving past it. A harness that wandered into [`line`], [`is_granted`] or
/// [`config_page`] would fail loudly rather than report a proof about a fiction. What is left is
/// [`push`], which is the only thing here that decides anything.
#[cfg(kani)]
mod proofs {
    /// The buffer every caller of [`super::push`] gives it.
    const CAP: usize = 96;
    /// A byte [`super::push`] never writes on its own, so an untouched cell is distinguishable
    /// from a written one without keeping a second copy of the buffer to compare against.
    const UNTOUCHED: u8 = 0xAA;

    /// **`push` writes only inside its buffer, appends only at the offset it was given, and never
    /// leaves that offset past the end.**
    ///
    /// The one thing standing between a value this program did not write and a byte past the end of
    /// a fixed array is the `*n < buf.len()` inside [`super::push`]. `key_line` calls it up to
    /// three times against one 96-byte buffer, with two of the three arguments coming out of the
    /// configuration page (`ConfigPage::tz`, `lang`, `term`), so the running offset is a function
    /// of somebody else's bytes. This says the guard holds for **every** offset it could be handed
    /// and every content, rather than for the three the program happens to produce.
    ///
    /// Four claims, none implied by the others:
    ///
    /// - Nothing outside `buf` is written. Kani checks that for free on any run, which is why the
    ///   starting offset below is left **unconstrained**: `push` must be memory-safe even when
    ///   handed an offset already past the end, and it is, because the guard is a comparison rather
    ///   than a subtraction.
    /// - The offset never *becomes* out of range. Conditional on starting in range, which is the
    ///   precondition every call site here satisfies by starting at zero.
    /// - It appends: nothing below the starting offset moves.
    /// - It reports what it wrote: nothing at or above the final offset moves either, so a caller
    ///   that keeps pushing lands on ground nobody has touched.
    ///
    /// **The unwind bound is `bytes.len() == 4`, and here is what it does and does not cost.** The
    /// loop body is one comparison and one store per byte, and the starting offset is symbolic over
    /// the whole of `usize`, so four bytes already exercise every way a push can meet the
    /// boundary: entirely below it, crossing it, sitting exactly on it, and entirely past it. What
    /// four bytes cannot see is a defect that only appears at some larger length, and no such
    /// defect is expressible in a loop whose state is a single `usize` that only ever increments.
    /// Stating the bound rather than raising it is the honest trade; raising it costs solver time
    /// and buys nothing this argument does not already give.
    ///
    /// Falsification: replayable `components/falsifications/proofs.push_never_writes_past_the_buffer_it_was_given.patch`
    #[kani::proof]
    fn push_never_writes_past_the_buffer_it_was_given() {
        let mut buf = [UNTOUCHED; CAP];
        let start: usize = kani::any();
        let mut n = start;
        let bytes: [u8; 4] = kani::any();

        super::push(&mut buf, &mut n, &bytes);

        if start <= CAP {
            assert!(n <= CAP);
            assert!(n >= start);
            // Both walks are written over the whole buffer with the interesting range selected
            // inside, rather than as `0..start` and `n..CAP`. A range whose endpoint is symbolic
            // has no unwinding bound CBMC can find, and the harness simply never terminates; this
            // shape costs `CAP` iterations of a byte comparison and finishes in a tenth of a
            // second. Found the slow way, and recorded in notes/user-proofs.md.
            for k in 0..CAP {
                if k < start || k >= n {
                    assert!(buf[k] == UNTOUCHED);
                }
            }
        } else {
            // Handed a nonsense offset it must still not write anything, and must not move it.
            assert!(n == start);
            for k in 0..CAP {
                assert!(buf[k] == UNTOUCHED);
            }
        }
    }

    /// **The buffer can actually be filled**, so the assertion above is not proved by an assumption
    /// set that never reaches the boundary.
    ///
    /// DECISIONS §134's note is that this tree carried 23 `cover` sites against 141 harnesses, and
    /// a bound nothing approaches is how a proof becomes decoration. This says an offset and four
    /// bytes exist that leave the buffer exactly full with bytes still to place, which is the
    /// truncation [`super::push`]'s own doc describes.
    ///
    /// Falsification: unfalsified. A `cover` states reachability, so the defect it catches is an
    /// assumption set that quietly stops admitting the full-buffer case; nothing has proposed one,
    /// and inventing a patch to fill this row is what §134's three states exist to refuse.
    #[kani::proof]
    fn the_buffer_can_be_filled_exactly() {
        let mut buf = [UNTOUCHED; CAP];
        let start: usize = kani::any();
        kani::assume(start <= CAP);
        let mut n = start;
        let bytes: [u8; 4] = kani::any();

        super::push(&mut buf, &mut n, &bytes);

        kani::cover!(n == CAP && start < CAP);
    }
}
