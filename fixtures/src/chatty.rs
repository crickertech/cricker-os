//! **The client that does not notice, and the attacker that cannot** (milestone 23, DECISIONS §41).
//!
//! One binary, three roles, because an attacker that holds the honest client's own capabilities is
//! a fair test of the boundary rather than a different program failing for its own reasons. The
//! third is `ROLE_PRODUCER`, the same conversation one rung up the latency ladder on the queued
//! channel. (This line said "two roles" until 2026-09-13; the producer arrived with the queue
//! broker and nothing updated the count.)
//!
//! # The client (`ROLE_CLIENT`)
//!
//! It calls a stable endpoint sixty-four times in a loop and checks every answer. It holds one
//! capability to that endpoint for its whole life, it never reconnects, it never retries, and it
//! has no code path for "the server went away", because there is no such event to have a path for.
//! Somewhere in the middle of that loop the server is torn down and a different program, written in
//! a different language, takes its place.
//!
//! **It is its own witness.** The verdict it reports is computed here, from what it observed:
//! every call returned, every reply echoed the sequence number that asked for it, every digest
//! matched this program's own independent computation, and the version word went up exactly once.
//! The operator has a second, independent witness (the shared log page, in the operator's address
//! space); neither is taken on the other's word, which is the shape milestones 29, 33 and 36 used.
//!
//! What it *cannot* claim, stated plainly: the client does observe the version word change, so it
//! is not blind to the swap. The claim is that its stream was unbroken, not that a swap is
//! undetectable by a client that goes looking for one.
//!
//! # The attacker (`ROLE_USURPER`)
//!
//! It is endowed with exactly the client's capabilities, including a real, working capability to
//! the stable endpoint. If endpoint-only naming meant "whoever holds the endpoint is the server",
//! it would be able to park itself in `RECEIVE_CAP` and take the client's requests. It cannot: its
//! capability carries `WRITE` and not `READ`, and the same object handed out with different rights
//! is a one-way pipe in whichever direction each holder was trusted with. It reports the refusal.
//!
//! Name: provisional, and ruled: calef ruled **`swap_boundary_witness`** on 2026-09-13, working
//! the unratified worklist. The block stays `provisional` for the reason `flaky`'s does: the
//! ratified name is not this file's until the rename is performed, and this one travels with
//! `flaky`'s, whose file set it sits inside. Introduced 2026-07-30 with milestone 23's live replacement, as a fixture rather
//! than a component.
//!
//! `chatty` was an adjective, one of exactly two among the seventy-odd programs in `user/src/`
//! (`flaky` was the other, from the following day), where AGENTS.md says a program is a thing
//! and takes the name of a thing. But the decisive fault was not the part of speech. This
//! program is **one binary on purpose**: an attacker holding the honest client's own
//! capabilities is a fair test of the boundary, where a separate program would fail for its own
//! reasons. So a name picking one role denies the property that makes the fixture valid, and
//! there are three roles to pick wrongly from, not the two this header claimed until today.
//!
//! Refused `swap_client` and `swap_attacker` (calef's own proposal) for that reason, each naming
//! one of the two roles. `swap_attacker` also names the derived role rather than the base one:
//! `swapper` spawns `ROLE_CLIENT` twice and `ROLE_USURPER` once, and `swap_protocol` defines the
//! attacker as "the same code and capabilities as `ROLE_CLIENT`". Refused `caller` and `client`,
//! this header's own earlier candidates, as so generic they would name half the tree. Refused
//! the shorter `swap_witness` in favour of naming what is witnessed. All three roles witness one
//! boundary: the client by its stream being unbroken across the swap, the producer by the same
//! continuity on the queued channel, the attacker by its failure to take over. `witness` is
//! already this tree's word for the job, here and in `outlaw`.
//!
//! The stem is the family's. `swapper` is the operator, `rust_swappable` and `c_swappable` are
//! the two instances, and this was the only member off that stem.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

// A source file shared by several binaries through `#[path]`, and each uses a different slice of it,
// so the unused halves are expected (§38).
use swap_protocol::client_checks as ck;
use user_mode_runtime::{call, send};

/// What `swapper` endowed us with, **derived from our own declaration** (`swap_protocol::CLIENT`,
/// milestone 23's manifest). The attacker gets the same three, deliberately, and now that is a fact
/// about the code rather than about two arrays a reader has to compare.
///
/// `service` is `Direction::Use`, so the capability carries `WRITE` and not `READ`: we may ask, and
/// we may not answer. The operator no longer spells either right, which is what makes the usurper
/// refusal a property of the declaration instead of a typo waiting to happen.
const SVC: u64 = component_plan::slot_of(&swap_protocol::CLIENT, "service");
const RPT: u64 = component_plan::slot_of(&swap_protocol::CLIENT, "report");
const NOTE: u64 = component_plan::slot_of(&swap_protocol::CLIENT, "operator");

#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, _a1: u64, _a2: u64) -> ! {
    match role {
        swap_protocol::ROLE_USURPER => usurp(),
        swap_protocol::ROLE_CONFUSED => confused_deputy(),
        swap_protocol::ROLE_PRODUCER => produce(),
        swap_protocol::ROLE_PLANT => plant(),
        _ => converse(),
    }
}

/// **The conversation.** Sixty-four calls, one capability, no branches for failure.
fn converse() -> ! {
    let mut bits = ck::ALL_REPLIED | ck::SEQ_ECHOED | ck::DIGEST_CORRECT;
    let mut last_version = 0u64;
    let mut transitions = 0u64;
    let mut changed_at = 0u64;

    for seq in 0..swap_protocol::REQUESTS {
        let (mut got, mut tag) = call(SVC, swap_protocol::OPERATION_PUT, seq);

        // **We were stranded inside this call, and the thing that stranded us let go** (milestone
        // 23's hung-component channel). A component took this request and then stopped answering
        // without dying, so nothing in the kernel could wake us: a caller awaiting a reply is woken
        // by the reply and by nothing else, and `abi::Error::Gone` reaches a caller whose *endpoint*
        // died, never one whose *server is alive and silent*.
        //
        // So this arm is not error handling. It is the tail of a request that took a detour through
        // the operator's recovery, and the only honest thing to do with it is ask again: a released
        // caller has learned that its request was never served, not that it failed. On the healthy
        // channels this value never arrives and this branch is dead.
        if got == swap_protocol::WEDGE_RELEASED {
            bits |= ck::WAS_RELEASED;
            let again = call(SVC, swap_protocol::OPERATION_PUT, seq);
            got = again.0;
            tag = again.1;
        }

        // Did somebody answer at all, and was it an answer to *this* question? A reply carrying
        // another request's sequence number would mean the kernel's one-shot Reply capability had
        // misrouted, which is the guarantee DECISIONS §12 exists to make unrepresentable.
        if got == swap_protocol::BAD_REQUEST {
            bits &= !ck::ALL_REPLIED;
        }
        if swap_protocol::tag_seq(tag) != seq {
            bits &= !ck::SEQ_ECHOED;
        }
        // The independent check: our own computation of the same definition, against whatever the
        // server on the other end happened to be at the time.
        if got != swap_protocol::digest(seq) {
            bits &= !ck::DIGEST_CORRECT;
        }

        let version = swap_protocol::tag_version(tag);
        if version != last_version {
            if last_version != 0 {
                transitions += 1;
                changed_at = seq;
            }
            if version < last_version {
                // Backwards is worse than churn: it would mean the old instance answered after the
                // new one had, which is two servers on one endpoint.
                transitions += 100;
            }
            last_version = version;
        }
    }

    if transitions == 1 {
        bits |= ck::ONE_TRANSITION;
    }
    if changed_at > 0 && changed_at < swap_protocol::REQUESTS - 1 {
        bits |= ck::SPANNED_SWAP;
    }

    send(RPT, swap_protocol::RPT_CLIENT, bits, changed_at);
    // Tell the operator the conversation is over, so it reads the witness page after the last
    // request rather than in the middle of one.
    send(NOTE, swap_protocol::NOTE_CLIENT_DONE, changed_at, 0);
    user_mode_runtime::exit()
}

/// **The producer, one rung up the ladder.** The same conversation, on a channel whose front end is
/// a queue broker instead of the component itself.
///
/// The difference it can see, and the only one: some calls come back `ACCEPTED` instead of with an
/// answer, because the backend did not exist at that moment and the broker took custody. It never
/// blocks on an absent consumer and it never has to retry. What happened to those items is not this
/// program's claim to make; the backend's own log page is where they turn up, in order, and the
/// operator reads that.
fn produce() -> ! {
    let mut bits = ck::ALL_REPLIED | ck::SEQ_ECHOED | ck::DIGEST_CORRECT | ck::NONE_REFUSED;
    let mut buffered = 0u64;

    for seq in 0..swap_protocol::REQUESTS {
        let (got, tag) = call(SVC, swap_protocol::OPERATION_PUT, seq);
        match got {
            swap_protocol::ACCEPTED => {
                buffered += 1;
                continue; // no answer to check: the item is in the broker's custody, not served yet
            }
            swap_protocol::QUEUE_FULL | swap_protocol::BAD_REQUEST => {
                bits &= !ck::NONE_REFUSED;
                continue;
            }
            _ => {}
        }
        if swap_protocol::tag_seq(tag) != seq {
            bits &= !ck::SEQ_ECHOED;
        }
        if got != swap_protocol::digest(seq) {
            bits &= !ck::DIGEST_CORRECT;
        }
    }

    if buffered > 0 {
        bits |= ck::WAS_BUFFERED;
    }
    send(RPT, swap_protocol::RPT_CLIENT, bits, buffered);
    send(NOTE, swap_protocol::NOTE_CLIENT_DONE, buffered, 0);
    user_mode_runtime::exit()
}

/// **The attacker.** It tries to become the server on the endpoint it is a client of.
fn usurp() -> ! {
    let r = swap_protocol::try_receive_cap(SVC);
    // A success here is the catastrophe: it means the attacker is now parked in the queue the real
    // component receives on, and the next client request goes to it. Report the code either way and
    // let the test decide; a program that judged its own attack would be judging itself.
    send(
        RPT,
        swap_protocol::RPT_ATTACK,
        (-r) as u64,
        abi::rendezvous::RECEIVE_CAP,
    );
    // Also say so on the operator's channel, so the operator knows the attack has been made and the
    // run is not simply missing a report. `w1` carries the verdict, mirroring the report's word:
    // the operator must tell a stolen plant message from a slow plant before the report is read.
    send(NOTE, swap_protocol::NOTE_ATTACK_DONE, (-r) as u64, 0);
    user_mode_runtime::exit()
}

/// **The plant** (milestone 800 (a non-Anthropic model attacks the confinement claim), fourth pass).
/// Not an attacker: an instrument for the usurper's. A `RECEIVE_CAP` the kernel fails to refuse has
/// nothing to take when the endpoint's queue is empty, so it blocks, and the break surfaces as a
/// watchdog hang instead of a red assertion: the wrong-reason red milestone 202 (every confinement
/// test is a ritual until somebody breaks the confinement) recorded, and the
/// reason claim 26's own test could never fail. This role parks a message on the endpoint first,
/// so a let-open `RECEIVE_CAP` *returns* it ([`ipc_receive_cap`] takes any parked sender's word),
/// and the usurper's report carries a word nothing honest produces
/// ([`swap_protocol::PLANT_MARKER`]).
///
/// It parks one plain `SEND` and exits when anyone takes it: the usurper, on a kernel that let
/// one through, or the operator's bounded receive, on one that refused it. Every interleaving
/// works without any handshake: if the usurper's try has already parked as a receiver, the send
/// delivers to it directly.
fn plant() -> ! {
    send(SVC, swap_protocol::PLANT_MARKER, 0, 0);
    user_mode_runtime::exit()
}

/// **The confused deputy** (milestone 633 (an outside agent attacks the confinement claim), third pass). The same capabilities as the honest client:
/// a `WRITE` view of the stable endpoint and no device. It cannot touch the device itself, so it
/// asks the server to. `OPERATION_PUT`'s offset is the client's own word, and the server writes a
/// byte at `LOG_VA + offset` with no bound, so an offset of `DEPUTY_PROBE` lands in the device's
/// register page, which this program holds no capability for. It reports the reply, and the operator
/// and the test judge it: a normal answer means the server reached outside its log on our behalf.
fn confused_deputy() -> ! {
    // First the honest refusal, so the report stream shows this program holds only `WRITE` and could
    // not simply receive, exactly as `usurp` does.
    let r = swap_protocol::try_receive_cap(SVC);
    send(
        RPT,
        swap_protocol::RPT_ATTACK,
        (-r) as u64,
        abi::rendezvous::RECEIVE_CAP,
    );
    // The deputy call: drive the server to write far past its log page.
    let (d0, d1) = call(
        SVC,
        swap_protocol::OPERATION_PUT,
        swap_protocol::DEPUTY_PROBE,
    );
    send(RPT, swap_protocol::RPT_DEPUTY, d0, d1);
    send(NOTE, swap_protocol::NOTE_ATTACK_DONE, (-r) as u64, 0);
    user_mode_runtime::exit()
}

user_mode_runtime::panic_handler!();
