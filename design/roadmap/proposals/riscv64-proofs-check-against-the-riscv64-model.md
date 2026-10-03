---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: 589
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# riscv64 proofs check against the riscv64 model

*(Title and slug are drafts. Number minted at promotion.)*

Raised by the `lane/kani-upstream` lane on 2026-10-03 (UTC), after calef approved a follow-up for
the architecture overwrite that tautschnig traced in review of
[model-checking/kani#4913](https://github.com/model-checking/kani/pull/4913). The parent record is
[`notes/kani-upstream.md`](../../../notes/kani-upstream.md).

## 1. What our riscv64 proofs are checked against today

**The riscv64 kernel row is checked against the host's machine model, not riscv64's.** Measured on
patagonia (Apple Silicon, macOS) on 2026-10-03 (UTC), with the cached build of
`patches/kani-0.67.0-riscv64-target.patch` and its CBMC 6.8.0. All seven harnesses of the
`kernel` row for `riscv64gc-unknown-linux-gnu` were run with `--keep-temps`, and the
`__CPROVER_architecture_*` symbols of each linked `.out` were read with `goto-instrument
--show-symbol-table`:

| symbol | Kani writes (`.symtab.out`) | CBMC checks (linked `.out`) |
|---|---|---|
| `arch` | `"riscv64"` | `"arm64"` |
| `os` | absent | `"macos"` |
| `char_is_unsigned` | 1 | 0 |
| `long_double_width` | 128 | 64 |
| pointer, `int`, `long`, `wchar_t` widths, endianness, alignment, `NULL_is_zero` | riscv64 LP64D | equal values |

CBMC agrees: every harness logs `Adding CPROVER library (arm64)`. The cause is the link order in
the patched Kani, the same as upstream's. `link_goto_binary` passes `kani_lib.c` to `goto-cc` as C
source after the Rust symbol table. `goto-cc` compiles it with the host's configuration, and that
configuration overwrites the one Kani wrote.

**Does it change a verdict? Not for any harness we have.** The same seven harnesses were run again
through a measurement shim that hands `goto-cc` a precompiled `kani_lib.c` object after the Rust
inputs. Every linked `.out` then reads `"riscv64"`, and CBMC logs `Adding CPROVER library
(riscv64)`. For each harness the set of checks, each check's status, and the SAT variable and
clause counts were identical under both models. The only difference was one program step per
harness, not traced; the extra `os` symbol's initialiser is the likely one. So for today's harnesses the overwrite
is cosmetic, measured rather than argued.

That result does not generalise on its own. The fields that differ only matter where CBMC
interprets C. That means the CPROVER library that `--add-library` compiles with the active
configuration, `kani_lib.c`, and any `extern "C"` model a harness reaches. Kani's Rust codegen
writes explicit bit widths and signedness, which the equal rows above confirm. The list of CBMC
consumers of these fields is from memory of `config.cpp` and `ansi-c`, not re-read for this note.
A future harness that reaches a C library model taking `char` or `long double` could verify
differently, and nothing would say so.

**CI, unverified.** The `prove the kernel on riscv64` job runs on `ubuntu-24.04-arm`, an aarch64
Linux host. By the same link order its linked model should read `arch` `"arm64"` and `os`
`"linux"`. aarch64 Linux has unsigned `char` and a 128-bit `long double`, so it should differ
from riscv64 only in those two names. This is reasoned from the macOS measurement and has not been
observed on the runner. The gate in section 2 confirms or refutes it on its first CI run.

## 2. The gate

`script/verify-riscv64` already refuses a run whose goto output did not land under the riscv64
triple. Extend that check: after the run, read `__CPROVER_architecture_arch` from every linked
`.out` newer than the run's marker, with the `goto-instrument` it already installs, and fail
unless every one reads `"riscv64"`. Today that gate fails on every harness. That is the point:
it can come back red, and it has to go green only because of the fix below.

## 3. The fix

Precompile `kani_lib.c` to a goto object once, and pass it to `goto-cc` after the Rust inputs in
`link_goto_binary`. When `goto-cc` links goto binaries, the first input's architecture symbols win,
so the model Kani wrote survives. The shim in section 1 is this fix, applied from outside.

- In tree: change `patches/kani-0.67.0-riscv64-target.patch`. Editing the patch rebuilds the
  cached Kani in CI once, which is the cost.
- Upstream: the follow-up pull request promised on #4913, in
  [the reply to tautschnig](https://github.com/model-checking/kani/pull/4913#discussion_r4172069063).
  It changes the linked model for host runs too, which is why it is separate from #4913. When it
  ships in a Kani release, the in-tree patch loses this hunk with the rest of the patch.

## 4. The fatal risks it bears on

[Fatal risk 2](../../fatal-risks/README.md) ("The proofs prove trivia, and the real bugs live where
Kani cannot reach") is the direct one. Its reach argument counts the `arch/` subtrees that
milestone 589 (Kani can prove riscv64 from the hosts we already have) made provable. Section 1 is
a fact for that record: the riscv64 harnesses run against the host's C machine model. For the
seven harnesses that exist, no verdict depends on it, so it narrows no claim the entry makes
today. Without the gate it is a soundness gap in waiting rather than a hole.

Fatal risk 3 ("The tests do not test anything, and the quality is illusory") is measured by
mutation testing over the host crates, which this does not touch. It bears on 3 only through
principle 2's sentence that the proofs are among the gates that make the method work.
