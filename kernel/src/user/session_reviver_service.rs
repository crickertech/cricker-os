use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// Where `session_reviver` expects its FS channel; must match `components/src/session_reviver.rs`'s own
/// `FS_VA`.
const FS_VA: u64 = address_space_map::pair_page(0x0000_0000_00e6_0000);

/// Extra stack pages beyond `run`'s own one-page default (`kernel::user::USER_STACK_VA` maps
/// exactly one page unless a caller adds more, `fs_service::CLIENT_EXTRA_STACK`'s own reasoning).
/// `session_reviver`'s own `.bss` statics (`MANIFEST_BUF`/`DOC_BUF`) already keep this process's
/// largest data off the stack; this margin is for the ordinary call-frame depth of `_start` and
/// `rederive_one` together, found short by one page under `script/test`'s own aarch64 run before
/// this was added (a data abort at the stack's guard page).
const REVIVER_EXTRA_STACK_PAGES: u64 = 2;

/// A fresh, zeroed frame, returned by physical address; zeroed so no stale RAM is visible in this
/// process's extra stack pages. The zeroing itself is [`crate::memory::alloc_zeroed`]'s.
fn page_frame() -> u64 {
    crate::memory::alloc_zeroed()
        .expect("no frame for session_reviver's extra stack")
        .addr()
}

/// Report words `session_reviver.rs` sends; must match that file.
pub const RPT_OK: u64 = 1;
#[allow(dead_code)] // named for completeness with RPT_OK; the test suite checks `!= RPT_OK`
// rather than `== RPT_FAILED` so its failure message can print the stage word too
pub const RPT_FAILED: u64 = 2;

/// **Spawn `session_reviver` once**, after checking its own bytes against the boot's measurement
/// table (DECISIONS §123's second hardening refinement: gate the re-deriver's binary through
/// measured boot before it is ever granted the capability). This is the same check
/// `crates/system_initializer::boot` performs for every component it spawns and `login.rs` performs
/// for `fs_subtree_caretaker` before building one, done here because this lane wires
/// `session_reviver` directly from the kernel test harness rather than through
/// `system_initializer` (see that program's own module doc on why a new process rather than a new
/// boot phase, and why full boot integration is out of this lane's scope).
///
/// `fs_ep`/`fs_page_frame` are the store-read capability (the principal tree's root directory
/// capability, unnarrowed, matching `login.rs`/`identity_provisioner.rs`'s own bound for the
/// identical grant) and the page its clients share with the FS server. `budget_pages` sizes the
/// construction budget `session_reviver` spends per identity it re-derives, the same way
/// `login.rs`'s `mint()` sizes a caretaker's construction budget.
///
/// Returns `None` if the archive has no `session_reviver` entry at all, or the measurement table
/// refuses its bytes (unvouched); `Some([w0, w1, w2])` otherwise, the report `session_reviver`
/// sends before it exits ([`RPT_OK`]/[`RPT_FAILED`], detail, and on success whether the deletion
/// proof held; see that program's own `_start` for the exact words).
pub fn revive(fs_ep: RendezvousId, fs_page_frame: u64, budget_pages: u64) -> Option<[u64; 3]> {
    let image = program("session_reviver")?;

    // **The same measurement check `crates/system_initializer::boot` performs for every component
    // it spawns** (milestone 104), read the identical way: bytes that are not UTF-8 become the
    // empty table rather than a fault, and an empty table vouches for nothing, so a build that
    // packed no measurement data refuses this process rather than spawning it unvouched.
    let table = program(measured_boot::PROGRAM_MEASUREMENTS)
        .and_then(|b| core::str::from_utf8(b).ok())
        .unwrap_or("");
    if measured_boot::verify_in_manifest(table, "session_reviver", image).is_err() {
        return None;
    }

    let report = crate::sched::create_rendezvous();
    let ut = crate::memory_region::create(budget_pages)
        .expect("no untyped for session_reviver's construction budget");

    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; 1 + REVIVER_EXTRA_STACK_PAGES as usize];
    maps[0] = Mapping {
        va: FS_VA,
        phys: fs_page_frame,
        flags: Flags::user_data(),
    };
    for (k, m) in maps[1..].iter_mut().enumerate() {
        // SAFETY-relevant fact, not a SAFETY comment (this is safe Rust): `USER_STACK_VA` is the
        // one page `run` already maps; these land directly below it, growing the stack downward,
        // the identical shape `fs_service::spawn_fs_client`'s own `extra_stack` uses.
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
        m.phys = page_frame();
    }

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE), // slot 0: REPORT
                    memory_region_cap(ut),                 // slot 1: UT
                    rendezvous_cap(fs_ep, Rights::WRITE),  // slot 2: FS_EP
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn session_reviver");

    let r = crate::sched::ipc_recv(report);
    Some([r[0], r[1], r[2]])
}
