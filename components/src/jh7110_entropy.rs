//! **The JH7110 TRNG driver** (milestone 159; roadmap `design/roadmap/159-jh7110-trng-driver.md`,
//! notes/entropy.md), an alternate backend for the entropy contract (`entropy_protocol`,
//! DECISIONS §44), alongside `entropy.rs`'s virtio-rng one. Same contract, same shape ("a driver,
//! not a new protocol"), different device: no virtqueue, no DMA page, just a register window.
//!
//! # It is wired now, and it still has never run
//!
//! **Something spawns this program as of 2026-09-01**: `kernel/src/user/entropy_service.rs`'s
//! `Bus` enum grew a `Jh7110` variant, and the riscv64 boot tour asks for it. The decision the
//! previous draft of this paragraph was holding open (how a spawner locates this binary in an
//! initrd) turned out to need no new answer: `entropy_service::ensure` already takes the program's
//! bytes from its caller, exactly as it does for `entropy`, so the tour reads
//! `user::program("jh7110_entropy")` and hands them over. Nothing about the interactive boot changed;
//! DECISIONS §120's stopgap question is still open and still untouched by this.
//!
//! **Nothing below has run against a real TRNG**, and wiring it did not change that: QEMU has no
//! JH7110, so on every machine this repository's CI boots the wiring resolves to a skip. The
//! register sequence follows
//! `jh7110_entropy`'s citations (the Linux `jh7110-trng.c` driver); the polling bounds
//! ([`POLL_TRIES`], [`LOCKUP_RETRIES`]) are placeholders with no board measurement behind them, the
//! same honest gap `entropy.rs`'s `WAIT_WAKEUPS` once was before QEMU could prove it. See the
//! roadmap doc for what remains before this can be trusted.
//!
//! # What is proven, and where
//!
//! The register decode ([`jh7110_entropy::interpret`]), the device-tree query
//! ([`jh7110_entropy::discover`]) and the byte buffer ([`jh7110_entropy::Pool`]) are all in the crate,
//! host-tested and Kani-reachable, because none of them needs a device to be wrong in an
//! interesting way. What is left in this file is exactly the part that cannot be tested without
//! silicon: the volatile reads and writes, and the two polling bounds below.
//!
//! # Its authority, once something grants it
//!
//! - slot 0, the **request** endpoint (RECV): clients `CALL` here, `entropy_protocol`'s wire format,
//!   unchanged from `entropy.rs`'s;
//! - slot 1, a **readiness** endpoint (WRITE): exactly one message, either
//!   [`proto::READY`](entropy_protocol::READY) once the device has answered a first generation with
//!   bytes that are not all zero, or a
//!   [`bringup_failure`](entropy_protocol::bringup_failure) word naming which step failed;
//! - mapped: **one page**, the TRNG's register block, device-typed at [`TRNG_VA`], placed there by
//!   whoever spawns this (rule 2: a base address, passed in, nothing this driver looks up). The
//!   binding's `reg` window is `0x4000` and the spawner maps `0x1000` of it, because
//!   `jh7110_entropy::regs` reaches only `0x68`;
//! - and nothing else. **No third slot, no DMA page, no IRQ**, unlike `entropy.rs`'s virtio-rng
//!   backend: this device has no virtqueue and no shared buffer to negotiate, only the eight
//!   `RAND` registers, so its authority is smaller by construction, not by omission. An earlier
//!   draft named a slot 2 for a `DeviceFrame` capability; the spawner that actually exists grants
//!   the page as a `Mapping` rather than as a capability the driver holds, so the slot was
//!   describing a thing nobody hands over.
//!
//! # BUGS
//!
//! **This driver still programs no clock and deasserts no reset, and something else now does.**
//! The binding names two clock inputs (`hclk`, `ahb`) and one reset line, and Linux's
//! `jh7110-trng.c` takes all three through the clock and reset frameworks before it touches a
//! register. This driver takes none of them: it is handed a mapped register window and assumes the
//! block behind it is powered, which is rule 2 working as intended rather than an omission. The
//! prediction this entry used to carry came true on 2026-09-04 (all-zero registers, a failed first
//! refill, an all-zero bring-up diagnostic), and **milestone 220 answered it**: the kernel now
//! ungates the STG CRG's `SEC_HCLK` and `SEC_MISCAHB_CLK` and releases the block's reset before
//! this program is spawned, and the second boot that day read live registers. What remains here is
//! the dependency itself: nothing in this program checks that it happened, and a boot where it did
//! not still presents as zeros.
//!
//! **A device that answers with zeros is condemned for the whole boot, with no way back.** An
//! all-zero first generation is treated as a bring-up failure
//! ([`proto::STEP_FIRST_ALL_ZERO`](entropy_protocol::STEP_FIRST_ALL_ZERO)) and this driver then
//! answers every request `NO_ENTROPY` until it is restarted, even if the block is powered on
//! underneath it a moment later. That is deliberate (a device that answers wrongly is worse than
//! one that does not answer) and it is also the case a real clock driver would want to retry;
//! milestone 220 is where that becomes worth building, and `entropy_protocol`'s own `BUGS` carries the
//! probability this refusal is wrong on a working device.
//!
//! **The polling bounds below are guesses.** [`POLL_TRIES`] and [`LOCKUP_RETRIES`] have no board
//! measurement behind them. A bound that is too small reports `NO_ENTROPY` on a working device;
//! too large, and a dead device stalls the boot tour for as long as the loop takes. Nothing has
//! measured which side of that this lands on.
//!
//! **The 256-bit mode selection has never been read back.** [`init`] writes `MODE.R256` because
//! the width a JH7110's TRNG resets to is a build-time parameter of the silicon rather than
//! something the documentation settles, and it reports the `STAT` it read afterwards so a bench
//! session can check `STAT.R256`. Nobody has. If that bit reads clear on radon, this driver is
//! assembling 32 bytes out of a 16-byte answer and the upper four `RAND` words are not device
//! output; the count served would have to drop to 16. **This is the one open correctness question
//! about the bytes themselves**, and one line of a bench transcript closes it.
//!
//! **The polling loops are unbounded in time, not just in count.** [`POLL_TRIES`] counts
//! iterations of a loop with no delay in it, so what it actually bounds depends on how fast this
//! core runs and what the compiler did to the loop. It is a guard against hanging forever, not a
//! timeout anyone can state in microseconds.
//!
//! # Why polling, not the completion interrupt
//!
//! [binding] names `interrupts = <30>`, and the Linux driver is interrupt-driven. This program
//! polls `ISTAT` instead, for one reason worth stating rather than defaulting to: routing PLIC
//! line 30 correctly (which context, which priority) is one more fact this project cannot check
//! without the board, and a wrong interrupt binding fails silently as a hang, while a wrong poll
//! bound fails loudly as `NO_ENTROPY`. This is a reversible choice (AGENTS.md's "recommend on
//! reversible forks"): switching to the interrupt once the board confirms the binding is a
//! smaller change than getting IRQ routing wrong on the first hardware boot ever attempted.
//!
//! [binding]: https://github.com/torvalds/linux/blob/master/Documentation/devicetree/bindings/rng/starfive%2Cjh7110-trng.yaml
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist), replacing the provisional
//! `jh7110_entropy_source`, which this program carried from 2026-08-24 without a ruling. It moves
//! with the crate it is the volatile shell over (`jh7110_entropy`), which is the deliberate
//! crate-and-program pair AGENTS.md describes: the crate is the logic, host-tested and reachable
//! by the prover, and the program keeps the IO. Splitting the two names would hide that
//! relationship, and that is what disqualified `jh7110_entropy_driver` for this half too, even
//! though it reads well beside `block_driver` and its three siblings in this directory. The chip
//! qualifier, the measurement that retired the `_source` suffix, and the rest of the refusals are
//! the crate's own argument and are not repeated here; see its header.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use abi::rendezvous;
use entropy_protocol as proto;
use jh7110_entropy::{
    CTRL_EXEC_RANDRESEED, CTRL_GENE_RANDNUM, ISTAT_ALL, ISTAT_RAND_RDY, ISTAT_SEED_DONE, MODE_R256,
    Outcome, Pool, interpret,
};
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
use user_mode_runtime::{recv_cap, reply, send};

register_structs! {
    /// The JH7110 TRNG's register block, migrated onto `tock_registers` (milestone 139 round 5):
    /// offsets transcribed from `jh7110_entropy::regs` (itself transcribed from the Linux driver, see
    /// `crates/jh7110_entropy`'s module doc), now checked at compile time instead of asserted by a
    /// hand-written comment. Unlike the NS16550 (`kernel/src/drivers/ns16550.rs`'s own module
    /// doc), this device's layout has no runtime-variable stride or width: [binding] gives one
    /// `reg` window (`reg = <0x1600C000 0x4000>`) with no `reg-shift`/`reg-io-width` knob, so there
    /// is nothing here `register_structs!`'s compile-time-fixed layout cannot express. Only the
    /// registers this driver touches are named (`CTRL`, `STAT`, `ISTAT`, `RAND0..RAND7`); `MODE`,
    /// `SMODE`, `IE`, `AUTO_RQSTS` and `AUTO_AGE` are reserved padding here, the same "not
    /// otherwise used" status `jh7110_entropy::regs`'s own doc gives several of them.
    ///
    /// [binding]: https://github.com/torvalds/linux/blob/master/Documentation/devicetree/bindings/rng/starfive%2Cjh7110-trng.yaml
    #[allow(non_snake_case)]
    RegisterBlock {
        (0x00 => CTRL: WriteOnly<u32>),
        // `STAT` is read on **every** poll now, not only for the bring-up diagnostic: it carries
        // `SEEDED`, which is what says a latched `RAND_RDY` is an answer rather than a leftover.
        // See `jh7110_entropy::Outcome::Unseeded`.
        (0x04 => STAT: ReadOnly<u32>),
        (0x08 => MODE: ReadWrite<u32>),
        (0x0c => _reserved_smode),
        (0x10 => IE: ReadWrite<u32>),
        // **Write-1-to-clear**, per the TRM's own register map (see `jh7110_entropy::regs::ISTAT`).
        // It was `ReadOnly` here while how to acknowledge a bit was unknown, and that is exactly
        // the bug: `RAND_RDY` latches, so a driver that never writes this register sees every
        // generation after the first complete instantly.
        (0x14 => ISTAT: ReadWrite<u32>),
        (0x18 => _reserved_pad),
        (0x20 => RAND0: ReadOnly<u32>),
        (0x24 => RAND1: ReadOnly<u32>),
        (0x28 => RAND2: ReadOnly<u32>),
        (0x2c => RAND3: ReadOnly<u32>),
        (0x30 => RAND4: ReadOnly<u32>),
        (0x34 => RAND5: ReadOnly<u32>),
        (0x38 => RAND6: ReadOnly<u32>),
        (0x3c => RAND7: ReadOnly<u32>),
        (0x40 => _reserved_seed),
        (0x60 => AUTO_RQSTS: ReadWrite<u32>),
        (0x64 => AUTO_AGE: ReadWrite<u32>),
        (0x68 => @END),
    }
}

/// Capability slots, by convention with whatever kernel-side wiring eventually spawns this (see
/// the module doc: none does yet).
const REQ: u64 = 0;
const READY: u64 = 1;

/// Where the spawner maps the TRNG's register page (a device-typed mapping, the same mechanism
/// `console.rs`'s `UART_VA` uses). **Must match `kernel/src/user/entropy_service.rs`'s `TRNG_VA`.**
/// Provisional and distinct from the `0x0090_0000` DMA convention `entropy.rs` and
/// `keyboard_driver.rs` share, so the two backends cannot collide if a wiring ever needs both
/// mapped in different processes at once.
const TRNG_VA: u64 = address_space_map::pair_page(0x0000_0000_0094_0000);

/// How many times to poll one generation attempt before giving up on it. **Not a measured bound**:
/// see the module doc. Large enough that a real device answering in microseconds never hits it in
/// practice, small enough that a device that will never answer does not hang the driver.
const POLL_TRIES: usize = 100_000;

/// How many lockup-and-reseed cycles to absorb before telling a caller the device would not
/// answer. Bounded for the reason `entropy.rs`'s `REFILL_TRIES` is: a device that faults once is
/// worth retrying, one that never stops faulting is a fact the caller needs, not something to
/// spin on forever.
const LOCKUP_RETRIES: usize = 4;

fn regs() -> &'static RegisterBlock {
    // SAFETY: TRNG_VA is our device mapping of the JH7110 TRNG's register window, handed to us at
    // spawn (rule 2: this driver is told the address, never told to look), for the whole lifetime
    // of this process. This is the same invariant the hand-written r32/w32 calls used to assert by
    // comment; register_structs! now checks every offset above at compile time instead.
    unsafe { &*(TRNG_VA as *const RegisterBlock) }
}

fn rand_words() -> [u32; 8] {
    let r = regs();
    [
        r.RAND0.get(),
        r.RAND1.get(),
        r.RAND2.get(),
        r.RAND3.get(),
        r.RAND4.get(),
        r.RAND5.get(),
        r.RAND6.get(),
        r.RAND7.get(),
    ]
}

/// **Acknowledge `ISTAT` bits.** The register is write-1-to-clear (`jh7110_entropy::regs::ISTAT`),
/// so writing a mask back clears exactly the bits in it and leaves the rest standing.
fn ack(bits: u32) {
    regs().ISTAT.set(bits);
}

/// **Put the block in a known state before asking it for anything** (milestone 159), the sequence
/// all three drivers `jh7110_entropy`'s module doc cites agree on. Each step is there because a
/// reset value nobody documented is not something to rely on:
///
/// 1. `AUTO_AGE` and `AUTO_RQSTS` to zero, which is how the TRM says the two reseed-reminder
///    alarms are disabled. Left alone, a nonzero counter raises `AGE_ALARM` or `RQST_ALARM` in a
///    register this driver polls for other reasons.
/// 2. Clear every `ISTAT` bit, because this block was running under U-Boot before nife started
///    and a latched `RAND_RDY` from then would make the first poll below return instantly.
/// 3. `MODE.R256`, so a generation answers with all eight `RAND` words. **Without it the device
///    may be in 128-bit mode**, where only `RAND0..RAND3` carry the answer and this driver's
///    32-byte assembly is half real bytes and half whatever the upper words hold. The width after
///    reset is a build-time parameter of the silicon, so it has to be set rather than assumed.
/// 4. `IE` left at zero, deliberately: see the module doc's "Why polling".
///
/// Returns what `STAT` read back afterwards, which is the bench diagnostic: `STAT.R256` says
/// whether step 3 took, and `STAT.SEEDED` whether the reseed the caller runs next is still needed.
fn init() -> u32 {
    let r = regs();
    r.AUTO_AGE.set(0);
    r.AUTO_RQSTS.set(0);
    ack(ISTAT_ALL);
    r.MODE.set(r.MODE.get() | MODE_R256);
    r.IE.set(0);
    r.STAT.get()
}

/// Force a reseed and wait (bounded) for `ISTAT.SEED_DONE`, acknowledging it. Called once at
/// bring-up, mirroring the init sequence in `jh7110_entropy`'s module doc, and again whenever
/// [`generate`] sees [`jh7110_entropy::Outcome::Lockup`] or [`jh7110_entropy::Outcome::Unseeded`].
/// `false` on a bound-out: the caller decides what that means.
///
/// **The acknowledgement is the part that was missing.** `SEED_DONE` is latched, so an unacked one
/// makes every later reseed appear to complete immediately.
fn reseed_and_wait() -> bool {
    ack(ISTAT_SEED_DONE);
    regs().CTRL.set(CTRL_EXEC_RANDRESEED);
    for _ in 0..POLL_TRIES {
        if regs().ISTAT.get() & ISTAT_SEED_DONE != 0 {
            ack(ISTAT_SEED_DONE);
            return true;
        }
    }
    false
}

/// Ask the device for 32 fresh bytes, retrying a hardware-reported lockup or an unseeded core by
/// reseeding, bounded by [`LOCKUP_RETRIES`]. `None` if the device never produced an answer inside
/// the bound: this driver's whole failure mode, reported to callers as [`proto::NO_ENTROPY`]
/// rather than a hang.
///
/// **`RAND_RDY` is acknowledged on both sides of the command**, and that is the fix for the defect
/// this milestone found. Before the write, because a bit standing from the previous generation
/// would otherwise satisfy the very first poll and hand back the *previous* answer. After the
/// words are read, so the next call starts from a clean register. A driver that never wrote
/// `ISTAT` served its first generation correctly and then answered every later request instantly
/// with whatever the register file happened to hold, which is indistinguishable from working right
/// up until two draws come back identical.
fn generate() -> Option<[u8; 32]> {
    for _ in 0..=LOCKUP_RETRIES {
        ack(ISTAT_RAND_RDY);
        regs().CTRL.set(CTRL_GENE_RANDNUM);
        for _ in 0..POLL_TRIES {
            match interpret(regs().STAT.get(), regs().ISTAT.get(), rand_words()) {
                Outcome::Ready(bytes) => {
                    ack(ISTAT_RAND_RDY);
                    return Some(bytes);
                }
                // Both of these mean "the core is not in a state to answer, seed it and ask
                // again", and they are bounded together because a device that keeps returning to
                // either one is a device to tell the caller about rather than to spin on.
                Outcome::Lockup | Outcome::Unseeded => {
                    reseed_and_wait();
                    break; // out of the poll loop; the outer loop's next iteration retries GENE_RANDNUM
                }
                Outcome::NotReady => continue,
            }
        }
    }
    None
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    // Put the block in a known state before asking it for anything, then seed it. `init` returns
    // what `STAT` read back, which is the one snapshot taken before any generation has happened
    // and so the only one that can say whether `MODE.R256` took.
    let stat_after_init = init();
    reseed_and_wait();

    // Draw the first 32 bytes before reporting anything, the same discipline `entropy.rs` uses:
    // "the service is up" should mean "a client that asks will be answered", not just that the
    // handshake completed. **The bytes decide the word**, which is the fix for the defect radon
    // found on 2026-09-04: this line used to send `proto::READY` unconditionally and put the
    // truth in a second word nothing was obliged to read, so a device whose clock was gated
    // reported itself ready holding zeros.
    let first = generate();
    let report = proto::readiness(first.into_iter().flatten());
    let mut pool = Pool::new();
    if let Some(bytes) = first {
        pool.refill(&mut || Some(bytes));
    }
    // **Word 2 is always the same fact now**, and that change is this milestone's other finding.
    // It used to be the byte count on success and a `(STAT << 32) | ISTAT` snapshot on failure,
    // which meant the number a bench session read first meant two different things depending on a
    // word printed beside it. On 2026-09-04 radon printed `0x20` there and it was read as
    // `ISTAT` bit 5, an undocumented status bit, when it was in fact `pool.remaining() == 32`:
    // the success path's byte count, on a boot the tour had labelled FAILED for an unrelated
    // reason. An hour went into decoding a number that was never a register.
    //
    // So the diagnostic is unconditionally `(STAT << 32) | ISTAT`, read now, and the byte count
    // moves into word 1 beside the flag it belongs with. An all-zero pair still says the register
    // window read as nothing (a gated clock, an undeasserted reset, or a base that is not the
    // TRNG); a nonzero `STAT` with `SEEDED` clear still says the device is alive and the seeding
    // sequence did not finish. Both readings now hold whatever word 0 says.
    let diagnostic = (u64::from(regs().STAT.get()) << 32) | u64::from(regs().ISTAT.get());
    // Word 1 carries the byte count and, in its high half, the `STAT` this driver saw right after
    // `init`. `STAT.R256` there is the answer to "is this device in 256-bit mode", which decides
    // whether the 32 bytes above are 32 bytes of device output or 16 bytes and 16 of something
    // else. Nothing has read it on silicon yet; the bench procedure now says to.
    let bytes_in_hand = pool.remaining() as u64;
    send(
        READY,
        report,
        (u64::from(stat_after_init) << 32) | bytes_in_hand,
        diagnostic,
    );

    // **A device that answered with zeros is refused for the rest of the boot**, and that is a
    // stronger response than the report word alone. The report reaches whoever wired this service;
    // a client only ever sees a reply, so a service that reported dead and went on serving its
    // zero buffer would hand out those zeros as randomness to everyone who was not watching the
    // handshake. A device that answered with *nothing* is not refused: it has told the truth at
    // every step, `take` already answers `NO_ENTROPY` while it stays dry, and it recovers by
    // itself if it starts answering.
    serve(
        pool,
        report == proto::bringup_failure(proto::STEP_FIRST_ALL_ZERO),
    )
}

/// The serve loop: one endpoint, one wait point, forever. Identical in shape to `entropy.rs`'s,
/// because the contract (`entropy_protocol`) is the thing that does not change between backends.
fn serve(mut pool: Pool, refuse: bool) -> ! {
    loop {
        let (w0, cap, _) = recv_cap(REQ);
        if cap == rendezvous::NO_CAP {
            // A plain SEND on a CALL-only contract: nothing to answer. See `entropy.rs`'s
            // identical comment.
            continue;
        }
        let (count, word) = match proto::op(w0) {
            // `refuse` is a device this driver condemned at bring-up (see `_start`). It is
            // answered exactly the way a dry device is, because `NO_ENTROPY` already means the one
            // thing a client has to know: it is not getting randomness here.
            proto::GET if !refuse => pool.take(proto::want(w0), &mut generate),
            _ => (proto::NO_ENTROPY, 0),
        };
        reply(cap, count, word);
    }
}

/// **The two 8s are the same 8.** `jh7110_entropy::Pool::take` clamps to the width of the word it
/// answers with; `entropy_protocol::want` clamps to the width of the word the wire carries. This file
/// is the only one that depends on both, so it is where the agreement can be checked, and a
/// compile-time assert is the rung this project reaches for when a fact can be made unrepresentable
/// rather than remembered.
const _: () = assert!(jh7110_entropy::WORD_BYTES == proto::MAX_BYTES);

user_mode_runtime::panic_handler!();

// There is no capability slot for the registers, and that is worth a sentence rather than a
// silence: this driver reaches them by direct volatile access at `TRNG_VA`, the same way
// `console.rs` reaches the UART, not through a kernel-mediated `invoke` the way the virtio
// backends reach `Virtio` capabilities. The authority is the mapping the spawner installed; the
// driver holds no name for it.
