//! **radon's SD/MMC block server** (milestone 53 (the board's own peripherals: network and storage
//! on real silicon)): `filesystem_protocol::blk` over the JH7110's `DesignWare` Mobile Storage
//! Host Controller, from EL0. Name provisional; it shares its crate's name, as
//! `non_volatile_memory_express` does under §154 (the acronym test is whether the phrase is
//! spoken, applied recursively).
//!
//! What this process holds, all of it given at spawn by `kernel/src/user/
//! designware_mobile_storage_service.rs`, and nothing more:
//!
//! - one page of the controller, device-typed, at `serve::REGISTER_VA`, with the controller's
//!   clocks already running (the kernel ungates them; the clock generator is shared, so it stays in
//!   the kernel, the split of DECISIONS §86 (whether an NVMe driver can leave the kernel));
//! - the transfer region it shares with its one client, at `serve::TRANSFER_VA`;
//! - the request endpoint (slot 0) and the readiness endpoint (slot 1);
//! - three words: the window of the card it serves, and the slot's clock, width and FIFO depth.
//!
//! The whole driver is `designware_mobile_storage`, the same crate the kernel's bench boot runs;
//! this file is its window over the mapped page and its serve loop. The CPU moves every byte
//! through the FIFO, so this server does no DMA at all.
//!
//! # The confinement, stated plainly
//!
//! The page this process holds also holds the controller's DMA registers (`BMOD`, `DBADDR`). The
//! JH7110 has no IOMMU, so a server that enabled the IDMAC could aim the controller anywhere in
//! memory. It is as confined as its own code, the position milestone 261 (the NVMe driver leaves
//! the kernel) takes for a machine with no IOMMU, and the network half's note states for its
//! controller. What the split keeps from it is everything else: no other device, no clock
//! generator, and no card outside its window except through that one register page.
//!
//! Name: provisional (milestone 53's storage lane, 2026-10-06 UTC). The program shares its crate's
//! name, the pairing `non_volatile_memory_express` has; an architect renaming the crate renames
//! this with it.
//!
//! # BUGS
//!
//! - **Never run on silicon.** `PROVEN_ON_SILICON` in the kernel service keeps the booted system
//!   from starting it until a bench boot has.
//! - **One request at a time**, polled, one `CMD18` or `CMD25` per request.
//! - **`FLUSH` asks the card for its status** and counts; an SD card at default speed has no cache
//!   this driver turned on, so there is nothing else to flush.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68 (code-quality gates: one
// lint policy)'s ratchet tracks (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`, opt-out rather than opt-in)):
// each `[[bin]]` is its own crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use designware_mobile_storage::host::{Host, Registers};
use designware_mobile_storage::sd::{self, Card};
use designware_mobile_storage::serve::{Handoff, REGISTER_VA, TRANSFER_VA};
use designware_mobile_storage::{command as c, regs};
use filesystem_protocol::blk;
use user_mode_runtime::{exit, receive_request, reply, send};

/// The request endpoint.
const REQ: u64 = 0;
/// The readiness endpoint, sent on once.
const READY: u64 = 1;

/// Readiness words that are not `fixture::READY`: which step failed.
const STEP_BAD_HANDOFF: u64 = 1;
const STEP_CONTROLLER: u64 = 2;
const STEP_IDENTIFY: u64 = 3;
const STEP_WINDOW: u64 = 4;
const STEP_FIRST_READ: u64 = 5;

/// `EIO`, `EINVAL`, `EROFS`.
const EIO: i32 = 5;
const EINVAL: i32 = 22;
const EROFS: i32 = 30;

/// The controller's first page, as the kernel mapped it.
struct Page;

impl Registers for Page {
    fn read(&mut self, offset: u32) -> u32 {
        debug_assert!(offset < regs::WINDOW_USED);
        // SAFETY: the spawner maps one device-typed page of the controller at REGISTER_VA before
        // `_start` runs, and every offset the crate passes is a word-aligned register in it.
        unsafe { core::ptr::read_volatile((REGISTER_VA + u64::from(offset)) as *const u32) }
    }

    fn write(&mut self, offset: u32, value: u32) {
        debug_assert!(offset < regs::WINDOW_USED);
        // SAFETY: as `read`.
        unsafe {
            core::ptr::write_volatile((REGISTER_VA + u64::from(offset)) as *mut u32, value);
        }
    }

    fn now_us(&mut self) -> u64 {
        user_mode_runtime::monotonic_nanos() / 1000
    }
}

fn transfer() -> &'static mut [u8] {
    // SAFETY: the spawner maps `blk::TRANSFER_BLOCKS` pages read/write at TRANSFER_VA before
    // `_start` runs; this process is single-threaded, so this is the only reference at a time.
    unsafe { core::slice::from_raw_parts_mut(TRANSFER_VA as *mut u8, blk::TRANSFER_MAX) }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(arg0: u64, arg1: u64, arg2: u64) -> ! {
    let Some(h) = Handoff::unpack([arg0, arg1, arg2]) else {
        send(READY, STEP_BAD_HANDOFF, arg0, 0);
        exit();
    };
    let Ok((mut host, _)) = Host::new(Page, h.ciu_hz, Some(u32::from(h.fifo_depth))) else {
        send(READY, STEP_CONTROLLER, 0, 0);
        exit();
    };
    let Ok(card) = sd::identify(&mut host, h.bus_width) else {
        send(READY, STEP_IDENTIFY, 0, 0);
        exit();
    };
    if h.window.first.saturating_add(h.window.sectors) > card.blocks {
        send(READY, STEP_WINDOW, card.blocks, 0);
        exit();
    }
    // Read the window's first block before saying ready, `non_volatile_memory_express`'s
    // discipline: ready means a client that asks will be answered.
    let first = transfer_blocks(&mut host, &card, &h, 0, 1, false);
    let report = if first >= 0 {
        filesystem_protocol::fixture::READY
    } else {
        STEP_FIRST_READ
    };
    send(
        READY,
        report,
        h.window.sectors * sd::BLOCK as u64,
        card.blocks,
    );
    serve(host, card, h)
}

/// Move `count` blocks from window block `block` through the transfer region. The `blk` answer:
/// the block count, or a negated errno.
fn transfer_blocks(
    host: &mut Host<Page>,
    card: &Card,
    h: &Handoff,
    block: u64,
    count: usize,
    write: bool,
) -> i64 {
    let Some(lba) = h.window.translate(block, count as u64) else {
        return filesystem_protocol::reply_err(EINVAL);
    };
    if write && !h.window.writable {
        return filesystem_protocol::reply_err(EROFS);
    }
    let buf = &mut transfer()[..count * blk::BLOCK_SIZE];
    let r = if write {
        sd::write_blocks(host, card, lba, buf)
    } else {
        sd::read_blocks(host, card, lba, buf)
    };
    match r {
        Ok(()) => count as i64,
        Err(_) => filesystem_protocol::reply_err(EIO),
    }
}

/// The serve loop: one endpoint, one wait point, forever.
fn serve(mut host: Host<Page>, card: Card, h: Handoff) -> ! {
    let mut flushes = 0i64;
    loop {
        let req = receive_request(REQ);
        let (w0, block) = (req.w0, req.w1);
        let Some(reply_cap) = req.delivered.into_reply() else {
            continue;
        };
        // Clamped, as every block server here clamps: the packing could spell more.
        let count = blk::req_blocks(w0).min(blk::TRANSFER_BLOCKS);
        let r0 = match filesystem_protocol::operation(w0) {
            blk::READ => transfer_blocks(&mut host, &card, &h, block, count, false),
            blk::WRITE => transfer_blocks(&mut host, &card, &h, block, count, true),
            blk::SIZE => (h.window.sectors * sd::BLOCK as u64) as i64,
            blk::FLUSH => {
                let status = host.command(&c::Command::new(
                    c::SEND_STATUS,
                    u32::from(card.rca) << 16,
                    c::Response::R1,
                ));
                match status {
                    Ok(r) if r[0] & designware_mobile_storage::card::R1_ERRORS == 0 => {
                        flushes += 1;
                        flushes
                    }
                    _ => filesystem_protocol::reply_err(EIO),
                }
            }
            _ => filesystem_protocol::reply_err(EINVAL),
        };
        reply(reply_cap, r0 as u64, 0);
    }
}

user_mode_runtime::panic_handler!();
