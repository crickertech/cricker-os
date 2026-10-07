//! **The wiring for the three network time programs** (milestone 51; split from one binary by
//! milestone 290).
//!
//! Until milestone 290 there was one `ntp` binary with three `arg0` roles. There are three programs
//! now: `components/src/network_time_client.rs`, `fixtures/src/network_time_test_server.rs` and
//! `fixtures/src/unwritable_clock_witness.rs`.
//!
//! **The invariant this file carries is that the client and the witness are endowed by one
//! function.** `spawn_with_client_endowment` takes the image as a parameter, so a slot added to
//! the client is a slot the witness gets, and there is no second capability list to forget. That,
//! rather than the two sharing a binary, is what keeps
//! `an_ntp_client_holds_no_writable_clock_page` proving what it claims: the fault comes from the
//! capability set, and any process holding that set faults at that address whatever code it runs.
//! **Do not grow a second list here.** A hand-maintained copy of a fact the wiring already holds is
//! how a confinement claim decays without breaking a build (milestone 117's `swish` `caps` bug).

use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// The words the three network time programs report. Must match `RPT_*` in
/// `components/src/network_time_client.rs`, `fixtures/src/network_time_test_server.rs` and
/// `fixtures/src/unwritable_clock_witness.rs`.
///
/// **One numbering space across all three**, which is what it was when they were one binary and did
/// not move in the split, so a boot log from before milestone 290 still reads. Each program declares
/// only the words it sends; the whole vocabulary is here even though the tests assert on five of the
/// eight, because the point of a mirror is that it is complete and a failing test that prints `3` is
/// only readable if `3` has a name on this side too.
#[allow(dead_code)]
pub mod rpt {
    pub const SYNCED: u64 = 1;
    pub const REJECTED: u64 = 2;
    pub const NO_REPLY: u64 = 3;
    pub const NO_ENTROPY: u64 = 4;
    pub const NET_ERROR: u64 = 5;
    pub const PROBING: u64 = 6;
    pub const SERVED: u64 = 7;
    pub const BAD_LOCAL_TIME: u64 = 8;
}

/// Which reply the test server sends. Must match `fixtures/src/network_time_test_server.rs`'s `srv`.
pub mod srv {
    pub const GOOD: u64 = 0;
    pub const BAD_ORIGIN: u64 = 1;
    pub const KISS_OF_DEATH: u64 = 2;
    pub const SHORT: u64 = 3;
}

/// The small integers the client reports a `network_time_protocol::Reject` as. Must match
/// `components/src/network_time_client.rs`'s `reject_code`; kept as distinct values rather than a
/// bool because which check refused a packet is the difference between a broken server and an
/// attack.
pub mod reject {
    pub const LENGTH: u64 = 1;
    pub const KISS_OF_DEATH: u64 = 4;
    pub const ORIGIN_MISMATCH: u64 = 7;
}

/// How many requests the client makes before giving up. Must match `ATTEMPTS` in
/// `components/src/network_time_client.rs`.
pub const ATTEMPTS: u64 = 3;

/// Each of the three mints or maps at most one shared frame and pays for its page tables. Small
/// and fixed: none of them links a heap.
const BUDGET_PAGES: u64 = 16;
/// Extra stack pages. The client builds a 48-byte packet and does IPC; the same three `socket_test_client`
/// and its relatives get is plenty and leaves no doubt.
const STACK_PAGES: u64 = 3;

/// A running test server: the endpoint a client speaks the socket contract on, and the endpoint
/// it reports the first request it saw on.
pub struct Server {
    /// The socket contract's endpoint. The server holds `READ`; a client is given `WRITE`.
    pub stack: RendezvousId,
    /// **Drain this before waiting on the client.** The server reports once, with a blocking
    /// `send`, and the client's `RECEIVE` is queued behind it.
    pub report: RendezvousId,
}

/// Extra stack for one of the three, allocated and zeroed. Returned by value so the spawn closure
/// owns it.
fn stack_pages() -> [Mapping; STACK_PAGES as usize] {
    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; STACK_PAGES as usize];
    for (k, m) in maps.iter_mut().enumerate() {
        // Zeroed so the process starts clean.
        let phys = crate::memory::alloc_zeroed()
            .expect("no frame for a network time program's stack")
            .addr();
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
        m.phys = phys;
    }
    maps
}

/// Spawn `network_time_test_server`: `variant` selects the reply it builds, `claimed_nanos` is the
/// wall-clock time it claims. The kernel supplies the claimed time because the server holds no
/// clock capability of its own, which keeps every test's expectation an exact number rather
/// than a window.
///
/// **Four slots, and it is a different endowment from the client's on purpose.** It holds no
/// propose endpoint and no entropy endpoint, so a reader can see from the wiring alone that the peer
/// answering the client cannot itself reach the clock.
pub fn start_server(image: &'static [u8], variant: u64, claimed_nanos: u64) -> Server {
    let stack = crate::sched::create_rendezvous();
    let report = crate::sched::create_rendezvous();
    let budget = crate::memory_region::create(BUDGET_PAGES).expect("no untyped for the ntp server");
    let maps = stack_pages();

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: variant,
                arg1: claimed_nanos,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE), // slot 0: the one report
                    rendezvous_cap(stack, Rights::READ),   // slot 1: serve the socket contract
                    memory_region_cap(budget),             // slot 2: map the client's frame
                    // slot 3: what an opened socket is minted from (§255 (each socket is its own
                    // capability))
                    rendezvous_cap(stack, Rights::WRITE.union(Rights::GRANT)),
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn network_time_test_server");

    Server { stack, report }
}

/// **The client's endowment, in one place, and the whole security argument rests on there being
/// only one of these.** `entropy` is an `Option` on purpose: a client wired without it is the test
/// that proves the refusal is loud, and the empty slot is what "no entropy capability" actually is.
///
/// `image` is a parameter, which is the load-bearing part. Both `network_time_client` and
/// `unwritable_clock_witness` come through here, so the witness is given **the same five slots** as
/// the client by construction: what it fails to reach, it fails to reach as a fully endowed network
/// time client rather than as a stripped-down one. Before milestone 290 the two were one binary and
/// this file's argument was that they shared *code*; they never needed to, because the fault the
/// witness proves comes from the capability set rather than from the instructions. **Adding a second
/// list of grants beside this one is the way this claim decays without breaking a build.**
///
/// `a0` and `a1` are the program's own two arguments (the client's server address and port, the
/// witness's target address); `arg2` is unused by both.
fn spawn_with_client_endowment(
    image: &'static [u8],
    a0: u64,
    a1: u64,
    stack: RendezvousId,
    propose: RendezvousId,
    entropy: Option<RendezvousId>,
) -> RendezvousId {
    let report = crate::sched::create_rendezvous();
    let budget = crate::memory_region::create(BUDGET_PAGES).expect("no untyped for the ntp client");
    let maps = stack_pages();
    // Slot 4 is granted or it is not; there is no third state and no flag inside it. An
    // ungranted slot answers a `CALL` with `NoSuchSlot`, which is how the client tells "there
    // is no entropy service" from "the service has none".
    let n_grants = if entropy.is_some() { 5 } else { 4 };
    let entropy = entropy.unwrap_or(0);

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: a0,
                arg1: a1,
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE),  // slot 0: the verdict
                    rendezvous_cap(stack, Rights::WRITE),   // slot 1: the network
                    memory_region_cap(budget),              // slot 2: the shared frame
                    rendezvous_cap(propose, Rights::WRITE), // slot 3: ask, never tell
                    rendezvous_cap(entropy, Rights::WRITE), // slot 4: the nonce
                ][..n_grants],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn a network time program");

    report
}

/// Spawn `network_time_client` against `stack`, told which server to ask (`ip` packed big-endian,
/// `port`). Returns its report endpoint.
pub fn start_client(
    image: &'static [u8],
    stack: RendezvousId,
    propose: RendezvousId,
    entropy: Option<RendezvousId>,
    ip: u32,
    port: u16,
) -> RendezvousId {
    spawn_with_client_endowment(image, ip as u64, port as u64, stack, propose, entropy)
}

/// Spawn `unwritable_clock_witness`: the client's endowment, pointed at `va`. **Same function, same
/// five slots, different image**, which is the whole of why the witness cannot drift away from what
/// the client actually holds.
pub fn start_witness(
    image: &'static [u8],
    stack: RendezvousId,
    propose: RendezvousId,
    entropy: Option<RendezvousId>,
    va: u64,
) -> RendezvousId {
    spawn_with_client_endowment(image, va, 0, stack, propose, entropy)
}
