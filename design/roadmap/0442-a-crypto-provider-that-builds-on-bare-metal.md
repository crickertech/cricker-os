---
status: BUILT
raised: 2026-09-19
built: 2026-09-20
---
# 442. A crypto provider `rustls` can use on all three bare-metal targets

Built (2026-09-20), except clause 3, which is repriced into a proposal rather than
carried; see `## Follow-on`.
Minted 2026-09-19 by the maintainer, from calef's ruling in DECISIONS §196 (nife carries TLS: `rustls` for the protocol, and a crypto provider we make work),
which is [design/decisions/0196-nife-carries-tls-and-builds-the-provider.md](../decisions/0196-nife-carries-tls-and-builds-the-provider.md).
*(Number provisional until the merge queue lands it.)*

One decision is owed and it is named below: the provider that works is a dependency, and
DECISIONS §46 (thin primitives or whole subsystems; we write everything in between) makes taking
one calef's rather than a lane's. Nothing is blocked on the answer.

## In brief

The premise was true about §196's table and false about nife's targets, and the difference is
the milestone. §196 measured `rustls` and every candidate provider against the *stock* bare-metal
targets (`aarch64-unknown-none-softfloat` and its two siblings) on the stable host toolchain, and
its own limits paragraph says those are not our targets. nife builds user programs for
`targets/*-unknown-nife.json`: `"os": "nife"`, `"std": true`, `-Zbuild-std` against the patched
farm. Re-measured there, on the pinned nightly, with `entropy_backend` wired in:

`rustls` 0.23.45 plus `rustls-rustcrypto` 0.0.2-alpha builds and runs on all three
architectures. The full table, both failure classes and what each one turned out to be are in
[notes/cryptography-provider.md](../../notes/cryptography-provider.md).

The two things standing in the way were smaller than "no provider builds" suggested:

- **A `getrandom` major, not a missing backend.** `entropy_backend` answered 0.3 and 0.4;
  `rustls-rustcrypto` reaches 0.2 through `rand_core` 0.6, whose hook is a different symbol
  with a different signature. That crate's own `BUGS` section had forecast needing it
  (*"nothing currently needs the second shape. Something will."*). It answers both now.
- **Six build-configuration lines, not a wall.** `x86_64-unknown-nife` sets `+soft-float` and
  switches off every SSE level; the RustCrypto crates compile their x86 intrinsic paths anyway and
  LLVM dies with `Do not know how to split the result of this operator!`. One cargo feature
  (`sha2`'s `force-soft`) and five `--cfg` flags select the portable implementations.
- And one of those six is a run-time hazard that belongs to the target rather than to crypto.
  `chacha20` compiles for `x86_64-unknown-nife` and then executes an AVX2 instruction in ring 3,
  where that target has no SSE or AVX state, and the program dies with `vector 6 (invalid opcode)`
  before printing a byte. It picks its backend by asking the CPU rather than the compiler, and the
  CPU truthfully has AVX2. Any crate that runtime-detects SIMD is in the same position on this
  target, and the failure is a silent dead program rather than a build error.

The only providers that still fail are the ones written in C, `ring` and `aws-lc-rs`, both in
their own build scripts. That is §46 holding rather than a disappointment.

## What this milestone owed, and what happened

1. A re-measurement that is ours. Done: `script/crypto-probes`, eighteen probes times three
   architectures, run twice (as the crates ship, and with the portable paths forced). Provisional
   name. Its header records a trap that cost an hour twice and that `script/crate-probes` may
   still be in: a probe built under `target/` is inside this repository, so the repository's own
   `rust-toolchain.toml` beats `RUSTUP_TOOLCHAIN` for the rustc `-Zbuild-std` invokes and `std` is
   compiled unpatched, which reads exactly like the crate under test failing.
2. One provider that builds and passes its own test vectors on all three architectures. Done,
   and since calef refused the one crate that could have supplied it, the provider is
   `cryptography_provider`, assembled here. `cryptography_exerciser` runs vectors transcribed from
   FIPS 180-4, RFC 4231, RFC 5869, the GCM specification, RFC 8439, RFC 7748 and FIPS 186-4, and
   `kernel/src/user/cryptography_tests.rs` boots it on all three under QEMU. Provisional names.
   The RSA half is proven against the two hosts that made the case for it rather than against a
   synthetic key: the full chains from `objects.githubusercontent.com` (RSA 2048) and `ghcr.io`
   (RSA 4096 over a 3072-bit intermediate) are pinned as fixtures and verified through
   `rustls-webpki` with this provider's algorithms, anchored on the root the host itself served,
   which is §196 clause 4's model rather than a system trust store. A flipped signature bit and a
   certificate offered for the wrong host are both refused, because a verifier that only ever says
   yes is not one.
3. A client that speaks TLS 1.3 to one peer, holding that peer's root as a capability
   (§196 clause 4). **Not attempted, deliberately, and this block is the record of that.** §196's
   own `BUGS` says no HTTP client exists in the tree. A working provider with no client is what 442
   was written to produce if the client did not fit, and it did not. Certificate verification is no
   longer the gap it was when that was written: clause 2 now exercises it against real chains. What
   is missing is a socket, an HTTP request, and a peer.
4. **An honest `BUGS` section.** Below, and in the note, and in the program's own module docs.

## The decisions, and calef made both

`rustls-rustcrypto` is refused, and that is now DECISIONS §198 (the glue is ours, the primitives
are not) rather than only a lane report. calef, 2026-09-20: *"rustls-rustcrypto doesn't seem like a
high quality dependency."* The numbers are in that section and in
[notes/cryptography-provider.md](../../notes/cryptography-provider.md): 0.0.2-alpha published
2024-04-24, three versions ever with two yanked, 73 crates, pinning majors of `sha2`, `aes-gcm` and
`p256` that were all superseded during 2026.

`rsa` is taken. calef, 2026-09-20: *"Take rsa."* The provider had been built without it, at 53
crates, and that could not honestly be called right: measured with `openssl s_client`,
`github.com` is ECDSA P-256 throughout but `objects.githubusercontent.com`, where a Release asset
is actually served, is RSA 2048, and `ghcr.io` is RSA 4096. A client without RSA
verification reaches GitHub and cannot download from it.

So the provider is 67 crates, which is exactly what this block predicted for the written path
at equal coverage, and six fewer than the alpha's 73. The size argument is as weak as it can
be, and the record says so: the glue was written because it is the piece with the least exposure
in the chain, not because it is smaller.

The advisory is answered where a gate can check it. `deny.toml` carries the tree's first
`ignore` entry, for RUSTSEC-2023-0071, with its argument in the `reason` field so `cargo-deny`
prints it with the finding: the Marvin attack is a timing side channel in PKCS#1 v1.5
decryption, and this provider verifies signatures and never decrypts, holds no private key and
offers no oracle. It names what would end the claim, which is any use of `rsa` here to decrypt, to
sign, or for key transport. And `script/supply-chain` now scans the two manifests that suppression
is about, which it did not before: an `ignore` for a graph no gate reads is a claim nobody can
check. That brought three licences onto the allow-list with their own reasons (ISC for
`rustls-webpki` and `untrusted`, Unicode-3.0 for a proc-macro dependency that ships nothing).

§46's line, drawn out loud rather than assumed. The primitives are taken: their correctness
includes constant-time behavior and resistance to attacks no specification states, which is what
§46 (thin primitives or whole subsystems; we write everything in between) means by won through
exposure, and `aes-gcm` and `chacha20poly1305` carry NCC Group's 2020 audit in their own README.
The glue is written, because a `CryptoProvider` is five fields that select, name and plumb and
compute nothing. RSA PKCS#1 v1.5 verification is still refused as something to write here, even
now that `rsa` is taken and even though it is only public-key arithmetic: Bleichenbacher's forgery
and BERserk are spec-reading failures in exactly that code.

## What it unblocks

Rung 3c of the trivial install (§157): a package fetched by host name from a source somebody else
operates. Rung 3a does not wait for it, because §195 (a recipe vouches, and the owner may overrule)
makes a recipe's digest decide what may run and a digest is checkable over any transport.

## BUGS

- Nothing here measures the cost, and this milestone made that worse rather than better. A
  handshake on a board with no hardware crypto may be slow enough to matter to the install story,
  and no number exists. The soft-implementation cfgs cannot be applied per target when the target
  is a JSON specification path, so aarch64 and riscv64 now run portable code their vector units
  could have done faster, also unmeasured.
- The pinned-key model has no rotation story, which is the part every real deployment
  eventually needs and this block does not attempt.
- **A program that aborts is never heard**, and this lane published a wrong finding because of it.
  `drain_sink` waits for an end-of-stream marker the std runtime sends after `main` returns, and a
  panic under `panic = "abort"` traps instead, so the panic message and everything before it are
  delivered to the endpoint and discarded. An empty transcript is indistinguishable from a program
  that never started, and was read as that for a day. Corrected in
  [notes/cryptography-provider.md](../../notes/cryptography-provider.md), warned about at
  `std_tests::drain_sink`, and proposed as its own work.
- No handshake has ever completed, and that is the honest ceiling of this milestone. The
  provider assembles, offers what it claims, and verifies real certificate chains; no peer has
  answered it, because there is no HTTP client and no socket wiring. Proving it further would need
  either a peer or an in-process server, and an in-process server would need a `SigningKey`, which
  this provider deliberately refuses to load because a package client presents no certificate.
- **Certificate expiry is never exercised.** The pinned chains are verified at a fixed instant,
  2026-09-20, because verifying them against the clock would turn this test red the day the
  certificates expire for a reason that has nothing to do with this system. That makes the
  assertion historical, which is right for a signature test and means expiry logic is untested.
- **Two encodings a real chain may use are not supported**, and both fail closed: an RSA key whose
  SPKI names `RSASSA-PSS` rather than `rsaEncryption`, and a PKCS#1 v1.5 algorithm identifier with
  absent rather than explicit NULL parameters. `rustls`' own ring-backed table carries separate
  entries for the second. Neither GitHub chain needed either.
- **A vector proves the answer, not the manner.** Nothing measures timing, so a portable fallback
  that is correct and not constant-time passes every line, and on x86_64 the fallbacks are exactly
  what runs.

## Follow-on

- **Decision.** calef's refusal of `rustls-rustcrypto`, written up as
  `design/decisions/0198-the-glue-is-ours-the-primitives-are-not.md`.
- **Done.** Whether to take `rsa`: calef ruled "Take rsa" on 2026-09-20 and this milestone carried
  it out, in `cryptography_provider/src/verify.rs`, `deny.toml`'s first `ignore` entry, and
  `script/supply-chain`'s manifest list. The proposal that framed the question is gone rather than
  left standing as an open fork; the argument it held lives where a reader now meets it, which is
  `notes/cryptography-provider.md` and the `reason` field beside the suppression itself.
- **Milestone 501.** A client that speaks TLS 1.3 to one peer, holding that peer's root or pinned key as
  a capability, which was this block's clause 3 and is repriced out of it above:
  milestone 501 (a TLS client that speaks to one), `design/roadmap/501-a-tls-client-that-speaks-to-one-pinned-peer.md`.
- **Milestone 496.** That a program which aborts is never heard, so its panic message and every line
  before it are discarded:
  milestone 496 (a dying program's last words reach nobody), `design/roadmap/496-a-dying-programs-last-words-reach-nobody.md`. This lane's own wrong
  finding is the worked example inside it.
- **Recorded.** That `script/crate-probes` may be measuring the unpatched `std`, beside the
  measurement it feeds, in `notes/cryptography-provider.md`'s `BUGS`. The fifty crates of
  milestone 64 (enough `std` to run somebody else's crate) have not been re-run against it.
- **Recorded.** Every gap this milestone did not close, in the `BUGS` section above and in
  `cryptography_provider/src/lib.rs`: no completed handshake, no expiry check, no timing claim, no
  cost, and two certificate encodings that fail closed.

## Index row

§196's stock-target table said no crypto provider built on any nife target; re-measured against
this tree's own target specifications, the blockers were a `getrandom` major `entropy_backend` did
not answer and six lines of build configuration, one of them a run-time hazard where a crate asks
the CPU for SIMD that `x86_64-unknown-nife` has no ring-3 state for. calef refused
`rustls-rustcrypto` as a dependency and ruled "Take rsa", so nife assembles its own 67-crate
`CryptoProvider` over audited primitives, proven by published vectors and by verifying the real
certificate chains of `objects.githubusercontent.com` (RSA 2048) and `ghcr.io` (RSA 4096) under
QEMU on all three architectures. Clause 3, a TLS client, is repriced into a proposal: no handshake
has completed, because there is no peer.
