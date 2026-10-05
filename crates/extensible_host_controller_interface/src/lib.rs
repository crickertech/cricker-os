#![no_std]
//! **xHCI, as pure logic** (milestone 242, USB host and HID; notes/usb.md).
//!
//! Everything a USB host controller driver computes, with nothing it touches. The register file
//! and the DMA memory are the kernel's to find and confine (`kernel/src/extensible_host_controller_interface.rs`)
//! and the EL0 driver's to drive (`components/src/usb_keyboard_driver.rs`); what lives here is the
//! arithmetic those volatile accesses carry. It is the NVMe split again, one device over:
//! `crates/non_volatile_memory_express` is the precedent and rule 7 is the reason.
//!
//! The protocol in one paragraph (xHCI 1.2, chapters 4 and 6). **This is the first controller in
//! the tree whose data structures the device walks on its own.** The driver lays out, in memory
//! the controller reads by DMA, a *device context base address array* (one pointer per device
//! slot), a *command ring*, an *event ring* with its segment table, and one *transfer ring* per
//! endpoint. Every ring is an array of 16-byte **TRBs** (transfer request blocks) with a **cycle
//! bit** in each: the producer writes TRBs carrying its current cycle, the consumer accepts a TRB
//! only while the bit matches its own, and both flip at the end of the ring, where a producer ring
//! carries a **link TRB** back to its start. The driver rings a **doorbell** to say a ring has new
//! work; the controller writes **events** (command completions, transfer completions, port
//! changes) to the event ring and raises an interrupt.
//!
//! What is here:
//!
//! - [`regs`]: register offsets and bit names, and [`Capabilities`], the read-only facts the
//!   capability registers state.
//! - [`register_window`]: **which pages of the register file a confined driver is mapped**, the
//!   confinement decision this crate exists to make checkable.
//! - [`Handoff`]: the three spawn words, packed and unpacked.
//! - [`dma`]: the DMA region's layout, which both ends agree on.
//! - [`Trb`], [`Event`], [`ProducerRing`] and [`EventRing`]: the ring discipline.
//! - [`context`]: slot and endpoint contexts.
//! - [`port`]: the port status register, whose write-one-to-clear bits are a trap.
//!
//! # Examples
//!
//! The cycle bit is the mechanism worth showing. A producer ring of four entries has three usable
//! slots and a link TRB in the fourth; the third enqueue also writes the link, and the cycle the
//! producer stamps flips for the next lap:
//!
//! ```
//! use extensible_host_controller_interface::{ProducerRing, Trb};
//!
//! let mut ring = ProducerRing::new(0x1000, 4);
//! let a = ring.enqueue(Trb::enable_slot());
//! assert_eq!((a.index, a.trb.cycle(), a.link), (0, true, None));
//! ring.enqueue(Trb::enable_slot());
//! let c = ring.enqueue(Trb::enable_slot());
//! assert_eq!(c.index, 2);
//! let (at, link) = c.link.unwrap(); // the third TRB fills the lap, so the link is written too
//! assert_eq!(at, 3);
//! assert!(link.cycle()); // stamped with the lap it ends, so the controller follows it
//! let d = ring.enqueue(Trb::enable_slot());
//! assert_eq!((d.index, d.trb.cycle()), (0, false)); // second lap, cycle flipped
//! assert_eq!(ring.address_of(d.index), 0x1000);
//! ```
//!
//! And the register window: QEMU's `qemu-xhci` puts its MSI-X table in the fourth page of its
//! 16 KiB register file, so a driver is mapped the first three and never the fourth.
//!
//! ```
//! use extensible_host_controller_interface::{Capabilities, register_window};
//!
//! // qemu-xhci's capability registers: 0x40 bytes of them, 64 slots, 4 ports, runtime at 0x1000,
//! // doorbells at 0x2000.
//! let caps = Capabilities::decode(0x0100_0040, 0x0400_0840, 0, 0x0000_7000, 0x2000, 0x1000);
//! let window = register_window(&caps, 0x4000, &[(0x3000, 0x3000 + 16 * 16), (0x3800, 0x3808)]);
//! assert_eq!(window, Ok(0b0111));
//! ```
//!
//! # BUGS
//!
//! - **One page size.** [`dma`] assumes the controller accepts 4 KiB pages (`PAGESIZE` bit 0),
//!   which every xHCI this project has read about does; the driver checks and refuses otherwise.
//! - **No streams, no isochronous, no bulk.** The TRB builders cover the commands and transfers a
//!   keyboard needs and nothing more.
//! - **Hubs are not modelled.** The slot context built here always has a route string of 0, so a
//!   device behind a hub cannot be addressed. Milestone 242's own BUGS entry; the keyboards on
//!   the bench plug into a root port.
//!
//! Name: provisional. Introduced 2026-10-04 for milestone 242 (USB host and HID). The expansion
//! of xHCI, by DECISIONS §154 (the acronym test is whether the phrase is spoken) as `crates/non_volatile_memory_express` was named;
//! too long for a program (`nifefs`'s 32-byte limit), so the program that drives it is
//! `usb_keyboard_driver`, which says what it is for. An architect's call; expect it to be asked.

pub mod context;
pub mod dma;
pub mod port;
pub mod regs;
pub mod report;

pub use regs::Capabilities;

/// **How many pages of the register file a driver can be mapped**: the bits of [`Handoff`]'s
/// register mask. 64 KiB, which is the BAR of every Intel xHCI this project has read about; a
/// larger BAR is mapped this far and no further, and [`register_window`] refuses a controller
/// whose needed registers lie beyond it.
pub const MAX_REGISTER_PAGES: u64 = 16;

/// The page size every layout here is in.
pub const PAGE: u64 = 4096;

/// **Why a register window could not be drawn.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRefusal {
    /// A register the driver must reach (named by its byte offset) is past
    /// [`MAX_REGISTER_PAGES`] or past the end of the BAR.
    BeyondWindow {
        /// The register's byte offset into the BAR.
        offset: u64,
    },
    /// A register the driver must reach shares a page with the MSI-X table or pending-bit array,
    /// which the driver must never be mapped: whoever can write the table can aim the device's
    /// interrupt message anywhere.
    SharesTheInterruptPage {
        /// The register's byte offset into the BAR.
        offset: u64,
    },
}

/// **Which pages of the register file a confined driver is mapped**, as a mask (bit `n` is the
/// page at byte `n * 4096`), or why none can be.
///
/// Every page of the BAR up to [`MAX_REGISTER_PAGES`] is mapped **except** any page that overlaps
/// a `withheld` byte range. The kernel passes the MSI-X table and pending-bit array there, so the
/// rule a reader can check is one sentence: *the driver holds every register page of its
/// controller except the page its interrupt message is programmed through.* With interrupt
/// remapping off (every x86 boot here, DECISIONS §86 (whether an NVMe driver can leave the kernel)'s interrupt finding), a process that could
/// write an MSI-X entry could make the device deliver any vector it liked, and no IOMMU would stop
/// it, because an interrupt message is not a DMA the IOMMU translates.
///
/// Then the pages the driver **needs** are checked against the mask: the capability and
/// operational registers through the last port's status register, interrupter 0's registers, and
/// the doorbells for every slot. A needed page that is withheld or out of reach is a refusal,
/// because a driver started without one of those would fault on its first access to it.
pub fn register_window(
    caps: &Capabilities,
    bar_bytes: u64,
    withheld: &[(u64, u64)],
) -> Result<u64, WindowRefusal> {
    let pages = (bar_bytes / PAGE).min(MAX_REGISTER_PAGES);
    let mut mask = 0u64;
    for page in 0..pages {
        let (start, end) = (page * PAGE, (page + 1) * PAGE);
        if !withheld.iter().any(|&(a, b)| a < end && start < b) {
            mask |= 1 << page;
        }
    }
    // Every page each needed range touches, not just its two ends, so a range that spans a
    // withheld page is refused too. The offset named is the first needed byte on the bad page.
    for (first, last) in caps.needed_ranges() {
        for page in first / PAGE..=last / PAGE {
            let offset = first.max(page * PAGE);
            if page >= pages {
                return Err(WindowRefusal::BeyondWindow { offset });
            }
            if mask & (1 << page) == 0 {
                return Err(WindowRefusal::SharesTheInterruptPage { offset });
            }
        }
    }
    Ok(mask)
}

/// **Everything the driver is told at spawn**, and the whole of what it could not discover for
/// itself: where its DMA region is physically (a process knows only virtual addresses, and every
/// pointer the controller reads is physical), how many pages it is, and which register pages it
/// was mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handoff {
    /// The DMA region's physical base. Page aligned.
    pub dma_phys: u64,
    /// The region's length in pages: [`dma::WORK_PAGES`] plus the controller's scratchpads.
    pub dma_pages: u32,
    /// The register pages mapped, [`register_window`]'s mask.
    pub registers: u16,
}

impl Handoff {
    /// The three spawn words.
    pub const fn pack(&self) -> [u64; 3] {
        [
            self.dma_phys,
            self.dma_pages as u64 | (self.registers as u64) << 32,
            0,
        ]
    }

    /// The three spawn words back, or `None` for words no kernel would have sent: an unaligned or
    /// zero base, a region too small for the layout, or a register mask without page 0 (which
    /// holds the capability registers the driver reads first).
    pub fn unpack(w: [u64; 3]) -> Option<Handoff> {
        let dma_pages = u32::try_from(w[1] & 0xffff_ffff).ok()?;
        let registers = u16::try_from(w[1] >> 32).ok()?;
        let ok = w[0] != 0
            && w[0].is_multiple_of(PAGE)
            && u64::from(dma_pages) >= dma::WORK_PAGES
            && registers & 1 != 0
            && w[2] == 0;
        ok.then_some(Handoff {
            dma_phys: w[0],
            dma_pages,
            registers,
        })
    }
}

/// TRB type codes (xHCI 1.2 Table 6-91).
pub mod trb_type {
    /// Normal: a data buffer on a bulk or interrupt ring.
    pub const NORMAL: u8 = 1;
    /// Setup Stage of a control transfer.
    pub const SETUP: u8 = 2;
    /// Data Stage of a control transfer.
    pub const DATA: u8 = 3;
    /// Status Stage of a control transfer.
    pub const STATUS: u8 = 4;
    /// Link: the producer ring's way back to its start.
    pub const LINK: u8 = 6;
    /// Enable Slot command.
    pub const ENABLE_SLOT: u8 = 9;
    /// Disable Slot command.
    pub const DISABLE_SLOT: u8 = 10;
    /// Address Device command.
    pub const ADDRESS_DEVICE: u8 = 11;
    /// Configure Endpoint command.
    pub const CONFIGURE_ENDPOINT: u8 = 12;
    /// Evaluate Context command.
    pub const EVALUATE_CONTEXT: u8 = 13;
    /// Reset Endpoint command: take an endpoint out of the Halted state a stall left it in.
    pub const RESET_ENDPOINT: u8 = 14;
    /// Set TR Dequeue Pointer command: move an endpoint's ring past what a stall abandoned.
    pub const SET_TR_DEQUEUE: u8 = 16;
    /// Transfer Event.
    pub const TRANSFER_EVENT: u8 = 32;
    /// Command Completion Event.
    pub const COMMAND_COMPLETION: u8 = 33;
    /// Port Status Change Event.
    pub const PORT_STATUS_CHANGE: u8 = 34;
}

/// Completion codes (xHCI 1.2 Table 6-90), the ones a keyboard driver tells apart.
pub mod completion {
    /// The command or transfer did what was asked.
    pub const SUCCESS: u8 = 1;
    /// USB transaction error: the device did not answer, or answered garbage.
    pub const TRANSACTION_ERROR: u8 = 4;
    /// The device stalled the endpoint: it refused the request.
    pub const STALL: u8 = 6;
    /// The transfer finished with fewer bytes than the buffer: normal for a descriptor read.
    pub const SHORT_PACKET: u8 = 13;
    /// No slot was free for Enable Slot.
    pub const NO_SLOTS: u8 = 9;
}

/// **One TRB**: four little-endian dwords, the cycle bit in bit 0 of the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trb(pub [u32; 4]);

/// Interrupt On Completion: ask for an event when this TRB completes.
const IOC: u32 = 1 << 5;
/// Interrupt on Short Packet.
const ISP: u32 = 1 << 2;
/// Immediate Data: the TRB's first two dwords are the data itself (the setup packet).
const IDT: u32 = 1 << 6;

const fn control(kind: u8) -> u32 {
    (kind as u32) << 10
}

impl Trb {
    /// The TRB's type code.
    pub const fn kind(&self) -> u8 {
        ((self.0[3] >> 10) & 0x3f) as u8
    }

    /// The cycle bit.
    pub const fn cycle(&self) -> bool {
        self.0[3] & 1 != 0
    }

    /// The same TRB stamped with `cycle`.
    pub const fn with_cycle(self, cycle: bool) -> Trb {
        let mut d = self.0;
        d[3] = (d[3] & !1) | cycle as u32;
        Trb(d)
    }

    const fn pointer(address: u64, rest: [u32; 2]) -> Trb {
        Trb([address as u32, (address >> 32) as u32, rest[0], rest[1]])
    }

    /// A link TRB back to `target` with Toggle Cycle set, which is what flips the consumer's
    /// cycle as it follows the link.
    pub const fn link(target: u64) -> Trb {
        Trb::pointer(target, [0, control(trb_type::LINK) | 1 << 1])
    }

    /// Enable Slot: ask the controller for a device slot.
    pub const fn enable_slot() -> Trb {
        Trb([0, 0, 0, control(trb_type::ENABLE_SLOT)])
    }

    /// Disable Slot `slot`: give it back.
    pub const fn disable_slot(slot: u8) -> Trb {
        Trb([
            0,
            0,
            0,
            control(trb_type::DISABLE_SLOT) | (slot as u32) << 24,
        ])
    }

    /// Address Device: take the input context at `input` (physical) for `slot`, and send the
    /// device its address.
    pub const fn address_device(input: u64, slot: u8) -> Trb {
        Trb::pointer(
            input,
            [0, control(trb_type::ADDRESS_DEVICE) | (slot as u32) << 24],
        )
    }

    /// Configure Endpoint: add the endpoints the input context's add flags name.
    pub const fn configure_endpoint(input: u64, slot: u8) -> Trb {
        Trb::pointer(
            input,
            [
                0,
                control(trb_type::CONFIGURE_ENDPOINT) | (slot as u32) << 24,
            ],
        )
    }

    /// Evaluate Context: update the fields the input context names (here, endpoint 0's packet
    /// size once the device has said what it is).
    pub const fn evaluate_context(input: u64, slot: u8) -> Trb {
        Trb::pointer(
            input,
            [0, control(trb_type::EVALUATE_CONTEXT) | (slot as u32) << 24],
        )
    }

    /// Reset Endpoint `dci` of `slot`, after a stall halted it.
    pub const fn reset_endpoint(slot: u8, dci: u8) -> Trb {
        Trb([
            0,
            0,
            0,
            control(trb_type::RESET_ENDPOINT) | (dci as u32) << 16 | (slot as u32) << 24,
        ])
    }

    /// Set TR Dequeue Pointer of endpoint `dci` of `slot` to `pointer` (the ring position with the
    /// consumer cycle in bit 0, [`ProducerRing::dequeue_pointer`]), skipping what a stall left on
    /// the ring.
    pub const fn set_dequeue(pointer: u64, slot: u8, dci: u8) -> Trb {
        Trb::pointer(
            pointer,
            [
                0,
                control(trb_type::SET_TR_DEQUEUE) | (dci as u32) << 16 | (slot as u32) << 24,
            ],
        )
    }

    /// A control transfer's Setup Stage, the eight-byte packet carried immediately. `data_in` is
    /// `Some(true)` for a data stage device to host, `Some(false)` host to device, `None` for none
    /// (the Transfer Type field, xHCI 1.2 section 6.4.1.2.1).
    pub const fn setup(packet: u64, data_in: Option<bool>) -> Trb {
        let trt = match data_in {
            None => 0,
            Some(false) => 2,
            Some(true) => 3,
        };
        Trb([
            packet as u32,
            (packet >> 32) as u32,
            8,
            control(trb_type::SETUP) | IDT | trt << 16,
        ])
    }

    /// A control transfer's Data Stage: `len` bytes at `buffer` (physical).
    pub const fn data(buffer: u64, len: u16, inbound: bool) -> Trb {
        Trb::pointer(
            buffer,
            [
                len as u32,
                control(trb_type::DATA) | (inbound as u32) << 16 | ISP,
            ],
        )
    }

    /// A control transfer's Status Stage, which flows opposite to the data stage (device to host
    /// when there was none, or when it flowed host to device). Asks for the completion event.
    pub const fn status(data_was_inbound: bool) -> Trb {
        Trb([
            0,
            0,
            0,
            control(trb_type::STATUS) | (!data_was_inbound as u32) << 16 | IOC,
        ])
    }

    /// A Normal TRB: `len` bytes at `buffer` (physical), with an event on completion or a short
    /// packet. What an interrupt IN endpoint's ring holds.
    pub const fn normal(buffer: u64, len: u16) -> Trb {
        Trb::pointer(buffer, [len as u32, control(trb_type::NORMAL) | IOC | ISP])
    }
}

/// **One event, decoded.** The fields a driver uses, by event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A transfer finished (or failed).
    Transfer {
        /// The physical address of the TRB it was about.
        trb: u64,
        /// Bytes *not* transferred: zero for a full buffer, positive for a short packet.
        residual: u32,
        /// The completion code.
        code: u8,
        /// The device slot.
        slot: u8,
        /// The endpoint's device context index (1 for endpoint 0).
        endpoint: u8,
    },
    /// A command finished.
    Command {
        /// The physical address of the command TRB.
        trb: u64,
        /// The completion code.
        code: u8,
        /// The slot it concerns: for Enable Slot, the slot the controller chose.
        slot: u8,
    },
    /// A port's status changed: the driver reads `PORTSC` to learn how.
    PortStatus {
        /// The port, numbered from 1.
        port: u8,
    },
    /// Any other event type, which this driver does not act on.
    Other {
        /// Its type code.
        kind: u8,
    },
}

impl Event {
    /// Decode an event TRB. Total: every input is some variant.
    pub const fn decode(t: Trb) -> Event {
        let d = t.0;
        let pointer = d[0] as u64 | (d[1] as u64) << 32;
        let code = (d[2] >> 24) as u8;
        let slot = (d[3] >> 24) as u8;
        match t.kind() {
            trb_type::TRANSFER_EVENT => Event::Transfer {
                trb: pointer,
                residual: d[2] & 0x00ff_ffff,
                code,
                slot,
                endpoint: ((d[3] >> 16) & 0x1f) as u8,
            },
            trb_type::COMMAND_COMPLETION => Event::Command {
                trb: pointer,
                code,
                slot,
            },
            trb_type::PORT_STATUS_CHANGE => Event::PortStatus {
                port: (d[0] >> 24) as u8,
            },
            kind => Event::Other { kind },
        }
    }
}

/// **A ring the driver produces into**: the command ring and every transfer ring. Its last entry is
/// always a link TRB back to the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProducerRing {
    base: u64,
    entries: u16,
    enqueue: u16,
    cycle: bool,
}

/// **What [`ProducerRing::enqueue`] says to write**: the TRB at `index`, stamped with this lap's
/// cycle, and when the ring has just filled its lap, the link TRB at the last index too.
///
/// The driver writes `trb` and then `link`, each with the cycle dword last: until a TRB's cycle bit
/// matches, the controller treats it as not yet produced, so the dword that carries it goes in
/// after the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// Where the TRB goes.
    pub index: u16,
    /// The TRB, cycle stamped.
    pub trb: Trb,
    /// The link TRB and where it goes, when this enqueue filled the lap.
    pub link: Option<(u16, Trb)>,
}

impl ProducerRing {
    /// A ring of `entries` TRBs (the last of them the link) at physical `base`, in the state a
    /// zeroed ring starts in: producer cycle 1, so the zeros read as not produced.
    ///
    /// # Panics
    /// When `entries` is under 2, which is a ring with no room for anything but its link; every
    /// caller passes a constant.
    pub const fn new(base: u64, entries: u16) -> ProducerRing {
        assert!(
            entries >= 2,
            "a producer ring needs a slot besides its link"
        );
        ProducerRing {
            base,
            entries,
            enqueue: 0,
            cycle: true,
        }
    }

    /// The physical address of entry `index`.
    pub const fn address_of(&self, index: u16) -> u64 {
        self.base + index as u64 * 16
    }

    /// The physical address and cycle of the next TRB to be written: what a TR Dequeue Pointer or
    /// `CRCR` is set to (the cycle in bit 0, as both registers take it).
    pub const fn dequeue_pointer(&self) -> u64 {
        self.address_of(self.enqueue) | self.cycle as u64
    }

    /// **Place one TRB**, and the link if this fills the lap.
    pub fn enqueue(&mut self, trb: Trb) -> Placement {
        let index = self.enqueue;
        let placed = trb.with_cycle(self.cycle);
        self.enqueue += 1;
        let mut link = None;
        if self.enqueue == self.entries - 1 {
            link = Some((self.enqueue, Trb::link(self.base).with_cycle(self.cycle)));
            self.enqueue = 0;
            self.cycle = !self.cycle;
        }
        Placement {
            index,
            trb: placed,
            link,
        }
    }
}

/// **The event ring, from the consumer's side**: one segment of `entries` TRBs at a physical base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventRing {
    base: u64,
    entries: u16,
    dequeue: u16,
    cycle: bool,
}

impl EventRing {
    /// A zeroed ring: consumer cycle 1, so the zeros read as not yet written.
    ///
    /// # Panics
    /// When `entries` is zero; every caller passes a constant.
    pub const fn new(base: u64, entries: u16) -> EventRing {
        assert!(entries > 0, "an event ring needs an entry");
        EventRing {
            base,
            entries,
            dequeue: 0,
            cycle: true,
        }
    }

    /// The entry the next event will be at.
    pub const fn index(&self) -> u16 {
        self.dequeue
    }

    /// Whether `trb`, read at [`index`](Self::index), is an event the controller has written
    /// this lap.
    pub const fn is_ready(&self, trb: &Trb) -> bool {
        trb.cycle() == self.cycle
    }

    /// Consume the entry at [`index`](Self::index).
    pub fn advance(&mut self) {
        self.dequeue += 1;
        if self.dequeue == self.entries {
            self.dequeue = 0;
            self.cycle = !self.cycle;
        }
    }

    /// The physical address to write to `ERDP`: where the next event will be.
    pub const fn dequeue_pointer(&self) -> u64 {
        self.base + self.dequeue as u64 * 16
    }

    /// The one event ring segment table entry describing this ring.
    pub const fn segment_table_entry(&self) -> [u32; 4] {
        [
            self.base as u32,
            (self.base >> 32) as u32,
            self.entries as u32,
            0,
        ]
    }
}

/// **The `bInterval` of an interrupt endpoint as the xHCI Interval field**, which is an exponent:
/// the period is `2^Interval` microframes of 125 µs (xHCI 1.2 section 6.2.3.6).
///
/// Full- and low-speed devices state the period in 1 ms frames (1 to 255), so the field is the
/// largest exponent whose period does not exceed it, clamped to the specification's 3 (1 ms) to
/// 10 (128 ms). High-speed and `SuperSpeed` devices already state an exponent, plus one (1 to 16),
/// so the field is that minus one.
pub const fn interrupt_interval(speed: u8, b_interval: u8) -> u8 {
    if speed == port::speed::FULL || speed == port::speed::LOW {
        let microframes = (if b_interval == 0 { 1 } else { b_interval }) as u32 * 8;
        let exponent = 31 - microframes.leading_zeros();
        let exponent = if exponent < 3 { 3 } else { exponent };
        (if exponent > 10 { 10 } else { exponent }) as u8
    } else {
        let b = if b_interval == 0 {
            1
        } else if b_interval > 16 {
            16
        } else {
            b_interval
        };
        b - 1
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    fn qemu() -> Capabilities {
        Capabilities::decode(0x0100_0040, 0x0400_0840, 0, 0x0000_7000, 0x2000, 0x1000)
    }

    #[test]
    fn qemus_window_withholds_the_msix_page_and_nothing_else() {
        assert_eq!(
            register_window(&qemu(), 0x4000, &[(0x3000, 0x3100), (0x3800, 0x3808)]),
            Ok(0b0111)
        );
        // No MSI-X at all (a function delivering INTx): every page.
        assert_eq!(register_window(&qemu(), 0x4000, &[]), Ok(0b1111));
    }

    /// **The refusal that makes the rule checkable**: a controller whose doorbells share a page
    /// with its MSI-X table cannot be handed to a driver at all, because mapping the doorbells
    /// would map the table.
    #[test]
    fn a_needed_register_on_the_interrupt_page_is_refused() {
        assert_eq!(
            register_window(&qemu(), 0x4000, &[(0x2000, 0x2100)]),
            Err(WindowRefusal::SharesTheInterruptPage { offset: 0x2000 })
        );
    }

    #[test]
    fn a_needed_register_beyond_the_bar_is_refused() {
        assert_eq!(
            register_window(&qemu(), 0x2000, &[]),
            Err(WindowRefusal::BeyondWindow { offset: 0x2000 })
        );
    }

    /// An Intel-shaped controller: a 64 KiB BAR, runtime at 0x2000, doorbells at 0x3000, the
    /// MSI-X table well above them. Values from the xHCI capability layout Intel's datasheets
    /// describe, not read from xenon (calef's bench step reads xenon's own).
    #[test]
    fn a_sixty_four_kibibyte_bar_maps_sixteen_pages_less_the_table() {
        let caps = Capabilities::decode(0x0100_0080, 0x1a00_0840, 0, 0x0000_7000, 0x3000, 0x2000);
        let mask = register_window(&caps, 0x1_0000, &[(0xa000, 0xa000 + 16 * 8)]).unwrap();
        assert_eq!(mask, 0xffff & !(1 << 10));
    }

    #[test]
    fn the_handoff_round_trips_and_refuses_what_no_kernel_sends() {
        let h = Handoff {
            dma_phys: 0x4000_0000,
            dma_pages: 12,
            registers: 0b0111,
        };
        assert_eq!(Handoff::unpack(h.pack()), Some(h));
        assert_eq!(Handoff::unpack([0x4000_0001, 12 | 7 << 32, 0]), None);
        assert_eq!(Handoff::unpack([0, 12 | 7 << 32, 0]), None);
        assert_eq!(Handoff::unpack([0x4000_0000, 3 | 7 << 32, 0]), None);
        assert_eq!(Handoff::unpack([0x4000_0000, 12 | 6 << 32, 0]), None);
    }

    #[test]
    fn events_decode_by_type() {
        // A command completion for Enable Slot, success, slot 1.
        let t = Trb([0x1000, 0, 1 << 24, 33 << 10 | 1 << 24 | 1]);
        assert_eq!(
            Event::decode(t),
            Event::Command {
                trb: 0x1000,
                code: completion::SUCCESS,
                slot: 1
            }
        );
        // A transfer event on endpoint 3 (EP1 IN), short by 2.
        let t = Trb([0x7000, 0, 13 << 24 | 2, 32 << 10 | 3 << 16 | 1 << 24 | 1]);
        assert_eq!(
            Event::decode(t),
            Event::Transfer {
                trb: 0x7000,
                residual: 2,
                code: completion::SHORT_PACKET,
                slot: 1,
                endpoint: 3
            }
        );
        let t = Trb([5 << 24, 0, 1 << 24, 34 << 10 | 1]);
        assert_eq!(Event::decode(t), Event::PortStatus { port: 5 });
    }

    /// The event ring's consumer flips its cycle at the wrap, so last lap's events read as stale.
    #[test]
    fn the_event_ring_laps() {
        let mut ring = EventRing::new(0x2000, 2);
        let fresh = Trb([0, 0, 0, 1]);
        assert!(ring.is_ready(&fresh));
        ring.advance();
        assert_eq!(ring.dequeue_pointer(), 0x2010);
        ring.advance();
        assert_eq!(ring.dequeue_pointer(), 0x2000);
        assert!(!ring.is_ready(&fresh));
        assert_eq!(ring.segment_table_entry(), [0x2000, 0, 2, 0]);
    }

    #[test]
    fn a_setup_trb_carries_the_packet_and_the_transfer_type() {
        let t = Trb::setup(0x0008_0000_0100_0680, Some(true));
        assert_eq!(t.0[0], 0x0100_0680);
        assert_eq!(t.0[1], 0x0008_0000);
        assert_eq!(t.0[2], 8);
        assert_eq!(t.kind(), trb_type::SETUP);
        assert_eq!((t.0[3] >> 16) & 3, 3);
        assert!(t.0[3] & IDT != 0);
        // The status stage of an IN transfer flows OUT, and of a no-data transfer flows IN.
        assert_eq!((Trb::status(true).0[3] >> 16) & 1, 0);
        assert_eq!((Trb::status(false).0[3] >> 16) & 1, 1);
    }

    #[test]
    fn the_interval_field_follows_the_speed() {
        // Full speed, 10 ms: 80 microframes, so 2^6 = 64 is the largest period not over it.
        assert_eq!(interrupt_interval(port::speed::FULL, 10), 6);
        assert_eq!(interrupt_interval(port::speed::LOW, 1), 3);
        assert_eq!(interrupt_interval(port::speed::FULL, 255), 10);
        assert_eq!(interrupt_interval(port::speed::FULL, 0), 3);
        // High speed states an exponent plus one.
        assert_eq!(interrupt_interval(port::speed::HIGH, 4), 3);
        assert_eq!(interrupt_interval(port::speed::HIGH, 0), 0);
        assert_eq!(interrupt_interval(port::speed::SUPER, 200), 15);
    }

    /// **A simulated controller consuming what the driver produces**: it reads from its dequeue
    /// index while the cycle bit matches its own, follows a link TRB and toggles its cycle when the
    /// link says so (xHCI 1.2 section 4.9.2), and returns what it ran. Three laps of a four-entry
    /// ring, so every wrap and every link is exercised: the controller must run exactly the TRBs
    /// produced, in order, and stop where production stops.
    #[test]
    fn a_simulated_controller_runs_exactly_what_was_produced_across_laps() {
        const ENTRIES: u16 = 4;
        let base = 0x8000;
        let mut memory = [Trb([0; 4]); ENTRIES as usize];
        let mut ring = ProducerRing::new(base, ENTRIES);
        let (mut dequeue, mut ccs) = (0u16, true);
        let run = |memory: &[Trb; ENTRIES as usize], dequeue: &mut u16, ccs: &mut bool| {
            let mut ran = std::vec::Vec::new();
            for _ in 0..16 {
                let t = memory[*dequeue as usize];
                if t.cycle() != *ccs {
                    break;
                }
                if t.kind() == trb_type::LINK {
                    let target = u64::from(t.0[0]) | u64::from(t.0[1]) << 32;
                    assert_eq!(target, base, "a link leads back to the ring's start");
                    if t.0[3] & 1 << 1 != 0 {
                        *ccs = !*ccs;
                    }
                    *dequeue = 0;
                    continue;
                }
                ran.push(t.0[0]);
                *dequeue += 1;
            }
            ran
        };
        let mut next = 1u32;
        // Never more than the ring's three usable entries between two runs: a fourth would overwrite
        // a TRB the controller has not reached, which is the producer's rule, not the ring's.
        for burst in [2u32, 3, 1, 3, 2, 3] {
            let mut produced = std::vec::Vec::new();
            for _ in 0..burst {
                let p = ring.enqueue(Trb::normal(u64::from(next), 8));
                memory[p.index as usize] = p.trb;
                if let Some((at, link)) = p.link {
                    memory[at as usize] = link;
                }
                produced.push(next);
                next += 1;
            }
            assert_eq!(run(&memory, &mut dequeue, &mut ccs), produced);
            // Where the controller stopped is where the driver will write next.
            assert_eq!(ring.dequeue_pointer() & !1, base + u64::from(dequeue) * 16);
            assert_eq!(ring.dequeue_pointer() & 1 != 0, ccs);
        }
    }

    /// **The three shapes of a control transfer**, as a controller reads them: the setup stage's
    /// transfer type must agree with the data stage's direction, and the status stage flows the
    /// other way (xHCI 1.2 section 4.11.2.2). Only the status stage asks for an event on success,
    /// which is what the driver waits for; the data stage asks only on a short packet.
    #[test]
    fn control_transfers_agree_on_direction_in_all_three_shapes() {
        for (data_in, trt, status_in) in [
            (None, 0, true),
            (Some(false), 2, true),
            (Some(true), 3, false),
        ] {
            let setup = Trb::setup(0, data_in);
            assert_eq!((setup.0[3] >> 16) & 3, trt);
            let status = Trb::status(data_in == Some(true));
            assert_eq!(status.kind(), trb_type::STATUS);
            assert_eq!((status.0[3] >> 16) & 1 != 0, status_in);
            assert!(status.0[3] & IOC != 0);
            if let Some(inbound) = data_in {
                let data = Trb::data(0x1_2345_6000, 18, inbound);
                assert_eq!(data.kind(), trb_type::DATA);
                assert_eq!((data.0[0], data.0[1], data.0[2]), (0x2345_6000, 1, 18));
                assert_eq!((data.0[3] >> 16) & 1 != 0, inbound);
                assert_eq!((data.0[3] & IOC, data.0[3] & ISP != 0), (0, true));
            }
        }
        let normal = Trb::normal(0x4000, 8);
        assert_eq!((normal.kind(), normal.0[2]), (trb_type::NORMAL, 8));
        assert!(normal.0[3] & IOC != 0 && normal.0[3] & ISP != 0);
    }

    /// **Every command names its slot where a controller looks for it** (bits 31:24 of the last
    /// dword), the pointer commands carry their input context, and the endpoint commands their
    /// endpoint (bits 20:16). A simulated command dispatcher decodes each one back.
    #[test]
    fn every_command_decodes_back_to_its_slot_endpoint_and_pointer() {
        let decode = |t: Trb| {
            (
                t.kind(),
                (t.0[3] >> 24) as u8,
                ((t.0[3] >> 16) & 0x1f) as u8,
                u64::from(t.0[0]) | u64::from(t.0[1]) << 32,
            )
        };
        assert_eq!(decode(Trb::enable_slot()), (trb_type::ENABLE_SLOT, 0, 0, 0));
        assert_eq!(
            decode(Trb::disable_slot(7)),
            (trb_type::DISABLE_SLOT, 7, 0, 0)
        );
        assert_eq!(
            decode(Trb::address_device(0x5000, 1)),
            (trb_type::ADDRESS_DEVICE, 1, 0, 0x5000)
        );
        assert_eq!(
            decode(Trb::configure_endpoint(0x5000, 2)),
            (trb_type::CONFIGURE_ENDPOINT, 2, 0, 0x5000)
        );
        assert_eq!(
            decode(Trb::evaluate_context(0x5000, 3)),
            (trb_type::EVALUATE_CONTEXT, 3, 0, 0x5000)
        );
        assert_eq!(
            decode(Trb::reset_endpoint(1, 3)),
            (trb_type::RESET_ENDPOINT, 1, 3, 0)
        );
        assert_eq!(
            decode(Trb::set_dequeue(0x7001, 1, 3)),
            (trb_type::SET_TR_DEQUEUE, 1, 3, 0x7001)
        );
        // An event type this driver does not act on decodes as itself, never as a known one.
        assert_eq!(
            Event::decode(Trb([0, 0, 0, 37 << 10])),
            Event::Other { kind: 37 }
        );
        let ring = EventRing::new(0x2000, 4);
        assert_eq!(ring.index(), 0);
    }

    /// A doorbell range that runs onto a withheld page is refused at the first needed byte on it,
    /// and so is a needed range that starts on one.
    #[test]
    fn a_needed_range_crossing_onto_a_withheld_page_is_refused() {
        // 64 slots of doorbells from 0x2f00 run to 0x3003, onto page 3.
        let caps = Capabilities::decode(0x0100_0040, 0x0400_0840, 0, 0, 0x2f00, 0x1000);
        assert_eq!(
            register_window(&caps, 0x4000, &[(0x3000, 0x3100)]),
            Err(WindowRefusal::SharesTheInterruptPage { offset: 0x3000 })
        );
    }

    #[test]
    fn a_producer_ring_of_two_links_every_time() {
        let mut ring = ProducerRing::new(0x9000, 2);
        let p = ring.enqueue(Trb::enable_slot());
        assert_eq!(p.index, 0);
        let (at, link) = p.link.unwrap();
        assert_eq!(at, 1);
        assert_eq!(link.kind(), trb_type::LINK);
        assert_eq!(link.0[0], 0x9000);
        assert_eq!(ring.dequeue_pointer(), 0x9000); // back at 0, cycle now 0
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    /// **No page that overlaps a withheld range is ever in the mask**, for any capability values,
    /// any BAR size and any one withheld range: the MSI-X table is never mapped into the driver.
    /// Falsification: replayable `crates/extensible_host_controller_interface/falsifications/verification.the_window_never_maps_a_withheld_page.patch`
    #[kani::proof]
    #[kani::unwind(18)]
    fn the_window_never_maps_a_withheld_page() {
        let caps = Capabilities::decode(
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
        );
        let bar: u64 = kani::any();
        let a: u64 = kani::any();
        let len: u64 = kani::any();
        kani::assume(len > 0 && len <= 0x1_0000 && a <= u64::MAX - len);
        if let Ok(mask) = register_window(&caps, bar, &[(a, a + len)]) {
            for page in 0..MAX_REGISTER_PAGES {
                if mask & (1 << page) != 0 {
                    let (s, e) = (page * PAGE, (page + 1) * PAGE);
                    assert!(!(a < e && s < a + len));
                }
            }
            assert!(mask >> MAX_REGISTER_PAGES == 0);
        }
    }

    /// **A producer ring never places a TRB on its link slot or past the end**, and places the link
    /// exactly on the last slot, from every reachable state.
    /// Falsification: replayable `crates/extensible_host_controller_interface/falsifications/verification.a_producer_ring_stays_in_bounds.patch`
    #[kani::proof]
    fn a_producer_ring_stays_in_bounds() {
        let entries: u16 = kani::any();
        let enqueue: u16 = kani::any();
        kani::assume(entries >= 2 && enqueue < entries - 1);
        let mut ring = ProducerRing {
            base: kani::any(),
            entries,
            enqueue,
            cycle: kani::any(),
        };
        let p = ring.enqueue(Trb(kani::any()));
        assert!(p.index < entries - 1);
        if let Some((at, link)) = p.link {
            assert_eq!(at, entries - 1);
            assert_eq!(link.kind(), trb_type::LINK);
        }
        assert!(ring.enqueue < entries - 1);
    }

    /// **The handoff round-trips, and anything `unpack` accepts is a layout the driver can use.**
    /// Falsification: replayable `crates/extensible_host_controller_interface/falsifications/verification.the_handoff_round_trips.patch`
    #[kani::proof]
    fn the_handoff_round_trips() {
        let w: [u64; 3] = kani::any();
        if let Some(h) = Handoff::unpack(w) {
            assert_eq!(h.pack(), w);
            assert!(u64::from(h.dma_pages) >= dma::WORK_PAGES);
            assert!(h.registers & 1 != 0);
        }
    }
}
