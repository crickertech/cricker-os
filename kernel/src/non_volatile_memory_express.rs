//! **The NVMe controller's admin plane** (milestone 53's storage half, narrowed to the admin half
//! by milestone 261 (the NVMe driver leaves the kernel); notes/non-volatile-memory-express.md, [DECISIONS §86 (whether an NVMe driver can leave the kernel)](../../design/decisions/86-el0-nvme-driver.md)).
//!
//! What is left in the kernel after §86's **option 2a**, and the line is the one the hardware
//! already draws. Creating a queue names the physical address a ring lives at, in a PRP field of
//! an admin command; the admin rings' own bases are the `ASQ` and `ACQ` registers. **That is the
//! dangerous authority**, so it stays at EL1: reset the controller, build the admin pair by
//! register, enable it, IDENTIFY the namespace, and create the one I/O queue pair whose rings are
//! inside the region the IOMMU has already confined the device to. A process that never issues an
//! admin command and never touches those registers cannot choose where any ring lives, which is
//! what makes the geometry below a guarantee rather than an agreement.
//!
//! **The data path is gone from this file and runs at EL0.** Building a command, copying it into
//! the submission ring, ringing the doorbell and watching a completion's phase tag are
//! `components/src/non_volatile_memory_express.rs`'s now, over the same `crates/non_volatile_memory_express` arithmetic this module
//! computes with. The bring-up below is still a test of that machinery, because the admin queue
//! rides the identical ring discipline.
//!
//! Rule 2 shapes what is here exactly as it shaped the whole driver before: [`NonVolatileMemoryExpress`] takes the
//! register window's virtual base and one DMA region, passed in, and reaches for nothing else.
//! What reaches into the kernel is [`bring_up`], the policy function, which finds the controller
//! on the bus (`pci::find_nvme_device`), allocates the DMA region, and **confines the device to it
//! in hardware** before the controller is enabled: on all three `virt` machines the IOMMU denies an
//! unlisted requester id by default (milestone 16b), so a controller that has not been confined
//! cannot fetch its first command, and this bring-up is proof the confinement admits exactly what
//! it should.
//!
//! Name: ratified 2026-09-17 (calef, DECISIONS §154), performed 2026-09-18. Introduced 2026-08-15
//! with milestone 53's NVMe block driver as `nvme.rs`, the volatile half of the `nvme` crate, and
//! it takes its crate's name because that is what the pairing means: `pci` and `virtio` already
//! sit this way. The argument and the refusals are recorded once, beside the crate, in
//! `crates/non_volatile_memory_express`.

use non_volatile_memory_express::{
    Cap, Command, Completion, CqState, Doorbell, Handoff, IdentifyNamespace, SqState, regs,
};

use crate::arch::mmu;

/// One transfer unit: a filesystem block, the same unit the blk-IPC protocol moves, so the EL0
/// block server serves the FS server without a translation layer.
pub const BLOCK_SIZE: usize = filesystem_protocol::blk::BLOCK_SIZE;

/// The DMA region's layout, in page offsets. One contiguous run, allocated by [`bring_up`] and
/// confined as one region, in two halves that are **not** the same authority:
///
/// - pages 0..[`DATA_PLANE_PAGE`] are the **admin plane's**, and no EL0 process is mapped them:
///   the two admin rings and the IDENTIFY buffer;
/// - pages [`DATA_PLANE_PAGE`].. are the **data plane's**, mapped into the EL0 server: the I/O
///   submission ring, the I/O completion ring, and [`TRANSFER_PAGES`] pages of transfer buffer.
///
/// The IOMMU confines the device to the whole run, because the controller fetches from both
/// halves. Splitting the *mappings* is what withholds the admin plane from the driver; see
/// `components/src/non_volatile_memory_express.rs`'s "What it holds" for what that does and does not buy.
const ADMIN_SQ_PAGE: u64 = 0;
const ADMIN_CQ_PAGE: u64 = 1;
const IDENTIFY_PAGE: u64 = 2;
/// The first page of the data plane's run, and the base [`Handoff::data_plane_phys`] carries.
pub const DATA_PLANE_PAGE: u64 = 3;
const IO_SQ_PAGE: u64 = 3;
const IO_CQ_PAGE: u64 = 4;
/// Where the transfer buffer starts, relative to [`DATA_PLANE_PAGE`]. The EL0 server computes the
/// same offset from what it was handed; the constant is duplicated there against this one because
/// the two live in different address spaces and nothing but the spawn contract joins them.
pub const TRANSFER_PAGES: u64 = filesystem_protocol::blk::TRANSFER_BLOCKS as u64;
const DMA_PAGES: u64 = 5 + TRANSFER_PAGES;

/// **Test-only escape-target hook for the confinement falsification** (milestone 261's Outstanding
/// item; milestone 202 (every confinement test is a ritual until somebody breaks the confinement and watches it fail)'s convention: a claim, a test, a replayable falsification). Zero in every
/// shipped boot and in every passing test, so [`bring_up`] confines the controller to its own DMA
/// region and nothing else. The replayable falsification
/// (`system_tests/falsifications/...a_confined_el0_server_cannot_dma_outside_its_region.patch`)
/// stores the physical base of a frame *outside* that region here; [`bring_up`] then widens the
/// controller's IOMMU domain to include it, so an EL0 server that aims a PRP at that frame has its
/// DMA land instead of faulting, and the test's canary is overwritten. That one store is the whole
/// falsification, and it lives only in the patch, which is why the shipped tree never writes this.
#[cfg(feature = "system_tests")]
pub static ESCAPE_TARGET: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Sixteen entries per I/O ring: one page holds 64 (submission) or 256 (completion), but the EL0
/// server completes each command before submitting the next, so depth buys nothing and a small
/// ring keeps the wrap (where the phase discipline earns its keep) inside every test run.
const ENTRIES: u16 = 16;

/// The one I/O queue pair the admin plane creates.
const IO_QID: u16 = 1;

/// The one namespace QEMU's `-device nvme,drive=` carries. Namespace ids are 1-based.
pub const NSID: u32 = 1;

/// How many polls of a status or completion before the driver declares the controller hung. QEMU
/// completes synchronously inside the doorbell write, so a bound this size is pure paranoia; on
/// hardware it is roughly a second of spinning, which is longer than CAP.TO promises.
const SPIN_BOUND: u32 = 50_000_000;

/// Everything that can go wrong under this driver, each naming the phase it died in, because "the
/// disk did not come up" from a five-step bring-up is the kind of message this project's own
/// debugging notes complain about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// CSTS.RDY never followed CC.EN within the spin bound (either direction).
    ControllerTimeout,
    /// CSTS.CFS: the controller declared a fatal internal error.
    ControllerFatal,
    /// A completion never arrived for a submitted command.
    CompletionTimeout,
    /// The controller completed a command with a nonzero status field (the field is carried).
    Command(u16),
    /// The identify data described a namespace this driver cannot serve (LBA format outside
    /// 512..=4096 bytes, or a size of zero). `lba_shift` is the format's LBADS as the controller
    /// reported it, 0 if the structure was too short to carry one, so the refusal names the LBA
    /// size it refused (fatal risk 6's second night-of condition).
    UnsupportedNamespace { lba_shift: u8, blocks: u64 },
    /// CAP describes a controller this driver cannot drive: queues too shallow, a minimum page
    /// size above the kernel's 4 KiB frames, or a doorbell stride whose file would not fit the
    /// one page of BAR0 an EL0 data plane is mapped (`non_volatile_memory_express::MAX_DSTRD`).
    UnsupportedController,
}

/// An initialized NVMe controller with one I/O queue pair, ready to be handed to an EL0 data
/// plane. Constructed only by [`NonVolatileMemoryExpress::new`]; holding one is holding a disk that has already
/// proven it can answer admin commands, and the authority to create more queues.
pub struct NonVolatileMemoryExpress {
    /// BAR0's virtual base: the register file and, from 0x1000, the doorbells.
    regs: u64,
    /// The doorbell stride from CAP, needed for every ring.
    dstrd: u32,
    /// The DMA region, both addresses: commands are built through `direct_memory_access_va`, the
    /// controller is told `direct_memory_access_phys`.
    direct_memory_access_phys: u64,
    direct_memory_access_va: u64,
    admin_sq: SqState,
    admin_cq: CqState,
    /// The namespace's geometry, from IDENTIFY: size and blocks-per-filesystem-block.
    ns: IdentifyNamespace,
    /// The next command identifier. Echoed back in completions and checked there; monotonic and
    /// wrapping, so a stale completion cannot impersonate a fresh one within 65536 commands.
    cid: u16,
}

impl NonVolatileMemoryExpress {
    /// Bring the controller from reset to ready to serve I/O: reset, admin queues, enable, identify
    /// the namespace, create the I/O queue pair. `regs_va` is BAR0's virtual base;
    /// `direct_memory_access_phys`/`direct_memory_access_va` name the same zeroed, physically
    /// contiguous [`DMA_PAGES`]-page region through the two address spaces. The caller has already
    /// confined the device to that region if an IOMMU is active; nothing in here can tell, which is
    /// the point of the confinement being outside.
    pub fn new(
        regs_va: u64,
        direct_memory_access_phys: u64,
        direct_memory_access_va: u64,
    ) -> Result<NonVolatileMemoryExpress, Error> {
        let mut c = NonVolatileMemoryExpress {
            regs: regs_va,
            dstrd: 0,
            direct_memory_access_phys,
            direct_memory_access_va,
            admin_sq: SqState::new(ENTRIES),
            admin_cq: CqState::new(ENTRIES),
            ns: IdentifyNamespace {
                blocks: 0,
                lba_shift: 9,
            },
            cid: 0,
        };
        let cap = Cap(c.reg64(regs::CAP));
        // The doorbell stride is checked here as well as in `non_volatile_memory_express::Handoff::unpack`, and the
        // duplication is deliberate: a controller whose doorbell file does not fit the one page
        // this wiring hands an EL0 data plane should fail at bring-up, loudly, naming the
        // controller, rather than as a process that starts and then cannot address its own
        // doorbells. `MAX_DSTRD` is Kani's finding, and its own doc has the arithmetic.
        if cap.max_queue_entries() < ENTRIES as u32
            || cap.min_page_size() > page_frames::FRAME_SIZE
            || cap.doorbell_stride() > non_volatile_memory_express::MAX_DSTRD
        {
            return Err(Error::UnsupportedController);
        }
        c.dstrd = cap.doorbell_stride();

        // A controller that was already enabled (a warm reboot, or a previous test) must be taken
        // down before the admin queue registers may change; RDY follows EN in both directions.
        if c.reg32(regs::CSTS) & non_volatile_memory_express::CSTS_RDY != 0 {
            c.wr32(regs::CC, 0);
            c.wait_rdy(false)?;
        }

        // The admin queue pair, by register: attributes (both depths, 0-based), then the two
        // page-aligned ring bases. The rings are zeroed pages, which is what makes the phase
        // discipline sound: the controller's first lap writes phase 1 into memory that reads 0.
        c.wr32(regs::AQA, (ENTRIES as u32 - 1) << 16 | (ENTRIES as u32 - 1));
        c.wr64(
            regs::ASQ,
            direct_memory_access_phys + ADMIN_SQ_PAGE * page_frames::FRAME_SIZE,
        );
        c.wr64(
            regs::ACQ,
            direct_memory_access_phys + ADMIN_CQ_PAGE * page_frames::FRAME_SIZE,
        );
        #[cfg(feature = "disk_throughput")]
        c.print_registers("pre ");
        c.wr32(regs::CC, non_volatile_memory_express::cc_enabled());
        let ready = c.wait_rdy(true);
        #[cfg(feature = "disk_throughput")]
        c.print_registers("post");
        ready?;

        // IDENTIFY the namespace: its block count and LBA format are the two facts the block
        // arithmetic below stands on, and refusing an exotic format here beats corrupting it later.
        let prp = direct_memory_access_phys + IDENTIFY_PAGE * page_frames::FRAME_SIZE;
        let cmd = Command::identify(
            c.next_cid(),
            non_volatile_memory_express::CNS_NAMESPACE,
            NSID,
            prp,
        );
        c.transact(cmd)?;
        // SAFETY: the identify page is ours (inside the region bring_up allocated), and the
        // controller finished writing it before the completion above was posted (NVMe's ordering
        // guarantee); the barrier in `transact` ordered those writes before this read.
        let data = unsafe {
            core::slice::from_raw_parts(
                (direct_memory_access_va + IDENTIFY_PAGE * page_frames::FRAME_SIZE) as *const u8,
                page_frames::FRAME_SIZE as usize,
            )
        };
        let refused = Error::UnsupportedNamespace {
            lba_shift: non_volatile_memory_express::lba_format_shift(data).unwrap_or(0),
            blocks: data
                .get(0..8)
                .and_then(|b| b.try_into().ok())
                .map_or(0, u64::from_le_bytes),
        };
        c.ns = non_volatile_memory_express::parse_identify_namespace(data).ok_or(refused)?;
        if c.ns.blocks == 0 || c.ns.blocks_per(BLOCK_SIZE as u64).is_none() {
            return Err(refused);
        }

        // The I/O pair, by admin command, completion queue first: the submission queue names its
        // completion queue, so creating them in the other order is an Invalid Queue Identifier.
        // **Both rings are inside the data plane's half of the region**, which is what makes the
        // EL0 server able to reach them at all; it is the kernel that decided so, here, and the
        // server has no command it could issue to change it.
        let cq_prp = direct_memory_access_phys + IO_CQ_PAGE * page_frames::FRAME_SIZE;
        let cmd = Command::create_io_cq(c.next_cid(), IO_QID, ENTRIES, cq_prp);
        c.transact(cmd)?;
        let sq_prp = direct_memory_access_phys + IO_SQ_PAGE * page_frames::FRAME_SIZE;
        let cmd = Command::create_io_sq(c.next_cid(), IO_QID, ENTRIES, IO_QID, sq_prp);
        c.transact(cmd)?;
        Ok(c)
    }

    /// The disk's capacity in bytes: the blk-IPC `SIZE` answer, which the EL0 server is told
    /// rather than left to ask, because asking means IDENTIFY and IDENTIFY is an admin command.
    pub fn size_bytes(&self) -> u64 {
        self.ns.bytes()
    }

    /// **Take the controller down, then give its DMA region back** (milestone 261 (the NVMe driver
    /// leaves the kernel)'s DMA-escape test, the one caller). Clears `CC.EN` and waits for
    /// `CSTS.RDY` to follow, the same step [`NonVolatileMemoryExpress::new`] takes before reprogramming a
    /// controller that was left enabled. A disabled controller fetches no command and posts no
    /// completion, so only after that are the region's frames freed: freeing them under an enabled
    /// controller is the use-after-free-by-hardware notes/dma.md warns about.
    ///
    /// The IOMMU domain still names the region afterwards, and its tables are not freed: the device
    /// stays attached to them, and the next [`bring_up`] replaces the domain. A frame reallocated
    /// meanwhile is reachable only by a controller that is off.
    ///
    /// `Err` when the controller will not report not-ready (or reports fatal status), and then
    /// nothing is freed, because a controller that may still be running may still DMA.
    ///
    /// Name: ratified 2026-10-05 (calef, #1647).
    #[cfg(feature = "system_tests")]
    pub fn retire(self) -> Result<(), Error> {
        self.wr32(regs::CC, 0);
        self.wait_rdy(false)?;
        for page in 0..DMA_PAGES {
            crate::memory::free(page_frames::PageFrame::from_addr(
                self.direct_memory_access_phys + page * page_frames::FRAME_SIZE,
            ));
        }
        Ok(())
    }

    /// **Everything the EL0 data plane is told**, and the whole of what it could not compute for
    /// itself. See [`non_volatile_memory_express::Handoff`] for why it is three words.
    pub fn handoff(&self) -> Handoff {
        Handoff {
            dstrd: self.dstrd,
            // `new` refused a namespace this returns `None` for, so the unwrap cannot fire.
            blocks_per: self
                .ns
                .blocks_per(BLOCK_SIZE as u64)
                .expect("new() refuses a namespace whose format 4096 is not a multiple of"),
            entries: ENTRIES,
            data_plane_phys: self.direct_memory_access_phys
                + DATA_PLANE_PAGE * page_frames::FRAME_SIZE,
            size_bytes: self.size_bytes(),
        }
    }

    /// Submit one **admin** command and poll its completion: the copy into the ring, the publish
    /// barrier, the tail doorbell, the phase-gated poll, the head doorbell. The I/O queue rides
    /// the identical discipline one privilege level down, which is why this bring-up is itself a
    /// test of the EL0 server's machinery.
    fn transact(&mut self, cmd: Command) -> Result<(), Error> {
        let expect_cid = (cmd.0[0] >> 16) as u16;

        let slot = self.admin_sq.push();
        // SAFETY: slot < ENTRIES and ENTRIES 64-byte entries fit one frame, so the write stays
        // inside the admin submission ring's page of our own DMA region.
        unsafe {
            let dst = (self.direct_memory_access_va
                + ADMIN_SQ_PAGE * page_frames::FRAME_SIZE
                + slot as u64 * 64) as *mut [u32; 16];
            core::ptr::write_volatile(dst, cmd.0);
        }
        // Publish the command before the doorbell: the controller is another observer, and the
        // doorbell write must not be reordered ahead of the entry it announces.
        // direct_memory_access_write_barrier is a full barrier on both ISAs (DSB SY / fence), so it
        // also orders the poll reads below against this ring, which is why one barrier a side is
        // enough.
        crate::arch::direct_memory_access_write_barrier();
        self.wr32(
            non_volatile_memory_express::doorbell(0, Doorbell::SubmissionTail, self.dstrd),
            self.admin_sq.tail() as u32,
        );

        // Poll the completion ring at the head slot until the phase tag says the entry is this
        // lap's. Volatile reads through the direct map; the controller DMAs into the same page.
        let head = self.admin_cq.head();
        let cqe = (self.direct_memory_access_va
            + ADMIN_CQ_PAGE * page_frames::FRAME_SIZE
            + head as u64 * 16) as *const [u32; 4];
        let mut done: Option<Completion> = None;
        for _ in 0..SPIN_BOUND {
            // SAFETY: head < ENTRIES and ENTRIES 16-byte entries fit one frame; reads of our own
            // DMA region are always safe, whatever the device is writing there.
            let c = Completion::from_dwords(unsafe { core::ptr::read_volatile(cqe) });
            if self.admin_cq.is_owned(&c) {
                done = Some(c);
                break;
            }
            if self.reg32(regs::CSTS) & non_volatile_memory_express::CSTS_CFS != 0 {
                return Err(Error::ControllerFatal);
            }
            core::hint::spin_loop();
        }
        #[cfg(feature = "disk_throughput")]
        if done.is_none() {
            // Bench diagnostic (fatal risk 6, xenon, 2026-10-04): which command, and what the
            // head slot of the completion ring and the submission entry hold now.
            // SAFETY: both inside the admin rings' pages of our own DMA region, as above.
            let (cqe_now, sqe0) = unsafe {
                let sqe = (self.direct_memory_access_va
                    + ADMIN_SQ_PAGE * page_frames::FRAME_SIZE
                    + (slot as u64) * 64) as *const u32;
                (core::ptr::read_volatile(cqe), core::ptr::read_volatile(sqe))
            };
            crate::println!(
                "diag nvme cid {expect_cid} opcode {:#04x} never completed; csts {:#x}; sq tail {} cq head {head}",
                cmd.0[0] & 0xff,
                self.reg32(regs::CSTS),
                self.admin_sq.tail(),
            );
            crate::println!(
                "diag nvme cqe[{head}] {:#x} {:#x} {:#x} {:#x}; sqe[{slot}] dw0 {sqe0:#x}",
                cqe_now[0],
                cqe_now[1],
                cqe_now[2],
                cqe_now[3],
            );
        }
        let c = done.ok_or(Error::CompletionTimeout)?;
        // Order the completion's phase read before the payload reads that follow (the identify
        // parse): on a weakly-ordered CPU nothing else stops the data read hoisting above the flag
        // read. Full barrier, as above.
        crate::arch::direct_memory_access_write_barrier();

        // Consume: advance the head (flipping phase on wrap), tell the controller, and let the
        // submission ring reuse what the controller has read.
        let new_head = self.admin_cq.pop();
        self.wr32(
            non_volatile_memory_express::doorbell(0, Doorbell::CompletionHead, self.dstrd),
            new_head as u32,
        );
        self.admin_sq.note_head(c.sq_head);

        // The one-at-a-time discipline makes any other cid a protocol violation worth dying on
        // legibly rather than misattributing a completion.
        assert!(
            c.cid == expect_cid,
            "NVMe completion for cid {} while {} was in flight",
            c.cid,
            expect_cid
        );
        if c.status != 0 {
            return Err(Error::Command(c.status));
        }
        Ok(())
    }

    /// Bench diagnostic (fatal risk 6, xenon, 2026-10-04): the admin plane's registers in one
    /// line each side of enable, so a photograph says which phase a bring-up died in. CSTS bit 1
    /// is CFS.
    #[cfg(feature = "disk_throughput")]
    fn print_registers(&self, when: &str) {
        if when == "pre " {
            crate::println!(
                "diag nvme cap {:#x} vs {:#x} dstrd {}",
                self.reg64(regs::CAP),
                self.reg32(0x08),
                self.dstrd
            );
        }
        crate::println!(
            "diag nvme {when} cc {:#x} csts {:#x} aqa {:#x} asq {:#x} acq {:#x}",
            self.reg32(regs::CC),
            self.reg32(regs::CSTS),
            self.reg32(regs::AQA),
            self.reg64(regs::ASQ),
            self.reg64(regs::ACQ),
        );
    }

    fn next_cid(&mut self) -> u16 {
        self.cid = self.cid.wrapping_add(1);
        self.cid
    }

    /// Wait for CSTS.RDY to become `want`, bounded. CFS short-circuits: a controller that has
    /// declared a fatal error will never become ready and the caller should hear which happened.
    fn wait_rdy(&self, want: bool) -> Result<(), Error> {
        for _ in 0..SPIN_BOUND {
            let csts = self.reg32(regs::CSTS);
            if csts & non_volatile_memory_express::CSTS_CFS != 0 {
                return Err(Error::ControllerFatal);
            }
            if (csts & non_volatile_memory_express::CSTS_RDY != 0) == want {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(Error::ControllerTimeout)
    }

    fn reg32(&self, off: u64) -> u32 {
        // SAFETY: BAR0 is device-mapped (inside the PCI BAR window `mmu::map_everything` maps),
        // `off` comes from `regs`/`doorbell`, both bounded well inside the BAR's 16 KiB.
        unsafe { core::ptr::read_volatile((self.regs + off) as *const u32) }
    }

    fn wr32(&self, off: u64, v: u32) {
        // SAFETY: as `reg32`; the doorbell and control offsets written here are the driver's own.
        unsafe { core::ptr::write_volatile((self.regs + off) as *mut u32, v) }
    }

    /// The spec allows 64-bit registers to be accessed as aligned 32-bit halves (NVMe 1.4 §3.1),
    /// which keeps the accessor surface one width on both ISAs.
    fn reg64(&self, off: u64) -> u64 {
        self.reg32(off) as u64 | (self.reg32(off + 4) as u64) << 32
    }

    fn wr64(&self, off: u64, v: u64) {
        self.wr32(off, v as u32);
        self.wr32(off + 4, (v >> 32) as u32);
    }
}

/// **Where the controller's registers are, once it is up.** The physical base of BAR0, so a
/// spawner can map the **doorbell page** (`bar0 + 0x1000`) and only that page into an EL0 data
/// plane. Offsets 0x000..0x1000 hold `CC`, `CSTS`, `AQA`, `ASQ` and `ACQ`: a process that cannot
/// name that page cannot reset the controller and cannot move the admin rings.
pub struct Found {
    /// BAR0's physical base.
    pub bar0: u64,
    /// The controller's PCIe requester id, which is what an IOMMU unit owns or does not.
    pub rid: u32,
    /// The initialized admin plane.
    pub controller: NonVolatileMemoryExpress,
}

/// Why [`bring_up`] produced no controller. **Two different facts**, and until milestone 261's
/// bench rehearsal they shared one `None`: a machine with no NVMe, and a machine whose NVMe this
/// driver refused. The boot test turned both into the same "skipped" line, and xenon's second
/// attempt on 2026-09-17 printed exactly that for a disk that was there
/// (`bench/xenon-2026-09-17/nvme-attempt-2-no-controller-found.log`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Absent {
    /// No function on the bus carries the NVMe class code.
    NoController,
    /// A controller is there and failed bring-up at the named phase.
    Refused { rid: u32, why: Error },
}

/// **Find, confine, and initialize the machine's NVMe disk.** [`Absent::NoController`] when no
/// controller is on the bus (every boot the runner did not attach one), which is a fact about the
/// machine; a controller that is present but fails bring-up prints the phase it died in and
/// returns [`Absent::Refused`], which is a fact about this driver and never a reason to skip.
///
/// The order is the confinement story: the device gets its DMA region *before* the controller is
/// enabled, so there is no instant at which an enabled controller could reach anything else. On a
/// machine with no IOMMU (a plain `virt` boot with the flag off) the confinement step is skipped
/// and the driver still runs, with nothing but the driver's own arithmetic bounding the addresses;
/// the test boots all have one, so the proven configuration is the confined one.
pub fn bring_up() -> Result<Found, Absent> {
    let dev = crate::pci::find_nvme_device().ok_or(Absent::NoController)?;
    // Zeroing is load-bearing for the completion rings: the phase discipline starts from
    // all-zero entries.
    let dma = crate::memory::alloc_contiguous_zeroed(DMA_PAGES as usize)
        .expect("no DMA region for the NVMe driver")
        .addr();
    if crate::iommu::is_active() {
        // The device's own DMA region, and in a shipped boot nothing else: the controller faults
        // on any address outside it.
        let own = paging::domain::DmaRegion {
            base: dma,
            size: DMA_PAGES * page_frames::FRAME_SIZE,
        };
        // **The confinement falsification's widening point** ([`ESCAPE_TARGET`]). `None` in the
        // shipped tree (the static is zero), so the confinement is tight. The replayable patch
        // stores a frame outside the region into `ESCAPE_TARGET`, and that frame then joins the
        // controller's domain: an EL0 server aiming a PRP at it succeeds, and the confinement
        // test's canary is overwritten. See milestone 202.
        #[cfg(feature = "system_tests")]
        let extra = {
            let escape = ESCAPE_TARGET.load(core::sync::atomic::Ordering::Relaxed);
            (escape != 0).then_some(paging::domain::DmaRegion {
                base: escape,
                size: page_frames::FRAME_SIZE,
            })
        };
        #[cfg(not(feature = "system_tests"))]
        let extra: Option<paging::domain::DmaRegion> = None;
        match extra {
            Some(x) => crate::iommu::confine(dev.rid, &[own, x]),
            None => crate::iommu::confine(dev.rid, &[own]),
        }
    }
    match NonVolatileMemoryExpress::new(mmu::phys_to_virt(dev.bar0), dma, mmu::phys_to_virt(dma)) {
        Ok(controller) => Ok(Found {
            bar0: dev.bar0,
            rid: dev.rid,
            controller,
        }),
        Err(e) => {
            crate::println!(
                "  non_volatile_memory_express: controller present but failed bring-up: {e:?}"
            );
            Err(Absent::Refused {
                rid: dev.rid,
                why: e,
            })
        }
    }
}
