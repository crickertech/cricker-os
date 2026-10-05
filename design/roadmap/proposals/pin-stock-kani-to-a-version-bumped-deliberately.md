---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: 589
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# Pin stock Kani to a version, bumped deliberately like the nightly

Raised by the lane `lane/verify-args` on 2026-10-05 (UTC), from a run of a patched Kani against nife
that day. Title, slug and every name here are provisional.

**Reuse:** `script/toolchain-bump` and the `rust-toolchain.toml` pin are the existing deliberate-bump mechanism, and this extends it to the prover; `cargo install --version` is the stock way to pin, so nothing new is built.

## The problem

`.github/workflows/verify.yml` installs stock Kani with `cargo install --locked kani-verifier` in
three places, unpinned. That was 0.68.0 on 2026-10-05. The riscv64 job builds 0.67.0 plus
`patches/kani-0.67.0-riscv64-target.patch`. So a proof can change outcome because a Kani release
moved while nife did not, and the architecture rows of `kernel` are not proved by one version.
`rust-toolchain.toml` pins the nightly and `script/toolchain-bump` moves it on purpose; the prover is
the one input to the proofs that moves by itself.

## Proposal

Pin stock Kani to an exact version (`cargo install --locked kani-verifier --version X`) in the
workflow and in `script/verify`'s first-run install, and move it with a deliberate bump: a commit
that changes the pin, runs the full suite, and records any harness whose result or time changed.
Whether the stock pin and the riscv64 pin should be one version is part of the bump decision; today
the patch forces 0.67.0 on one row.

Refused: leaving it floating and relying on the failure being obvious. A newer Kani that proves less
(a changed default, a dropped check) is green, and nothing would say so.

The pin policy is an architect's call; this lane changed nothing.

## What becomes droppable if upstream Kani PR #4913 lands

[model-checking/kani#4913](https://github.com/model-checking/kani/pull/4913) adds `--target`
(see `notes/kani-upstream.md`). Once a released Kani carries it:

- `patches/kani-0.67.0-riscv64-target.patch` and the patched-Kani build in `script/verify-riscv64`.
- The `KANI_TARGET` build-time and run-time environment hack.
- Possibly the arm64 kernel verify job. It exists because `crate::arch` follows the host's
  `target_arch` (milestone 304), so each host proves one architecture. With `--target`, one host can
  prove all three kernel rows, so the second host has no reason left.
- The version skew above, since riscv64 would run on the same stock pin as every other row.

Exit criteria: one pinned stock version proves all three `kernel` rows in CI; the patch, the
`KANI_TARGET` plumbing and (if the rows agree) the arm64 job are gone.
