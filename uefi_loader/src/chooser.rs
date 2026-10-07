//! **The chooser**: the half of this loader that decides *which* image boots, so that a bad
//! upgrade cannot brick the machine (rung 2b of milestone 198 (a package manager, and the trivial install that makes a second customer possible); calef ruled it on 2026-09-21).
//!
//! An installed nife machine keeps two copies of its boot image in two partitions of its own type
//! (`crates/boot_slot`). The file the firmware starts, at `\EFI\BOOT\BOOTX64.EFI`, is a third copy,
//! and its extra job is this module: read the slot state out of the GPT, pick a slot, **spend one
//! of that slot's tries and write it to the disk**, and chain-load the image in it. If an upgrade
//! never comes up, its tries run out and the machine returns to the previous image on its own, with
//! nobody at the console.
//!
//! # Why the chooser is here and not in the firmware
//!
//! Because it cannot be anywhere else. UEFI 2.11 section 5.3.3 reserves GPT attribute bits 48 to 63 for
//! the owner of the partition's type GUID and tells everyone else to leave them alone, so OVMF and
//! ordinary firmware do not read them and were never going to. ChromeOS, which is where this design
//! comes from, gets firmware-level selection because depthcharge is a coreboot payload that
//! *replaces* UEFI. We have UEFI, so **the first code of ours that runs is the selector**, and the
//! first code of ours that runs is this file.
//!
//! `crates/boot_slot` carries the policy, the bit layout and the refusal of the `Boot####` variable
//! design; what is here is the firmware work.
//!
//! # The shape, and the one thing that keeps it from recursing
//!
//! The chooser and the image it starts are **the same binary**. That is the elegant half: there is
//! no second program to build, keep in step, or seal against the kernel, and a machine whose slots
//! are all unbootable still has a complete, known-good nife image to fall back to, namely the one
//! doing the choosing.
//!
//! What stops it chain-loading itself forever is [`STARTED_BY_CHOOSER`]: the chooser writes that
//! marker and a slot number into the child's `LoadOptions`, and an image that finds it there knows
//! it is the one being tried and goes straight on to boot its own kernel.
//!
//! # What it does, in order
//!
//! 1. Find the disk **this file is on**, and read the boot slots in its GPT. Slots on any other disk
//!    are another system's and are never started or written: a stick booted on an installed machine
//!    boots the stick (the live stick proposal's G1, measured in boot 3 of `cargo xtask
//!    install-boot`). Which disk is "this one" is a device-path prefix, `uefi_loader::device_path`.
//! 2. Pick the bootable slot of highest priority ([`boot_slot::select_excluding`]).
//! 3. **Spend a try and write the table back, flushed, before anything else.** This is the crux of
//!    the design rather than an ordering detail; the next section is about it.
//! 4. Read the slot header and the image, and check the image against the header's checksum.
//! 5. `LoadImage` from that buffer, `StartImage`, and never come back.
//!
//! A slot the chooser cannot read at all is given up on immediately
//! ([`boot_slot::State::unreadable`]) and the next slot is tried **in the same boot**, because
//! making somebody reboot to recover from a failure the chooser watched happen would be a worse
//! machine for no gain.
//!
//! # Which failures this catches, and which it does not
//!
//! The honest table, because a rollback mechanism that is vague about this is worse than none:
//!
//! | failure | caught | by what |
//! |---|---|---|
//! | the slot was never written, or its header is corrupt | yes, in the same boot | the header's own CRC |
//! | the copy into the slot died partway | yes, in the same boot | the image CRC in the header |
//! | the image is not a PE the firmware will start | yes, in the same boot | `LoadImage` refuses, the next slot is tried |
//! | the image starts and returns an error | yes, in the same boot | `StartImage` returns, the next slot is tried |
//! | **the image starts and hangs**, before or after `ExitBootServices` | yes, on the **next** boot | the try spent at step 3, which is already on the disk |
//! | the image comes up but is subtly wrong | **no** | nothing here can tell, and nothing will |
//! | the machine loses power during step 3 | **no** | see `BUGS` |
//! | every slot is bad | the machine still boots | the image in this file is started instead |
//!
//! The row in bold is the reason step 3 is where it is. A chooser that only read the state, leaving
//! the booted system to mark itself good, would handle every row above it and none of that one: a
//! machine that wedges before userspace would retry the same bad image forever and show nothing to
//! the console nobody is standing at. ChromeOS's firmware decrements before the launch for exactly
//! this reason.
//!
//! # EXAMPLES
//!
//! An ordinary boot of an installed machine, where slot 0 was confirmed at install time and
//! therefore spends nothing:
//!
//! ```text
//! nife uefi_loader: milestone 87 (the x86_64 bare-metal machine)
//! uefi_loader: boot slots on one disk, 2 of them
//! uefi_loader: starting boot slot 0
//! nife uefi_loader: milestone 87 (the x86_64 bare-metal machine)
//! uefi_loader: started from boot slot 0
//! ```
//!
//! The boot after an upgrade that hung, where nobody did anything:
//!
//! ```text
//! uefi_loader: boot slots on one disk, 2 of them
//! uefi_loader: boot slot 1 has no tries left
//! uefi_loader: starting boot slot 0
//! ```
//!
//! # BUGS
//!
//! - **It sees only the disks the firmware connected.** Started from a boot option naming only the
//!   stick, OVMF connects the stick alone and this loader sees no other disk at all (measured
//!   2026-10-04: two `BlockIo` handles). That is harmless now that only this file's own disk counts,
//!   and it is why `install-boot`'s third boot names the disk as a second boot option.
//! - **The logical block size is assumed to be 512.** A disk reporting anything else is skipped
//!   rather than misread, which is the honest half; the GPT crate reads other sizes and nothing
//!   here passes the size through. `system_installer` has the same assumption and the same note.
//! - **A power cut during the write at step 3 is not survivable by this design.** The table is
//!   rewritten in four block ranges and there is no journal; a machine interrupted between them can
//!   come up with a primary and backup table that disagree, which this loader's own `parse` will
//!   then refuse, and the machine falls back to the image in the chooser's file. That is a safe
//!   outcome rather than a good one, and it is untested: nothing in this tree cuts power to a QEMU
//!   between two block writes.
//! - **This chooser's four block writes go primary-first**, so there is a window in which neither
//!   copy of the table parses: the primary's array is new while its header still describes the old
//!   one, and the backup has not been touched. `system_installer`'s `ROLE_CONFIRM` writes the same four
//!   ranges **backup-first**, which leaves one complete self-consistent copy on the disk at every
//!   instant and costs nothing. Making this one agree is a reordering of four lines and belongs to
//!   whoever next touches this function; it is recorded here rather than done because the
//!   interrupted case is untested either way (see the entry above) and changing an untested
//!   failure path on the way past is how a rollback stops working.
//! - **Nothing here reads the backup table when the primary fails to parse.** With the reordering
//!   above that would be a real recovery; without it, it is what lets a foreign tool repair a
//!   machine this loader has given up on.
//! - **An image started from a slot has no boot file**, because `LoadImage` from a buffer leaves
//!   the child's `DeviceHandle` null and there is no volume to read a file back from. So an
//!   installed machine cannot install itself onto a second disk; only a machine booted from the
//!   stick or from the chooser's own file can. That is a narrowing rather than a loss; the fix
//!   would hand the child the slot's device path, which nothing here builds yet.
//! - **A slot-started image says it came from the NVMe disk without looking**, in
//!   [`booted_from_nvme`], because it has no device handle to look with. That holds while the
//!   installer writes slots only to NVMe (`system_installer`'s whole-disk, NVMe-only scope). A slot on
//!   another kind of disk would have its kernel mount the NVMe one; passing the medium in the
//!   child's load options is the fix, and nothing needs it yet.
//! - **x86_64 only**, the same scope as the rest of rung 2a and 2b (DECISIONS §19 (architectural parity is a tenet; the targets are aarch64, riscv64 and x86_64)): the
//!   device-tree architectures have no second module slot for a boot file, which
//!   `design/roadmap/568-the-boot-file-has-nowhere-to-go-on-a-device-tree-machine.md` prices.
//!   Nothing about the slot format is x86-specific; the chooser is what has not been built there.

use core::ffi::c_void;
use core::ptr;

use boot_slot::{SlotHeader, State, select_excluding};
use globally_unique_identifier_partition_table as gpt;
use gpt::guid::types;
use gpt::{Entry, GloballyUniqueIdentifierPartitionTable as Table};
use uefi_loader::device_path;
use uefi_loader::efi::{
    BLOCK_IO_PROTOCOL_GUID, BY_PROTOCOL, BlockIo, BlockIoMedia, BootServices,
    DEVICE_PATH_PROTOCOL_GUID, Handle, LOADED_IMAGE_PROTOCOL_GUID, LoadedImage, SUCCESS,
    SystemTable, memory_type,
};
use universally_unique_identifier::Uuid;

use crate::{BOOT_FILE_MAX, PAGE, allocate_below, say, say_decimal};

/// **The marker the chooser puts in a chain-loaded image's `LoadOptions`**, followed by one ASCII
/// digit naming the slot.
///
/// It is the one thing that stops a chooser starting itself forever, and it is deliberately plain
/// ASCII rather than the UTF-16 a UEFI shell would pass: these bytes are a contract between this
/// file and itself, and a reader running `strings` on the image should meet them without decoding.
///
/// Provisional, like everything else about this format; calef names what a reader meets.
pub const STARTED_BY_CHOOSER: &[u8] = b"nife-slot=";

/// How many boot slots the chooser will consider on one disk. Two are laid out; the policy does not
/// care and this is only the size of the arrays below.
const MAX_SLOTS: usize = 4;

/// How many partition entries the chooser will decode and write back. A table with more used
/// entries than this is read but never rewritten, so a try is never silently unspent.
const MAX_PARTITIONS: usize = 16;

/// The logical block size this chooser reads and writes. See `BUGS`.
const LBA: usize = 512;

/// Blocks of the primary table: the protective MBR, the header, and the 32-block entry array.
const PRIMARY_BLOCKS: usize = 34;

/// A buffer the firmware's block driver will accept: `BlockIo` requires the caller's buffer to meet
/// the medium's `IoAlign`, and a page is more alignment than any disk asks for.
#[repr(C, align(4096))]
struct Aligned<const N: usize>([u8; N]);

/// LBA 0 through 33 of the candidate disk, read once and then used as the staging buffer for the
/// rewrite. Static rather than stack: 17 KiB, and this runs before the kernel exists.
static mut PRIMARY: Aligned<{ PRIMARY_BLOCKS * LBA }> = Aligned([0; PRIMARY_BLOCKS * LBA]);

/// One block of scratch for the backup header, which is the only block written on its own.
static mut BLOCK: Aligned<LBA> = Aligned([0; LBA]);

/// The `LoadOptions` handed to a chain-loaded image: [`STARTED_BY_CHOOSER`] and a digit.
static mut OPTIONS: [u8; 16] = [0; 16];

/// **Which boot slot started this image, if a chooser did.**
///
/// `None` means the firmware started this file directly, which is the stick, the first boot after
/// an install, and every `-kernel` boot. The caller uses it twice: to know not to act as a chooser,
/// and to know that there is no boot file to read back.
pub fn started_from_slot(handle: Handle, services: &BootServices) -> Option<u8> {
    let loaded = loaded_image(handle, services)?;
    if loaded.load_options.is_null() {
        return None;
    }
    let len = loaded.load_options_size as usize;
    if len <= STARTED_BY_CHOOSER.len() {
        return None;
    }
    // SAFETY: the firmware reports `load_options_size` bytes at `load_options`, and they were
    // written by this loader's own chooser in the parent image, which outlives the child.
    let options = unsafe { core::slice::from_raw_parts(loaded.load_options.cast::<u8>(), len) };
    if !options.starts_with(STARTED_BY_CHOOSER) {
        return None;
    }
    let digit = options[STARTED_BY_CHOOSER.len()];
    digit.is_ascii_digit().then(|| digit - b'0')
}

/// **Choose a boot slot and start the image in it.**
///
/// Returns only when nothing was started, which is the caller's signal to boot the kernel embedded
/// in this file. Every refusal says why on the console first: this runs while there is still a
/// firmware console, and a machine that silently did something other than what its disk asked for
/// is the worst outcome available here.
pub fn choose(handle: Handle, table: &SystemTable, services: &BootServices) {
    // **Where this file came from.** Without it there is no telling a stick from the installed
    // disk, and the answer to not knowing is the image in hand: a stick that boots the disk's
    // system is the defect this is here to prevent (the live stick proposal's G1).
    let Some(own) = loaded_image(handle, services).and_then(|l| path_of(l.device_handle, services))
    else {
        say(
            table,
            "uefi_loader: the firmware did not say which disk this file is on; using the image in this file\r\n",
        );
        return;
    };
    let Some(mut disk) = the_disk_with_slots(table, services, own) else {
        // Not an installed machine, or a stick on one. Both are ordinary.
        return;
    };

    say(table, "uefi_loader: boot slots on one disk, ");
    say_decimal(table, disk.slots as u32);
    say(table, " of them\r\n");

    let mut tried = 0u64;
    while let Some(slot) = select_excluding(&disk.states[..disk.slots], tried) {
        // **The try is spent here, before anything is read and long before anything is started.**
        // Everything below this line can hang the machine, and the whole point of the design is
        // that a hang costs a try anyway.
        let spent = disk.states[slot].attempted();
        if spent != disk.states[slot] {
            disk.states[slot] = spent;
            if !disk.write_back(table, services, slot) {
                say(table, "uefi_loader: the disk refused the boot state; ");
                say(
                    table,
                    "not starting a slot whose try could not be spent\r\n",
                );
                break;
            }
        }

        tried |= 1 << slot;
        match disk.start(handle, table, services, slot) {
            Started::Unreadable => {
                // A failure the chooser watched happen. Give up on the slot now rather than over
                // three reboots of a machine somebody is waiting on.
                disk.states[slot] = disk.states[slot].unreadable();
                let _ = disk.write_back(table, services, slot);
            }
            Started::Refused => {}
        }
    }

    say(
        table,
        "uefi_loader: no boot slot would start; using the image in this file\r\n",
    );
}

/// Why a slot did not boot. There is no success arm: a slot that starts never comes back.
enum Started {
    /// `StartImage` was never reached: the header, the checksum or `LoadImage` said no.
    Unreadable,
    /// The image started and returned, which for a nife boot image means it failed with the
    /// firmware console still up and has already said why.
    Refused,
}

/// The candidate disk, its table, and the slot state read off it.
struct Disk {
    block_io: *mut BlockIo,
    media_id: u32,
    disk_guid: Uuid,
    block_count: u64,
    /// Every used entry, in index order, so a rewrite puts them back where they were.
    parts: [Entry; MAX_PARTITIONS],
    used: usize,
    /// For each slot, its index in [`Self::parts`].
    at: [usize; MAX_SLOTS],
    states: [State; MAX_SLOTS],
    slots: usize,
}

/// **Find the disk this file is on, if its GPT carries boot slots.**
///
/// `own` is the device path of the medium this file was loaded from, without its end node. A disk
/// carrying slots that is not that medium is another machine's system as far as this boot is
/// concerned (an installed disk under a stick), and is said out loud and left alone: not read past
/// its table, and never written.
fn the_disk_with_slots(table: &SystemTable, services: &BootServices, own: &[u8]) -> Option<Disk> {
    let mut count = 0usize;
    let mut handles: *mut Handle = ptr::null_mut();
    if (services.locate_handle_buffer)(
        BY_PROTOCOL,
        &BLOCK_IO_PROTOCOL_GUID,
        ptr::null_mut(),
        &mut count,
        &mut handles,
    ) != SUCCESS
        || handles.is_null()
    {
        return None;
    }
    // SAFETY: the firmware answered with `count` handles in a pool buffer it owns, and it stays
    // valid until boot services end.
    let handles = unsafe { core::slice::from_raw_parts(handles, count) };

    let mut elsewhere = false;
    for &h in handles {
        let Some(disk) = read_table(h, services) else {
            continue;
        };
        if path_of(h, services).is_some_and(|path| device_path::is_on(own, path)) {
            // One file is on one disk, so the first match is the only one.
            return Some(disk);
        }
        elsewhere = true;
    }
    if elsewhere {
        say(
            table,
            "uefi_loader: the boot slots here are on another disk; this file's medium has none, \
             so it is using the image in this file\r\n",
        );
    }
    None
}

/// **Did this boot come from the NVMe disk?** The answer the loader writes as
/// `boot_slot::medium::NVME`, and the only case in which the kernel mounts that disk (the live stick
/// proposal's G2; calef ruled the token on PR #1652, 2026-10-04 UTC).
///
/// An image a chooser started has no device handle of its own (`LoadImage` from a buffer), so for
/// it the answer is the chooser's: since G1 a chooser starts only slots on its own disk, and the
/// installer writes slots only to an NVMe disk. See `BUGS`.
pub fn booted_from_nvme(handle: Handle, services: &BootServices, from_slot: Option<u8>) -> bool {
    if from_slot.is_some() {
        return true;
    }
    loaded_image(handle, services)
        .and_then(|l| path_of(l.device_handle, services))
        .is_some_and(device_path::is_nvme)
}

/// **A handle's device path, without its end node**, or `None` when the firmware has none for it or
/// it does not parse. See `uefi_loader::device_path`.
fn path_of(handle: Handle, services: &BootServices) -> Option<&'static [u8]> {
    if handle.is_null() {
        return None;
    }
    let mut interface: *mut c_void = ptr::null_mut();
    if (services.handle_protocol)(handle, &DEVICE_PATH_PROTOCOL_GUID, &mut interface) != SUCCESS
        || interface.is_null()
    {
        return None;
    }
    let base = interface.cast::<u8>().cast_const();
    let len = device_path::length(|at| {
        // SAFETY: `length` asks only for the header at the start of a node it has reached by
        // adding the lengths of the nodes before it, none of them past the end node, and stops
        // within `MAX_LEN`; the firmware's path is at least that long up to its end node.
        Some(unsafe { ptr::read_unaligned(base.add(at).cast::<[u8; 4]>()) })
    })?;
    // SAFETY: `length` walked these `len` bytes node by node; the firmware owns them until boot
    // services end, which is after the chooser is done with them.
    Some(unsafe { core::slice::from_raw_parts(base, len) })
}

/// Read one handle's table, and answer `Some` only if it is a whole disk carrying boot slots.
fn read_table(handle: Handle, services: &BootServices) -> Option<Disk> {
    let mut interface: *mut c_void = ptr::null_mut();
    if (services.handle_protocol)(handle, &BLOCK_IO_PROTOCOL_GUID, &mut interface) != SUCCESS
        || interface.is_null()
    {
        return None;
    }
    let block_io = interface.cast::<BlockIo>();
    // SAFETY: the firmware answered with its own `EFI_BLOCK_IO_PROTOCOL` for this handle, and the
    // `media` pointer inside it is the firmware's and outlives boot services.
    let media = unsafe { media(block_io) }?;
    let media_id = media.media_id;
    // **Whole disks only.** A GPT lives at LBA 1 of a disk; LBA 1 of a partition is somebody's
    // filesystem, and every partition on this machine also carries a `BlockIo`.
    if media.logical_partition != 0 || media.media_present == 0 || media.block_size as usize != LBA
    {
        return None;
    }
    if media.last_block < PRIMARY_BLOCKS as u64 {
        return None;
    }

    let head = primary();
    // SAFETY: `block_io` is the firmware's protocol, `head` is page-aligned and exactly
    // `PRIMARY_BLOCKS` blocks long, and the disk has at least that many.
    if unsafe {
        ((*block_io).read_blocks)(block_io, media_id, 0, head.len(), head.as_mut_ptr()) != SUCCESS
    } {
        return None;
    }

    let head: &[u8] = head;
    let table = Table::parse(&head[LBA..2 * LBA], &head[2 * LBA..]).ok()?;

    let mut disk = Disk {
        block_io,
        media_id,
        disk_guid: table.disk_guid(),
        block_count: table.block_count(),
        parts: [Entry::UNUSED; MAX_PARTITIONS],
        used: 0,
        at: [0; MAX_SLOTS],
        states: [State::EMPTY; MAX_SLOTS],
        slots: 0,
    };
    for (index, part) in table.partitions() {
        // `create` writes the partitions it is given at indices 0, 1, 2..., so a table whose used
        // entries are not contiguous from zero would be renumbered by a rewrite. Ours never are;
        // refusing is what keeps that an assumption this code states rather than one it holds.
        if index != disk.used || disk.used == MAX_PARTITIONS {
            return None;
        }
        disk.parts[disk.used] = part;
        if part.type_guid == types::NIFE_BOOT && disk.slots < MAX_SLOTS {
            disk.at[disk.slots] = disk.used;
            disk.states[disk.slots] = State::from_attributes(part.attributes);
            disk.slots += 1;
        }
        disk.used += 1;
    }
    (disk.slots > 0).then_some(disk)
}

impl Disk {
    /// **Write one slot's boot state back to both copies of the table**, and flush, because a try
    /// this loader has spent and a controller has not written down is a try nothing spent.
    ///
    /// `false` if any part of it was refused, and the caller's answer to that is not to start the
    /// slot: an image tried without its try being recorded is an image that will be tried forever.
    fn write_back(&mut self, table: &SystemTable, services: &BootServices, slot: usize) -> bool {
        let at = self.at[slot];
        self.parts[at].attributes = self.states[slot].into_attributes(self.parts[at].attributes);

        let (head, array) = primary().split_at_mut(2 * LBA);
        let Ok(built) = Table::create(
            self.disk_guid,
            LBA,
            self.block_count,
            &self.parts[..self.used],
            array,
        ) else {
            say(
                table,
                "uefi_loader: could not rebuild the partition table\r\n",
            );
            return false;
        };

        // `head[..LBA]` is still the protective MBR as it was read, and it is not rewritten: the
        // attribute bits are the only thing changing and the MBR does not describe them.
        let header_block = &mut head[LBA..];
        if built.write_primary_header(header_block).is_err() {
            return false;
        }
        let backup_entry_lba = built.backup_entry_lba();
        let backup_header_lba = built.backup_header_lba();

        let backup_header = one_block();
        backup_header.fill(0);
        if built.write_backup_header(backup_header).is_err() {
            return false;
        }

        let _ = services;
        // Four ranges and no journal; see BUGS. The primary header goes last of the two primary
        // writes so that a table whose array landed and whose header did not fails its own CRC
        // rather than pointing at an array that is not the one it describes.
        self.write(built.primary_entry_lba(), built.entry_array())
            && self.write(backup_entry_lba, built.entry_array())
            && self.write(backup_header_lba, backup_header)
            && self.write(gpt::PRIMARY_HEADER_LBA, header_block)
            && self.flush()
    }

    /// One `WriteBlocks`, with the caller's buffer already a whole number of blocks.
    fn write(&self, lba: u64, bytes: &[u8]) -> bool {
        // SAFETY: `block_io` is the firmware's protocol for this disk; `bytes` lives in the
        // page-aligned statics above and its length is a whole number of blocks.
        unsafe {
            ((*self.block_io).write_blocks)(
                self.block_io,
                self.media_id,
                lba,
                bytes.len(),
                bytes.as_ptr(),
            ) == SUCCESS
        }
    }

    /// `FlushBlocks`, whose absence is the difference between a recorded try and a remembered one.
    fn flush(&self) -> bool {
        // SAFETY: as above.
        unsafe { ((*self.block_io).flush_blocks)(self.block_io) == SUCCESS }
    }

    /// Read the slot's header and image, and start it. Returns only on failure.
    fn start(
        &self,
        handle: Handle,
        table: &SystemTable,
        services: &BootServices,
        slot: usize,
    ) -> Started {
        let first = self.parts[self.at[slot]].first_lba;
        let block = one_block();
        // SAFETY: `block_io` is the firmware's protocol and `block` is one page-aligned block.
        if unsafe {
            ((*self.block_io).read_blocks)(
                self.block_io,
                self.media_id,
                first,
                LBA,
                block.as_mut_ptr(),
            ) != SUCCESS
        } {
            say(table, "uefi_loader: boot slot ");
            say_decimal(table, slot as u32);
            say(table, " would not read\r\n");
            return Started::Unreadable;
        }
        let Some(header) = SlotHeader::decode(block) else {
            say(table, "uefi_loader: boot slot ");
            say_decimal(table, slot as u32);
            say(table, " holds no image\r\n");
            return Started::Unreadable;
        };
        if header.image_len == 0 || header.image_len > BOOT_FILE_MAX {
            say(table, "uefi_loader: boot slot ");
            say_decimal(table, slot as u32);
            say(table, " claims an impossible size\r\n");
            return Started::Unreadable;
        }

        let blocks = header.image_len.div_ceil(LBA as u64);
        let pages = (blocks * LBA as u64).div_ceil(PAGE) as usize;
        let Some(base) = allocate_below(services, pages, memory_type::LOADER_DATA) else {
            say(table, "uefi_loader: no memory for a boot slot's image\r\n");
            return Started::Unreadable;
        };
        // SAFETY: `pages` covers `blocks` whole blocks at `base`, which the firmware just granted
        // exclusively and which is page-aligned.
        if unsafe {
            ((*self.block_io).read_blocks)(
                self.block_io,
                self.media_id,
                first + boot_slot::SLOT_IMAGE_OFFSET / LBA as u64,
                (blocks * LBA as u64) as usize,
                base as *mut u8,
            ) != SUCCESS
        } {
            say(table, "uefi_loader: boot slot ");
            say_decimal(table, slot as u32);
            say(table, "'s image would not read\r\n");
            return Started::Unreadable;
        }

        // SAFETY: the firmware granted the range and the read above filled it.
        let image =
            unsafe { core::slice::from_raw_parts(base as *const u8, header.image_len as usize) };
        if !header.matches(image) {
            // The failure worth having a checksum for: an install or an upgrade that died partway
            // through the copy leaves a valid-looking prefix of a boot image.
            say(table, "uefi_loader: boot slot ");
            say_decimal(table, slot as u32);
            say(table, " failed its checksum\r\n");
            return Started::Unreadable;
        }

        say(table, "uefi_loader: starting boot slot ");
        say_decimal(table, slot as u32);
        say(table, "\r\n");

        let mut child: Handle = ptr::null_mut();
        if (services.load_image)(
            0, // BootPolicy FALSE: this is not a boot-manager path, it is a buffer we hold
            handle,
            ptr::null(),
            image.as_ptr(),
            image.len(),
            &mut child,
        ) != SUCCESS
            || child.is_null()
        {
            say(table, "uefi_loader: the firmware would not load boot slot ");
            say_decimal(table, slot as u32);
            say(table, "\r\n");
            return Started::Unreadable;
        }

        tell_it_which_slot(child, services, slot);

        let mut exit_size = 0usize;
        let mut exit_data: *mut u16 = ptr::null_mut();
        let _ = (services.start_image)(child, &mut exit_size, &mut exit_data);
        // A nife boot image never returns, so arriving here means it failed and has already said so
        // on this same console.
        say(table, "uefi_loader: boot slot ");
        say_decimal(table, slot as u32);
        say(table, " came back, so it did not boot\r\n");
        Started::Refused
    }
}

/// **Tell the child which slot it is**, by writing [`STARTED_BY_CHOOSER`] and a digit into its
/// `LoadOptions`.
///
/// A failure here is not fatal and is not reported: the child then looks like an image the firmware
/// started directly, which means it boots its own kernel, which is what it was going to do anyway.
/// What it loses is the ability to say which slot it came from.
fn tell_it_which_slot(child: Handle, services: &BootServices, slot: usize) {
    let Some(loaded) = loaded_image(child, services) else {
        return;
    };
    let options = options();
    options[..STARTED_BY_CHOOSER.len()].copy_from_slice(STARTED_BY_CHOOSER);
    options[STARTED_BY_CHOOSER.len()] = b'0' + (slot as u8 % 10);
    let len = STARTED_BY_CHOOSER.len() + 1;

    let loaded = ptr::from_ref(loaded).cast_mut();
    // SAFETY: the firmware's `EFI_LOADED_IMAGE_PROTOCOL` for an image that has been loaded and not
    // yet started is the caller's to fill in; that is the specified way to pass load options, and
    // `OPTIONS` is a static that outlives the child.
    unsafe {
        (*loaded).load_options = options.as_ptr().cast::<c_void>();
        (*loaded).load_options_size = len as u32;
    }
}

/// `HandleProtocol` for `EFI_LOADED_IMAGE_PROTOCOL`, which both callers here want.
fn loaded_image(handle: Handle, services: &BootServices) -> Option<&'static LoadedImage> {
    let mut interface: *mut c_void = ptr::null_mut();
    if (services.handle_protocol)(handle, &LOADED_IMAGE_PROTOCOL_GUID, &mut interface) != SUCCESS
        || interface.is_null()
    {
        return None;
    }
    // SAFETY: the firmware answered with its own protocol interface for this image handle, and it
    // outlives boot services.
    Some(unsafe { &*interface.cast::<LoadedImage>() })
}

/// [`PRIMARY`] as a slice.
///
/// # Safety
/// One thread, no interrupts, and the chooser runs to completion before anything else in this
/// binary touches it. Taking the raw pointer first is what `static_mut_refs` asks for.
fn primary() -> &'static mut [u8] {
    let p = &raw mut PRIMARY;
    // SAFETY: see above.
    unsafe { &mut (*p).0 }
}

/// [`BLOCK`] as a slice. Same reasoning as [`primary`].
fn one_block() -> &'static mut [u8] {
    let p = &raw mut BLOCK;
    // SAFETY: see above.
    unsafe { &mut (*p).0 }
}

/// [`OPTIONS`] as a slice. Same reasoning as [`primary`], with one addition: the buffer must
/// outlive the child image, which a static does and a stack array would not.
fn options() -> &'static mut [u8] {
    let p = &raw mut OPTIONS;
    // SAFETY: see above.
    unsafe { &mut *p }
}

/// What the firmware says the medium behind a `BlockIo` is, or `None` for a null description.
///
/// A free function rather than a method because [`BlockIo`] belongs to this loader's library half
/// and this file is the binary: an inherent `impl` here would be an orphan.
///
/// # Safety
/// `block_io` must be a firmware-owned `EFI_BLOCK_IO_PROTOCOL` whose `media` pointer is the
/// firmware's own and outlives boot services.
unsafe fn media(block_io: *mut BlockIo) -> Option<&'static BlockIoMedia> {
    // SAFETY: the caller's contract.
    let pointer = unsafe { (*block_io).media };
    if pointer.is_null() {
        return None;
    }
    // SAFETY: as above.
    Some(unsafe { &*pointer })
}
