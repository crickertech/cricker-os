//! **`x86_64`: an `hvm_start_info`, and leaving long mode** (milestone 87 (the x86_64 bare-metal
//! machine)).
//!
//! The kernel's entry contract on this architecture is PVH's: 32-bit protected mode, paging off,
//! `eax` = `0x336EC578`, `ebx` = the physical address of an `hvm_start_info`. Everything here
//! produces that state from the one UEFI hands over. This is the code that was `main.rs` until the
//! loader learned a second and third architecture; it moved here unchanged in behaviour, and
//! notes/x86-uefi-boot.md is still the account of it.

use core::ptr;

use machine_discovery::framebuffer::{Framebuffer, PixelOrder};
use uefi_loader::efi::{
    self, ALLOCATE_ADDRESS, ALLOCATE_MAX_ADDRESS, BootServices, Handle, SUCCESS, SystemTable,
    memory_type,
};
use uefi_loader::handoff::{
    CMDLINE_LEN, MEMMAP_ENTRY_LEN, MODULE_ENTRY_LEN, START_INFO_LEN, StartInfo, e820_kind,
    encode_memmap_entry, encode_module,
};

use crate::{MAP_SLACK_DESCRIPTORS, PAGE, Placed, say, say_conflict, say_decimal, say_span};

// The mode-switch trampoline. See leave_long_mode.s, which is where the interesting half of this
// file's job actually happens.
core::arch::global_asm!(include_str!("leave_long_mode.s"));

unsafe extern "C" {
    /// The trampoline's first instruction. Copied, not called where it is linked.
    static x86_leave_long_mode: [u8; 0];
    /// The 32-bit continuation, whose address in the *copy* is an argument to the trampoline.
    static x86_leave_long_mode_pmode32: [u8; 0];
    /// The three descriptors the trampoline loads.
    static x86_leave_long_mode_gdt: [u8; 0];
    /// The `lgdt` operand, whose 8-byte base this loader patches after the copy.
    static x86_leave_long_mode_gdtr: [u8; 0];
    /// One past the last byte to copy.
    static x86_leave_long_mode_end: [u8; 0];
}

/// **The ceiling on every allocation this loader makes.**
///
/// Everything the kernel is handed has to be nameable by a 32-bit instruction stream running with
/// paging off, which is the state `_start` is entered in. A `hvm_start_info` at 5 GiB would be a
/// pointer the kernel's trampoline literally cannot load.
pub const ALLOCATION_CEILING: u64 = 0xffff_ffff;

/// **The page a STARTUP IPI can name**, which the kernel's AP bring-up copies its real-mode
/// trampoline into.
///
/// It is `AP_TRAMPOLINE_PHYS` in `kernel/link-x86_64.ld` and `.ap_trampoline`'s link address in the
/// kernel image, so the two files name one number. It is not part of the kernel's `p_paddr` span
/// (that section is linked low and *loaded* beside `.rodata`), which is why the span allocation
/// does not cover it and this loader has to ask for it separately.
const AP_TRAMPOLINE_PHYS: u64 = 0x8000;

/// Where the boot command line sits inside page 0 of the handoff block.
///
/// 256 rather than 120 (the `hvm_start_info` plus the **two**-entry module list) so that a field
/// added to either does not silently start overwriting the command line; the page has four
/// kilobytes and there is nothing else to spend them on.
///
/// **It was 128 against a one-entry list**, which left exactly eight bytes of slack, and milestone
/// 198 (a package manager, and the trivial install that makes a second customer possible)'s rung 2a
/// added the second entry (this loader's own file). Doubling it costs nothing and puts the slack
/// back.
const CMDLINE_OFFSET: u64 = 256;

/// What this architecture reads from the firmware before anything is placed.
pub struct Found {
    rsdp: u64,
    screen: Option<Framebuffer>,
}

/// **The two reads a kernel cannot make once the firmware is gone**: the ACPI root pointer, and the
/// screen (milestone 243).
///
/// Both are reads of something that already exists, so nothing later can invalidate them, and the
/// answers are in hand before the console these sentences are printed on goes away. **A machine
/// with no screen is not an error**: the `OptiPlex` with its serial module is that machine.
pub fn discover(table: &SystemTable, services: &BootServices) -> Result<Found, &'static str> {
    let rsdp = find_rsdp(table).ok_or("the firmware's configuration table has no ACPI RSDP")?;
    let screen = find_screen(services);
    match screen {
        Some(found) => {
            say_span(
                table,
                "uefi_loader: screen at ",
                found.base,
                found.base + found.span().unwrap_or(0) as u64,
            );
            say(table, "uefi_loader:   ");
            say_decimal(table, found.width);
            say(table, "x");
            say_decimal(table, found.height);
            say(table, ", stride ");
            say_decimal(table, found.stride);
            say(table, ", ");
            say(table, found.order.token());
            say(table, "\r\n");
        }
        None => say(
            table,
            "uefi_loader: no linear framebuffer; the kernel will have only a UART\r\n",
        ),
    }
    Ok(Found { rsdp, screen })
}

/// **Everything from "the kernel and the archive are placed" to "the kernel is running".**
/// Returns only on failure, and only while there is still a console to say so on.
pub fn hand_over(
    handle: Handle,
    table: &SystemTable,
    services: &BootServices,
    found: Found,
    kernel: &Placed,
    module: Option<(u64, u64)>,
    boot_file: Option<(u64, u64)>,
    // **Which boot slot started this image**, when a chooser did, so the running system can say
    // the boot worked. `None` for a stick, a `-kernel` boot, and a chooser's own fallback to the
    // image in its file; all three are boots with nothing to confirm.
    from_slot: Option<u8>,
    // **Whether this boot came from the NVMe disk**, which is the only boot whose kernel may mount
    // it (`boot_slot::medium`; calef's ruling on PR #1652, 2026-10-04 UTC).
    on_nvme: bool,
) -> Result<(), &'static str> {
    // --- The page the kernel's AP bring-up needs, asked for by name ---
    //
    // A STARTUP IPI names a physical PAGE below 1 MiB (`vector << 12`), so `arch::x86_64::ap_boot`
    // copies its real-mode trampoline to a fixed low address that `kernel/link-x86_64.ld` picks at
    // link time (`AP_TRAMPOLINE_PHYS`). Until milestone 195 this loader never mentioned that page,
    // and secondary cores under firmware therefore worked or did not by luck: OVMF happens to leave
    // the first 640 KiB conventional, and a firmware that did not would have been discovered by a
    // core that started executing something else's bytes in real mode.
    //
    // **A refusal is not fatal, and that asymmetry is deliberate.** A single-core boot on a machine
    // whose firmware wants this page is far more useful than no boot at all, and the kernel brings
    // up secondaries only when it is asked to. So this says what it found and carries on; the
    // person at the bench gets the one line that explains why `smp: 1 core(s) online` on a machine
    // with eight.
    let mut ap_page = AP_TRAMPOLINE_PHYS;
    if (services.allocate_pages)(ALLOCATE_ADDRESS, memory_type::LOADER_DATA, 1, &mut ap_page)
        != SUCCESS
    {
        say_span(
            table,
            "uefi_loader: WARNING no AP trampoline page at ",
            AP_TRAMPOLINE_PHYS,
            AP_TRAMPOLINE_PHYS + PAGE,
        );
        say_conflict(
            table,
            services,
            AP_TRAMPOLINE_PHYS,
            AP_TRAMPOLINE_PHYS + PAGE,
        );
        say(
            table,
            "uefi_loader: secondary cores will not be brought up\r\n",
        );
    }

    // --- Size the memory map, then allocate everything the handoff needs in one block ---
    //
    // The sizing call is expected to fail with EFI_BUFFER_TOO_SMALL and to report the size it
    // wanted; a firmware that returned success for a zero-length buffer would be reporting an empty
    // machine, which is why the size is checked rather than the status.
    let mut map_bytes = 0usize;
    let mut map_key = 0usize;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;
    (services.get_memory_map)(
        &mut map_bytes,
        ptr::null_mut(),
        &mut map_key,
        &mut descriptor_size,
        &mut descriptor_version,
    );
    if map_bytes == 0 || descriptor_size < size_of::<efi::MemoryDescriptor>() {
        return Err("the firmware reported no memory map");
    }

    let map_capacity = map_bytes + MAP_SLACK_DESCRIPTORS * descriptor_size;
    let entry_capacity = map_capacity / descriptor_size;

    // One allocation, three regions, so there is one failure to report and one address to print:
    //   page 0                    the `hvm_start_info` and the one-entry module list
    //   [entries)                 the PVH memory map this loader writes
    //   [raw)                     the firmware's own memory map, read once and never handed on
    let entries_bytes = (entry_capacity * MEMMAP_ENTRY_LEN) as u64;
    let handoff_pages = (PAGE
        + entries_bytes.next_multiple_of(PAGE)
        + (map_capacity as u64).next_multiple_of(PAGE))
        / PAGE;
    let mut handoff_base = ALLOCATION_CEILING;
    if (services.allocate_pages)(
        ALLOCATE_MAX_ADDRESS,
        memory_type::LOADER_DATA,
        handoff_pages as usize,
        &mut handoff_base,
    ) != SUCCESS
    {
        return Err("no memory below 4 GiB for the boot handoff");
    }
    let start_info_at = handoff_base;
    let module_list_at = handoff_base + START_INFO_LEN as u64;
    // **The boot command line lives in the same page**, after the 56-byte structure and the 32-byte
    // module list, which together end at 88. It is at most `Framebuffer::MAX_LEN` plus a NUL and
    // there are four kilobytes here, so it costs no allocation and adds no failure path: one more
    // `AllocatePages` for sixty bytes would be one more thing that can be refused on somebody's
    // firmware, in the middle of the one sequence in this loader whose order is rigid.
    let cmdline_at = handoff_base + CMDLINE_OFFSET;
    let memmap_at = handoff_base + PAGE;
    let raw_map_at = memmap_at + entries_bytes.next_multiple_of(PAGE);

    // --- The mode-switch trampoline, in a page the firmware will let us execute ---
    //
    // `LOADER_CODE` rather than `LOADER_DATA`, and that is not tidiness: firmware with a memory
    // protection policy (OVMF has one, and so does every recent vendor firmware) sets the
    // execute-disable bit on data allocations, and the first instruction of the copy would fault
    // with boot services still live and nothing watching.
    //
    // Below 4 GiB for the same reason everything else is: the second half of this blob executes
    // with paging off, where a linear address is a physical one.
    let mut trampoline_base = ALLOCATION_CEILING;
    if (services.allocate_pages)(
        ALLOCATE_MAX_ADDRESS,
        memory_type::LOADER_CODE,
        1,
        &mut trampoline_base,
    ) != SUCCESS
    {
        return Err("no executable page below 4 GiB for the mode-switch trampoline");
    }
    let trampoline = copy_trampoline(trampoline_base)?;

    // Everything the kernel is handed has to be reachable from 32 bits. Checked here, with a
    // console still available, rather than discovered as a triple fault on somebody's desk.
    if kernel.entry >= ALLOCATION_CEILING || start_info_at >= ALLOCATION_CEILING {
        return Err("the kernel entry or the handoff landed above 4 GiB");
    }

    say(
        table,
        "uefi_loader: kernel placed, exiting boot services\r\n",
    );

    // --- The final map, then the point of no return ---
    //
    // NOTHING may allocate between these two calls. `ExitBootServices` compares the key against the
    // map's current generation and refuses if anything has changed, which is the specification
    // protecting the kernel from being handed a map that no longer describes the machine.
    let mut final_bytes = map_capacity;
    if (services.get_memory_map)(
        &mut final_bytes,
        raw_map_at as *mut u8,
        &mut map_key,
        &mut descriptor_size,
        &mut descriptor_version,
    ) != SUCCESS
    {
        return Err("the firmware's memory map grew between the two GetMemoryMap calls");
    }
    if (services.exit_boot_services)(handle, map_key) != SUCCESS {
        return Err("ExitBootServices refused the map key");
    }

    // --- From here the firmware is gone. No console, no allocation, no going back. ---
    let descriptors = final_bytes / descriptor_size;
    let written = descriptors.min(entry_capacity);
    for i in 0..written {
        // SAFETY: `raw_map_at` holds `final_bytes` of descriptors and `i < final_bytes /
        // descriptor_size`. Read unaligned because the firmware chooses `descriptor_size` and
        // nothing promises the stride keeps 8-byte alignment.
        let descriptor = unsafe {
            ptr::read_unaligned(
                (raw_map_at as usize + i * descriptor_size) as *const efi::MemoryDescriptor,
            )
        };
        let bytes = encode_memmap_entry(
            descriptor.physical_start,
            descriptor.page_count * PAGE,
            e820_kind(descriptor.kind),
        );
        // SAFETY: `written <= entry_capacity`, which is what the region was sized for.
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (memmap_at as usize + i * MEMMAP_ENTRY_LEN) as *mut u8,
                MEMMAP_ENTRY_LEN,
            );
        }
    }

    // The command line, NUL-terminated, in the page reserved for it above. Written after
    // `ExitBootServices` like everything else in this section: it names memory this loader already
    // owns and needs no firmware call, so there is nothing here that could invalidate the map key.
    // **A command line whenever there is anything to put on it**, which since rung 2b of milestone
    // 198 is not the same as "whenever there is a screen": an installed machine with a serial
    // console and no framebuffer still has to be told which slot it came from, or it can never
    // confirm the boot and every upgrade to it reverts a few boots later.
    let mut token = [0u8; CMDLINE_LEN];
    // The one writer, so what this places is decoded in a host test by the kernel's own parsers;
    // `handoff::cmdline` says what each token is and what `screen_hold` adds.
    let written =
        uefi_loader::handoff::cmdline(found.screen.as_ref(), from_slot, on_nvme, &mut token);
    let cmdline = if written == 0 {
        0
    } else {
        // SAFETY: `CMDLINE_OFFSET + CMDLINE_LEN` is inside page 0 of the handoff block, which was
        // allocated and zeroed above, and `written + 1` is at most `CMDLINE_LEN`.
        unsafe { ptr::copy_nonoverlapping(token.as_ptr(), cmdline_at as *mut u8, written + 1) };
        cmdline_at
    };

    // **Module 0 is the archive and module 1 is this loader's own file, in that order, and the
    // order is the contract.** `arch::x86_64::machine::initrd` reads module 0 and has since
    // milestone 87; `machine::boot_file` reads module 1 and is milestone 198 (a package manager,
    // and the trivial install that makes a second customer possible)'s rung 2a. A boot with no
    // archive therefore hands over **no** modules at all rather than sliding the boot file into
    // slot 0, which would be read as an archive and fail the measurement that protects it.
    let modules: [Option<(u64, u64)>; 2] = match module {
        Some(_) => [module, boot_file],
        None => [None, None],
    };
    let module_count = modules.iter().filter(|m| m.is_some()).count() as u32;

    let info = StartInfo {
        rsdp: found.rsdp,
        modules: if module_count > 0 { module_list_at } else { 0 },
        module_count,
        memmap: memmap_at,
        memmap_entries: written as u32,
        cmdline,
    };
    for (index, entry) in modules.iter().enumerate() {
        let Some((addr, size)) = *entry else { continue };
        let bytes = encode_module(addr, size);
        // SAFETY: the module list sits inside page 0 of the handoff block, after the 56-byte
        // structure, and is two 32-byte entries ending well before `CMDLINE_OFFSET`.
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (module_list_at + (index * MODULE_ENTRY_LEN) as u64) as *mut u8,
                MODULE_ENTRY_LEN,
            )
        };
    }
    let bytes = info.encode();
    // SAFETY: page 0 of the handoff block, 56 bytes.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), start_info_at as *mut u8, START_INFO_LEN) };

    // **The last word anything says before the kernel exists** (milestone 243's early-boot half).
    //
    // From `ExitBootServices` to the kernel's own `attach_screen` nothing can speak on a machine
    // with no serial port: the firmware's console is gone, the kernel's is not up, and a fault in
    // between (the trampoline below, `boot.s`'s 32-bit half, the page tables, the long-mode jump)
    // is a triple fault and a silent reset with no IDT to catch it. That window cannot be
    // *narrated*, because the code in it is 32-bit and has no idea where the screen is. It can be
    // **bounded**, which is what this does: the screen is cleared and given one sentence, so the
    // five things a person at a monitor can be looking at are distinguishable rather than four of
    // them being one black rectangle. `uefi_loader::screen` has the ladder, and so does
    // notes/serial-less-output.md.
    //
    // Painted AFTER `ExitBootServices` on purpose: before it the screen is the firmware's console
    // and writing the aperture underneath it would race the firmware's own scrolling. The aperture
    // survives that call, which is the founding observation of milestone 243 (a machine with no
    // serial port): what ends is the firmware's *console*, not the *display*.
    if let Some(screen) = found.screen
        && let Some(span) = screen.span()
    {
        // SAFETY: `screen` came from this firmware's own `EFI_GRAPHICS_OUTPUT_PROTOCOL` and
        // `find_screen` checked its span against the aperture size the firmware reported. Boot
        // services are gone, so no firmware code is drawing there any more, and the kernel has not
        // started, so nothing else is either: this loader is the only writer in this instant. The
        // loader runs identity-mapped, so the physical base is the address.
        let pixels = unsafe { core::slice::from_raw_parts_mut(screen.base as *mut u8, span) };
        let _ = uefi_loader::screen::paint_handoff(screen, pixels);
    }

    // The trampoline was copied to an executable page below 4 GiB and its GDT pointer patched;
    // every argument is a physical address below 4 GiB, which is the contract stated at the top of
    // leave_long_mode.s. It does not return, so this expression's type is `!` and the `Ok` arm of
    // this function's signature is unreachable.
    (trampoline.enter)(
        kernel.entry,
        start_info_at,
        trampoline.gdtr,
        trampoline.pmode32,
    )
}

/// Where the copied trampoline's three interesting addresses ended up.
struct Trampoline {
    /// Its first instruction, called with the System V convention.
    enter: extern "sysv64" fn(u64, u64, u64, u64) -> !,
    // The `lgdt` operand, argument three.
    gdtr: u64,
    // The 32-bit continuation, argument four.
    pmode32: u64,
}

/// Copy `leave_long_mode.s` into `base` and patch the one address it cannot know at link time.
fn copy_trampoline(base: u64) -> Result<Trampoline, &'static str> {
    let start = (&raw const x86_leave_long_mode).cast::<u8>();
    let end = (&raw const x86_leave_long_mode_end).cast::<u8>();
    let len = end as usize - start as usize;
    if len > PAGE as usize {
        return Err("the mode-switch trampoline no longer fits in one page");
    }

    let offset =
        |symbol: *const [u8; 0]| base + (symbol.cast::<u8>() as usize - start as usize) as u64;
    let gdt = offset(&raw const x86_leave_long_mode_gdt);
    let gdtr = offset(&raw const x86_leave_long_mode_gdtr);
    let pmode32 = offset(&raw const x86_leave_long_mode_pmode32);

    // SAFETY: the source is this image's own `.text`, the destination is a page the firmware just
    // granted exclusively, and the length was checked against the page above.
    unsafe { ptr::copy_nonoverlapping(start, base as *mut u8, len) };

    // The GDT pointer's base is the only field in the blob whose correct value depends on where the
    // copy landed. It sits two bytes into the descriptor, after the 16-bit limit, and is therefore
    // unaligned by construction: `lgdt` does not care, and `write_unaligned` is why this is not a
    // fault on the first boot.
    // SAFETY: `gdtr` is inside the page just written, and the 8 bytes at `gdtr + 2` are the base
    // field `leave_long_mode.s` reserved with `.quad 0`.
    unsafe { ptr::write_unaligned((gdtr as usize + 2) as *mut u64, gdt) };

    Ok(Trampoline {
        // SAFETY: `base` now holds the trampoline's first instruction, and the signature is the
        // register contract leave_long_mode.s documents.
        enter: unsafe {
            core::mem::transmute::<u64, extern "sysv64" fn(u64, u64, u64, u64) -> !>(base)
        },
        gdtr,
        pmode32,
    })
}

/// **Where the ACPI root pointer comes from on a real machine.**
///
/// The 2.0 entry is preferred because its RSDP is revision 2 or later and carries the 64-bit
/// `xsdt_address`; the 1.0 entry is the fallback, and `machine_discovery::acpi` reads whichever
/// root the revision says is there. This is what makes a UEFI boot hand the kernel a **non-zero**
/// `rsdp`, where QEMU's PVH loader leaves it zero and the kernel falls back to scanning the BIOS
/// area for `"RSD PTR "`.
fn find_rsdp(table: &SystemTable) -> Option<u64> {
    let mut fallback = None;
    for entry in crate::configuration_table(table) {
        if entry.vendor_guid == efi::ACPI_20_TABLE_GUID {
            return Some(entry.vendor_table as u64);
        }
        if entry.vendor_guid == efi::ACPI_10_TABLE_GUID {
            fallback = Some(entry.vendor_table as u64);
        }
    }
    fallback
}

/// **Ask the firmware where the screen is** (milestone 243).
///
/// One `LocateProtocol` call and three field reads. It deliberately does **not** set a video mode:
/// the firmware has already chosen one that works on this monitor, and a mode set here would be a
/// mode the kernel has to be told about through the same channel anyway.
///
/// `None` on every unhappy answer, and they are all the same answer to the caller: no protocol, a
/// null `mode` or `info`, a `PixelBltOnly` adapter (which has no linear framebuffer at all, only a
/// boot-services `Blt` call that is gone by the time the kernel runs), a `PixelBitMask` adapter
/// (whose channels are described by masks `machine_discovery::framebuffer` cannot express), or a
/// geometry whose arithmetic does not close. A machine with no screen is the `OptiPlex`, and it is
/// not an error.
fn find_screen(services: &BootServices) -> Option<Framebuffer> {
    let mut interface: *mut core::ffi::c_void = ptr::null_mut();
    if (services.locate_protocol)(
        &efi::GRAPHICS_OUTPUT_PROTOCOL_GUID,
        ptr::null_mut(),
        &mut interface,
    ) != SUCCESS
        || interface.is_null()
    {
        return None;
    }
    // SAFETY: `LocateProtocol` returned success, so the firmware asserts this is an
    // `EFI_GRAPHICS_OUTPUT_PROTOCOL` and that it outlives boot services.
    let gop = unsafe { &*interface.cast::<efi::GraphicsOutput>() };
    if gop.mode.is_null() {
        return None;
    }
    // SAFETY: as above; `Mode` is a required field of the protocol and non-null was just checked.
    let mode = unsafe { &*gop.mode };
    if mode.info.is_null() {
        return None;
    }
    // SAFETY: as above. Read unaligned is not needed: the firmware allocates this structure and the
    // specification gives it natural alignment.
    let info = unsafe { &*mode.info };
    let order = match info.pixel_format {
        efi::pixel_format::BGRX => PixelOrder::Bgrx,
        efi::pixel_format::RGBX => PixelOrder::Rgbx,
        _ => return None,
    };
    let found = Framebuffer {
        base: mode.framebuffer_base,
        width: info.horizontal_resolution,
        height: info.vertical_resolution,
        // The firmware reports the stride in PIXELS and every consumer of it wants bytes. This is
        // the one multiplication in the whole path and getting it wrong shears the picture.
        stride: info.pixels_per_scan_line.checked_mul(4)?,
        order,
    };
    // The kernel's console indexes this with a `usize` computed from the geometry, so a geometry
    // whose span does not close is refused here rather than trusted there.
    found.span()?;
    // And the aperture the firmware reports has to actually hold what the geometry claims. A
    // firmware that reported a stride larger than its own framebuffer would have this loader
    // handing the kernel a licence to write past the end of a BAR.
    if found.span()? > mode.framebuffer_size {
        return None;
    }
    Some(found)
}

/// Park the core. `hlt` rather than a bare spin for the reason AGENTS.md gives about `wfi`: a
/// halted core should not burn a real one. Under QEMU this is the difference between 0% and 100% of
/// a host thread.
pub fn park() -> ! {
    loop {
        // SAFETY: `hlt` at CPL 0, with no effect other than parking this core until an interrupt.
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)) };
    }
}
