# Notes index: Verification and security

Part of [the notes index](../README.md), which says how to add a line.

- [Machine-checked proofs (Kani)](../verification.md): how the Kani proofs work, and what they prove.
- [Proving things about `kernel/src`](../kernel-proofs.md): proving kernel code, and the stub boundary. Name provisional.
- [Proving things about `user/`](../user-proofs.md): proving the EL0 programs, and what it found. Name provisional.
- [Verus, and whether it reaches the code Kani stops at](../verus.md). Name provisional.
- [Upstreaming the riscv64 target to Kani](../kani-upstream.md): the branch, and the pull request text. Name provisional.
- [Did the proofs catch the bugs?](../proof-retrospective.md). Name provisional.
- [Falsification records](../falsification.md): recording that each proof harness can fail.
- [Fuzzing the parse surface](../fuzzing.md): coverage-guided fuzzing of the parsers that read outside bytes.
- [Overflow checks](../overflow-checks.md): which builds panic on integer overflow and which wrap, what checking the shipped build found and costs, and the options (provisional name).
- [Dynamic undefined-behavior checking (Miri)](../undefined-behavior.md): Miri over the host crates, and what "clean" means.
- [Interleavings, model-checked (loom)](../interleaving.md): loom over the hand-rolled concurrency protocols, and its finds.
- [Mutation testing](../mutation-testing.md): the cargo-mutants triage rule, the current census, and per-crate triage in 17 appendices.
- [The mutation census record](../mutation-census.md): per-crate mutation scores for every census, comparable. Names provisional.
- [Where an unsafe obligation is written, and where it is only implied](../unsafe-obligations.md).
- [What nife claims a confined component cannot do](../confinement-claims.md).
- [Verdict briefs for fatal risks 6 and 7](../fatal-risks-6-and-7-verdict-briefs.md): evidence and a recommended colour each, for the architect to rule on.
- [A security audit](../security.md): the first adversarial review of the whole kernel.
- [Auditing the shared pages](../shared-page-audit.md): the second security audit, reading for double fetches.
- [Auditing untrusted counterparty input](../untrusted-input-audit.md): network and device input read as hostile.
- [What each system makes you trust, measured](../trusted-base.md).
- [The incremental path to a safer kernel, and why nife is not on it](../incremental-path.md). Name provisional.
- [RedLeaf, and the opposite bet about where isolation comes from](../redleaf.md).
