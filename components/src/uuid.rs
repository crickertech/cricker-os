//! **`uuid`**: print a version-4 UUID drawn from the entropy service (milestone 111, DECISIONS §44,
//! notes/entropy.md).
//!
//! The whole program is: ask the entropy service for sixteen bytes, stamp the six bits RFC 9562
//! reserves, print the canonical form. It holds two capabilities and neither of them can reach the
//! random device, exactly `date`'s shape one manifest field over
//! ([`grant_plan::Manifest::entropy`] rather than `Manifest::clock`).
//!
//! # Why this program exists, and why it is not a demonstration
//!
//! Milestone 56 built the entropy service and the grant that reaches it, and nothing at the prompt
//! could pass that grant on: a program needing randomness worked when the *system* spawned it
//! (`credentialer` at boot, `disk_partitioner` under the kernel's own test harness) and could not
//! be run by a person. `date` before `components/src/date.rs` existed is the position that left; this is
//! that milestone's `date`.
//!
//! It is **`disk_partitioner`'s draw with the disk taken away**. Both call
//! `globally_unique_identifier_partition_table::guid::Guid::v4_from_random` over sixteen bytes
//! from the same service, because a GPT gives every partition a random globally unique id and
//! `crates/globally_unique_identifier_partition_table` refuses to invent one. The partitioner also
//! needs a disk capability, which this shell does not hold and cannot attenuate; the sixteen bytes
//! and the stamping are the half that a prompt can reach today.
//!
//! # The capability table
//!
//! | slot | what | why it is that and not wider |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the identifier goes |
//! | 8 | the diagnostics sink, `WRITE` | where the refusal goes, so `>` cannot swallow it |
//! | 9 | the entropy service, `WRITE` | the right to `CALL` it, and nothing else |
//!
//! `WRITE` on slot 9 is the whole grant. `READ` would let this program `RECEIVE` on the service's own
//! request endpoint, which is to take another client's request out from under it; `GRANT` would let
//! it hand a random source to something it spawned. Neither is given, and neither is needed.
//!
//! # The refusal is the interesting half
//!
//! A program spawned without [`grant_plan::Manifest::entropy`] holds an empty slot 9. Its first
//! `CALL` comes back as `abi::Error::NoSuchSlot`, which `entropy_protocol::delivered` reports as
//! `None` rather than as a short reply, and this program then prints **nothing at all** and says
//! why on its second stream.
//!
//! That is deliberate and it is why the manifest declares `OutputSpec::BytesAndDiagnostics`. A
//! `uuid > id.txt` on a boot with no entropy service must leave the file empty, because a file
//! containing a predictable identifier is worse than a file containing nothing: the first is wrong
//! and looks right. There is no counter fallback here for the same reason `disk_partitioner` has
//! none (its `R_NO_ENTROPY`), and the same reason
//! `crates/globally_unique_identifier_partition_table` will not invent a GUID.
//!
//! # Arguments: none
//!
//! There is no argv on this ABI (notes/abi.md) and this program needs none. Unix's `uuidgen` takes
//! a version selector; there is one version here, and a count would want a positional argument
//! `ArgSpec` cannot carry yet (the same gap `printenv`'s and `date`'s module docs name).
//!
//! # EXAMPLES
//!
//! ```text
//! uuid                     A4E1B0C7-2F3D-4A81-9C6E-5B7D0F2A1E44
//! uuid > id.txt            (the identifier is in the file, nothing on the terminal)
//! uuid  (no entropy)       uuid: no entropy capability was granted; nothing was generated
//! caps uuid                cap 9  endpoint  entropy  WRITE. it may ask the entropy service ...
//! ```
//!
//! # BUGS
//!
//! - **One identifier per invocation.** `uuidgen -n 10` has no spelling here, because `ArgSpec` has
//!   no positional argument yet. Ten invocations are ten spawns.
//! - **Version 4 only.** No name-based (v3/v5) or time-based (v1/v7) form, which would need a
//!   hash and a clock this program does not hold. A v7 would be worth having once something wants
//!   sortable identifiers; nothing does.
//! - **"Unpredictable" is a claim about the boot, not about this program.** The bytes are whatever
//!   the entropy service delivers, and on QEMU that is a virtio-rng backed by the host
//!   (DECISIONS §120's stopgap). Whether a real board's TRNG is sound is
//!   `notes/entropy.md`'s open question and endowing a grant does not settle it.
//! - **A short draw is treated as a failure.** `entropy_protocol` delivers at most eight bytes a round
//!   trip and this program needs sixteen, so it makes two calls and refuses if either answers with
//!   fewer than eight. It does not retry. `disk_partitioner::random16` makes exactly the same call.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist). Introduced 2026-09-05
//! alongside `grant_plan::Manifest::entropy`. RFC 9562's own term for the object, and a term of art
//! already right per this tree's naming convention for standard terms.
//!
//! Refused `uuidgen`, Unix's name for the *tool*: that name carries an argument surface this ABI
//! cannot deliver, so it would promise a program this is not.
//!
//! Refused `guid`, **and the reason recorded here until 2026-09-13 was wrong in a way worth
//! correcting rather than quietly replacing.** It said `crates/gpt` calls the same sixteen bytes a
//! `Guid` "because GPT's spec does, but the spec is Microsoft's spelling of the same object". They
//! are not the same object in the same layout. A GUID is **mixed-endian**: the first three groups
//! go on disk little-endian and the last two in the order written, where RFC 9562's UUID is
//! big-endian throughout. `crates/globally_unique_identifier_partition_table/src/guid.rs` is built
//! around exactly that trap ("a GUID that looks plausible, matches nothing, and is byte-reversed in
//! three places out of five") and proves the round trip for all 2^128 with
//! `a_guid_survives_printing_and_parsing`.
//!
//! So the refusal stands and is stronger than its old reason: `guid` here would not be a
//! stylistic borrowing from another vendor, it would assert a byte order this program does not
//! produce. For the same reason **`crates/gpt` keeps `Guid`** (calef, 2026-09-13, asked directly
//! whether it should follow this ratification): that name distinguishes two encodings a reader
//! will otherwise conflate, which is the load-bearing version of the argument §113's amendment
//! found false for `crates/pci`. (The crate was `gpt` when both of these were written; §154
//! renamed it `globally_unique_identifier_partition_table` on 2026-09-18, and `Guid` stayed.)

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and documenting an
// OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use entropy_protocol as entropy;
use globally_unique_identifier_partition_table::guid::Guid;
use user_mode_runtime::{call, exit, is_granted, send};

/// Slot 0: where the identifier goes. An endpoint with `WRITE`, under the sink contract.
const REPORT: u64 = 0;

/// Slot 8: the declared second stream (DECISIONS §67). The refusal goes here so a `>` on the line
/// cannot swallow it into a file that then looks like a successful run.
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;

/// Slot 9: the entropy service, `WRITE`. Its *presence* is what [`random16`] finds out about, and
/// it finds out by asking rather than by probing: a `CALL` on an empty slot answers
/// `abi::Error::NoSuchSlot`, which `entropy_protocol::delivered` separates from every real count.
const ENTROPY_SLOT: u64 = grant_plan::ENTROPY_SLOT;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let has_diag = is_granted(DIAG_SLOT);

    // **Draw first, print second**, which is the ordering the refusal forces rather than a style:
    // this program must write nothing at all when it holds no entropy, so nothing may go out
    // before the draw has answered. §67's reader also drains the second stream to end-of-stream
    // before it reads a byte of output, so the complaint has to be finished before the identifier
    // starts.
    let drawn = random16();

    if drawn.is_none() {
        write_on(
            if has_diag { DIAG_SLOT } else { REPORT },
            b"uuid: no entropy capability was granted; nothing was generated\n",
        );
    }
    if has_diag {
        send(DIAG_SLOT, byte_sink_protocol::eof(), 0, 0);
    }

    if let Some(bytes) = drawn {
        let mut line = [b'\n'; 37];
        line[..36].copy_from_slice(&Guid::v4_from_random(bytes).to_ascii());
        write_on(REPORT, &line);
    }

    send(REPORT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

/// Sixteen random bytes from the entropy service, or `None` if this process holds no entropy
/// capability or the service has none to give.
///
/// Two round trips, because a reply carries one word (`entropy_protocol::MAX_BYTES` is 8; DECISIONS
/// §44 spends the round trip rather than putting random bytes in a page a second party maps).
/// `entropy_protocol::delivered` is what separates "no capability" (a kernel error in the register a
/// count would have arrived in) from "the service has none" (a real count of zero) from a real
/// draw, and this program treats the first two the same: nothing is written either way.
///
/// **`components/src/disk_partitioner.rs`'s `random16`, verbatim in shape.** The two are the same draw
/// against the same contract, and keeping them recognisably one thing is worth more than sharing
/// twelve lines through a crate neither would otherwise need.
fn random16() -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    for half in 0..2 {
        let (r0, r1) = call(
            ENTROPY_SLOT,
            entropy::req(entropy::GET, entropy::MAX_BYTES),
            0,
        );
        let n = entropy::delivered(r0)?;
        if n != entropy::MAX_BYTES as usize {
            return None;
        }
        entropy::take(n, r1, &mut out[half * 8..half * 8 + 8]);
    }
    Some(out)
}

/// Write bytes to an endpoint under the sink contract, sixteen at a time. No newline is added:
/// where a line ends is the caller's business and not the transport's.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
