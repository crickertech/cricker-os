use entropy_service::Bus;

use super::*;

/// Reach the service on `bus`, wiring it if this is the first test to ask, and check its
/// startup report when this call is the one that wired it.
///
/// Returns `None` if the bus has no virtio-rng device (milestone 145: correct on a bare board
/// boot, which has no equivalent of the runner's `NIFE_RNG`-gated `-device virtio-rng` line);
/// callers must `skip!()` at their own call site rather than have this helper do it, because
/// `skip!()` returns from its immediate caller and a helper is not the test.
fn start(bus: Bus) -> Option<entropy_service::Wiring> {
    let image = program("entropy").expect("no entropy program in the initrd archive");
    let w = entropy_service::ensure(image, bus)?;
    if let Some(report) = w.wait_for_ready() {
        assert_eq!(
            report[0],
            entropy_protocol::READY,
            "the entropy service did not come up on {bus:?} (it reported {:#x}; a 0xDEAD_.. \
             word's low byte names the step, see components/src/entropy.rs)",
            report[0],
        );
        assert_eq!(
            report[1], 1,
            "the entropy service came up on {bus:?} but the device gave it no bytes at all",
        );
    }
    Some(w)
}

/// Draw `WORDS` eight-byte words through the request endpoint, asserting every draw is full.
/// Deliberately more than one bufferful (the service fetches 256 bytes per device request), so
/// this crosses the refill boundary and a cursor that wrapped instead of refilling shows up as
/// a repeat below.
const WORDS: usize = 64;

fn draw(w: &entropy_service::Wiring) -> [u64; WORDS] {
    let mut words = [0u64; WORDS];
    for (i, slot) in words.iter_mut().enumerate() {
        let mut buf = [0u8; 8];
        let n = w.get(8, &mut buf);
        assert_eq!(
            n, 8,
            "draw {i} of {WORDS} on {:?} returned {n} bytes, not 8: the device ran dry, or the \
             service failed to refill",
            w.bus,
        );
        *slot = u64::from_le_bytes(buf);
    }
    words
}

/// Every word distinct, and none of them zero. With a real source a collision among 64 draws is
/// a 2^-58 event, so a failure here is a bug rather than bad luck: a stuck device, a buffer
/// served twice, or a used ring the driver never re-read all present exactly this way.
fn assert_unpredictable(words: &[u64; WORDS], what: &str) {
    for (i, &a) in words.iter().enumerate() {
        assert_ne!(
            a, 0,
            "{what}: draw {i} is all zeros, which is the DMA page unwritten"
        );
        for (j, &b) in words.iter().enumerate().skip(i + 1) {
            assert_ne!(a, b, "{what}: draws {i} and {j} are identical ({a:#018x})");
        }
    }
}

/// **The headline, over virtio-mmio.** A client that holds one endpoint and no device gets
/// bytes off a real random-number generator, 512 of them, across a refill, all different.
#[test_case]
fn a_client_obtains_unpredictable_bytes_from_a_virtio_rng_over_mmio() {
    let Some(w) = start(Bus::Mmio) else {
        crate::testing::skip!("no virtio-rng device on the mmio bus (NIFE_RNG not set?)");
    };
    let words = draw(&w);
    assert_unpredictable(&words, "mmio");
}

/// **The same service, the same binary, over PCIe** (DECISIONS §18), and behind the IOMMU while
/// it is there. An entropy source's buffer is the one page in memory whose contents must not be
/// guessable, so an unconfined device writing it is worth asserting against rather than hoping.
#[test_case]
fn a_client_obtains_unpredictable_bytes_from_a_virtio_rng_over_pcie() {
    let Some(w) = start(Bus::Pci) else {
        crate::testing::skip!("no virtio-rng device on the pcie bus (NIFE_RNG not set?)");
    };
    assert!(
        w.confined_by_iommu,
        "the PCIe RNG is present but not behind the IOMMU: the buffer the device writes the \
         system's key material into is unconfined (is iommu_platform=on missing from the \
         runner's virtio-rng-pci line?)",
    );
    let words = draw(&w);
    assert_unpredictable(&words, "pcie");
}

/// **The JH7110 backend refuses to wire on a machine that has no JH7110** (milestone 159), which
/// is every machine this suite boots. That is worth a test rather than a comment because the
/// failure it guards is silent: a wiring that spawned the driver anyway would hand a userspace
/// process a device mapping of an address nobody described, and the first symptom would be a load
/// fault (or worse, a read of some unrelated peripheral) on the one machine nobody can run this
/// suite on.
///
/// On a `StarFive` VisionFive 2 (radon) this test skips and the boot tour's `hw entropy` line is
/// what speaks instead; see `design/roadmap/159-jh7110-trng-driver.md` for the bench procedure.
#[test_case]
fn the_jh7110_backend_refuses_to_wire_where_there_is_no_jh7110() {
    if entropy_service::jh7110_trng_device().is_some() {
        crate::testing::skip!(
            "this machine's tree describes a JH7110 TRNG (under either spelling): the boot \
             tour's hw entropy line is the test that matters here, not this one"
        );
    }
    let Some(image) = program("jh7110_entropy") else {
        crate::testing::skip!(
            "no jh7110_entropy program in this archive (every archive packs one since milestone 150)"
        );
    };
    assert!(
        entropy_service::ensure(image, Bus::Jh7110).is_none(),
        "the JH7110 entropy backend wired itself on a machine whose device tree describes no \
         such device: something spawned a driver holding a device mapping of an address nobody \
         named",
    );
}

/// **No clock-and-reset window is mapped on a machine that is not a JH7110** (milestone 220).
///
/// The guard on the one genuinely dangerous thing this milestone added: `jh7110_clock_and_reset::discover`
/// deliberately never fails to produce an address, falling back to the constant both published
/// device trees agree on, so a caller that took its answer unconditionally would store to
/// `0x1023_0000` on every board this kernel boots. `memory::init` is what stops that, by
/// recording the window only for a machine whose tree names a JH7110, and this pins it.
///
/// **A comment could not do this job**, because the failure is silent in the direction that
/// matters: on QEMU's `virt` board `0x1023_0000` is unmapped, so the symptom would be a load or
/// store fault during boot on a machine nobody would think to blame the clock driver for.
///
/// This is also the honest boundary of what an emulator can gate here. QEMU's `virt` machine has
/// no clock or reset controller of any kind, so what CI exercises is the *absence* path; the
/// sequence itself has never run against silicon. See `notes/jh7110-clock-and-reset.md`.
#[test_case]
fn no_clock_window_is_mapped_where_there_is_no_jh7110() {
    if entropy_service::jh7110_trng_device().is_some() {
        crate::testing::skip!(
            "this machine's tree describes a JH7110 TRNG, so a mapped clock window is correct \
             here; the boot tour's hw clock line is what speaks on that machine"
        );
    }
    assert!(
        entropy_service::jh7110_crg_window().is_none(),
        "a JH7110 clock-and-reset window was recorded on a machine whose device tree names no \
         JH7110: the constant fallback escaped its guard, and the next boot stores to an address \
         nobody described",
    );
}

/// **Two independent sources do not agree**, which is what says the bytes came from the devices
/// rather than from anything shared underneath them (a fixed seed, a counter, the DMA page's
/// previous contents). Also the cheapest proof that two services can hold two devices at once.
#[test_case]
fn two_entropy_services_on_two_devices_do_not_produce_the_same_bytes() {
    let Some(mmio) = start(Bus::Mmio) else {
        crate::testing::skip!("no virtio-rng device on the mmio bus (NIFE_RNG not set?)");
    };
    let Some(pci) = start(Bus::Pci) else {
        crate::testing::skip!("no virtio-rng device on the pcie bus (NIFE_RNG not set?)");
    };
    let a = draw(&mmio);
    let b = draw(&pci);
    for (i, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
        assert_ne!(
            x, y,
            "draw {i} is identical on both devices ({x:#018x}): these are not two sources",
        );
    }
}

/// **The count in a reply is the truth about the reply.** A short request gets exactly that
/// many bytes and leaves the rest of the caller's buffer alone, so a caller cannot be handed
/// padding it mistakes for entropy; an oversized one is clamped and answered rather than
/// refused; and an opcode the service does not implement is answered with nothing rather than
/// killing the service, which the draw afterwards proves.
#[test_case]
fn a_reply_never_delivers_more_bytes_than_it_says() {
    let Some(w) = start(Bus::Mmio) else {
        crate::testing::skip!("no virtio-rng device on the mmio bus (NIFE_RNG not set?)");
    };

    let mut buf = [0xAAu8; 8];
    assert_eq!(w.get(3, &mut buf), 3, "asked for three bytes");
    assert_eq!(
        &buf[3..],
        &[0xAA; 5],
        "the service wrote past the count it reported",
    );

    let mut big = [0u8; 8];
    assert_eq!(
        w.get(200, &mut big),
        entropy_protocol::MAX_BYTES as usize,
        "an oversized request should be clamped and answered, not refused",
    );

    let r = crate::sched::ipc_call(w.request, [entropy_protocol::req(0xff, 8), 0]);
    assert_eq!(
        r[0],
        entropy_protocol::NO_ENTROPY,
        "an unknown opcode should be answered with no bytes",
    );

    let mut after = [0u8; 8];
    assert_eq!(
        w.get(8, &mut after),
        8,
        "the service stopped serving after an unknown opcode",
    );
}

/// **More than one word's worth takes more than one round trip, and the loop that does it is
/// checked here** (milestone 159).
///
/// `entropy_protocol` carries [`entropy_protocol::MAX_BYTES`] bytes per exchange, so a caller wanting a
/// 32-byte buffer needs four. Nothing in this suite exercised that until now, and the gap had a
/// cost: the riscv64 boot tour asked `get(32, ..)`, compared the 8 it got against 32, and printed
/// FAILED on a working JH7110 for three days, on the one machine in the world that runs the
/// branch. `Wiring::fill` is the loop, and this runs it against the backend CI actually has.
///
/// Two claims, and the second is the one the tour was really making. A full buffer comes back
/// full, across round trips. And two consecutive fills differ, which for a 32-byte fill means the
/// service crossed at least one internal refill boundary without repeating itself.
#[test_case]
fn a_fill_gathers_across_round_trips() {
    let Some(w) = start(Bus::Mmio) else {
        crate::testing::skip!("no virtio-rng device on the mmio bus (NIFE_RNG not set?)");
    };

    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    assert_eq!(
        w.fill(&mut a),
        32,
        "a 32-byte fill should gather four replies, not stop at the first",
    );
    assert_eq!(w.fill(&mut b), 32, "the second fill came up short");
    assert!(
        a.iter().any(|&x| x != 0),
        "a full fill of zeros is a dead source, not entropy",
    );
    assert_ne!(a, b, "two fills returned identical bytes");
}

/// **The instruction backend, milestone 162: RNDRRS on aarch64, no virtio device at all.**
///
/// `Bus::Instruction` needs `ID_AA64ISAR0_EL1.RNDR` set, which the suite's default CPU
/// (`cortex-a72`, an ARMv8.0-A part) does not implement: `FEAT_RNG` is ARMv8.5. This is the same
/// shape as the virtio tests above skipping when `NIFE_RNG` is unset, one level down the stack:
/// a real hardware precondition this suite cannot fake, named rather than assumed. Run this test
/// for real **under TCG** with `script/test --arch aarch64 --cpu neoverse-n2` (verified 2026-08-24
/// against QEMU 11.0.2), or with `--cpu max`, which carries `FEAT_RNG` too and passed this test on
/// 2026-09-19. Until that date `max` could not be used here: the kernel refused to boot on it,
/// reading its `ID_AA64MMFR0_EL1.TGran4 = 0b0001` (the `FEAT_LPA2` encoding of "supported") as
/// "no 4 KiB granule". That was a decoder bug, since fixed; `crates/machine_discovery/src/aarch64.rs`
/// keeps the record.
///
/// **That flag reaches a TCG run and nothing else, which is a narrower fix than it sounds**
/// (checked 2026-09-10, after the earlier wording here read as though a flag reliably fixed this
/// skip). `--cpu` selects an *emulated* CPU model, so it has no meaning under HVF, where the guest
/// runs the physical Apple Silicon core: `helpers/qemu-runner-aarch64.sh` refuses `NIFE_CPU` there
/// outright ("the guest runs the physical core; `-cpu host` is mandatory"). And the HVF leg cannot
/// answer the question anyway, for a reason that has nothing to do with entropy: QEMU 11.1.1's HVF
/// accelerator wants `GICv3` and `kernel/src/drivers/gic.rs` speaks GICv2 only, so `--hvf` refuses
/// before a single test runs (milestone 227, a `GICv3` driver; milestone 222 is why that refusal is
/// loud rather than silent, and `notes/interrupts.md`'s `BUGS` section has the detail). **So
/// whether Apple Silicon implements `FEAT_RNG` is unknown here, not known to be absent**, and
/// nobody can find out from this machine until 227 lands.
///
/// **The practical consequence: this test has passed exactly once, by hand, when milestone 162
/// landed.** CI never takes the passing leg either. `.github/workflows/ci.yml`'s `test` job runs
/// `script/ci-build` over QEMU/TCG with no `NIFE_CPU` set anywhere, so it gets the default
/// `cortex-a72`, and the `cpu-matrix` job that does sweep CPU models is riscv64-only by its own
/// name. Every run since has read as clean with this test in the skip column. Giving aarch64 the
/// CPU-matrix coverage riscv64 already has is a roadmap item of its own rather than this file's
/// job; what this comment owes a reader in the meantime is not to imply that a flag will fix their
/// skip. On `x86_64`, `RDSEED` has no such gap
/// because ring 3 does not exist yet on that port at all (see
/// `design/roadmap/162-cpu-instruction-entropy.md`), so there is no service to test end to end
/// there yet, only the kernel-side probe the boot tour already proves. This test also compiles and
/// runs (and skips) on riscv64, which has neither instruction: `is_instruction_backend_available`
/// there is unconditionally `false`, the JH7110's real hardware source (milestone 159) being a
/// separate driver entirely, so the skip is correct there too, just for a different reason. **That
/// exclusion is checked, not pending**: milestone 162 refused a riscv64 arm outright, having read
/// the ISA rather than assumed it, and recorded the refusal under "Follow-on" in
/// `design/roadmap/162-cpu-instruction-entropy.md`.
///
/// **`x86_64`'s `is_instruction_backend_available` arm now checks
/// `arch::isa::get().has_random_seed_instruction()`** (ring 3 landed, milestone 161 (the `x86_64`
/// kernel port) item 3), so this test's logic is complete on that architecture too. It does not yet
/// run there in this suite: this whole module is `#[cfg(all(test, initrd))]`, and
/// `kernel/build.rs::declare_initrd_cfg` has no `x86_64` arm until milestone 161 item 4's
/// userspace-compilation hand-off lands, so `x86_64` has no `entropy` binary in its initrd to spawn
/// yet. Confirmed working ahead of that landing (2026-08-25): cherry-picking the `x86_64` fix of
/// milestone 162 (real hardware entropy on `x86_64` and aarch64) onto that hand-off's branch makes
/// this exact test pass under `-cpu max`, `x86_64`'s suite default, no `--cpu` override needed the
/// way aarch64's does.
#[test_case]
fn a_client_obtains_unpredictable_bytes_from_rndrrs_with_no_device_at_all() {
    let Some(w) = start(Bus::Instruction) else {
        crate::testing::skip!(
            "no instruction-mode entropy source on this build (aarch64: ID_AA64ISAR0_EL1.RNDR is \
             clear, so no FEAT_RNG. riscv64: the ISA has neither instruction, refused on purpose \
             in design/roadmap/162-cpu-instruction-entropy.md). --cpu neoverse-n2 reaches the \
             passing path under TCG only, no CI leg takes it, and the HVF leg cannot boot this \
             kernel at all today (milestone 227). Read this test's doc comment first"
        );
    };
    assert!(
        !w.confined_by_iommu,
        "an instruction has no device for an IOMMU to confine",
    );
    let words = draw(&w);
    assert_unpredictable(&words, "rndrrs");
}
