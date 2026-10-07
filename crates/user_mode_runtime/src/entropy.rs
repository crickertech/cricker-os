//! **Fill a buffer from the entropy service**, the client half of `entropy_protocol` (milestone 56
//! (secrets, credentials, and the entropy to make them safe), DECISIONS §44 (entropy is a
//! capability), notes/entropy.md).
//!
//! Four programs (`uuid`, `disk_partitioner`, `system_installer`, and `redoxfs_server`'s `mkfs`)
//! each carried the same twelve-line `random16`: two `CALL`s of eight bytes, a `None` for a missing
//! capability, and a `None` for a short answer. The `uuid` copy's doc said it was kept
//! "recognisably one thing" with the partitioner's rather than shared; four verbatim copies are the
//! §94 (what may live in a userspace library) tell that it was one thing, asserted by hand four
//! times. Lifted here on 2026-10-06 (UTC).
//!
//! # Why here and not in `entropy_protocol`
//!
//! `entropy_protocol` is the contract: request and reply encodings, pure and dependency-free, and
//! the kernel's entropy service and the std PAL build on it as well as these clients. A fill
//! function has to `CALL` an endpoint, and putting it in the contract would make the contract
//! depend on this runtime, which owns `call` (calef, 2026-10-06, steering the lane that wrote this).
//! So the split is the usual one: the loop, which is pure and is where the refusals live, is
//! `entropy_protocol::fill_with` and takes the `CALL` as a closure, so it is host-tested there
//! (this crate has no host tests: it is bare-metal `asm!`); this module supplies the closure.
//! The std PAL's `fill_bytes` (`patches/std-nife/overlay/std/src/sys/random/nife.rs`) was the other
//! candidate and is not shareable: it lives inside `std`, these programs are `no_std`, and it
//! panics where these four must refuse quietly. Scoped to a module, the way [`super::virtio`] is,
//! because only a program granted an entropy endpoint has a use for it.
//!
//! # BUGS
//!
//! - **A short answer is a refusal, not a retry.** The service answers fewer bytes than asked only
//!   when its device is dry, and these callers would rather write nothing than wait on it. The std
//!   PAL accepts a partial count and loops; the two policies differ on purpose and are recorded at
//!   both ends.

/// Fill `out` with random bytes from the entropy service whose endpoint is in `slot`, or `None` if
/// that cannot be done in full.
///
/// `None` covers both failures, which every caller so far treats alike: this process holds no
/// entropy endpoint in `slot`, or the service answered with fewer bytes than asked. On `None` the
/// contents of `out` are unspecified and must not be used. See `entropy_protocol::fill_with` for
/// the loop and its tests.
#[must_use]
pub fn fill(slot: u64, out: &mut [u8]) -> Option<()> {
    entropy_protocol::fill_with(out, |w0| super::call(slot, w0, 0))
}
