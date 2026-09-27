//! **The offer a booted stick makes, and the two programs that carry it out** (milestone 198 (a
//! package manager, and the trivial install that makes a second customer possible), rung 2a).
//!
//! The proposal it follows is milestone 515 (a stick that puts itself on the machine's disk).
//!
//! # What this module is, and what it deliberately is not
//!
//! It is the **boot policy**: decide whether this boot could install at all, say what would be
//! destroyed, ask, and then wire the two confined programs that do the work. It is not the
//! installer. Nothing here writes a byte to a disk.
//!
//! The split is the whole argument, and it is the one `disk_service` already makes for milestone
//! 57's partitioner:
//!
//! | | holds | can destroy |
//! |---|---|---|
//! | this module | the boot path, the console | nothing; it has no block endpoint of its own to spend |
//! | `components/src/installer.rs` | one disk, one entropy endpoint, a read-only copy of the boot file | that disk |
//! | `redoxfs_server`'s `mkfs` | the same disk, the same entropy endpoint | that disk |
//!
//! **The question is asked here rather than in the installer**, and that is not an accident of
//! where the console is. A confirmation a program prints to itself is a confirmation that program
//! could also decide to skip; asking before the capability is granted means the authority to wipe
//! the disk does not exist until the answer does.
//!
//! # When the offer is made
//!
//! Three things must be true, and all three are facts about the machine rather than preferences:
//!
//! 1. **This boot came from a file the loader could read back** (`memory::boot_file_region`), so
//!    there is something to install. A `-kernel` boot has no such file and is silently skipped.
//! 2. **There is an NVMe controller**, which is the disk an install would go to.
//! 3. **That disk does not already carry nife.** An installed machine boots from a file too, so
//!    without this check every boot of every installed machine would pause to offer to wipe itself.
//!    The check is a survey process, not a parse in the kernel: `installer`'s `ROLE_SURVEY`, spawned
//!    with the disk and **no entropy endpoint**. That is `disk_partitioner`'s verify role, one
//!    milestone along. **What the missing endpoint narrows is what the survey would write, not what
//!    it can** (2026-09-24 security audit): a `blk` endpoint is the whole disk with no read-only
//!    form, a reader of a partition table never checks that its ids were drawn from anywhere, so a
//!    survey that chose to could write any table, or zeros, anywhere on that disk. What stops it is
//!    the program's role, which is rung three of AGENTS.md's ladder; `confirm`'s section below
//!    prices the read-only `blk` that would make it rung one.
//! 4. **Somebody answers.** The wait is bounded, so a stick booted on a machine with no console
//!    reaches the prompt instead of hanging.
//!
//! Anything else is a refusal, and every refusal is silent-or-explained and then continues to the
//! ordinary boot. **Refusing by default is the whole safety property**: the only path to a write is
//! a person typing [`CONFIRMATION`].
//!
//! # EXAMPLES
//!
//! ```text
//!   install     : this system was booted from a file and can install itself.
//!   install     :   TARGET: the NVMe disk attached to this machine, 1073741824 bytes.
//!   install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.
//!   install     :   Type INSTALL and press return to proceed; anything else continues the boot.
//!   install     : > INSTALL
//!   install     : partitioning and copying the boot file...
//!   install     : installed. nife data at LBA 2048, EFI system at LBA 1046528.
//!   install     : filesystem created.
//!   install     : DONE. Remove the installation medium and reboot.
//! ```
//!
//! # BUGS
//!
//! - **x86_64 only**, because `memory::boot_file_region` is: a device-tree handoff has one initrd
//!   slot in `/chosen` and no second one for the loader's own file. `uefi_loader`'s
//!   `place_boot_file` carries the argument and the gap, and
//!   `design/roadmap/568-the-boot-file-has-nowhere-to-go-on-a-device-tree-machine.md` prices
//!   closing it and names the three other things that still stand between it and an install on
//!   those architectures.
//! - **It asks on the console, which means the kernel reads the UART** (`console::read_line`).
//!   That is only sound because this runs before the progenitor hands the device to the input
//!   driver, and it is the reason the offer is made at this exact point in the boot and not later.
//! - **The disk is named by what it is, not by model and serial.** Milestone 515's `BUGS` asks for
//!   model and size, and only the size is here: reading a controller's model is a second IDENTIFY
//!   (`CNS_CONTROLLER`) in the admin plane and there is nowhere in
//!   `non_volatile_memory_express::Handoff` to put forty ASCII bytes, whose three scalars are all
//!   spent. On a machine with two disks this sentence would not say which, and that is why the
//!   offer says "the NVMe disk attached to this machine" rather than naming an ordinal it cannot
//!   back up. **It is the single most load-bearing sentence a stranger reads**, and it has a
//!   milestone of its own, `design/roadmap/569-the-disk-an-installer-names-has-no-model.md`,
//!   which finds that the kernel-side half needs no ruling and the program-side half is the wire
//!   question milestone 421 (the block roster cannot name an NVMe disk) is already held on.
//! - **Nothing here is crash-atomic**, and a power cut between the partitioner and `mkfs` leaves a
//!   partitioned disk with no filesystem, which the next boot reports as a disk it cannot mount.
//! - **It checks for nife and for nothing else.** A disk carrying Windows, or a Linux root, looks
//!   exactly like an empty one from here: the survey asks "is there a nife data partition" and any
//!   other answer is "installable". A person who says yes to the question loses that disk. Naming
//!   what is already on it is `disk_surveyor`'s job, and
//!   `design/roadmap/570-the-install-offer-should-say-what-is-already-on-the-disk.md` is the
//!   milestone for it.
//! - **A machine that already carries nife is never offered an install again**, which is what stops
//!   an installed system offering to wipe itself on every boot and also means there is no way to
//!   reinstall from the stick. The sharp case is a power cut between the installer and `mkfs`,
//!   which leaves a nife data partition with no filesystem in it and a machine that will never be
//!   offered an install again. `design/roadmap/572-there-is-no-way-back-from-the-stick.md`
//!   is the milestone, and the fix is a second question rather than a second mechanism.

use super::*;
use crate::cap::{Rights, memory_region_cap, page_frame_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// **The word a person types to destroy a disk.** Upper case and unabbreviated, so that a stray
/// newline, a `y`, or a terminal echoing something cannot be it.
pub const CONFIRMATION: &str = "INSTALL";

/// How long the offer waits for an answer before continuing the boot.
///
/// Long enough that somebody who has just watched a machine come up can read three lines and type
/// seven characters; short enough that a stick left in a machine in a cupboard still reaches a
/// prompt. It is a duration measured against the counter, not a spin count.
const PATIENCE: core::time::Duration = core::time::Duration::from_secs(30);

/// Where the kernel maps the boot file, read-only, into the installer. **Must match
/// `components/src/installer.rs`'s `BOOT_FILE_VA`**; the bytes are a run of physical frames the
/// kernel owns, so the program cannot map them itself.
const BOOT_FILE_VA: u64 = address_space_map::runtime_window(0x1000_0000);

/// The installer's capability table. Must match `components/src/installer.rs`.
const SLOT_REPORT: u64 = 0;
const SLOT_BLK: u64 = 1;
const SLOT_ENTROPY: u64 = 2;
const SLOT_BUDGET: u64 = 3;
const SLOT_BLK_PAGE: u64 = 4;

/// The untyped the installer spends on the page tables for the one page it maps itself. The same
/// figure `disk_service` gives the partitioner, and for the same reason: it maps one page.
const INSTALLER_BUDGET_PAGES: u64 = 64;

/// Extra stack pages for the installer, below the single page `run` maps.
///
/// It keeps a 16 KiB entry array in `.bss` rather than on the stack and recurses nowhere, so this
/// is the partitioner's number rather than a measurement of its own; the deepest thing it does is
/// `GloballyUniqueIdentifierPartitionTable::create`.
const INSTALLER_STACK_PAGES: usize = 8;

/// The report's first word, from `components/src/installer.rs`. Duplicated here rather than shared
/// through a crate because it is the kernel's own convention with one program it spawns, the same
/// terms `disk_service` and `disk_tests` already state for `R_PARTITIONED` and `R_MADE`.
const R_INSTALLED: u64 = 0x_49_4E_53_54_44;

/// `mkfs`'s create role, from `redoxfs_server/src/bin/mkfs.rs`. Spelled here for the same reason
/// `disk_tests` spells it: the kernel's convention with a program it spawns.
const ROLE_MAKE: u64 = 0;
/// `mkfs`'s "a filesystem was created and a file written into it" verdict, ASCII `MKFSD`.
const R_MADE: u64 = 0x_4D_4B_46_53_44;

// The installer's roles, in `a0`. Must match `components/src/installer.rs`.
const ROLE_INSTALL: u64 = 0;
const ROLE_SURVEY: u64 = 1;
const ROLE_CONFIRM: u64 = 2;
/// The survey's "this disk already carries a nife data partition" verdict, ASCII `HAVIT`.
const R_ALREADY: u64 = 0x_48_41_56_49_54;
/// The confirm role's "that slot is successful" verdict, ASCII `CNFRM`.
const R_CONFIRMED: u64 = 0x_43_4E_46_52_4D;
/// Its "there is no such boot slot on this disk" refusal, ASCII `NOSLT`.
const R_NO_SLOT: u64 = 0x_4E_4F_53_4C_54;

/// **Offer to install, and carry it out if a person says so.** Returns after either, so the boot
/// continues in every case.
///
/// The three programs it needs are named rather than passed, because a caller on the boot path
/// has no business knowing which binaries an install is made of.
pub fn offer() {
    let Some((_, boot_file_len)) = crate::memory::boot_file_region() else {
        return; // Not booted from a file: nothing to install.
    };
    let Some(installer) = program("installer") else {
        crate::println!(
            "  install     : this boot came from a file, but the archive has no installer."
        );
        return;
    };
    let Some(nvme) = program("non_volatile_memory_express") else {
        return;
    };
    let Some(disk) = non_volatile_memory_express_service::ensure(nvme) else {
        return; // No NVMe controller: nothing to install onto.
    };
    disk.wait_for_ready();

    // **Before the question, and by a process that cannot write.** An installed machine boots from
    // a file too, so an offer that did not look at the disk first would ask every installed machine
    // to wipe itself, once per boot.
    let report = spawn_installer(installer, &disk, None, ROLE_SURVEY, 0, 0);
    if crate::sched::ipc_recv(report)[0] == R_ALREADY {
        return; // nife is already on this disk. See BUGS: there is no reinstall.
    }

    if !asked(disk.size_bytes) {
        crate::println!("  install     : not confirmed; continuing the boot.");
        return;
    }

    // Entropy from the instruction rather than from a device: `RDSEED` is on every x86_64 part a
    // stranger would install this on, and needing no virtio-rng is what lets the offer work on a
    // machine that has only a disk. A machine without it gets `None` here and the installer's own
    // refusal below, which is the honest answer rather than a GPT full of invented ids.
    let entropy = program("entropy")
        .and_then(|image| entropy_service::ensure(image, entropy_service::Bus::Instruction))
        .map(|w| {
            w.wait_for_ready();
            w.request
        });
    let Some(entropy) = entropy else {
        crate::println!(
            "  install     : this machine has no source of randomness, so a partition table \
             written here could not carry unique ids. Nothing was written."
        );
        return;
    };

    crate::println!("  install     : partitioning and copying the boot file...");
    let report = spawn_installer(
        installer,
        &disk,
        Some(entropy),
        ROLE_INSTALL,
        boot_file_len,
        boot_file_len,
    );
    let answer = crate::sched::ipc_recv(report);
    if answer[0] != R_INSTALLED {
        crate::println!(
            "  install     : FAILED ({:#x}, {}, {}). The disk may be in any state.",
            answer[0],
            answer[1],
            answer[2]
        );
        return;
    }
    crate::println!(
        "  install     : installed. nife data at LBA {}, EFI system at LBA {}.",
        answer[1],
        answer[2]
    );

    // The filesystem, by the program that already knows how to make one inside a partition it finds
    // by type GUID. It is `mkfs` unmodified, on the same disk and the same entropy endpoint.
    let Some(maker) = program("mkfs") else {
        crate::println!("  install     : the archive has no mkfs, so the data partition is empty.");
        return;
    };
    let target = disk_service::BlankDisk {
        blk_ep: disk.request,
        blk_shared: disk.transfer_phys,
        blk_ready: None,
    };
    let report = disk_service::start_maker(maker, &target, ROLE_MAKE, Some(entropy), true);
    let answer = crate::sched::ipc_recv(report);
    if answer[0] != R_MADE {
        crate::println!(
            "  install     : the partition table is written but mkfs refused ({:#x}).",
            answer[0]
        );
        return;
    }
    crate::println!("  install     : filesystem created.");
    crate::println!("  install     : DONE. Remove the installation medium and reboot.");
}

/// **Say that this boot worked**, so an upgrade that came up is not rolled back once its tries run
/// out (rung 2b of milestone 198's other half).
///
/// A no-op on every boot no chooser started, which is a stick, a `-kernel` boot, and an installed
/// machine whose chooser fell back to the image in its own file. All three reach a running kernel
/// with nothing to confirm.
///
/// # What "this boot worked" means here, and it is a promise rather than a mechanism
///
/// **The criterion is: the machine got far enough that a person can log in and fix whatever else is
/// wrong.** Concretely, at this call site, all of the following already happened:
///
/// | |
/// |---|
/// | the kernel came up, its self-tests passed, and it reached userspace |
/// | the NVMe controller was brought up and its server answered requests from ring 3 |
/// | the block server and the filesystem server started, and the filesystem server **mounted the installed disk and reported ready** |
/// | the progenitor was loaded out of the archive, measured against the trust root, and built |
///
/// Nothing is confirmed unless every one of those held, because this runs after all of them and
/// the boot does not reach here otherwise.
///
/// **Why not earlier, and why not later.** Confirming at the kernel's self-test would mark an
/// upgrade good that has no working disk and no userspace, which is the failure the whole feature
/// exists to prevent; the bias has to be late. Confirming later is what the rest of this comment
/// is about, and the honest answer is that later is not reachable automatically on this system
/// today: past this point the machine is waiting for a person, and an unattended appliance has
/// nobody to ask. The Boot Loader Specification leaves the call to the operating system for the
/// same reason, and ChromeOS's update engine sets the bit from userspace on the same judgement.
///
/// **What a boot that satisfies this criterion can still be wrong about**, which is the honest half
/// of the feature and is repeated in `crates/boot_slot`'s `BUGS` where a reader meets it:
///
/// - **Anything nothing exercises at boot.** The network stack, the compositor, a driver for a
///   device this machine has and the boot path does not touch. An upgrade that breaks only those
///   is confirmed and kept.
/// - **The shell**, which is the one thing the proposal's recommended criterion names and this does
///   not reach. The filesystem is mounted and the progenitor is built, but nothing has typed
///   anything. A regression between here and a prompt survives.
/// - **Anything that fails after the first few seconds**: a leak, a wedge under load, a filesystem
///   that mounts and then corrupts. This is a statement about coming up, not about running.
///
/// # What holds the authority to write the partition table, and how narrow it is
///
/// `components/src/installer.rs`'s [`ROLE_CONFIRM`], spawned here with four capabilities and
/// nothing else: the report endpoint, the disk's `blk` endpoint, the page it shares with that
/// disk's server, and a budget for the one page it maps itself. **No entropy endpoint**, so it
/// cannot draw the unique ids a new partition table carries and the only table it can write is the
/// one it read; **no boot file**, so it cannot rewrite the image it is vouching for; no console, no
/// filesystem, no network.
///
/// It is not as narrow as it should be, and the gap is named rather than papered over: a `blk`
/// endpoint is the whole disk, because nothing in `filesystem_protocol::blk` bounds a client to a
/// block range. So this process *could* write anywhere on that disk. That is the same limitation
/// `installer`'s own `BUGS` records for the filesystem server, it is one wire field away from
/// fixed, and it is the reason this is a boot-path call rather than something a program could ask
/// for later.
///
/// # What serialises this against the filesystem server, and the answer is an ordering
///
/// **Nothing enforces it.** One NVMe server has one transfer region, shared by every client of its
/// endpoint, and a client stages bytes into that region before it calls. Two clients staging at
/// once would corrupt each other, and no lock, lease or range check prevents it.
///
/// What makes this write safe is where it is: **after the filesystem server's ready report and
/// before the progenitor's first instruction.** In that window the filesystem server is blocked in
/// receive with no client that could wake it, because the kernel boot path is single-threaded and
/// is the only thing that can hand its endpoint to anybody. So the region has one user.
///
/// That is rung three of `AGENTS.md`'s ladder, a written record at the thing itself, and it is an
/// **exception** to the rule that the higher rung wins. It is stated as one: **this is a foot gun.**
/// The day something spawns a second disk client before the progenitor runs, or this call moves
/// after it, the ordering silently stops holding and the symptom is a corrupted partition table on
/// somebody's installed machine. The mechanism that would make it hold is a transfer region per
/// client, or a `blk` endpoint bounded to a block range; both are wire decisions and neither is
/// this lane's.
pub fn confirm() {
    let Some(slot) = crate::memory::boot_slot() else {
        return; // No chooser started this boot: there is nothing to confirm.
    };
    let Some(installer) = program("installer") else {
        crate::println!(
            "  boot slot   : slot {slot} cannot be confirmed: the archive has no installer."
        );
        return;
    };
    let Some(nvme) = program("non_volatile_memory_express") else {
        return;
    };
    let Some(disk) = non_volatile_memory_express_service::ensure(nvme) else {
        // A machine that booted from a slot and has no NVMe controller is a machine whose disk
        // went away between the chooser and here. Nothing to write to, and saying so is better
        // than a silent revert three boots later.
        crate::println!("  boot slot   : slot {slot} cannot be confirmed: no disk.");
        return;
    };
    disk.wait_for_ready();

    let report = spawn_installer(installer, &disk, None, ROLE_CONFIRM, slot as u64, 0);
    let answer = crate::sched::ipc_recv(report);
    match answer[0] {
        R_CONFIRMED if answer[2] == 0 => {
            crate::println!("  boot slot   : slot {slot} was already confirmed; nothing written.");
        }
        R_CONFIRMED => {
            crate::println!(
                "  boot slot   : slot {slot} confirmed. This upgrade will not roll back."
            );
        }
        R_NO_SLOT => {
            // The number crossed a boot and did not match anything, so nothing was written. A
            // machine in this state rolls back, which is the safe direction, and this line is the
            // only place a person would find out why.
            crate::println!(
                "  boot slot   : slot {slot} is not on this disk ({} boot slots found); NOT confirmed.",
                answer[1]
            );
        }
        other => {
            crate::println!(
                "  boot slot   : slot {slot} NOT confirmed ({other:#x}, step {}). This boot will roll back.",
                answer[1]
            );
        }
    }
}

/// Print what is about to be destroyed and read the answer. `true` only for exactly
/// [`CONFIRMATION`].
///
/// Name: provisional, flagged 2026-09-24 by the boolean-predicate pass
/// (design/naming/boolean-predicates-worklist.md). It acts, so the Rust predicate rule calef
/// ratified 2026-09-24 exempts it, but it is a participle that reads as a question, on a function
/// that prompts and reads the reply; recommended `ask_to_confirm`.
fn asked(size_bytes: u64) -> bool {
    crate::println!("  install     : this system was booted from a file and can install itself.");
    crate::println!(
        "  install     :   TARGET: the NVMe disk attached to this machine, {size_bytes} bytes."
    );
    crate::println!("  install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.");
    crate::println!(
        "  install     :   Type {CONFIRMATION} and press return to proceed; anything else \
         continues the boot."
    );
    crate::print!("  install     : > ");

    let mut typed = [0u8; 32];
    let Some(n) = crate::console::read_line(&mut typed, PATIENCE) else {
        crate::println!();
        return false;
    };
    &typed[..n] == CONFIRMATION.as_bytes()
}

/// Spawn the installer with the four things it needs and nothing else: the disk, the randomness,
/// the page it shares with the disk's server, and a read-only copy of the file this machine booted
/// from.
fn spawn_installer(
    image: &'static [u8],
    disk: &non_volatile_memory_express_service::Wiring,
    entropy: Option<RendezvousId>,
    role: u64,
    // What the role reads out of `a1`: the boot file's length for [`ROLE_INSTALL`], the slot
    // number for [`ROLE_CONFIRM`], nothing for [`ROLE_SURVEY`].
    arg1: u64,
    // **How many bytes of the boot file to map**, which is a separate number from `arg1` because
    // only one role wants both. A role that maps none of it cannot rewrite the image it is
    // looking at, which is the same narrowing the missing entropy endpoint is.
    boot_file_len: u64,
) -> RendezvousId {
    let report = crate::sched::create_rendezvous();
    let budget =
        crate::memory_region::create(INSTALLER_BUDGET_PAGES).expect("no budget for the installer");
    let blk_ep = disk.request;
    let blk_shared = disk.transfer_phys;
    // Read only when `entropy` is `Some`; copied out here so the closure captures a plain `u64`,
    // the same shape `disk_service::start_partitioner` uses for the same reason.
    let ep = entropy.unwrap_or_default();

    crate::sched::spawn(move || {
        let maps = installer_maps(boot_file_len);
        crate::sched::grant_at(SLOT_REPORT, rendezvous_cap(report, Rights::WRITE))
            .expect("installer slot 0 was occupied");
        crate::sched::grant_at(SLOT_BLK, rendezvous_cap(blk_ep, Rights::WRITE))
            .expect("installer slot 1 was occupied");
        // **Slot 2 is the difference between the two roles and nothing else is.** A survey holds
        // the same disk, writable; without entropy it cannot draw the unique ids a *new* table
        // carries, which narrows what the program would write and not what it can (module docs,
        // "When the offer is made", item 3).
        if entropy.is_some() {
            crate::sched::grant_at(SLOT_ENTROPY, rendezvous_cap(ep, Rights::WRITE))
                .expect("installer slot 2 was occupied");
        }
        crate::sched::grant_at(SLOT_BUDGET, memory_region_cap(budget))
            .expect("installer slot 3 was occupied");
        crate::sched::grant_at(
            SLOT_BLK_PAGE,
            page_frame_cap(blk_shared, Rights::READ.union(Rights::WRITE)),
        )
        .expect("installer slot 4 was occupied");
        run(
            image,
            Spawn {
                arg0: role,
                arg1,
                arg2: 0,
                grants: &[], // every one of them is placed above, at its own slot
                maps,
            },
        )
    })
    .expect("could not spawn the installer");

    report
}

/// **Build the installer's extra mappings**: its stack, then the boot file read-only, one entry per
/// page.
///
/// The boot file is about ten megabytes, so this is a few thousand entries and they do not fit on a
/// stack or in `.bss` worth spending on a feature used once in the life of a machine. They go in
/// frames allocated for the purpose and are never freed, which costs about sixty kilobytes for the
/// rest of this boot and is the cheapest honest answer: the alternative is a `pages` field on
/// [`Mapping`], and that struct has ninety-six literals in this tree.
fn installer_maps(boot_file_len: u64) -> &'static [Mapping] {
    let boot_file_phys = crate::memory::boot_file_region().map_or(0, |(at, _)| at);
    let pages = boot_file_len.div_ceil(FRAME_SIZE) as usize;
    let total = INSTALLER_STACK_PAGES + pages;

    let bytes = total * core::mem::size_of::<Mapping>();
    let scratch = crate::memory::alloc_contiguous_zeroed(bytes.div_ceil(FRAME_SIZE as usize))
        .expect("no scratch for the installer's mappings")
        .addr();
    // SAFETY: `total` `Mapping`s fit in the run of frames just allocated, which nothing else holds;
    // the direct map reaches them, and `Mapping` is a plain `Copy` struct of integers, so the
    // zeroed frames are already a valid initialised slice of them.
    let maps: &'static mut [Mapping] = unsafe {
        core::slice::from_raw_parts_mut(mmu::phys_to_virt(scratch) as *mut Mapping, total)
    };

    for (k, m) in maps[..INSTALLER_STACK_PAGES].iter_mut().enumerate() {
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
        m.phys = crate::memory::alloc()
            .expect("no frame for the installer's stack")
            .addr();
        m.flags = Flags::user_data();
    }
    for (i, m) in maps[INSTALLER_STACK_PAGES..].iter_mut().enumerate() {
        m.va = BOOT_FILE_VA + i as u64 * FRAME_SIZE;
        m.phys = boot_file_phys + i as u64 * FRAME_SIZE;
        // Read-only, and it is the whole of what keeps a program holding a disk from also being
        // able to rewrite the image it is about to install.
        m.flags = Flags::user_rodata();
    }
    maps
}
