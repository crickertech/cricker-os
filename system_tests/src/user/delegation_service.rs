/// The word the receiver sends back through the delegated capability, so a test can confirm a
/// capability minted by one process works when invoked by another.
///
/// **One definition, in `capability_witness_protocol`.** This was a `pub const` here with a matching
/// literal in `fixtures/src/hello.rs` and a "must match" comment beside it, which is the shape
/// AGENTS.md rule 7 exists to remove; milestone 291 split the receiver into its own binary and the
/// duplicate became a crate.
pub use capability_witness_protocol::USED_WORD;

use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::RendezvousId;

/// Spawn the pair and return `(resource endpoint, report endpoint)`. The granter delegates its
/// `resource` capability (held `WRITE | GRANT`) to the receiver, narrowed to `WRITE`. The
/// receiver `SEND`s [`USED_WORD`] on the received capability (a `RECV` on `resource` collects
/// it) and reports a two-bit verdict on `report`.
pub fn wire() -> (RendezvousId, RendezvousId) {
    let granter = program("delegation_granter").expect("no delegation_granter in the archive");
    let receiver = program("delegation_receiver").expect("no delegation_receiver in the archive");
    let channel = crate::sched::create_rendezvous(); // granter SEND_CAP -> receiver RECV_CAP
    let resource = crate::sched::create_rendezvous(); // the capability being delegated
    let loopback = crate::sched::create_rendezvous(); // the receiver's refused re-delegation target
    let report = crate::sched::create_rendezvous(); // the receiver's verdict

    crate::sched::spawn(move || {
        run(
            granter,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(channel, Rights::WRITE), // slot 0: SEND_CAP over it
                    rendezvous_cap(resource, Rights::WRITE.union(Rights::GRANT)), // slot 1: delegate this
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the delegation granter");

    crate::sched::spawn(move || {
        run(
            receiver,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(channel, Rights::READ),   // slot 0: RECV_CAP
                    rendezvous_cap(report, Rights::WRITE),   // slot 1: report the verdict
                    rendezvous_cap(loopback, Rights::WRITE), // slot 2: attempt re-delegation here
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the delegation receiver");

    (resource, report)
}
