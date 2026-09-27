use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// Spawn the revoker with an 8-page memory region; returns the endpoint it reports its verdict on.
///
/// It was role 16 of the `hello` multiplexer until milestone 291 and is
/// `fixtures/src/frame_revoker.rs` now, which reads nothing from `x0`.
pub fn wire() -> RendezvousId {
    let image = program("frame_revoker").expect("no frame_revoker program in the archive");
    let region = crate::memory_region::create(8).expect("no untyped for the revoke demo");
    let report = crate::sched::create_rendezvous();
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0, // one job, no role selector
                arg1: 0,
                arg2: 0,
                grants: &[
                    memory_region_cap(region),             // slot 0: retype + page tables
                    rendezvous_cap(report, Rights::WRITE), // slot 1: report the verdict
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the revoke demo");
    report
}
