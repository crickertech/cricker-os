use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::RendezvousId;

/// Where the tool expects the caller to have staged the identity and secret, in
/// `credential_protocol::place`'s layout. Must match `components/src/identity_provisioner.rs`'s own `REQ_VA`.
const REQ_VA: u64 = address_space_map::pair_page(0x0000_0000_00e4_0000);
/// The page shared with the credential service. Must match the same file's `PROV_VA`, and
/// `components/src/credentialer.rs`'s own `PROV_VA` (the physical frame behind both must be the one
/// `credential_service::Wiring::provision_page_frame` names).
const PROV_VA: u64 = address_space_map::pair_page(0x0000_0000_00e0_0000);
/// The page shared with the file service. Must match the same file's `FS_VA`.
const FS_VA: u64 = address_space_map::pair_page(0x0000_0000_00e5_0000);

/// Report words `components/src/identity_provisioner.rs` sends; must match the same file.
pub const RPT_OK: u64 = 1;
#[allow(dead_code)] // named for completeness with the others; no test in this suite provokes it
pub const RPT_MALFORMED: u64 = 2;
#[allow(dead_code)] // named for completeness with the others; no test in this suite provokes it
pub const RPT_SUBTREE_FAILED: u64 = 3;
pub const RPT_CRED_FAILED: u64 = 4;

/// **Run the provisioning tool once**, against an already-wired credential service's provision
/// endpoint and an already-wired file service's root directory, and wait for its report.
///
/// `prov` must be the credential service's own provision endpoint, before its seal (milestone 56);
/// `prov_page_frame` is the exact physical frame it maps at its own `PROV_VA`
/// (`credential_service::Wiring::provision_page_frame` on the instance `prov` came from). `fs_ep`/
/// `fs_page_frame` are the file service's root directory capability and the page its clients share with
/// it (`fs_service::root_directory`), unnarrowed: see `components/src/identity_provisioner.rs`'s own
/// module docs on why that is this slice's bound and not a design decision.
///
/// This module plays the operator's role that a real shell does not yet: it stages `identity` and
/// `secret` into the request page directly, the same way `narrow_dir`'s `name: &'static str`
/// argument is supplied by the kernel test rather than by guest-side staging. The tool itself never
/// sees where the bytes came from; it only relays what is in the page.
///
/// Returns `[report_word0, report_word1]` verbatim (see `identity_provisioner.rs`'s `RPT_*`
/// constants and this module's re-exports of them).
pub fn provision(
    image: &'static [u8],
    prov: RendezvousId,
    prov_page_frame: u64,
    fs_ep: RendezvousId,
    fs_page_frame: u64,
    identity: &[u8],
    secret: &[u8],
) -> [u64; 2] {
    let report = crate::sched::create_rendezvous();
    let id_len = identity.len() as u64;
    let secret_len = secret.len() as u64;

    let req_phys = crate::memory::alloc()
        .expect("no frame for the provisioning request")
        .addr();
    // SAFETY: a fresh frame, direct-mapped, owned by nobody else until the mapping below hands it
    // to the spawned process. Written before that mapping exists, so there is no concurrent access.
    let req_slice = unsafe {
        core::slice::from_raw_parts_mut(mmu::phys_to_virt(req_phys) as *mut u8, FRAME_SIZE as usize)
    };
    req_slice.fill(0);
    credential_protocol::place(req_slice, identity, secret, credential_protocol::provision::PUT).expect(
        "identity/secret out of credential_protocol's bounds; this is a test-fixture bug, not a wiring one",
    );

    let maps = [
        Mapping {
            va: REQ_VA,
            phys: req_phys,
            flags: Flags::user_data(),
        },
        Mapping {
            va: PROV_VA,
            phys: prov_page_frame,
            flags: Flags::user_data(),
        },
        Mapping {
            va: FS_VA,
            phys: fs_page_frame,
            flags: Flags::user_data(),
        },
    ];

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: id_len,
                arg1: secret_len,
                arg2: 0,
                grants: &[
                    rendezvous_cap(prov, Rights::WRITE), // slot 0: PUT the credential
                    rendezvous_cap(fs_ep, Rights::WRITE), // slot 1: MKDIR the home subtree
                    rendezvous_cap(report, Rights::WRITE), // slot 2: say what happened
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn identity_provisioner");

    let r = crate::sched::ipc_recv(report);
    [r[0], r[1]]
}
