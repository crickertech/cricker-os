use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::RendezvousId;

/// Spawn `uuid` with **nothing but somewhere to print**, which is what a program that did not
/// declare [`grant_plan::Manifest::entropy`] is born holding.
///
/// `printenv_tests::spawn_printenv`'s `None` arm, and the same claim one authority over: the
/// program's whole cspace is slot 0, so [`grant_plan::ENTROPY_SLOT`] is empty and a `CALL` on it
/// can only be refused by the kernel.
///
/// **There is no `Some` arm here, and that is a limitation rather than a choice.** `Spawn::grants`
/// fills a child's capability table from slot 0 upward and cannot place a capability at the slot a
/// manifest names, which is the same gap `xtask`'s `swish-check` list already records for `date`'s
/// second stream. So the endowed half of this milestone is proven at the real prompt, through the
/// real `crates/system_initializer`, by `script/swish-check`; see this module's own note in
/// `design/roadmap/111-entropy-for-a-child.md`.
fn spawn_uuid_holding_no_entropy() -> RendezvousId {
    let image = program("uuid").expect("no uuid program in the initrd archive");
    let out = crate::sched::create_rendezvous();
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[rendezvous_cap(out, Rights::WRITE)],
                maps: &[],
            },
        )
    })
    .expect("could not spawn uuid");
    out
}

/// One message off the sink, as bytes, or `None` at end of stream.
/// `printenv_tests::line`'s reader with the framing left visible, because this test needs to see
/// **where the stream ends** and not only what a line said.
fn chunk(out: RendezvousId, buf: &mut [u8; 16]) -> Option<usize> {
    let words = crate::sched::ipc_recv(out);
    if words[0] == byte_sink_protocol::eof() {
        return None;
    }
    let count = words[0] as usize;
    assert!(
        (1..=16).contains(&count),
        "uuid: a stdout message with a bad byte count: {count}"
    );
    buf[..8].copy_from_slice(&words[1].to_le_bytes());
    buf[8..].copy_from_slice(&words[2].to_le_bytes());
    Some(count)
}

/// **A program that was not endowed entropy draws none, prints no identifier, and says so.**
///
/// This is the half of milestone 111 that carries the claim. The endowment is the easy direction:
/// wire a capability, watch a program use it. The claim is the other one, that authority a program
/// did not declare is not quietly available anyway, and randomness is the authority where that is
/// hardest to check by looking: a process that draws a key and a process that hardcodes one make
/// the same syscalls and produce output of the same shape. Nothing but a test that takes the
/// capability away can tell them apart.
///
/// So the assertions are about **silence**. Not one byte of the 36-character identifier reaches the
/// sink, the whole stream is the one sentence, and the process ends normally rather than faulting
/// on a `CALL` that could not succeed. A `uuid` that fell back to a counter, a boot-time seed or a
/// monotonic tick would pass no part of this, which is exactly the fallback
/// `crates/globally_unique_identifier_partition_table` refuses to provide and `disk_partitioner`
/// refuses to invent.
///
/// Arch-neutral, so **both ISAs run literally this test** (DECISIONS §19): what is under test is
/// the capability model's answer to an empty slot, which is not instruction-set-specific.
#[test_case]
fn a_process_granted_no_entropy_prints_no_identifier() {
    let out = spawn_uuid_holding_no_entropy();

    let mut said = [0u8; 128];
    let mut len = 0usize;
    let mut buf = [0u8; 16];
    while let Some(n) = chunk(out, &mut buf) {
        assert!(
            len + n <= said.len(),
            "uuid with no entropy said more than a sentence"
        );
        said[len..len + n].copy_from_slice(&buf[..n]);
        len += n;
    }
    let said = &said[..len];

    assert_eq!(
        said, b"uuid: no entropy capability was granted; nothing was generated\n",
        "uuid with an empty entropy slot said something other than its refusal"
    );

    // And the shape of what it did not say. A version-4 identifier is 36 characters with four
    // hyphens in fixed places, so a single hyphen anywhere in this stream would mean bytes derived
    // from *something* got out. `entropy_protocol::delivered` reading a kernel error as `None` rather
    // than as a short count is what makes that impossible, and this is the assertion that it holds
    // through the whole program rather than only at the accessor.
    assert!(
        !said.contains(&b'-'),
        "uuid with no entropy emitted identifier-shaped bytes"
    );
}
