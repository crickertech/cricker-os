use super::*;
use crate::cap::{Rights, memory_region_cap, page_frame_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// **The `std` demo's binary, or `None` because this target has no `std` for it** (milestone 161).
///
/// `std_exerciser` is not built with the rest of `user/`: it is its own workspace, compiled with
/// `-Zbuild-std` for a **custom target** (`aarch64-unknown-nife`, `riscv64-unknown-nife`,
/// `x86_64-unknown-nife` since milestone 184) against the patched `std` in the `nife-dev`
/// toolchain. It is absent from an archive packed by something that never built it (a bare
/// `cargo xtask initrd-x86`, an interactive `run`), and every std test skips rather than fails.
///
/// A named accessor rather than an `.expect` at nine call sites, for [`super::fs_service::
/// fs_server_image`]'s reason: a fixture missing because of the *toolchain* rather than because of
/// the *machine* deserves to say so once, where a reader will find it.
pub fn std_exerciser_image() -> Option<&'static [u8]> {
    program("std_exerciser")
}

/// The reason a test gives when [`std_exerciser_image`] is `None`.
pub const NO_STD_EXERCISER: &str = "no std_exerciser in this archive: it is built with -Zbuild-std \
                                    for a custom target against the nife-dev toolchain, and \
                                    whatever packed this archive did not build it first (`cargo \
                                    xtask std-exerciser`, which `script/test` runs)";

/// Where the loader maps the clock page for a std program, and the slot it grants the page in.
/// From `std_runtime_protocol`, which the std PAL's `rt` re-exports, so they cannot drift.
const CLOCK_PAGE_STD: u64 = std_runtime_protocol::CLOCK_PAGE;
const CLOCK_SLOT: u64 = std_runtime_protocol::CLOCK_SLOT;

/// Where the loader maps the inert-configuration page for a std program (milestone 47's
/// environment-variable fork, DECISIONS §111 (inert configuration is a read-only page)), and its
/// slot, from `std_runtime_protocol`. Clear of the clock page above and of the FS contract's shared
/// page (`fs_service::FS_CLIENT_PAGE_VA`, `0x0060_0000`, which this spawn does not use).
const CONFIG_PAGE_STD: u64 = std_runtime_protocol::CONFIG_PAGE;
const CONFIG_SLOT: u64 = std_runtime_protocol::CONFIG_SLOT;

/// The entropy service's request endpoint (milestone 56 (secrets, credentials, and the entropy to
/// make them safe)), at `std_runtime_protocol`'s slot. **An endpoint, and no mapping**: unlike the
/// clock, whose read authority IS a page, randomness is obtained by asking, so the whole grant is
/// one endpoint that names no device.
const ENTROPY_SLOT: u64 = std_runtime_protocol::ENTROPY_SLOT;

/// **Which entropy backend a std program's `SystemRng` is served from.** The program cannot tell:
/// its slot 6 names an endpoint either way (DECISIONS §44), which is why this is a spawner's choice
/// and not the PAL's.
///
/// `x86_64` takes `RDSEED` (milestone 162's instruction backend) because its runner attaches no
/// virtio-rng at all (`helpers/qemu-runner-x86_64.sh`: "no NIC, no GPU, no RNG") and the suite's
/// `-cpu max` implements the instruction. The other two keep the virtio-mmio device their legs have
/// always attached. Milestone 184 made this a per-architecture constant; before it, `x86_64` never
/// reached this function because it had no std program to spawn.
#[cfg(target_arch = "x86_64")]
const STD_ENTROPY_BUS: entropy_service::Bus = entropy_service::Bus::Instruction;
#[cfg(not(target_arch = "x86_64"))]
const STD_ENTROPY_BUS: entropy_service::Bus = entropy_service::Bus::Mmio;

/// The heap high-water for the demo's Vec/String/HashMap workout plus std's own runtime
/// allocations and the heap's page tables is well under 1 MiB; 256 pages is comfortable, and
/// the initial region only needs to be contiguous at spawn, when memory is unfragmented.
pub const BUDGET_PAGES: u64 = 256;

/// std's startup, formatting machinery, and collection code use far more stack than a
/// hand-written `no_std` worker. `load` maps one stack page; map `std_runtime_protocol::STACK_PAGES`
/// (32) more below it, generous so a stack-depth surprise is not what a first std bring-up debugs.
/// The progenitor maps exactly that many and not the one extra, because its loader has no `load`
/// page of its own to add.
const EXTRA_STACK_PAGES: u64 = std_runtime_protocol::STACK_PAGES;

/// Wire a std program and hand back the endpoint it prints on **and the thread it runs as**.
///
/// The tid is what lets a caller wait for the program to be *gone* rather than merely quiet
/// (milestone 64). The transcript ends at `cleanup`, which runs before the process leaves, so a
/// test that stops at the last byte is still racing the exit it wants to make a claim about.
pub fn start(
    image: &'static [u8],
    clock_image: &'static [u8],
    entropy_image: &'static [u8],
) -> (RendezvousId, crate::thread::ThreadId) {
    let spawned = start_reclaimable(image, clock_image, entropy_image);
    (spawned.report, spawned.thread)
}

/// What [`start_reclaimable`] hands back. [`StdSpawn`] is `fs_service`'s name for the same idea and
/// this is deliberately its twin: the stdout endpoint, the thread, and **the untyped region the
/// heap was drawn from**, so a caller that knows the program has exited can give the pages back.
pub struct StdRun {
    pub report: RendezvousId,
    pub thread: crate::thread::ThreadId,
    pub heap: u64,
}

/// The same spawn as [`start`], **also handing back the heap region**, for
/// [`super::fs_service::start_std_full`]'s reason exactly, at milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets).
///
/// `std_exerciser` is in every archive, so its 256 pages are a charge this suite's frame ledger has
/// always carried and [`start`] does not bother. A program present only when somebody ran a build
/// script is different: leaving its pages spoken for makes the ledger fail **for exactly the person
/// running the experiment** and pass for everyone else, which is the worst shape a gate can have.
/// `cryptography_exerciser` is that kind of program, and the 196 frames it kept are what forced
/// this function to exist.
pub fn start_reclaimable(
    image: &'static [u8],
    clock_image: &'static [u8],
    entropy_image: &'static [u8],
) -> StdRun {
    let report = crate::sched::create_rendezvous();
    let (heap, thread) = start_on_full(image, clock_image, entropy_image, report);
    StdRun {
        report,
        thread,
        heap,
    }
}

/// The same spawn, with **the output sink chosen by the caller** (milestone 50).
///
/// This split is the milestone's finding expressed as a function signature: everything about a
/// std program's wiring is fixed except one endpoint capability, and putting a different one in
/// slot 1 is the whole of redirection. The program is not told, cannot ask, and the two callers
/// of this function hand it an endpoint the kernel receives on and an endpoint a file sink
/// receives on. See `sink_tests`.
pub fn start_on(
    image: &'static [u8],
    clock_image: &'static [u8],
    entropy_image: &'static [u8],
    report: RendezvousId,
) -> crate::thread::ThreadId {
    start_on_full(image, clock_image, entropy_image, report).1
}

/// [`start_on`], returning the heap region beside the thread. See [`start_reclaimable`] for why a
/// caller would want it; everything else about this function is `start_on`'s documentation.
pub fn start_on_full(
    image: &'static [u8],
    clock_image: &'static [u8],
    entropy_image: &'static [u8],
    report: RendezvousId,
) -> (u64, crate::thread::ThreadId) {
    let budget = crate::memory_region::create(BUDGET_PAGES).expect("no untyped for std_exerciser");

    // The entropy service, wired once per boot and shared with the milestone-56 tests. Its
    // request endpoint is the whole of a std program's randomness authority: `SystemRng` is a
    // `CALL` on it, and nothing about it reaches the device (DECISIONS §44).
    let entropy = entropy_service::ensure(entropy_image, STD_ENTROPY_BUS)
        .expect("no entropy source for the std program (a virtio-rng device on aarch64/riscv64, is NIFE_RNG set on this leg? RDSEED on x86_64)");
    if let Some(ready) = entropy.ready {
        let report = crate::sched::ipc_recv(ready);
        assert_eq!(
            report[0],
            entropy_protocol::READY,
            "the entropy service did not come up for the std program (it reported {:#x})",
            report[0],
        );
    }

    // The clock first, and its startup report taken before the program starts, so the offset is
    // published by the time std reads the page. Waiting is not a synchronisation trick, it is
    // the honest order: a std program that started first would see `state::UNKNOWN` and be
    // correct to say so.
    let clock = clock_service::start(clock_image);
    let _ = crate::sched::ipc_recv(clock.report);

    // **The inert-configuration page** (milestone 47's environment-variable fork, DECISIONS
    // §111). Unlike the clock, nothing here runs a service: the page is assembled once, into a
    // frame nothing else can see, and only mapped read-only afterward, so there is no seqlock
    // and no readiness handshake to wait on (see `environment_protocol`'s own docs for why). The values are
    // the conservative universal defaults ("no clock service configured this program's locale
    // or terminal, so tell it the least assuming thing"), the same posture `date` takes when no
    // clock service is running: an honest baseline rather than a guess. There is no shell here
    // yet to hold a *different* default and pass it explicitly (the "inheritance with
    // visibility" shape the roadmap names); that arrives with whatever program first declares it
    // wants this page through `grant_plan::Manifest`, which none does today.
    let config_bytes = environment_protocol::PageBuilder::new()
        .tz("UTC")
        .expect("UTC is not a recognized environment_protocol::domain::KNOWN_TZ member")
        .lang("C")
        .expect("C is not a recognized environment_protocol::domain::KNOWN_LANG member")
        .term("dumb")
        .expect("dumb is not a recognized environment_protocol::domain::KNOWN_TERM member")
        .build();
    // Zeroed, so nothing left behind by a previous occupant of this physical page is visible
    // through the reserved tail past `PAGE_BYTES` (`ConfigPage` only ever reads the first
    // `PAGE_BYTES`, but a frame's contents are otherwise unspecified until written).
    let config_phys = crate::memory::alloc_zeroed()
        .expect("no frame for the std program's config page")
        .addr();
    // SAFETY: `config_phys` is that fresh frame, direct-mapped and owned by nobody else, and
    // `config_bytes` is `PAGE_BYTES` long, far under `FRAME_SIZE`.
    unsafe {
        core::ptr::copy_nonoverlapping(
            config_bytes.as_ptr(),
            mmu::phys_to_virt(config_phys) as *mut u8,
            config_bytes.len(),
        );
    }

    // The clock page and the config page read-only, then the deep stack std needs.
    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; 2 + EXTRA_STACK_PAGES as usize];
    maps[0] = Mapping {
        va: CLOCK_PAGE_STD,
        phys: clock.page_phys,
        flags: Flags::user_rodata(), // a READER, and the mapping is what says so
    };
    maps[1] = Mapping {
        va: CONFIG_PAGE_STD,
        phys: config_phys,
        flags: Flags::user_rodata(), // same shape as the clock page: a READER, never a writer
    };
    for (k, m) in maps[2..].iter_mut().enumerate() {
        // Zeroed so the new process starts clean.
        let phys = crate::memory::alloc_zeroed()
            .expect("no frame for std_exerciser stack")
            .addr();
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
        m.phys = phys;
    }

    let tid = crate::sched::spawn(move || {
        // The clock, config and entropy capabilities go in at their named slots BEFORE `run`
        // grants in order, so `run`'s two grants land at 0 and 1 and slots 2 to 4 stay empty.
        // The clock and config pages are `READ` only: the whole point of each is that a reader
        // cannot write it. See `grant_at`.
        crate::sched::grant_at(CLOCK_SLOT, page_frame_cap(clock.page_phys, Rights::READ))
            .expect("the std clock slot was already occupied");
        crate::sched::grant_at(CONFIG_SLOT, page_frame_cap(config_phys, Rights::READ))
            .expect("the std config slot was already occupied");
        crate::sched::grant_at(ENTROPY_SLOT, rendezvous_cap(entropy.request, Rights::WRITE))
            .expect("the std entropy slot was already occupied");
        run(
            image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    memory_region_cap(budget),             // slot 0: the heap's budget
                    rendezvous_cap(report, Rights::WRITE), // slot 1: stdout/stderr
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn std_exerciser");
    (budget, tid)
}
