//! **aarch64: the firmware's device tree, and the Linux arm64 boot state.**
//!
//! The kernel's entry contract here is the one U-Boot's `booti` meets (Linux's
//! Documentation/arch/arm64/booting.rst, which `kernel/src/arch/aarch64/boot.s` cites): entered at
//! the image's first byte with **the MMU off**, `x0` = the physical address of the device tree and
//! `x1`..`x3` zero, at EL2 or EL1 (the kernel drops itself from EL2). UEFI hands over the opposite
//! state: MMU on with an identity map, caches on. So the whole delta is:
//!
//! 1. copy the firmware's device tree, adding `/chosen/linux,initrd-*` for the archive
//!    (`uefi_loader::device_tree_patch`);
//! 2. exit boot services;
//! 3. clean and invalidate the data cache to the point of coherency over everything the kernel will
//!    read with its caches off, so it reads what was written rather than stale RAM;
//! 4. turn the MMU and the data cache off at whichever exception level we are running at, and
//!    branch (`leave_uefi.s`).
//!
//! # BUGS
//!
//! - **An ACPI machine is described second-hand.** A machine that offers no device tree is read out
//!   of its ACPI tables and a tree is **written** for it here
//!   ([`uefi_loader::device_tree_from_acpi`]), which is what makes an SBBR server (every aarch64
//!   cloud instance) bootable at all. The translation loses what only AML holds: no PCI bus, no
//!   virtio-mmio, no `fw_cfg`. That module's own BUGS section is the list, and the firmware's tree
//!   stays first in [`discover`]'s order precisely because it is the richer of the two.
//! - **The kernel is linked for `0x4008_0000`** (`kernel/link-aarch64.ld`), which is QEMU `virt`'s
//!   RAM. argon's RAM starts at `0x8000_0000`, so this payload cannot place its kernel there and
//!   says so with the firmware's memory map printed: the same limit milestone 127's port has to
//!   lift for `booti`, not one the stick adds.
//! - **Everything the kernel reads early must be below 2 GiB** ([`ALLOCATION_CEILING`]), because
//!   the boot map in `boot.s` is one 1 GiB block of RAM at `0x4000_0000`. A firmware that keeps its
//!   device tree higher is fine (it is copied down); one with no free RAM below 2 GiB is not.

use core::ptr;

use device_tree_blob::Region;
use machine_discovery::acpi;
use machine_discovery::gic::Gic;
use uefi_loader::efi::{self, BootServices, Handle, SUCCESS, SystemTable, memory_type};
use uefi_loader::{device_tree_from_acpi, device_tree_patch};

use crate::{
    MAP_SLACK_DESCRIPTORS, PAGE, Placed, allocate_below, configuration_table, copy_device_tree,
    exit_boot_services, say, say_decimal, say_hex,
};

core::arch::global_asm!(include_str!("leave_uefi.s"));

unsafe extern "C" {
    /// `leave_uefi.s`: MMU and data cache off at the current EL, then `x0` = `device_tree`, branch.
    fn aarch64_leave_uefi(entry: u64, device_tree: u64) -> !;
}

/// **The ceiling on everything the kernel reads before its own page tables are up**: the device
/// tree and the archive. `boot.s`'s boot map covers RAM with a single 1 GiB block at
/// `0x4000_0000`, so the last byte it reaches is `0x7fff_ffff`.
pub const ALLOCATION_CEILING: u64 = 0x7fff_ffff;

/// The firmware's device tree, found before anything moves.
pub struct Found {
    device_tree: *const u8,
}

/// **Find, or failing that write, the tree the kernel is handed.**
///
/// # The order, and why it is this one
///
/// The firmware's own device tree is taken whenever the firmware offers one, and ACPI is read only
/// when it does not. That order is not "the old path first"; it is the richer description first.
/// A machine's device tree names its PCI bus with the BAR windows and the interrupt map, its
/// virtio-mmio transports and its `fw_cfg`, none of which exist outside AML on an ACPI machine, so a
/// tree written from ACPI ([`uefi_loader::device_tree_from_acpi`]) is strictly a subset of one the
/// firmware hands over. Preferring ACPI where both exist would cost a working boot facts it already
/// had.
///
/// A machine offering both is rare (QEMU `virt` with `acpi=off` offers only the tree, and with
/// `acpi=on` only the tables), and the rule still has to be written down, because the machine this
/// eventually runs on may not be either of those.
pub fn discover(table: &SystemTable, services: &BootServices) -> Result<Found, &'static str> {
    if let Some(entry) = configuration_table(table)
        .iter()
        .find(|entry| entry.vendor_guid == efi::DEVICE_TREE_GUID)
    {
        return Ok(Found {
            device_tree: entry.vendor_table.cast(),
        });
    }
    let rsdp = find_rsdp(table).ok_or(
        "the firmware offers neither a device tree nor ACPI, so nothing describes this machine",
    )?;
    Ok(Found {
        device_tree: tree_from_acpi(table, services, rsdp)?,
    })
}

/// Clean and invalidate the data cache to the point of coherency over `[start, end)`, by virtual
/// address, which under UEFI's identity map is the physical one.
fn clean_to_coherency(start: u64, end: u64) {
    let ctr: u64;
    // SAFETY: reading the cache type register has no side effects.
    unsafe { core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr, options(nomem, nostack)) };
    // DminLine, bits 19:16: log2 of the smallest data cache line, in words.
    let line = 4u64 << ((ctr >> 16) & 0xf);
    let mut at = start & !(line - 1);
    while at < end {
        // SAFETY: `dc civac` on an address the firmware mapped for us; it moves no data this
        // program can observe, only where the current copy lives.
        unsafe { core::arch::asm!("dc civac, {}", in(reg) at, options(nostack)) };
        at += line;
    }
    // SAFETY: a barrier.
    unsafe { core::arch::asm!("dsb sy", options(nostack)) };
}

/// Exit boot services and enter the kernel. Returns only on failure, with a console.
pub fn hand_over(
    handle: Handle,
    table: &SystemTable,
    services: &BootServices,
    found: Found,
    kernel: &Placed,
    module: Option<(u64, u64)>,
    // **Always `None` here**, and that is a recorded gap rather than an unused parameter.
    // `main.rs`'s `place_boot_file` reads this loader's own file so the running system can install
    // itself, and hands it over as a second PVH module. A device-tree handoff has `/chosen` with
    // one initrd and no second slot, so there is nowhere to put it; see that function's BUGS and
    // milestone 198 (a package manager, and the trivial install that makes a second customer
    // possible)'s rung 2a, which is an x86_64 claim for this reason.
    _boot_file: Option<(u64, u64)>,
    // **Always `None` here**, and that is a scope note (DECISIONS
    // §19 (architectural parity is a tenet; the targets are aarch64, riscv64, and x86_64)) rather than an unused parameter. The chooser that would set it is
    // `uefi_loader::chooser`, which is x86_64 only for the same reason the install offer is: an
    // installed machine on this architecture is what rung 2b of milestone 198 built, and the
    // device-tree architectures have no installed disk to choose slots on yet. When they do, this
    // parameter is where the number arrives.
    _from_slot: Option<u8>,
    // The boot medium token (`boot_slot::medium`) is x86_64 only, like the chooser.
    _on_nvme: bool,
) -> Result<(), &'static str> {
    if kernel.end > ALLOCATION_CEILING + 1 {
        return Err("the kernel is linked above what its own boot map reaches");
    }
    let device_tree = copy_device_tree(services, found.device_tree, module)?;
    say(
        table,
        "uefi_loader: kernel placed, exiting boot services\r\n",
    );
    exit_boot_services(handle, services)?;

    // --- The firmware is gone. No console from here. ---
    clean_to_coherency(kernel.start, kernel.end);
    if let Some((start, len)) = module {
        clean_to_coherency(start, start + len);
    }
    // The tree's own header says how long it is; the copy was written with that length.
    // SAFETY: `device_tree` is the copy made above and its header was written by us.
    let len = unsafe { u32::from_be(ptr::read_unaligned((device_tree + 4) as *const u32)) } as u64;
    clean_to_coherency(device_tree, device_tree + len);

    // SAFETY: everything the kernel reads is placed, cleaned to coherency, and below the ceiling;
    // `leave_uefi.s` documents the state it leaves, which is the arm64 boot protocol's.
    unsafe { aarch64_leave_uefi(kernel.entry, device_tree) }
}

/// Park the core with `wfi`, which QEMU turns into a real host-thread sleep (AGENTS.md).
pub fn park() -> ! {
    loop {
        // SAFETY: waits for an interrupt; no other effect.
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
    }
}

// ---------------------------------------------------------------------------------------------
// The other description: ACPI, for a machine that offers no device tree.
// ---------------------------------------------------------------------------------------------

/// **Find the ACPI root pointer**, the 2.0 entry first because its RSDP carries the 64-bit XSDT.
/// Identical in shape to the x86_64 loader's `find_rsdp`, and deliberately a separate four lines
/// rather than a shared helper: the two architectures reach it through the same configuration table
/// but do entirely different things with it.
fn find_rsdp(table: &SystemTable) -> Option<u64> {
    let mut fallback = None;
    for entry in configuration_table(table) {
        if entry.vendor_guid == efi::ACPI_20_TABLE_GUID {
            return Some(entry.vendor_table as u64);
        }
        if entry.vendor_guid == efi::ACPI_10_TABLE_GUID {
            fallback = Some(entry.vendor_table as u64);
        }
    }
    fallback
}

/// A system descriptor table, as the bytes its own header says it is.
///
/// # Safety
///
/// `phys` must be a physical address the firmware described as holding an ACPI table. The loader
/// runs under UEFI's identity map, so the physical address is the one to read.
unsafe fn table_at(phys: u64) -> Option<&'static [u8]> {
    if phys == 0 {
        return None;
    }
    // SAFETY: the caller's contract; the header's length field is at offset 4 and is read before
    // anything past the 36 bytes every table is guaranteed to have.
    let header = unsafe { core::slice::from_raw_parts(phys as *const u8, acpi::SDT_HEADER_LEN) };
    let len = acpi::parse_sdt_header(header).ok()?.length as usize;
    if len < acpi::SDT_HEADER_LEN {
        return None;
    }
    // SAFETY: as above, now for the length the header states.
    let bytes = unsafe { core::slice::from_raw_parts(phys as *const u8, len) };
    // **The checksum is the whole reason to bother**, and it is cheap. A table that does not sum to
    // zero is one this loader will not describe a machine from.
    acpi::is_checksum_ok(bytes).then_some(bytes)
}

/// The first table in the root list with this signature, checksum already checked.
///
/// # Safety
///
/// `rsdp` must be the firmware's ACPI root pointer.
unsafe fn find_table(rsdp: u64, signature: &[u8; 4]) -> Option<&'static [u8]> {
    // SAFETY: the caller's contract. An RSDP is 36 bytes at most and `parse_rsdp` checks both
    // checksums before any field is believed.
    let bytes = unsafe { core::slice::from_raw_parts(rsdp as *const u8, 36) };
    let rsdp = acpi::parse_rsdp(bytes).ok()?;
    let (root, wide) = rsdp.root_table();
    // SAFETY: the address the RSDP states, read as the table it says it is.
    let root = unsafe { table_at(root) }?;
    let body = &root[acpi::SDT_HEADER_LEN..];
    let count = acpi::root_entry_count(root.len() as u32, wide);
    for i in 0..count {
        let Some(at) = acpi::root_entry(body, i, wide) else {
            continue;
        };
        // SAFETY: an entry of the root table, which is a list of table addresses by definition.
        let Some(table) = (unsafe { table_at(at) }) else {
            continue;
        };
        if &table[..4] == signature {
            return Some(table);
        }
    }
    None
}

/// **Read the firmware's memory map into [`device_tree_from_acpi::Ram`] runs.**
///
/// Adjacent descriptors of RAM-ish types are merged, which on every machine this can describe
/// collapses a few dozen descriptors into one or two runs, the same shape the firmware's own device
/// tree would have stated. The map is read here rather than at hand-over because the tree has to be
/// written while allocation still works; nothing later widens or narrows RAM, so a map read now
/// describes the same machine.
fn read_ram(
    services: &BootServices,
    out: &mut [device_tree_from_acpi::Ram; device_tree_from_acpi::MAX_RAM],
) -> Result<usize, &'static str> {
    let mut bytes = 0usize;
    let mut key = 0usize;
    let mut descriptor_size = 0usize;
    let mut version = 0u32;
    (services.get_memory_map)(
        &mut bytes,
        ptr::null_mut(),
        &mut key,
        &mut descriptor_size,
        &mut version,
    );
    if bytes == 0 || descriptor_size < size_of::<efi::MemoryDescriptor>() {
        return Err("the firmware reported no memory map");
    }
    let capacity = bytes + MAP_SLACK_DESCRIPTORS * descriptor_size;
    let mut buffer: *mut u8 = ptr::null_mut();
    if (services.allocate_pool)(memory_type::LOADER_DATA, capacity, &mut buffer) != SUCCESS
        || buffer.is_null()
    {
        return Err("no memory for the firmware's memory map");
    }
    let mut got = capacity;
    if (services.get_memory_map)(
        &mut got,
        buffer,
        &mut key,
        &mut descriptor_size,
        &mut version,
    ) != SUCCESS
    {
        (services.free_pool)(buffer);
        return Err("the firmware's memory map would not fit the buffer it asked for");
    }

    let mut count = 0usize;
    for i in 0..got / descriptor_size {
        // SAFETY: `buffer` holds `got` bytes of descriptors and `i` is inside that count. Read
        // unaligned because the firmware chooses `descriptor_size` and nothing promises the stride
        // keeps 8-byte alignment.
        let d = unsafe {
            ptr::read_unaligned(
                (buffer as usize + i * descriptor_size) as *const efi::MemoryDescriptor,
            )
        };
        // **What counts as RAM the kernel may have.** Conventional memory, plus the two
        // boot-services types (free the instant `ExitBootServices` returns), plus the loader's own
        // allocations, which is where the kernel image and the archive already sit and which the
        // kernel carves out for itself. Deliberately NOT runtime services, ACPI reclaim or NVS: the
        // tables this tree was written from live there, and handing them to the frame allocator
        // would be handing out the description of the machine.
        let usable = matches!(
            d.kind,
            memory_type::CONVENTIONAL
                | memory_type::BOOT_SERVICES_CODE
                | memory_type::BOOT_SERVICES_DATA
                | memory_type::LOADER_CODE
                | memory_type::LOADER_DATA
        );
        if !usable || d.page_count == 0 {
            continue;
        }
        let start = d.physical_start;
        let len = d.page_count * PAGE;
        // The map is sorted by address, so a descriptor that begins where the previous run ended
        // extends it. That is what turns a firmware's several dozen entries into the one or two
        // runs a device tree would have stated.
        if count > 0 && out[count - 1].start + out[count - 1].len == start {
            out[count - 1].len += len;
            continue;
        }
        if count == device_tree_from_acpi::MAX_RAM {
            break;
        }
        out[count] = device_tree_from_acpi::Ram { start, len };
        count += 1;
    }
    (services.free_pool)(buffer);
    if count == 0 {
        return Err("the firmware's memory map describes no usable RAM");
    }
    Ok(count)
}

/// **Describe this machine from its ACPI tables, and write the device tree the kernel reads.**
///
/// Returns the physical address of the blob, which from the caller's side is indistinguishable from
/// one the firmware offered: everything downstream (`copy_device_tree`, the initrd patch, the
/// handover) is the path a device-tree machine already takes.
fn tree_from_acpi(
    table: &SystemTable,
    services: &BootServices,
    rsdp: u64,
) -> Result<*const u8, &'static str> {
    // SAFETY: `rsdp` came from the firmware's own configuration table, and every table reached
    // from it is checksummed before a field is read.
    let madt = unsafe { find_table(rsdp, b"APIC") }
        .ok_or("this machine's ACPI has no MADT, so nothing describes its interrupt controller")?;
    let body = &madt[acpi::SDT_HEADER_LEN..];

    // --- The interrupt controller, which is the one thing a boot cannot do without ---
    let mut distributor = 0u64;
    let mut version = 0u8;
    let mut redistributors = Region { start: 0, size: 0 };
    let mut cpu_interface = 0u64;
    let mut per_cpu_redistributor = 0u64;
    let mut cpus = [device_tree_from_acpi::Cpu::default(); device_tree_from_acpi::MAX_CPUS];
    let mut cpu_count = 0usize;
    for entry in acpi::madt_entries(body) {
        match entry {
            acpi::MadtEntry::GenericDistributor {
                address,
                version: v,
            } => {
                distributor = address;
                version = v;
            }
            acpi::MadtEntry::GenericRedistributor { address, length } => {
                redistributors = Region {
                    start: address,
                    size: u64::from(length),
                };
            }
            acpi::MadtEntry::GenericInterruptController {
                mpidr,
                enabled,
                online_capable,
                cpu_interface: gicc,
                redistributor,
                ..
            } => {
                if gicc != 0 {
                    cpu_interface = gicc;
                }
                if redistributor != 0 && per_cpu_redistributor == 0 {
                    per_cpu_redistributor = redistributor;
                }
                if cpu_count < device_tree_from_acpi::MAX_CPUS {
                    cpus[cpu_count] = device_tree_from_acpi::Cpu {
                        mpidr,
                        // `online_capable` is a socket firmware would start later, and the kernel's
                        // `is_startable` predicate has no third state: it is not a core to call
                        // `CPU_ON` on now, so the node is written disabled.
                        enabled,
                    };
                    let _ = online_capable;
                    cpu_count += 1;
                }
            }
            _ => {}
        }
    }
    if distributor == 0 {
        return Err("this machine's MADT names no GIC distributor");
    }
    // **A version byte of zero means "work it out"**, and the evidence is which spelling the CPU
    // entries used: a redistributor (per-core or as a range) exists only on a GICv3, and a
    // memory-mapped CPU interface only on a GICv2. A machine that states neither is one this loader
    // will not guess about.
    let v3 = version >= 3 || redistributors.size != 0 || per_cpu_redistributor != 0;
    let v2 = version == 2 || (version == 0 && cpu_interface != 0);
    let distributor = Region {
        start: distributor,
        size: device_tree_from_acpi::GICD_LEN,
    };
    let gic = if v3 {
        if redistributors.size == 0 {
            // The per-core spelling: each GICC entry carries its own frame and the MADT states no
            // range. Two 64 KiB frames per core is the GICv3 architecture's own layout, and the
            // frames are contiguous from the first, which is what `GICR_TYPER`'s Last bit then
            // confirms when the kernel walks them.
            redistributors = Region {
                start: per_cpu_redistributor,
                size: 0x2_0000 * cpu_count.max(1) as u64,
            };
        }
        Gic::V3 {
            distributor,
            redistributors,
        }
    } else if v2 {
        Gic::V2 {
            distributor,
            cpu_interface: Region {
                start: cpu_interface,
                size: device_tree_from_acpi::GICC_LEN,
            },
        }
    } else {
        return Err("this machine's MADT names a GIC whose version this loader cannot work out");
    };

    // --- The rest, every one of which a machine may legitimately not have ---
    // SAFETY: as above, for the same root pointer.
    let timer = unsafe { find_table(rsdp, b"GTDT") }
        .and_then(|t| acpi::parse_gtdt(&t[acpi::SDT_HEADER_LEN..]).ok());
    // SAFETY: as above.
    let psci_hvc = unsafe { find_table(rsdp, b"FACP") }
        .and_then(|t| acpi::parse_arm_boot(&t[acpi::SDT_HEADER_LEN..]).ok())
        .and_then(|arm| arm.psci.then_some(arm.hvc));
    // SAFETY: as above.
    let uart = unsafe { find_table(rsdp, b"SPCR") }
        .and_then(|t| acpi::parse_spcr(&t[acpi::SDT_HEADER_LEN..]).ok())
        .filter(|spcr| {
            // System memory only, and only the two PL011 spellings: the kernel's console driver is
            // a PL011 and a node claiming to be one over a 16550 would be a lie this loader told.
            spcr.address_space == 0
                && spcr.address != 0
                && matches!(spcr.interface, acpi::SPCR_PL011 | acpi::SPCR_SBSA_UART)
        })
        .map(|spcr| device_tree_from_acpi::Uart {
            address: spcr.address,
            interrupt: spcr.interrupt,
        });

    let mut ram = [device_tree_from_acpi::Ram::default(); device_tree_from_acpi::MAX_RAM];
    let ram_count = read_ram(services, &mut ram)?;

    let machine = device_tree_from_acpi::Machine {
        gic,
        cpus,
        cpu_count,
        psci_hvc,
        timer,
        uart,
        ram,
        ram_count,
    };

    let pages = (device_tree_from_acpi::output_len() as u64).div_ceil(PAGE) as usize;
    let base = allocate_below(services, pages, memory_type::LOADER_DATA)
        .ok_or("no memory below the kernel's reach for a device tree written from ACPI")?;
    // SAFETY: `pages` pages at `base` were just granted exclusively.
    let out = unsafe { core::slice::from_raw_parts_mut(base as *mut u8, pages * PAGE as usize) };
    device_tree_from_acpi::build(&machine, out).map_err(device_tree_patch::Error::reason)?;

    say(
        table,
        "uefi_loader: no device tree; describing this machine from ACPI\r\n",
    );
    say(table, "uefi_loader:   ");
    say_decimal(table, cpu_count as u32);
    say(
        table,
        match gic {
            Gic::V2 { .. } => " core(s), GICv2 at ",
            Gic::V3 { .. } => " core(s), GICv3 at ",
        },
    );
    say_hex(table, distributor.start);
    say(table, ", ");
    say_decimal(table, ram_count as u32);
    say(table, " RAM region(s)\r\n");
    if uart.is_none() {
        say(
            table,
            "uefi_loader:   no SPCR names a PL011; the kernel will fall back for its console irq\r\n",
        );
    }
    if psci_hvc.is_none() {
        say(
            table,
            "uefi_loader:   the FADT does not claim PSCI; the kernel will run one core\r\n",
        );
    }
    Ok(base as *const u8)
}
