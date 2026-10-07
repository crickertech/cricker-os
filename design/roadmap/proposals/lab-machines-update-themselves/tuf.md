# Rust TUF clients on nife's targets, 2026-10-07

An appendix of [lab machines update themselves](../lab-machines-update-themselves.md), for its
Fork 9. Measured 2026-10-07 (UTC) by build and link only, with
[tuf-probe.sh](tuf-probe.sh), whose recipe is `notes/filesystem-crates-2026-10-07/probe.sh`'s
(#1803). The farm was the main checkout's `target/nife-farm`, so this lane did not retake the
account-wide `nife-dev` link. Nothing ran under nife.

## The two clients

| | rust-tuf (`tuf`) | tough |
|---|---|---|
| repository | theupdateframework/rust-tuf, branch `develop` | awslabs/tough |
| license | MIT or Apache-2.0 | MIT or Apache-2.0 |
| upkeep | crates.io stops at 0.3.0-beta9 (2022-07-19); `develop` is 0.3.0-beta15, last pushed 2026-08-24; README says "Beta Software" | 0.24.0, 2026-07-10, the sixth release since 2024-10 |
| who uses it | Fuchsia's `pkg-resolver` (`src/sys/pkg/bin/pkg-resolver/BUILD.gn` lists `//third_party/rust_crates:tuf`) | Bottlerocket's `updog` (`sources/updater/updog/Cargo.toml`) |
| crypto | `ring` 0.17, referenced in 11 source files; `crypto.rs` is 1,285 lines | `aws-lc-rs` |
| async and threads | `futures-io` and `futures-util`; no runtime, no spawn found | tokio with `fs` and `rt`; tokio's file system runs on its blocking pool, a thread (tokio's documentation, recalled) |
| `std` | needs it (`tempfile`, `chrono`, `url`) | needs it (`tempfile`, `walkdir`) |

## Builds

| probe | host | aarch64 | riscv64 | x86_64 | stops at |
|---|---|---|---|---|---|
| tough 0.24.0 | pass | fail | fail | fail | `aws-lc-sys`'s build script: no `stdlib.h` |
| tuf 0.3.0-beta9 (crates.io) | fail | | | | E0282 in its own `client.rs` on nightly-2026-10-06 |
| tuf at `develop` `219ca7d0` | pass | fail | fail | fail | `ring`'s build script: no `assert.h` |

Both stops are the class `notes/cryptography-provider.md` already records for `ring` and
`aws-lc-rs` (C crypto with no libc headers), which is why the tree built `cryptography_provider`
on RustCrypto (`ed25519-dalek`, `p256`). Neither crate's own code was reached on nife's targets,
so whatever else it needs is unmeasured. The next measurement is rust-tuf with its `ring` calls
replaced by those verifiers.

## Prior art

- The specification: <https://theupdateframework.github.io/specification/latest/>.
- PyPI, PEP 458 (Accepted, <https://peps.python.org/pep-0458/>): root, targets and bins keys are
  offline; timestamp, snapshot and bin-n keys are online. PEP 480 is the "maximum security model",
  where developers sign their own distributions.
- Sigstore (<https://github.com/sigstore/root-signing>): its trusted root is delivered by TUF and
  signed by a threshold of five keyholders, with online signatures re-signed about every three days.
- Uptane Standard 2.1.0 (<https://uptane.org/docs/latest/standard/uptane-standard>): an Image
  repository holds images and their metadata, and a Director repository tells each device what to
  install, on demand. A lab with a channel per machine is the Director's shape.
- Fuchsia and Bottlerocket, above.
