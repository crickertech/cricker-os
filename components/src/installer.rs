//! **`installer`**: the program a booted stick runs to put this system on the machine's own disk
//! (milestone 198 (a package manager, and the trivial install that makes a second customer
//! possible), rung 2a).
//!
//! The proposal this follows is
//! [milestone 515 (a stick that puts itself on the machine's disk)](../../design/roadmap/515-the-installer-a-stick-runs-to-put-itself-on-the-disk.md),
//! under DECISIONS §157 (a trivial install is a web page, a USB drive, and packages).
//!
//! Name: provisional, minted 2026-09-21 by the rung 2a lane. `crates/stick_maker`'s own header
//! set the word aside for exactly this program when it refused it for itself: *"DECISIONS §157's
//! rung 2 installs nife onto a PC's own disk, and that is a different program."* calef names
//! programs; expect this to change.
//!
//! # This is the most destructive program in this tree, and its shape is the answer to that
//!
//! It replaces a disk's partition table and everything the table described. What stands between a
//! stranger and a wiped disk is **not** this program's judgement, and that is the point:
//!
//! - It holds **one disk**, as a `filesystem_protocol::blk` endpoint, and there is no path to type.
//!   `parted /dev/sda` as root reaches every disk in the machine; this reaches the one it was
//!   handed. That is `disk_partitioner`'s claim, from milestone 57 (partitioning and formatting a
//!   real drive), and this program inherits it whole.
//! - It holds an **entropy endpoint** and nothing else besides, so a GPT's unique ids are drawn
//!   rather than invented, and a run with no entropy writes nothing at all.
//! - **It is not spawned until a person has answered a question naming the disk.** The asking is
//!   `kernel/src/user/install_service.rs`'s, deliberately: a confirmation this program printed to
//!   itself would be a confirmation it could also decide to skip, and the authority to wipe the
//!   disk arrives here only after the answer.
//!
//! # Two roles, and the second is what keeps the first honest
//!
//! | `a0` | |
//! |---|---|
//! | [`ROLE_SURVEY`] | read the table back and say whether this disk already carries nife |
//! | [`ROLE_INSTALL`] | draw the ids, write the table, lay out the EFI system partition, copy the boot file |
//! | [`ROLE_CONFIRM`] | mark the boot slot this machine started from successful, so a good upgrade sticks |
//!
//! [`ROLE_CONFIRM`] is the trial boot's other half, and it is here rather than in a program of its
//! own for one reason: it is `ROLE_SURVEY` with a write. It reads the same 34 blocks with the same
//! parser, finds the slot the kernel told it about, and puts back one attribute word. A second
//! program would be a second copy of the table read, the block-page mapping, the `blk` client and
//! the verdict convention, to change nine bits.
//!
//! **It holds no entropy endpoint either**, which is the sentence that makes the write narrow
//! rather than merely small. A partition table carries unique ids, so a process with no source of
//! randomness cannot mint one; the only table it can write is a table it read, with the bits
//! `boot_slot` owns changed. What it *can* still do is the honest caveat, and it is in `BUGS`.
//!
//! `ROLE_SURVEY` is **a separate process with no entropy endpoint**, exactly as
//! `disk_partitioner`'s verify role is, and for the same reason: a program that cannot draw a
//! unique id cannot write a partition table anything would read back. It is what lets the boot ask
//! "is nife already on this disk" without the kernel parsing a partition table and without granting
//! the authority to write one.
//!
//! # What it writes, in order
//!
//! | | |
//! |---|---|
//! | 1 | A GPT: a **nife data partition first**, then two boot slots, then an EFI system partition at the end of the disk |
//! | 2 | A FAT32 volume in the EFI system partition (`crates/file_allocation_table`) |
//! | 3 | The boot file, into that volume, at `\EFI\BOOT\BOOTX64.EFI`, where it is the **chooser** |
//! | 4 | The same boot file again, raw, into boot slot 0, with the slot marked good |
//!
//! ## The two boot slots, which are rung 2b and calef's ruling of 2026-09-21
//!
//! *"Yes, write the tries and priority attributes in 2b."* **An installed machine must not be
//! brickable by a bad upgrade**: if a newly written boot image fails to come up, the machine has to
//! go back to the previous one by itself, with nobody at the console.
//!
//! So an install lays out **two** boot slots and fills one. The image in the EFI system partition
//! is a third copy and it has a different job: the firmware starts it, and it chooses between the
//! slots (`uefi_loader`'s chooser). Upgrades replace a slot; the chooser stays where it is.
//!
//! The state that decides which slot boots is three fields in each slot partition's **GPT
//! attribute bits**, which is `crates/boot_slot` and where the whole argument is. What belongs
//! here is only why the slots are partitions and not files: UEFI 2.11 5.3.3 gives bits 48 to 63 to
//! the owner of the partition's type GUID, and
//! [`types::NIFE_BOOT`](globally_unique_identifier_partition_table::guid::types::NIFE_BOOT) is
//! ours. On a file, or on somebody else's partition type, they would not be.
//!
//! This install writes **slot 0 confirmed** ([`boot_slot::State::installed`]) and leaves slot 1
//! laid out and empty, so that an upgrade has somewhere to write without repartitioning a running
//! machine.
//!
//! It does **not** create the filesystem in the data partition. `redoxfs_server`'s `mkfs` already
//! does exactly that, finding the partition by its type GUID, and it is run next by the same
//! service. Two programs with one capability each beats one program with two.
//!
//! ## Why the data partition comes first, which is the unusual half
//!
//! A GPT does not care about order, and every installer a reader has met puts the EFI system
//! partition first. This one does not, and the reason is a property of the filesystem engine rather
//! than a preference. `redoxfs`'s `FileSystem::open` scans blocks `0..65536` of whatever disk it is
//! handed for its header, and adopts the block it finds it at as the filesystem's origin. So a
//! RedoxFS that begins inside the first 256 MiB of a disk is mounted correctly **by a server that
//! was given the whole disk**, and the installed system's boot needs no partition-aware mount, no
//! base-block field on the `blk` wire, and no program in the middle.
//!
//! That is a real simplification and it is also a real authority loss, recorded in `BUGS` below:
//! the filesystem server on an installed machine can address the EFI system partition and the
//! partition table, which a partition-bounded mount would make impossible. `mkfs` shows what the
//! bounded version looks like (its `PartitionDisk`), and closing the gap is a lane of its own.
//!
//! # The boot file, and why it is not in the archive
//!
//! The bytes written at step 3 are a copy of the file this machine was booted from, mapped
//! read-only into this program by the kernel. It cannot come from the archive, because the file
//! *contains* the archive; `uefi_loader`'s `place_boot_file` reads it back off the boot volume
//! while the firmware is still up, and the whole argument is there.
//!
//! **The kernel and the archive therefore move as a set**, which is the failure this design had to
//! make unexpressible: a disk carrying a new kernel beside an archive it does not vouch for halts
//! at `MEASURED BOOT REFUSED`. They are sealed together inside the one file at build time, and this
//! program copies the file.
//!
//! # EXAMPLES
//!
//! ```text
//!   install: disk 1073741824 bytes, 2097152 blocks of 512
//!   install: nife data at 2048..1046527, EFI system at 1046528..2095103
//!   install: EFI system partition laid out, 2099 metadata sectors
//!   install: boot file 10392064 bytes written
//!   report: R_INSTALLED, data at 2048
//! ```
//!
//! # BUGS
//!
//! - **Whole disk only.** The table is replaced wholesale. Installing beside another operating
//!   system means resizing a filesystem nife cannot read, which is milestone 140 (mount a drive
//!   this system did not create)'s territory and a different order of risk to somebody's data.
//! - **The installed filesystem is not bounded by its partition**, for the reason the section above
//!   gives. **The filesystem server on an installed machine holds the whole disk**, bounded only by
//!   the extent `mkfs` recorded in the filesystem's own header, so its allocator never learns about
//!   the blocks past the end and never reaches the EFI system partition or the table. That is a
//!   property of the filesystem rather than a capability, which is the wrong rung of AGENTS.md's
//!   ladder, and it is recorded here rather than papered over. Closing it means bounding the
//!   *server*: either a base-block field on the `blk` wire, which is a value two programs agree on
//!   and therefore calef's, or a caretaker between the disk and the FS server, which costs one IPC
//!   hop per filesystem block. `mkfs`'s own `PartitionDisk` is what the bounded version looks like.
//! - **The logical block size is assumed to be 512**, the same assumption and the same reason as
//!   `disk_surveyor` and `disk_partitioner`: nothing in `filesystem_protocol::blk` carries the
//!   device's. An NVMe namespace formatted with 4096-byte logical blocks would get a table no other
//!   operating system can read. The fix is a field on the wire.
//! - **The boot slots cost 128 MiB and one of them is always empty after an install.** Two 64 MiB
//!   partitions is the price of never being one bad upgrade away from a dead machine, and it is
//!   paid on every installed machine whether or not it is ever upgraded. 64 MiB is
//!   `uefi_loader`'s own `BOOT_FILE_MAX` rather than a measurement of the image.
//! - **Nothing writes slot 1**, because nothing in this tree upgrades a running machine yet. The
//!   slot is laid out and marked never-boot so that whatever does it later does not have to
//!   repartition a disk somebody's data is on. Until then the rollback mechanism is exercised by
//!   `cargo xtask rollback-boot` and by nothing a person does.
//! - **Nothing marks a trial boot successful**, so an upgrade that came up perfectly still rolls
//!   back once its tries are spent. That fails safe and it means upgrades do not stick;
//!   `crates/boot_slot`'s `BUGS` has the whole of it and names the proposal.
//! - **A disk under about 700 MiB is refused**, up from about 600 before the slots. The floor is
//!   the 512 MiB EFI system partition, two 64 MiB slots and the 64 MiB minimum for data, and
//!   nothing tests the refusal.
//! - **Nothing here is crash-atomic.** A power cut partway through leaves a disk that boots nothing.
//!   Real installers share the property; nobody has measured this one.
//! - **There is no progress report during the copy.** Ten megabytes at one 4096-byte request per
//!   round trip is a few thousand calls, and a person watching a serial console sees nothing until
//!   it is done.
//! - **It writes no firmware boot entry.** The file goes at the removable-media fallback path
//!   `\EFI\BOOT\BOOTX64.EFI` on the new disk, which OVMF starts and which most firmware starts.
//!   Milestone 515 records this as B1 and calls it unmeasured on anything but OVMF; a firmware that
//!   refuses it needs `SetVariable`, which is a runtime service this kernel does not map.

#![no_std]
// Program entry points, not the crates/ library surface tracked by milestone 68 (code-quality
// gates: one lint policy)'s ratchet
// (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)).
#![allow(missing_docs)]
#![no_main]

use boot_slot::{SlotHeader, State};
use entropy_protocol as entropy;
use file_allocation_table as fat;
use filesystem_protocol::fixture::blank;
use filesystem_protocol::{blk, req};
use globally_unique_identifier_partition_table::entry::Entry;
use globally_unique_identifier_partition_table::guid::{Guid, types};
use globally_unique_identifier_partition_table::{
    ENTRY_ARRAY_BYTES, GloballyUniqueIdentifierPartitionTable, PRIMARY_HEADER_LBA,
};
use user_mode_runtime::{call, send};

/// Slot 0: where the verdict goes. An endpoint with `WRITE`.
const REPORT: u64 = 0;
/// Slot 1: the block service for the one disk this program was handed, `WRITE`.
const BLK: u64 = 1;
/// Slot 2: the entropy service, `WRITE`.
const ENTROPY: u64 = 2;
/// Slot 3: the untyped this program spends on the page tables its own mapping needs.
const BUDGET: u64 = 3;
/// Slot 4: the page shared with the block server, `READ|WRITE`.
const BLK_PAGE_FRAME: u64 = 4;

/// Where this program puts the page it shares with the block server. Its own choice
/// (milestone 108 (the drivers move onto frame capabilities)), so nothing on the kernel side
/// names this address.
const BLK_PAGE: u64 = address_space_map::service_window(0x5000_0000);

/// **Where the kernel maps the boot file, read-only.** This one *is* named on the kernel side
/// (`kernel/src/user/install_service.rs`'s `BOOT_FILE_VA`) and the two must agree; the bytes are a
/// run of physical frames the kernel owns, so this program cannot map them itself.
const BOOT_FILE_VA: u64 = address_space_map::runtime_window(0x1000_0000);

// The roles, in `a0`. Must match kernel/src/user/install_service.rs.
/// Write the table, the EFI system partition and the boot file.
pub const ROLE_INSTALL: u64 = 0;
/// **Read the table back and report what is on the disk.** No entropy endpoint, no writes.
pub const ROLE_SURVEY: u64 = 1;
/// **Mark the boot slot named in `a1` successful**, so an upgrade that came up is not rolled back.
/// No entropy endpoint; the only table it can write is the one it read.
pub const ROLE_CONFIRM: u64 = 2;

/// The transfer unit of the block service: one filesystem block per request.
const TRANSFER: u64 = blk::BLOCK_SIZE as u64;

/// The logical block size a GPT counts in. See BUGS.
const LBA: u64 = blank::LBA;

/// Logical blocks per transfer block: eight 512-byte sectors in one 4096-byte request.
const SECTORS_PER_TRANSFER: u64 = TRANSFER / LBA;

/// **The alignment every partition starts on**, in logical blocks: 1 MiB, which is what every real
/// partitioning tool uses and what keeps a partition's first byte on an erase block of anything
/// this will ever be written to.
const ALIGN: u64 = 2048;

/// **How big the EFI system partition is**, in logical blocks: 512 MiB.
///
/// It is not the 100 MiB a reader expects, and the reason is arithmetic rather than generosity.
/// `crates/file_allocation_table` refuses to build a volume with fewer than 65525 data clusters,
/// because below that the volume is FAT16 whatever its boot sector claims; at 4096-byte clusters
/// that floor is a little over 256 MiB. 512 MiB clears it with room for a second boot file when
/// somebody builds the two-slot layout milestone 515 lists as L2.
const ESP_SECTORS: u64 = 512 * 1024 * 1024 / 512;

/// **How big one boot slot is**, in logical blocks: 64 MiB.
///
/// The image is about 10 MiB for the tour build and about 19 for the test build, and
/// `uefi_loader`'s own `BOOT_FILE_MAX` refuses anything over 64 as not being this file at all. So
/// 64 MiB is that ceiling exactly: a slot that cannot hold an image the loader would agree to read
/// is a slot that would fail at the worst possible moment.
const SLOT_SECTORS: u64 = 64 * 1024 * 1024 / 512;

/// **How many boot slots an install lays out.** Two, which is the smallest number that can hold a
/// new image without destroying the one that is running.
const SLOTS: usize = 2;

/// **The smallest nife data partition this program will leave behind.** A disk that cannot spare
/// this after the EFI system partition and the boot slots is refused, rather than installed onto
/// and then found to be full.
const DATA_MIN_SECTORS: u64 = 64 * 1024 * 1024 / 512;

/// How many logical blocks the backup table needs at the far end of the disk: the entry array plus
/// its header.
const BACKUP_BLOCKS: u64 = 33;

/// The names written into the partition entries, in disk order. Read back by nothing; a person
/// running `disk_surveyor` or `sgdisk -p` on the installed machine meets them.
const NAMES: [&str; 2 + SLOTS] = ["nife data", "nife slot 0", "nife slot 1", "nife boot"];

/// The removable-media boot file's name on the EFI system partition. 8.3, which is what
/// `file_allocation_table` will write; see that crate's BUGS for the riscv64 name that is not.
const BOOT_FILE_NAME: &str = "BOOTX64.EFI";

// The report's first word. Must match kernel/src/user/install_service.rs.
/// The disk was partitioned, the EFI system partition laid out, and the boot file written. ASCII
/// `INSTD`.
pub const R_INSTALLED: u64 = 0x_49_4E_53_54_44;
/// **No entropy endpoint, so nothing was written.** The same refusal `disk_partitioner` makes, for
/// the same reason: an id that is not random is not unique, and a made-up one is worse than none
/// because it looks right (DECISIONS §42 (a filesystem declares what it offers and must be
/// truthful)).
pub const R_NO_ENTROPY: u64 = 0x_4E_4F_52_4E_47;
/// The disk is too small to hold the layout. The second word is its size in bytes.
pub const R_TOO_SMALL: u64 = 0x_53_4D_41_4C_4C;
/// The disk refused a read or a write, or a layout step failed. The second word says which step.
pub const R_DISK_FAILED: u64 = 0x_44_49_53_4B_45;
/// There is no boot file to copy, so an install would produce a disk that boots nothing.
pub const R_NO_BOOT_FILE: u64 = 0x_4E_4F_42_4F_54;

// [`ROLE_SURVEY`]'s verdicts.
/// **The disk already carries a nife data partition**, so this machine is installed rather than
/// installable. ASCII `HAVIT`.
pub const R_ALREADY: u64 = 0x_48_41_56_49_54;
/// The disk carries no nife data partition: no table at all, or somebody else's.
pub const R_EMPTY: u64 = 0x_45_4D_50_54_59;

// [`ROLE_CONFIRM`]'s verdicts.
/// **The slot is successful**, whether this process set the bit or found it already set. One
/// verdict for both, because an idempotent write has one outcome and a caller that had to tell
/// them apart would be a caller doing bookkeeping the disk already holds. ASCII `CNFRM`.
pub const R_CONFIRMED: u64 = 0x_43_4E_46_52_4D;
/// **There is no such boot slot on this disk**, so nothing was written. ASCII `NOSLT`.
///
/// It is the refusal that matters most: the slot number crosses a boot, and a number this program
/// cannot match to a partition is one it must not guess at. Word 1 carries how many boot slots
/// were found, so a transcript says what it was looking at.
pub const R_NO_SLOT: u64 = 0x_4E_4F_53_4C_54;

/// The entry array under construction. 16 KiB, so it goes in `.bss` rather than on the stack.
static mut ARRAY: [u8; ENTRY_ARRAY_BYTES] = [0; ENTRY_ARRAY_BYTES];
/// The primary table as read back by [`survey`]: LBA 0..33 is 34 logical blocks, five transfer
/// blocks. Also `.bss`, for the same reason.
static mut PRIMARY: [u8; 5 * blk::BLOCK_SIZE] = [0; 5 * blk::BLOCK_SIZE];

#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, a1: u64, _a2: u64) -> ! {
    // The page shared with the block server, mapped out of this program's own budget
    // (milestone 108). Before anything else, because every `blk` call goes through it.
    if !user_mode_runtime::map_page_frame(BLK_PAGE_FRAME, BLK_PAGE, true, BUDGET) {
        user_mode_runtime::exit()
    }
    if role == ROLE_SURVEY {
        survey()
    }
    if role == ROLE_CONFIRM {
        // `a1` is the slot number the chooser wrote onto the kernel's command line, carried here
        // rather than inferred: see `boot_slot::cmdline` for why a running system cannot work it
        // out for itself.
        confirm(a1)
    }
    debug_assert!(role == ROLE_INSTALL);
    // `a1` is the boot file's length for this role, which is the one thing the kernel knows and
    // this program cannot measure: the file is mapped read-only at a fixed address with no length
    // beside it.
    if a1 == 0 {
        send(REPORT, R_NO_BOOT_FILE, 0, 0);
        user_mode_runtime::exit()
    }
    install(a1)
}

/// **Is nife already on this disk?** Read the primary table and look for a partition of the nife
/// data type (DECISIONS §45 (a nife partition is `EC5CC08B-D749-4434-AC38-A274C50385BA`)). Never by
/// name: on any disk a Mac has touched there are no partition
/// names at all.
///
/// A disk this cannot read, or one with no table, is [`R_EMPTY`]: "there is no nife here" is the
/// honest answer to both, and the caller's next step after either is to ask a person.
fn survey() -> ! {
    let size = call(BLK, req(blk::SIZE), 0).0 as i64;
    if size <= 0 {
        send(REPORT, R_EMPTY, 0, 0);
        user_mode_runtime::exit()
    }

    if !read_primary() {
        send(REPORT, R_EMPTY, 0, 0);
        user_mode_runtime::exit()
    }
    let head: &[u8] = primary();
    let Ok(table) = GloballyUniqueIdentifierPartitionTable::parse(
        &head[LBA as usize..2 * LBA as usize],
        &head[2 * LBA as usize..],
    ) else {
        send(REPORT, R_EMPTY, 0, 0);
        user_mode_runtime::exit()
    };
    for (_, part) in table.partitions() {
        if part.type_guid == types::NIFE_DATA {
            send(REPORT, R_ALREADY, part.first_lba, 0);
            user_mode_runtime::exit()
        }
    }
    send(REPORT, R_EMPTY, 0, 0);
    user_mode_runtime::exit()
}

/// **Read LBA 0 through 33 into [`primary()`]**: the protective MBR, the primary header and the
/// whole entry array, which is five transfer blocks. `false` if the disk refused any of them.
///
/// Shared by [`survey`] and [`confirm`], which read the same thing for different reasons, so the
/// window and the staging copy have one spelling.
fn read_primary() -> bool {
    let head = primary();
    for i in 0..5u64 {
        if (call(BLK, req(blk::READ), i).0 as i64) < 0 {
            return false;
        }
        // SAFETY: `BLK_PAGE` is a mapped page of exactly one transfer block, and the destination
        // window is inside `head`, which is five of them.
        unsafe {
            core::ptr::copy_nonoverlapping(
                BLK_PAGE as *const u8,
                head.as_mut_ptr().add(i as usize * blk::BLOCK_SIZE),
                blk::BLOCK_SIZE,
            );
        }
    }
    true
}

/// How many used partition entries [`confirm`] will decode and write back. A table with more than
/// this is read and refused rather than rewritten, so a table this program cannot reproduce
/// exactly is one it never touches. `uefi_loader::chooser`'s own `MAX_PARTITIONS`, and the same
/// reasoning.
const MAX_PARTITIONS: usize = 16;

/// **Say that this boot worked**, by setting `boot_slot::State::successful` on the slot the
/// machine started from.
///
/// # The one number that comes from outside
///
/// `slot` is the index among this disk's `NIFE_BOOT` partitions, in table order, as
/// `uefi_loader`'s chooser counted them and wrote onto the kernel's command line. It is counted
/// the same way here, which is the whole of the agreement between the two: the chooser's
/// `the_disk_with_slots` collects boot-type partitions in index order and so does the loop below.
///
/// **A number this program cannot match is a refusal, never a nearest slot.** Confirming the wrong
/// entry would mark an image good that never ran, which is worse than not confirming at all: the
/// machine would then keep an upgrade that does not work instead of rolling back to one that does.
///
/// # Why it is safe to run twice, and safe to lose power in the middle
///
/// Setting a bit that is already set writes the same bytes, so a second run is a second copy of
/// the same table, and this one returns before writing anything when it finds the slot already
/// successful. That is the *cheap* half of idempotence and it is the one a reader should not rely
/// on: the property that matters is the byte-level one below.
///
/// The bits this changes are nine, inside one `u64`, inside one 128-byte entry, inside one 512-byte
/// logical block. They are never split across two writes, so the entry array on the disk is always
/// either the old entry or the new one and never a third thing. What *can* be interrupted is the
/// four-write sequence that puts the array and the two headers back, and the order below is chosen
/// so that **at every instant at least one complete, self-consistent copy of the table is on the
/// disk**: the backup is finished before the primary is touched. `uefi_loader::chooser`'s
/// `write_back` writes the same four ranges in a different order, which does not have that
/// property; changing it is that lane's and it is named in this program's `BUGS`.
///
/// The worst outcome of an interrupted confirmation is therefore a slot that is not confirmed,
/// which is exactly the state the machine was in a moment earlier and rolls back safely, or a
/// primary table that does not parse beside a backup that does, which is the recovery case
/// `BUGS` says nothing yet reads.
fn confirm(slot: u64) -> ! {
    let size = call(BLK, req(blk::SIZE), 0).0 as i64;
    if size <= 0 || !(size as u64).is_multiple_of(LBA) {
        send(REPORT, R_DISK_FAILED, 1, size as u64);
        user_mode_runtime::exit()
    }
    let block_count = size as u64 / LBA;

    if !read_primary() {
        send(REPORT, R_DISK_FAILED, 2, 0);
        user_mode_runtime::exit()
    }

    // Everything needed to rebuild the table, taken while the parse is still borrowed from the
    // staging buffer: `array()` is a different buffer, but the entries themselves are decoded
    // copies and the disk's identity and geometry come from the table it already has.
    let mut parts = [Entry::UNUSED; MAX_PARTITIONS];
    let mut used = 0usize;
    let mut at = usize::MAX;
    let mut slots = 0u64;
    let disk_guid;
    {
        let head: &[u8] = primary();
        let Ok(table) = GloballyUniqueIdentifierPartitionTable::parse(
            &head[LBA as usize..2 * LBA as usize],
            &head[2 * LBA as usize..],
        ) else {
            send(REPORT, R_DISK_FAILED, 3, 0);
            user_mode_runtime::exit()
        };
        disk_guid = table.disk_guid();
        for (index, part) in table.partitions() {
            // `create` writes the partitions it is given at indices 0, 1, 2..., so a table whose
            // used entries are not contiguous from zero would be renumbered by a rewrite. Ours
            // never are; a disk whose are not is refused rather than silently rearranged. The same
            // check the chooser makes, for the same reason.
            if index != used || used == MAX_PARTITIONS {
                send(REPORT, R_NO_SLOT, slots, 0);
                user_mode_runtime::exit()
            }
            if part.type_guid == types::NIFE_BOOT {
                if slots == slot {
                    at = used;
                }
                slots += 1;
            }
            parts[used] = part;
            used += 1;
        }
    }

    if at == usize::MAX {
        send(REPORT, R_NO_SLOT, slots, slot);
        user_mode_runtime::exit()
    }

    let state = State::from_attributes(parts[at].attributes);
    if state.successful {
        // **Already good, so the disk is not written at all.** An installed machine boots its
        // confirmed slot every day of its life, and a write on every one of those boots is a write
        // that can be interrupted on every one of them. `State::attempted` refuses the same write
        // for the same reason, one stage earlier.
        send(REPORT, R_CONFIRMED, slot, 0);
        user_mode_runtime::exit()
    }
    parts[at].attributes = state.confirmed().into_attributes(parts[at].attributes);

    let array = array();
    let Ok(table) = GloballyUniqueIdentifierPartitionTable::create(
        disk_guid,
        LBA as usize,
        block_count,
        &parts[..used],
        array,
    ) else {
        send(REPORT, R_DISK_FAILED, 4, 0);
        user_mode_runtime::exit()
    };

    let mut block = [0u8; LBA as usize];
    let entry_array: &[u8] = table.entry_array();

    // The backup, complete, before the primary is touched; then the primary, its header last so a
    // half-written primary fails its own CRC rather than pointing at an array it does not
    // describe. The protective MBR is not rewritten: the attribute bits are the only thing
    // changing and the MBR does not describe them.
    if !write_at(table.backup_entry_lba(), entry_array) {
        send(REPORT, R_DISK_FAILED, 5, 0);
        user_mode_runtime::exit()
    }
    block.fill(0);
    if table.write_backup_header(&mut block).is_err()
        || !write_at(table.backup_header_lba(), &block)
    {
        send(REPORT, R_DISK_FAILED, 6, 0);
        user_mode_runtime::exit()
    }
    if !write_at(table.primary_entry_lba(), entry_array) {
        send(REPORT, R_DISK_FAILED, 7, 0);
        user_mode_runtime::exit()
    }
    block.fill(0);
    if table.write_primary_header(&mut block).is_err() || !write_at(PRIMARY_HEADER_LBA, &block) {
        send(REPORT, R_DISK_FAILED, 8, 0);
        user_mode_runtime::exit()
    }

    send(REPORT, R_CONFIRMED, slot, 1);
    user_mode_runtime::exit()
}

/// The whole of it, in the order that makes a refusal cheap: ask the disk its size, lay the
/// arithmetic out, draw the randomness, and only then write.
fn install(boot_file_len: u64) -> ! {
    let size = call(BLK, req(blk::SIZE), 0).0 as i64;
    if size <= 0 || !(size as u64).is_multiple_of(LBA) {
        send(REPORT, R_DISK_FAILED, 1, size as u64);
        user_mode_runtime::exit()
    }
    let block_count = size as u64 / LBA;

    let Some(layout) = Layout::fit(block_count) else {
        send(REPORT, R_TOO_SMALL, size as u64, 0);
        user_mode_runtime::exit()
    };

    // **Every random byte this install needs, drawn together and before the first write**, so that
    // a process with no entropy endpoint finds out while the disk still holds whatever it held.
    // One id for the disk, one per partition, and one serial for the FAT volume.
    let mut guids = [Guid::ZERO; 1 + 2 + SLOTS];
    for g in guids.iter_mut() {
        let Some(bytes) = random16() else {
            send(REPORT, R_NO_ENTROPY, 0, 0);
            user_mode_runtime::exit()
        };
        *g = Guid::v4_from_random(bytes);
    }
    let Some(serial) = random16() else {
        send(REPORT, R_NO_ENTROPY, 0, 0);
        user_mode_runtime::exit()
    };
    let volume_id = u32::from_le_bytes([serial[0], serial[1], serial[2], serial[3]]);

    let Ok(volume) = fat::Volume::new(
        ESP_SECTORS,
        layout.esp_first,
        boot_file_len,
        volume_id,
        BOOT_FILE_NAME,
    ) else {
        send(REPORT, R_DISK_FAILED, 2, boot_file_len);
        user_mode_runtime::exit()
    };

    if let Err(step) = write_table(&layout, block_count, &guids) {
        send(REPORT, R_DISK_FAILED, step, 0);
        user_mode_runtime::exit()
    }
    if let Err(step) = write_efi_system_partition(&layout, &volume, boot_file_len) {
        send(REPORT, R_DISK_FAILED, step, 0);
        user_mode_runtime::exit()
    }

    // **Boot slot 0**, which is the same bytes a third time and is what the chooser will start from
    // the next boot onward. The table above already marked it confirmed.
    if let Err(step) = write_slot(layout.slot_first[0], boot_file_len) {
        send(REPORT, R_DISK_FAILED, step, 0);
        user_mode_runtime::exit()
    }

    // One flush, so that what a reboot reads is what this program wrote rather than what a
    // controller still has in a cache.
    let _ = call(BLK, req(blk::FLUSH), 0);

    send(REPORT, R_INSTALLED, layout.data_first, layout.esp_first);
    user_mode_runtime::exit()
}

/// **Where the four partitions go**, in logical blocks, inclusive at both ends the way a GPT entry
/// records them.
struct Layout {
    data_first: u64,
    data_last: u64,
    /// The first block of each boot slot, in disk order. The image inside one starts
    /// [`boot_slot::SLOT_IMAGE_OFFSET`] bytes further in; the blocks before it are the slot header.
    slot_first: [u64; SLOTS],
    slot_last: [u64; SLOTS],
    esp_first: u64,
    esp_last: u64,
}

impl Layout {
    /// Fit the layout onto a disk of `block_count` logical blocks, or answer `None` if it will not
    /// fit. **All of the arithmetic, in one place**, so the writes below have nothing to decide.
    ///
    /// The EFI system partition goes at the end, the boot slots just below it, and the data
    /// partition takes everything before them, for the reason this file's header gives.
    fn fit(block_count: u64) -> Option<Layout> {
        // The last block a partition may use: the backup entry array and its header sit above it.
        let last_usable = block_count.checked_sub(BACKUP_BLOCKS + 1)?;

        // The EFI system partition, pushed down to an alignment boundary. Its length then grows by
        // whatever the rounding left over, which is harmless: FAT is told how many sectors it has.
        let esp_first = (last_usable + 1).checked_sub(ESP_SECTORS)? / ALIGN * ALIGN;

        // The slots, stacked immediately below it. `SLOT_SECTORS` is a whole number of `ALIGN`
        // units, so each one lands aligned without a second rounding step.
        let mut slot_first = [0u64; SLOTS];
        let mut slot_last = [0u64; SLOTS];
        let mut above = esp_first;
        for i in (0..SLOTS).rev() {
            slot_first[i] = above.checked_sub(SLOT_SECTORS)?;
            slot_last[i] = above - 1;
            above = slot_first[i];
        }

        if above < ALIGN + DATA_MIN_SECTORS {
            return None;
        }
        let data_first = ALIGN;
        let data_last = above - 1;

        // `mkfs` refuses a partition that is not 4096-aligned at both ends rather than rounding one
        // in, so this must not hand it one. Both bounds are multiples of `ALIGN`, which is a
        // multiple of eight sectors, so this cannot fail; it is checked anyway, because the day it
        // can is the day somebody changes `ALIGN`.
        if !data_first.is_multiple_of(SECTORS_PER_TRANSFER)
            || !(data_last + 1).is_multiple_of(SECTORS_PER_TRANSFER)
        {
            return None;
        }

        Some(Layout {
            data_first,
            data_last,
            slot_first,
            slot_last,
            esp_first,
            esp_last: last_usable,
        })
    }
}

/// Write both copies of the partition table. `Err(step)` names which write failed.
///
/// **The slot entries carry their boot state in the attribute word**, which is the one thing here a
/// reader should not skim: slot 0 is written confirmed because the bytes about to go into it are
/// the bytes the firmware started this machine with a few seconds ago, and slot 1 is written empty
/// so that an upgrade finds a partition rather than having to make one.
fn write_table(
    layout: &Layout,
    block_count: u64,
    guids: &[Guid; 1 + 2 + SLOTS],
) -> Result<(), u64> {
    let mut entries = [(types::UNUSED, 0u64, 0u64, "", 0u64); 2 + SLOTS];
    entries[0] = (
        types::NIFE_DATA,
        layout.data_first,
        layout.data_last,
        NAMES[0],
        0,
    );
    for i in 0..SLOTS {
        // Slot 0 holds the image this install is about to copy; every other slot is laid out and
        // will not be chosen until something writes an image into it.
        let state = if i == 0 {
            State::installed()
        } else {
            State::EMPTY
        };
        entries[1 + i] = (
            types::NIFE_BOOT,
            layout.slot_first[i],
            layout.slot_last[i],
            NAMES[1 + i],
            state.into_attributes(0),
        );
    }
    entries[1 + SLOTS] = (
        types::EFI_SYSTEM,
        layout.esp_first,
        layout.esp_last,
        NAMES[1 + SLOTS],
        0,
    );

    let mut parts = [Entry::UNUSED; 2 + SLOTS];
    for (i, (type_guid, first, last, name, attributes)) in entries.iter().enumerate() {
        parts[i] = Entry::new(*type_guid, guids[i + 1], *first, *last)
            .with_name(name)
            .map_err(|_| 10 + i as u64)?;
        parts[i].attributes = *attributes;
    }

    let array = array();
    let table = GloballyUniqueIdentifierPartitionTable::create(
        guids[0],
        LBA as usize,
        block_count,
        &parts,
        array,
    )
    .map_err(|_| 12u64)?;

    // One logical block of scratch. Built and written one at a time: the protective MBR, the two
    // headers and the entry array are each at a known LBA.
    let mut block = [0u8; LBA as usize];
    let entry_array: &[u8] = table.entry_array();
    let steps: [(u64, Option<&[u8]>); 5] = [
        (0, None),
        (PRIMARY_HEADER_LBA, None),
        (table.primary_entry_lba(), Some(entry_array)),
        (table.backup_entry_lba(), Some(entry_array)),
        (table.backup_header_lba(), None),
    ];
    for (n, (lba, bytes)) in steps.iter().enumerate() {
        let data: &[u8] = match bytes {
            Some(b) => b,
            None => {
                block.fill(0);
                let built = match n {
                    0 => table.write_protective_mbr(&mut block),
                    1 => table.write_primary_header(&mut block),
                    _ => table.write_backup_header(&mut block),
                };
                built.map_err(|_| 20 + n as u64)?;
                &block
            }
        };
        if !write_at(*lba, data) {
            return Err(30 + n as u64);
        }
    }
    Ok(())
}

/// **Lay the FAT32 volume down and copy the boot file into it.**
///
/// Written in whole transfer blocks throughout, which is what the alignment above buys: the EFI
/// system partition begins on a multiple of [`ALIGN`], so a partition-relative sector `i` and the
/// device's own transfer blocks line up with no read-modify-write anywhere.
fn write_efi_system_partition(
    layout: &Layout,
    volume: &fat::Volume,
    boot_file_len: u64,
) -> Result<(), u64> {
    // The metadata: the boot sector, `FSInfo`, their backups, both copies of the table, and the
    // three directory clusters. Eight sectors are assembled in the shared page and written as one
    // request.
    let metadata = volume.metadata_sectors();
    let mut sector = 0u64;
    while sector < metadata {
        for slot in 0..SECTORS_PER_TRANSFER {
            // SAFETY: `BLK_PAGE` is a mapped, writable page of exactly one transfer block, and
            // `slot` is below the number of sectors in one.
            let out = unsafe {
                core::slice::from_raw_parts_mut(
                    (BLK_PAGE as *mut u8).add(slot as usize * LBA as usize),
                    LBA as usize,
                )
            };
            // Past the end of the metadata the volume answers `false` and the sector stays zero,
            // which is what belongs there: it is the first of the file's own clusters and is about
            // to be overwritten.
            if !volume.sector(sector + slot, out) {
                out.fill(0);
            }
        }
        let block = (layout.esp_first + sector) / SECTORS_PER_TRANSFER;
        if (call(BLK, req(blk::WRITE), block).0 as i64) < 0 {
            return Err(40);
        }
        sector += SECTORS_PER_TRANSFER;
    }

    // The boot file itself, contiguous from the cluster the volume's directory entry names.
    //
    // **Refuse a volume whose clusters do not land on transfer blocks**, rather than dividing and
    // silently writing somewhere else. `file_allocation_table` aligns its data area for exactly
    // this reason, and the run that made it do so wrote ten megabytes six sectors early onto a real
    // disk and produced a file the firmware would not start. This is the check that would have said
    // so in one line.
    let first = layout.esp_first + volume.file_first_sector();
    if !first.is_multiple_of(SECTORS_PER_TRANSFER) {
        return Err(42);
    }
    copy_boot_file(first / SECTORS_PER_TRANSFER, boot_file_len, 41)
}

/// **Fill one boot slot**: its header, then the image, starting at logical block `slot_first`.
///
/// The header is what tells the chooser how many of the slot's 64 MiB are an image and whether
/// those bytes are the ones that were meant to be there. Its checksum is not ceremony: the failure
/// it catches is an install or an upgrade that died partway through this very copy, which without
/// it leaves a slot holding a valid-looking prefix of a boot image.
///
/// `crates/boot_slot` reserves a whole 4096-byte transfer block for the header so the image after
/// it starts on a block boundary. Milestone 198's rung 2a lost ten megabytes to an unaligned start
/// once (`crates/file_allocation_table`'s own notes) and that is the reason the padding is there.
fn write_slot(slot_first: u64, boot_file_len: u64) -> Result<(), u64> {
    // SAFETY: the kernel mapped `boot_file_len` bytes read-only at `BOOT_FILE_VA`; this program
    // has one thread, and nothing else in it writes there.
    let image =
        unsafe { core::slice::from_raw_parts(BOOT_FILE_VA as *const u8, boot_file_len as usize) };

    // SAFETY: `BLK_PAGE` is a mapped, writable page of exactly one transfer block.
    let page = unsafe { core::slice::from_raw_parts_mut(BLK_PAGE as *mut u8, TRANSFER as usize) };
    page.fill(0);
    if !SlotHeader::of(image).encode(page) {
        return Err(50);
    }
    if !slot_first.is_multiple_of(SECTORS_PER_TRANSFER) {
        return Err(51);
    }
    let header_block = slot_first / SECTORS_PER_TRANSFER;
    if (call(BLK, req(blk::WRITE), header_block).0 as i64) < 0 {
        return Err(52);
    }

    copy_boot_file(
        header_block + boot_slot::SLOT_IMAGE_OFFSET / TRANSFER,
        boot_file_len,
        53,
    )
}

/// Copy the mapped boot file onto the disk, one transfer block per request, starting at transfer
/// block `first_block`. `Err(step)` is the caller's own name for a refused write.
///
/// The one subtlety is the tail: the last block is short, and the bytes past the end are zeroed
/// rather than left as whatever the shared page held, because otherwise somebody else's data goes
/// onto the disk.
fn copy_boot_file(first_block: u64, boot_file_len: u64, step: u64) -> Result<(), u64> {
    for i in 0..boot_file_len.div_ceil(TRANSFER) {
        let at = i * TRANSFER;
        let n = core::cmp::min(TRANSFER, boot_file_len - at) as usize;
        // SAFETY: the kernel mapped `boot_file_len` bytes read-only at `BOOT_FILE_VA`, and
        // `at + n` is bounded by that length; `BLK_PAGE` is one mapped writable transfer block.
        unsafe {
            core::ptr::copy_nonoverlapping(
                (BOOT_FILE_VA as *const u8).add(at as usize),
                BLK_PAGE as *mut u8,
                n,
            );
            if n < TRANSFER as usize {
                core::ptr::write_bytes((BLK_PAGE as *mut u8).add(n), 0, TRANSFER as usize - n);
            }
        }
        if (call(BLK, req(blk::WRITE), first_block + i).0 as i64) < 0 {
            return Err(step);
        }
    }
    Ok(())
}

/// Sixteen random bytes from the entropy service, or `None` if this process holds no entropy
/// endpoint. Two round trips, because a reply carries one word; `entropy_protocol::delivered` is
/// what separates "the service answered with n bytes" from "the kernel refused the call".
fn random16() -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    for half in 0..2 {
        let (r0, r1) = call(ENTROPY, entropy::req(entropy::GET, entropy::MAX_BYTES), 0);
        let n = entropy::delivered(r0)?;
        if n != entropy::MAX_BYTES as usize {
            return None;
        }
        entropy::take(n, r1, &mut out[half * 8..half * 8 + 8]);
    }
    Some(out)
}

/// Lay `data` over the disk starting at logical block `first_lba`, **preserving everything else in
/// the transfer blocks it touches**: each is read, patched in the shared page, and written back.
///
/// The partition table is the one thing here written this way, and it has to be: the primary table
/// is 34 logical blocks and the transfer unit is eight of them, so the tail of the last transfer
/// block is the first three kilobytes of the partition the table is only supposed to be
/// *describing*.
fn write_at(first_lba: u64, data: &[u8]) -> bool {
    let mut done = 0usize;
    while done < data.len() {
        let byte_at = first_lba * LBA + done as u64;
        let block = byte_at / TRANSFER;
        let offset = (byte_at % TRANSFER) as usize;
        let n = core::cmp::min(blk::BLOCK_SIZE - offset, data.len() - done);
        if (call(BLK, req(blk::READ), block).0 as i64) < 0 {
            return false;
        }
        // SAFETY: `BLK_PAGE` is a mapped, writable page of exactly one transfer block, and
        // `offset + n` is bounded by that block above.
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr().add(done),
                (BLK_PAGE as *mut u8).add(offset),
                n,
            );
        }
        if (call(BLK, req(blk::WRITE), block).0 as i64) < 0 {
            return false;
        }
        done += n;
    }
    true
}

/// The entry-array buffer.
///
/// # Safety
/// One thread per address space here (DECISIONS §33 (the compositor's authority is memory, not
/// messages)), so there is no second reference. Taking the
/// raw pointer first is what `static_mut_refs` asks for.
fn array() -> &'static mut [u8] {
    let p = &raw mut ARRAY;
    // SAFETY: see above.
    unsafe { &mut *p }
}

/// [`survey`]'s read buffer. Same reasoning as [`array()`].
fn primary() -> &'static mut [u8] {
    let p = &raw mut PRIMARY;
    // SAFETY: see above.
    unsafe { &mut *p }
}

user_mode_runtime::panic_handler!();
