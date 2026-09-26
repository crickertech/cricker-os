//! **The NVMe block server, at EL0** (milestone 261;
//! [DECISIONS §86](../../design/decisions/86-el0-nvme-driver.md), DECIDED 2026-09-03 as option 2a;
//! notes/non-volatile-memory-express.md).
//!
//! An unprivileged process that drives a real, non-virtio DMA device: it builds 64-byte commands,
//! rings a hardware doorbell, and watches a completion ring's phase tag, while holding no
//! authority to reset the controller, to move a queue, or to reach a byte of memory outside the
//! region an IOMMU has already confined the controller to. That combination is the microkernel
//! thesis for storage, and until this program existed the NVMe driver was kernel code.
//!
//! ```text
//!   NVMe controller ──doorbell page (1 page of BAR0)──► this program ──blk IPC──► clients
//!    (a PCIe function)                                     │
//!    confined by the IOMMU to ──────────────────────────── its data plane: two I/O rings
//!    the kernel's DMA region                                and a transfer buffer
//! ```
//!
//! # What it holds, and what it is denied
//!
//! **Held**, and this is the complete list, because a capability system has no ambient
//! environment:
//!
//! - slot 0, the **request** endpoint (RECV): clients `CALL` here, `filesystem_protocol::blk`'s
//!   wire format, the same contract the virtio block server speaks;
//! - slot 1, a **readiness** endpoint (WRITE): one message, once the first command has
//!   round-tripped;
//! - mapped at [`DATA_PLANE_VA`]: the **data plane's** pages of the DMA region, read/write: the
//!   I/O submission ring, the I/O completion ring, and [`TRANSFER_BLOCKS`] pages of buffer;
//! - mapped at [`DOORBELL_VA`]: **one page of BAR0**, device-typed, at `bar0 + 0x1000`.
//!
//! **Denied**, and every line of it is a decision rather than an omission:
//!
//! - **BAR0's controller register page.** `CC`, `CSTS`, `AQA`, `ASQ` and `ACQ` live at offsets
//!   0x000..0x1000 and this process is not mapped them. It cannot reset the controller, cannot
//!   disable it, and cannot repoint the admin rings. NVMe 1.4 §3.1 put a page boundary exactly
//!   where the authority boundary belongs, which is why this split costs no new syscall.
//! - **The admin plane's pages of the DMA region.** The two admin rings and the IDENTIFY buffer
//!   are in the same confined region and are not in this address space.
//! - **Any admin command at all.** Creating a queue is what names the physical address a ring
//!   lives at; this process never issues one, so where its rings live is the kernel's statement
//!   and not its own.
//! - **An `Irq` capability**, because this server polls. One authority fewer than
//!   `entropy.rs` holds, and see `BUGS`.
//! - **A `Virtio` capability**, because there is no kernel-mediated transport here: that is the
//!   whole reason §86 existed.
//! - **A `DeviceFrame` or `PageFrame` capability.** The two windows were installed by the spawner
//!   before `_start` ran, so this process holds no *name* for either and can neither delegate nor
//!   remap them. Milestone 159's TRNG driver made the same choice, and its header records why an
//!   earlier draft's capability slot was describing a thing nobody hands over.
//! - **The initrd, a budget, a filesystem, a network, a clock.** None of them is here.
//!
//! # What is proven, and where
//!
//! Every piece of arithmetic is in `crates/non_volatile_memory_express`, host-tested and Kani-reachable: the ring
//! indices and the phase discipline, the doorbell offsets, the PRP pair, the identify decode, and
//! (milestone 261) the spawn handoff's round trip and the block-range check
//! [`non_volatile_memory_express::Handoff::transfer_command`] refuses outside. What is left in this file is what cannot
//! be tested without a device: volatile reads and writes, two barriers, and a bounded poll.
//!
//! # BUGS
//!
//! **One command in flight.** Each request completes before the next is submitted, so the ring
//! depth buys nothing and any throughput measured against this server is a lower bound on the
//! device rather than a measurement of it. Inherited from the kernel-resident driver this
//! replaced; notes/non-volatile-memory-express.md carries it.
//!
//! **It polls rather than waiting on an interrupt**, so a request costs a spin instead of a
//! block, and a busy server is a busy core. The controller is created with `IEN=0` and names no
//! vector (`non_volatile_memory_express::Command::create_io_cq`), so there is no interrupt to grant and switching would
//! change the admin plane as well as this file. It also keeps `Object::Irq` off this program's
//! list, which is a smaller authority, and §86's own interrupt finding is the reason not to reach
//! for MSI-X casually: this machine runs `-device intel-iommu` with no `intremap=on` and
//! `gic-version=2` with no ITS, so nothing would confine an interrupt a userspace driver aimed.
//!
//! **A hung controller presents as a timeout, not as `CSTS.CFS`.** Reading the fatal-error bit
//! means reading the controller register page, which this process deliberately cannot. So
//! [`SPIN_BOUND`] polls with no completion becomes `EIO` and the reason is lost. That is the
//! direct price of the split, and it is a worse diagnostic than the kernel-resident driver gave.
//!
//! **A confined driver can still aim DMA inside its own confinement.** The IOMMU bounds the
//! controller to the whole allocation, admin rings included, because the controller fetches from
//! both halves; so a PRP computed backwards from [`Handoff::data_plane_phys`] could make the
//! controller overwrite the admin ring. Not an escape, and §86's option 2a says so in those
//! terms; option 4's doorbell validator is what would close it.
//!
//! **A transfer is one command per filesystem block**, even when a request carries
//! [`TRANSFER_BLOCKS`] contiguous ones. `non_volatile_memory_express::prp_pair` refuses anything needing a PRP *list*
//! (notes/non-volatile-memory-express.md: "One namespace, PRP-only, no SGLs, no PRP lists"), and a 16-page transfer needs
//! one. The virtio block server issues a single request for the same range, so this server is
//! slower on bulk by construction and the fix is a PRP list rather than anything about this file.
//!
//! Name: ratified 2026-09-17 (calef, DECISIONS §154) for the whole family rather than for this
//! program alone, with the program's own half ruled on 2026-09-18 and performed the same day. The
//! argument, the refusals and what did not move are recorded once, beside the crate, in
//! `crates/non_volatile_memory_express`. Refused `nvme_server`, `nvme_driver` and
//! `non_volatile_memory_express_server`.
//!
//! **This program takes the crate's own name, and calef ruled that on 2026-09-18 rather than a
//! lane inferring it.** The obvious spelling does not exist: `non_volatile_memory_express_server`
//! is **34 bytes** against `nifefs`'s 32-byte `NAME_LEN`, which bounds a program's name and not a
//! crate's, so the family ruling could not reach this program by itself. The options were to share
//! the crate's name (27 bytes), to raise `NAME_LEN` to 40, or to name the program something else
//! entirely.
//!
//! **Raising `NAME_LEN` was refused on its cost**, which is measured rather than asserted: it is a
//! format two programs agree on, which AGENTS.md's *move fast on what can be undone* puts in the
//! expensive category, and it would take the archive from 127 entries to 106 against 87 in use
//! today and about 93 after the tracked nine-role split. The naming rules already say not to let
//! the limit pick a name and not to spend a format change on bytes nothing needs; this is the
//! second half of that.
//!
//! **What sharing buys beyond fitting.** It is the pair AGENTS.md blesses and says *means*
//! something: the crate is this program's logic, lifted out to be host-tested and Kani-reachable,
//! while the program keeps the IO, exactly as `compositor` and `line_editor` do. And it settles
//! the `_server`-versus-`_driver` split this block used to raise, by removing the suffix rather
//! than choosing between them, which puts this program with `entropy`, `clock` and `input`: named
//! for the thing, not for the role.
//!
//! **The precedent, stated because §154 will produce more of these**: a long expansion reaches a
//! program by sharing its crate's name. `acpi` and `uart` expand to 42 and 43 bytes if they are
//! ever ruled, and neither could ever name a program on its own.
//!
//! `nvme_server` was §86's name for this program in passing, and §86 said plainly that was not a
//! ratification. Two things were wrong with it. It carries the unexpanded acronym, which is what
//! the ruling is about. And it was the tree's **only** `_server` against four `_driver` programs
//! (`block_driver`, `gpu_driver`, `keyboard_driver`, `serial_driver`), drawing a distinction no
//! reader can recover from the name: `block_driver` serves a contract too, and `crates/virtio`'s
//! own prose calls that program's role *"the block server"*.
//!
//! **That inconsistency outlives this ruling and is not settled here.** `block_driver` and this
//! program will spell the same role two ways until somebody rules on the `_driver`/`_server` split
//! itself, which is calef's and is a separate question from the acronym.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use abi::rendezvous;
use filesystem_protocol::blk;
use non_volatile_memory_express::{Command, Completion, CqState, Doorbell, Handoff, SqState};
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{exit, recv_cap, reply, send};

/// Capability slots, by convention with `kernel/src/user/non_volatile_memory_express_service.rs`.
const REQ: u64 = 0;
const READY: u64 = 1;

/// Where the spawner maps this server's data plane. **Must match
/// `kernel/src/user/non_volatile_memory_express_service.rs`'s `DATA_PLANE_VA`.**
const DATA_PLANE_VA: u64 = address_space_map::pair_page(0x0000_0000_0098_0000);

/// Where the spawner maps the doorbell page of BAR0, device-typed. **Must match
/// `kernel/src/user/non_volatile_memory_express_service.rs`'s `DOORBELL_VA`.**
const DOORBELL_VA: u64 = address_space_map::pair_page(0x0000_0000_009c_0000);

/// How many blocks one request may carry, and how many pages of transfer buffer the spawner
/// mapped: the blk contract's own number, so this server and the virtio one clamp identically.
const TRANSFER_BLOCKS: usize = blk::TRANSFER_BLOCKS;

/// The data plane's layout, in byte offsets from [`DATA_PLANE_VA`]. It is the spawner's layout,
/// stated again here because the two halves live in different address spaces and nothing but the
/// spawn contract joins them; `kernel/src/non_volatile_memory_express.rs`'s page constants are the other end.
const IO_SQ_OFF: u64 = 0;
const IO_CQ_OFF: u64 = PAGE;
const TRANSFER_OFF: u64 = 2 * PAGE;

/// The whole data-plane window: two rings and the buffer.
// SAFETY: the spawner maps exactly these pages read/write at DATA_PLANE_VA before `_start` runs
// (`kernel/src/user/non_volatile_memory_express_service.rs`), for the lifetime of this process.
const DATA: MappedWindow =
    unsafe { MappedWindow::new(DATA_PLANE_VA, TRANSFER_OFF + TRANSFER_BLOCKS as u64 * PAGE) };

/// The doorbell page. Device-typed, so every access is uncached and unreordered, which is what a
/// doorbell needs: a cached write is a command the controller never hears about.
// SAFETY: the spawner maps one device-typed page at DOORBELL_VA before `_start` runs.
const BELLS: MappedWindow = unsafe { MappedWindow::new(DOORBELL_VA, PAGE) };

/// The one I/O queue pair the kernel's admin plane created. Ringing any other queue's doorbell
/// would be ringing a queue that does not exist.
const IO_QID: u16 = 1;

/// The one namespace QEMU's `-device nvme,drive=` carries; the admin plane identified it.
const NSID: u32 = 1;

/// How many polls of the completion ring before this server declares the command lost. QEMU
/// completes synchronously inside the doorbell write, so this is paranoia there; on hardware it is
/// roughly a second of spinning. **Not a measured bound**, and it is the only thing standing
/// between a dead controller and a server that never answers, because `CSTS` is unreachable from
/// here. See this module's `BUGS`.
const SPIN_BOUND: u32 = 50_000_000;

/// The bring-up step words this server reports on its readiness endpoint, in the `0xDEAD_..`
/// shape `entropy_protocol::bringup_failure` established, so a boot transcript reads the same way
/// across services. A report of [`filesystem_protocol::fixture::READY`] means a command went to
/// the controller and came back.
const STEP_BAD_HANDOFF: u64 = 0xDEAD_0001;
const STEP_FIRST_READ: u64 = 0xDEAD_0002;

/// **Order our writes to the rings before the doorbell that announces them, and the phase-tag read
/// before the payload it gates.** The controller is another observer of this memory, and on a
/// weakly-ordered CPU nothing else stops a doorbell write hoisting above the command it announces.
///
/// **Local rather than shared, and now for a stated reason rather than for want of a name.**
/// Milestone 186 (derive the architecture list) lifted the five virtqueue copies into
/// [`user_mode_runtime::virtio::virtio_ring_barrier`], which is a weaker barrier: those drivers
/// notify through a syscall instruction, so `x86_64` needs only a compiler fence and aarch64 only
/// `dmb ish`. This one's doorbell is a direct store to a device-typed page in this address space,
/// with no syscall between the ring stores and it, so it keeps `dmb sy` and a real `mfence`.
/// Sharing the two would mean strengthening the ring barrier or weakening this one, and both are
/// ordering changes rather than deduplication.
fn barrier() {
    // SAFETY: a barrier has no operands and cannot be unsound; it only constrains ordering.
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("dmb sy", options(nostack, nomem, preserves_flags));
    }
    // SAFETY: as above.
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence", options(nostack, preserves_flags));
    }
    // SAFETY: as above.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mfence", options(nostack, nomem, preserves_flags));
    }
    // A fourth architecture fails to build rather than silently ordering nothing. This function
    // already had all three arms; the arm below is what stops it becoming the five that did not.
    #[cfg(not(any(
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86_64"
    )))]
    compile_error!(
        "barrier(): this architecture has no ordering named here. A ring publish must be visible \
         before the doorbell that announces it; name the instruction that does that."
    );
}

/// The server's whole mutable state: the two ring cursors and a command id.
struct Plane {
    handoff: Handoff,
    sq: SqState,
    cq: CqState,
    cid: u16,
    /// How many device flushes have completed, answered back on every successful `blk::FLUSH`.
    /// A strictly increasing count is what makes that verb's success falsifiable; see the
    /// contract's own argument at `filesystem_protocol::blk::FLUSH`.
    flushes: i64,
}

impl Plane {
    fn next_cid(&mut self) -> u16 {
        self.cid = self.cid.wrapping_add(1);
        self.cid
    }

    /// **Submit one command on the I/O queue and poll its completion.** The copy into the
    /// submission ring, the publish barrier, the tail doorbell, the phase-gated poll, the head
    /// doorbell. Every one of those is a volatile access through a window the spawner installed;
    /// every index and offset in it came out of `crates/non_volatile_memory_express`.
    ///
    /// `Err(status)` carries the controller's status field, or [`u16::MAX`] for a completion that
    /// never arrived, which this server cannot distinguish from a fatal controller error because
    /// `CSTS` is not in its address space.
    fn transact(&mut self, cmd: Command) -> Result<(), u16> {
        let expect_cid = (cmd.0[0] >> 16) as u16;

        let slot = self.sq.push();
        DATA.write(IO_SQ_OFF + slot as u64 * 64, cmd.0);
        // Publish the command before the doorbell: the controller is another observer.
        barrier();
        BELLS.w32(
            non_volatile_memory_express::doorbell(
                IO_QID,
                Doorbell::SubmissionTail,
                self.handoff.dstrd,
            ) - PAGE,
            self.sq.tail() as u32,
        );

        // Poll the completion ring at the head slot until the phase tag says the entry is this
        // lap's. That is the whole completion mechanism: no shared index, no memory the controller
        // and this process both write.
        let cqe = IO_CQ_OFF + self.cq.head() as u64 * 16;
        let mut done: Option<Completion> = None;
        for _ in 0..SPIN_BOUND {
            let c = Completion::from_dwords(DATA.read::<[u32; 4]>(cqe));
            if self.cq.is_owned(&c) {
                done = Some(c);
                break;
            }
            core::hint::spin_loop();
        }
        let c = done.ok_or(u16::MAX)?;
        // Order the phase read before the payload reads that follow (the block's bytes).
        barrier();

        let new_head = self.cq.pop();
        BELLS.w32(
            non_volatile_memory_express::doorbell(
                IO_QID,
                Doorbell::CompletionHead,
                self.handoff.dstrd,
            ) - PAGE,
            new_head as u32,
        );
        self.sq.note_head(c.sq_head);

        // The one-at-a-time discipline makes any other cid a protocol violation. Answering EIO
        // rather than panicking, because this is a server and a client is waiting on it: a
        // process that traps here takes the disk down for everyone holding the endpoint.
        if c.cid != expect_cid {
            return Err(u16::MAX);
        }
        if c.status != 0 {
            return Err(c.status);
        }
        Ok(())
    }

    /// One whole-filesystem-block transfer into or out of buffer page `slot_index`.
    fn transfer(&mut self, block: u64, slot_index: usize, write: bool) -> Result<(), u16> {
        let data_phys = self.handoff.data_plane_phys + TRANSFER_OFF + slot_index as u64 * PAGE;
        let cid = self.next_cid();
        let cmd = self
            .handoff
            .transfer_command(cid, NSID, block, PAGE, data_phys, PAGE, write)
            // `None` means the block is outside the namespace or the arithmetic would not close;
            // either way it is the caller's request that is wrong, and the crate refused to build
            // a command rather than let the controller answer for it.
            .ok_or(u16::MAX)?;
        self.transact(cmd)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(arg0: u64, arg1: u64, arg2: u64) -> ! {
    // The three words are the whole of what this process was told; a word that cannot describe a
    // controller it could serve is a wiring bug, reported rather than acted on.
    let Some(handoff) = Handoff::unpack([arg0, arg1, arg2]) else {
        send(READY, STEP_BAD_HANDOFF, arg0, 0);
        exit();
    };
    let mut plane = Plane {
        handoff,
        sq: SqState::new(handoff.entries),
        cq: CqState::new(handoff.entries),
        cid: 0,
        flushes: 0,
    };

    // **Read one block before reporting ready**, the discipline `entropy.rs` uses: "the service is
    // up" should mean "a client that asks will be answered", not that the spawn completed. It is
    // also the only end-to-end check available here, since this process cannot read `CSTS` to ask
    // the controller how it is.
    let first = plane.transfer(0, 0, false);
    let report = if first.is_ok() {
        filesystem_protocol::fixture::READY
    } else {
        STEP_FIRST_READ
    };
    // Word 1 is the namespace size this process was handed, so a boot transcript can see the
    // geometry reached ring 3 intact; word 2 is the status of that first read, which is the one
    // number that says whether the controller answered and what it said if it refused.
    send(
        READY,
        report,
        handoff.size_bytes,
        first.err().unwrap_or(0) as u64,
    );

    serve(plane)
}

/// The serve loop: one endpoint, one wait point, forever. `filesystem_protocol::blk`, the same
/// four verbs the virtio block server answers, so the FS server and the disk tools cannot tell
/// which kind of disk is underneath.
fn serve(mut plane: Plane) -> ! {
    loop {
        let (w0, reply_cap, block) = recv_cap(REQ);
        if reply_cap == rendezvous::NO_CAP {
            // A plain SEND on a CALL-only contract: nothing to answer.
            continue;
        }
        // **Clamp**, the defence the virtio block server states at the same line: every caller
        // today sends at most `TRANSFER_BLOCKS` because the field cannot encode more, and a
        // request is never trusted to stay inside the region it shares just because the packing
        // allows a larger number to be spelled.
        let count = blk::req_blocks(w0).min(TRANSFER_BLOCKS);
        let r0: i64 = match filesystem_protocol::op(w0) {
            blk::READ => transfer_range(&mut plane, block, count, false),
            blk::WRITE => transfer_range(&mut plane, block, count, true),
            blk::SIZE => plane.handoff.size_bytes as i64,
            blk::FLUSH => {
                let cid = plane.next_cid();
                match plane.transact(Command::flush(cid, NSID)) {
                    // Unlike the virtio server, this one never answers EOPNOTSUPP: NVMe 1.4 §6.8
                    // makes Flush mandatory, so a controller that answered a command at all has
                    // one. The count discipline is the contract's, and it is what makes a
                    // success falsifiable: two syncs returning the same number would mean the
                    // second never reached the device.
                    Ok(()) => {
                        plane.flushes += 1;
                        plane.flushes
                    }
                    Err(_) => filesystem_protocol::reply_err(5), // EIO
                }
            }
            _ => filesystem_protocol::reply_err(22), // EINVAL: an opcode this server has no verb for
        };
        reply(reply_cap, r0 as u64, 0);
    }
}

/// `count` contiguous blocks, one command each. See this module's `BUGS` for why it is not one
/// command: `non_volatile_memory_express::prp_pair` refuses a transfer needing a PRP list, and a multi-page one does.
fn transfer_range(plane: &mut Plane, block: u64, count: usize, write: bool) -> i64 {
    for i in 0..count {
        if plane
            .transfer(block.saturating_add(i as u64), i, write)
            .is_err()
        {
            return filesystem_protocol::reply_err(5); // EIO
        }
    }
    0
}

user_mode_runtime::panic_handler!();
