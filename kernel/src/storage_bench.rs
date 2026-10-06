//! **Milestone 53 (the board's own peripherals: network and storage on real silicon)'s storage
//! bench boot**, behind the `storage_bench` feature, riscv64 only, name provisional.
//!
//! Nothing emulates radon's SD/MMC controller, so the first time this driver meets the part is a
//! boot on the bench, and the boot has to say everything at once: what the firmware left, what the
//! controller is, what card answered, what the card holds, and how fast it reads. Every line is
//! prefixed `storage-bench:` so a serial capture and a photograph of a screen read the same way.
//!
//! In order, for the microSD slot (slot 1) and then the eMMC socket (slot 0):
//!
//! 1. the slot as the tree describes it;
//! 2. its clocks and reset as the firmware left them, then walked through
//!    `jh7110_clock_and_reset::SDIO_BRING_UP` (every step idempotent);
//! 3. the controller's identity (`VERID`, `HCON`, the FIFO, `CDETECT`) and the five registers the
//!    firmware's driver last wrote, read before anything is written;
//! 4. the read-only probe (`designware_mobile_storage::bench::read_only`): identification, the CID,
//!    the capacity, sector 0's partition table, the first partition's boot sector, and an 8 MiB
//!    sequential read from the start of that partition, timed;
//! 5. **only on the microSD slot and only in a build made with `NIFE_STORAGE_BENCH_WRITE=scratch`**,
//!    the scratch write test (`bench::scratch_write`): up to eight sectors before the first
//!    partition, read, overwritten with a pattern, read back, restored, read back.
//!
//! Then one verdict line about the microSD slot, and a halt.
//!
//! # BUGS
//!
//! - **The client is the kernel.** The driver runs on the boot thread, polled, with the direct
//!   map. The booted system has no block device on radon yet; serving `filesystem_protocol::blk`
//!   from an EL0 process over this driver is the next step, in the 53 block.
//! - **It re-initializes the card U-Boot booted from.** `CMD0` puts the card back to idle, which
//!   is harmless (the next power-on does the same) and means this boot cannot hand the card back
//!   to anything that expected it selected. It halts instead.
//! - **The eMMC socket is probed read-only, always.** Whether radon has a module fitted is a bench
//!   fact this boot reports rather than assumes, and nothing here writes to one.

use designware_mobile_storage::bench::{self, ReadFailure, ReadReport, WriteVerdict};
use designware_mobile_storage::host::{Error, Host, Identity};
use designware_mobile_storage::jh7110::Slot;
use designware_mobile_storage::regs;
use designware_mobile_storage::sd::{BLOCK, InitError, IoError, Kind};

use crate::designware_mobile_storage::Window;
use crate::{arch, println};

/// Whether this build carries the scratch write test: `NIFE_STORAGE_BENCH_WRITE=scratch` at build
/// time, and nothing else, so a card is never written by a build that did not ask in so many words.
const WRITE_TEST: bool = match option_env!("NIFE_STORAGE_BENCH_WRITE") {
    Some(v) => is_scratch(v.as_bytes()),
    None => false,
};

/// `v == b"scratch"`, in a `const` context, where string comparison is not yet allowed.
const fn is_scratch(v: &[u8]) -> bool {
    matches!(v, b"scratch")
}

/// The timed read's length: 8 MiB, long enough that a polled read at default speed takes about a
/// second, short enough that a slow card does not stall the boot.
const TIMED_BLOCKS: u64 = 16_384;
/// Pages in the timed read's buffer: 64 KiB, 128 blocks per `CMD18`.
const READ_PAGES: usize = 16;

/// Run the probe on both slots, print the verdict, and never come back.
pub fn run() -> ! {
    println!();
    println!(
        "storage-bench: milestone 53 storage bench boot, {} build, {} Hz counter. {}",
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "release"
        },
        arch::timer::frequency(),
        if WRITE_TEST {
            "WRITE TEST BUILT IN: the microSD card's pre-partition gap only."
        } else {
            "Read-only build: nothing on any card is written."
        },
    );
    let verdict = match crate::memory::jh7110_storage() {
        None => Outcome::NotDescribed,
        Some(slots) => {
            let sd = slots[1].map_or(Outcome::NotDescribed, |s| probe(&s, WRITE_TEST));
            if let Some(s) = slots[0] {
                let _ = probe(&s, false);
            } else {
                println!("storage-bench: slot 0 (eMMC socket): not in the device tree");
            }
            sd
        }
    };
    print_verdict(&verdict);
    println!("storage-bench: done, halting.");
    arch::halt(arch::HaltReason::measurement_boot());
}

/// How the microSD slot's probe ended: decided where each step ends, so the verdict line cannot
/// claim more than the step that produced it.
#[derive(Debug)]
enum Outcome {
    /// No JH7110 SD/MMC controller in the tree, or no microSD one.
    NotDescribed,
    /// No SYS clock window was recorded, so nothing was touched.
    NoClockWindow,
    /// A clock did not read back enabled, or the reset did not release.
    ClocksDown,
    /// The controller refused (its FIFO width).
    Controller(Error),
    /// The kernel had no pages for a buffer.
    NoMemory,
    /// Identification stopped: no card, or a card that did not finish.
    NoCard(InitError),
    /// A read failed: the block and the error.
    ReadFailed(u64, IoError),
    /// Read-only, and every read succeeded: the timed rate in KiB/s.
    ReadOk(u64),
    /// Read, then the scratch write test: the rate and the test's verdict.
    Written(u64, WriteVerdict),
}

fn print_verdict(o: &Outcome) {
    match o {
        Outcome::ReadOk(kib) => println!("storage-bench: verdict READ-OK {kib} KiB/s"),
        Outcome::Written(kib, WriteVerdict::Verified(first, count, blank)) => println!(
            "storage-bench: verdict READ-OK {kib} KiB/s, WRITE-VERIFIED sectors {first}..{} (were {}), restored",
            first + count,
            if *blank { "zero" } else { "not zero" },
        ),
        Outcome::Written(kib, WriteVerdict::Refused) => println!(
            "storage-bench: verdict READ-OK {kib} KiB/s, WRITE-REFUSED: no gap before the first partition"
        ),
        Outcome::Written(_, v) => println!("storage-bench: verdict FAILED: write test {v:?}"),
        Outcome::NotDescribed => {
            println!("storage-bench: verdict SKIPPED: the tree describes no microSD controller");
        }
        Outcome::NoCard(e) => println!("storage-bench: verdict NO-CARD: {e:?}"),
        Outcome::Controller(e) => println!("storage-bench: verdict FAILED: controller {e:?}"),
        Outcome::ReadFailed(lba, e) => {
            println!("storage-bench: verdict FAILED: read of block {lba}: {e:?}");
        }
        other => println!("storage-bench: verdict FAILED: {other:?}"),
    }
}

/// Print one line about slot `i`, prefixed the same way every time.
macro_rules! say {
    ($slot:expr, $($arg:tt)*) => {
        println!(
            "storage-bench: slot {} ({}): {}",
            $slot.index,
            $slot.role(),
            format_args!($($arg)*)
        )
    };
}

fn probe(slot: &Slot, write: bool) -> Outcome {
    say!(
        slot,
        "{:#x}+{:#x}, compatible {}, status {}, bus-width {}, fifo-depth {:?}, ciu {} Hz, interrupt {:?}",
        slot.base,
        slot.size,
        core::str::from_utf8(slot.compatible).unwrap_or("?"),
        if slot.status_okay { "okay" } else { "disabled" },
        slot.bus_width,
        slot.fifo_depth,
        slot.ciu_hz,
        slot.interrupt,
    );

    let Some((sys, _)) = crate::memory::jh7110_pmic_bus() else {
        say!(
            slot,
            "no SYS clock window recorded; not touching the controller"
        );
        return Outcome::NoClockWindow;
    };
    let plan = jh7110_clock_and_reset::SDIO_BRING_UP[usize::from(slot.index)];
    // SAFETY: `memory::jh7110_pmic_bus` is set only on a JH7110, and `mmu::init` step 6c mapped
    // exactly this window device-typed. The plan and the domain are the same crate's, so every
    // identifier is in range; every step is idempotent on a controller already up.
    let report = unsafe {
        crate::drivers::jh7110_clock_and_reset::bring_up(
            crate::arch::mmu::phys_to_virt(sys.base) as usize,
            &jh7110_clock_and_reset::SYS,
            plan,
        )
    };
    say!(
        slot,
        "clocks before {:#010x} {:#010x} after {:#010x} {:#010x} (dividers {} {}), reset assert {:#x}->{:#x} status {:#x}, released {}, already up {}",
        report.clock_before[0],
        report.clock_before[1],
        report.clock_after[0],
        report.clock_after[1],
        jh7110_clock_and_reset::clock_divider(report.clock_after[0]),
        jh7110_clock_and_reset::clock_divider(report.clock_after[1]),
        report.reset_assert_before,
        report.reset_assert_after,
        report.reset_status_after,
        report.released,
        report.was_already_up(),
    );
    if !report.has_clocks_running() || !report.released {
        return Outcome::ClocksDown;
    }

    // SAFETY: step 10 of `mmu::init` mapped this slot's first page from the same
    // `memory::jh7110_storage` answer, and this boot thread is the only thing that drives it.
    let window = unsafe { Window::new(slot.base) };
    let (mut host, id) = match Host::new(window, slot.ciu_hz, slot.fifo_depth) {
        Ok(h) => h,
        Err(e) => {
            say!(slot, "controller refused: {e:?}");
            return Outcome::Controller(e);
        }
    };
    print_identity(slot, &id, host.fifo_depth());

    let Some(buf) = pages(READ_PAGES) else {
        return Outcome::NoMemory;
    };
    let r = match bench::read_only(&mut host, slot.bus_width, buf, TIMED_BLOCKS) {
        Err(ReadFailure::Init(e)) => {
            say!(slot, "identification stopped: {e:?}");
            return Outcome::NoCard(e);
        }
        Err(ReadFailure::Read(lba, e)) => {
            say!(slot, "read of block {lba} failed: {e:?}");
            return Outcome::ReadFailed(lba, e);
        }
        Ok(r) => r,
    };
    print_read(slot, &r);
    if !write {
        return Outcome::ReadOk(kib_per_s(&r));
    }
    let (Some(a), Some(b)) = (pages(1), pages(1)) else {
        return Outcome::NoMemory;
    };
    let v = bench::scratch_write(&mut host, &r.card, r.mbr.as_ref(), a, b);
    say!(slot, "scratch write test: {v:?}");
    Outcome::Written(kib_per_s(&r), v)
}

fn print_identity(slot: &Slot, id: &Identity, depth: u32) {
    say!(
        slot,
        "VERID {:#010x} HCON {:#010x} USRID {:#x} (FIFO at {:#x}, {:?} bytes wide, {} words deep, 64-bit DMA {}), CDETECT {:#x}",
        id.verid,
        id.hcon,
        id.usrid,
        regs::fifo_offset(id.verid),
        regs::fifo_width(id.hcon),
        depth,
        regs::dma_64bit(id.hcon),
        id.cdetect,
    );
    say!(
        slot,
        "as the firmware left it: CTRL {:#x} PWREN {:#x} CLKDIV {:#x} CLKENA {:#x} CTYPE {:#x}",
        id.firmware[0],
        id.firmware[1],
        id.firmware[2],
        id.firmware[3],
        id.firmware[4],
    );
}

fn print_read(slot: &Slot, r: &ReadReport) {
    let c = &r.card;
    let name = c
        .cid
        .name
        .map(|b| if b.is_ascii_graphic() { b } else { b'.' });
    say!(
        slot,
        "card {:?}, manufacturer {:#04x} \"{}\" serial {:#010x}, {} blocks ({} MiB), {}-addressed, {}-bit at {} Hz, OCR {:#010x}",
        c.kind,
        c.cid.manufacturer,
        core::str::from_utf8(&name).unwrap_or("?"),
        c.cid.serial,
        c.blocks,
        c.blocks * BLOCK as u64 / (1 << 20),
        if c.block_addressed { "block" } else { "byte" },
        c.bus_width,
        c.clock_hz,
        c.ocr,
    );
    if c.kind == Kind::Mmc {
        say!(
            slot,
            "an eMMC answered; this boot reads it and never writes it"
        );
    }
    match r.mbr {
        None => say!(slot, "sector 0 carries no MBR signature"),
        Some(m) => {
            for (i, e) in m.entries.iter().enumerate().filter(|(_, e)| e.is_used()) {
                say!(
                    slot,
                    "partition {}: type {:#04x}, sectors {}..{}",
                    i + 1,
                    e.kind,
                    e.start,
                    u64::from(e.start) + u64::from(e.sectors),
                );
            }
            say!(
                slot,
                "first partition's boot sector: signature {}, type field \"{}\"",
                if r.partition_boot_signature {
                    "55aa"
                } else {
                    "absent"
                },
                core::str::from_utf8(&r.fs_type).unwrap_or("?"),
            );
            say!(
                slot,
                "scratch range a write test would use: {:?}",
                m.scratch(bench::SCRATCH_SECTORS)
            );
        }
    }
    say!(
        slot,
        "timed read: {} blocks in {} us, {} KiB/s ({} blocks per command, polled FIFO)",
        r.timed_blocks,
        r.timed_us,
        kib_per_s(r),
        READ_PAGES * 4096 / BLOCK,
    );
}

fn kib_per_s(r: &ReadReport) -> u64 {
    (r.timed_blocks * BLOCK as u64 * 1_000_000)
        .checked_div(r.timed_us)
        .unwrap_or(0)
        / 1024
}

/// `count` zeroed, contiguous pages as a byte slice, leaked: this boot halts.
fn pages(count: usize) -> Option<&'static mut [u8]> {
    let frame = crate::memory::alloc_contiguous_zeroed(count)?;
    let va = crate::arch::mmu::phys_to_virt(frame.addr()) as *mut u8;
    // SAFETY: the allocator handed out `count` contiguous pages that nothing else owns, and the
    // direct map covers all of RAM; nothing frees them, because the boot ends in a halt.
    Some(unsafe { core::slice::from_raw_parts_mut(va, count * 4096) })
}
