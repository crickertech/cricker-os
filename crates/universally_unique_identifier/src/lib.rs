//! **The universally unique identifier** of RFC 9562: sixteen bytes, a text form, and the
//! version-4 stamp.
//!
//! # One byte order, and the one place it is not used
//!
//! RFC 9562 section 4 stores a UUID by "sequencing all fields in big-endian format", so the sixteen bytes
//! and the printed hex are in the same order: `919108F7-52D1-4320-9BAC-F847DB4148A8` is the bytes
//! `91 91 08 F7 52 D1 43 20 9B AC F8 47 DB 41 48 A8`. [`Uuid`] holds exactly those bytes, so
//! [`Uuid::to_bytes`], [`Uuid::to_ascii`] and the RFC agree with no swapping anywhere.
//!
//! The same section names the exception: Microsoft's COM `GUID` struct, which UEFI inherited and the
//! GUID Partition Table writes to disk, stores its first three fields **little-endian**. That layout
//! is a property of the container, not of the identifier, so it lives with the container:
//! `globally_unique_identifier_partition_table::guid::from_disk` and `to_disk` are the only two
//! functions in the tree that produce or consume it. A `Uuid` never holds mixed-endian bytes, which
//! is what makes it impossible to print one in the wrong order: there is no second order for the
//! printer to be wrong about.
//!
//! # This crate has no randomness
//!
//! [`Uuid::v4_from_random`] stamps bytes the caller brings. On nife they come from the entropy
//! service (`user_mode_runtime::entropy::fill`); a UUID built from a counter would be unique on one
//! machine and collide with every other's, which is the failure the format exists to prevent.
//!
//! # Examples
//!
//! RFC 9562 Appendix A.3's own example, from the sixteen random bytes it starts with to the string
//! it ends with:
//!
//! ```
//! use universally_unique_identifier::Uuid;
//!
//! let random = [
//!     0x91, 0x91, 0x08, 0xF7, 0x52, 0xD1, 0x33, 0x20, 0x5B, 0xAC, 0xF8, 0x47, 0xDB, 0x41, 0x48, 0xA8,
//! ];
//! let id = Uuid::v4_from_random(random);
//! assert_eq!(&id.to_ascii(), b"919108F7-52D1-4320-9BAC-F847DB4148A8");
//! assert_eq!(Uuid::try_from_ascii(b"919108f7-52d1-4320-9bac-f847db4148a8"), Some(id));
//! ```
//!
//! # BUGS
//!
//! - **Version 4 only.** No name-based (v3, v5) or time-based (v1, v6, v7) constructor: each needs
//!   a hash or a clock this crate does not hold, and nothing in the tree has asked for one.
//! - **No `version()` or `variant()` reader.** Nothing reads them back except the tests, which look
//!   at the printed form where a person would.
//!
//! Name: ratified 2026-10-06 (UTC) by calef: "The crate should be `universally_unique_identifier`."
//! Minted the same day by lane/uuid-crate when the type moved out of the partition table's crate.
//! RFC 9562's term, which the `uuid` program already uses, expanded per §154 (the acronym test is
//! whether the phrase is spoken) because "universally unique identifier" is a phrase people say.
//! One concept takes one name, the rule calef's brief for this lane cited from §113 (the plain,
//! standard term). RFC 9562 section 1 says a UUID is "also known as" a GUID, so this crate does not
//! say GUID; the partition table keeps that word only where it names the UEFI specification's own
//! fields (`PartitionTypeGUID`, `DiskGUID`).

#![cfg_attr(not(test), no_std)]

/// A 128-bit UUID, in RFC 9562's byte order (big-endian in every field, the order it prints in).
///
/// Build one from its printed fields with [`Uuid::from_fields`] (const, so it serves for constants),
/// from canonical bytes with [`Uuid::from_bytes`], or from random bytes with
/// [`Uuid::v4_from_random`]. A GPT on disk does not hold canonical bytes; see the crate docs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// The Nil UUID (RFC 9562 section 5.9), all 128 bits zero. In a GPT entry's type field it means "this
    /// entry is unused": the one UUID with a meaning rather than an identity.
    pub const NIL: Uuid = Uuid([0; 16]);

    /// Build a UUID from its five printed groups. `C12A7328-F81F-11D2-BA4B-00A0C93EC93B` is
    /// `from_fields(0xC12A7328, 0xF81F, 0x11D2, [0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B])`.
    ///
    /// The integers go in big-endian because that is the RFC's order, not because they are being
    /// encoded for anything: the bytes this returns are the printed digits, two at a time.
    pub const fn from_fields(a: u32, b: u16, c: u16, rest: [u8; 8]) -> Uuid {
        let a = a.to_be_bytes();
        let b = b.to_be_bytes();
        let c = c.to_be_bytes();
        Uuid([
            a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], rest[0], rest[1], rest[2], rest[3],
            rest[4], rest[5], rest[6], rest[7],
        ])
    }

    /// Wrap sixteen bytes already in RFC 9562 order. Every bit pattern is a UUID, so this cannot
    /// fail. **Not for bytes read off a GPT**, which are mixed-endian; those go through the
    /// partition table crate's `guid::from_disk`.
    pub const fn from_bytes(bytes: [u8; 16]) -> Uuid {
        Uuid(bytes)
    }

    /// The sixteen bytes in RFC 9562 order, which is the order [`Uuid::to_ascii`] prints them.
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Stamp sixteen random bytes into an RFC 9562 version-4 UUID.
    ///
    /// Six bits are spent (RFC 9562 section 5.4): the version nibble `4` is the high half of byte 6 and the variant
    /// `0b10` is the top of byte 8. Because this type is in the RFC's byte order those are the
    /// RFC's offsets, and the printed form shows them at characters 14 and 19, which is where
    /// `uuidgen` and `sgdisk -i` read them. The other 122 bits are the caller's, untouched.
    ///
    /// **Bring unpredictable bytes.** This function adds no randomness and cannot detect its
    /// absence: sixteen zeros in give a well-formed UUID that every other zero-fed caller also gets.
    pub const fn v4_from_random(mut bytes: [u8; 16]) -> Uuid {
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid(bytes)
    }

    /// True for [`Uuid::NIL`] only.
    pub const fn is_nil(self) -> bool {
        let mut i = 0;
        while i < 16 {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// The `XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX` form, uppercase, as ASCII.
    ///
    /// Uppercase because RFC 9562 section 4 allows either case and the tools a nife UUID is compared
    /// against most (`sgdisk`, macOS's `uuidgen`, the UEFI shell) print uppercase. A fixed buffer
    /// rather than a `String` because this crate allocates nothing; `Display` wraps it.
    pub fn to_ascii(self) -> [u8; 36] {
        const HEX: [u8; 16] = *b"0123456789ABCDEF";
        let mut out = [b'-'; 36];
        let mut at = 0;
        for (n, &byte) in self.0.iter().enumerate() {
            if n == 4 || n == 6 || n == 8 || n == 10 {
                at += 1; // leave the dash this position was initialized with
            }
            out[at] = HEX[(byte >> 4) as usize];
            out[at + 1] = HEX[(byte & 0xf) as usize];
            at += 2;
        }
        out
    }

    /// Parse the hex-and-dash form, either case. `None` unless it is 36 characters with dashes at
    /// the four places RFC 9562's grammar puts them and hex digits everywhere else.
    pub fn try_from_ascii(text: &[u8]) -> Option<Uuid> {
        if text.len() != 36 {
            return None;
        }
        for &dash in &[8usize, 13, 18, 23] {
            if text[dash] != b'-' {
                return None;
            }
        }
        let mut bytes = [0u8; 16];
        let mut at = 0;
        for (n, byte) in bytes.iter_mut().enumerate() {
            if n == 4 || n == 6 || n == 8 || n == 10 {
                at += 1;
            }
            *byte = (nibble(text[at])? << 4) | nibble(text[at + 1])?;
            at += 2;
        }
        Some(Uuid(bytes))
    }
}

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl core::fmt::Display for Uuid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ascii = self.to_ascii();
        // to_ascii emits only hex digits and dashes, so this is always valid UTF-8.
        f.write_str(core::str::from_utf8(&ascii).unwrap_or("<uuid>"))
    }
}

/// `Debug` prints the same form as `Display`. The derived one would print sixteen decimal bytes,
/// which nobody can match against a `uuidgen` or `sgdisk -i` listing.
impl core::fmt::Debug for Uuid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 9562 Appendix A.3, end to end: the sixteen random bytes the RFC starts from, stamped,
    /// give the string the RFC ends with. This is the test that pins the byte order: a type that
    /// stamped GPT's on-disk offsets (version in byte 7) would print `F7089191-D152-4033-...` from
    /// the same input, a valid version-4 UUID and not the RFC's.
    #[test]
    fn the_rfc_9562_version_4_example_comes_out_as_printed() {
        let random = [
            0x91, 0x91, 0x08, 0xF7, 0x52, 0xD1, 0x33, 0x20, 0x5B, 0xAC, 0xF8, 0x47, 0xDB, 0x41,
            0x48, 0xA8,
        ];
        let id = Uuid::v4_from_random(random);
        assert_eq!(&id.to_ascii(), b"919108F7-52D1-4320-9BAC-F847DB4148A8");
        assert_eq!(
            id.to_bytes(),
            [
                0x91, 0x91, 0x08, 0xF7, 0x52, 0xD1, 0x43, 0x20, 0x9B, 0xAC, 0xF8, 0x47, 0xDB, 0x41,
                0x48, 0xA8,
            ],
            "canonical bytes are the printed digits in order",
        );
        assert_eq!(
            Uuid::try_from_ascii(b"919108f7-52d1-4320-9bac-f847db4148a8"),
            Some(id)
        );
    }

    /// `from_fields` is the printed groups, not an encoding of them. RFC 9562 Figure 1's example.
    #[test]
    fn fields_are_the_printed_groups() {
        let id = Uuid::from_fields(
            0xF81D_4FAE,
            0x7DEC,
            0x11D0,
            [0xA7, 0x65, 0x00, 0xA0, 0xC9, 0x1E, 0x6B, 0xF6],
        );
        assert_eq!(&id.to_ascii(), b"F81D4FAE-7DEC-11D0-A765-00A0C91E6BF6");
        assert_eq!(id.to_bytes()[..4], [0xF8, 0x1D, 0x4F, 0xAE]);
    }

    #[test]
    fn text_round_trips_and_junk_is_refused() {
        let id = Uuid::from_fields(0x1234_5678, 0x9ABC, 0x4DEF, [1, 2, 3, 4, 5, 6, 7, 8]);
        for g in [id, Uuid::NIL] {
            assert_eq!(Uuid::try_from_ascii(&g.to_ascii()), Some(g));
        }
        assert_eq!(
            &Uuid::NIL.to_ascii(),
            b"00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(Uuid::try_from_ascii(b"too short"), None);
        assert_eq!(
            Uuid::try_from_ascii(b"C12A7328+F81F-11D2-BA4B-00A0C93EC93B"),
            None,
            "a dash in the wrong place"
        );
        assert_eq!(
            Uuid::try_from_ascii(b"G12A7328-F81F-11D2-BA4B-00A0C93EC93B"),
            None,
            "G is not hex"
        );
    }

    /// Display and Debug both print the string a person looks up.
    #[test]
    fn a_uuid_shows_itself() {
        let g = Uuid::from_fields(0x1234_5678, 0x9ABC, 0x4DEF, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(g.to_string(), "12345678-9ABC-4DEF-0102-030405060708");
        assert_eq!(format!("{g:?}"), "12345678-9ABC-4DEF-0102-030405060708");
    }

    /// The six reserved bits land where a reader looks for them: character 14 is the version and
    /// character 19 the variant. At any other offsets the result would still be unique and would
    /// read as some other version forever.
    #[test]
    fn stamping_random_bytes_gives_a_version_4_uuid() {
        for fill in [0x00u8, 0xff, 0x5a, 0xa5] {
            let text = Uuid::v4_from_random([fill; 16]).to_ascii();
            assert_eq!(text[14], b'4', "version nibble, from a {fill:#04x} fill");
            assert!(
                matches!(text[19], b'8' | b'9' | b'A' | b'B'),
                "variant bits, from a {fill:#04x} fill: got {}",
                text[19] as char,
            );
        }
    }

    /// The other 122 bits are the caller's. A stamp that normalized more than the format reserves
    /// would throw away entropy the caller paid a round trip for.
    #[test]
    fn stamping_keeps_every_bit_it_does_not_reserve() {
        let raw = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let out = Uuid::v4_from_random(raw).to_bytes();
        for (i, (&before, &after)) in raw.iter().zip(out.iter()).enumerate() {
            match i {
                6 => assert_eq!(after, (before & 0x0f) | 0x40),
                8 => assert_eq!(after, (before & 0x3f) | 0x80),
                _ => assert_eq!(after, before, "byte {i} was modified"),
            }
        }
    }

    #[test]
    fn only_the_nil_uuid_is_nil() {
        assert!(Uuid::NIL.is_nil());
        // One bit set, in the last byte, which a "first word is zero" check misses.
        let mut last = [0u8; 16];
        last[15] = 1;
        assert!(!Uuid::from_bytes(last).is_nil());
    }
}

// =================================================================================================
// Kani proofs. The tests above pin the RFC's vectors; these quantify over all 2^128 inputs.
// =================================================================================================
#[cfg(kani)]
mod verification {
    use super::*;

    /// **A UUID survives printing and parsing**, for all 2^128 of them.
    ///
    /// Moved from the partition table crate on 2026-10-06 (UTC), where it was
    /// `a_guid_survives_printing_and_parsing` and also covered the mixed-endian swap. The swap now
    /// lives in that crate's `guid::from_disk`/`to_disk` and is proved there.
    /// Falsification: replayable `crates/universally_unique_identifier/falsifications/verification.a_uuid_survives_printing_and_parsing.patch`
    #[kani::proof]
    #[kani::unwind(37)]
    fn a_uuid_survives_printing_and_parsing() {
        let g = Uuid::from_bytes(kani::any());
        assert_eq!(Uuid::try_from_ascii(&g.to_ascii()), Some(g));
    }

    /// **The version-4 stamp lands where a reader looks, and touches nothing else**, for all 2^128
    /// inputs.
    ///
    /// Two claims in one, and the second is the one a test cannot make convincingly: the printed
    /// form says version 4 and a `10` variant, *and* the other 122 bits are exactly the caller's.
    /// Falsification: replayable `crates/universally_unique_identifier/falsifications/verification.stamping_reserves_six_bits_and_keeps_the_other_hundred_and_twenty_two.patch`
    #[kani::proof]
    #[kani::unwind(37)]
    fn stamping_reserves_six_bits_and_keeps_the_other_hundred_and_twenty_two() {
        let raw: [u8; 16] = kani::any();
        let stamped = Uuid::v4_from_random(raw).to_bytes();
        let text = Uuid::v4_from_random(raw).to_ascii();
        assert_eq!(text[14], b'4');
        assert!(matches!(text[19], b'8' | b'9' | b'A' | b'B'));
        for i in 0..16 {
            if i != 6 && i != 8 {
                assert_eq!(stamped[i], raw[i]);
            }
        }
        assert_eq!(stamped[6] & 0x0f, raw[6] & 0x0f);
        assert_eq!(stamped[8] & 0x3f, raw[8] & 0x3f);
    }
}
