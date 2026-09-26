use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// Where the service maps the provisioner's page. Must match components/src/credentialer.rs.
const PROV_VA: u64 = address_space_map::pair_page(0x0000_0000_00e0_0000);
/// Where the service and a client map the verify page. Must match both programs.
const VERIFY_VA: u64 = address_space_map::pair_page(0x0000_0000_00e1_0000);

/// The service's untyped budget, in pages: 6 MiB. It pays for one thing, the Argon2id scratch,
/// which is `credentialer::Cost::DEFAULT.blocks()` KiB (4 MiB today), plus the page tables that map it
/// and the allocator's slack. Sized from the cost parameter rather than guessed, so raising the
/// cost is a change in two places that fail loudly together rather than one that fails at run
/// time under load.
const CRED_BUDGET_PAGES: u64 = 1536;

/// Extra stack pages for the service, and this is a **measured** number rather than a cautious
/// one: with the default single page the service takes a data abort at `0x4fff00`, 256 bytes
/// below the stack, on its first derivation. Argon2's inner loop copies whole 1 KiB `Block`s
/// through locals (`block_r`, `block_tmp` in `fill_block`), so one page is not close to enough
/// and no amount of care in *our* code would have made it enough. The net server takes 8 pages
/// for smoltcp for the same kind of reason; this takes 16, because a KDF that overflows its
/// stack fails as a killed process on every login rather than as a bad answer, and the pages
/// are cheap next to the 4 MiB the scratch already costs.
const CRED_STACK_PAGES: u64 = 16;

/// The clients' budgets. A `credentialer_test_client` role holds no untyped at all: it maps one page the wiring
/// placed and calls one endpoint. There is nothing for it to build.
///
/// This constant does not exist. The absence is the point, and it is written down because a
/// reader looking for "what memory does the attacker get" should find the answer rather than
/// conclude it was overlooked.
const _NO_CLIENT_BUDGET: () = ();

/// The `credentialer_test_client` roles; must match `fixtures/src/credentialer_test_client.rs`.
pub const ROLE_HONEST: u64 = 0;
pub const ROLE_ATTACKER: u64 = 1;
pub const ROLE_PROVISIONER: u64 = 2;

/// The flag bits a client packs into its report's third word; must match the same file.
pub const F_CLEAN: u64 = 1 << 0;
// Bits 1 and 2 were the NTLM role's, removed 2026-08-30 with the SMB implementation
// (notes/smb.md). Not reused: the next flag takes bit 3, so an old transcript cannot be misread.

/// The report words `credentialer_test_client` and the service send, likewise.
pub const RPT_DONE: u64 = 0x_c2ed_c11e_0000_0001;
pub const RPT_READY: u64 = 0x_c2ed_0000_0000_0001;

/// A running credential service and the endpoints that reach it.
pub struct Wiring {
    /// The service's readiness endpoint. It reports **after** the seal, so receiving on this is
    /// also how a caller knows provisioning is over.
    pub ready: RendezvousId,
    /// The verify endpoint. This is what a client is given, with WRITE.
    pub verify: RendezvousId,
    /// The provision endpoint. Held here only so the provisioner can be spawned against it;
    /// after the seal, the service has deleted its receive end and a `CALL` here would block
    /// forever, which is why nothing sends on it afterwards.
    pub provision: RendezvousId,
    /// The physical frame behind the verify page, **for this instance specifically**. Captured at
    /// [`start`] time and carried on the `Wiring` itself, rather than through a shared global keyed
    /// on "the credential service": milestone 155 wired a second, independently wired store (a
    /// fresh, unsealed one to provision against, alongside the suite's one shared, already-sealed
    /// fixture), and a global that assumed exactly one instance for the life of the test binary
    /// could not tell the two apart. Every caller in this module now reads this field, or
    /// [`provision_page_frame`](Wiring::provision_page_frame), instead.
    pub verify_page_frame: u64,
    /// The physical frame behind the provision page, for this instance. Same reason as
    /// [`verify_page_frame`](Wiring::verify_page_frame).
    pub provision_page_frame: u64,
}

/// **Wire and spawn the credential service.** It blocks on its provision endpoint immediately,
/// so nothing happens until [`provisioner`] runs.
///
/// `entropy` is the entropy service's request endpoint (DECISIONS §44). It is not optional: a
/// credential service that cannot draw a salt refuses to start, and passing it a dead endpoint
/// is how that path gets tested.
pub fn start(image: &'static [u8], entropy: RendezvousId) -> Wiring {
    let provision = crate::sched::create_rendezvous();
    let verify = crate::sched::create_rendezvous();
    let ready = crate::sched::create_rendezvous();
    let budget = crate::memory_region::create(CRED_BUDGET_PAGES)
        .expect("no untyped for the credential store");

    // The two shared pages, then the extra stack, in one array the spawn closure owns.
    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; CRED_STACK_PAGES as usize + 2];
    maps[0] = Mapping {
        va: PROV_VA,
        phys: page_frame(),
        flags: Flags::user_data(),
    };
    maps[1] = Mapping {
        va: VERIFY_VA,
        phys: page_frame(),
        flags: Flags::user_data(),
    };
    // Captured locally for `Wiring::provision_page_frame`/`Wiring::verify_page_frame`, which is what every
    // caller elsewhere in this module now reads instead of a shared global (see those fields' own
    // docs): correct for *this* instance no matter how many others are wired in the same boot.
    let (provision_page_frame, verify_page_frame) = (maps[0].phys, maps[1].phys);
    for k in 0..CRED_STACK_PAGES as usize {
        // Zeroed so the process starts clean, which for this process also means it does not
        // start with somebody else's bytes where its key material will go.
        let phys = crate::memory::alloc_zeroed()
            .expect("no frame for the credential service's stack")
            .addr();
        maps[k + 2] = Mapping {
            va: USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE,
            phys,
            flags: Flags::user_data(),
        };
    }
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0,
                arg1: 0, // no physical address: this process touches no device
                arg2: 0,
                grants: &[
                    rendezvous_cap(provision, Rights::READ), // slot 0: write the store, until SEAL
                    rendezvous_cap(verify, Rights::READ),    // slot 1: answer questions, forever
                    rendezvous_cap(entropy, Rights::WRITE),  // slot 2: salts, naming no device
                    memory_region_cap(budget),               // slot 3: the memory-hard scratch
                    rendezvous_cap(ready, Rights::WRITE),    // slot 4: one message, after the seal
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the credential service");

    Wiring {
        ready,
        verify,
        provision,
        verify_page_frame,
        provision_page_frame,
    }
}

/// **Spawn the provisioner** and wait for its report. It fills the store and seals it, so when
/// this returns the service is in phase two and the provision endpoint is dead at both ends.
///
/// Its endowment is the provision endpoint, a report endpoint, and the provisioner's page. It
/// holds no verify endpoint, which is the mirror of the client's position: neither party can do
/// the other's job, and neither is prevented from it by a check.
pub fn provisioner(image: &'static [u8], w: &Wiring) -> [u64; 5] {
    spawn_cli(
        image,
        ROLE_PROVISIONER,
        w.provision,
        PROV_VA,
        w.provision_page_frame,
    )
}

/// **Spawn a client** in `role` against the verify endpoint, and wait for its report.
pub fn client(image: &'static [u8], w: &Wiring, role: u64) -> [u64; 5] {
    spawn_cli(image, role, w.verify, VERIFY_VA, w.verify_page_frame)
}

/// The one spawn site the three `credentialer_test_client` roles share, because the whole claim is that they
/// differ in their endowment and not in their code. Changing `endpoint` and `va` here is the
/// entire difference between a provisioner and an attacker.
///
/// `phys` is taken from the caller's own `Wiring` (`w.verify_page_frame`/`w.provision_page_frame`) rather than
/// from a shared global: milestone 155 wired a second, independent credential service instance in
/// the same boot, so nothing here can assume there is only one to ask about. A caller with its own
/// `Wiring` in hand always knows which frame is correct.
fn spawn_cli(
    image: &'static [u8],
    role: u64,
    endpoint: RendezvousId,
    va: u64,
    phys: u64,
) -> [u64; 5] {
    let report = crate::sched::create_rendezvous();
    let maps = [Mapping {
        va,
        phys,
        flags: Flags::user_data(),
    }];
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: role,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(endpoint, Rights::WRITE), // slot 0: the service
                    rendezvous_cap(report, Rights::WRITE),   // slot 1: say what happened
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn a credential client");
    crate::sched::ipc_recv(report)
}

/// Allocate one fresh shared frame. Every caller keeps the return value itself (in a `Wiring`
/// field, or in a `Mapping`'s own `phys`), rather than this module remembering it on a caller's
/// behalf: milestone 155's own fix removed the global that used to do that (`FRAMES`), because a
/// global keyed on nothing but "the credential service" cannot distinguish two independently wired
/// instances in the same boot, and after that milestone there can be more than one.
fn page_frame() -> u64 {
    // Zeroed so a client's first look at the page cannot find somebody else's memory, which for
    // this contract would mean finding it where a secret is supposed to go.
    crate::memory::alloc_zeroed()
        .expect("no frame for a credential page")
        .addr()
}

// `peek`, which read a shared frame through the direct map, lived here until 2026-08-30. Its only
// caller was the check that no NTLM key material survived in the frame the adapter and this service
// shared, and it went with the SMB implementation (notes/smb.md). It is worth knowing that the
// technique exists: a kernel test can read a userspace shared frame directly, which is the only way
// to assert what a program did *not* leave behind, and `git show 685900ec` has the ten lines.

/// Unpack the `k`th reply code from a `credentialer_test_client` report's second word. One byte per code; see
/// `fixtures/src/credentialer_test_client.rs` `Codes`.
pub const fn nth(packed: u64, k: u32) -> u64 {
    (packed >> (8 * k)) & 0xff
}
