#![cfg_attr(not(test), no_std)]
//! **The Synopsys `DesignWare` Mobile Storage Host Controller, as pure logic**: radon's microSD
//! slot and eMMC socket (milestone 53 (the board's own peripherals: network and storage on real
//! silicon); notes/designware-mobile-storage.md).
//!
//! The `StarFive` JH7110 has two of these, at `0x1601_0000` (the VisionFive 2's eMMC socket) and
//! `0x1602_0000` (its microSD slot, the card radon boots from). [`jh7110`] has the evidence for
//! what the part is. This crate is the whole driver behind one small trait,
//! [`host::Registers`]: a window to read and write, and a clock. The kernel's
//! `kernel/src/designware_mobile_storage.rs` implements that trait over a mapped device window, and
//! that is all it does. Nothing here dereferences a pointer, which is what lets every sequence be
//! tested on a host against [`sim`] when the device exists on one desk and in no emulator.
//!
//! | module | what it holds |
//! |---|---|
//! | [`regs`] | the register map, and what `VERID` and `HCON` say about the FIFO |
//! | [`command`] | SD and eMMC commands, and the `CMD` word that sends each |
//! | [`host`] | reset, clock, bus width, and one command with a polled data phase |
//! | [`card`] | the OCR, CID, CSD, R1 and `EXT_CSD`, decoded |
//! | [`sd`] | identification, and block reads and writes |
//! | [`partition`] | the MBR, and the only sectors a write test may touch |
//! | [`bench`] | the bench step's read-only probe and scratch write test |
//! | [`jh7110`] | the device-tree query and the board's constants |
//!
//! # Examples
//!
//! The card clock never runs faster than asked, and a divider the field cannot hold is refused
//! rather than written as zero (the fastest setting there is):
//!
//! ```
//! use designware_mobile_storage::host::{card_clock, clock_divider};
//!
//! assert!(card_clock(50_000_000, clock_divider(50_000_000, 400_000).unwrap()) <= 400_000);
//! assert_eq!(clock_divider(50_000_000, 50_000), None);
//! ```
//!
//! A write test on radon's boot card may only land before the first partition:
//!
//! ```
//! use designware_mobile_storage::partition::{Entry, Mbr};
//!
//! let fat = Entry { kind: 0x0b, start: 2048, sectors: 1 << 20 };
//! let mbr = Mbr { entries: [fat, Entry::default(), Entry::default(), Entry::default()] };
//! assert_eq!(mbr.scratch(8), Some((2040, 8)));
//! ```
//!
//! # Where the code comes from, and the license that comes with it
//!
//! Read, not recalled, on 2026-10-06 (UTC). **OpenBSD's `dwmmc` driver is the source this crate
//! adapts**: `sys/dev/fdt/dwmmc.c` revision 1.33 (2026-03-11), which matches both
//! `starfive,jh7110-mmc` and `snps,dw-mshc` and so drives exactly this part on exactly this board.
//! The register map, the reset, clock and bus-width sequences, the command flags and the polled
//! FIFO transfer are its, adapted; the identification order follows its `sdmmc_mem.c`. It is ISC
//! licensed and carries this notice, which the license requires be kept:
//!
//! ```text
//! Copyright (c) 2017 Mark Kettenis
//!
//! Permission to use, copy, modify, and distribute this software for any
//! purpose with or without fee is hereby granted, provided that the above
//! copyright notice and this permission notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
//! WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
//! MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
//! ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//! WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
//! ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
//! OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//! ```
//!
//! The crates.io candidate, `starfive-jh7110-dwmmc` 0.1.8 (Apache-2.0, `rcore-os/tgoskits`), was
//! read and **not taken**, and the reason is recorded in notes/designware-mobile-storage.md: it is
//! a 333-line wrapper over `dwmmc-host` 0.4.2 (5,253 lines) and three more crates from the same
//! tree (`sdmmc-host`, `sdmmc-protocol`, `dma-api`), sixteen releases since August, with an
//! IDMAC-only data path and volatile accesses inside the driver core, where this tree's split puts
//! them outside it. Its JH7110 constants (a 50 MHz reference, a 32-word, 32-bit FIFO) agree with
//! the device trees and were used as a third cross-check. Linux's `dw_mmc.c` and
//! `dw_mmc-starfive.c`, U-Boot's `dw_mmc.c` and both device trees are GPL and were read only for
//! facts about the hardware (where the FIFO is by release, which clocks and resets the part takes),
//! each named where it is used. No GPL code was copied.
//!
//! # BUGS
//!
//! - **Nothing here has touched the device.** Every test runs against [`sim`], which models the
//!   contract as the databook and OpenBSD describe it. The bench step in
//!   notes/designware-mobile-storage.md is what makes it a measurement.
//! - **The CPU moves every byte.** The IDMAC (the controller's own DMA) is not used; see
//!   [`host`]'s header for why the first silicon read goes through the FIFO. Throughput is
//!   whatever polled MMIO gives, and the bench step prints it.
//! - **Default speed only**: 25 MHz, 4-bit on SD, 1-bit on eMMC. No high-speed switch, no UHS-I,
//!   no HS200 and so none of the JH7110's sample-phase tuning (`dw_mmc-starfive.c`'s
//!   `execute_tuning`), no 8-bit eMMC bus.
//! - **Polled, not interrupt-driven.** `INTMASK` stays zero. The vendor tree names no interrupt for
//!   either controller; mainline names 74 and 75.
//! - **A 64-bit FIFO is refused.** OpenBSD handles one; whether radon's is 32-bit is the first
//!   thing the bench step reads (`HCON`).
//! - **No card detect or write-protect handling.** The probe prints `CDETECT` and proceeds; radon's
//!   slot has a card in it or there is nothing to boot from.
//!
//! Name: provisional (milestone 53's storage lane, 2026-10-06 UTC). "`DesignWare` Mobile Storage"
//! is the Synopsys databook's name for the part (`DW_mshc`, the `snps,dw-mshc` compatible read
//! aloud), spelled out as §154 (the acronym test is whether the phrase is spoken, applied
//! recursively) asks and as the same milestone's `designware_ethernet` spells its own. The
//! alternatives an architect may prefer are `designware_mmc`, after Linux's `dw_mmc`, or a name for
//! what the crate does (`sd_card`), which would hide that the eMMC socket is driven by it too.

pub mod bench;
pub mod card;
pub mod command;
pub mod host;
pub mod jh7110;
pub mod partition;
pub mod regs;
pub mod sd;

#[cfg(test)]
mod sim;

#[cfg(test)]
mod tests {
    use crate::bench::{self, WriteVerdict};
    use crate::host::{Error, Host};
    use crate::sd::{self, BLOCK, InitError, IoError, Kind};
    use crate::sim::{CardKind, Sim};

    fn host(kind: CardKind, blocks: usize) -> Host<Sim> {
        let (h, id) = Host::new(Sim::new(kind, blocks, 32), 50_000_000, Some(32)).unwrap();
        assert_eq!(id.verid, crate::sim::VERID);
        h
    }

    fn mbr_card(blocks: usize, first: u32) -> Sim {
        let mut s = Sim::new(CardKind::SdHighCapacity, blocks, 32);
        let m = &mut s.storage[0];
        m[446 + 4] = 0x0b;
        m[446 + 8..446 + 12].copy_from_slice(&first.to_le_bytes());
        m[446 + 12..446 + 16].copy_from_slice(&(blocks as u32 - first).to_le_bytes());
        m[510] = 0x55;
        m[511] = 0xaa;
        let fat = &mut s.storage[first as usize];
        fat[82..90].copy_from_slice(b"FAT32   ");
        fat[510] = 0x55;
        fat[511] = 0xaa;
        s
    }

    #[test]
    fn an_sdhc_card_is_identified_selected_and_widened_in_the_spec_order() {
        let mut h = host(CardKind::SdHighCapacity, 8192);
        let card = sd::identify(&mut h, 4).unwrap();
        assert_eq!(card.kind, Kind::SdHighCapacity);
        assert!(card.block_addressed);
        assert_eq!(card.blocks, 8192);
        assert_eq!((card.bus_width, card.clock_hz), (4, 25_000_000));
        assert_eq!(&card.cid.name[..5], b"NIFE1");
        let sim = h.into_registers();
        let order: Vec<u8> = sim.commands.iter().map(|c| c.0).collect();
        // CMD0, CMD8, then ACMD41 until ready (three busy polls), CID, RCA, CSD, select, ACMD6,
        // block length.
        assert_eq!(
            order,
            [0, 8, 55, 41, 55, 41, 55, 41, 55, 41, 2, 3, 9, 7, 55, 6, 16]
        );
        assert!(
            !sim.clockless_command,
            "a command went out with the card clock off"
        );
    }

    #[test]
    fn an_old_sd_card_is_byte_addressed_and_an_emmc_is_told_apart_by_its_silence() {
        let mut h = host(CardKind::SdV1, 4096);
        let card = sd::identify(&mut h, 4).unwrap();
        assert_eq!(card.kind, Kind::SdV1);
        assert!(!card.block_addressed);
        assert_eq!(card.address(3), 3 * 512);

        let mut h = host(CardKind::Mmc, 16_384);
        let card = sd::identify(&mut h, 8).unwrap();
        assert_eq!(card.kind, Kind::Mmc);
        assert_eq!(card.rca, sd::MMC_RCA);
        assert!(card.block_addressed);
        // The size came from EXT_CSD, not the CSD's placeholder.
        assert_eq!(card.blocks, 16_384);
        assert_eq!(card.bus_width, 1);
    }

    #[test]
    fn an_empty_slot_is_no_card_not_a_hang() {
        let mut h = host(CardKind::Empty, 1024);
        assert!(matches!(
            sd::identify(&mut h, 4),
            Err(InitError::NoCard(Error::NoResponse(1)))
        ));
    }

    #[test]
    fn blocks_written_read_back_and_multi_block_transfers_stop_by_themselves() {
        let mut h = host(CardKind::SdHighCapacity, 8192);
        let card = sd::identify(&mut h, 4).unwrap();
        let data: Vec<u8> = (0..8 * BLOCK).map(|i| (i * 7 % 251) as u8).collect();
        sd::write_blocks(&mut h, &card, 100, &data).unwrap();
        let mut back = vec![0u8; 8 * BLOCK];
        sd::read_blocks(&mut h, &card, 100, &mut back).unwrap();
        assert_eq!(back, data);
        let mut one = [0u8; BLOCK];
        sd::read_blocks(&mut h, &card, 103, &mut one).unwrap();
        assert_eq!(one[..], data[3 * BLOCK..4 * BLOCK]);
        let sim = h.into_registers();
        assert_eq!(sim.written, (100..108).collect::<Vec<u64>>());
        // CMD25 and CMD18 each followed by the controller's own CMD12; CMD17 not.
        let tail: Vec<u8> = sim.commands.iter().rev().take(6).map(|c| c.0).collect();
        assert_eq!(tail, [17, 12, 18, 13, 12, 25]);
    }

    #[test]
    fn a_transfer_that_does_not_fit_the_card_or_the_buffer_is_refused_before_anything_is_sent() {
        let mut h = host(CardKind::SdHighCapacity, 8192);
        let card = sd::identify(&mut h, 4).unwrap();
        let sent = h.registers().commands.len();
        let mut buf = [0u8; BLOCK];
        assert_eq!(
            sd::read_blocks(&mut h, &card, 8192, &mut buf),
            Err(IoError::Range)
        );
        assert_eq!(
            sd::read_blocks(&mut h, &card, 0, &mut buf[..100]),
            Err(IoError::Length)
        );
        assert_eq!(
            sd::write_blocks(&mut h, &card, u64::MAX, &buf),
            Err(IoError::Range)
        );
        assert_eq!(h.registers().commands.len(), sent);
    }

    #[test]
    fn a_controller_before_2_40a_reads_through_its_fifo_at_0x100_and_gets_no_threshold_write() {
        let sim = Sim::new(CardKind::SdHighCapacity, 8192, 32).with_verid(0x5342_230a);
        let (mut h, id) = Host::new(sim, 50_000_000, Some(32)).unwrap();
        assert_eq!(crate::regs::fifo_offset(id.verid), 0x100);
        let card = sd::identify(&mut h, 4).unwrap();
        let data: Vec<u8> = (0..2 * BLOCK).map(|i| (i % 253) as u8).collect();
        sd::write_blocks(&mut h, &card, 7, &data).unwrap();
        let mut back = vec![0u8; 2 * BLOCK];
        // A threshold write to 0x100 here would land in the FIFO and shift every word read.
        sd::read_blocks(&mut h, &card, 7, &mut back).unwrap();
        assert_eq!(back, data);
        assert!(!h.into_registers().stray_fifo_write);
    }

    #[test]
    fn a_read_whose_tail_is_below_the_watermark_is_drained_on_transfer_over() {
        // 24 words deep: a 128-word block arrives as five fills of 24 and a tail of 8, below the
        // receive watermark of 11, so the tail raises DTO and never RXDR.
        let mut sim = Sim::new(CardKind::SdHighCapacity, 8192, 24);
        sim.strict_watermark = true;
        sim.storage[5] = core::array::from_fn(|i| (i % 249) as u8);
        let (mut h, _) = Host::new(sim, 50_000_000, Some(24)).unwrap();
        let card = sd::identify(&mut h, 4).unwrap();
        let mut one = [0u8; BLOCK];
        sd::read_blocks(&mut h, &card, 5, &mut one).unwrap();
        assert_eq!(one, h.registers().storage[5]);
    }

    #[test]
    fn a_data_crc_error_is_reported_with_its_command() {
        let mut h = host(CardKind::SdHighCapacity, 8192);
        let card = sd::identify(&mut h, 4).unwrap();
        h.registers().fail_next_read_crc = true;
        let mut buf = [0u8; BLOCK];
        assert!(matches!(
            sd::read_blocks(&mut h, &card, 0, &mut buf),
            Err(IoError::Host(Error::Data(17, rint))) if rint & crate::regs::INT_DCRC != 0
        ));
    }

    #[test]
    fn the_read_only_probe_reads_and_never_writes() {
        let (mut h, _) = Host::new(mbr_card(8192, 2048), 50_000_000, Some(32)).unwrap();
        let mut buf = vec![0u8; 128 * BLOCK];
        let r = bench::read_only(&mut h, 4, &mut buf, 1024).unwrap();
        assert_eq!(r.mbr.unwrap().first_start(), Some(2048));
        assert!(r.partition_boot_signature);
        assert_eq!(&r.fs_type, b"FAT32   ");
        assert_eq!(r.timed_blocks, 1024);
        let sim = h.into_registers();
        assert!(sim.written.is_empty());
        assert!(
            !sim.commands.iter().any(|c| matches!(c.0, 24 | 25)),
            "the read-only probe sent a write command"
        );
    }

    #[test]
    fn the_write_test_touches_only_the_gap_and_puts_back_what_was_there() {
        let mut sim = mbr_card(8192, 2048);
        // Something already in the gap: the test must restore it, not zero it.
        sim.storage[2043][0] = 0x42;
        let before = sim.storage.clone();
        let (mut h, _) = Host::new(sim, 50_000_000, Some(32)).unwrap();
        let mut buf = vec![0u8; 8 * BLOCK];
        let r = bench::read_only(&mut h, 4, &mut buf, 0).unwrap();
        let (mut a, mut b) = (vec![0u8; 8 * BLOCK], vec![0u8; 8 * BLOCK]);
        let v = bench::scratch_write(&mut h, &r.card, r.mbr.as_ref(), &mut a, &mut b);
        assert_eq!(v, WriteVerdict::Verified(2040, 8, false));
        let sim = h.into_registers();
        assert!(sim.written.iter().all(|&lba| (2040..2048).contains(&lba)));
        assert_eq!(sim.written.len(), 16);
        assert_eq!(sim.storage, before);
    }

    #[test]
    fn a_card_with_no_gap_is_refused_and_not_written() {
        let (mut h, _) = Host::new(mbr_card(8192, 1), 50_000_000, Some(32)).unwrap();
        let mut buf = vec![0u8; 8 * BLOCK];
        let r = bench::read_only(&mut h, 4, &mut buf, 0).unwrap();
        let (mut a, mut b) = (vec![0u8; 8 * BLOCK], vec![0u8; 8 * BLOCK]);
        let v = bench::scratch_write(&mut h, &r.card, r.mbr.as_ref(), &mut a, &mut b);
        assert_eq!(v, WriteVerdict::Refused);
        assert!(h.into_registers().written.is_empty());
    }

    #[test]
    fn a_sixteen_bit_fifo_is_refused_at_the_door() {
        // A controller whose HCON says 16-bit, over the sim.
        struct Narrow(Sim);
        impl crate::host::Registers for Narrow {
            fn read(&mut self, o: u32) -> u32 {
                if o == crate::regs::HCON {
                    0
                } else {
                    self.0.read(o)
                }
            }
            fn write(&mut self, o: u32, v: u32) {
                self.0.write(o, v);
            }
            fn now_us(&mut self) -> u64 {
                self.0.now_us()
            }
        }
        let narrow = Narrow(Sim::new(CardKind::SdHighCapacity, 1024, 32));
        assert!(matches!(
            Host::new(narrow, 50_000_000, None),
            Err(Error::FifoWidth(0))
        ));
    }

    #[test]
    fn without_a_tree_depth_the_fifo_depth_is_read_from_its_reset_watermark() {
        let (h, _) = Host::new(
            Sim::new(CardKind::SdHighCapacity, 1024, 16),
            50_000_000,
            None,
        )
        .unwrap();
        assert_eq!(h.fifo_depth(), 16);
    }
}

#[cfg(kani)]
mod proofs {
    use crate::host::fifo_step;
    use crate::partition::{Entry, Mbr};
    use crate::sd::in_range;

    /// No partition table makes the write test's range touch a partition or sector 0.
    ///
    /// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.the_scratch_range_never_touches_a_partition_or_sector_zero.patch`
    #[kani::proof]
    #[kani::unwind(5)]
    fn the_scratch_range_never_touches_a_partition_or_sector_zero() {
        let entry = || Entry {
            kind: kani::any(),
            start: kani::any(),
            sectors: kani::any(),
        };
        let mbr = Mbr {
            entries: [entry(), entry(), entry(), entry()],
        };
        let want: u32 = kani::any();
        if let Some((first, count)) = mbr.scratch(want) {
            assert!(first >= 1);
            assert!(count >= 1 && count <= want);
            let k: u32 = kani::any();
            kani::assume(k < count);
            let sector = u64::from(first) + u64::from(k);
            for e in &mbr.entries {
                assert!(!e.contains(sector));
            }
        }
    }

    /// A FIFO step never moves more than is owed, and a write step never more than the FIFO has
    /// room for, whatever `STATUS` says.
    ///
    /// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.a_fifo_step_never_overruns_the_fifo_or_the_buffer.patch`
    #[kani::proof]
    fn a_fifo_step_never_overruns_the_fifo_or_the_buffer() {
        let status: u32 = kani::any();
        let depth: u32 = kani::any();
        kani::assume(depth >= 1 && depth <= 4096);
        let write: bool = kani::any();
        let remaining: u32 = kani::any();
        let n = fifo_step(status, depth, write, remaining);
        assert!(n <= remaining);
        assert!(n <= depth * 4);
        let filled = crate::regs::fifo_count(status);
        if write && filled <= depth {
            assert!(n <= (depth - filled) * 4);
        }
    }

    /// A range the driver admits is inside the card and inside what the command's 32-bit address
    /// can name.
    ///
    /// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.an_admitted_range_is_inside_the_card_and_the_address.patch`
    #[kani::proof]
    fn an_admitted_range_is_inside_the_card_and_the_address() {
        let blocks: u64 = kani::any();
        let block_addressed: bool = kani::any();
        let lba: u64 = kani::any();
        let count: u64 = kani::any();
        if in_range(blocks, block_addressed, lba, count) {
            assert!(count >= 1);
            assert!(lba + count <= blocks);
            let last = lba + count - 1;
            if block_addressed {
                assert!(last <= u64::from(u32::MAX));
            } else {
                assert!((last + 1) * 512 <= 1 << 32);
            }
        }
    }
}
