//! **The entropy service** (milestone 56; DECISIONS §44, notes/entropy.md).
//!
//! The one process that owns a virtio-rng device, and the only thing in the system that can read
//! it. Everything else holds an endpoint that means *"you may obtain randomness"*, which is a
//! strictly smaller authority than *"you may reach the device"*: a client cannot program the
//! queue, cannot see the DMA region the device writes into, and cannot ask the device for anything
//! this service did not ask on its behalf.
//!
//! ```text
//!   virtio-rng ──virtio (mmio or PCIe, §18)──► entropy ──the request endpoint──► clients
//!    (a device)                                  │        (CALL, bytes in the reply)
//!                                                └── its DMA page: nobody else maps it
//! ```
//!
//! Its whole authority is four things placed before it ran:
//!
//! - slot 0, the **request** endpoint (RECV): clients `CALL` here and nothing else;
//! - slot 1, an **`Irq`**: the device's completion interrupt;
//! - slot 2, a **`Virtio`**: the confined transport, and the only way it can reach the device;
//! - slot 3, a **readiness** endpoint (WRITE): one message once the first bytes are in hand;
//! - mapped: one DMA page, which nobody else maps.
//!
//! No initrd, no budget, no filesystem, no network. A compromised entropy service is a machine
//! whose random numbers an attacker chooses, which is exactly as much damage as owning the
//! entropy source should be worth, and it is why the device does not sit inside every program.
//!
//! # It passes the device's bytes through, and computes nothing
//!
//! No pool, no whitening, no mixing, no DRBG. There is no cryptography in this tree yet (that is
//! milestone 56's other half), and without a one-way function every transformation available here
//! is a reversible permutation: it would change the bytes without adding an unpredictability an
//! attacker could not undo, while making the security claim harder to state. So the claim stays
//! one sentence long: **these are the device's bytes**. See DECISIONS §44.
//!
//! What it does keep is a **buffer**, which is a different thing from a pool. One virtio request
//! fetches [`POOL_LEN`] bytes and the service hands them out in order, refilling when they run
//! out. Byte *i* out is byte *i* in, unmodified, handed to exactly one client, and zeroed behind
//! the cursor. That is a cache for round trips, not an entropy transformation, and it turns 32
//! device round trips into one.
//!
//! # When the device gives less than it was asked for
//!
//! virtio-rng is allowed to return fewer bytes than the buffer holds, and the used ring's `len` is
//! where it says so. **QEMU's really does**, which is worth stating as a measurement rather than a
//! spec allowance: the first version of this file passed a short buffer straight through to the
//! client, and the milestone-56 test caught a five-byte reply to an eight-byte request thirty draws
//! in. So the service takes what arrived and asks again for the rest, gathering across the boundary
//! (see [`Pool::take`]). It never pads, never repeats a byte it has already served, and never
//! substitutes a pseudo-random stand-in. If the device produces nothing at all across
//! [`REFILL_TRIES`] attempts, the reply is [`entropy_protocol::NO_ENTROPY`] and the caller finds out,
//! because a caller who cannot be given randomness must not be told otherwise (DECISIONS §42).
//!
//! Name: ratified 2026-09-14 (calef, working the unratified worklist), confirming milestone 63's
//! name table (design/roadmap/63-name-spellings.md). Introduced 2026-07-30 with milestone 56. The
//! same argument `clock` carries, made in the same sentence: 63 cites the two together as the
//! resource-name pattern it departed from for `credentialer`, on the ground that a credential
//! service never hands you a credential. This service does hand you entropy, so the pattern holds.
//! The word is also the field's own for the quantity, which the protected class keeps.
//!
//! The ratification also settles the pair: calef ratified `jh7110_entropy` on 2026-09-13 for the
//! other backend of this same contract, refusing `_source` there partly because this peer carries
//! no such suffix. Naming one of two backends for one contract decides how the other reads, so the
//! two are one decision made a day apart.
//!
//! # A second backend, and a smaller authority than the first (milestone 162)
//!
//! RDRAND/RDSEED (`x86_64`) and RNDR/RNDRRS (aarch64, `FEAT_RNG`) are **unprivileged CPU instructions**:
//! no MMIO, no capability, no device discovery, executable at any privilege level. That is exactly
//! the trap DECISIONS §44 exists to name: if this service let a client obtain those bytes by
//! executing the instruction itself, the request endpoint above would be theatre, because ambient
//! authority to the CPU is ambient authority to entropy. So [`instr`] is not a function a client
//! links; it is the same shape as the virtio backend above, a second way *this* process reaches
//! bytes, still gated behind the one endpoint every client already holds.
//!
//! **The kernel decides which backend a boot gets, the same way it decides `Bus::Mmio` vs.
//! `Bus::Pci`.** `ID_AA64ISAR0_EL1.RNDR` (aarch64) is only readable at EL1 (a userspace `MRS` on it
//! traps as an unknown register, `arch::isa::init` already reads it into the boot-time record), so
//! this process never probes for the feature itself; it trusts the kernel's choice of which mode to
//! spawn it in the same way it already trusts a granted `Virtio` capability to name a real device.
//! See `kernel/src/user/entropy_service.rs::ensure_instruction`.
//!
//! **Why RDSEED/RNDRRS and not RDRAND/RNDR.** Intel's own DRNG Software Implementation Guide (rev.
//! 2.2) documents `RDRAND` as SP800-90C RBG2(P) output: a hardware `CTR_DRBG` seeded from the
//! conditioned entropy source, servicing up to 511 draws per reseed. `RDSEED` instead comes
//! straight off the SP800-90C RBG3(XOR) enhanced non-deterministic generator, one draw, one fresh
//! sample of the conditioned entropy source, no DRBG in the path. ARM's RNDR/RNDRRS pair is the same
//! split (RNDR may be DRBG-buffered the way RDRAND is; RNDRRS forces a reseed from the entropy
//! source before it answers, the way RDSEED does). This service's whole discipline is "no pool, no
//! whitening, no mixing, **no DRBG**", stated two paragraphs up; taking RDRAND or RNDR would
//! silently reintroduce, in hardware, the exact primitive this file already refuses to add in
//! software. RDSEED/RNDRRS are the instructions that keep the claim "these are the device's bytes"
//! true. The cost is real and is not hidden: both are rate-limited by the physical entropy source
//! and can run dry under load in a way RDRAND/RNDR, buffered by their DRBG, do not; a caller that
//! exhausts the retry budget gets [`proto::NO_ENTROPY`], the same honest answer a dry virtio device
//! gives, rather than a fallback to the DRBG-backed sibling instruction.
//!
//! # BUGS
//!
//! **A first bufferful of zeros is reported as a dead device, and it could in principle be
//! randomness.** Both backends here refuse to report [`proto::READY`] when their first draw is all
//! zero (`entropy_protocol::readiness`), and then answer every request `NO_ENTROPY` for the rest of
//! the boot rather than serving those bytes. On a working source that is wrong with probability
//! 2^-2048 for the virtio backend's 256-byte bufferful and 2^-64 for the instruction backend's
//! eight bytes. The trade is deliberate and is stated where the contract is:
//! `entropy_protocol`'s own `BUGS`, and the roadmap block that recorded the defect
//! (`design/roadmap/159-jh7110-trng-driver.md`, "The bench ran it, 2026-09-04").
//!
//! **A condemned backend does not recover.** There is no path back short of restarting the
//! service, even if the device starts producing bytes a moment later.
//!
//! **Nothing checks a byte after bring-up.** A device that answers correctly once and degrades, or
//! whose buffer latches and repeats, is served through untouched. Continuous health testing is
//! `design/decisions/137-trng-health-tests.md`'s question and is `PROPOSED`, not decided here.
//!
//! On-die conditioning is a different question from a DRBG and this service does not refuse it:
//! both Intel's noise source and ARM's entropy source run an AES-CBC-MAC-shaped conditioner
//! (SP800-90B's noise-source-plus-conditioning-function model) before either instruction's output is
//! ever visible to software. That conditioning is part of the instruction's own architectural
//! contract, the same way it is part of what "a device" means for virtio-rng; this file adds nothing
//! on top of it either way.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use abi::rendezvous;
use entropy_protocol as proto;
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::virtio::{
    virtio_notify, virtio_read_reg, virtio_ring_barrier, virtio_setup_queue, virtio_write_reg,
};
use user_mode_runtime::{exit, irq_ack, irq_wait, recv_cap, reply, send};

/// Capability slots for the virtio backend, by convention with `kernel/src/user/entropy_service.rs`.
const REQ: u64 = 0;
const IRQ: u64 = 1;
const VIRTIO: u64 = 2;
const READY: u64 = 3;

/// What `arg0` means, by convention with `kernel/src/user/entropy_service.rs::start` and
/// `::start_instruction`. Not a wire contract between two *user* programs (rule 7 does not apply):
/// it is the kernel's own spawn-argument convention with the one program it spawns, the same
/// footing `DMA_VA` below is already on.
const MODE_VIRTIO: u64 = 0;
const MODE_INSTRUCTION: u64 = 1;

/// Capability slots for the instruction backend: two, not four. No `Irq`, no `Virtio`, no DMA
/// mapping at all, because `RNDRRS`/`RDSEED` need none of them. Smaller authority than the virtio
/// backend's, which is itself already minimal.
const I_REQ: u64 = 0;
const I_READY: u64 = 1;

/// Where the kernel maps this service's DMA page. Must match `kernel/src/user/entropy_service.rs`.
const DMA_VA: u64 = address_space_map::pair_page(0x0000_0000_0090_0000);

// SAFETY: the wiring maps one page read/write at DMA_VA before this program runs (milestone 139;
// see `user_mode_runtime::mapped_window`, which is what collapsed the hand-rolled r8/w8/r16/w16/r32 below).
const WINDOW: MappedWindow = unsafe { MappedWindow::new(DMA_VA, PAGE) };

// virtio-mmio register offsets. The §18 transport seam speaks this vocabulary on both buses, so
// this driver runs over PCIe and over mmio without knowing which one it got.
const MAGIC: u64 = 0x000;
const DEVICE_ID: u64 = 0x008;
const DEVICE_FEATURES: u64 = 0x010;
const DEVICE_FEATURES_SEL: u64 = 0x014;
const DRIVER_FEATURES: u64 = 0x020;
const DRIVER_FEATURES_SEL: u64 = 0x024;
const INTERRUPT_STATUS: u64 = 0x060;
const INTERRUPT_ACK: u64 = 0x064;
const STATUS: u64 = 0x070;

const S_ACKNOWLEDGE: u32 = 1;
const S_DRIVER: u32 = 2;
const S_DRIVER_OK: u32 = 4;
const S_FEATURES_OK: u32 = 8;
const F_VERSION_1_HI: u32 = 1; // feature bit 32
const F_ACCESS_PLATFORM_HI: u32 = 1 << 1; // feature bit 33 (behind an IOMMU)

const VIRTQ_DESC_F_WRITE: u16 = 2;

/// The virtio device type this driver will talk to. Checked, not assumed: the display driver found
/// the PCI transport answering every driver's `DeviceID` read with a hardcoded 2, and it found it
/// because it was the first driver that looked.
const VIRTIO_ID_ENTROPY: u32 = 4;

/// virtio-rng has exactly one virtqueue, the request queue. Nothing rides the second slot of the
/// two-queue confinement ceiling here.
const RNG_Q: u64 = 0;
const QSIZE: u16 = 8;

/// The DMA page's layout: the queue's rings at the kernel's per-queue offsets, then the buffer the
/// device fills. All inside the one 4 KiB page the kernel granted, so the shadow-ring validator
/// confines every address the device is handed.
const Q_DESC: u64 = 0x000;
const Q_AVAIL: u64 = 0x080;
const Q_USED: u64 = 0x100;
const POOL_OFF: u64 = 0x400;

/// How many bytes one device request fetches. 256 is 32 clients' worth of [`proto::MAX_BYTES`] per
/// round trip, and it leaves the rest of the page unused rather than crowding the rings.
const POOL_LEN: u64 = 256;

/// How many times to re-ask a device that returned nothing before telling the caller so. A device
/// that has momentarily run dry is worth one more request; a device that never answers is a fact
/// the caller needs, not something to spin on.
const REFILL_TRIES: usize = 4;

/// How many interrupt wakeups to absorb while waiting for one completion. A wakeup can be stale,
/// coalesced, or a previous operator's, so the **used ring**, not the wakeup, is the completion;
/// the bound is what keeps a device that stopped answering from becoming a hang.
const WAIT_WAKEUPS: usize = 64;

/// Bring-up failures, in a `0xDEAD_...` word so a failure names its step instead of hanging.
const E_MAGIC: u64 = 0x01;
const E_DEVICE_ID: u64 = 0x02;
const E_FEATURES: u64 = 0x03;
const E_QUEUE: u64 = 0x04;

/// The virtio steps above are this backend's own, and `entropy_protocol` reserves `0x01..=0x0f` for
/// exactly that. The two steps every backend shares (`STEP_NO_FIRST_BYTES`, `STEP_FIRST_ALL_ZERO`)
/// start at `0x10`, and this assert is what keeps a fifth virtio step from silently becoming one
/// of them: a report word decodes to one step or it decodes to a lie.
const _: () = assert!(E_QUEUE < proto::STEP_NO_FIRST_BYTES);

fn r8(off: u64) -> u8 {
    WINDOW.r8(off)
}

fn w8(off: u64, v: u8) {
    WINDOW.w8(off, v);
}

fn r16(off: u64) -> u16 {
    WINDOW.r16(off)
}

fn w16(off: u64, v: u16) {
    WINDOW.w16(off, v);
}

fn r32(off: u64) -> u32 {
    WINDOW.r32(off)
}

fn mr(off: u64) -> u32 {
    virtio_read_reg(VIRTIO, off) as u32
}

fn mw(off: u64, v: u32) {
    virtio_write_reg(VIRTIO, off, v as u64);
}

fn die(code: u64) -> ! {
    send(READY, proto::bringup_failure(code), 0, 0);
    exit();
}

/// Write descriptor 0: the whole buffer, device-**writable**, which is the direction that matters
/// here. This is a receive-only device; nothing this process holds is ever read by it.
fn write_desc(addr: u64, len: u32) {
    // Inside the DMA page, at the descriptor table the kernel programmed the queue with; all four
    // writes are bounds-checked by `WINDOW` rather than trusted by hand.
    WINDOW.write(Q_DESC, addr);
    WINDOW.write(Q_DESC + 8, len);
    WINDOW.write::<u16>(Q_DESC + 12, VIRTQ_DESC_F_WRITE);
    WINDOW.write::<u16>(Q_DESC + 14, 0);
}

/// The service's running state: where the device's bytes are and how many of them are still ours
/// to give. Not a pool: `cursor` only ever moves forward, so no byte is served twice.
struct Pool {
    /// The DMA region's physical base. Descriptors speak physical addresses; a process knows
    /// virtual ones, so the spawner passes this in (rule 2: the driver is told, never told to look).
    direct_memory_access_phys: u64,
    /// Available-ring index we have published up to.
    avail: u16,
    /// Used-ring index we have drained up to.
    seen: u16,
    /// Next unserved byte's offset within the buffer.
    cursor: u64,
    /// One past the last byte the device wrote. `cursor == filled` means empty.
    filled: u64,
}

impl Pool {
    /// Ask the device for a bufferful and wait for it. Returns how many bytes arrived, which the
    /// spec allows to be fewer than asked for and, on a dry device, zero.
    fn request(&mut self) -> u64 {
        write_desc(self.direct_memory_access_phys + POOL_OFF, POOL_LEN as u32);
        w16(Q_AVAIL + 4 + (self.avail % QSIZE) as u64 * 2, 0); // ring[idx] = descriptor head 0
        virtio_ring_barrier(); // the descriptor must be visible before the index that advertises it
        self.avail = self.avail.wrapping_add(1);
        w16(Q_AVAIL + 2, self.avail);
        virtio_ring_barrier(); // and the index before the doorbell

        // The kernel walks the descriptor we just published, refuses it if it leaves our region,
        // and only then rings the device.
        if virtio_notify(VIRTIO, RNG_Q) < 0 {
            return 0; // the kernel refused our own in-region descriptor: a bug here, not a dry device
        }

        // **The completion is the used ring advancing, not the wakeup**, and this driver looks
        // before it blocks. Two reasons, and the second one is a fact about the board rather than
        // an optimisation:
        //
        //  1. QEMU completes a virtio request synchronously inside `NOTIFY`, so on the machine this
        //     runs on the ring has already moved by the time we get here.
        //  2. **PCI INTx lines are shared four ways on both `virt` boards** (`pci::intx_irq`
        //     swizzles device number modulo 4) while the kernel routes an intid to exactly one
        //     endpoint (`sched::bind_irq`). With five PCI functions attached there is no
        //     unshared line left, so a driver that blocked before looking would be betting on
        //     owning its line. Looking first costs two loads and removes the bet.
        //
        // The wait is still here, and is what a genuinely asynchronous device gets: a wakeup can be
        // stale, coalesced, or a previous operator's, so the loop re-checks the ring each time.
        // Bounded, so a device that stopped answering becomes an honest "no entropy" rather than a
        // hang.
        let mut len = 0;
        for _ in 0..WAIT_WAKEUPS {
            virtio_ring_barrier();
            if r16(Q_USED + 2) != self.seen {
                // used-ring element: { u32 id; u32 len }. `len` is the device saying how many
                // bytes it actually wrote, and it is allowed to be short.
                let slot = (self.seen % QSIZE) as u64;
                len = (r32(Q_USED + 4 + slot * 8 + 4) as u64).min(POOL_LEN);
                self.seen = self.seen.wrapping_add(1);
                break;
            }
            irq_wait(IRQ);
            self.quiet();
        }
        // Quiet the device and re-enable the line even on the path that never waited: the device
        // raised its line when it completed and the kernel masked it, and a line left masked is one
        // the *next* holder of a shared intid never hears from. `irq::ACK` is
        // `arch::irq::enable`, which is idempotent, so this is safe when nothing fired.
        self.quiet();
        len
    }

    /// Acknowledge the device's interrupt and re-enable the line at the controller, in that order:
    /// the disk driver's discipline, and the reason an interrupt does not immediately re-fire.
    fn quiet(&self) {
        let istatus = mr(INTERRUPT_STATUS);
        mw(INTERRUPT_ACK, istatus);
        irq_ack(IRQ); // re-enable the line the kernel masked when it fired
    }

    /// Refill the buffer, retrying a device that returned nothing. `false` when the device gave us
    /// nothing at all, which is the one case a client is told about.
    fn refill(&mut self) -> bool {
        for _ in 0..REFILL_TRIES {
            let n = self.request();
            if n > 0 {
                self.cursor = 0;
                self.filled = n;
                return true;
            }
        }
        false
    }

    /// Take `n` bytes, as a little-endian word plus a count.
    ///
    /// **The loop is here because the buffer boundary is not the client's problem.** A refill can
    /// land short (QEMU's virtio-rng really does return fewer than the 256 bytes asked for, which
    /// is what turned this from a spec allowance into a fixed bug), so the bytes a client asked for
    /// can straddle two device requests. Gathering across the boundary keeps a short *device* read
    /// invisible, which is the difference between "the device paced us" and "the caller cannot have
    /// what it asked for".
    ///
    /// A count below `n` therefore means one thing only: the device went dry part-way through. It
    /// is never padding. Zero means it was dry from the start.
    fn take(&mut self, n: u64) -> (u64, u64) {
        let mut word = 0u64;
        let mut got = 0;
        while got < n {
            if self.cursor == self.filled && !self.refill() {
                break;
            }
            let run = (n - got).min(self.filled - self.cursor);
            for i in 0..run {
                let at = POOL_OFF + self.cursor + i;
                word |= (r8(at) as u64) << (8 * (got + i));
                // Zero behind the cursor. The cursor alone already guarantees no byte is served
                // twice; this is hygiene, so a byte a client now holds is not also still sitting in
                // a page this long-lived process keeps mapped for the rest of the boot.
                w8(at, 0);
            }
            self.cursor += run;
            got += run;
        }
        (got, word)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(mode: u64, direct_memory_access_phys: u64, _arg2: u64) -> ! {
    if mode == MODE_INSTRUCTION {
        // No virtio device, no DMA page, no IRQ: the kernel would not have spawned this mode
        // unless it already confirmed the instruction's feature bit (aarch64's `ID_AA64ISAR0_EL1`,
        // checked at EL1 because EL0 cannot read it). This process trusts that the same way it
        // trusts a granted `Virtio` capability to name a real device.
        serve_instruction();
    }
    debug_assert_eq!(mode, MODE_VIRTIO, "entropy: unknown spawn mode {mode:#x}");
    if mr(MAGIC) != 0x7472_6976 {
        die(E_MAGIC);
    }
    // **Check what we are talking to**, for the reason the keyboard driver records: a transport
    // that answered every `DeviceID` read with a hardcoded number went unnoticed until a driver
    // looked. An entropy service that programmed a disk's queue would be worse than most.
    if mr(DEVICE_ID) != VIRTIO_ID_ENTROPY {
        die(E_DEVICE_ID);
    }

    mw(STATUS, 0);
    mw(STATUS, S_ACKNOWLEDGE);
    mw(STATUS, S_ACKNOWLEDGE | S_DRIVER);

    mw(DRIVER_FEATURES_SEL, 0);
    mw(DRIVER_FEATURES, 0); // virtio-rng defines no low-word features at all
    mw(DEVICE_FEATURES_SEL, 1);
    let dev_hi = mr(DEVICE_FEATURES);
    let mut ack_hi = F_VERSION_1_HI;
    if dev_hi & F_ACCESS_PLATFORM_HI != 0 {
        ack_hi |= F_ACCESS_PLATFORM_HI; // behind an IOMMU, which on the PCIe wiring it is
    }
    mw(DRIVER_FEATURES_SEL, 1);
    mw(DRIVER_FEATURES, ack_hi);

    mw(STATUS, S_ACKNOWLEDGE | S_DRIVER | S_FEATURES_OK);
    if mr(STATUS) & S_FEATURES_OK == 0 {
        die(E_FEATURES);
    }

    // The kernel programs the queue's ring addresses; this service never writes a queue-address
    // register, which is the §18 seam and the reason the shadow ring can be trusted.
    if virtio_setup_queue(VIRTIO, QSIZE as u64, RNG_Q) != 0 {
        die(E_QUEUE);
    }
    mw(
        STATUS,
        S_ACKNOWLEDGE | S_DRIVER | S_FEATURES_OK | S_DRIVER_OK,
    );

    let mut pool = Pool {
        direct_memory_access_phys,
        avail: 0,
        seen: 0,
        cursor: 0,
        filled: 0,
    };

    // Fetch the first bufferful before reporting ready, so "the service is up" means "a client
    // that asks will be answered" rather than "the handshake completed". A device that produces
    // nothing is a fact worth learning at boot rather than at the first `CALL`.
    //
    // **And the bytes decide the word, not the fact that a request completed.** This line used to
    // send `proto::READY` unconditionally with the truth in a second word; the JH7110 backend was
    // the one that got caught doing it on silicon (2026-09-04, radon), but the shape of the bug
    // was here too. `readiness` reads the bufferful the device actually left in the DMA page,
    // through the same bounds-checked window everything else here uses.
    let first = pool.refill();
    let report = proto::readiness((0..pool.filled).map(|i| r8(POOL_OFF + i)));
    send(READY, report, u64::from(first), pool.filled);

    // A device that wrote a page of zeros is condemned for the rest of the boot; a device that
    // wrote nothing is not. See `jh7110_entropy.rs`'s `_start`, which says why at length: the report
    // word reaches whoever wired the service, and a client only ever sees a reply.
    serve(
        pool,
        report == proto::bringup_failure(proto::STEP_FIRST_ALL_ZERO),
    )
}

/// The serve loop: one endpoint, one wait point, forever.
fn serve(mut pool: Pool, refuse: bool) -> ! {
    loop {
        let (w0, cap, _) = recv_cap(REQ);
        if cap == rendezvous::NO_CAP {
            // A plain SEND on a CALL-only contract. Nobody is waiting for an answer, so there is
            // nothing to do and nothing to report; drop it rather than replying into a slot we do
            // not hold. (The clock service answers the same way for the same reason.)
            continue;
        }
        let (count, word) = match proto::op(w0) {
            // `refuse` is a device this service condemned at bring-up for answering with zeros. It
            // is answered the way a dry device is: `NO_ENTROPY` already means the one thing a
            // client has to know, which is that it is not getting randomness here.
            proto::GET if !refuse => pool.take(proto::want(w0)),
            // There is exactly one operation, and the reply's first word is a byte count with no
            // room in it for an error code. So an unknown opcode is answered "you got nothing",
            // which is true. A second operation would be the moment to widen the reply, and that
            // is a contract change rather than a branch added here.
            _ => (proto::NO_ENTROPY, 0),
        };
        reply(cap, count, word);
    }
}

/// **The instruction backend** (milestone 162): RDSEED on `x86_64`, RNDRRS on aarch64. Confined here
/// rather than lifted into a shared userspace arch-abstraction crate: nothing in `components/src/`
/// has
/// needed one before this (the precedent, `crates/user_mode_runtime`, is the syscall ABI itself, one crate
/// *every* program depends on, not a single-consumer helper), so a module inside the one program
/// that uses it is the smaller thing to build. **Provisional**, flagged for calef: if a second
/// userspace program ever needs per-architecture `asm!` of its own, this is the first candidate to
/// pull out into a crate rather than duplicate.
mod instr {
    /// How many times to retry a transient "no data this cycle" result before giving up.
    ///
    /// Both instructions this module uses draw straight from the conditioned entropy source rather
    /// than a buffered DRBG (see the module doc above for why that is the point), so underflow under
    /// load is expected rather than exceptional, unlike `RDRAND`/`RNDR`. Intel's DRNG Software
    /// Implementation Guide (rev. 2.2, §5.3.1.2) calls this service an "asynchronous application"
    /// (one that cannot block indefinitely waiting on a seed) and says to give up after "somewhere
    /// between 1 and 100" retries depending on sensitivity to delay. This service is one serialized
    /// request queue, not many threads hammering the instruction at once, so the high end of that
    /// range costs nothing. ARM's own text for `RNDR`/`RNDRRS` ("if the instruction cannot return a
    /// genuine random number in a reasonable period of time") reads as the hardware already
    /// retrying before it ever signals failure, and Linux's `arch/arm64/include/asm/archrandom.h`
    /// does not retry at the instruction level at all; one bound shared by both architectures is not
    /// a stronger claim than either vendor makes, and it is simpler than two.
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    const RETRIES: u32 = 100;

    /// Draw eight bytes of real entropy, or `None` if the source stayed dry across [`RETRIES`]
    /// attempts. Unlike the virtio backend's [`super::Pool`], there is no gathering across a
    /// boundary to do: one successful draw is exactly [`super::proto::MAX_BYTES`] bytes, so a
    /// single request never needs more than one attempt loop.
    #[cfg(target_arch = "x86_64")]
    pub fn draw() -> Option<[u8; 8]> {
        for _ in 0..RETRIES {
            let v: u64;
            let ok: u8;
            // SAFETY: `rdseed` is unprivileged at any ring (Intel DRNG guide §3.3.2: "no hardware
            // ring requirements... may be invoked as part of an operating system... or directly by
            // an application") and touches no memory. Success/failure rides the carry flag, not an
            // exception, which `setc` captures in the same block before anything else can disturb
            // it.
            unsafe {
                core::arch::asm!(
                    "rdseed {v}",
                    "setc {ok}",
                    v = out(reg) v,
                    ok = out(reg_byte) ok,
                    options(nomem, nostack),
                );
            }
            if ok != 0 {
                return Some(v.to_le_bytes());
            }
            // SAFETY: `pause` is Intel's own documented spin-loop hint for exactly this retry shape
            // (DRNG guide §5.3.1.1/.2); it touches no memory and has no failure mode.
            unsafe { core::arch::asm!("pause", options(nomem, nostack)) };
        }
        None
    }

    /// See the `x86_64` [`draw`] above; same contract, ARM's instruction.
    #[cfg(target_arch = "aarch64")]
    pub fn draw() -> Option<[u8; 8]> {
        for _ in 0..RETRIES {
            let v: u64;
            let ok: u64;
            // SAFETY: `RNDRRS` (`S3_3_C2_C4_1`, the numeric encoding rather than the mnemonic alias
            // so this assembles regardless of the assembler's FEAT_RNG name table) is unprivileged
            // once FEAT_RNG is present. The kernel already confirmed that before spawning this
            // process in instruction mode (`ID_AA64ISAR0_EL1` is not EL0-readable, so this file
            // cannot check it itself); executing it without the feature present would trap as an
            // unknown register access. Success/failure is this instruction's own architected side
            // effect on PSTATE.NZCV (Arm ARM, "Random Number instructions": `0b0000` success,
            // `0b0100` otherwise), which `cset ok, ne` reads back in the same block, the identical
            // idiom Linux's `arch/arm64/include/asm/archrandom.h` uses. Touches no memory.
            unsafe {
                core::arch::asm!(
                    "mrs {v}, S3_3_C2_C4_1",
                    "cset {ok}, ne",
                    v = out(reg) v,
                    ok = out(reg) ok,
                    options(nomem, nostack),
                );
            }
            if ok != 0 {
                return Some(v.to_le_bytes());
            }
        }
        None
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    pub fn draw() -> Option<[u8; 8]> {
        // The kernel never spawns MODE_INSTRUCTION on an architecture with neither instruction
        // (riscv64: milestone 159's JH7110 TRNG is the real hardware source there), so this arm
        // exists only to keep the crate building for a fourth architecture that adds neither
        // instruction, rather than to be reached.
        None
    }
}

/// The instruction-mode serve loop: `RECV` on `I_REQ`, one instruction draw per request, one wait
/// point. No device, so no bring-up steps and nothing that can [`die`]: the only degenerate case is
/// a dry source, reported the same way a request's own [`proto::NO_ENTROPY`] answer already is.
fn serve_instruction() -> ! {
    let first = instr::draw();
    let report = proto::readiness(first.into_iter().flatten());
    send(
        I_READY,
        report,
        u64::from(first.is_some()),
        first.map_or(0, |_| proto::MAX_BYTES),
    );
    // Eight zero bytes out of `RDSEED`/`RNDRRS` condemn the source for the boot, the same rule the
    // two device backends follow. The false-positive probability is 2^-64 here rather than the
    // 2^-256 a 32-byte draw gets, which is the width of the answer rather than a weaker check;
    // `entropy_protocol`'s `BUGS` carries the number.
    let refuse = report == proto::bringup_failure(proto::STEP_FIRST_ALL_ZERO);
    loop {
        let (w0, cap, _) = recv_cap(I_REQ);
        if cap == rendezvous::NO_CAP {
            // Same reasoning as `serve`'s identical line: nobody is waiting for an answer.
            continue;
        }
        let (count, word) = match proto::op(w0) {
            proto::GET if !refuse => match instr::draw() {
                Some(bytes) => (proto::want(w0), u64::from_le_bytes(bytes)),
                None => (proto::NO_ENTROPY, 0),
            },
            _ => (proto::NO_ENTROPY, 0),
        };
        reply(cap, count, word);
    }
}

user_mode_runtime::panic_handler!();
