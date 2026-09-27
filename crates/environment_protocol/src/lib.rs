//! **The inert-configuration contract** (milestone 47's environment-variable fork, DECISIONS
//! §111). One definition of the read-only page that carries `TZ`, `LANG` and `TERM` into a
//! process, so whoever assembles it (init, or a kernel test harness standing in for init today)
//! and whoever reads it (the `std` PAL, and eventually a `no_std` program through `user_mode_runtime`)
//! cannot drift. The same split `clock_protocol` makes for the wall clock and `filesystem_protocol` makes for
//! the filesystem.
//!
//! # Why a page, and why validated
//!
//! Milestone 47 splits what Unix puts in one string-to-string environment map into three parts:
//! inert configuration (`TZ`, `LANG`, `TERM`, genuinely just data), names (`PATH`, `HOME`,
//! already answered by the namespace half), and secrets (already answered, an endpoint, §41).
//! DECISIONS §111 decided the inert third's wire encoding: a read-only page a process is either
//! granted or is not (the same rights ladder the clock page uses: no capability, or a `PageFrame`
//! with `READ`), because a config value is never something a person designates on a command
//! line and there is nothing to propose or set from inside the process that holds it.
//!
//! **Why validated rather than a free-form string.** A capability governs reach, not meaning:
//! once a value is bytes on a page, nothing about the capability model can tell a password from
//! a timezone, and `AWS_SECRET_KEY`-as-env-var is a real, repeated Unix problem for exactly that
//! reason. §111's answer is this tree's strongest tool applied to value shape instead of
//! authority: **make the wrong state unrepresentable.** A byte sequence has to parse as a member
//! of a specific, known-safe [`domain`] to go onto this page at all, so an API key does not
//! parse as `America/Los_Angeles` and cannot ride through disguised as one. [`PageBuilder`]'s
//! three setters are where that is enforced, at assembly time, before the page exists to anyone.
//!
//! # The page has one writer, once, before it has a second reader
//!
//! Unlike [`clock_protocol`](../clock_protocol)'s page, which is published to repeatedly by a running
//! service while readers hold a stale mapping, this page is assembled once, in a buffer nothing
//! else can see, and only *then* mapped read-only into the process that will read it. There is
//! no seqlock here because there is nothing to race: the writer finishes before the page has a
//! second observer. [`ConfigPage::new`]'s contract is the same "stays mapped, never written by
//! this process" shape the clock page's reader relies on; it just does not need the seqlock's
//! seq-and-fence dance to get there.
//!
//! # A zeroed frame reads as "no configuration", by construction
//!
//! [`MAGIC`] is checked before anything else is read, the same default-honest shape
//! `clock_protocol::ClockPage` uses: a frame nobody has assembled a page into, or a page belonging
//! to something else entirely, answers `None` for every key rather than inventing a locale
//! nobody chose (DECISIONS §42's no-silent-degradation rule, applied here rather than to time).
//!
//! # Examples
//!
//! Assembling a page is validate-then-write, and an unrecognized value is refused before it
//! reaches the page at all:
//!
//! ```
//! use environment_protocol::{ConfigPage, PageBuilder, Refused};
//!
//! let page = PageBuilder::new()
//!     .tz("America/Los_Angeles").unwrap()
//!     .lang("en_US.UTF-8").unwrap()
//!     .term("xterm-256color").unwrap()
//!     .build();
//!
//! // SAFETY: `page` is a live, aligned buffer of exactly `PAGE_BYTES`, alive for this block.
//! let read = unsafe { ConfigPage::new(page.as_ptr() as u64) };
//! assert_eq!(read.tz(), Some("America/Los_Angeles"));
//! assert_eq!(read.lang(), Some("en_US.UTF-8"));
//! assert_eq!(read.term(), Some("xterm-256color"));
//!
//! // A value outside the domain is refused when the page is built, not carried through
//! // disguised as configuration.
//! assert_eq!(
//!     PageBuilder::new().tz("definitely-not-a-timezone"),
//!     Err(Refused::UnknownTz),
//! );
//! ```
//!
//! A key nobody declared reads as absent, and a frame nobody has assembled a page into reads as
//! "no configuration" rather than as empty strings:
//!
//! ```
//! use environment_protocol::{ConfigPage, PageBuilder};
//!
//! let page = PageBuilder::new().tz("UTC").unwrap().build(); // LANG and TERM left undeclared
//! // SAFETY: as above.
//! let read = unsafe { ConfigPage::new(page.as_ptr() as u64) };
//! assert_eq!(read.tz(), Some("UTC"));
//! assert_eq!(read.lang(), None);
//! assert_eq!(read.term(), None);
//!
//! let zeroed = [0u8; environment_protocol::PAGE_BYTES];
//! // SAFETY: as above.
//! let nothing = unsafe { ConfigPage::new(zeroed.as_ptr() as u64) };
//! assert_eq!(nothing.tz(), None);
//! assert_eq!(nothing.lang(), None);
//! assert_eq!(nothing.term(), None);
//! ```
//!
//! # BUGS
//!
//! - **The domains are curated, real, and deliberately not exhaustive.** [`domain::KNOWN_TZ`] is
//!   not the ~600-zone IANA timezone database, [`domain::KNOWN_LANG`] is not glibc's locale
//!   list, and [`domain::KNOWN_TERM`] is not terminfo's. Vendoring any of those wholesale is a
//!   dependency decision (DECISIONS §46) this crate does not make on its own; the lists here are
//!   grown on demand, the same posture `clock_protocol::rtc` takes toward RTC bindings. Every entry
//!   is a real, checkable value; the list is short, not fabricated.
//! - **A value longer than its key's cap is a build-time bug, not a runtime refusal.** The
//!   domains are curated short on purpose (§111 does not ask this crate to support arbitrary
//!   IANA identifiers), so every member of every `KNOWN_*` list fits under its field's cap by
//!   construction, and [`PageBuilder::build`] truncates rather than panics if that invariant is
//!   ever violated by a future addition to a domain list. A host test pins it instead: growing a
//!   domain with an over-length member fails the suite, not a process at spawn.
//! - **No wire announcement exists yet**, because nothing that spawns through `grant_plan`'s
//!   shell protocol declares wanting this page: `Manifest` carries no `config` field, the way it
//!   carries `clock` for `date`. What is built end to end is the page format and the kernel- and
//!   `std`-PAL-side wiring that grants it to a std program, proven by `std_exerciser`
//!   (`system_tests/src/user/std_service.rs`, `patches/std-nife/overlay/std/src/sys/env/nife.rs`). A
//!   shell-facing program that wants a declared key, and the `caps` preview extension §111 also
//!   asks for, wait on a real customer the way `clock` waited on `date`.
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from
//! `env_proto`: spell out the contraction fully. The suffix then became `_protocol` at milestone 265
//! (calef, 2026-09-05), which spells out the other truncation in the same name; the stem and the
//! 2026-08-23 ruling are untouched.

#![cfg_attr(not(test), no_std)]

/// The curated, validated domains each declared key is checked against.
///
/// See the crate's own `BUGS` section for what "curated" means here: every value listed is real
/// and checkable, and the lists are short by design rather than by neglect.
pub mod domain {
    /// A non-exhaustive but real subset of IANA timezone identifiers. Grown on demand; see the
    /// crate's `BUGS` section.
    pub const KNOWN_TZ: &[&str] = &[
        "UTC",
        "America/New_York",
        "America/Chicago",
        "America/Denver",
        "America/Los_Angeles",
        "America/Anchorage",
        "America/Sao_Paulo",
        "Europe/London",
        "Europe/Paris",
        "Europe/Berlin",
        "Europe/Moscow",
        "Africa/Cairo",
        "Africa/Johannesburg",
        "Asia/Tokyo",
        "Asia/Shanghai",
        "Asia/Kolkata",
        "Asia/Dubai",
        "Asia/Singapore",
        "Australia/Sydney",
        "Pacific/Auckland",
    ];

    /// A non-exhaustive but real subset of POSIX/glibc locale identifiers.
    pub const KNOWN_LANG: &[&str] = &[
        "C",
        "POSIX",
        "en_US",
        "en_US.UTF-8",
        "en_GB",
        "en_GB.UTF-8",
        "de_DE.UTF-8",
        "fr_FR.UTF-8",
        "es_ES.UTF-8",
        "ja_JP.UTF-8",
        "zh_CN.UTF-8",
        "pt_BR.UTF-8",
    ];

    /// A non-exhaustive but real subset of terminfo terminal type names.
    pub const KNOWN_TERM: &[&str] = &[
        "dumb",
        "vt100",
        "vt220",
        "ansi",
        "linux",
        "xterm",
        "xterm-256color",
        "screen",
        "screen-256color",
        "tmux",
        "tmux-256color",
    ];

    /// Whether `s` is a member of [`KNOWN_TZ`].
    pub fn is_valid_tz(s: &str) -> bool {
        KNOWN_TZ.contains(&s)
    }

    /// Whether `s` is a member of [`KNOWN_LANG`].
    pub fn is_valid_lang(s: &str) -> bool {
        KNOWN_LANG.contains(&s)
    }

    /// Whether `s` is a member of [`KNOWN_TERM`].
    pub fn is_valid_term(s: &str) -> bool {
        KNOWN_TERM.contains(&s)
    }
}

/// The page's first eight bytes, so a reader can tell an assembled config page from a zeroed
/// frame or from somebody else's page. ASCII, ends unpadded at 8 bytes so the ASCII reads
/// straight in a hex dump the way `clock_protocol::MAGIC` does.
pub const MAGIC: [u8; 8] = *b"ENVCONF1";

/// The most bytes a `TZ` value may occupy. Longer than every entry in [`domain::KNOWN_TZ`]
/// today (the longest IANA identifiers, like `America/Argentina/ComodRivadavia`, run to about
/// 33 bytes), so a curated list can grow toward the real database's longer names without a
/// layout change.
pub const TZ_MAX: usize = 40;

/// The most bytes a `LANG` value may occupy. Covers every entry in [`domain::KNOWN_LANG`] with
/// headroom for a codeset suffix this crate does not carry yet (`en_US.ISO8859-1` is 16 bytes).
pub const LANG_MAX: usize = 24;

/// The most bytes a `TERM` value may occupy. Covers every entry in [`domain::KNOWN_TERM`]
/// (`screen-256color` is the longest at 15) with headroom to grow.
pub const TERM_MAX: usize = 24;

const OFF_MAGIC: usize = 0;
const OFF_TZ_LEN: usize = OFF_MAGIC + 8;
const OFF_TZ: usize = OFF_TZ_LEN + 1;
const OFF_LANG_LEN: usize = OFF_TZ + TZ_MAX;
const OFF_LANG: usize = OFF_LANG_LEN + 1;
const OFF_TERM_LEN: usize = OFF_LANG + LANG_MAX;
const OFF_TERM: usize = OFF_TERM_LEN + 1;

/// The whole page's size in bytes: magic, then one length-prefixed field per declared key. Well
/// under one frame (4096 bytes), which is the unit both the kernel and the `std` PAL map this
/// as.
pub const PAGE_BYTES: usize = OFF_TERM + TERM_MAX;

/// Why [`PageBuilder`] refused a value: it does not parse as a member of its key's [`domain`].
///
/// Not an errno space, the same way `clock_protocol::status` is not one: this is a build-time
/// refusal, not a runtime failure a process observes. Whoever assembles a page (init, or a test
/// harness standing in for it) handles this the way a manifest mismatch is handled elsewhere in
/// this tree, as a refusal at the point of assembly rather than a value silently dropped.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refused {
    /// The `TZ` value is not in [`domain::KNOWN_TZ`].
    UnknownTz,
    /// The `LANG` value is not in [`domain::KNOWN_LANG`].
    UnknownLang,
    /// The `TERM` value is not in [`domain::KNOWN_TERM`].
    UnknownTerm,
}

/// Assemble an inert-configuration page. Each setter validates against its key's [`domain`]
/// before accepting the value, so a page this type builds can never carry a value outside its
/// key's closed set. A key never given a value stays absent: [`ConfigPage`] reads it back as
/// `None`, not as an empty string.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PageBuilder<'a> {
    tz: Option<&'a str>,
    lang: Option<&'a str>,
    term: Option<&'a str>,
}

impl<'a> PageBuilder<'a> {
    /// A builder with every key undeclared.
    pub const fn new() -> Self {
        PageBuilder {
            tz: None,
            lang: None,
            term: None,
        }
    }

    /// Declare `TZ`. Refused if `value` is not a member of [`domain::KNOWN_TZ`].
    pub fn tz(mut self, value: &'a str) -> Result<Self, Refused> {
        if domain::is_valid_tz(value) {
            self.tz = Some(value);
            Ok(self)
        } else {
            Err(Refused::UnknownTz)
        }
    }

    /// Declare `LANG`. Refused if `value` is not a member of [`domain::KNOWN_LANG`].
    pub fn lang(mut self, value: &'a str) -> Result<Self, Refused> {
        if domain::is_valid_lang(value) {
            self.lang = Some(value);
            Ok(self)
        } else {
            Err(Refused::UnknownLang)
        }
    }

    /// Declare `TERM`. Refused if `value` is not a member of [`domain::KNOWN_TERM`].
    pub fn term(mut self, value: &'a str) -> Result<Self, Refused> {
        if domain::is_valid_term(value) {
            self.term = Some(value);
            Ok(self)
        } else {
            Err(Refused::UnknownTerm)
        }
    }

    /// Write the declared keys into a fresh page buffer. Bytes for a key never declared stay
    /// zero, which is what [`ConfigPage`] reads as absent.
    pub fn build(&self) -> [u8; PAGE_BYTES] {
        let mut page = [0u8; PAGE_BYTES];
        page[OFF_MAGIC..OFF_MAGIC + 8].copy_from_slice(&MAGIC);
        write_field(&mut page, OFF_TZ_LEN, OFF_TZ, TZ_MAX, self.tz);
        write_field(&mut page, OFF_LANG_LEN, OFF_LANG, LANG_MAX, self.lang);
        write_field(&mut page, OFF_TERM_LEN, OFF_TERM, TERM_MAX, self.term);
        page
    }
}

/// Write one length-prefixed field. `val`'s bytes are truncated to `cap` rather than causing a
/// panic if a future domain member ever exceeds its field's cap (see the crate's `BUGS`
/// section); a host test pins every domain member's length against its cap so this path is not
/// how that gets caught in practice.
fn write_field(page: &mut [u8], len_off: usize, data_off: usize, cap: usize, val: Option<&str>) {
    let Some(v) = val else { return };
    let bytes = v.as_bytes();
    let n = bytes.len().min(cap).min(u8::MAX as usize);
    page[len_off] = n as u8;
    page[data_off..data_off + n].copy_from_slice(&bytes[..n]);
}

/// Read one length-prefixed field back as a `&str`. `None` for a zero length (the key was never
/// declared) or for bytes that do not decode as UTF-8 (a page [`PageBuilder`] wrote never
/// produces this; a foreign or corrupt page might, and the honest answer is absence rather than
/// a panic).
fn read_field(page: &[u8], len_off: usize, data_off: usize) -> Option<&str> {
    let n = page[len_off] as usize;
    if n == 0 {
        return None;
    }
    core::str::from_utf8(&page[data_off..data_off + n]).ok()
}

/// **The inert-configuration page**, as seen through one process's read-only mapping of it.
///
/// See the crate's own docs for why this needs no seqlock: the page has exactly one writer, and
/// that writer finishes before the page has a second reader.
#[derive(Debug, Clone, Copy)]
pub struct ConfigPage {
    base: *const u8,
}

// SAFETY: the whole point of the page is that the process that assembled it and the process
// that reads it are different address spaces; every access is a plain read of immutable bytes
// (see the crate docs), so there is no mutable aliasing to protect against.
unsafe impl Send for ConfigPage {}
// SAFETY: as `Send` above.
unsafe impl Sync for ConfigPage {}

impl ConfigPage {
    /// Name the config page mapped at `va`.
    ///
    /// # Safety
    ///
    /// `va` must be a mapped, byte-aligned buffer of at least [`PAGE_BYTES`] bytes (either an
    /// assembled page, or a zeroed frame, which reads as "no configuration"), and it must stay
    /// mapped and unwritten by this process for as long as this value is used. The page is
    /// read-only by construction on the kernel side (the same `PageFrame` `READ`-only shape the
    /// clock page uses), so a caller with a correctly typed capability cannot violate this by
    /// accident.
    pub const unsafe fn new(va: u64) -> Self {
        ConfigPage {
            base: va as *const u8,
        }
    }

    /// # Safety
    ///
    /// Upheld by [`new`](Self::new)'s contract: `base` names at least `PAGE_BYTES` mapped,
    /// stable bytes.
    fn bytes(&self) -> &[u8] {
        // SAFETY: `new`'s contract.
        unsafe { core::slice::from_raw_parts(self.base, PAGE_BYTES) }
    }

    fn is_recognized(&self) -> bool {
        self.bytes()[OFF_MAGIC..OFF_MAGIC + 8] == MAGIC
    }

    /// `TZ`, or `None` if this process holds no config page, the page is unrecognized, or `TZ`
    /// was never declared onto it.
    pub fn tz(&self) -> Option<&str> {
        self.is_recognized()
            .then(|| read_field(self.bytes(), OFF_TZ_LEN, OFF_TZ))
            .flatten()
    }

    /// `LANG`, or `None` for the same three reasons [`tz`](Self::tz) can be `None`.
    pub fn lang(&self) -> Option<&str> {
        self.is_recognized()
            .then(|| read_field(self.bytes(), OFF_LANG_LEN, OFF_LANG))
            .flatten()
    }

    /// `TERM`, or `None` for the same three reasons [`tz`](Self::tz) can be `None`.
    pub fn term(&self) -> Option<&str> {
        self.is_recognized()
            .then(|| read_field(self.bytes(), OFF_TERM_LEN, OFF_TERM))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every curated domain member fits under its field's cap. This is what makes
    /// [`write_field`]'s truncation path dead in practice: a domain grown past its cap fails
    /// here rather than silently truncating a real value at assembly time.
    #[test]
    fn every_domain_member_fits_its_field() {
        for tz in domain::KNOWN_TZ {
            assert!(
                tz.len() <= TZ_MAX,
                "{tz:?} is {} bytes, over TZ_MAX",
                tz.len()
            );
        }
        for lang in domain::KNOWN_LANG {
            assert!(
                lang.len() <= LANG_MAX,
                "{lang:?} is {} bytes, over LANG_MAX",
                lang.len()
            );
        }
        for term in domain::KNOWN_TERM {
            assert!(
                term.len() <= TERM_MAX,
                "{term:?} is {} bytes, over TERM_MAX",
                term.len()
            );
        }
    }

    /// The domains are real (not synthesized) and this is what makes them worth being closed:
    /// every UTC-adjacent name a reader would type is one of the recognized ones, and one that
    /// looks plausible but is not (a made-up city, a common typo) is refused.
    #[test]
    fn domain_checks_accept_real_values_and_refuse_invented_ones() {
        assert!(domain::is_valid_tz("UTC"));
        assert!(domain::is_valid_tz("America/Los_Angeles"));
        assert!(!domain::is_valid_tz("Mars/Olympus_Mons"));
        assert!(!domain::is_valid_tz("america/los_angeles")); // case matters, like the real db

        assert!(domain::is_valid_lang("en_US.UTF-8"));
        assert!(!domain::is_valid_lang("klingon"));

        assert!(domain::is_valid_term("xterm-256color"));
        assert!(!domain::is_valid_term("my-cool-terminal"));
    }

    /// [`PageBuilder`]'s setters refuse before they ever touch a page, and the refusal names
    /// which key.
    #[test]
    fn the_builder_refuses_a_value_outside_its_domain() {
        assert_eq!(
            PageBuilder::new().tz("not-a-real-zone"),
            Err(Refused::UnknownTz)
        );
        assert_eq!(
            PageBuilder::new().lang("not-a-real-locale"),
            Err(Refused::UnknownLang)
        );
        assert_eq!(
            PageBuilder::new().term("not-a-real-terminal"),
            Err(Refused::UnknownTerm)
        );
    }

    /// The full round trip: build with all three keys, read every one back through a page built
    /// from a raw pointer the way a real mapped frame would be named.
    #[test]
    fn a_fully_declared_page_round_trips() {
        let bytes = PageBuilder::new()
            .tz("Europe/Berlin")
            .unwrap()
            .lang("de_DE.UTF-8")
            .unwrap()
            .term("screen-256color")
            .unwrap()
            .build();
        // SAFETY: `bytes` is a live, aligned buffer of exactly PAGE_BYTES for this call's scope.
        let page = unsafe { ConfigPage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.tz(), Some("Europe/Berlin"));
        assert_eq!(page.lang(), Some("de_DE.UTF-8"));
        assert_eq!(page.term(), Some("screen-256color"));
    }

    /// A key never declared reads back absent, distinct from an empty string: this is the
    /// property that makes a page carrying only `TZ` legible as "no LANG was granted" rather
    /// than "LANG was granted the empty string".
    #[test]
    fn an_undeclared_key_reads_as_absent_not_empty() {
        let bytes = PageBuilder::new().tz("UTC").unwrap().build();
        // SAFETY: as above.
        let page = unsafe { ConfigPage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.tz(), Some("UTC"));
        assert_eq!(page.lang(), None);
        assert_eq!(page.term(), None);
    }

    /// A page nobody has ever built into (a zeroed frame, the state a fresh page starts in
    /// before `init` maps something over it) reads as "no configuration" for every key, never
    /// as a fabricated default. This is DECISIONS §42's rule applied to configuration instead of
    /// to the clock.
    #[test]
    fn a_zeroed_frame_carries_no_configuration() {
        let zeroed = [0u8; PAGE_BYTES];
        // SAFETY: as above.
        let page = unsafe { ConfigPage::new(zeroed.as_ptr() as u64) };
        assert_eq!(page.tz(), None);
        assert_eq!(page.lang(), None);
        assert_eq!(page.term(), None);
    }

    /// A page with a recognized magic but garbage in a field's length byte must not read out of
    /// bounds or panic; a length taken from an untrusted-looking page is still bounded by the
    /// field's own cap in this test's construction, which is what the offsets guarantee: a
    /// length byte can be at most 255, and `read_field` slices `data_off..data_off + n`, so a
    /// caller passing a page shorter than `PAGE_BYTES` (a contract violation `new`'s safety
    /// comment already forbids) is the only way this could go wrong, which is exactly why that
    /// safety contract exists rather than a runtime length check.
    #[test]
    fn a_page_with_a_zero_length_after_the_magic_reads_that_field_as_absent() {
        let mut bytes = PageBuilder::new().tz("UTC").unwrap().build();
        // Corrupt the length byte to zero after building, simulating an assembler that wrote
        // the magic but never declared TZ.
        bytes[OFF_TZ_LEN] = 0;
        // SAFETY: as above.
        let page = unsafe { ConfigPage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.tz(), None);
    }

    /// An unrecognized magic refuses every field, even one whose bytes look plausible: the magic
    /// check runs before any field read, so a frame that merely happens to start with the same
    /// bytes as a real `TZ` value does not get misread as a config page.
    #[test]
    fn an_unrecognized_magic_refuses_every_field() {
        let mut bytes = PageBuilder::new()
            .tz("UTC")
            .unwrap()
            .lang("C")
            .unwrap()
            .term("dumb")
            .unwrap()
            .build();
        bytes[OFF_MAGIC] ^= 0xff; // flip one byte of the magic
        // SAFETY: as above.
        let page = unsafe { ConfigPage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.tz(), None);
        assert_eq!(page.lang(), None);
        assert_eq!(page.term(), None);
    }

    /// The layout constants are self-consistent (no overlap, no gap that would misalign a field
    /// against what `write_field`/`read_field` compute), pinned so a mutant swapping an offset
    /// is caught here rather than by a torn read on real hardware. Asserting on constants is
    /// this test's entire purpose (the same shape `clock_protocol`'s
    /// `the_sanity_window_is_where_it_says` test pins its own layout constants with), so the
    /// lint that flags a constant assertion as pointless is switched off on purpose here.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_layout_offsets_do_not_overlap() {
        assert_eq!(OFF_TZ_LEN, 8);
        assert_eq!(OFF_TZ, 9);
        assert_eq!(OFF_LANG_LEN, 9 + TZ_MAX);
        assert_eq!(OFF_LANG, OFF_LANG_LEN + 1);
        assert_eq!(OFF_TERM_LEN, OFF_LANG + LANG_MAX);
        assert_eq!(OFF_TERM, OFF_TERM_LEN + 1);
        assert_eq!(PAGE_BYTES, OFF_TERM + TERM_MAX);
        assert!(PAGE_BYTES < 4096, "the page must fit in one frame");
    }
}
