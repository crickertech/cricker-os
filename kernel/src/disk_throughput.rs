//! **Fatal risk 6's bench boot** (milestone 261 (the NVMe driver leaves the kernel); design/fatal-risks/README.md risk 6,
//! notes/risk-6-bench-evening.md). Behind the `disk_throughput` feature, name provisional.
//!
//! The decisive experiment is "one real, non-virtio device on real silicon, confined, at
//! throughput", and on xenon two things have to be true on the night that no code can make true:
//! the VT-d unit this kernel programs must be the one that owns the NVMe's requester id, and the
//! namespace's LBA size must give `blocks_per` in `1..=8`. Either one failing used to arrive at the
//! bench as something that looked like a result: a throughput number from a device nothing was
//! translating, or a boot test that said "skipped". So this boot prints **both verdicts before it
//! measures anything**, then measures, then prints **one verdict line** whose first word cannot be
//! read as a pass unless both held.
//!
//! What it prints, in order, every line prefixed `disk-throughput:` so a photograph of the screen
//! and a serial capture are read the same way:
//!
//! 1. `preflight 1/2 dmar scope : PASS|FAIL`, with which unit owns the controller and how;
//! 2. `preflight 2/2 lba size   : PASS|FAIL`, with the LBA size and `blocks_per`;
//! 3. `ipc floor`, `write`, `flush` and `read`, each with its count and its time;
//! 4. `verdict CONFINED-AT-RATE | UNCONFINED | SKIPPED | FAILED`, and nothing after it.
//!
//! # What the numbers count
//!
//! The client is the kernel's boot thread, playing the part a client process would: every block is
//! one `CALL` across a rendezvous into the EL0 server, which builds one NVMe command, rings the
//! doorbell, polls the completion's phase tag, and replies. So a figure is **one command in flight,
//! 4096 bytes per command, polled, plus two context switches per block**. That is a lower bound on
//! the device and an honest measure of this driver; it is not comparable to `fio` at queue depth 32,
//! and the `ipc floor` line exists so the IPC share can be subtracted by whoever quotes it.
//!
//! # BUGS
//!
//! - **It writes.** A window of [`WINDOW_BLOCKS`] blocks starting 1 MiB into the namespace is
//!   overwritten and read back. On xenon that disk was wiped on 2026-09-17 and holds nothing; on
//!   any other machine this build is a way to destroy data, which is why it is a feature no gate
//!   builds and why the verdict names the window it wrote.
//! - **The client is in the kernel**, so the figure leaves out the cost of a client that is itself
//!   a process. The server is the thing risk 6 is about, and it is the same process either way.
//! - **Sequential only**, one pass, no warm-up and no repeats. A second boot is the repeat.

use filesystem_protocol::blk;

use crate::non_volatile_memory_express::{Absent, BLOCK_SIZE, Error};
use crate::user::non_volatile_memory_express_service::{self as service, Wiring};
use crate::{arch, println};

/// Where the written window starts, in 4096-byte blocks: 1 MiB in, clear of a GPT's primary header
/// and table (LBA 0..34 at 512 bytes) should the disk ever carry one again.
const WINDOW_START: u64 = 256;

/// How many blocks the write and read phases each move: 64 MiB, or as much of the namespace past
/// [`WINDOW_START`] as there is. Large enough that a phase takes tens of milliseconds on silicon,
/// small enough that TCG finishes it in seconds.
const WINDOW_BLOCKS: u64 = 16_384;

/// How many `SIZE` round trips the IPC floor is timed over. `SIZE` touches no device, so this is
/// the rendezvous and the two context switches alone.
const FLOOR_CALLS: u64 = 2_000;

/// Run the preflight and the measurement, print the verdict, and never come back.
pub fn run() -> ! {
    println!();
    println!("disk-throughput: fatal risk 6 bench boot (milestone 261). WRITES to the NVMe disk.");
    println!(
        "disk-throughput: preflight first; a skip is not a pass. {} build, {} Hz counter.",
        // The photograph has to say which, because a debug build's figures understate the
        // driver and `cargo xtask disk-throughput --debug` is the documented fallback.
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "release"
        },
        arch::timer::frequency(),
    );
    let verdict = measure();
    println!("disk-throughput: verdict {verdict}");
    println!("disk-throughput: done, halting.");
    arch::halt(arch::HaltReason::measurement_boot());
}

/// The verdict line's text. Kept as strings built at the one place each is decided, so the
/// preflight and the verdict cannot disagree about what was checked.
enum Verdict {
    ConfinedAtRate {
        read: u64,
        write: u64,
        unit: crate::iommu::Scope,
    },
    Unconfined {
        read: u64,
        write: u64,
    },
    Skipped(&'static str),
    Failed(&'static str),
}

impl core::fmt::Display for Verdict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Verdict::ConfinedAtRate { read, write, unit } => write!(
                f,
                "CONFINED-AT-RATE: read {read} B/s, write {write} B/s, EL0 driver; {unit}"
            ),
            Verdict::Unconfined { read, write } => write!(
                f,
                "UNCONFINED: read {read} B/s, write {write} B/s, but nothing this kernel \
                 programmed translates this device's DMA. Not risk 6's answer."
            ),
            Verdict::Skipped(why) => write!(f, "SKIPPED: {why}. Not a pass."),
            Verdict::Failed(why) => write!(f, "FAILED: {why}"),
        }
    }
}

/// **The driver whose number is published is the measured one.** An ordinary boot measures the
/// progenitor and the progenitor measures everything it loads; this boot replaces the hand-over, so
/// it runs the same chain itself through `trust::require_program` (milestone 563 (a seal check that
/// reads bytes cannot see a check that was dropped) moved it there from this file, so the soak,
/// job-mix and bench boots share it): the archive's measurement table must be the one this kernel
/// image vouches for, and the NVMe server's bytes must hash to that table's entry for it. Either
/// refusal halts with `MEASURED BOOT REFUSED`. It is also what keeps `TRUST_ROOT` in the image at
/// all: with the hand-over gone nothing else reads it, a release build drops it, and
/// `uefi_loader`'s seal check then refuses the pair.
fn measured_server() -> Result<&'static [u8], &'static str> {
    const NAME: &str = "non_volatile_memory_express";
    let image = crate::trust::require_program(NAME)
        .ok_or("no non_volatile_memory_express program in the initrd archive")?;
    let hex = measured_boot::hex(&measured_boot::sha256(image));
    println!(
        "disk-throughput: measured    {NAME} sha256 {}.. matches the table this kernel vouches for",
        core::str::from_utf8(&hex[..16]).unwrap_or("?"),
    );
    Ok(image)
}

fn measure() -> Verdict {
    let image = match measured_server() {
        Ok(image) => image,
        Err(why) => return Verdict::Failed(why),
    };
    let disk = match service::ensure_or_why(image) {
        Ok(disk) => disk,
        Err(Absent::NoController) => {
            println!(
                "disk-throughput: preflight 1/2 dmar scope : FAIL  no NVMe controller on the bus"
            );
            return Verdict::Skipped("no NVMe controller on the bus");
        }
        Err(Absent::Refused {
            rid,
            why: Error::UnsupportedNamespace { lba_shift, blocks },
        }) => {
            // The server never started, so there is no wiring to ask for a scope; the controller's
            // requester id is enough to answer it anyway, and the answer is still worth the photo.
            print_scope(rid, crate::iommu::scope_of(rid));
            println!(
                "disk-throughput: preflight 2/2 lba size   : FAIL  {}-byte lbas ({blocks} of them): \
                 4096-byte blocks need 512, 1024, 2048 or 4096 (blocks_per 1..=8)",
                1u64.checked_shl(lba_shift as u32).unwrap_or(0),
            );
            return Verdict::Skipped("the namespace's lba size gives no blocks_per in 1..=8");
        }
        Err(Absent::Refused { rid, why }) => {
            print_scope(rid, crate::iommu::scope_of(rid));
            println!("disk-throughput: controller {rid:#06x} failed bring-up: {why:?}");
            return Verdict::Failed("the controller is present and failed bring-up (line above)");
        }
    };
    if let Some(report) = disk.wait_for_ready()
        && report[0] != filesystem_protocol::fixture::READY
    {
        println!(
            "disk-throughput: the server did not come up: step word {:#x}, first-read status {:#x}",
            report[0], report[2]
        );
        return Verdict::Failed("the EL0 server reported a failed step (line above)");
    }

    let confined = print_scope(disk.rid, disk.scope);
    let lba = BLOCK_SIZE as u64 / disk.blocks_per as u64;
    println!(
        "disk-throughput: preflight 2/2 lba size   : PASS  {lba}-byte lbas, blocks_per {} \
         (needs 1..=8); namespace {} bytes",
        disk.blocks_per, disk.size_bytes,
    );
    if !confined {
        println!(
            "disk-throughput: measuring anyway, labelled: a rate from an untranslated device \
             answers a different question"
        );
    }

    let total = disk.size_bytes / BLOCK_SIZE as u64;
    if total <= WINDOW_START + 1 {
        return Verdict::Failed("the namespace is too small for the window");
    }
    let count = WINDOW_BLOCKS.min(total - WINDOW_START);

    // The IPC floor: the same path with no device on it.
    let start = arch::timer::now();
    for _ in 0..FLOOR_CALLS {
        if disk.blk(blk::SIZE, 0) as u64 != disk.size_bytes {
            return Verdict::Failed("SIZE answered the wrong size during the ipc floor");
        }
    }
    let floor = arch::timer::now().wrapping_sub(start);
    println!(
        "disk-throughput: ipc floor   {FLOOR_CALLS} SIZE round trips in {} us ({} ns each, no device)",
        micros(floor),
        nanos(floor) / FLOOR_CALLS,
    );

    // Write: stamp each block's first and last words with its number, so the read phase can tell
    // a block that came back from the wrong place. The rest of the page is a fixed pattern filled
    // once, outside the timed loop.
    // SAFETY: the blk contract's turn-taking. A request is a CALL, so this thread holds the buffer
    // exactly while the server is not running.
    unsafe { disk.transfer_block() }.fill(0x5a);
    let start = arch::timer::now();
    for b in WINDOW_START..WINDOW_START + count {
        stamp(&disk, b);
        if disk.blk(blk::WRITE, b) != 0 {
            return Verdict::Failed("the server refused a write inside the namespace");
        }
    }
    let write = arch::timer::now().wrapping_sub(start);
    print_phase("write", count, write);

    let start = arch::timer::now();
    let flushed = disk.blk(blk::FLUSH, 0);
    let flush = arch::timer::now().wrapping_sub(start);
    if flushed < 1 {
        return Verdict::Failed("the server refused a flush");
    }
    println!(
        "disk-throughput: flush       one FLUSH in {} us",
        micros(flush)
    );

    // Read, and verify every block's stamp. Clobbering the stamp words first is what makes a read
    // that never reached the device fail rather than pass on what the write phase left behind.
    let mut wrong = 0u64;
    let start = arch::timer::now();
    for b in WINDOW_START..WINDOW_START + count {
        clobber(&disk);
        if disk.blk(blk::READ, b) != 0 {
            return Verdict::Failed("the server refused a read inside the namespace");
        }
        if !is_stamped(&disk, b) {
            wrong += 1;
        }
    }
    let read = arch::timer::now().wrapping_sub(start);
    print_phase("read", count, read);
    println!(
        "disk-throughput: verified    {} of {count} blocks came back stamped with their own number \
         (window {}..{} in 4096-byte blocks)",
        count - wrong,
        WINDOW_START,
        WINDOW_START + count,
    );
    if wrong != 0 {
        return Verdict::Failed("blocks came back from the wrong place or not at all");
    }

    let (read, write) = (
        rate(count * BLOCK_SIZE as u64, read),
        rate(count * BLOCK_SIZE as u64, write),
    );
    if confined {
        Verdict::ConfinedAtRate {
            read,
            write,
            unit: disk.scope,
        }
    } else {
        Verdict::Unconfined { read, write }
    }
}

/// Print preflight 1 and say whether it passed.
fn print_scope(rid: u32, scope: crate::iommu::Scope) -> bool {
    let pass = scope.is_confining();
    println!(
        "disk-throughput: preflight 1/2 dmar scope : {}  nvme {:02x}:{:02x}.{} (rid {rid:#06x}): {scope}",
        if pass { "PASS" } else { "FAIL" },
        rid >> 8,
        (rid >> 3) & 0x1f,
        rid & 7,
    );
    pass
}

fn print_phase(what: &str, count: u64, ticks: u64) {
    println!(
        "disk-throughput: {what:<11} {count} x 4096 B in {} us = {} B/s ({} ns per block)",
        micros(ticks),
        rate(count * BLOCK_SIZE as u64, ticks),
        nanos(ticks) / count.max(1),
    );
}

const STAMP_WORD: usize = 8;

fn stamp(disk: &Wiring, block: u64) {
    // SAFETY: as in `measure`; the server is not running while this thread holds the buffer.
    let buf = unsafe { disk.transfer_block() };
    buf[..STAMP_WORD].copy_from_slice(&block.to_le_bytes());
    buf[BLOCK_SIZE - STAMP_WORD..].copy_from_slice(&(!block).to_le_bytes());
}

fn clobber(disk: &Wiring) {
    // SAFETY: as above.
    let buf = unsafe { disk.transfer_block() };
    buf[..STAMP_WORD].fill(0xaa);
    buf[BLOCK_SIZE - STAMP_WORD..].fill(0xaa);
}

fn is_stamped(disk: &Wiring, block: u64) -> bool {
    // SAFETY: as above.
    let buf = unsafe { disk.transfer_block() };
    buf[..STAMP_WORD] == block.to_le_bytes()[..]
        && buf[BLOCK_SIZE - STAMP_WORD..] == (!block).to_le_bytes()[..]
        && buf[STAMP_WORD] == 0x5a
}

fn micros(ticks: u64) -> u64 {
    ticks.saturating_mul(1_000_000) / arch::timer::frequency()
}

fn nanos(ticks: u64) -> u64 {
    // u128 because a 2.7 GHz counter over a minute already overflows `ticks * 1e9` in a u64.
    ((ticks as u128 * 1_000_000_000) / arch::timer::frequency() as u128) as u64
}

/// Bytes per second over `ticks`, or zero for a span too short to measure.
fn rate(bytes: u64, ticks: u64) -> u64 {
    if ticks == 0 {
        return 0;
    }
    ((bytes as u128 * arch::timer::frequency() as u128) / ticks as u128) as u64
}
