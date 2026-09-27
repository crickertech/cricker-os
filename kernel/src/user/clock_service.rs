use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::RendezvousId;

/// Where the service expects its two mappings. Must match components/src/clock.rs.
///
/// `CLOCK_VA` is public because it is the address the **set** authority lives at, and milestone
/// 51's NTP tests aim a write there from a process that holds no such mapping. An attack on an
/// address nobody uses would prove nothing.
pub const CLOCK_VA: u64 = address_space_map::pair_page(0x00c0_0000);
const RTC_VA: u64 = address_space_map::pair_page(0x00d0_0000);

/// What the clock service was wired with, so a test (or a real progenitor) can play its clients.
pub struct Wiring {
    /// The service's startup report.
    pub report: RendezvousId,
    /// The propose endpoint. A holder of `WRITE` here may ask; it may not tell.
    pub propose: RendezvousId,
    /// The clock page's **physical** frame, so a reader can be given a read-only mapping of it
    /// (and so the kernel's own tests can read it through the direct map).
    pub page_phys: u64,
    /// Which RTC the machine turned out to have, one of `clock_protocol::rtc`.
    pub kind: u64,
}

/// **Does this machine have an RTC at all?** (milestone 161; `x86_64`'s CMOS answer is milestone
/// 176/DECISIONS §130.)
///
/// `memory::rtc_region()` is the device tree's answer, and both QEMU `virt` boards give one (a
/// PL031 on aarch64, a Goldfish on riscv64). `x86_64` never populates it (its RTC is the CMOS at I/O
/// ports `0x70`/`0x71`, which has no page for a device capability to be a mapping of, and
/// DECISIONS §121 keeps that port range kernel-resident rather than granted), so this asks the
/// kernel's own CMOS reader instead of the device tree there: a `None` means the machine gave a
/// reading this kernel could not make into a real calendar date, not that nobody looked.
///
/// Here rather than in a test module because four of them ask the same question, and it is asked
/// **before** [`start`] rather than of the [`Wiring::kind`] it returns: a test that skipped after
/// spawning the service would leave its frames charged to a run that did not happen.
pub fn machine_has_no_rtc() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        crate::arch::rtc::read_unix_nanos().is_none()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        crate::memory::rtc_region().is_none()
    }
}

/// The reason every RTC-dependent test gives when it skips. One string, because four files share
/// one cause and a reader comparing two runs should not have to decide whether two wordings mean
/// the same thing.
pub const NO_RTC: &str = "this machine has no working real-time clock (no RTC binding in its \
                          device tree, or on x86_64 a CMOS reading this kernel could not make \
                          into a real calendar date)";

/// Wire and spawn the clock service.
pub fn start(image: &'static [u8]) -> Wiring {
    // Zeroed, which is also the honest starting state: a page nobody has published to reads as
    // `state::UNKNOWN` rather than as 1970 (clock_protocol's `a_zeroed_page_reads_as_unknown`).
    let page_phys = crate::memory::alloc_zeroed()
        .expect("no frame for the clock page")
        .addr();

    let report = crate::sched::create_rendezvous();
    let propose = crate::sched::create_rendezvous();

    let rtc = crate::memory::rtc_region();

    // `kind`/`seed`: which RTC (if any) the machine has, and, on x86_64 only, the wall clock the
    // kernel already read from it. Everywhere else the driver in `components/src/clock.rs` maps and
    // polls its own register, so `seed` there is unused and stays 0. On x86_64 there is no
    // register to map (`rtc_region()` is always `None`: DECISIONS §121 keeps CMOS's ports
    // kernel-resident, never a capability), so the *kernel* reads it here, once, and hands the
    // answer across as data instead (DECISIONS §130, option 3; milestone 176's piece 2).
    #[cfg(target_arch = "x86_64")]
    let (kind, seed) = match crate::arch::rtc::read_unix_nanos() {
        Some(nanos) => (clock_protocol::rtc::CMOS, nanos),
        None => (clock_protocol::rtc::NONE, 0u64),
    };
    #[cfg(not(target_arch = "x86_64"))]
    let (kind, seed) = (rtc.map_or(clock_protocol::rtc::NONE, |(_, _, k)| k), 0u64);

    // Two mappings, or one on a machine with no RTC. The device page is mapped and no
    // `DeviceFrame` capability is granted, exactly as the console server's UART is: the service
    // drives the registers and never delegates them onward, and §41's take-back revocation
    // reaches an address-space mapping whether or not a capability table slot names it too.
    let mut maps = [
        Mapping {
            va: CLOCK_VA,
            phys: page_phys,
            flags: Flags::user_data(), // read/WRITE: the service is a setter
        },
        Mapping {
            va: RTC_VA,
            phys: 0,
            flags: Flags::user_device(),
        },
    ];
    let n_maps = match rtc {
        Some((phys, _, _)) => {
            maps[1].phys = phys;
            2
        }
        None => 1,
    };

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: kind, // which register layout the MACHINE has, not which ISA we are
                arg1: seed, // x86_64/CMOS only: the wall clock the kernel already read; 0 elsewhere
                arg2: 0,
                grants: &[
                    rendezvous_cap(propose, Rights::READ), // slot 0: serve proposals
                    rendezvous_cap(report, Rights::WRITE), // slot 1: the startup verdict
                ],
                maps: &maps[..n_maps],
            },
        )
    })
    .expect("could not spawn the clock service");

    Wiring {
        report,
        propose,
        page_phys,
        kind,
    }
}

impl Wiring {
    /// The clock page as the kernel sees it, through the direct map. A reader process would
    /// hold a read-only mapping instead; the seqlock and the layout are the same either way,
    /// because they come from the one contract crate.
    pub fn page(&self) -> clock_protocol::ClockPage {
        // SAFETY: a frame this module allocated and still owns, named through the direct map.
        unsafe { clock_protocol::ClockPage::new(mmu::phys_to_virt(self.page_phys)) }
    }

    /// Wall-clock nanoseconds as a reader would compute them: the page's offset plus the
    /// ambient monotonic counter. 0 when the machine does not know.
    pub fn wall_nanos(&self) -> u64 {
        let r = self.page().read();
        if clock_protocol::state::is_known(r.state) {
            clock_protocol::wall_nanos(r.offset_nanos, monotonic_nanos())
        } else {
            0
        }
    }

    /// Play a proposer: `CALL` the propose endpoint. Returns `(status, wall_nanos_after)`.
    pub fn propose_nanos(&self, unix_nanos: u64) -> (u64, u64) {
        let r = crate::sched::ipc_call(
            self.propose,
            [
                clock_protocol::propose::req(clock_protocol::propose::PROPOSE),
                unix_nanos,
            ],
        );
        (r[0], r[1])
    }
}

/// Monotonic nanoseconds since boot, the kernel's copy of the reader's arithmetic. Split into
/// seconds and a remainder for the same reason the service's version is: the naive
/// `ticks * 1_000_000_000` overflows a `u64` a few minutes into a boot.
pub fn monotonic_nanos() -> u64 {
    let freq = crate::arch::timer::frequency();
    let ticks = crate::arch::timer::now();
    let secs = ticks / freq;
    let rem = ticks % freq;
    secs * clock_protocol::NANOS_PER_SEC + rem * clock_protocol::NANOS_PER_SEC / freq
}
