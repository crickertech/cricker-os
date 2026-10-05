//! **The MMU, `x86_64`.** The third implementation of the seam milestone 20 built: `crates/paging`
//! owns the level walk and `paging::x86_64::Ia32e` owns the entry format, and this module is the
//! glue that names `CR3`, the direct map, and where this machine's devices are.
//!
//! # Two bases, which is the one structural difference of the three architectures
//!
//! aarch64 and RISC-V put the kernel image and the direct map at a single base, so one constant and
//! one subtraction serve both. `x86_64` cannot: `x86_64-unknown-none` uses `code-model: kernel`,
//! which promises LLVM that every symbol is in the **top 2 GiB**, so the image is pinned at
//! [`KERNEL_VA_BASE`] (`0xffffffff80000000`) and there is no room above it to name more than 2 GiB
//! of physical memory. `VA = PA | KERNEL_VA_BASE` was therefore never going to reach a real
//! machine's RAM, and it is not even invertible above the line: the local APIC at `0xfee00000` maps
//! to a distinct address whose inverse is not `0xfee00000`.
//!
//! So the two jobs that constant was doing are split, exactly as Linux splits them (milestone 161,
//! item 1 of the roadmap's open list):
//!
//! | What | Base | Who needs it there |
//! |---|---|---|
//! | The kernel **image** | [`KERNEL_VA_BASE`] `0xffffffff80000000` | the code model, non-negotiably |
//! | The **direct map** of physical memory | [`DIRECT_MAP_BASE`] `0xffff888000000000` | [`phys_to_virt`], and it has room for 64 TiB |
//!
//! They are separate PML4 entries (511 and 273), so nothing about them interferes, and both are
//! canonically high, so `Ia32e::is_in_half(Half::High, ..)` admits both by the same bit-47 test.
//! The `paging` crate needed no change for either, which is the claim of milestone 20 (a portable
//! HAL, proven on a second architecture) holding a second time.
//!
//! [`phys_to_virt`] is the direct map's arithmetic and [`virt_to_phys`] inverts **both** bases,
//! because the kernel asks it about image addresses (`memory::image_start`, linker symbols) as well
//! as about direct-map ones. That two-branch shape is Linux's `__pa()`, for the same reason.
//!
//! # Why `boot.s` installs the direct map, and not this module
//!
//! The roadmap flagged one hazard here and it is real: `phys_to_virt` must not change meaning under
//! anything that already used it. The frame allocator's bitmap is the sharp case, because
//! `memory::bring_up_page_frames` turns a physical address into a `&'static mut [u8]` and **stores it**,
//! long before there is a fine map; the PVH structure and the ACPI tables are read the same way.
//!
//! Rather than sequence around that (map the old alias too, switch, then drop it), `boot.s` gives
//! the boot tables a PML4[273] entry pointing at the same low PDPT the identity map uses. The
//! direct map therefore exists from before the first Rust instruction, covering the low 4 GiB, and
//! [`init`] widens it rather than introducing it. **`phys_to_virt` means one thing for the
//! machine's whole life**, so the hazard stops existing instead of being documented for the next
//! reader to remember. The cost is one PML4 entry and no memory at all.
//!
//! What [`init`] *does* drop is the identity map, which was a complete alias of physical memory
//! sitting in the half user programs will get.
//!
//! # BUGS
//!
//! - **The direct map's RAM is built out of blocks, but its memory type is not checked against the
//!   MTRRs.** Each RAM claim is mapped with the largest leaf that fits ([`largest_leaf`]: 1 GiB
//!   where `CPUID` offers `Page1GB`, else 2 MiB), which took QEMU's 256 MiB from 560 KiB of page
//!   tables to 60 KiB (milestone 161; 4 KiB leaves cost 0.2% of RAM, ~64 MiB on a 32 GiB
//!   machine). The SDM leaves the memory type undefined when one large page spans two MTRR ranges
//!   of different types, and this kernel never reads the MTRRs, so it relies on firmware keeping
//!   every range it calls usable RAM write-back throughout, which is what firmware does and what
//!   Linux checks rather than assumes (`mtrr_type_lookup`'s "uniform" answer). Device windows and
//!   firmware reservations stay in 4 KiB leaves for the same reason: they are where mixed types
//!   live, and they are a few hundred pages.
//!
//! - **`mmu::init` must allocate its page tables from the low 4 GiB**, because that is all the boot
//!   tables' direct map reaches and the mapper writes every table through it. The frame allocator
//!   hands out the lowest free frame first, so this holds today on any machine; a machine whose RAM
//!   *starts* above 4 GiB would fault inside `map_everything`. Widening the boot map to 1 GiB
//!   leaves (one PDPT covers 512 GiB) is the fix, and it needs a `CPUID` check for `PDPE1GB`.
//!
//! - **`CR4.PGE` is off, so the `G` bit the kernel's `Flags` set is ignored.** Every `mov cr3`
//!   therefore discards the kernel's own translations as well as the outgoing process's, which is
//!   correct and costs a TLB refill of whatever kernel pages the next thread touches. **Measured
//!   2026-09-19 (milestone 161) and left off, because the only instrument this tree has cannot see
//!   the thing PGE changes.** QEMU 11.0.2's `cpu_x86_update_cr3` (`target/i386/helper.c`) calls
//!   `tlb_flush` on every `CR3` write, whatever `PGE` and `PCIDE` say, so under TCG a global entry
//!   never survives a switch; and icount counts retired guest instructions, which a TLB refill is
//!   not. The x86 bench with an initrd (so `ctx_switch`, two processes and a `CR3` write per
//!   switch, runs) printed **byte-identical tick counts on every line** with PGE on and off, which
//!   is the proof of the first half rather than a result about the second. notes/benchmarks.md's
//!   2026-09-19 section has the numbers and what would measure it: KVM on cordoba (blocked on one
//!   `usermod`) or a bench boot on xenon. Turning it on is a two-line change after `install` in
//!   [`init`] and [`init_secondary`] (after the fine map, or the boot map's entries are the ones
//!   pinned), and nothing else here depends on it being off: kernel mappings are already global,
//!   user mappings already are not, and every kernel unmap already ends in `invlpg`, which does
//!   invalidate a global entry.
//!
//! - **`CR4.PCIDE` is off, so an address space has no hardware tag.** `crates/address_space_identifier` hands every
//!   space a number and this architecture has nowhere to put it: PCID is `CR3[11:0]`, and with
//!   PCIDE clear those bits are reserved-zero rather than a tag. [`ttbr0_value`] therefore drops the
//!   number and [`flush_asid`] flushes the whole TLB rather than one space's entries. Both say so in
//!   their own words; neither pretends to a selectivity the hardware does not have. **Unmeasured for
//!   the same reason as PGE, and not a bit to flip**: with PCIDE on, `invlpg` and a plain `CR3`
//!   write act on the *current* PCID only, so [`unmap_user_at`] on a space that is not installed,
//!   [`flush_asid`] and the NMI shootdown's discard-everything arm would each leave another space's
//!   tagged entries alive. That is a stale-translation defect (memory reading back as its previous
//!   owner's), and closing it is `INVPCID` (checked in `CPUID`) on every one of those paths plus a
//!   rule for tag reuse, which is a milestone rather than a line.
//!
//! - **The cacheable fill follows the firmware's map only as far as that map describes memory
//!   *contiguously*** ([`firmware_fill_ceiling`]). That is a claim about how firmware writes
//!   memory maps, not an architectural guarantee: DRAM and the carve-outs taken out of it abut
//!   each other, and the 32-bit MMIO hole above them is a gap. A machine whose map had an
//!   undescribed gap *inside* low DRAM, with ACPI tables above it, would have those tables left
//!   out of the direct map and would fault reading one. No map in this tree has that shape and
//!   xenon's does not (`notes/x86-uefi-boot.md` transcribes it). The bound it replaced was "the
//!   top of RAM", which was the same number on every machine whose RAM ends below the hole and
//!   swallowed every MMIO window on the first machine whose RAM does not.
//!
//! - **An aperture that firmware reports as usable RAM would panic, and that is deliberate.** It
//!   was the leading hypothesis for the 2026-09-05 `AlreadyMapped` and it was wrong: xenon's
//!   framebuffer is in the MMIO hole, in no RAM region. If a machine ever did report both, the fix
//!   is to exclude the aperture from the RAM direct map *and* from the frame allocator, not to
//!   skip the second mapping and not to remap the frames device-typed. Skipping leaves the
//!   aperture cacheable, and this architecture leaves the effective memory type **undefined** when
//!   one frame is reachable through two mappings with conflicting types, so that is wrong rather
//!   than merely slow. Remapping would give one correct mapping and leave the allocator handing
//!   out frames the display adapter answers at. Until a machine forces the question, the panic
//!   names both ranges and is the better answer.

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use paging::x86_64::Ia32e;
use paging::{Flags, Half, MapError, Mapper, PAGE_SIZE, PageSize, PageTable};

use crate::memory;

/// This architecture's page-table format. Portable code names it as `arch::mmu::Format`.
pub type Format = Ia32e;

/// The base of the kernel **image**'s virtual addresses.
///
/// **Fixed at 0xffffffff80000000 by the target's code model**, not chosen for taste: see the module
/// header and link-x86_64.ld. It decomposes as PML4[511], PDPT[510], PD[0], which is why `boot.s`
/// can alias the first gigabyte into the high half with a single PDPT entry.
///
/// This is deliberately **not** where physical memory is mapped; see [`DIRECT_MAP_BASE`].
pub const KERNEL_VA_BASE: u64 = 0xffff_ffff_8000_0000;

/// **The base of the direct map: where physical address `pa` is readable as `pa + this`.**
///
/// `0xffff888000000000`, which is Linux's `page_offset_base`, taken rather than invented because a
/// reader who has met one `x86_64` kernel has met this number. What the value has to satisfy is
/// short: canonically high (bit 47 set and sign-extended, so the same `Ia32e::is_in_half` test that
/// admits the kernel image admits this), clear of PML4[511] where the image lives, clear of the low
/// half that ring 3 will get, and with room above it for all of physical memory. This has 64 TiB of
/// room before it would reach anything else, which is 2^46 and not a limit worth thinking about.
///
/// **`boot.s` installs the PML4 entry for this before any Rust runs**, so the arithmetic below is
/// valid from the first instruction rather than from [`init`]; the module header says why that
/// matters more than it looks like it should.
///
/// Name: provisional (milestone 161 (the `x86_64` kernel port)): calef names the constants, and
/// this one was minted by a lane. The tree's analogous name is [`KERNEL_VA_BASE`], which is why
/// this is `_BASE` too.
pub const DIRECT_MAP_BASE: u64 = 0xffff_8880_0000_0000;

/// The top-level (PML4) index the direct map occupies, **duplicated in `boot.s`** because a 32-bit
/// assembler cannot compute it from the constant above. This is the gate that keeps the two in
/// agreement: change [`DIRECT_MAP_BASE`] without changing `boot.s` and the kernel does not build.
/// **Where kernel thread stacks live, virtually** (`thread.rs`'s `STACK_AREA`).
///
/// `0xffff_c900_0000_0000` is **Linux's `VMALLOC_START`**, taken rather than invented for the same
/// reason [`DIRECT_MAP_BASE`] is Linux's `page_offset_base`: a reader who has met one `x86_64` kernel
/// has met this number, and the alternative is a constant nobody can check against anything.
///
/// **The portable expression the other two architectures use does not survive here**, and the way
/// it fails is silent. `thread.rs` computed this as `KERNEL_VA_BASE | 0x10_0000_0000` ("64 GiB above
/// the direct map"), which works where the kernel base is a *half* base with room above it.
/// [`KERNEL_VA_BASE`] here is `0xffff_ffff_8000_0000`, which already has every bit of
/// `0x10_0000_0000` set, so the OR was the identity: every kernel thread stack would have been
/// mapped at the kernel image's own base, over `.text`. Caught by reading rather than by booting,
/// because the mapper's overwrite refusal turns it into `kmem: no memory to wire one` far from the
/// cause.
///
/// PML4[402], which is nothing else's: the image is PML4[511] and the direct map PML4[273].
///
/// Name: provisional (milestone 161, roadmap item 4).
pub const THREAD_STACK_AREA: u64 = 0xffff_c900_0000_0000;

const DIRECT_MAP_PML4_INDEX: u64 = 273;

const _: () = {
    assert!(
        DIRECT_MAP_BASE >> 47 == 0x1_ffff,
        "the direct map base must be canonically high, or every address in it faults"
    );
    assert!(
        (DIRECT_MAP_BASE >> 39) & 0x1ff == DIRECT_MAP_PML4_INDEX,
        "boot.s writes PML4[273]: change both or neither"
    );
    assert!(
        (KERNEL_VA_BASE >> 39) & 0x1ff != DIRECT_MAP_PML4_INDEX,
        "the image and the direct map must not share a PML4 entry"
    );
};

/// The kernel's view of physical address `pa`: its address in the direct map.
pub const fn phys_to_virt(pa: u64) -> u64 {
    pa + DIRECT_MAP_BASE
}

/// The physical address behind kernel virtual address `va`.
///
/// **Two branches, because this architecture has two kernel bases** (see the module header). An
/// address at or above [`KERNEL_VA_BASE`] is part of the kernel image, linked there by
/// link-x86_64.ld; anything else in the high half is a direct-map address. Both callers exist:
/// `memory::image_start` hands this a linker symbol, and `sched`/`kmem` hand it pointers that came
/// out of [`phys_to_virt`]. Linux's `__pa()` makes the same distinction for the same reason.
///
/// It is the exact inverse of [`phys_to_virt`] over the direct map, now for **all** of physical
/// memory rather than the low 2 GiB the single-base arithmetic could reach.
pub const fn virt_to_phys(va: u64) -> u64 {
    if va >= KERNEL_VA_BASE {
        va - KERNEL_VA_BASE
    } else {
        va - DIRECT_MAP_BASE
    }
}

/// COM1's I/O port on every x86 machine since the PC/AT, and on QEMU's `q35`. Not an address in the
/// memory space at all; see `arch/x86_64/port.rs` for what that means and why the console reaches it
/// through a different accessor than the other two architectures' UARTs.
pub const COM1_PORT: usize = 0x3f8;

/// The local APIC's default physical base. Relocatable through `IA32_APIC_BASE`, and nothing
/// relocates it; the constant is where the machine puts it at reset.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const LOCAL_APIC_PHYS: u64 = 0xfee0_0000;

/// The IO APIC's default physical base on a PC-compatible machine, used only when the ACPI MADT
/// did not say. `irq::io_apic_phys()` is what the machine actually reported, and q35 reports
/// exactly this.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const IO_APIC_PHYS: u64 = 0xfec0_0000;

/// **Where q35 puts the PCIe ECAM window**, and QEMU's q35 default. `map_everything` no longer
/// maps this constant directly: the real window is read from `memory::pci_regions()`, which
/// `main.rs` fills from ACPI's MCFG the same way `memory::init` fills it from a
/// `pci-host-ecam-generic` node on the other two architectures. This stays as the value
/// `machine::print_acpi_summary` prints the MCFG's own answer against, which is the second
/// witness aarch64 and riscv64 also kept (their old device-tree hardcodes, held equal to the
/// discovered value by `pci.rs`'s own test).
///
/// **The asymmetry is worth knowing before reading a boot log.** Those two architectures only ever
/// boot QEMU `virt`, so there the second witness is an *equality invariant* and a difference would
/// be a finding. Here it is not, and not only on real hardware. Firmware places this window, so the
/// two values agree on QEMU q35 **under `SeaBIOS`** and nowhere else that has been measured: OVMF
/// relocates it to `0xe000_0000`, which is what both of `script/test --arch x86_64`'s real-firmware
/// boots print, and xenon's own MCFG says `0xf000_0000` over 128 buses. That is why
/// `print_acpi_summary` names q35 rather than this constant when it prints the comparison
/// (milestone 314); the earlier wording read as a defect report everywhere except the one firmware
/// it was written against, and was eventually reported as one.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const PCI_ECAM_PHYS: u64 = 0xb000_0000;

/// How many PCI buses of the ECAM window the kernel maps **before it has asked the machine**. One
/// bus is 1 MiB of configuration space, and the MCFG on the first real machine this port met
/// describes a hundred and twenty-eight of them.
///
/// **The floor, not the answer, since milestone 320.** `pci::ecam_buses()` is what the kernel maps
/// and reads: this value until `pci::survey` has run, and the topology the survey found afterwards.
/// The survey runs from `kernel_main` while the boot tables still cover the low 4 GiB
/// indiscriminately, which is the only window in the boot where every bus the MCFG describes is
/// readable; `map_everything` runs after it and maps what it found.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const PCI_ECAM_BUSES: u16 = 1;

// Deleted 2026-09-30 by the CI-warnings lane: `is_enabled` (was paging on? read back CR0.PG) had
// no caller in any configuration on this, its only, architecture. The boot print that asks the
// question is riscv's ("paging on", in `lib.rs`'s riscv boot); if the x86_64 boot ever wants the
// same self-test, that print is the caller to write and git has the body.

/// The physical address of the page-table root the CPU is currently walking (`CR3`, with the
/// PCID/flag bits masked off).
pub fn current_root() -> u64 {
    read_cr3() & 0x000f_ffff_ffff_f000
}

/// Print what the MMU is doing, one line, on every boot. The x86 twin of the other two
/// architectures' summaries.
pub fn print_summary() {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    if root == 0 {
        crate::println!(
            "  mmu             : 4-level paging on (cr3 {:#x}), boot map: 4 GiB identity + high alias",
            current_root(),
        );
    } else {
        crate::println!(
            "  mmu             : fine W^X 4-level map installed (cr3 {:#x}), image {:#x}, direct map {:#x}",
            current_root(),
            KERNEL_VA_BASE,
            DIRECT_MAP_BASE,
        );
        // The cost, measured rather than asserted, and the leaf size that set it: RAM goes in the
        // largest block the machine offers and everything else in 4 KiB pages (this module's
        // BUGS). On QEMU's 256 MiB it was 560 KiB in pages alone.
        crate::println!(
            "                  : {} KiB of page tables (ram in {} blocks), no identity map, guard pages are holes",
            TABLE_FRAMES.load(Ordering::Relaxed) * PAGE_SIZE / 1024,
            match largest_leaf() {
                PageSize::Size1GiB => "1 GiB",
                PageSize::Size2MiB => "2 MiB",
                PageSize::Size4KiB => "4 KiB",
            },
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The fine-grained kernel map (milestone 161, roadmap item 1). The third instance of the same
// shape aarch64 and riscv64 already have: build a root, walk it with `paging::Mapper`, check the
// things that would kill us in software, then hand the root to the hardware.
// ---------------------------------------------------------------------------------------------

/// The kernel's fine-map root, saved by [`init`] so a secondary CPU can adopt it and so the kernel
/// mapper does not have to read `CR3` back (which will name a *process* root once ring 3 exists).
static KERNEL_ROOT: AtomicU64 = AtomicU64::new(0);

/// How many frames the fine map cost, root and intermediate tables together. Reported on every
/// boot rather than left to be estimated: it is the number blocks exist to shrink.
static TABLE_FRAMES: AtomicU64 = AtomicU64::new(0);

/// **The largest leaf this machine's direct map may use**: 1 GiB where `CPUID` leaf 0x80000001
/// reports `Page1GB` (EDX bit 26), 2 MiB everywhere else, since every long-mode CPU has 2 MiB
/// pages. Asked of the hardware rather than assumed, because a `PS` bit in a PDPT entry on a CPU
/// without `Page1GB` is a reserved bit, and the fault it raises is taken through tables that
/// cannot describe the fault handler. The extended range's own maximum (leaf 0x80000000) is
/// checked first, because a leaf past it returns another leaf's answer rather than zeros.
fn largest_leaf() -> PageSize {
    const PAGE_1GB: u32 = 1 << 26;
    if super::isa::cpuid(0x8000_0000).eax >= 0x8000_0001
        && super::isa::cpuid(0x8000_0001).edx & PAGE_1GB != 0
    {
        PageSize::Size1GiB
    } else {
        PageSize::Size2MiB
    }
}

/// Serializes edits to the kernel's live tables: two CPUs must not mutate them at once. Same role
/// and lock rank as the other two architectures' `KERNEL_MMU`.
static KERNEL_MMU: crate::sync::IrqSafeMutex<()> =
    crate::sync::IrqSafeMutex::new(crate::sync::rank::KERNEL_MMU, ());

/// **Replace the boot map with a fine-grained one**: W^X kernel sections, the guard pages actually
/// unmapped, device registers device-typed, a direct map that covers all of physical memory, and
/// **no identity map** (which was a complete alias of physical memory in the half user programs
/// get).
///
/// We are already executing in the high half on the boot table, and the fine table maps the same
/// kernel VAs (image and direct map alike) to the same frames, so the `mov cr3` is seamless: the
/// next instruction fetch resolves identically. That is what [`verify`] checks before the hardware
/// bets the machine on it.
///
/// **The console does not enter into it here, unlike on the other two architectures.** x86's COM1 is
/// an I/O port, not a mapping, so nothing this function does can make the machine go silent. That is
/// a real convenience while debugging and it is also the reason a port capability is an open
/// question (DECISIONS §121).
pub fn init() {
    let free_before = memory::free_page_frames();
    let root = memory::alloc()
        .expect("no frame for the root page table")
        .addr();
    // SAFETY: a fresh frame from the allocator, reachable through the boot tables' direct map. Zero
    // it before the hardware could ever walk it.
    unsafe {
        (*phys_to_ptr(root)).entries = [0; paging::ENTRIES];
    }

    // SAFETY: `root` is zeroed and page-aligned; `phys_to_ptr` is valid because `boot.s` installed
    // the direct map over the low 4 GiB before any Rust ran, so every frame the mapper allocates is
    // addressable through it (see this module's BUGS for the >4 GiB case).
    let mut mapper = unsafe {
        Mapper::<_, _, Ia32e>::new(
            root,
            Half::High,
            || memory::alloc().map(|f| f.addr()),
            phys_to_ptr,
        )
    };

    if let Err(f) = map_everything(&mut mapper) {
        // Not `expect`: the message is the whole transcript on a machine with no serial cable, so
        // it names the two ranges rather than the symptom. See `MapFailure`.
        panic!("failed to build the kernel page tables: {f}");
    }
    verify(&mapper);
    TABLE_FRAMES.store(
        (free_before - memory::free_page_frames()) as u64,
        Ordering::Relaxed,
    );

    // SAFETY: `verify` just walked these tables and asserted they cover this function's own code,
    // this CPU's stack, and the direct map every pointer already in flight was derived through.
    unsafe { install(root) };

    KERNEL_ROOT.store(root, Ordering::Relaxed);
}

/// Install the tables rooted at physical `root` by writing `CR3`.
///
/// Writing `CR3` invalidates every non-global TLB entry on this CPU, and `CR4.PGE` is clear, so in
/// practice it invalidates everything: there is no stale boot-table translation left afterwards.
///
/// # Safety
/// `root` must be a complete kernel map covering the currently-executing code, the current stack,
/// and any memory touched before the next barrier; otherwise the instruction after the write is
/// fetched through a table that does not describe it, which on this architecture is a page fault
/// escalating to a triple fault and a silent machine reset.
unsafe fn install(root: u64) {
    // SAFETY: the caller's contract.
    unsafe { super::instructions::write_cr3(root) };
}

/// Adopt the kernel map on a secondary CPU. Every CPU shares one kernel root (`CR3` names the whole
/// address space, unlike `TTBR0`/`TTBR1`'s split), so there is nothing per-CPU to build: the
/// trampoline (`boot.s`'s `secondary_boot`) already installed `boot_pml4`, and this just switches
/// to the same fine root the boot core is already running on. Called from `secondary_main`'s step
/// 3, same as the other two architectures.
pub fn init_secondary() {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    assert_ne!(
        root, 0,
        "a secondary CPU reached init_secondary before init"
    );
    // SAFETY: the primary built this root, verified it, and is running on it.
    unsafe { install(root) };
}

/// The kernel's live page tables, as a `Mapper`. **Call only while holding [`KERNEL_MMU`].**
#[allow(clippy::type_complexity)]
fn kernel_mapper() -> Mapper<impl FnMut() -> Option<u64>, fn(u64) -> *mut PageTable, Ia32e> {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    // SAFETY: `root` is the fine kernel table `init` built; the direct map makes `phys_to_ptr` valid
    // for every table frame.
    unsafe {
        Mapper::new(
            root,
            Half::High,
            || memory::alloc().map(|f| f.addr()),
            phys_to_ptr,
        )
    }
}

/// Map one page in the kernel's address space.
pub fn map_page(va: u64, pa: u64, flags: Flags) -> Result<(), MapError> {
    let _guard = KERNEL_MMU.lock(); // exclusive: two CPUs must not mutate the tables at once
    kernel_mapper().map(va, pa, flags)
}

/// Unmap one page in the kernel's address space, invalidate the TLB, and return the physical frame
/// (the caller's to free; the mapper never owned it).
#[allow(dead_code)]
pub fn unmap_page(va: u64) -> Result<u64, MapError> {
    let _guard = KERNEL_MMU.lock(); // exclusive: see map_page
    let (pa, flush) = kernel_mapper().unmap(va)?;
    flush.flush(flush_tlb);
    Ok(pa)
}

/// Invalidate the TLB entry for `va`, **on every online CPU**.
///
/// The x86 instruction is `invlpg`, which unlike aarch64's `tlbi ..., is` is local to the CPU that
/// executes it. So this is the local invalidate plus [`shoot_down_others`], the same shape RISC-V's
/// `flush_tlb` has (an `sfence.vma` plus an SBI RFENCE) and for the same reason: without the remote
/// half, a core that cached a translation for `va` keeps using it after this one has handed the
/// frame away.
///
/// Local first, so this core's own page-table write is retired before anyone is told to look.
pub fn flush_tlb(va: u64) {
    // Getting TLB maintenance wrong means a stale translation, which is the memory-unsafety that
    // matters here rather than Rust's.
    super::instructions::invlpg(va);
    shoot_down_others(va);
}

/// The [`SHOOTDOWN_VA`] value meaning "discard everything", as opposed to one page. `u64::MAX` is
/// not a canonical address, so it can never be a real `va` and needs no separate flag word.
const SHOOTDOWN_ALL: u64 = u64::MAX;

/// **The one shootdown in flight**, and the lock that makes it one.
///
/// A raw `AtomicBool` rather than an [`crate::sync::IrqSafeMutex`], deliberately, and the reason is
/// the whole design below: this is taken from inside `KERNEL_MMU`, which has already masked
/// interrupts, so there is nothing left to mask and no rank to check. A core spinning here is still
/// reachable, because the message that ends the spin is an NMI.
static SHOOTDOWN_LOCK: AtomicBool = AtomicBool::new(false);

/// The payload of the round in flight, read according to [`SHOOTDOWN_KIND`]: a virtual address (or
/// [`SHOOTDOWN_ALL`]) for [`KIND_PAGE`], and a packed `(base, count)` port range for
/// [`KIND_PORT_GRANT`]. Written under [`SHOOTDOWN_LOCK`], published by the `Release` store to
/// `SHOOTDOWN_PENDING` that follows it.
///
/// **One word for two kinds rather than two words**, because only one round is ever in flight and a
/// second static would be a second thing to keep in step with the first. The name kept its `_VA`
/// spelling: it is what every existing reader calls it, and renaming a static that carries the older
/// meaning in every one of them would be a naming decision this lane does not get to make.
static SHOOTDOWN_VA: AtomicU64 = AtomicU64::new(0);

/// **What the cores named by [`SHOOTDOWN_PENDING`] are being asked to do**, added by
/// milestone 315 (a port revoke that reaches every core). Until then the answer was always
/// "invalidate a translation" and did not need saying.
///
/// Written under [`SHOOTDOWN_LOCK`] and published by the same `Release` store to
/// `SHOOTDOWN_PENDING` that publishes [`SHOOTDOWN_VA`], so a handler that sees its own bit has seen
/// both halves of the message.
static SHOOTDOWN_KIND: AtomicU8 = AtomicU8::new(KIND_PAGE);

/// [`SHOOTDOWN_KIND`]: discharge [`SHOOTDOWN_VA`] from the TLB, one page or everything.
const KIND_PAGE: u8 = 0;

/// [`SHOOTDOWN_KIND`]: take the port range packed into [`SHOOTDOWN_VA`] out of this core's TSS I/O
/// permission bitmap if it is the one installed there (milestone 315). Nothing about memory, and it
/// shares this protocol for one reason: the NMI is the only message on x86 that reaches a core with
/// interrupts masked, which is where a revoked holder's core routinely is.
const KIND_PORT_GRANT: u8 = 1;

/// Pack a port range into [`SHOOTDOWN_VA`]'s word. Both halves are `u16`, so this is lossless and
/// needs no sentinel: a range is never confused with an address because [`SHOOTDOWN_KIND`] says
/// which one the word is.
const fn pack_port_range(base: u16, count: u16) -> u64 {
    (base as u64) << 16 | count as u64
}

/// **Which cores have not yet acknowledged**, as a bitmask of cpu ids. The sender sets it and spins
/// until it reads zero; each target clears its own bit from the NMI handler. A mask rather than a
/// countdown so that an NMI from any other source (or a late one from the previous round) finds its
/// bit already clear and is a no-op instead of an acknowledgement nobody owed.
static SHOOTDOWN_PENDING: AtomicUsize = AtomicUsize::new(0);

/// **Make every other online CPU discharge `va` from its TLB, and do not return until they have.**
///
/// # Why this is an NMI and not an ordinary IPI
///
/// This is called from inside [`unmap_page`], which holds `KERNEL_MMU`, an `IrqSafeMutex`: **this
/// core reaches here with interrupts masked, and so does every other core running the same code.**
/// A maskable IPI would therefore deadlock on the first concurrent spawn-and-reap: core A holds
/// `KERNEL_MMU` and waits for core B's acknowledgement, while core B spins for `KERNEL_MMU` with
/// interrupts off and can never take the message that would let A finish. That is not a corner
/// case; it is what two cores running the thread tests do continuously.
///
/// notes/riscv-tlb-shootdown.md already names the property that makes the RISC-V protocol work,
/// and it is exactly the one at issue: *"The IPI arrives as an M-mode software interrupt, so a hart
/// with S-mode interrupts masked still services it. That is not a footnote: without it, any kernel
/// code that disables interrupts and spins would deadlock whoever was flushing, and this kernel
/// disables interrupts routinely."* RISC-V gets that from a privilege level below the kernel's, and
/// aarch64 does not need it at all because `tlbi ..., is` is broadcast by the hardware. x86 has
/// neither, and has exactly one delivery mode `cli` cannot suppress, so the NMI is not a stylistic
/// choice here: it is the only message that keeps the guarantee.
///
/// # What it does not do
///
/// Nothing before the local APIC is up, and nothing when this is the only online core: boot maps a
/// great many pages with nobody to tell.
///
/// # BUGS
///
/// **One page per round trip.** A thread reap unmaps [`crate::thread::STACK_PAGES`] pages and pays
/// for six full shootdowns, where a batched protocol (collect the range, send once) would pay for
/// one. Correct and unbatched was chosen over fast and first: the batched version needs `unmap_page`
/// to hand its caller an undischarged obligation, which is the one thing `paging::TlbFlush` exists
/// to prevent. Worth revisiting with `script/bench` numbers rather than by argument.
fn shoot_down_others(va: u64) {
    broadcast(KIND_PAGE, va);
}

/// **Take a port range out of every other online core's TSS bitmap** (milestone 315), the remote
/// half of `segments::revoke_port_grant_everywhere`, which carries the defect this closes and the
/// measurement that found it.
///
/// It is [`shoot_down_others`]'s protocol with a different step at the far end, and reusing it is
/// the point: the acknowledgement mask, the single-round lock and the NMI delivery are the parts
/// that are hard to get right, and there is no second copy of them to keep in step.
pub(super) fn revoke_port_grant_others(base: u16, count: u16) {
    broadcast(KIND_PORT_GRANT, pack_port_range(base, count));
}

/// Send `kind` with `word` to every other online core and spin until each has acknowledged. The
/// body [`shoot_down_others`] and [`revoke_port_grant_others`] share; the doc above
/// `shoot_down_others` is the design, because that is the caller the design was written for.
fn broadcast(kind: u8, word: u64) {
    // Nothing to tell, or nothing to tell it with. `is_local_apic_ready` is the earlier of the two:
    // `mmu::init` maps the whole machine before `init_local_apic` runs.
    if !super::irq::is_local_apic_ready() {
        return;
    }
    let others = crate::smp::online_harts_mask() & !(1usize << crate::cpu::id());
    if others == 0 {
        return;
    }

    // One round at a time. See this static's own comment for why a raw spin is the right lock here
    // and not a lapse: a core waiting here is still reachable by the only message that matters.
    while SHOOTDOWN_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }

    SHOOTDOWN_KIND.store(kind, Ordering::Relaxed);
    SHOOTDOWN_VA.store(word, Ordering::Relaxed);
    // Release: this publishes the two stores above. A handler that sees its own bit set has, by the
    // matching `Acquire` load, also seen the message that bit is about.
    SHOOTDOWN_PENDING.store(others, Ordering::Release);

    for cpu in cpu_set::cpus_in(others) {
        // The local APIC id *is* the cpu id on this port: `smp::seat_cpus_from_acpi` seats every
        // core at the slot its own APIC id names, which is what makes `smp::hwid(id) == Some(id)`
        // for a filled slot. Asserted rather than assumed, because the whole protocol is addressed
        // by this number.
        debug_assert_eq!(
            crate::smp::hwid(cpu),
            Some(cpu as u64),
            "shootdown addressed cpu {cpu}, whose local APIC id is not its own id"
        );
        super::irq::send_nmi(cpu as u8);
    }

    while SHOOTDOWN_PENDING.load(Ordering::Acquire) != 0 {
        core::hint::spin_loop();
    }

    SHOOTDOWN_LOCK.store(false, Ordering::Release);
}

/// **Serve a shootdown NMI**: the receiving half of [`shoot_down_others`].
///
/// Called from the trap path for vector 2 and from nowhere else. Returns whether this NMI was one
/// of ours, so the caller can tell a shootdown from an NMI this kernel did not send.
///
/// # It may not touch anything reached through `gs`
///
/// An NMI lands wherever it lands, and `trap.s` documents one window where `IA32_GS_BASE` still
/// holds the *user's* value while `cs` says ring 0 (between the exit `swapgs` and the `iretq`). In
/// that window `cpu::id()`, which reads that MSR, would answer with somebody else's arithmetic. So
/// this core names itself from the local APIC's own id register instead, which is hardware ground
/// truth and needs no per-CPU pointer to read. Nothing else here dereferences per-CPU state, takes
/// a lock, or prints.
pub fn serve_shootdown_nmi() -> bool {
    let me = super::irq::local_apic_id() as usize;
    let bit = 1usize << me;

    // Acquire: pairs with the sender's `Release` store, so seeing our bit means seeing its `va`.
    if SHOOTDOWN_PENDING.load(Ordering::Acquire) & bit == 0 {
        return false;
    }

    let word = SHOOTDOWN_VA.load(Ordering::Relaxed);
    match SHOOTDOWN_KIND.load(Ordering::Relaxed) {
        // The port revoke (milestone 315). It names this core by the APIC id read above rather than
        // by `cpu::id()`, for the reason this function's own heading gives.
        KIND_PORT_GRANT => {
            super::segments::revoke_installed_port_grant_on(me, (word >> 16) as u16, word as u16);
        }
        // SAFETY: rewriting CR3 with the value it already holds changes no mapping and invalidates
        // every non-global entry, which with `CR4.PGE` clear is every entry. See `flush_asid`.
        _ if word == SHOOTDOWN_ALL => unsafe { install(read_cr3()) },
        _ => super::instructions::invlpg(word),
    }

    // Release: the invalidate above is complete before the sender may observe the acknowledgement
    // and go on to hand the frame to somebody else.
    SHOOTDOWN_PENDING.fetch_and(!bit, Ordering::Release);
    true
}

/// Translate a kernel virtual address through the live kernel tables.
pub fn translate(va: u64) -> Option<(u64, Flags)> {
    let _guard = KERNEL_MMU.lock();
    kernel_mapper().translate(va)
}

/// **Is this kernel address mapped, asked without taking a lock?**
///
/// For fault handlers, which is why it does not go through [`translate`]: a handler that has already
/// lost the machine must not block on a lock whose holder might be the thread that just died. Same
/// disposition and same caller (`stack::print_text_words`) as the RISC-V twin.
pub fn is_mapped(va: u64) -> bool {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    if root == 0 {
        // Before `init`, the boot map covers the low 4 GiB three ways over and nothing is a hole.
        return true;
    }
    // SAFETY: `root` is the live kernel root; the direct map makes `phys_to_ptr` valid; a translate
    // allocates nothing, so the `|| None` allocator is never called.
    let mapper = unsafe { Mapper::<_, _, Ia32e>::new(root, Half::High, || None, phys_to_ptr) };
    mapper.translate(va).is_some()
}

/// The first megabyte: real-mode legacy that is nonetheless memory, and the only part of the address
/// space this kernel reads by absolute physical address (the BIOS Data Area's EBDA pointer at
/// `0x40e`, the BIOS area the RSDP is scanned out of). The frame allocator clips it rather than
/// managing it, so it is in no RAM region and would otherwise be unmapped.
const LOW_MEGABYTE: u64 = 0x10_0000;

/// **One physical range the direct map covers, and what to call it when it goes wrong.**
///
/// Every direct-map range [`map_everything`] builds comes out of [`direct_map_claims`] as one of
/// these, and the failure path walks the *same* enumeration to say what a refused mapping collided
/// with. That is the point of the type: a diagnosis derived from a second, hand-written list of
/// "what we probably map" can drift from what is mapped, and the first time anyone reads it will be
/// on a bench with no debugger.
///
/// Name: provisional. calef names the types, and this one was minted by a lane.
struct Claim {
    /// What a reader of [`map_everything`] would call this range, short enough to fit a panic line
    /// on a screen console.
    what: &'static str,
    /// The physical range, half-open. Empty (`hi <= lo`) is legal and maps nothing.
    lo: u64,
    hi: u64,
    flags: Flags,
    /// **Map page by page, skipping what is already mapped, instead of refusing.**
    ///
    /// True only for the loader's reserved entries, which may abut or overlap each other and the
    /// RAM regions in the map the firmware wrote; nobody promised that list is disjoint. Everywhere
    /// else a second mapping of a frame is a real defect and [`MapError::AlreadyMapped`] is the
    /// mechanism that says so, so this is deliberately not a convenience anything else may reach
    /// for.
    guarded: bool,
}

/// **How high the cacheable direct map may follow the loader's map**, given every span the map
/// describes (RAM and reserved alike, in whatever order the firmware wrote them).
///
/// The chain, and it is the whole rule: start at [`LOW_MEGABYTE`] and keep extending upward for as
/// long as some entry in the map begins at or below the current top and ends above it. **DRAM and
/// the carve-outs firmware takes out of it are contiguous** (ACPI tables, NVS, the SMM and graphics
/// reservations at the top of low memory), so the chain covers them. **The 32-bit MMIO hole above
/// them is a gap the map does not describe at all**, so the chain stops at its floor and the local
/// APIC, the IO APIC, the ECAM window, the SPI flash and the framebuffer aperture stay out of it.
/// Those are device windows, they must be device-typed rather than cacheable, and
/// [`map_everything`]'s step 5 maps each by name.
///
/// **This replaced a bound of "the top of RAM", which was the same rule for a machine whose RAM
/// ends below the hole and no rule at all for one whose RAM does not.** That is what killed the
/// first boot on real hardware: xenon has 16.7 GiB, so the top of its RAM is `0x42e000000`, every
/// MMIO window on the machine is below that, and the fill claimed the local APIC's page cacheably
/// a few lines before step 5 asked for it device-typed. `AlreadyMapped`, in
/// `mmu.rs:325`, on 2026-09-05. See notes/x86-uefi-boot.md.
///
/// Nothing here is a 4 GiB constant on purpose. The hole is where it is because 32-bit BARs have to
/// be addressable, but its floor moves with how much memory the firmware stole, and asking the map
/// where it stops describing memory needs no such number.
fn firmware_fill_ceiling(mut spans: impl FnMut(&mut dyn FnMut(u64, u64))) -> u64 {
    let mut ceiling = LOW_MEGABYTE;
    loop {
        let mut grown = ceiling;
        // One pass extends through every entry that already chains in map order; the outer loop is
        // what makes the answer independent of that order, since the firmware's map is not sorted
        // (xenon's puts `0xa0000..0x100000` after a 16 GiB region). It runs at most once per entry.
        spans(&mut |start, end| {
            if start <= grown && end > grown {
                grown = end;
            }
        });
        if grown == ceiling {
            return ceiling;
        }
        ceiling = grown;
    }
}

/// **The floor and the ceiling of the 32-bit MMIO hole, from the firmware's memory map alone**
/// (milestone 256).
///
/// The floor is [`firmware_fill_ceiling`]: the point where the map stops describing memory
/// contiguously upward from the low megabyte, which is where DRAM and the carve-outs taken out of
/// it end. The ceiling is the next thing the map does describe above that, because whatever it is
/// (a reserved window, the ECAM aperture, the flash at the top of the address space) it is not
/// space for this kernel to place a BAR in. A map that describes nothing at all above the floor
/// gives 4 GiB, which is where a 32-bit BAR stops being addressable anyway.
///
/// Pure in its argument, like [`firmware_fill_ceiling`] and for the same reason: the machine that
/// makes this interesting has 16 GiB and cannot be booted under QEMU, so the map goes into the
/// tests as a value (see `map_tests`).
///
/// # BUGS
///
/// **A gap in the firmware's map is not a promise that nothing decodes there.** It is only a
/// promise that firmware did not call it RAM, which is the property that matters for not writing
/// device registers over memory the allocator owns, and is strictly weaker than knowing what the
/// host bridge routes. The MMIO hole on a real machine holds things no table this kernel reads
/// will name; [`memory_mapped_io_window`] takes the windows it *does* know about out of the answer, and the
/// ones it does not know about are the residual risk this note exists to name.
fn firmware_mmio_hole(mut spans: impl FnMut(&mut dyn FnMut(u64, u64))) -> (u64, u64) {
    let floor = firmware_fill_ceiling(&mut spans);
    let mut ceiling = FOUR_GIB;
    spans(&mut |start, end| {
        if end > floor && start > floor && start < ceiling {
            ceiling = start;
        }
    });
    (floor, ceiling)
}

/// One past the last address a 32-bit BAR can name.
const FOUR_GIB: u64 = 0x1_0000_0000;

/// **The lowest `size`-aligned span of `size` bytes inside `lo..hi` that overlaps nothing in
/// `avoid`**, or `None` when the hole has no such room.
///
/// Alignment is `size` rather than a page, because that is what a BAR register can express: the
/// writable bits of a BAR encode its size, so its address has to be a multiple of it, and a window
/// that begins at an address the BARs inside it cannot be aligned to wastes its own first bytes.
///
/// `avoid` is enumerated once per candidate rather than sorted, which is quadratic in a list that
/// has never had more than five entries in it (the framebuffer, the ECAM aperture, both APICs and
/// VT-d's register file). The bound on the outer loop is what keeps that honest: each step moves
/// the candidate strictly past the end of something it collided with, so it runs at most once per
/// entry in `avoid` plus one.
fn window_in_hole(
    lo: u64,
    hi: u64,
    size: u64,
    mut avoid: impl FnMut(&mut dyn FnMut(u64, u64)),
) -> Option<u64> {
    let mut candidate = lo.next_multiple_of(size);
    loop {
        if candidate + size > hi {
            return None;
        }
        // The furthest end of anything this candidate runs into. Taking the furthest rather than
        // the first means one pass per collision group instead of one per entry.
        let mut past = candidate;
        avoid(&mut |start, end| {
            if end > start && start < candidate + size && end > candidate && end > past {
                past = end;
            }
        });
        if past == candidate {
            return Some(candidate);
        }
        candidate = past.next_multiple_of(size);
    }
}

/// **Why [`memory_mapped_io_window`] could not answer.** Each arm is a machine this kernel does not know how to
/// place a BAR on, and every one of them is a panic rather than a fallback: the constant this
/// replaced was right on emulated machines and was RAM on the first real one, so falling back to
/// it would be the original bug wearing the clothes of a graceful degradation. Milestone 215 took
/// the same posture for the analogous case (no MCFG means no PCI, and deliberately no legacy
/// fallback).
///
/// Name: provisional. calef names the types, and this one was minted by a lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMappedIoWindowError {
    /// The boot structure could not be re-read, so there is no memory map to find a hole in.
    NoMemoryMap,
    /// **The two sources disagree.** The firmware's map stops describing memory at one address and
    /// the host bridge's `TOLUD` names another. One of them is wrong about this machine and
    /// nothing here can tell which, so neither is used.
    Disagreement { map: u64, tolud: u64 },
    /// The hole is real and has no room left in it for a window of the size asked for, once the
    /// windows this kernel already knows about are taken out of it. On xenon the framebuffer
    /// aperture sits at the very floor of the hole, so this arm is closer than it looks.
    NoRoom { lo: u64, hi: u64, size: u64 },
}

impl core::fmt::Display for MemoryMappedIoWindowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MemoryMappedIoWindowError::NoMemoryMap => {
                write!(f, "no firmware memory map to read a hole from")
            }
            MemoryMappedIoWindowError::Disagreement { map, tolud } => write!(
                f,
                "the firmware map stops describing memory at {map:#x} but the host bridge's \
                 TOLUD says {tolud:#x}"
            ),
            MemoryMappedIoWindowError::NoRoom { lo, hi, size } => write!(
                f,
                "no {size:#x}-byte window free inside the MMIO hole {lo:#x}..{hi:#x}"
            ),
        }
    }
}

/// **Where this machine's PCI BARs may go, asked of the machine rather than assumed** (milestone
/// 256).
///
/// This replaced `PCI_BAR_PHYS`, a hardcoded `0xc000_0000` that was q35's conventional 32-bit PCI
/// hole and was checked once, against QEMU's own `info mtree` at `-m 256M`. On xenon (a Dell
/// `OptiPlex` 7050, 16 GiB) low DRAM runs to `0xc894_0000`, straight through it, and the kernel
/// panicked mapping a device window on top of memory it had already claimed. The panic was the
/// lucky half: a machine whose RAM ended just below the constant would have had thirteen of
/// fifteen functions relocated on top of memory the allocator believes it owns, and the failure
/// would have arrived as corruption somewhere unrelated.
///
/// **Two sources, and they must agree.** The firmware's memory map names the gap
/// ([`firmware_mmio_hole`]), and Intel's host bridge names the same boundary in `TOLUD`
/// (`super::machine::top_of_low_dram`). When both answer and they differ, this returns
/// [`MemoryMappedIoWindowError::Disagreement`] and the boot stops; there is no arm that quietly picks one.
///
/// **A `TOLUD` that is absent is not a `TOLUD` that disagrees**, and the distinction is what lets
/// this run under emulation at all: QEMU's `q35` does not model the register (measured
/// 2026-09-04, zero on both the PVH and the OVMF path), so on that machine the firmware map is the
/// only source and is used alone. That is not a fallback to the constant, which no arm of this
/// function can reach: it is one of the machine's two answers rather than both.
///
/// `ecam` is passed in rather than read from `memory::pci_regions`, because the boot tour calls
/// this in order to *fill* that static and it is empty until this answers.
///
/// **Name ratified 2026-09-05 (calef).** The lane shipped `bar_window`, on the grounds that it was
/// the vocabulary the retired `PCI_BAR_PHYS` used for the thing in its own first line. calef asked
/// what BAR stood for, which settled it: an argument from this project's own history is not an
/// argument that a reader knows the word. **BAR is base address register**, a PCI term, and the
/// window is where the blocks those registers point at appear in the physical address space.
///
/// `memory_mapped_io_window` was chosen over `device_register_window` **for accuracy**: a base
/// address register can map a framebuffer or a memory aperture as well as a register block, so the
/// plainer name would have been slightly narrower than the truth. `mmio_window` was refused under
/// the naming rule calef set the same day: an acronym is spelled out unless its expansion teaches
/// nothing, and a reader who knows the term recognises the expansion instantly, so spelling it out
/// costs the expert nothing. `io` stays, because nobody bounces off it.
///
/// [`firmware_mmio_hole`], [`window_in_hole`] and [`top_of_low_dram`] are still provisional.
pub fn memory_mapped_io_window(ecam: (u64, u64)) -> Result<(u64, u64), MemoryMappedIoWindowError> {
    let Some(info) = super::machine::boot_info(crate::DTB.load(Ordering::Relaxed)) else {
        return Err(MemoryMappedIoWindowError::NoMemoryMap);
    };
    let spans = |span: &mut dyn FnMut(u64, u64)| {
        for i in 0..info.memmap_entries as usize {
            let Some(e) = super::machine::memory_map_entry(&info, i) else {
                break;
            };
            span(
                e.addr & !(PAGE_SIZE - 1),
                e.end().next_multiple_of(PAGE_SIZE),
            );
        }
    };
    let (lo, hi) = firmware_mmio_hole(spans);
    if let Some(tolud) = super::machine::top_of_low_dram()
        && tolud != lo
    {
        return Err(MemoryMappedIoWindowError::Disagreement { map: lo, tolud });
    }

    // Everything this kernel already knows decodes inside the hole. The APICs and VT-d's register
    // file are usually above it (on xenon all three are, and the hole ends at 0xf0000000), so most
    // of this list costs nothing on most machines; the framebuffer is the one that earns it, and it
    // earns it on the only real machine this port has met. xenon's aperture is at 0xd0000000, which
    // is the floor of its hole *exactly*, so a window that took the floor and did not ask would
    // have collided with the screen the panic would have been printed on.
    let window = window_in_hole(lo, hi, PCI_BAR_MAPPED, |avoid| {
        avoid(ecam.0, ecam.0 + ecam.1);
        if let Some(apic) = super::irq::local_apic_phys() {
            avoid(apic, apic + PAGE_SIZE);
        }
        if let Some(io_apic) = super::irq::io_apic_phys() {
            avoid(io_apic, io_apic + PAGE_SIZE);
        }
        for (base, size) in memory::vtd_regions().into_iter().flatten() {
            avoid(base, base + size);
        }
        for (base, size) in memory::amd_vi_regions().into_iter().flatten() {
            avoid(base, base + size);
        }
        if let Some((base, size)) = memory::framebuffer() {
            avoid(base, base + size);
        }
    });
    match window {
        Some(base) => Ok((base, PCI_BAR_MAPPED)),
        None => Err(MemoryMappedIoWindowError::NoRoom {
            lo,
            hi,
            size: PCI_BAR_MAPPED,
        }),
    }
}

/// **Every physical range the direct map covers, in the order [`map_everything`] builds them.**
///
/// Split out from the mapping loop so that the failure path can ask the same question the mapping
/// asked. See [`Claim`].
fn direct_map_claims(each: &mut dyn FnMut(Claim)) {
    // 1. **Every RAM region**, not just the low gigabyte the old single-base arithmetic could
    //    reach.
    //
    //    The kernel image's own frames are skipped: on this architecture the image lives at a
    //    *different* base, so a direct-map entry for those frames would not collide with the
    //    section mappings below (as it would on the other two ISAs), it would be a second,
    //    WRITABLE alias of `.text`. W^X that a second mapping undoes is not W^X. Nothing needs
    //    those frames by physical address anyway: the image is on the frame allocator's forbidden
    //    list, so they are never handed out.
    let image_lo = memory::image_start();
    let image_hi = memory::image_end();
    for (start, size) in memory::ram_regions() {
        let end = start + size;
        each(Claim {
            what: "ram",
            lo: start,
            hi: image_lo.min(end),
            flags: Flags::kernel_data(),
            guarded: false,
        });
        each(Claim {
            what: "ram above the image",
            lo: image_hi.max(start),
            hi: end,
            flags: Flags::kernel_data(),
            guarded: false,
        });
    }

    // 1b. **One deliberate hole inside the excluded image range** (milestone 161's SMP item): the
    //     AP trampoline's LMA, the gap link-x86_64.ld opens between `.rodata` and `.data` so the
    //     trampoline's *source* bytes are not `.boot_scratch` or the secondary stacks (both
    //     runtime-mutated; see that file's comment). It is inside `image_lo..image_hi`, so step 1
    //     skips it same as `.text`, and it is *between* two mapped sections rather than inside
    //     either, so neither section's own mapping reaches it either. Nobody's mapping covers it
    //     unless this does. `ap_boot::prepare` is the one reader, through the direct map.
    each(Claim {
        what: "ap trampoline",
        lo: super::ap_boot::trampoline_lma(),
        hi: super::ap_boot::trampoline_lma() + super::ap_boot::trampoline_size(),
        flags: Flags::kernel_data(),
        guarded: false,
    });

    // 5. **The device registers, device-typed (uncacheable), one window each**, and they come
    //    before the firmware reservations rather than after.
    //
    //    **The order is load-bearing and it did not used to be.** The reservation fill is guarded
    //    and these are not, so whichever runs first decides the memory type of any page both want,
    //    and only one of the two answers is correct: a cacheable mapping of a device window is a
    //    write that may sit in a cache line and never reach the device. Naming the windows first
    //    makes device typing win by construction, rather than by [`firmware_fill_ceiling`] being
    //    right about where the hole starts. Both now hold; this is the one that holds without
    //    knowing anything about the machine.
    //
    //    The local APIC's address comes from the machine's own MADT when ACPI answered, and falls
    //    back to the architectural reset default when it did not: `IA32_APIC_BASE` is relocatable,
    //    and mapping the constant while the hardware is somewhere else would be a page of nothing
    //    that reads as all-ones.
    let apic = super::irq::local_apic_phys().unwrap_or(LOCAL_APIC_PHYS);
    each(Claim {
        what: "local apic",
        lo: apic,
        hi: apic + PAGE_SIZE,
        flags: Flags::device(),
        guarded: false,
    });

    // The IO APIC's own page, from the MADT when ACPI answered and from the architectural default
    // when it did not, for the same reason the local APIC's is: the address is the machine's to
    // state, and a machine with two of them puts the second somewhere this constant does not name.
    let io_apic = super::irq::io_apic_phys().unwrap_or(IO_APIC_PHYS);
    each(Claim {
        what: "io apic",
        lo: io_apic,
        hi: io_apic + PAGE_SIZE,
        flags: Flags::device(),
        guarded: false,
    });

    // The PCIe windows: bus 0 of the ECAM config space ACPI's MCFG named, and the BAR window the
    // kernel assigns device registers from (derived from this machine's own memory map and host
    // bridge; see `memory_mapped_io_window`, milestone 256, which has no ACPI or AML source to read instead).
    // Same shape as the other two architectures' `memory::pci_regions()` (a device-tree
    // node there, ACPI's MCFG here, both recorded before this function runs): no window recorded,
    // no mapping, and every probe in `pci.rs` reports nobody home rather than touching MMIO that
    // was never confirmed present. The one-bus cap on the ECAM side is the same as the other two
    // architectures apply and for the same reason: everything QEMU puts on this machine is on bus
    // 0, and all 256 buses would be 64K leaves describing space that reads all-ones.
    if let Some(((ecam, ecam_size), (bar, bar_size))) = memory::pci_regions() {
        each(Claim {
            what: "pci ecam",
            lo: ecam,
            hi: ecam + crate::pci::ecam_bytes().min(ecam_size),
            flags: Flags::device(),
            guarded: false,
        });
        each(Claim {
            what: "pci bar window",
            lo: bar,
            hi: bar + PCI_BAR_MAPPED.min(bar_size),
            flags: Flags::device(),
            guarded: false,
        });
    }

    // VT-d's register files (milestone 161, roadmap item 6), one per unit since milestone 594 (every VT-d unit translates its own devices),
    // device-typed, each as large as its DRHD's `Size` byte says (one page on every unit met so
    // far), at the addresses ACPI's DMAR named (`memory::record_vtd_region`, called from `main.rs`
    // before this function runs). Same shape as the local APIC and IO APIC windows above: no DRHD,
    // no mapping, and `arch::iommu::init` is simply never called. Without this a DRHD's register
    // reads fault the instant the fine map replaces the coarse boot map that covered every
    // physical address indiscriminately; the first version of this driver found that by faulting.
    for (base, size) in memory::vtd_regions().into_iter().flatten() {
        each(Claim {
            what: "vt-d registers",
            lo: base,
            hi: base + size,
            flags: Flags::device(),
            guarded: false,
        });
    }
    // AMD-Vi's register files (lane `amd-vi`), the same claim on a machine whose IVRS named them.
    for (base, size) in memory::amd_vi_regions().into_iter().flatten() {
        each(Claim {
            what: "amd-vi registers",
            lo: base,
            hi: base + size,
            flags: Flags::device(),
            guarded: false,
        });
    }

    // **The screen** (milestone 243), when the boot handoff described one. Device-typed like every
    // other aperture here, and mapped for the same reason the VT-d window is: the kernel console
    // has been writing to it through the coarse boot map since before the tour's first line, and
    // the instant this fine map replaces that one an unmapped framebuffer would fault inside
    // `println!`, whose fault handler prints. There is no diagnostic that survives that.
    //
    // It is the largest window in this function by three orders of magnitude: 1920x1080 is 8 MiB,
    // which is 2026 leaves. That is the price of a screen and it is paid once.
    //
    // **Write-combining, not device-typed** (2026-10-04). Device-typed is strong uncacheable, one
    // bus transaction per four-byte store, and on xenon the boot console's scrolling swept visibly
    // down the screen. `Flags::write_combining()` is still device memory to every rule here (pages
    // rather than blocks, never cached), and selects the PAT entry `arch::x86_64::init` made WC on
    // every core before this map exists. The console only ever writes it (`screen_console`'s
    // `PixelSink` has no read), which is the other half: WC reads are as slow as UC ones.
    //
    // **An aperture inside a RAM region would be a different problem and is not this one.** On
    // xenon the aperture sits in the 32-bit MMIO hole, which the firmware's map does not describe;
    // it is in no RAM region and never was. See this module's BUGS for what a machine that really
    // did report both would need, and why "map it twice" is not among the options on x86.
    if let Some((base, size)) = memory::framebuffer() {
        each(Claim {
            what: "framebuffer",
            lo: base,
            hi: base + size,
            flags: Flags::write_combining(),
            guarded: false,
        });
    }

    // 2. **Memory the loader did not call usable RAM but which is still *memory***: the first
    //    megabyte (IVT, BDA, EBDA, the BIOS area the ACPI RSDP is scanned out of, and the page
    //    below 1 MiB a STARTUP IPI's vector will have to name), and the reserved entries carved out
    //    of DRAM (the ACPI tables, NVS, firmware scratch). Those are read through `phys_to_virt`
    //    and are in no RAM region, so without this they become arithmetic with no mapping the
    //    moment the boot map goes.
    //
    //    Guarded, because a reserved entry can abut or overlap something step 1 already mapped and
    //    nothing promises the firmware's list is disjoint. **`AlreadyMapped` is not treated as
    //    success anywhere**; asking first is what keeps the refusal meaning something everywhere
    //    else.
    firmware_claims(each);
}

/// The loader-reserved half of [`direct_map_claims`]: the low megabyte, every reserved entry below
/// [`firmware_fill_ceiling`], and the initrd.
///
/// Physical page 0 is deliberately left out, so that `phys_to_virt(0)` faults instead of quietly
/// naming the interrupt vector table.
///
/// Claims only the low megabyte when the boot structure cannot be re-read. That is the right
/// failure: this kernel got its RAM regions from the same structure, so a machine that reaches here
/// without one had no ACPI tables to lose access to either.
///
/// Name: provisional (milestone 161, renamed from `map_firmware_regions` when it stopped mapping
/// and started describing).
fn firmware_claims(each: &mut dyn FnMut(Claim)) {
    each(Claim {
        what: "low megabyte",
        lo: PAGE_SIZE,
        hi: LOW_MEGABYTE,
        flags: Flags::kernel_data(),
        guarded: true,
    });

    let Some(info) = super::machine::boot_info(crate::DTB.load(Ordering::Relaxed)) else {
        return;
    };
    let ceiling = firmware_fill_ceiling(|span| {
        for i in 0..info.memmap_entries as usize {
            let Some(e) = super::machine::memory_map_entry(&info, i) else {
                break;
            };
            span(
                e.addr & !(PAGE_SIZE - 1),
                e.end().next_multiple_of(PAGE_SIZE),
            );
        }
    });

    for i in 0..info.memmap_entries as usize {
        let Some(e) = super::machine::memory_map_entry(&info, i) else {
            break;
        };
        if e.is_usable_ram() {
            continue; // step 1 already did these, and did them minus the kernel image
        }
        let start = (e.addr & !(PAGE_SIZE - 1)).max(LOW_MEGABYTE);
        let end = e.end().next_multiple_of(PAGE_SIZE).min(ceiling);
        each(Claim {
            what: "firmware reservation",
            lo: start,
            hi: end,
            flags: Flags::kernel_data(),
            guarded: true,
        });
    }

    // **The initrd, explicitly, regardless of how the memmap classified the bytes it occupies.**
    // Found 2026-08-25 (decisions §86's VT-d/NVMe data point): attaching an NVMe controller grows
    // the ACPI tables enough that the memmap's reserved entry immediately above the top of RAM
    // widens to swallow the initrd's last few hundred bytes (PVH's loader places it at a fixed
    // offset below the top of guest memory, sized for a smaller device set than this boot's
    // tables need). The ceiling above exists to keep this loop out of the MMIO hole, so widening
    // it is not the fix. Claiming the initrd's own recorded bounds directly sidesteps the question
    // of which reserved entry is real backing memory and which is not: `bring_up_memory` already
    // claimed this exact range as `forbidden` before this runs, so nothing else can be relying on
    // it staying unmapped, and a guarded claim is idempotent over anything already covered.
    if let Some((istart, isize)) = memory::initrd_region() {
        each(Claim {
            what: "initrd",
            lo: istart & !(PAGE_SIZE - 1),
            hi: (istart + isize).next_multiple_of(PAGE_SIZE),
            flags: Flags::kernel_data(),
            guarded: true,
        });
    }
}

/// **Which mapping [`map_everything`] was making when the mapper refused, and what else claims the
/// page it refused.**
///
/// [`MapError`] names the symptom. `AlreadyMapped` on its own cost a bench session on 2026-09-05:
/// the machine had no serial cable, a photograph of the screen was the entire transcript, and the
/// panic distinguished none of the eleven kinds of range this function maps. The pair of ranges is
/// what makes such a photograph a diagnosis rather than a hypothesis, and it is worth more than any
/// particular fix, because the next machine nobody can attach a debugger to is the one after this.
///
/// Name: provisional.
struct MapFailure {
    what: &'static str,
    lo: u64,
    hi: u64,
    err: MapError,
    /// The first page of `lo..hi` some *other* claim also covers, and that claim. `None` when
    /// nothing else in the enumeration wants it, which for `AlreadyMapped` is itself a finding:
    /// something outside [`direct_map_claims`] put it there.
    conflict: Option<(u64, &'static str, u64, u64)>,
}

impl core::fmt::Display for MapFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?} mapping {} {:#x}..{:#x}",
            self.err, self.what, self.lo, self.hi
        )?;
        match self.conflict {
            Some((pa, other, olo, ohi)) => write!(
                f,
                ": {:#x} is also claimed by {} {:#x}..{:#x}",
                pa, other, olo, ohi
            ),
            None => write!(
                f,
                ": no other claim covers it, so something outside map_everything did"
            ),
        }
    }
}

/// The first page of `lo..hi` that a claim other than `what` also covers.
///
/// Walks the claims rather than the page tables, and walks them by range rather than page by page:
/// a RAM region on a 17 GiB machine is four million pages and this runs inside a panic path.
fn first_conflict(what: &'static str, lo: u64, hi: u64) -> Option<(u64, &'static str, u64, u64)> {
    let mut best: Option<(u64, &'static str, u64, u64)> = None;
    direct_map_claims(&mut |c| {
        if c.what == what || c.hi <= c.lo {
            return;
        }
        let start = c.lo.max(lo);
        let end = c.hi.min(hi);
        if start >= end {
            return;
        }
        if best.is_none_or(|(pa, ..)| start < pa) {
            best = Some((start, c.what, c.lo, c.hi));
        }
    });
    best
}

/// Build every mapping the kernel needs. Mirrors the aarch64 and RISC-V `map_everything`, and the
/// order is the same: physical memory first, then the image's own sections over the top of it.
fn map_everything<A, P>(m: &mut Mapper<A, P, Ia32e>) -> Result<(), MapFailure>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    // Steps 1, 1b, 5 and 2, in the order `direct_map_claims` yields them. The closure stops doing
    // work after the first failure rather than returning early, because the enumeration is a
    // callback and there is nothing to return through; the cost is a few dozen no-op calls on the
    // one path that is about to panic.
    let mut failed: Option<MapFailure> = None;
    direct_map_claims(&mut |c| {
        if failed.is_some() {
            return;
        }
        // Blocks for memory, pages for devices: see this module's BUGS on the MTRRs.
        let result = if c.guarded {
            guarded_direct_map(m, c.lo, c.hi, c.flags)
        } else if c.flags.is_device() {
            direct_map(m, c.lo, c.hi, c.flags, PageSize::Size4KiB)
        } else {
            direct_map(m, c.lo, c.hi, c.flags, largest_leaf())
        };
        if let Err(err) = result {
            failed = Some(MapFailure {
                what: c.what,
                lo: c.lo,
                hi: c.hi,
                err,
                conflict: if err == MapError::AlreadyMapped {
                    first_conflict(c.what, c.lo, c.hi)
                } else {
                    None
                },
            });
        }
    });
    if let Some(f) = failed {
        return Err(f);
    }

    // 3. The kernel image, section by section, at its linked VAs. W^X: text executable and
    //    read-only, rodata neither writable nor executable, everything else writable and never
    //    executable. Until this map is installed the boot table had all of it present, writable AND
    //    executable, in both halves.
    let mut va = |what: &'static str, lo: u64, hi: u64, flags: Flags| -> Result<(), MapFailure> {
        map_range(m, lo, hi, flags).map_err(|err| MapFailure {
            what,
            lo,
            hi,
            err,
            conflict: None, // these are virtual ranges; the claim enumeration is physical
        })
    };
    va(".text", text_start(), text_end(), Flags::kernel_code())?;
    va(
        ".rodata",
        rodata_start(),
        rodata_end(),
        Flags::kernel_rodata(),
    )?;
    va(".data/.bss", data_start(), bss_end(), Flags::kernel_data())?;

    // 4. The stacks. The guard page below each is deliberately NOT mapped, which is the entire
    //    stack-overflow mechanism and is asserted in `verify`: a mapped guard page is protection
    //    that is silently off.
    va(
        "boot stack",
        stack_bottom(),
        stack_top(),
        Flags::kernel_data(),
    )?;
    for id in 0..crate::cpu::MAX_CPUS {
        let (bottom, top) = crate::smp::secondary_stack_span(id);
        va("secondary stack", bottom, top, Flags::kernel_data())?;
    }
    for id in 0..crate::cpu::MAX_CPUS {
        let (bottom, top) = crate::interrupt_stack::span(id);
        va("interrupt stack", bottom, top, Flags::kernel_data())?;
    }

    Ok(())
}

/// Map a range of *physical* addresses into the direct map, **skipping pages already mapped**.
///
/// The one concession to a list nobody promised is disjoint, and it is deliberately not reachable
/// from the ordinary path: see [`Claim::guarded`].
fn guarded_direct_map<A, P>(
    m: &mut Mapper<A, P, Ia32e>,
    pa_start: u64,
    pa_end: u64,
    flags: Flags,
) -> Result<(), MapError>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    for pa in (pa_start..pa_end).step_by(PAGE_SIZE as usize) {
        if m.translate(phys_to_virt(pa)).is_none() {
            m.map(phys_to_virt(pa), pa, flags)?;
        }
    }
    Ok(())
}

/// Map a range of *virtual* addresses to the physical ones they were linked against.
fn map_range<A, P>(
    m: &mut Mapper<A, P, Ia32e>,
    va_start: u64,
    va_end: u64,
    flags: Flags,
) -> Result<(), MapError>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    if va_end <= va_start {
        return Ok(());
    }
    let pages = (va_end - va_start).div_ceil(PAGE_SIZE);
    m.map_range(va_start, virt_to_phys(va_start), pages, flags)
}

/// Map a range of *physical* addresses into the direct map at [`phys_to_virt`], in leaves no
/// larger than `largest`. `paging::Mapper::map_span` puts a block only where it lies wholly inside
/// the range, so this maps exactly the pages it always did, in fewer entries.
fn direct_map<A, P>(
    m: &mut Mapper<A, P, Ia32e>,
    pa_start: u64,
    pa_end: u64,
    flags: Flags,
    largest: PageSize,
) -> Result<(), MapError>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    if pa_end <= pa_start {
        return Ok(());
    }
    let len = (pa_end - pa_start).next_multiple_of(PAGE_SIZE);
    m.map_span(phys_to_virt(pa_start), pa_start, len, flags, largest)
}

/// Walk the tables in software and check the things that would kill us, **before** the hardware bets
/// the machine on them. The x86 counterpart of the other two architectures' `verify`, and it earns
/// its keep here more than anywhere: a mistake in these tables is not a fault report, it is a page
/// fault taken with a page-table root that cannot describe the fault handler, which escalates to a
/// double fault, then to a triple fault, which QEMU shows as a silent machine reset.
fn verify<A, P>(m: &Mapper<A, P, Ia32e>)
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    // The code we are executing right now. If this is wrong, the instruction after `mov cr3` never
    // gets fetched.
    let here = init as *const () as u64;
    let (pa, flags) = m
        .translate(here)
        .expect("the code switching tables is not mapped: we would die on the next fetch");
    assert_eq!(pa, virt_to_phys(here), "our .text maps to the wrong frame");
    assert!(
        flags.is_kernel_executable(),
        "our own .text is not executable"
    );
    assert!(
        !flags.is_writable(),
        "our own .text is writable (W^X violated)"
    );

    // This CPU's stack, which the `mov cr3` does not push to but the return from `install` does.
    let sp = super::current_sp();
    assert!(
        m.translate(sp).is_some(),
        "the current stack is not mapped: the return from `install` would fault"
    );

    // The direct map, asked about a frame the allocator actually owns rather than about the
    // arithmetic. Everything already in flight (the frame bitmap, the ACPI tables, every page table
    // this walk just wrote) is reached this way, so a direct map that came out short is the failure
    // that would look like memory corruption rather than like a page fault.
    let bitmap = memory::bitmap_region().0;
    assert!(
        m.translate(phys_to_virt(bitmap)).is_some(),
        "the frame allocator's own bitmap is not in the direct map"
    );

    // And the identity map is gone, which is half of what this step is for: an identity map is a
    // complete alias of physical memory sitting in the half ring 3 will get. Asked of the PML4
    // entry rather than through `translate`, which would answer `None` for any low address simply
    // because this mapper serves the high half, and so would pass whatever the tables said.
    //
    // SAFETY: `m.root()` is the table this function has been walking, reachable through the direct
    // map like every other table frame.
    let low_half_root = unsafe { (*phys_to_ptr(m.root())).entries[0] };
    assert_eq!(
        low_half_root, 0,
        "the low half of the fine tables is not empty: an identity map survived"
    );

    // The guard page must NOT be mapped, or stack-overflow protection is silently off and the next
    // time anyone finds out is during an overflow.
    assert!(
        m.translate(stack_guard()).is_none(),
        "the guard page IS mapped: stack overflow protection is off"
    );
    for id in 0..crate::cpu::MAX_CPUS {
        assert!(
            m.translate(crate::smp::secondary_stack_guard(id)).is_none(),
            "a secondary's guard page IS mapped: its stack overflow protection is off"
        );
        assert!(
            m.translate(crate::interrupt_stack::guard(id)).is_none(),
            "an interrupt stack's guard page IS mapped: its overflow protection is off"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The image bounds, read back out of the linker script. These are real: the addresses exist the
// moment the image is linked, and `stack.rs` (the overflow canary and the backtrace's is-this-code
// test) needs them before anything else here works.
// ---------------------------------------------------------------------------------------------

macro_rules! linker_symbol {
    ($(#[$doc:meta])* $name:ident, $sym:ident) => {
        $(#[$doc])*
        pub fn $name() -> u64 {
            unsafe extern "C" {
                static $sym: core::ffi::c_void;
            }
            (&raw const $sym) as u64
        }
    };
}

linker_symbol!(
    /// The boot stack's guard page: one page of address space beneath the stack, and a **hole** in
    /// the map [`init`] builds. `verify` asserts it is one; before `init` the boot map covers it
    /// like everything else, which is worth knowing while reading a stack-overflow report from the
    /// first few steps of the boot tour.
    stack_guard,
    __stack_guard
);
linker_symbol!(
    /// The low end of the boot stack (it grows down, so this is the limit, not the start).
    stack_bottom,
    __stack_bottom
);
linker_symbol!(
    /// One past the high end of the boot stack: where `rsp` started.
    stack_top,
    __stack_top
);
linker_symbol!(
    /// The first byte of the kernel's `.text`.
    text_start,
    __text_start
);
linker_symbol!(
    /// One past the last byte of the kernel's `.text`.
    text_end,
    __text_end
);
linker_symbol!(
    /// The first byte of the kernel's `.rodata`: readable, and executable by nobody.
    rodata_start,
    __rodata_start
);
linker_symbol!(
    /// One past the last byte of `.rodata`.
    rodata_end,
    __rodata_end
);
linker_symbol!(
    /// The first byte of `.data`. `.data`, `.bss` and the boot stack are contiguous in
    /// link-x86_64.ld, so one range from here to [`bss_end`] covers the writable image.
    data_start,
    __data_start
);
linker_symbol!(
    /// One past the last byte of `.bss`.
    bss_end,
    __bss_end
);

/// A page table at physical address `pa`, reached through the direct map.
pub fn phys_to_ptr(pa: u64) -> *mut paging::PageTable {
    phys_to_virt(pa) as *mut paging::PageTable
}

// ---------------------------------------------------------------------------------------------
// The user address spaces (milestone 161, roadmap item 3).
//
// The x86 shape is RISC-V's rather than aarch64's, which is what every function below is arranged
// around: `CR3` names the WHOLE address space the way `satp` does, so a process's own root must
// carry the kernel's high-half entries ([`share_kernel_half`]) or the `mov cr3` unmaps the kernel
// that is executing it. aarch64 needs no such copy, because TTBR1 is a second root every process
// shares implicitly.
//
// Where it is neither: the ASID equivalent. PCID lives in `CR3[11:0]` and is honoured **only** when
// `CR4.PCIDE` is set. It is not set here (see this module's BUGS and roadmap item 3), so a tag
// cannot be written into `CR3` at all -- bits 11:5 and 2:0 are reserved-zero with PCIDE clear, and
// a nonzero one faults -- and every `mov cr3` discards the entire non-global TLB. [`ttbr0_value`]
// and [`flush_asid`] are where that shows, and each says so in its own words.
// ---------------------------------------------------------------------------------------------

/// The raw `CR3`, flag and PCID bits included. [`current_root`] is this with the address masked
/// out; the two differ only once `CR4.PCIDE` or the PWT/PCD bits are used, and neither is today.
/// Kept apart so [`switch_user_root`]'s early return compares what the hardware actually holds.
fn read_cr3() -> u64 {
    super::instructions::read_cr3()
}

/// The physical root of the currently installed user address space.
///
/// On this architecture there is only one root, so this is the same register [`current_root`]
/// reads. A thread with no user space of its own is running on [`reserved_root`], whose low half is
/// empty, so "the user root" is always a real answer rather than a `None` in disguise.
pub fn current_user_root() -> u64 {
    current_root()
}

/// The root a thread with no user address space runs on.
///
/// **The kernel's own root**, exactly as on RISC-V and for the same reason: one register names the
/// whole space, so there is no separate empty user table to point at. Its low half is zero
/// (`init`'s `verify` asserts it), so every user address faults, which is what a kernel thread
/// should get.
#[allow(dead_code)]
pub fn reserved_root() -> u64 {
    ttbr0_value(KERNEL_ROOT.load(Ordering::Relaxed), 0)
}

/// Compose the value `CR3` should hold for a root and an address-space id. Named for aarch64's
/// register because that is the arch contract's word.
///
/// **The `asid` is deliberately dropped, and that is the honest encoding rather than a stub.** The
/// PCID field is `CR3[11:0]`, and with `CR4.PCIDE` clear those bits are not a tag: bits 2:0 and
/// 11:5 are reserved and must be written as zero, so `root | asid` would `#GP` on the very first
/// switch rather than tagging anything. There is nothing to write the number into until PCIDE is
/// on, and turning it on is roadmap item 3's own deferred decision (see this module's BUGS).
///
/// What that costs is stated where it is paid: [`flush_asid`] cannot flush one address space,
/// because the hardware holds no tag to select on.
#[allow(dead_code)]
pub fn ttbr0_value(root: u64, asid: u16) -> u64 {
    debug_assert_eq!(
        root & 0xfff,
        0,
        "a page-table root is page-aligned; CR3's low twelve bits are not part of it"
    );
    // The tag has nowhere to go while PCIDE is off; see this function's own doc comment. When it is
    // turned on, this becomes `root | (asid as u64 & 0xfff)` and `crates/address_space_identifier`'s reuse contract
    // acquires a meaning here that it does not have today.
    let _ = asid;
    root
}

/// Populate a fresh process root's **high half** with the kernel's, so one `CR3` pointing at it
/// sees both the process's user pages (low half) and the whole kernel (high half).
///
/// This is the single-root requirement RISC-V has and aarch64 does not, and x86 is on RISC-V's side
/// of it. The kernel half is PML4 entries 256..512, which is the `SPLIT_SHIFT = 47` line
/// `Ia32e::is_in_half` tests: **both** kernel bases live up there ([`KERNEL_VA_BASE`] is PML4[511]
/// and [`DIRECT_MAP_BASE`] is PML4[273]), so one `copy_from_slice` shares the image, the direct map
/// and every device window at once. The entries point at kernel intermediate tables, so this shares
/// the map itself rather than a snapshot of it: a page the kernel maps afterwards is visible in
/// every process, which is what a shared high half has to mean.
///
/// Called by `user::AddressSpace::new` right after it allocates a root.
#[allow(dead_code)]
pub fn share_kernel_half(root: u64) {
    let kernel_root = KERNEL_ROOT.load(Ordering::Relaxed);
    assert_ne!(
        kernel_root, 0,
        "share_kernel_half before mmu::init: there is no kernel map to share yet"
    );
    // SAFETY: both are page-aligned root tables reachable through the direct map, which `boot.s`
    // installed before any Rust ran. Only the high-half entries are written; the low half stays
    // zero for the process's own pages.
    unsafe {
        let dst = &mut (*phys_to_ptr(root)).entries;
        let src = &(*phys_to_ptr(kernel_root)).entries;
        dst[paging::ENTRIES / 2..].copy_from_slice(&src[paging::ENTRIES / 2..]);
    }
}

/// Install the address space named by `cr3`, **unless it is already installed**.
///
/// The early return is worth more here than on either of the other two architectures. `CR4.PGE` is
/// off, so a `mov cr3` throws away *every* translation this CPU holds, the kernel's included; a
/// switch between two threads of the same space, or between two kernel threads (both naming
/// [`reserved_root`]), would otherwise pay for a full TLB refill to change nothing. The comparison
/// reads the register back, so it is against what the hardware is walking rather than against our
/// record of it.
///
/// # Safety
/// `cr3` must be a value [`ttbr0_value`] composed over a **live** root that carries the kernel's
/// high half, or [`reserved_root`]. Anything else unmaps the instruction after this one, which on
/// this architecture is a page fault taken with tables that cannot describe the fault handler: a
/// double fault, then a triple fault, then a silent machine reset.
#[allow(dead_code)]
pub unsafe fn switch_user_root(cr3: u64) {
    if read_cr3() == cr3 {
        return;
    }
    // SAFETY: this function's own `# Safety` contract is exactly the one `install` needs; it
    // forwards, it does not weaken.
    unsafe { install(cr3) };
}

/// Read the address-space id back out of a composed [`ttbr0_value`]. The inverse of that function,
/// and it exists so a portable test can ask "which tag is this space wearing?" without knowing where
/// this ISA keeps it.
///
/// **It answers 0 for every space, and that is the truth rather than a stub.** `CR4.PCIDE` is off
/// (see [`ttbr0_value`]), so the hardware holds no tag at all: there is nothing in the register for
/// this to decode. A test that asserts two live address spaces wear *different* tags is therefore
/// asserting something this architecture does not do yet, and should say so with `skip!()` rather
/// than read a zero as agreement. The aarch64 twin shifts bits 63:48 out and the RISC-V one
/// `satp[59:44]`.
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))]
pub fn asid_of(cr3: u64) -> u16 {
    let _ = cr3;
    0
}

/// **Test-only: let ring 0 load and store through pages marked user-accessible**, returning whether
/// it was already permitted so the caller can put it back.
///
/// A no-op here, as on aarch64, and for a closely related reason: the feature that would forbid it
/// is **SMAP** (`CR4.SMAP`, plus `RFLAGS.AC` to punch through it per access), which this kernel does
/// not enable, exactly as it never sets aarch64's `PSTATE.PAN`. RISC-V is the odd one out, forbidding
/// it unless `sstatus.SUM` is set, which is why this function exists at all: without it a test that
/// reads through a user VA to see what the TLB holds compiles on all three ISAs and faults on one.
///
/// Returns the previous state (always `true`) so the three arch modules have one signature.
///
/// # BUGS
/// **Turning SMAP on would make this a real function and is worth doing**, for the reason RISC-V
/// leaves `SUM` clear in a shipping build: a kernel bug that strays into the low half should fault
/// rather than succeed quietly. It is not on because nothing has measured what it costs on the
/// syscall path, and a protection turned on without a number is the shape of change this tree asks
/// for evidence about.
#[cfg(any(test, feature = "system_tests"))]
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))] // the system tests call it; a unit-test boot on some ISAs does not
pub fn permit_kernel_access_to_user_pages(_allowed: bool) -> bool {
    true
}

/// Install a user address space unconditionally, by writing `CR3`.
///
/// The running kernel does not call this: it installs address spaces through the context switch,
/// which goes via [`switch_user_root`] so it can skip a `mov cr3` that would change nothing. This is
/// the unconditional install, which is what a test wants when it is asserting about a *specific*
/// address space rather than about scheduling.
///
/// # Safety
/// `cr3` must be a value [`ttbr0_value`] composed over a live root that carries the kernel's high
/// half. Anything else unmaps the instruction after this one; see [`switch_user_root`] for what that
/// costs on this architecture.
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))]
pub unsafe fn activate_user(cr3: u64) {
    // SAFETY: this function's own `# Safety` contract is exactly the one `install` needs; it
    // forwards, it does not weaken.
    unsafe { install(cr3) };
}

/// Remove the user address space from this CPU: fall back to the kernel-only reserved root.
#[allow(dead_code)]
pub fn deactivate_user() {
    // SAFETY: `reserved_root()` composes `KERNEL_ROOT`, the fine map `init` built, verified and is
    // running on. It carries the kernel half by construction and its low half is empty.
    unsafe { switch_user_root(reserved_root()) };
}

/// Discharge every translation belonging to the address space tagged `asid`.
///
/// **It flushes everything, and saying so is the point.** `crates/address_space_identifier`'s teardown contract is
/// "after this call, and only after it, the number may tag someone else". With `CR4.PCIDE` clear
/// the hardware holds no tag at all, so there is no set of entries this could select: the only
/// implementation that keeps the contract true is to discard the whole non-global TLB, which a
/// `CR3` reload does. Over-flushing is correct and slow; under-flushing would be one process
/// reading another's memory with no fault to announce it, which is the failure `flush_asid` exists
/// to prevent.
///
/// **Every online CPU, not just this one.** x86 has no TLB broadcast (`invlpg` and a `CR3` reload
/// are both local), so the remote half is [`shoot_down_others`]'s NMI protocol, the same problem
/// RISC-V solves with SBI RFENCE. Without it, a core that ran a thread of the dying space keeps its
/// translations and the next space handed this number reads them, which is one process reading
/// another's memory with no fault to announce it.
#[allow(dead_code)]
pub fn flush_asid(asid: u16) {
    let _ = asid; // nothing to select on; see this function's doc comment
    // SAFETY: rewriting CR3 with the value it already holds changes no mapping and invalidates
    // every non-global entry, which is exactly the discharge this owes. TLB maintenance is always
    // sound; getting it wrong means a stale translation, which is the unsafety that matters here.
    unsafe { install(read_cr3()) };
    shoot_down_others(SHOOTDOWN_ALL);
}

/// Map one user page at `va` onto the already-owned physical frame `phys` in the current address
/// space, drawing any intermediate tables from `alloc`.
///
/// The TLB is flushed for `va` afterwards. x86 does not require it for a *newly valid* leaf the way
/// RISC-V does, because the walker is not permitted to cache a not-present entry, but a `va` that
/// was mapped and unmapped inside one address space would otherwise keep the old translation, and
/// distinguishing the two cases here would be a foot gun for one `invlpg`.
#[allow(dead_code)]
pub fn map_current_user_page_frame(
    va: u64,
    phys: u64,
    flags: paging::Flags,
    alloc: impl FnMut() -> Option<u64>,
) -> Result<(), paging::MapError> {
    let root = current_root();
    // SAFETY: `root` is the live installed root; the direct map makes `phys_to_ptr` valid for every
    // table frame it reaches.
    let mut mapper = unsafe { Mapper::<_, _, Ia32e>::new(root, Half::Low, alloc, phys_to_ptr) };
    mapper.map(va, phys, flags)?;
    flush_tlb(va);
    Ok(())
}

/// Unmap one user page at `va` in the space rooted at `root`, invalidate the TLB, and return the
/// frame it named (the caller's to free; the mapper never owned it).
#[allow(dead_code)]
pub fn unmap_user_at(root: u64, va: u64) -> Option<u64> {
    // SAFETY: `root` is a low-half-owning root; `unmap` allocates nothing, so the `|| None`
    // allocator is never called; the direct map makes `phys_to_ptr` valid.
    let mut mapper = unsafe { Mapper::<_, _, Ia32e>::new(root, Half::Low, || None, phys_to_ptr) };
    let (pa, flush) = mapper.unmap(va).ok()?;
    flush.flush(flush_tlb);
    Some(pa)
}

/// **Cut the page table at `table` out of the walk that reaches `va` in the space rooted at
/// `root`**, and flush that space's tag. Returns the span of addresses that went with it, as
/// `(base, size)`, or `None` if the walk to `va` no longer passes through `table`. Name
/// provisional (the page-tables-outlive-destroy lane, 2026-10-05 UTC).
///
/// `revoke::revoke_region` calls this for each table a region paid for before the region's pages
/// go back: see `paging::Mapper::unlink_table` for why the cut is one entry. The flush is by ASID
/// rather than by page because the span is up to a whole table's reach and the walk caches above
/// the leaves have to go too, and a by-tag flush reaches both on every core.
pub fn cut_user_table(root: u64, va: u64, table: u64, asid: u16) -> Option<(u64, u64)> {
    // SAFETY: `root` is a live low-half table (the registry forgets a root before its space frees
    // it); the direct map makes `phys_to_ptr` valid; a cut allocates nothing.
    let mut mapper = unsafe { Mapper::<_, _, Ia32e>::new(root, Half::Low, || None, phys_to_ptr) };
    let (base, span, flush) = mapper.unlink_table(va, table)?;
    flush.flush(|_| flush_asid(asid));
    Some((base, span))
}

/// Translate `va` in the space rooted at physical `root`.
#[allow(dead_code)]
pub fn translate_at(root: u64, va: u64) -> Option<(u64, paging::Flags)> {
    // SAFETY: `root` is a page table; the direct map makes `phys_to_ptr` valid; a translate
    // allocates nothing, so the `|| None` allocator is never called.
    let mapper = unsafe { Mapper::<_, _, Ia32e>::new(root, Half::Low, || None, phys_to_ptr) };
    mapper.translate(va)
}

/// Translate `va` as a user address in the currently installed space.
#[allow(dead_code)]
pub fn translate_user(va: u64) -> Option<(u64, paging::Flags)> {
    translate_at(current_root(), va)
}

// ---------------------------------------------------------------------------------------------
// The device windows. Both `virt` machines describe theirs in a device tree; q35 describes its PCIe
// window in ACPI's MCFG and has no virtio-mmio bus at all.
// ---------------------------------------------------------------------------------------------

/// **There is no virtio-mmio transport on q35.** Both `virt` machines put eight virtio devices on a
/// memory-mapped bus the kernel finds by walking fixed slots; a PC has never had such a thing, and
/// every virtio device on q35 is a PCI function. So this base is zero and [`VIRTIO_SLOTS`] is zero,
/// which is what makes `virtio::find_block_device` find nothing rather than probe an address that
/// answers with bus noise. The PCIe transport (DECISIONS §18) is the one that carries here, which is
/// the arrangement that transport was built for.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_MMIO_BASE: u64 = 0;
/// Bytes between consecutive virtio-mmio transports. Meaningless with no bus; see
/// [`VIRTIO_MMIO_BASE`].
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_SLOT_STRIDE: u64 = 0x1000;
/// **Zero virtio-mmio slots**, which is the fact, not a placeholder. See [`VIRTIO_MMIO_BASE`].
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_SLOTS: u64 = 0;
/// The interrupt the first virtio-mmio slot would raise. Unreachable with no slots.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_IRQ_BASE: u32 = 0;

/// Bytes of PCI BAR space the kernel maps for device registers, matching what the other two
/// architectures reserve. 2 MiB covers every BAR QEMU hands out on this machine, and since
/// milestone 256 it is the size of the window rather than a slice of a larger one: **where** that
/// window goes is [`memory_mapped_io_window`]'s answer and is the machine's, not a constant's.
///
/// It stays small on purpose. It only ever has to hold the BARs this kernel **places**, and a BAR
/// the machine placed itself is adopted where it stands (`pci::place_bars`) rather than moved into
/// here, so on real firmware almost nothing is drawn from it. Every byte of it is mapped with
/// 4 KiB leaves at boot, which is this module's first recorded BUG, so a window sized for the
/// whole MMIO hole would be page tables for hundreds of megabytes nothing decodes.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const PCI_BAR_MAPPED: u64 = 0x20_0000;

/// The first interrupt a PCI function raises.
///
/// **Zero, and nothing on this architecture reads it any more** (milestone 215, `x86_64` PCI
/// interrupt routing). On the other two machines this is a fixed base the device tree states and
/// legacy INTx lines land at base + pin. On q35 a PCI function's legacy interrupt goes through the
/// PIRQ router to an IO APIC input the ACPI `_PRT` names, `_PRT` is AML, and this tree has no
/// interpreter for it; zero was the marker for that rather than a value to trust.
///
/// What replaced it is not a better constant, it is **not a constant at all**: a function's
/// interrupt is bound by `arch::x86_64::irq::alloc_msi_vector`, which reserves a vector and hands
/// `kernel/src/pci.rs` the address and data to write into the function's own MSI-X table. The
/// device is told where to deliver, so there is no board-specific routing to encode here. The
/// constant survives because the arch contract's other two implementations are real and the
/// portable code names it; `kernel/src/pci.rs` reaches it only on a machine whose
/// `alloc_msi_vector` answers `None`, which this one never does.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const PCI_IRQ_BASE: u32 = 0;

#[cfg(test)]
mod map_tests {
    //! **The 17 GiB machine, without the 17 GiB machine.**
    //!
    //! These run in the `x86_64` kernel leg under QEMU, which boots with 2 GiB and can therefore
    //! never produce the memory map that killed the first boot on real hardware. They do not need
    //! it to: [`super::firmware_fill_ceiling`] is a function of the map alone, so the map goes in
    //! as a value. It is transcribed from `art/bench/xenon-2026-09-05-first-light.jpg`, which is
    //! the only transcript that boot left. **It is 33 of the 38 entries the machine reports**, which
    //! is nearly the whole map rather than a small sample: every entry the photograph shows, being
    //! the tail plus the chain above 2 MiB. (This comment said 148 until 2026-09-04, from a
    //! misreading of the photograph that the second boot's clearer shot corrected. The count was
    //! wrong in the direction that undersold the fixture.)

    use super::{LOCAL_APIC_PHYS, firmware_fill_ceiling};

    /// xenon's memory map: `(start, end, usable)`, physical, in the order the firmware wrote it.
    /// **Not sorted**, and that is faithful rather than sloppy: the low `0xa0000..0x100000`
    /// reservation comes after a 16 GiB RAM region on this machine, which is why the ceiling walk
    /// takes more than one pass.
    const XENON: &[(u64, u64, bool)] = &[
        (0x0, 0xa_0000, true),
        (0x10_0000, 0x200_0000, true),
        (0x200_0000, 0x214_c000, false),
        (0x214_c000, 0xacfc_0000, true),
        (0xacfc_0000, 0xad00_0000, false),
        (0xad00_0000, 0xb7e1_b000, true),
        (0xb7e1_b000, 0xb82c_5000, false),
        (0xb82c_5000, 0xb9ef_d000, true),
        (0xb9ef_d000, 0xb9ef_e000, false),
        (0xb9ef_e000, 0xb9ef_f000, false),
        (0xb9ef_f000, 0xb9fb_0000, true),
        (0xb9fb_0000, 0xb9fb_a000, true),
        (0xb9fb_a000, 0xb9fb_b000, true),
        (0xb9fb_b000, 0xb9fb_f000, false),
        (0xb9fb_f000, 0xc894_0000, true),
        (0xc894_0000, 0xc8d3_e000, true),
        (0xc8d3_e000, 0xc97c_4000, true),
        (0xc97c_4000, 0xcadf_b000, false),
        (0xcadf_b000, 0xcae4_1000, false),
        (0xcae4_1000, 0xcb77_5000, false),
        (0xcb77_5000, 0xcbd1_8000, false),
        (0xcbd1_8000, 0xcbdf_f000, false),
        (0xcbdf_f000, 0xcbe0_0000, true),
        // The last entry below the 32-bit MMIO hole: firmware's own reservation at the top of low
        // DRAM. Above it the map describes nothing until 0xf0000000.
        (0xcbe0_0000, 0xd000_0000, false),
        (0x1_0000_0000, 0x1_4000_0000, true),
        (0x1_4000_0000, 0x1_408c_0000, true),
        (0x1_408c_0000, 0x4_2e00_0000, true),
        (0xa_0000, 0x10_0000, false),
        (0xf000_0000, 0xf800_0000, false),
        (0xfe00_0000, 0xfe01_1000, false),
        (0xfec0_0000, 0xfec0_1000, false),
        (0xfee0_0000, 0xfee0_1000, false),
        (0xff00_0000, 0x1_0000_0000, false),
    ];

    fn ceiling(map: &[(u64, u64, bool)]) -> u64 {
        firmware_fill_ceiling(|span| {
            for &(start, end, _) in map {
                span(start, end);
            }
        })
    }

    /// **The panic, reproduced as an assertion.** The old bound was the top of RAM, which on this
    /// machine is `0x42e000000`: every MMIO window it has is below that, so the cacheable
    /// reservation fill claimed the local APIC's page a few lines before the device mapping asked
    /// for it, and the mapper refused. The chain rule stops at the floor of the MMIO hole instead.
    #[test_case]
    fn the_fill_stops_below_the_devices_on_a_17_gib_machine() {
        let top_of_ram = XENON
            .iter()
            .filter(|(.., usable)| *usable)
            .map(|(_, end, _)| *end)
            .max()
            .expect("the fixture has RAM");
        assert_eq!(top_of_ram, 0x4_2e00_0000, "the fixture is not xenon's map");
        assert!(
            top_of_ram > LOCAL_APIC_PHYS,
            "the old bound would not have reached the local APIC, so this is not the failure"
        );

        assert_eq!(
            ceiling(XENON),
            0xd000_0000,
            "the fill should stop where the firmware's map stops describing memory"
        );
        assert!(
            ceiling(XENON) <= LOCAL_APIC_PHYS,
            "the local APIC is inside the cacheable fill again"
        );
    }

    /// **The hole xenon's BARs have to go in**, both ends of it, from the map alone.
    ///
    /// The floor is the same number [`the_fill_stops_below_the_devices_on_a_17_gib_machine`]
    /// asserts and that is the point: one walk of the map answers "how far may the cacheable fill
    /// follow memory" and "where does the MMIO hole start", because they are the same boundary
    /// asked from the two sides. The ceiling is the next thing the firmware describes above it,
    /// which on this machine is the reserved window at `0xf0000000`.
    #[test_case]
    fn the_mmio_hole_on_a_17_gib_machine_is_bounded_at_both_ends() {
        let hole = super::firmware_mmio_hole(|span| {
            for &(start, end, _) in XENON {
                span(start, end);
            }
        });
        assert_eq!(hole, (0xd000_0000, 0xf000_0000), "xenon's 32-bit MMIO hole");
        assert!(
            hole.0 > 0xc000_0000,
            "the retired constant was 0xc0000000, and this machine's hole starts above it, \
             which is the whole reason it is gone"
        );
    }

    /// **The window steps over the screen**, which on this machine is at the floor of the hole.
    ///
    /// xenon's framebuffer is at `0xd0000000`, byte for byte where its MMIO hole begins, so a
    /// derivation that took the floor and asked nothing else would put the first BAR it placed on
    /// top of the console the panic would have been printed on. 1920x1080 at 4 bytes is `0x7e9000`,
    /// and the next 2 MiB boundary above that is `0xd0800000`.
    ///
    /// **This is not only a prediction about a machine nobody here can boot.** The same collision
    /// reproduces under OVMF on QEMU, whose framebuffer also sits at the floor of its hole, and the
    /// boot line there reads `bar window 0x80400000..0x80600000` for the same reason.
    #[test_case]
    fn the_window_steps_over_a_framebuffer_at_the_floor_of_the_hole() {
        const FB: u64 = 0xd000_0000;
        const FB_END: u64 = FB + 1920 * 1080 * 4;
        let at = super::window_in_hole(0xd000_0000, 0xf000_0000, 0x20_0000, |avoid| {
            avoid(FB, FB_END);
        });
        assert_eq!(at, Some(0xd080_0000));
        let at = at.expect("checked above");
        assert!(at >= FB_END, "the window overlaps the screen");
        assert_eq!(
            at % 0x20_0000,
            0,
            "a BAR window must be aligned to its size"
        );
    }

    /// With nothing in the way the window is the floor itself, which is what both QEMU paths do
    /// when no aperture is in the first megabytes of the hole.
    #[test_case]
    fn an_empty_hole_gives_up_its_floor() {
        let at = super::window_in_hole(0x1000_0000, 0xb000_0000, 0x20_0000, |_| {});
        assert_eq!(at, Some(0x1000_0000));
    }

    /// **A hole with no room in it is refused rather than squeezed.** The caller panics on this,
    /// which is the posture the whole milestone turns on: there is no arm that falls back to a
    /// constant, because the constant is what put device registers on top of RAM.
    #[test_case]
    fn a_hole_with_no_room_answers_none() {
        assert_eq!(
            super::window_in_hole(0xd000_0000, 0xd010_0000, 0x20_0000, |_| {}),
            None,
            "a 1 MiB hole cannot hold a 2 MiB window"
        );
        assert_eq!(
            super::window_in_hole(0xd000_0000, 0xd040_0000, 0x20_0000, |avoid| {
                avoid(0xd000_0000, 0xd030_0000);
            }),
            None,
            "the only aligned slot left starts past the end"
        );
    }

    /// The two sources' disagreement is a sentence a person reads off a photograph, so it names
    /// both numbers. Asserted for the same reason [`the_failure_names_both_ranges`] is.
    #[test_case]
    fn the_disagreement_names_both_sources() {
        let mut s = Line::default();
        core::fmt::write(
            &mut s,
            format_args!(
                "{}",
                super::MemoryMappedIoWindowError::Disagreement {
                    map: 0xd000_0000,
                    tolud: 0xc000_0000,
                }
            ),
        )
        .expect("formatting cannot fail");
        let line = s.as_str();
        assert!(line.contains("0xd0000000"), "{line}");
        assert!(line.contains("0xc0000000"), "{line}");
    }

    /// A machine whose RAM ends below the hole, which is every machine this tree had booted before
    /// xenon: the new rule has to answer what the old one did, or it is a different bug.
    #[test_case]
    fn a_small_machine_is_unchanged() {
        let qemu: &[(u64, u64, bool)] = &[
            (0x0, 0x9_fc00, true),
            (0x9_fc00, 0xa_0000, false),
            (0xf_0000, 0x10_0000, false),
            (0x10_0000, 0xffd_d000, true),
            (0xffd_d000, 0x1000_0000, false),
            (0xfffc_0000, 0x1_0000_0000, false),
        ];
        assert_eq!(ceiling(qemu), 0x1000_0000);
    }

    /// The failure message names both ranges, which is the whole point of it. Asserted rather than
    /// eyeballed, because the reader it is written for is holding a camera.
    #[test_case]
    fn the_failure_names_both_ranges() {
        let f = super::MapFailure {
            what: "local apic",
            lo: 0xfee0_0000,
            hi: 0xfee0_1000,
            err: paging::MapError::AlreadyMapped,
            conflict: Some((
                0xfee0_0000,
                "firmware reservation",
                0xfee0_0000,
                0xfee0_1000,
            )),
        };
        let mut s = Line::default();
        core::fmt::write(&mut s, format_args!("{f}")).expect("formatting a failure cannot fail");
        let line = s.as_str();
        assert!(line.contains("local apic 0xfee00000..0xfee01000"), "{line}");
        assert!(
            line.contains("firmware reservation 0xfee00000..0xfee01000"),
            "{line}"
        );
    }

    /// A fixed line of text to format into. The kernel has no allocator, and one panic line is the
    /// only thing anything here needs to build.
    struct Line {
        bytes: [u8; 256],
        len: usize,
    }

    impl Default for Line {
        fn default() -> Self {
            Line {
                bytes: [0; 256],
                len: 0,
            }
        }
    }

    impl Line {
        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.bytes[..self.len]).expect("we only wrote &str into it")
        }
    }

    impl core::fmt::Write for Line {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let end = (self.len + s.len()).min(self.bytes.len());
            let take = end - self.len;
            self.bytes[self.len..end].copy_from_slice(&s.as_bytes()[..take]);
            self.len = end;
            Ok(())
        }
    }
}
