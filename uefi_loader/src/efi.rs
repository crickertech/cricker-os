//! **The slice of UEFI this loader speaks**, hand-written rather than taken as a dependency.
//!
//! # Why there is no `uefi` crate here
//!
//! DECISIONS §46: a dependency is a decision, and the tree's shape is thin architectural
//! primitives or whole subsystems nobody would write, with nothing in between. The `uefi` crate is
//! squarely in between. What this loader needs is **seven function pointers and three GUIDs**:
//! allocate pages, get the memory map, exit boot services, print a line, locate one protocol, and
//! the two ACPI configuration-table identifiers plus the graphics one. That is the whole of it, and
//! it is written below in a few hundred lines that a reader can check against the specification
//! without leaving the repository. (Six and two until milestone 243 (a machine with no serial port
//! has no way to say anything) asked where the screen is;
//! eight and five since milestone 198 (a package manager, and the trivial install that makes a
//! second customer possible)'s rung 2a asked the loader to read its own file, which adds
//! `HandleProtocol` and the three small protocol tables at the end of this file.)
//!
//! The cost of getting it wrong is also unusually visible: a mis-numbered field in
//! [`BootServices`] is a call to the wrong function pointer, which faults immediately and loudly at
//! the very start of boot, rather than being the sort of subtle wrongness a dependency protects
//! against.
//!
//! # The one rule to remember when editing this file
//!
//! **[`BootServices`]'s field order is the ABI.** The firmware hands over a pointer to a table it
//! filled in; every entry is found by its offset and nothing checks the name. Adding a field in the
//! middle, or getting the count of unused ones wrong, silently re-points every call after it. The
//! fields are therefore listed in specification order, in full, with the unused ones spelled
//! `usize` and *counted* in comments rather than collapsed into an array.
//!
//! # BUGS
//!
//! - **The struct definitions stop where this loader's needs stop.** [`SystemTable`] ends after
//!   `configuration_table`, [`BootServices`] ends after `locate_protocol`, and [`GraphicsOutput`]
//!   ends after `mode`; the real tables
//!   are longer. That is safe (the firmware's allocation is larger than ours, and we only read),
//!   but a field added below the last one listed will not be found by name and has to be counted
//!   in from the specification the same way these were.
//! - **Nothing validates the table headers' CRC.** The firmware is the trust root at this point in
//!   boot; there is nothing to check it against that is not also the firmware.

use core::ffi::c_void;

/// An opaque firmware object. Only ever passed back to the firmware.
pub type Handle = *mut c_void;

/// A UEFI return code. Zero is success; the high bit marks an error.
pub type Status = usize;

/// `EFI_SUCCESS`.
pub const SUCCESS: Status = 0;

/// A UEFI GUID, in the mixed-endian form the specification prints and stores.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Guid {
    /// First 32 bits, stored little-endian.
    pub a: u32,
    /// Next 16 bits, stored little-endian.
    pub b: u16,
    /// Next 16 bits, stored little-endian.
    pub c: u16,
    /// The last eight bytes, stored big-endian, which is why they are a byte array and not a `u64`.
    pub d: [u8; 8],
}

/// `EFI_ACPI_20_TABLE_GUID`, whose configuration-table entry is the **XSDT-capable** RSDP.
///
/// This is the one to prefer: an RSDP found here has revision 2 or later, so it carries the 64-bit
/// `xsdt_address` as well as the 32-bit `rsdt_address`, and `machine_discovery::acpi` reads
/// whichever the revision says is there.
pub const ACPI_20_TABLE_GUID: Guid = Guid {
    a: 0x8868_e871,
    b: 0xe4f1,
    c: 0x11d3,
    d: [0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
};

/// `ACPI_TABLE_GUID`, the ACPI 1.0 RSDP, taken only when the 2.0 entry is absent.
pub const ACPI_10_TABLE_GUID: Guid = Guid {
    a: 0xeb9d_2d30,
    b: 0x2d88,
    c: 0x11d3,
    d: [0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
};

/// **The flattened device tree's configuration-table entry** (`gFdtTableGuid`, read from EDK2's
/// `MdePkg/MdePkg.dec` on 2026-09-19). The aarch64 and riscv64 loaders find the machine's device
/// tree here; U-Boot's `bootefi` and EDK2 both install it.
///
/// **EDK2 on aarch64 installs it only when it is not presenting ACPI**, which under QEMU `virt`
/// means `-machine virt,acpi=off`: with ACPI on, the firmware hides the tree, and a device-tree
/// kernel has nothing to read. Recorded in notes/boot-stick.md, measured there.
pub const DEVICE_TREE_GUID: Guid = Guid {
    a: 0xb1b6_21d5,
    b: 0xf19c,
    c: 0x41a5,
    d: [0x83, 0x0b, 0xd9, 0x15, 0x2c, 0x69, 0xaa, 0xe0],
};

/// **`RISCV_EFI_BOOT_PROTOCOL`** (`gRiscVEfiBootProtocolGuid`, read from EDK2's
/// `UefiCpuPkg/UefiCpuPkg.dec`, 2026-09-19), which answers the one question RISC-V has that the
/// other two architectures do not: which hart is this. The kernel's entry contract wants it in
/// `a0`, and nothing a UEFI application runs in tells it otherwise.
pub const RISCV_BOOT_PROTOCOL_GUID: Guid = Guid {
    a: 0xccd1_5fec,
    b: 0x6f73,
    c: 0x4eec,
    d: [0x83, 0x95, 0x3e, 0x69, 0xe4, 0xb9, 0x40, 0xbf],
};

/// The protocol's interface: a revision and one function (`RiscVBootProtocol.h` in EDK2).
#[repr(C)]
pub struct RiscvBootProtocol {
    /// `RISCV_EFI_BOOT_PROTOCOL_REVISION`, 0x00010000 for the first.
    pub revision: u64,
    /// `GetBootHartId(this, &mut hart)`.
    pub get_boot_hart_id: extern "efiapi" fn(*mut RiscvBootProtocol, *mut usize) -> Status,
}

/// The header every UEFI table begins with. Read for nothing here; present so the fields after it
/// are at their specified offsets.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TableHeader {
    /// Identifies which table this is.
    pub signature: u64,
    /// The UEFI revision the table conforms to.
    pub revision: u32,
    /// Bytes in the header plus the table.
    pub header_size: u32,
    /// A CRC-32 over the table with this field zeroed.
    pub crc32: u32,
    /// Must be zero.
    pub reserved: u32,
}

/// `EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL`, cut off after the one method this loader calls.
///
/// It is worth having at all for one reason: **it is the only way a person standing at the machine
/// learns that anything ran before `ExitBootServices`.** On the `OptiPlex` the serial console does
/// not carry firmware output, so a stub that failed silently would be indistinguishable from a
/// stick the firmware never looked at.
#[allow(
    dead_code,
    reason = "the leading `reset` is a placeholder whose only job is to put `output_string` at its \
              specified offset; it is read by the firmware's own layout, never by this crate"
)]
#[repr(C)]
pub struct SimpleTextOutput {
    reset: usize,
    /// Print a NUL-terminated UTF-16 string.
    pub output_string: extern "efiapi" fn(*mut SimpleTextOutput, *const u16) -> Status,
}

/// `EFI_MEMORY_DESCRIPTOR`.
///
/// **Never stride an array of these by `size_of::<MemoryDescriptor>()`.** `GetMemoryMap` reports
/// its own `descriptor_size`, which the specification permits to be larger than the structure and
/// which real firmware does make larger. Striding by the Rust size is the classic UEFI loader bug:
/// it works on the firmware you tested and produces a garbage memory map on the next one.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct MemoryDescriptor {
    /// One of the `MEMORY_TYPE_*` constants below.
    pub kind: u32,
    /// Present so `physical_start` lands at offset 8, which is where the specification puts it.
    pub padding: u32,
    /// The first byte of the range.
    pub physical_start: u64,
    /// Where the range will be after `SetVirtualAddressMap`, which this loader never calls.
    pub virtual_start: u64,
    /// The length, in 4 KiB pages.
    pub page_count: u64,
    /// Cacheability and runtime attributes.
    pub attribute: u64,
}

/// `EFI_MEMORY_TYPE`. Only the ones [`crate::handoff`] classifies are named.
pub mod memory_type {
    /// Not usable by anyone.
    pub const RESERVED: u32 = 0;
    /// The loader's own image.
    pub const LOADER_CODE: u32 = 1;
    /// Memory the loader allocated, which is where this loader puts everything it hands over.
    pub const LOADER_DATA: u32 = 2;
    /// Firmware boot-services code, free once `ExitBootServices` returns.
    pub const BOOT_SERVICES_CODE: u32 = 3;
    /// Firmware boot-services data, free once `ExitBootServices` returns.
    pub const BOOT_SERVICES_DATA: u32 = 4;
    /// Firmware runtime code. Live forever.
    pub const RUNTIME_SERVICES_CODE: u32 = 5;
    /// Firmware runtime data. Live forever.
    pub const RUNTIME_SERVICES_DATA: u32 = 6;
    /// Free RAM.
    pub const CONVENTIONAL: u32 = 7;
    /// RAM the firmware found faulty.
    pub const UNUSABLE: u32 = 8;
    /// Holds ACPI tables; reclaimable once they have been read.
    pub const ACPI_RECLAIM: u32 = 9;
    /// ACPI non-volatile storage.
    pub const ACPI_NVS: u32 = 10;
    /// Memory-mapped IO.
    pub const MMIO: u32 = 11;
    /// Memory-mapped IO port space.
    pub const MMIO_PORT_SPACE: u32 = 12;
    /// Processor abstraction-layer code.
    pub const PAL_CODE: u32 = 13;
    /// Byte-addressable persistent memory.
    pub const PERSISTENT: u32 = 14;
}

/// `EFI_ALLOCATE_TYPE::AllocateMaxAddress`: give me pages **at or below** the address I pass in.
///
/// This is the one that matters here. Everything the kernel is handed has to be nameable by a
/// 32-bit trampoline running with paging off, so every allocation this loader makes asks for it
/// below 4 GiB rather than taking whatever the firmware felt like.
pub const ALLOCATE_MAX_ADDRESS: u32 = 1;

/// `EFI_ALLOCATE_TYPE::AllocateAddress`: give me exactly this address or fail.
pub const ALLOCATE_ADDRESS: u32 = 2;

/// `EFI_BOOT_SERVICES`, in specification order, truncated after `ExitBootServices`.
///
/// The unused entries are `usize` and are **counted in the comments**, because their number is the
/// only thing keeping the used ones at the right offsets.
#[allow(
    dead_code,
    reason = "the private fields are the unused entries of the firmware's table. They exist to \
              hold the used ones at their specified offsets and are deliberately spelled out one \
              by one rather than collapsed, because their COUNT is the ABI"
)]
#[repr(C)]
pub struct BootServices {
    /// The table header.
    pub hdr: TableHeader,

    // --- Task priority services (2) ---
    raise_tpl: usize,
    restore_tpl: usize,

    // --- Memory services (5) ---
    /// `AllocatePages(type, memory_type, pages, &mut physical_address)`.
    ///
    /// **`memory` is in/out**: with [`ALLOCATE_MAX_ADDRESS`] it carries the ceiling on the way in
    /// and the allocated base on the way out.
    pub allocate_pages: extern "efiapi" fn(u32, u32, usize, *mut u64) -> Status,
    /// `FreePages(physical_address, pages)`.
    pub free_pages: extern "efiapi" fn(u64, usize) -> Status,
    /// `GetMemoryMap(&mut size, buffer, &mut key, &mut descriptor_size, &mut descriptor_version)`.
    pub get_memory_map:
        extern "efiapi" fn(*mut usize, *mut u8, *mut usize, *mut usize, *mut u32) -> Status,
    /// `AllocatePool(memory_type, size, &mut buffer)`.
    pub allocate_pool: extern "efiapi" fn(u32, usize, *mut *mut u8) -> Status,
    /// `FreePool(buffer)`.
    pub free_pool: extern "efiapi" fn(*mut u8) -> Status,

    // --- Event and timer services (6) ---
    create_event: usize,
    set_timer: usize,
    wait_for_event: usize,
    signal_event: usize,
    close_event: usize,
    check_event: usize,

    // --- Protocol handler services (9) ---
    install_protocol_interface: usize,
    reinstall_protocol_interface: usize,
    uninstall_protocol_interface: usize,
    /// `HandleProtocol(handle, &guid, &mut interface)`.
    ///
    /// The one call milestone 198 (a package manager, and the trivial install that makes a second
    /// customer possible)'s rung 2a added, and unlike [`Self::locate_protocol`] it asks about a
    /// **named handle** rather than about the machine: which volume did *this image* come from.
    /// That question is what lets the loader open its own file, and the loader's own file is the
    /// only copy of the kernel-and-archive pair the running system has (`src/main.rs`'s
    /// `place_boot_file`).
    pub handle_protocol: extern "efiapi" fn(Handle, *const Guid, *mut *mut c_void) -> Status,
    reserved: usize,
    register_protocol_notify: usize,
    locate_handle: usize,
    locate_device_path: usize,
    install_configuration_table: usize,

    // --- Image services (4 before the one we want) ---
    /// `LoadImage(boot_policy, parent, device_path, source_buffer, source_size, &mut handle)`.
    ///
    /// The two calls milestone 198's rung 2b added, and they are what makes the image at
    /// `\EFI\BOOT\BOOTX64.EFI` a **chooser** rather than only a loader: it reads a boot slot off
    /// the disk and starts the image in it. `device_path` is null here and `source_buffer` is the
    /// slot's bytes, which is the specification's own way of starting an image that is not a file
    /// on a volume the firmware can see.
    pub load_image:
        extern "efiapi" fn(u8, Handle, *const c_void, *const u8, usize, *mut Handle) -> Status,
    /// `StartImage(handle, &mut exit_data_size, &mut exit_data)`. Returns only if the started image
    /// returns, which for a nife boot image means it failed.
    pub start_image: extern "efiapi" fn(Handle, *mut usize, *mut *mut u16) -> Status,
    exit: usize,
    unload_image: usize,
    /// `ExitBootServices(image_handle, map_key)`.
    ///
    /// **The `map_key` must come from a `GetMemoryMap` with nothing allocated since**, which is
    /// what makes the call order in the loader binary (`src/main.rs`) rigid rather than stylistic.
    pub exit_boot_services: extern "efiapi" fn(Handle, usize) -> Status,

    // --- Miscellaneous services (3) ---
    get_next_monotonic_count: usize,
    stall: usize,
    set_watchdog_timer: usize,

    // --- Driver support services (2) ---
    connect_controller: usize,
    disconnect_controller: usize,

    // --- Open and close protocol services (3) ---
    open_protocol: usize,
    close_protocol: usize,
    open_protocol_information: usize,

    // --- Library services (2 before the one we want) ---
    protocols_per_handle: usize,
    /// `LocateHandleBuffer(search_type, &guid, search_key, &mut count, &mut handles)`.
    ///
    /// Rung 2b's other addition, and it asks the one question the chooser cannot answer any other
    /// way: **which whole disks does this machine have?** `HandleProtocol` asks about a handle
    /// already in hand and `LocateProtocol` answers with the firmware's first choice; only this
    /// one enumerates. The buffer comes from the firmware's pool and is never freed, which costs a
    /// few dozen bytes for the rest of a boot.
    pub locate_handle_buffer:
        extern "efiapi" fn(u32, *const Guid, *mut c_void, *mut usize, *mut *mut Handle) -> Status,
    /// `LocateProtocol(&guid, registration, &mut interface)`.
    ///
    /// The one call milestone 243 added, and it is the cheapest possible form of the question:
    /// **is there a linear framebuffer on this machine, and where.** `registration` is always null
    /// here, which asks for the first handle carrying the protocol rather than the next one since
    /// some notification. On a machine with two display adapters that is the firmware's choice of
    /// console rather than ours, and this loader has no basis for a better one.
    pub locate_protocol: extern "efiapi" fn(*const Guid, *mut c_void, *mut *mut c_void) -> Status,
}

/// `EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID`.
pub const GRAPHICS_OUTPUT_PROTOCOL_GUID: Guid = Guid {
    a: 0x9042_a9de,
    b: 0x23dc,
    c: 0x4a38,
    d: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

/// `EFI_GRAPHICS_OUTPUT_MODE_INFORMATION`: what one video mode looks like.
///
/// **`pixels_per_scan_line` is not `horizontal_resolution`** and treating them as one is the
/// classic framebuffer bug. Firmware is free to pad each row out to a convenient stride, so a
/// writer that multiplies by the width paints a picture that shears progressively down the screen.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct GraphicsModeInformation {
    /// The structure's own version. 0 for everything this loader has met.
    pub version: u32,
    /// Width, in pixels.
    pub horizontal_resolution: u32,
    /// Height, in pixels.
    pub vertical_resolution: u32,
    /// One of [`pixel_format`]'s constants.
    pub pixel_format: u32,
    /// The channel masks, meaningful only for [`pixel_format::BIT_MASK`].
    pub pixel_information: [u32; 4],
    /// **The stride, in pixels**, which is the row pitch and not the width. See the note above.
    pub pixels_per_scan_line: u32,
}

/// `EFI_GRAPHICS_PIXEL_FORMAT`.
pub mod pixel_format {
    /// Bytes in memory are R, G, B, unused. A little-endian `u32` is therefore `0xXXBBGGRR`.
    pub const RGBX: u32 = 0;
    /// Bytes in memory are B, G, R, unused. A little-endian `u32` is therefore `0xXXRRGGBB`, which
    /// is the order every colour constant in this tree is already written in.
    pub const BGRX: u32 = 1;
    /// Channels described by masks rather than named. Not supported here; see `uefi_loader`'s BUGS.
    pub const BIT_MASK: u32 = 2;
    /// **There is no linear framebuffer at all**: this adapter can only be drawn on with `Blt`,
    /// which is a boot-services call and therefore gone by the time the kernel runs.
    pub const BLT_ONLY: u32 = 3;
}

/// `EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE`: the mode the adapter is *in*, and where its pixels live.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct GraphicsOutputMode {
    /// How many modes `GraphicsOutput::query_mode` would accept.
    pub max_mode: u32,
    /// Which one is current.
    pub mode: u32,
    /// The current mode's geometry.
    pub info: *const GraphicsModeInformation,
    /// How many bytes of [`Self::info`] the firmware filled in.
    pub size_of_info: usize,
    /// **The physical address of the linear framebuffer, and it survives `ExitBootServices`.**
    ///
    /// That is the whole reason milestone 243 can use this: it is a bar aperture on the display
    /// adapter, not firmware memory, so nothing about ending the boot phase moves it or takes it
    /// away. What ends is the firmware's *console*, not the display.
    pub framebuffer_base: u64,
    /// How many bytes of it there are.
    pub framebuffer_size: usize,
}

/// `EFI_GRAPHICS_OUTPUT_PROTOCOL`, truncated after the one field this loader reads.
///
/// The three function pointers ahead of it are placeholders holding [`Self::mode`] at its specified
/// offset, exactly as in [`BootServices`]. This loader never changes the video mode: it takes
/// whatever the firmware left on the screen, because a mode set here is a mode the kernel would
/// have to be told about through a channel that does not exist yet, and because the firmware has
/// already picked one that works on this monitor.
#[allow(
    dead_code,
    reason = "the three leading entries are the firmware's, present only to put `mode` at its \
              specified offset"
)]
#[repr(C)]
pub struct GraphicsOutput {
    query_mode: usize,
    set_mode: usize,
    blt: usize,
    /// The current mode, and the framebuffer's address.
    pub mode: *const GraphicsOutputMode,
}

/// `EFI_LOADED_IMAGE_PROTOCOL_GUID`.
pub const LOADED_IMAGE_PROTOCOL_GUID: Guid = Guid {
    a: 0x5b1b_31a1,
    b: 0x9562,
    c: 0x11d2,
    d: [0x8e, 0x3f, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

/// `EFI_DEVICE_PATH_PROTOCOL_GUID`: a handle's device path, which is how the chooser tells the disk
/// its own file came from (`crate::device_path`).
pub const DEVICE_PATH_PROTOCOL_GUID: Guid = Guid {
    a: 0x0957_6e91,
    b: 0x6d3f,
    c: 0x11d2,
    d: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

/// `EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID`.
pub const SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: Guid = Guid {
    a: 0x964e_5b22,
    b: 0x6459,
    c: 0x11d2,
    d: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

/// `EFI_FILE_MODE_READ`.
pub const FILE_MODE_READ: u64 = 0x0000_0000_0000_0001;

/// **Seek here and the file protocol puts the position at the end**, which is the specification's
/// own way of asking a file how long it is without decoding an `EFI_FILE_INFO`.
pub const FILE_POSITION_END: u64 = u64::MAX;

/// `EFI_LOADED_IMAGE_PROTOCOL`, truncated after the one field this loader reads.
///
/// **[`Self::device_handle`] is the whole reason it is here**: it names the volume the firmware
/// loaded this image from, which is the volume the image's own file is on.
///
/// `image_base` and `image_size` are deliberately **not** read, and the reason is worth stating
/// because they look like the easy answer. They describe the image as *loaded*: sections placed at
/// their RVAs, with section alignment rather than file alignment between them. Writing those bytes
/// to a disk would produce something that is not this file and that no firmware promises to start.
#[allow(
    dead_code,
    reason = "the leading entries are the firmware's, present only to put `device_handle` at its \
              specified offset"
)]
#[repr(C)]
pub struct LoadedImage {
    revision: u32,
    parent_handle: Handle,
    system_table: *mut SystemTable,
    /// **The handle of the device this image was loaded from.** A `SimpleFileSystem` on it is the
    /// volume the file lives on.
    pub device_handle: Handle,
    /// The image's own path on that volume, as a device path. Not decoded here: turning one into
    /// text is `EFI_DEVICE_PATH_TO_TEXT_PROTOCOL`, a third protocol and a parser, and the removable
    /// media path is a constant. See `src/main.rs`'s `place_boot_file` BUGS note.
    pub file_path: *const c_void,
    reserved: *const c_void,
    /// Bytes at [`Self::load_options`]. Set by whoever called `LoadImage`, which for a chain-loaded
    /// nife image is the chooser.
    pub load_options_size: u32,
    /// **What the chooser told this image about itself.** `uefi_loader`'s chooser writes
    /// `chooser::STARTED_BY_CHOOSER` followed by the slot number here, and the started image reads
    /// it to know that it is the image being tried rather than the chooser doing the trying. It is
    /// the one thing that keeps a chooser from chain-loading itself forever.
    pub load_options: *const c_void,
    image_base: *const c_void,
    image_size: u64,
}

/// `EFI_SIMPLE_FILE_SYSTEM_PROTOCOL`.
#[repr(C)]
pub struct SimpleFileSystem {
    /// The protocol's own revision. Not checked: there has only ever been one.
    pub revision: u64,
    /// `OpenVolume(this, &mut root)`.
    pub open_volume: extern "efiapi" fn(*mut SimpleFileSystem, *mut *mut FileProtocol) -> Status,
}

/// `EFI_FILE_PROTOCOL`, in specification order, truncated after `set_position`.
///
/// The same rule as [`BootServices`]: the entries are found by offset, so the unused ones are
/// spelled out rather than collapsed, and their count is the ABI.
#[allow(
    dead_code,
    reason = "the unused entries hold the used ones at their specified offsets; their COUNT is the \
              ABI"
)]
#[repr(C)]
pub struct FileProtocol {
    revision: u64,
    /// `Open(this, &mut new, name, open_mode, attributes)`. `name` is NUL-terminated UTF-16.
    pub open: extern "efiapi" fn(
        *mut FileProtocol,
        *mut *mut FileProtocol,
        *const u16,
        u64,
        u64,
    ) -> Status,
    /// `Close(this)`.
    pub close: extern "efiapi" fn(*mut FileProtocol) -> Status,
    delete: usize,
    /// `Read(this, &mut size, buffer)`. **`size` is in/out**: the buffer's capacity going in, the
    /// bytes actually read coming out, and a short read is success rather than an error.
    pub read: extern "efiapi" fn(*mut FileProtocol, *mut usize, *mut u8) -> Status,
    write: usize,
    /// `GetPosition(this, &mut position)`.
    pub get_position: extern "efiapi" fn(*mut FileProtocol, *mut u64) -> Status,
    /// `SetPosition(this, position)`. [`FILE_POSITION_END`] seeks to the end.
    pub set_position: extern "efiapi" fn(*mut FileProtocol, u64) -> Status,
}

/// One entry of the UEFI configuration table: a GUID and a pointer.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ConfigurationTable {
    /// What the pointer points at.
    pub vendor_guid: Guid,
    /// The table itself, physical (this loader never calls `SetVirtualAddressMap`).
    pub vendor_table: *const c_void,
}

/// `EFI_SYSTEM_TABLE`, truncated after `configuration_table`.
#[allow(
    dead_code,
    reason = "as in `BootServices` above: the private fields are the table entries this loader \
              does not call, and their presence is what keeps the ones it does call at the right \
              offsets"
)]
#[repr(C)]
pub struct SystemTable {
    /// The table header.
    pub hdr: TableHeader,
    firmware_vendor: *const u16,
    firmware_revision: u32,
    console_in_handle: Handle,
    con_in: usize,
    console_out_handle: Handle,
    /// The text console, or null when the firmware has none.
    pub con_out: *mut SimpleTextOutput,
    standard_error_handle: Handle,
    std_err: *mut SimpleTextOutput,
    runtime_services: usize,
    /// Everything this loader calls before it hands over.
    pub boot_services: *mut BootServices,
    /// How many entries [`Self::configuration_table`] has.
    pub number_of_table_entries: usize,
    /// **Where the ACPI RSDP is found**, and the reason a UEFI boot needs no `"RSD PTR "` scan.
    pub configuration_table: *const ConfigurationTable,
}

/// `EFI_BLOCK_IO_PROTOCOL_GUID`.
pub const BLOCK_IO_PROTOCOL_GUID: Guid = Guid {
    a: 0x964e_5b21,
    b: 0x6459,
    c: 0x11d2,
    d: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

/// `LocateHandleBuffer`'s `ByProtocol` search: every handle carrying the named protocol.
pub const BY_PROTOCOL: u32 = 2;

/// `EFI_BLOCK_IO_MEDIA`, truncated after the last field this loader reads.
///
/// The five `BOOLEAN`s are spelled out one by one rather than collapsed for the same reason
/// [`BootServices`]' unused entries are: their count and their types are the ABI, and a `[u8; 5]`
/// would hide the padding that puts [`Self::block_size`] at offset 12.
#[allow(
    dead_code,
    reason = "the unread flags hold the read fields at their specified offsets"
)]
#[repr(C)]
pub struct BlockIoMedia {
    /// Changes when the medium is swapped. Every read and write must carry the current value, which
    /// is how the firmware refuses a request aimed at a disk that is no longer there.
    pub media_id: u32,
    removable_media: u8,
    /// **False on an empty optical drive**, which is the one handle a naive enumeration trips over.
    pub media_present: u8,
    /// **True for a partition, false for the disk it is on.** The chooser wants whole disks: a GPT
    /// lives at LBA 1 of a disk, and LBA 1 of a partition is somebody's filesystem.
    pub logical_partition: u8,
    /// True for a medium that will refuse every write, which the chooser checks before it promises
    /// itself that a try has been spent.
    pub read_only: u8,
    write_caching: u8,
    /// Bytes per logical block. 512 on everything this has been run on, and not assumed.
    pub block_size: u32,
    io_align: u32,
    /// The last addressable block, so the disk is `last_block + 1` blocks long.
    pub last_block: u64,
}

/// `EFI_BLOCK_IO_PROTOCOL`.
///
/// **Reads and writes are whole blocks and nothing else**: `size` must be a multiple of
/// [`BlockIoMedia::block_size`] and the buffer must be aligned, which is why every caller here
/// works out of a page the firmware allocated.
#[allow(
    dead_code,
    reason = "`reset` holds the three used methods at their specified offsets"
)]
#[repr(C)]
pub struct BlockIo {
    /// The protocol revision. Not checked: this loader reads only revision-1 fields.
    pub revision: u64,
    /// What the medium is.
    pub media: *const BlockIoMedia,
    reset: usize,
    /// `ReadBlocks(this, media_id, lba, size, buffer)`.
    pub read_blocks: extern "efiapi" fn(*mut BlockIo, u32, u64, usize, *mut u8) -> Status,
    /// `WriteBlocks(this, media_id, lba, size, buffer)`.
    pub write_blocks: extern "efiapi" fn(*mut BlockIo, u32, u64, usize, *const u8) -> Status,
    /// `FlushBlocks(this)`. **The chooser calls it before it hands off**, because a try it has
    /// spent and a controller has not written down is a try nothing spent.
    pub flush_blocks: extern "efiapi" fn(*mut BlockIo) -> Status,
}
