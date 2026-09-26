//! **Try to reach what it did not declare, and print what the kernel answered** (milestone 590
//! (the booted system starts its network stack); widened by milestone 198 (a package manager)
//! rung 3a for milestone 202 (every confinement test is a ritual until somebody breaks the
//! confinement)).
//!
//! The negative control for `network_echo_client`, and since DECISIONS §219 (how the shell names an installed program to the spawner) gate D2 the fixture
//! for an unvouched child as well. Its manifest (`grant_plan::Prog::UnreachableNetworkWitness`)
//! declares nothing, so the progenitor's spawn service must leave the process domain, entropy and
//! network slots empty. It asks each of them anyway, with a real request, and then lists every slot
//! it holds:
//!
//! ```text
//! nife> unreachable_network_witness
//! network: refused (no capability at slot 10)
//! entropy: refused (no capability at slot 9)
//! domain: refused (no capability at slot 7)
//! slots held: 0
//! ```
//!
//! **Two ways to be run, one claim each.** By name it is a vouched program with an empty manifest,
//! and the progenitor must endow it the output alone. As `installed/unvouched` (`xtask/src/
//! disk.rs`), stripped so no digest in any table matches, it is bytes nobody vouched for, run by a
//! session holding the run-unvouched capability, and the progenitor must endow it what the line
//! delegated plus the clock and configuration pages (`grant_plan::UNVOUCHED_MANIFEST`), which is
//! `slots held: 0 1 2`. That is milestone 202's claim that an unvouched child holds no capability
//! the caller did not delegate beyond the two pages the ruling allows.
//!
//! **Its note asks for all three, and that is deliberate** (milestone 597 (a program carries its
//! manifest in an ELF note), provisional). The program carries a manifest note ([`NOTE_ASKS`])
//! declaring the network, entropy and the process domain, which its compiled-in manifest does not.
//! Run by name, the note is never read. Run as `installed/unvouched`, it is the only manifest the
//! bytes have, and DECISIONS §219 says an unvouched program's note grants nothing: the census must
//! still read `0 1 2`, while `caps installed/unvouched` prints what the note asks beside what will
//! be granted. A progenitor that honoured an unvouched note turns all three lines here into
//! `REACHED`.
//!
//! A progenitor that endowed any of the three authorities to a child that did not declare it turns
//! its line into `REACHED`, and the census gains that slot; `script/swish-check` fails on either.
//! That is the whole reason this is a program rather than a sentence in a doc: the claim is about
//! what the spawn service does, and only a child it spawned can report what it was given.
//!
//! **The census asks the kernel, not the program's own expectations.** Each slot below the reserved
//! fault slot is invoked with a method no object defines (`user_mode_runtime::is_granted`): an empty
//! slot answers `NoSuchSlot`, anything else is held. So a capability placed at a slot nobody thought
//! to probe is still counted.
//!
//! `unwritable_clock_witness`'s shape, three authorities over.
//!
//! # What this program holds
//!
//! - slot 0: its output, the sink contract (`crates/byte_sink_protocol`).
//! - slots 1 and 2, only when run unvouched: the clock and configuration pages, which it does not
//!   read.
//!
//! # BUGS
//!
//! - **The name says less than the program does**: it now witnesses three unreachable authorities,
//!   not one. Renaming is an architect's call; `installed/unvouched` is the name the milestone 202
//!   test uses.
//!
//! Name: provisional. Introduced 2026-09-24 for milestone 590, after `unwritable_clock_witness`.

#![no_std]
#![allow(missing_docs)]
#![no_main]

use socket_protocol::{OP_OPEN_TCP, req};
use user_mode_runtime::{call, exit, is_granted, send, survey};

/// The output slot: the sink contract.
const OUT: u64 = 0;

/// **What this program's note asks for, which is more than it is ever given.** Its compiled-in
/// manifest (`Prog::UnreachableNetworkWitness`), plus the three authorities it probes. See the
/// module documentation for why a witness carries a note that overreaches.
const NOTE_ASKS: grant_plan::Manifest = grant_plan::Manifest {
    network: true,
    entropy: true,
    domain: true,
    ..grant_plan::Prog::UnreachableNetworkWitness.manifest()
};

manifest_note::carry!(NOTE_ASKS);

// The lines below name the slots in prose; this keeps the prose honest if a constant moves.
const _: () = assert!(grant_plan::NETWORK_SLOT == 10);
const _: () = assert!(grant_plan::ENTROPY_SLOT == 9);
const _: () = assert!(grant_plan::DOMAIN_SLOT == 7);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    // A kernel refusal comes back in the first word as a negative `abi::Error`; a service's reply
    // is non-negative. So the sign alone says whether something answered, and anything answering
    // is the failure this program exists to catch.
    let no_slot = abi::Error::NoSuchSlot as i64;
    let (r0, _) = call(grant_plan::NETWORK_SLOT, req(OP_OPEN_TCP, 0), 0);
    say(match r0 as i64 {
        r if r == no_slot => b"network: refused (no capability at slot 10)\n",
        r if r < 0 => {
            b"network: refused, but not for want of a capability (slot 10 holds something)\n"
        }
        _ => b"network: REACHED. a program that declared no network was handed one\n",
    });
    let (r0, _) = call(
        grant_plan::ENTROPY_SLOT,
        entropy_protocol::req(entropy_protocol::GET, entropy_protocol::MAX_BYTES),
        0,
    );
    say(match r0 as i64 {
        r if r == no_slot => b"entropy: refused (no capability at slot 9)\n",
        r if r < 0 => {
            b"entropy: refused, but not for want of a capability (slot 9 holds something)\n"
        }
        _ => b"entropy: REACHED. a program that declared no entropy was handed it\n",
    });
    // `SURVEY` from cursor 0: an empty slot is `NoSuchSlot`, a view of a domain answers with a
    // cursor, and a capability without `ENUMERATE` is `NotPermitted`, which still means something
    // is there.
    let (r0, _, _) = survey(grant_plan::DOMAIN_SLOT, 0);
    say(match r0 {
        r if r == no_slot => b"domain: refused (no capability at slot 7)\n",
        r if r < 0 => {
            b"domain: refused, but not for want of a capability (slot 7 holds something)\n"
        }
        _ => b"domain: REACHED. a program that declared no process domain was handed one\n",
    });

    // The census, every slot below the reserved fault slot (which `START` read and cleared).
    let mut line = [0u8; 96];
    let head = b"slots held:";
    line[..head.len()].copy_from_slice(head);
    let mut n = head.len();
    for slot in 0..abi::fault::FAULT_EP_SLOT {
        if is_granted(slot) {
            line[n] = b' ';
            n += 1;
            if slot >= 10 {
                line[n] = b'0' + (slot / 10) as u8;
                n += 1;
            }
            line[n] = b'0' + (slot % 10) as u8;
            n += 1;
        }
    }
    line[n] = b'\n';
    say(&line[..n + 1]);
    send(OUT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

/// Write `line` to the output, a packed chunk at a time.
fn say(line: &[u8]) {
    let mut rest = line;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(OUT, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
