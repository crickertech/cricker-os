# Upstreaming the riscv64 target to Kani

*Name: provisional (`notes/kani-upstream.md` and the `notes/kani-upstream/` appendix). Written
2026-09-24 (UTC) by the lane `lane/kani-upstream`.*

calef ruled on 2026-09-25 (UTC) that nife carries a patch letting Kani verify riscv64 code, and
that the same change goes upstream against
[model-checking/kani#2402](https://github.com/model-checking/kani/issues/2402). Another lane carries
the patch. This note covers the upstream half: what was built, why it has this shape, and the two
texts calef posts under his own account. The research behind it is the proposal
`design/roadmap/proposals/kani-can-target-riscv64-from-the-hosts-we-have.md` (pull request #1280).

## Where it is

- Branch: [calef/kani `riscv64-target`](https://github.com/calef/kani/tree/riscv64-target), one
  commit on Kani `main` at `de756c936` (2026-09-24).
- Pull request body, ready to paste: [kani-upstream/pull-request.md](kani-upstream/pull-request.md).
- Comment for #2402: [kani-upstream/issue-2402-comment.md](kani-upstream/issue-2402-comment.md).

Opening it, from the root of this repository:

```console
$ gh pr create -R model-checking/kani --base main --head calef:riscv64-target \
    --title "Add an unstable --target option and a riscv64 machine model" \
    --body-file notes/kani-upstream/pull-request.md
$ gh issue comment 2402 -R model-checking/kani --body-file notes/kani-upstream/issue-2402-comment.md
```

## What Kani asks of a contributor

Read at `de756c936`, not recalled:

- **No DCO and no CLA.** No workflow checks `Signed-off-by` or a CLA. `CONTRIBUTING.md` and the pull
  request template ask for one sentence in the body: "By submitting this pull request, I confirm
  that my contribution is made under the terms of the Apache 2.0 and MIT licenses." That sentence is
  the attestation. It is already the last line of the body, and it is calef's to make.
- **Discuss significant work in an issue first.** #2402 is that issue: open since 2023-04-23, with a
  second use case added by a zerocopy maintainer and no objection from anyone.
- **The RFC process** applies to one-way doors and to changes with a significant design component.
  It also asks that a new feature be reachable only behind `-Z`. An option behind `-Z
  unstable-options` is not a one-way door. The multi-target sysroot layout could be read as design,
  so the body offers to write an RFC if a reviewer asks for one.
- **Kani's `AGENTS.md`** has guidance for AI assistants, so an agent-written pull request is
  expected there. The body still says it was written by an agent under calef's direction, as nife's
  `**Lane:**` line does here.
- Pull requests are squash-merged, so the branch is one commit.

## The shape, and why

**One pull request: an unstable `--target <TRIPLE>` flag, a sysroot that holds libraries for more
than one target, and the riscv64 machine model.**

- A smaller opening pull request with only the machine-model table was considered. It lost because
  nothing in Kani's CI could exercise it. A riscv64 model is reachable only from a riscv64 host or
  through a target flag, and Kani's runners are x86_64 and arm64. The RFC process asks for "a
  testable end-to-end flow" in every pull request. The flag and the model test each other, so they
  go together.
- The flag alone, tested with x86_64 to aarch64, was the other split. It would work, but it would
  send the riscv64 reason for the change in a second pull request, and that reason is the case that
  makes the first one worth reviewing.
- **Multi-target libraries.** The host's `lib/` is untouched. `cargo build-dev --lib-target T` puts
  T's libraries in `targets/T/lib/`, which is a sysroot of its own, so the driver's `LibConfig`
  needed no change. Release bundles and `cargo kani setup` are unchanged and still host-only. That
  is listed under "Not in this PR", along with 32-bit, 16-bit and big-endian targets.
- The gate is `-Z unstable-options`, as Kani uses for other experimental options, rather than a new
  named `-Z` feature. A feature name would be a naming decision for Kani's maintainers, so the body
  offers one.

Diff: 20 files, 366 lines added and 33 removed. The prototype patch (46 lines over 4 files, environment variable)
was the starting point. It is not what went upstream: the environment variable is gone, and so is
the `KANI_TARGET` read at `build-kani`'s compile time.

## Tests, on patagonia

Kani built with `cargo build-dev --lib-target riscv64gc-unknown-linux-gnu`, two jobs, niced, 20
minutes wall under a load average above 100 from other lanes. CBMC 6.11.0 (Kani's pin) was
extracted from the Homebrew bottle into the session scratchpad. Nothing was installed machine-wide.

| run | result |
|---|---|
| `cargo test -p kani-driver`, `-p kani_metadata`, `-p build-kani` | 104, 2+2, 0 passed; no failures |
| `./scripts/kani-fmt.sh --check` | clean |
| new `script-based-pre/target_riscv64` | passed (7.3 s). A `git clean` then deleted the untracked test before commit; it was rewritten from the lane's transcript, and compiletest's stamp skipped the rerun |
| suite `ui` | 152 passed |
| suite `cargo-ui` | 30 passed |
| suite `cargo-kani` | 71 passed |
| suite `expected` | 478 passed, 16 ignored |
| suite `cargo-coverage` | 2 passed |
| suite `script-based-pre` | 75 passed, 1 failed (`verify_std_cmd`), not yet diagnosed |
| suite `coverage` | failed: the harness could not find `kani-cov`, which the regression script builds first and this run did not. Not a result |

Not run: the `kani`, `firecracker`, `prusti`, `smack` and `kani-fixme` suites.

## Found on the way

`goto-cc` configures itself for the host, and linking `kani_lib.c` overwrites the
`__CPROVER_architecture_*` symbols Kani writes. Measured with `goto-instrument
--show-symbol-table`: for riscv64, `architecture_arch` is `"riscv64"` in the `.symtab.out` and
`"arm64"` in the linked `.out`, and `char_is_unsigned` goes from 1 to 0. A plain host run on
patagonia shows the same `char_is_unsigned` flip, so this predates the change. For Rust it looks
harmless, because the goto program carries explicit widths, and pointer width and endianness match
for every accepted target. The pull request body reports it rather than fixing it, since `goto-cc`
has no riscv64 `-march` entry to pass. It is upstream's to track, and the body offers an issue.

For nife this bounds what a riscv64 proof claims. Rust-level widths and `cfg` are riscv64's, but
CBMC's C-library models run with the host's C configuration. Nothing in `kernel/` calls C.

## Where this lane stopped

Paused 2026-09-25 05:30 (UTC) on the maintainer's instruction to save budget. The branch is pushed.
The upstream pull request is not open and nothing was posted to #2402. What is left before calef
can open it:

1. Diagnose `verify_std_cmd`. The change rejects `verify-std --target`, which that test does not
   pass, so the failure may be the machine (load average above 100) or a baseline failure. The way
   to tell is to run the test on unmodified `main`.
2. Rerun `coverage` with `kani-cov` on `PATH` (`cargo build -p kani-cov`, then add `target/debug`).
3. Fill `SUITE_RESULTS` in the pull request body from the table above.
4. Check this note against the prose ratchet (§212 (a prose budget), §213 (writing standards)) in
   `script/lint`. Not yet run.
