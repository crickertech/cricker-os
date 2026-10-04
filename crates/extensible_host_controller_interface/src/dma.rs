//! **The DMA region's layout**, in pages from its base, which the kernel allocates and confines and
//! the driver lays its structures out in. Both ends read these constants; nothing else joins them.
//!
//! The first [`WORK_PAGES`] are the driver's own and are mapped into it. The controller's
//! scratchpad pages follow and are **not** mapped: the driver writes their physical addresses into
//! [`SCRATCHPAD_ARRAY`] and never touches them, so a process that cannot read the controller's
//! private memory cannot be confused by it. The IOMMU confines the device to the whole region.

/// The device context base address array: one pointer per slot, entry 0 the scratchpad array.
pub const DEVICE_CONTEXT_ARRAY: u64 = 0;
/// The command ring.
pub const COMMAND_RING: u64 = 1;
/// The event ring's one segment.
pub const EVENT_RING: u64 = 2;
/// The event ring segment table (one entry).
pub const EVENT_SEGMENT_TABLE: u64 = 3;
/// The input context, rewritten for each command that takes one.
pub const INPUT_CONTEXT: u64 = 4;
/// The output device context of the one slot this driver drives at a time.
pub const DEVICE_CONTEXT: u64 = 5;
/// Endpoint 0's transfer ring.
pub const CONTROL_RING: u64 = 6;
/// The keyboard's interrupt IN transfer ring.
pub const INTERRUPT_RING: u64 = 7;
/// The control transfers' data buffer: descriptors land here.
pub const CONTROL_BUFFER: u64 = 8;
/// The interrupt transfers' buffers: one [`REPORT_SLOT`]-byte slot per queued report.
pub const REPORT_BUFFERS: u64 = 9;
/// The scratchpad buffer array: one pointer per scratchpad page.
pub const SCRATCHPAD_ARRAY: u64 = 10;
/// How many pages the driver is mapped. Scratchpad page `n` is at page `WORK_PAGES + n`.
pub const WORK_PAGES: u64 = 11;

/// How many TRBs a ring page holds, the last of a producer ring's being its link.
pub const RING_ENTRIES: u16 = 256;
/// The byte stride of one report buffer, a cache line so no two transfers share one.
pub const REPORT_SLOT: u64 = 64;
/// How many interrupt transfers the driver keeps queued, each into its own report slot. More than
/// one, so a key arriving while the driver handles the last does not wait for a requeue.
pub const REPORTS_QUEUED: u16 = 4;

/// The region's length in pages for a controller asking for `scratchpads` pages.
pub const fn region_pages(scratchpads: u16) -> u64 {
    WORK_PAGES + scratchpads as u64
}

/// The most scratchpad pages the scratchpad array's one page can name.
pub const MAX_SCRATCHPADS: u16 = 512;

// The queued reports fit their page, the ring holds every one of them and its link, and the
// scratchpad array's page names every scratchpad this layout accepts. Checked by the compiler, so
// an edit to any one constant that breaks the others fails the build.
const _: () = assert!(REPORTS_QUEUED as u64 * REPORT_SLOT <= crate::PAGE);
const _: () = assert!(REPORTS_QUEUED < RING_ENTRIES - 1);
const _: () = assert!(MAX_SCRATCHPADS as u64 * 8 == crate::PAGE);

#[cfg(test)]
mod tests {
    use super::*;

    /// **The layout both ends rely on**: every work page is used once and lies inside the work
    /// window the driver is mapped, and the scratchpad pages begin exactly where it ends, so the
    /// controller's private memory never overlaps a page the driver writes. A page constant moved
    /// onto another, or past `WORK_PAGES`, would hand the controller a ring the driver also uses
    /// as a buffer, or a scratchpad the driver is mapped.
    #[test]
    fn the_work_pages_are_distinct_and_the_scratchpads_follow_them() {
        let pages = [
            DEVICE_CONTEXT_ARRAY,
            COMMAND_RING,
            EVENT_RING,
            EVENT_SEGMENT_TABLE,
            INPUT_CONTEXT,
            DEVICE_CONTEXT,
            CONTROL_RING,
            INTERRUPT_RING,
            CONTROL_BUFFER,
            REPORT_BUFFERS,
            SCRATCHPAD_ARRAY,
        ];
        let mut seen = [false; WORK_PAGES as usize];
        for p in pages {
            assert!(p < WORK_PAGES, "page {p} is outside the mapped work pages");
            assert!(!seen[p as usize], "page {p} is used twice");
            seen[p as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "a work page is mapped and unused");
        assert_eq!(region_pages(0), WORK_PAGES);
        // The last scratchpad of the largest accepted set is the region's last page.
        assert_eq!(
            region_pages(MAX_SCRATCHPADS) - 1,
            WORK_PAGES + u64::from(MAX_SCRATCHPADS) - 1
        );
    }
}
