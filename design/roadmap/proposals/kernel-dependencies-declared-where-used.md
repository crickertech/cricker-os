---
status: PROPOSED
raised: 2026-10-02
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The kernel declares each dependency where it is used

A promoted block would carry the provisional number 625; the integrator mints it, and 624 was the
highest claimed when this lane cut its branch from `40ac2d5f0`. The title and slug are drafts. Raised 2026-10-02 (UTC) by
the maintainer. It is a proposal; it changes the shape of the kernel's dependency graph, which is why it is a
proposal for calef and not a change.

## Evidence

`nightly-2026-10-02`'s cargo adds a warning-only `unused_dependencies` lint, and PR #1474 (the
toolchain bump) found it firing. A dependency in the unconditional `[dependencies]` table is
compiled for every target, so any target whose source never names it warns. Nothing fails today.
Two kernel dependencies are in that state; `grep -rn` under `kernel/src` gives their users.

| Dependency | Used by | Warns on |
|---|---|---|
| `tock-registers` | aarch64 only: `drivers/{pl011,gic,gicv3}.rs`, `arch/aarch64/*`, and three `cfg(target_arch = "aarch64")` blocks in `lib.rs`. `ns16550.rs` names it only in a comment. | riscv64 and x86_64 |
| `memory_corruption_canary_gate` | All three architectures, in `sched.rs`'s `canary` module, which is `cfg(not(feature = "bench"))`. The bench build swaps in a no-op twin. | Any `--features bench` build, on every architecture |

`calendar` had the same problem and is fixed on PR #1370, which moved it to
`[target.'cfg(target_arch = "x86_64")'.dependencies]`. That is the pattern for an architecture
split. `aarch64-cpu` and `boot_slot` already follow it.

## Proposed shape

1. `tock-registers = "0.10"` moves into the existing
   `[target.'cfg(target_arch = "aarch64")'.dependencies]` table, beside `aarch64-cpu`, whose comment
   there already records why that table exists. The two are used by exactly the same files.
2. `memory_corruption_canary_gate` is a feature split, not a target split, so a target table does
   not fit. It becomes `optional = true`, and a new feature `corruption_canary` (name provisional)
   enables it with `dep:memory_corruption_canary_gate`. The feature is in `default`, and the bench
   build is the one that builds without it. `sched.rs` changes its two `cfg(feature = "bench")` and
   `cfg(not(feature = "bench"))` gates on `canary` to `corruption_canary`, so the code and the
   manifest name one switch. The alternative, an `allow` on the lint, hides the fact and is refused.

The second item needs a ruling on a point of fact first: `kernel/Cargo.toml` has no `default`
feature today, and every consumer (`xtask`, `system_tests`, the QEMU runners) would have to say
which they want. If that cost is larger than the warning, the fallback is to leave this one
dependency alone and record the warning in a `BUGS` entry beside the dependency, marked as an
exception.

## Not changed

No dependency is added or removed. No version, path or crate changes. Only the table each one is
declared in, and one feature name. The syscall surface, the ABI and the compiled code for any
currently built target are unchanged.

## Risk and proof

- A missed use turns a warning into a build error on one architecture. That is the failure mode,
  and it is loud: `cargo xtask` builds all three, so the gate is the same one that proves
  architectural parity, §19 (architectural parity is a tenet).
- `Cargo.lock` should not change. A diff there means the move did more than relocate.
- Proof, in order. First, `cargo tree -p kernel --target <triple>` for each of the three targets,
  before and after; only the `tock-registers` edge may leave riscv64 and x86_64. Second, a clean
  build of each architecture and of the bench build on the new nightly, with no
  `unused_dependencies` warning from the kernel. Third, the ordinary CI gate and `script/lint`,
  unchanged.
- Reversible by reverting one manifest edit and one `cfg` rename; nobody else depends on the
  placement.

## Blocked by

PR #1474 (`nightly-2026-10-02`). The lint exists only on that nightly, so before it lands there is
nothing to observe and no gate that would prove the fix. Calef's ruling on the feature split is the
other gate.
