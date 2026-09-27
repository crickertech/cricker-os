use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// Spawn the witness; returns the report endpoint carrying its verdict bits.
///
/// It was role 19 of the `hello` multiplexer until milestone 291 and is
/// `fixtures/src/address_space_witness.rs` now, which reads nothing from `x0`.
pub fn wire() -> RendezvousId {
    let image = program("address_space_witness").expect("no address_space_witness in the archive");
    let report = crate::sched::create_rendezvous();
    let region = crate::memory_region::create(8).expect("no region for the builder");

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0, // one job, no role selector
                arg1: 0,
                arg2: 0,
                grants: &[
                    memory_region_cap(region),             // slot 0: the budget
                    rendezvous_cap(report, Rights::WRITE), // slot 1: the verdict
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the address space builder");

    report
}
