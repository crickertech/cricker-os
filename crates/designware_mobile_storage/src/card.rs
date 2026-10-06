//! **What a card says about itself**: the OCR, the CID, the CSD, the R1 status word and the eMMC
//! `EXT_CSD`, decoded. Bit positions are the SD Physical Layer Simplified Specification's (version
//! 9.10, sections 5.1 to 5.3 and 4.10.1) and JEDEC JESD84-B51's for eMMC, in the spec's own
//! numbering, which is also Linux's `UNSTUFF_BITS` convention: a long response is held as four
//! words `[RESP0, RESP1, RESP2, RESP3]`, bit 0 of `RESP0` being the register's bit 0 (the always-one
//! end bit, with the CRC7 in bits 7:1).
//!
//! Nothing here can fail on hostile input. A card is a device on the far side of a bus and its
//! registers are data, so every decoder is total and every capacity is computed without overflow.

/// The OCR's power-up-complete bit: clear while the card is still busy initializing.
pub const OCR_BUSY_DONE: u32 = 1 << 31;
/// The OCR's card capacity status (SD) or access mode (eMMC, as bits 30:29 = `0b10`): set on a
/// high-capacity card, which is addressed in 512-byte blocks rather than bytes.
pub const OCR_HIGH_CAPACITY: u32 = 1 << 30;
/// The voltage window this host offers, 3.2 to 3.4 V: OpenBSD's `dwmmc_host_ocr`. The JH7110's
/// card slot is wired for 3.3 V and nothing here switches it.
pub const OCR_VOLTAGE_WINDOW: u32 = (1 << 20) | (1 << 21);
/// `ACMD41`'s argument: high capacity supported, and the window.
pub const SD_OP_COND_ARG: u32 = OCR_HIGH_CAPACITY | OCR_VOLTAGE_WINDOW;
/// eMMC `CMD1`'s argument: sector access mode (bits 30:29 = `0b10`) and 2.7 to 3.6 V plus the
/// 1.70 to 1.95 V bit, the value Linux and U-Boot both send.
pub const MMC_OP_COND_ARG: u32 = 0x40ff_8080;
/// `CMD8`'s argument: 2.7 to 3.6 V (`0x1`) and the check pattern `0xaa`. A card that understands
/// it echoes both.
pub const IF_COND_ARG: u32 = 0x1aa;

/// The R1 bits that report a failure, Linux's error mask: out of range, address, block length,
/// erase sequence and parameter, write-protect violation, lock failure, CRC, illegal command, ECC,
/// controller error, unknown error, CSD overwrite, write-protect erase skip, and the
/// authentication sequence error. `CARD_IS_LOCKED` (bit 25) is a state, not an error, and is not
/// in the mask.
pub const R1_ERRORS: u32 = 0xfdf9_8008;
/// The R1 bit saying the card is ready for data.
pub const R1_READY_FOR_DATA: u32 = 1 << 8;

/// The R1 word's current state, bits 12:9: 4 is transfer, 5 sending data, 6 receive data, 7
/// programming.
#[must_use]
pub const fn r1_state(r1: u32) -> u32 {
    (r1 >> 9) & 0xf
}

/// The transfer state, the one a selected card idles in.
pub const STATE_TRANSFER: u32 = 4;

/// **Bits `[start, start + len)` of a 128-bit register** held as four words, low word first.
/// `len` above 32 is clamped to 32; positions past bit 127 read as zero.
#[must_use]
pub const fn bits(r: &[u32; 4], start: u32, len: u32) -> u32 {
    let len = if len > 32 { 32 } else { len };
    if len == 0 || start >= 128 {
        return 0;
    }
    let word = (start / 32) as usize;
    let shift = start % 32;
    let mut v = (r[word] as u64) >> shift;
    if shift + len > 32 && word + 1 < 4 {
        v |= (r[word + 1] as u64) << (32 - shift);
    }
    let mask = if len == 32 {
        0xffff_ffff
    } else {
        (1u64 << len) - 1
    };
    (v & mask) as u32
}

/// The CSD, decoded to the two facts this driver uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Csd {
    /// The structure version, bits 127:126: 0 standard capacity, 1 high or extended capacity
    /// (SDHC and SDXC), 2 ultra capacity (SDUC). For eMMC it is a different field with different
    /// meaning; see [`Csd::sd`]'s caller.
    pub structure: u32,
    /// Capacity in 512-byte blocks, or 0 when the structure is one this driver does not decode.
    pub blocks: u64,
}

impl Csd {
    /// **Decode an SD card's CSD.**
    ///
    /// ```
    /// use designware_mobile_storage::card::Csd;
    ///
    /// // A 32 GB SDHC card: structure 1, C_SIZE 60_947 (bits 69:48).
    /// let csd = [0, 60_947u32 << 16, 0, 1 << 30];
    /// assert_eq!(Csd::sd(&csd).blocks, 60_948 * 1024);
    /// ```
    #[must_use]
    pub const fn sd(r: &[u32; 4]) -> Csd {
        let structure = bits(r, 126, 2);
        let blocks = match structure {
            // (C_SIZE + 1) * 2^(C_SIZE_MULT + 2) blocks of 2^READ_BL_LEN bytes. READ_BL_LEN is 9,
            // 10 or 11 on every card made; anything from 9 to 15 is decoded and a smaller one
            // refused rather than divided.
            0 => {
                let c_size = bits(r, 62, 12) as u64;
                let mult = bits(r, 47, 3);
                let read_bl_len = bits(r, 80, 4);
                if read_bl_len < 9 {
                    0
                } else {
                    (c_size + 1) << (mult + 2 + read_bl_len - 9)
                }
            }
            // (C_SIZE + 1) * 512 KiB, C_SIZE 22 bits.
            1 => (bits(r, 48, 22) as u64 + 1) * 1024,
            // SDUC: the same, C_SIZE 28 bits.
            2 => (bits(r, 48, 28) as u64 + 1) * 1024,
            _ => 0,
        };
        Csd { structure, blocks }
    }

    /// **Decode an eMMC's CSD**, which gives a capacity only up to 2 GB. Above that `C_SIZE` reads
    /// `0xfff` and the true count is `EXT_CSD`'s [`ext_csd_sectors`]. The layout of the size fields
    /// is the same as SD's version 1.
    #[must_use]
    pub const fn mmc(r: &[u32; 4]) -> Csd {
        let mut csd = Csd::sd(r);
        let c_size = bits(r, 62, 12) as u64;
        let mult = bits(r, 47, 3);
        let read_bl_len = bits(r, 80, 4);
        csd.blocks = if read_bl_len < 9 {
            0
        } else {
            (c_size + 1) << (mult + 2 + read_bl_len - 9)
        };
        csd
    }
}

/// The CID, decoded to what a transcript should name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cid {
    /// Manufacturer identifier, bits 127:120.
    pub manufacturer: u8,
    /// Product name: five ASCII bytes for SD (bits 103:64), six for eMMC (bits 103:56), padded with
    /// zero.
    pub name: [u8; 6],
    /// Product serial number, bits 55:24 (SD) or 47:16 (eMMC).
    pub serial: u32,
}

impl Cid {
    /// Decode an SD card's CID.
    #[must_use]
    pub const fn sd(r: &[u32; 4]) -> Cid {
        let mut name = [0u8; 6];
        let mut i = 0;
        while i < 5 {
            name[i] = bits(r, 96 - 8 * i as u32, 8) as u8;
            i += 1;
        }
        Cid {
            manufacturer: bits(r, 120, 8) as u8,
            name,
            serial: bits(r, 24, 32),
        }
    }

    /// Decode an eMMC's CID.
    #[must_use]
    pub const fn mmc(r: &[u32; 4]) -> Cid {
        let mut name = [0u8; 6];
        let mut i = 0;
        while i < 6 {
            name[i] = bits(r, 96 - 8 * i as u32, 8) as u8;
            i += 1;
        }
        Cid {
            manufacturer: bits(r, 120, 8) as u8,
            name,
            serial: bits(r, 16, 32),
        }
    }
}

/// `EXT_CSD`'s `SEC_COUNT`, bytes 212 to 215, little-endian: an eMMC's capacity in 512-byte
/// sectors. Zero for a buffer too short to hold it.
#[must_use]
pub fn ext_csd_sectors(ext_csd: &[u8]) -> u64 {
    match ext_csd.get(212..216) {
        Some(b) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(r: &mut [u32; 4], start: u32, len: u32, v: u32) {
        for i in 0..len {
            let bit = start + i;
            let w = (bit / 32) as usize;
            let mask = 1u32 << (bit % 32);
            if (v >> i) & 1 == 1 {
                r[w] |= mask;
            } else {
                r[w] &= !mask;
            }
        }
    }

    #[test]
    fn a_field_that_straddles_two_words_is_read_whole() {
        let mut r = [0u32; 4];
        put(&mut r, 62, 12, 0xabc);
        assert_eq!(bits(&r, 62, 12), 0xabc);
        put(&mut r, 120, 8, 0x1b);
        assert_eq!(bits(&r, 120, 8), 0x1b);
        assert_eq!(bits(&r, 128, 8), 0);
        assert_eq!(bits(&r, 0, 0), 0);
    }

    #[test]
    fn a_standard_capacity_card_is_sized_from_three_fields() {
        // A 2 GB card: C_SIZE 4095, C_SIZE_MULT 7, READ_BL_LEN 10.
        let mut r = [0u32; 4];
        put(&mut r, 126, 2, 0);
        put(&mut r, 62, 12, 4095);
        put(&mut r, 47, 3, 7);
        put(&mut r, 80, 4, 10);
        assert_eq!(Csd::sd(&r).blocks * 512, 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn a_read_block_length_below_512_is_refused_rather_than_shifted_negative() {
        let mut r = [0u32; 4];
        put(&mut r, 80, 4, 3);
        assert_eq!(Csd::sd(&r).blocks, 0);
        assert_eq!(Csd::mmc(&r).blocks, 0);
    }

    #[test]
    fn an_sdxc_card_is_sized_in_half_megabytes() {
        let mut r = [0u32; 4];
        put(&mut r, 126, 2, 1);
        put(&mut r, 48, 22, 0x3f_ffff);
        assert_eq!(Csd::sd(&r).blocks, 0x40_0000 * 1024);
    }

    #[test]
    fn a_cid_names_its_maker_and_product() {
        let mut r = [0u32; 4];
        put(&mut r, 120, 8, 0x03);
        for (i, b) in b"SN32G".iter().enumerate() {
            put(&mut r, 96 - 8 * i as u32, 8, u32::from(*b));
        }
        put(&mut r, 24, 32, 0xdead_beef);
        let cid = Cid::sd(&r);
        assert_eq!(cid.manufacturer, 3);
        assert_eq!(&cid.name[..5], b"SN32G");
        assert_eq!(cid.serial, 0xdead_beef);
    }

    #[test]
    fn ext_csd_sector_count_is_little_endian_and_short_buffers_read_zero() {
        let mut e = [0u8; 512];
        e[212..216].copy_from_slice(&0x01d2_0000u32.to_le_bytes());
        assert_eq!(ext_csd_sectors(&e), 0x01d2_0000);
        assert_eq!(ext_csd_sectors(&e[..100]), 0);
    }

    #[test]
    fn r1_state_and_errors() {
        let ready_in_transfer = (STATE_TRANSFER << 9) | R1_READY_FOR_DATA;
        assert_eq!(r1_state(ready_in_transfer), STATE_TRANSFER);
        assert_eq!(ready_in_transfer & R1_ERRORS, 0);
        // CARD_IS_LOCKED is not an error; ILLEGAL_COMMAND is.
        assert_eq!((1 << 25) & R1_ERRORS, 0);
        assert_ne!((1 << 22) & R1_ERRORS, 0);
    }
}
