//! **The progenitor's stack gauge**: how deep the first process's stack has ever been, said out
//! loud, and a floor that fails `script/swish-check` before the stack runs out.
//!
//! The progenitor runs on [`crate::user::INIT_STACK_PAGES`] pages, and its stack lives for the
//! whole boot because the spawn service runs inside `system_initializer::boot`. Until this existed
//! nothing measured it: #1360 and #1374 each found the limit by overflowing it, and milestone 205
//! (how a foreign program is told what to do), in #1402, found it a third time, in a debug build at
//! `package install uptime`. Every one of those
//! was a data abort at a prompt, with the size of the miss inferred from a fault address.
//!
//! # The instrument
//!
//! The same watermark milestone 84 (stack high-water: measure kernel stack depth) put on kernel
//! stacks: `crate::stack::paint` and `crate::stack::high_water`, and notes/stack-high-water.md for
//! why a paint outlives the frame that destroyed it. [`paint_page`] fills each stack page with the
//! paint word as
//! `boot_progenitor` maps it, before the progenitor has run an instruction, and remembers the
//! page's kernel view. [`report_peak`] scans from the lowest page up for the first word that is no
//! longer paint. The pages are not contiguous in the direct map, so the scan goes a page at a time.
//!
//! The kernel does the reading rather than the progenitor, which is what keeps this out of
//! `crates/system_initializer`'s way: the progenitor needs no code, no shared constant, and no
//! channel to report on, and a progenitor that is about to overflow is the one program that should
//! not be asked to measure itself.
//!
//! # When it speaks
//!
//! From the scheduler's idle loop (and `x86_64`'s yield syscall, `on_yield`), beside the
//! capability-slot gauge of milestone 231 (nothing counts how many capability slots a boot actually
//! uses), and coalesced the same way: it waits until the mark has not moved for [`STABLE_PASSES_NEEDED`] idle passes,
//! and it never prints the same number twice. So a boot prints one line after the progenitor
//! settles, and another each time a later command drives it deeper. That second property is what
//! makes the transcript a per-path measurement: the line after `package install uptime` is that
//! path's peak.
//!
//! # BUGS
//!
//! - **`x86_64` speaks from the yield syscall instead**, `on_yield`, because its idle loop stops at
//!   the hand-over. The capability-slot gauge has the same gap and does not have this fix; it
//!   could share the trigger, which would make that gauge's `x86_64` line a peak rather than the
//!   mark at the hand-over, and it is left for that gauge's owner rather than changed here.
//! - **A watermark sees exercised paths only.** A spawn of a program `swish-check` never types
//!   goes as deep as it goes, unmeasured. Milestone 84's BUGS has the general form.
//! - **Name provisional**: the module and its two functions (this lane, not ratified).

use core::sync::atomic::{AtomicU64, Ordering};

use crate::user::INIT_STACK_PAGES;

const PAGE: u64 = page_frames::FRAME_SIZE;

/// The whole stack, in bytes: what the gauge's numbers are out of.
pub const STACK_BYTES: u64 = INIT_STACK_PAGES * PAGE;

/// **The least headroom a boot may leave**, in bytes. Below this, the gauge says `BELOW` and
/// `script/swish-check` fails.
///
/// Two pages, and each half is a measured fact rather than a feeling. The install path's two
/// biggest frames are each mostly one page buffer (`system_initializer::edit`'s `new` and
/// `activate`'s `old`, `[u8; PAGE_BYTES]` apiece), and one more such buffer is the step size this
/// stack has actually grown by. The second page covers what the gauge cannot see: paths
/// `swish-check` never types, and indirect calls, which put the measured peak about a kilobyte
/// above the deepest chain a static walk of direct calls finds (32,440 against 31,424, aarch64
/// debug). notes/stack/progenitor-stack.md has both measurements.
pub const HEADROOM_FLOOR: u64 = 2 * PAGE;

/// The kernel view (direct-map address) of each stack page, by `k`, the page's distance below
/// the top: `k = 0` is the page at `USER_STACK_VA`, and the highest `k` is the stack's bottom.
/// Zero until [`paint_page`] records it.
static PAGES: [AtomicU64; INIT_STACK_PAGES as usize] =
    [const { AtomicU64::new(0) }; INIT_STACK_PAGES as usize];

/// Paint one of the progenitor's fresh stack pages and remember where it is.
///
/// Called by `boot_progenitor` for each page `map_new` hands back, before the progenitor exists,
/// so nothing is using the page. `k` is the page's distance below the top, as the map loop counts
/// it.
pub fn paint_page(k: u64, page: &mut [u8]) {
    let bottom = page.as_mut_ptr() as u64;
    // SAFETY: `page` is a whole, page-aligned, writable frame the caller just mapped and nothing
    // has run on yet, which is `paint`'s contract. The `&mut` is what proves nobody else holds it.
    unsafe { crate::stack::paint(bottom, bottom + PAGE) };
    PAGES[k as usize].store(bottom, Ordering::Release);
}

/// **The most bytes the progenitor's stack has ever used**, or `None` before it has been painted.
///
/// Scans the lowest page first and stops at the first page with anything used, so its cost is the
/// headroom rather than the stack: a few hundred words on a healthy boot.
pub fn high_water() -> Option<u64> {
    for k in (0..INIT_STACK_PAGES).rev() {
        let bottom = PAGES[k as usize].load(Ordering::Acquire);
        if bottom == 0 {
            return None;
        }
        // SAFETY: `bottom` is a page `paint_page` painted, recorded after the paint. The progenitor
        // never exits, so its address space, and this frame, live for the whole boot; a racing
        // write from the progenitor on another core can only make the answer deeper.
        let used = unsafe { crate::stack::high_water(bottom, bottom + PAGE) };
        if used != 0 {
            return Some(k * PAGE + used);
        }
    }
    Some(0)
}

static LAST_PASS: AtomicU64 = AtomicU64::new(0);
static STABLE_PASSES: AtomicU64 = AtomicU64::new(0);
static REPORTED: AtomicU64 = AtomicU64::new(0);

/// How still the mark has to be before [`report_peak`] believes a climb is over. The
/// capability-slot gauge's window (`kernel::cap`'s `PEAK_STABLE_PASSES_NEEDED`), for its reason:
/// the progenitor blocks on IPC several times mid-build, and each pause is an idle pass.
const STABLE_PASSES_NEEDED: u64 = 16;

/// Say the progenitor's stack high-water mark once it has stopped moving. Called from the
/// scheduler's idle loop; see this module's doc.
pub fn report_peak() {
    let Some(used) = high_water() else {
        return;
    };
    if used == 0 {
        return;
    }
    if LAST_PASS.swap(used, Ordering::Relaxed) != used {
        STABLE_PASSES.store(0, Ordering::Relaxed);
        return;
    }
    if STABLE_PASSES.fetch_add(1, Ordering::Relaxed) < STABLE_PASSES_NEEDED {
        return;
    }
    if REPORTED.fetch_max(used, Ordering::Relaxed) >= used {
        return;
    }
    announce(used);
}

/// **`x86_64`'s trigger**, called from the yield syscall there, because that leg's idle loop stops
/// running at the hand-over: the input driver polls COM1 and yields (milestone 299 (the x86 port-range capability: the
/// serial console becomes a userspace driver)), so the run
/// queue is never empty again. Every yield is a thread saying it has nothing to do, which is the
/// idle loop's reason to report, from userspace.
///
/// One yield in [`YIELDS_PER_LOOK`] looks, because the driver yields in a tight loop and a look
/// scans the headroom (about two thousand words today). Throttling cannot make the peak wrong,
/// since the paint keeps it; it only makes the line later.
///
/// Only on `x86_64`: on aarch64 and riscv64 the idle loop already runs between commands, and a
/// second trigger there would print more lines on a multi-core machine, where a kernel line and a
/// userspace one can shuffle byte by byte (`script/swish-check`'s `boot_claim` has that story).
#[cfg(target_arch = "x86_64")]
pub fn on_yield() {
    static YIELDS: AtomicU64 = AtomicU64::new(0);
    if YIELDS
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(YIELDS_PER_LOOK)
    {
        report_peak();
    }
}

/// How many yields `on_yield` lets pass between looks. A power of two picked to make a look
/// rare against a polling loop; nothing depends on its exact value.
#[cfg(target_arch = "x86_64")]
const YIELDS_PER_LOOK: u64 = 256;

/// The sentence `script/swish-check` reads. Its prefix is `xtask`'s `PROGENITOR_STACK_GAUGE`.
fn announce(used: u64) {
    let spare = STACK_BYTES.saturating_sub(used);
    if spare < HEADROOM_FLOOR {
        crate::println!(
            "  progenitor stack: {used} of {STACK_BYTES} bytes at peak, {spare} spare, BELOW the \
             {HEADROOM_FLOOR}-byte floor in kernel/src/progenitor_stack.rs"
        );
        return;
    }
    crate::println!("  progenitor stack: {used} of {STACK_BYTES} bytes at peak, {spare} spare");
}
