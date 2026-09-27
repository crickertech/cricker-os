//! **Two boot slots, and the rule that stops a bad upgrade bricking the machine.**
//!
//! An installed nife machine keeps two copies of its boot image. An upgrade writes the one that is
//! not running and marks it *on trial*. If the trial image fails to come up, **the machine goes
//! back to the previous one by itself**, with nobody at the console. This crate is the state that
//! makes that true and the policy that reads it; it touches no disk, so the program that writes the
//! state (`components/src/installer.rs`) and the program that acts on it (`uefi_loader`'s chooser)
//! share one implementation rather than two readings of a paragraph.
//!
//! calef ruled on 2026-09-21: *"Yes, write the tries and priority attributes in 2b."*
//!
//! Name: ratified 2026-09-21 by calef, minted the same day by the rung 2b lane. A noun for the thing on the disk,
//! singular like `page_frames`' members and `grant_plan`, rather than for the mechanism over it.
//! Refused `ab_boot`, which is the industry's word for this (A/B partitions) and which says
//! *two* when nothing in here does; refused `boot_slot_attributes`, which names the byte layout
//! rather than the concept and would have to change the day the state moved. `slot` alone was
//! refused as ambiguous in a capability system, where a slot is a place in a cspace. calef names
//! crates; he took this one as minted, and the refusals above are the valuable half of that record.
//!
//! # Where the state lives, and why it is allowed to live there
//!
//! In the **GPT partition entry attribute bits of the slot partitions themselves**. UEFI 2.11
//! section 5.3.3's attribute table says of bits 48 to 63, verbatim:
//!
//! > Reserved for GUID specific use. The use of these bits will vary depending on the
//! > `PartitionTypeGUID`. Only the owner of the `PartitionTypeGUID` is allowed to modify these
//! > bits. They must be preserved if Bits 0-47 are modified.
//!
//! A boot slot's partition type is
//! [`globally_unique_identifier_partition_table::guid::types::NIFE_BOOT`], which is ours, so these
//! sixteen bits are ours by specification and **no other operating system's partition tool will
//! touch them**. That is the whole reason to prefer them to a file: a file in a filesystem is
//! something anything that can mount the volume may rewrite, and the state that decides whether a
//! machine boots should not be.
//!
//! ## The correction that shapes the rest of this crate
//!
//! It is easy to say, and was said on this project, that *firmware* does the selecting. **It does
//! not, and cannot.** ChromeOS is where this design comes from and it gets firmware-level selection
//! because depthcharge is a custom coreboot payload that **replaces** UEFI; its BIOS is a program
//! Google wrote. Generic UEFI firmware, OVMF included, reads bits 48 to 63 of a type it does not
//! own for exactly no purpose: the specification just told it not to. So **the selector has to be
//! ours**, and on a UEFI machine the only code of ours that runs before the kernel is the image at
//! `\EFI\BOOT\BOOTX64.EFI`. That image is therefore the chooser, and the slots are what it chooses
//! between.
//!
//! The other mechanism a reader will reach for is UEFI `Boot####` variables with `BootOrder` and
//! `BootNext`, which firmware genuinely does read, and it was evaluated and refused.
//! Rung 2a of milestone 198 (a package manager, and the trivial install that makes a second customer possible) proved its boot **with the firmware variable store deleted**, on purpose,
//! so that "the firmware found the file on its own" was a claim about the disk rather than about
//! leftovers from a previous run. A rollback design that depends on those variables surviving
//! contradicts the one property that boot was built to demonstrate, and it would also make the
//! state unreadable and unwritable from anywhere but firmware runtime services, which this kernel
//! does not map.
//!
//! # The bit layout, which is a format two programs agree on
//!
//! **Ratified by calef on 2026-09-21** ("Ratify the GUID and bit positions"), and written here
//! because this is where a reader meets it. Before that date it was provisional; it is now a format
//! this project has committed to, and changing it is a decision about disks that already exist. The positions are ChromeOS's own, verified against the ChromiumOS disk-format
//! reference rather than recalled, so that anyone who has run `cgpt show` can read a nife disk:
//!
//! | bits | field | meaning |
//! |---|---|---|
//! | 48-51 | [`PRIORITY_SHIFT`] | 0 to 15. **0 means never select this slot.** Higher wins |
//! | 52-55 | [`TRIES_SHIFT`] | 0 to 15. Boot attempts left before the slot is given up on |
//! | 56 | [`SUCCESSFUL_BIT`] | this image has booted and been confirmed; [`State::tries`] stops mattering |
//! | 57-63 | | reserved, written zero, and [`State::into_attributes`] preserves whatever is there |
//!
//! Bits 0 to 47 are the specification's own (`ATTR_REQUIRED` and friends) and are preserved
//! untouched, which is what that table row asks for in its last sentence.
//!
//! # The policy
//!
//! A slot is bootable when its priority is above zero **and** it has either been confirmed or has
//! tries left ([`State::is_bootable`]). [`select`] returns the bootable slot of highest priority,
//! breaking a tie toward the lower index so that two slots written with the same priority still
//! produce the same boot twice running.
//!
//! **The chooser decrements `tries` before it hands off, not after**, and that is the crux of the
//! whole design rather than a detail. A chooser that only read, leaving the running system to mark
//! itself good once it was up, would protect against an image that *crashes* and not against one
//! that **hangs**: a machine that wedges before userspace would retry the same bad image forever,
//! and the console nobody is standing at would show nothing. Writing first costs a disk write on
//! every trial boot and buys the failure mode that actually strands people. ChromeOS decrements in
//! firmware before the launch for the same reason.
//!
//! # EXAMPLES
//!
//! The life of one upgrade, in the states the disk holds between reboots:
//!
//! ```
//! use boot_slot::{State, select};
//!
//! // After an install: slot 0 holds the image that was just written and is known good.
//! let mut slots = [State::installed(), State::EMPTY];
//! assert_eq!(select(&slots), Some(0));
//!
//! // An upgrade writes slot 1 and puts it on trial, above slot 0.
//! slots[1] = State::on_trial(State::installed().priority + 1, 3);
//! assert_eq!(select(&slots), Some(1));
//!
//! // Three boots that never come up. Each one spends a try BEFORE handing off.
//! for _ in 0..3 {
//!     let chosen = select(&slots).expect("something is always bootable here");
//!     slots[chosen] = slots[chosen].attempted();
//! }
//!
//! // Nobody had to do anything: the machine is back on the image it came with.
//! assert_eq!(select(&slots), Some(0));
//! ```
//!
//! And the same upgrade when it works: something in the booted system calls [`State::confirmed`],
//! and the trial slot stops being a trial.
//!
//! ```
//! # use boot_slot::{State, select};
//! let mut slots = [State::installed(), State::on_trial(3, 3)];
//! slots[1] = select(&slots).map(|i| slots[i].attempted()).unwrap();
//! slots[1] = slots[1].confirmed();
//! assert!(slots[1].successful);
//! assert_eq!(select(&slots), Some(1));
//! ```
//!
//! # BUGS
//!
//! - **The bit layout and [`SlotHeader`]'s bytes are ratified, which makes them expensive rather
//!   than safe.** They are a format two programs agree on, which `AGENTS.md` puts in the
//!   irreversible category; calef ratified both on 2026-09-21, so the cost of changing them is now
//!   paid in disks rather than in code. [`SlotHeader`] carries a version field for exactly that
//!   reason, and any future change should use it rather than redefining the bits.
//! - **What confirms a trial boot is a promise, and here is what it can be wrong about.**
//!   `installer`'s `ROLE_CONFIRM` calls [`State::confirmed`] on a running machine once the
//!   filesystem server has mounted the installed disk and reported ready, which is the latest
//!   point this system can reach without a person. So a boot that is confirmed still may not have
//!   exercised **anything the boot path does not touch** (the network stack, the compositor, a
//!   driver for a device nothing opens at boot), has **not reached a shell**, and says nothing
//!   about the seconds after it: a leak, a wedge under load, or a filesystem that mounts and then
//!   corrupts is confirmed and kept. `kernel/src/user/install_service.rs`'s `confirm` argues the
//!   criterion and the milestone block has the rest.
//! - **Nothing in this crate marks a slot successful on its own**, and that is the layering rather
//!   than a gap: this crate touches no disk. The program is `ROLE_CONFIRM` above, and a machine
//!   that never runs it rolls back an upgrade that was working, which fails safe.
//! - **Two slots is not a number this crate enforces.** [`select`] takes a slice of any length and
//!   the policy is the same for three; `installer` lays out two because a third costs a partition
//!   and buys nothing until something can use it.
//! - **Priority is not rotated on exhaustion.** ChromeOS drops an exhausted slot's priority to zero
//!   so that its firmware's plain highest-priority scan moves on. [`select`]'s predicate already
//!   excludes an exhausted slot, so nife does not need the write, and not doing it keeps the
//!   record of what the priorities *were* when the machine last chose. The one place nife does
//!   spend priority is [`State::unreadable`], for a slot whose image will not load at all.
//! - **A slot is 64 MiB of disk that is usually a duplicate.** Two copies of a ten-megabyte boot
//!   image is the price of the property, and it is paid on every installed machine whether or not
//!   it is ever upgraded.

#![no_std]

use globally_unique_identifier_partition_table::crc;

/// The lowest bit of the four-bit priority field: bits 48 to 51.
pub const PRIORITY_SHIFT: u32 = 48;
/// The lowest bit of the four-bit tries field: bits 52 to 55.
pub const TRIES_SHIFT: u32 = 52;
/// The confirmed-boot bit: bit 56.
pub const SUCCESSFUL_BIT: u32 = 56;

/// The largest value either four-bit field holds, and the ceiling both setters saturate at.
pub const MAX_NIBBLE: u8 = 15;

/// **The priority `installer` gives the slot it writes at install time.**
///
/// Not 15, deliberately. An upgrade has to be able to outrank the running image without first
/// lowering it, and a first install that took the top of the range would force every upgrader to
/// begin by rewriting a partition entry it is not otherwise touching.
pub const INSTALLED_PRIORITY: u8 = 2;

/// Mask of the bits this crate owns: 48 through 56. Everything else in an attribute word is
/// somebody else's and is preserved.
const OWNED: u64 = (0xF << PRIORITY_SHIFT) | (0xF << TRIES_SHIFT) | (1 << SUCCESSFUL_BIT);

/// **One slot's boot state**, as the three fields the attribute bits carry.
///
/// A plain struct rather than a wrapped `u64` so that a caller reading a disk in a debugger sees
/// three numbers, and so that the encode/decode round trip is something the tests can state.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct State {
    /// 0 to 15. **Zero means never select this slot**, whatever else is set.
    pub priority: u8,
    /// 0 to 15. Boot attempts left before the slot stops being bootable.
    pub tries: u8,
    /// This image has booted and something confirmed it, so [`Self::tries`] no longer matters.
    pub successful: bool,
}

impl State {
    /// A slot that holds nothing and will never be chosen. What `installer` writes into the spare
    /// slot, so that an upgrader finds a partition already laid out and does not have to repartition
    /// a running machine to install an update.
    pub const EMPTY: State = State {
        priority: 0,
        tries: 0,
        successful: false,
    };

    /// **The state of the slot an install just wrote**: confirmed, because the image in it is the
    /// one that is running at that moment and has therefore already booted this machine once.
    ///
    /// That claim is worth being precise about, because it is the strongest thing said anywhere in
    /// this crate. The bytes written into slot 0 by an install are a copy of the file the firmware
    /// started a few seconds earlier, on this machine, with this firmware. Marking it good is a
    /// record of an observation rather than an assumption.
    pub const fn installed() -> State {
        State {
            priority: INSTALLED_PRIORITY,
            tries: 0,
            successful: true,
        }
    }

    /// A slot holding a freshly written image that has never booted: outranking whatever is there,
    /// and with `tries` attempts to prove itself before the machine gives up on it.
    ///
    /// Both arguments saturate at [`MAX_NIBBLE`] rather than wrapping, because a wrapped priority
    /// of 16 is a priority of 0, which is the one value that means "never boot this".
    pub const fn on_trial(priority: u8, tries: u8) -> State {
        State {
            priority: if priority > MAX_NIBBLE {
                MAX_NIBBLE
            } else {
                priority
            },
            tries: if tries > MAX_NIBBLE {
                MAX_NIBBLE
            } else {
                tries
            },
            successful: false,
        }
    }

    /// **Can this slot be chosen?** Priority above zero, and either confirmed or with tries left.
    /// The same two clauses ChromeOS's firmware checks, in the same order.
    pub const fn is_bootable(&self) -> bool {
        self.priority > 0 && (self.successful || self.tries > 0)
    }

    /// **The state to write before handing off to this slot**: one try spent.
    ///
    /// A confirmed slot is returned unchanged, so a machine running a good image writes nothing to
    /// its disk on an ordinary boot. That is not only tidiness: a write on every boot is a write
    /// that can be interrupted on every boot.
    pub const fn attempted(&self) -> State {
        if self.successful || self.tries == 0 {
            *self
        } else {
            State {
                tries: self.tries - 1,
                ..*self
            }
        }
    }

    /// **The state of a slot that has proved itself**: confirmed, and with its tries returned to
    /// zero because they no longer mean anything.
    ///
    /// Priority is left alone. A trial slot was written with a priority above the running one, so
    /// confirming it is enough to keep it chosen; raising it further would spend range for nothing.
    pub const fn confirmed(&self) -> State {
        State {
            priority: self.priority,
            tries: 0,
            successful: true,
        }
    }

    /// **The state of a slot whose image cannot be read at all**: given up on immediately rather
    /// than after its remaining tries.
    ///
    /// This is the one place nife writes a zero priority, and the reason is that the failure is
    /// one the chooser can see for itself. Tries exist to bound a failure that happens *after* the
    /// handoff, where nothing is watching. A header that does not decode, or an image whose
    /// checksum is wrong, is a failure the chooser observed with its own eyes, and spending three
    /// reboots to re-observe it would be three reboots of a machine somebody is waiting on.
    /// ChromeOS does the same thing for an invalid kernel header.
    pub const fn unreadable(&self) -> State {
        State {
            priority: 0,
            tries: 0,
            successful: false,
        }
    }

    /// Decode the three fields out of a GPT entry's attribute word.
    ///
    /// Total: every `u64` decodes, because every bit pattern of four bits is a number from 0 to 15
    /// and every bit is a boolean. There is no such thing as a malformed slot state, only one that
    /// is not bootable, which is the same layering
    /// [`globally_unique_identifier_partition_table::Entry::decode`] keeps.
    pub const fn from_attributes(attributes: u64) -> State {
        State {
            priority: ((attributes >> PRIORITY_SHIFT) & 0xF) as u8,
            tries: ((attributes >> TRIES_SHIFT) & 0xF) as u8,
            successful: (attributes >> SUCCESSFUL_BIT) & 1 == 1,
        }
    }

    /// The attribute word to write back, **preserving every bit this crate does not own**: the
    /// specification's bits 0 to 47 and the reserved 57 to 63.
    ///
    /// That preservation is the specification's own instruction ("They must be preserved if Bits
    /// 0-47 are modified", read in both directions) and it is why this takes the previous word
    /// rather than building one from nothing.
    pub const fn into_attributes(self, previous: u64) -> u64 {
        let priority = (self.priority & 0xF) as u64;
        let tries = (self.tries & 0xF) as u64;
        let successful = self.successful as u64;
        (previous & !OWNED)
            | (priority << PRIORITY_SHIFT)
            | (tries << TRIES_SHIFT)
            | (successful << SUCCESSFUL_BIT)
    }
}

/// **Which slot to boot**: the bootable one of highest priority, ties broken toward the lower
/// index.
///
/// `None` when nothing is bootable, and the caller's answer to that is the interesting half. On
/// this system it is not "refuse to boot": see [`select_excluding`] and `uefi_loader`'s chooser,
/// which falls back to the image in its own file.
pub fn select(slots: &[State]) -> Option<usize> {
    select_excluding(slots, 0)
}

/// [`select`], ignoring every slot whose bit is set in `tried`.
///
/// **The chooser retries within one boot**, which is the difference between a failure it can see
/// and one it cannot. An image whose header will not decode is a failure the chooser observed
/// before handing off, and making a person wait for a reboot to recover from it would be a worse
/// machine for no gain. An image that starts and then hangs is a failure nothing observes, and
/// that one is what [`State::attempted`] and the write-before-handoff are for.
///
/// Bit `n` of `tried` corresponds to slot `n`; slots past bit 63 cannot be excluded, which is not a
/// limit anything reaches with two.
pub fn select_excluding(slots: &[State], tried: u64) -> Option<usize> {
    slots
        .iter()
        .enumerate()
        .filter(|(i, s)| s.is_bootable() && (*i >= 64 || tried & (1 << i) == 0))
        .max_by_key(|(i, s)| (s.priority, core::cmp::Reverse(*i)))
        .map(|(i, _)| i)
}

/// The magic at the start of a slot partition: eight bytes a reader can find with `strings`.
pub const SLOT_MAGIC: [u8; 8] = *b"NIFESLOT";

/// The layout version this crate writes and the only one it reads.
pub const SLOT_VERSION: u32 = 1;

/// **Bytes of a slot partition reserved for [`SlotHeader`] before the image begins**: 4096, one
/// whole transfer block of `filesystem_protocol::blk`.
///
/// Only the first [`SLOT_HEADER_BYTES`] carry anything. The rest is padding, and it is padding
/// rather than a smaller reservation so that **the image starts on a transfer-block boundary**.
/// `installer` writes whole 4096-byte blocks, and milestone 198 rung 2a already lost ten megabytes
/// to an unaligned start once; that bug is recorded in `crates/file_allocation_table` and this
/// number is what stops it happening a second time in a different file.
pub const SLOT_IMAGE_OFFSET: u64 = 4096;

/// Bytes of [`SLOT_IMAGE_OFFSET`] that [`SlotHeader::encode`] actually fills.
pub const SLOT_HEADER_BYTES: usize = 32;

/// **What is in a boot slot**, written at the very start of the partition and read by the chooser
/// before it spends a page of memory on the image.
///
/// A partition has no length field of its own that means "how much of me is in use", so this is
/// where the image's length lives. The checksum is the other half and is what turns "the installer
/// died halfway through the copy" from a machine that starts a truncated image into a machine that
/// refuses the slot and boots the other one.
///
/// | offset | size | |
/// |---|---|---|
/// | 0 | 8 | [`SLOT_MAGIC`] |
/// | 8 | 4 | [`SLOT_VERSION`], little-endian |
/// | 12 | 4 | reserved, zero |
/// | 16 | 8 | [`Self::image_len`], little-endian |
/// | 24 | 4 | [`Self::image_crc32`], little-endian |
/// | 28 | 4 | CRC-32 of bytes 0 to 27, little-endian |
///
/// CRC-32 is the GPT's own (`globally_unique_identifier_partition_table::crc`), reused rather than
/// written again: this crate already depends on that one for the attribute bits it lives in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlotHeader {
    /// How many bytes of image follow, starting [`SLOT_IMAGE_OFFSET`] into the partition.
    pub image_len: u64,
    /// CRC-32 of exactly those bytes.
    pub image_crc32: u32,
}

impl SlotHeader {
    /// The header for an image, with its checksum computed from the bytes themselves.
    pub fn of(image: &[u8]) -> SlotHeader {
        SlotHeader {
            image_len: image.len() as u64,
            image_crc32: crc::crc32(image),
        }
    }

    /// Write the header into the start of a buffer of at least [`SLOT_HEADER_BYTES`], zeroing
    /// nothing else. `false` if the buffer is too small.
    pub fn encode(&self, out: &mut [u8]) -> bool {
        if out.len() < SLOT_HEADER_BYTES {
            return false;
        }
        out[0..8].copy_from_slice(&SLOT_MAGIC);
        out[8..12].copy_from_slice(&SLOT_VERSION.to_le_bytes());
        out[12..16].copy_from_slice(&0u32.to_le_bytes());
        out[16..24].copy_from_slice(&self.image_len.to_le_bytes());
        out[24..28].copy_from_slice(&self.image_crc32.to_le_bytes());
        let own = crc::crc32(&out[0..28]);
        out[28..32].copy_from_slice(&own.to_le_bytes());
        true
    }

    /// Read a header back. `None` for a buffer that is too short, a wrong magic, a version this
    /// build does not know, or a header whose own checksum does not hold.
    ///
    /// **An empty slot decodes as `None` rather than as a zero-length image**, because a partition
    /// that was never written is all zeros and all zeros is not the magic. That is the shape a
    /// caller wants: "there is nothing here" and "there is something broken here" both mean do not
    /// boot this slot.
    pub fn decode(bytes: &[u8]) -> Option<SlotHeader> {
        if bytes.len() < SLOT_HEADER_BYTES || bytes[0..8] != SLOT_MAGIC {
            return None;
        }
        if u32::from_le_bytes(bytes[8..12].try_into().ok()?) != SLOT_VERSION {
            return None;
        }
        if u32::from_le_bytes(bytes[28..32].try_into().ok()?) != crc::crc32(&bytes[0..28]) {
            return None;
        }
        Some(SlotHeader {
            image_len: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
            image_crc32: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
        })
    }

    /// **Are these the bytes this header describes?** The check the chooser runs after reading an
    /// image and before starting it.
    pub fn matches(&self, image: &[u8]) -> bool {
        image.len() as u64 == self.image_len && crc::crc32(image) == self.image_crc32
    }
}

/// **The token that tells a booted kernel which slot it came from**, and the only thing that
/// crosses from the chooser into the running system.
///
/// # Why there is a token at all
///
/// A running system cannot work out which slot started it. The obvious inference, *"it is whatever
/// [`select`] would pick now"*, is wrong in exactly the case that matters: the chooser spends a try
/// before handing off, so a slot that was started with its last try is no longer
/// [`State::is_bootable`] and [`select`] now names the other one. The one boot a confirmation
/// exists for is the one the inference gets backwards, so the number has to be carried rather than
/// recomputed.
///
/// # Why it is a word on the command line
///
/// `uefi_loader`'s `hvm_start_info` already carries a NUL-terminated command line, already holds
/// more than one word on it (the screen, and `machine_discovery::framebuffer::SCREEN_HOLD` under
/// the gate's feature), and the kernel already reads it before `mmu::init`. So this is a third word
/// on a line that exists rather than a new agreement.
///
/// The three shapes it was weighed against, priced rather than asserted:
///
/// | | cost |
/// |---|---|
/// | a field in `machine_discovery`'s handoff | a new field in a structure the PVH specification fixes, or a nife-private one beside it, and a second thing for a loader on each of three architectures to fill in |
/// | a module in the `hvm_start_info` module list | the list has two entries and a third would be a page of memory and an entry format, to carry one nibble |
/// | a fixed physical page | a number two programs agree on with nothing checking it, and the one shape this tree has no precedent for |
/// | **a word on the command line** | **one token, one parser, and a host test that round-trips it against the kernel's own reader** |
///
/// **This is a value two programs agree on**, which `AGENTS.md` puts in the expensive category, so
/// **the spelling was ratified by calef on 2026-09-22** ("Ratified"), after he asked what a slot is
/// and why the number cannot be recomputed. It is now a value this project has committed to.
///
/// **Why it cannot be recomputed, which is what the ruling turned on.** A reader will reach for
/// inferring the slot from the table: the one with the highest priority and tries remaining must be
/// the one that booted. **That inference is wrong precisely on the boot a confirmation exists for.**
/// The chooser spends a try *before* handing off, so by the time the kernel looks, the state is no
/// longer the state the decision was made from: on a trial boot with one try left, the slot that
/// actually booted now shows zero, and the inference names the other one. A machine would then
/// confirm the copy it did not boot, which is worse than never confirming.
///
/// # The cost worth naming
///
/// The boot command line now has tokens owned by two crates: the screen's are
/// `machine_discovery::framebuffer`'s and this one is here. That split is deliberate (a reader
/// asking what `boot-slot=1` means looks in the crate named after boot slots) and it is a cost: no
/// single place enumerates the line's vocabulary. `uefi_loader::handoff::cmdline` is the writer
/// that sees all of it, and its `CMDLINE_LEN` is the one place the lengths are added up.
///
/// # EXAMPLES
///
/// ```
/// use boot_slot::cmdline;
///
/// let mut out = [0u8; cmdline::MAX_LEN];
/// let n = cmdline::encode(1, &mut out);
/// let line = core::str::from_utf8(&out[..n]).unwrap();
/// assert_eq!(line, "boot-slot=1");
/// assert_eq!(cmdline::parse(line), Some(1));
///
/// // Beside the screen, in either order, which is what the kernel actually reads.
/// assert_eq!(cmdline::parse("screen=0x80000000,800,600,3200,bgrx boot-slot=0"), Some(0));
///
/// // A line that says nothing about slots is a boot that was not started by a chooser.
/// assert_eq!(cmdline::parse("screen=0x80000000,800,600,3200,bgrx"), None);
/// ```
pub mod cmdline {
    /// The token's key, including the `=`. Provisional: names are an architect's.
    pub const KEY: &str = "boot-slot=";

    /// The longest this token can be: the key and one decimal digit.
    ///
    /// One digit, not two, because [`super::select_excluding`] already cannot exclude past slot 63
    /// and `installer` lays out two. A slot number that did not fit a digit would be a different
    /// disk layout, and [`encode`] refuses rather than truncating.
    pub const MAX_LEN: usize = KEY.len() + 1;

    /// Write `boot-slot=N` into `out`, returning its length, or **0 when it does not fit or `slot`
    /// is not a single digit**.
    ///
    /// Zero rather than a panic or a truncation: the caller is a bootloader with no console left,
    /// and a line it could not write is a boot that does not confirm, which is the direction this
    /// whole feature fails in anyway.
    #[must_use]
    pub fn encode(slot: u8, out: &mut [u8]) -> usize {
        if slot > 9 || out.len() < MAX_LEN {
            return 0;
        }
        out[..KEY.len()].copy_from_slice(KEY.as_bytes());
        out[KEY.len()] = b'0' + slot;
        MAX_LEN
    }

    /// **Which slot started this boot**, from the whole command line, or `None` when no word on it
    /// is this token.
    ///
    /// `None` is the ordinary answer and not an error: a stick, a `-kernel` boot, and an installed
    /// machine whose chooser fell back to the image in its own file all reach a running kernel with
    /// nothing to confirm.
    ///
    /// A token whose value is not a single digit is `None` too, for the reason [`encode`] refuses
    /// to write one: a malformed slot number that parsed as *some* slot would be a confirmation
    /// written to the wrong partition entry, which is the one outcome worse than not confirming.
    #[must_use]
    pub fn parse(cmdline: &str) -> Option<u8> {
        let word = cmdline
            .split_ascii_whitespace()
            .find(|w| w.starts_with(KEY))?;
        let digits = &word[KEY.len()..];
        match digits.as_bytes() {
            [d @ b'0'..=b'9'] => Some(d - b'0'),
            _ => None,
        }
    }
}
