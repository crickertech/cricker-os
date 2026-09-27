use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// Spawn the pair; returns the report endpoint carrying the word that crossed the minted
/// endpoint.
pub fn wire() -> RendezvousId {
    // Roles 17 and 18 of the `hello` multiplexer until milestone 291; two binaries now.
    let minter = program("rendezvous_minter").expect("no rendezvous_minter in the archive");
    let peer = program("rendezvous_peer").expect("no rendezvous_peer in the archive");
    let channel = crate::sched::create_rendezvous();
    let report = crate::sched::create_rendezvous();
    let region = crate::memory_region::create(4).expect("no region for the maker's budget");

    crate::sched::spawn(move || {
        run(
            minter,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    memory_region_cap(region),              // slot 0: the budget to mint from
                    rendezvous_cap(channel, Rights::WRITE), // slot 1: delegate the mint here
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the endpoint maker");

    crate::sched::spawn(move || {
        run(
            peer,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(channel, Rights::READ), // slot 0: receive the delegation
                    rendezvous_cap(report, Rights::WRITE), // slot 1: report the word
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the endpoint user");

    report
}
