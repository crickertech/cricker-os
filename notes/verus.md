# Verus, and whether it reaches the code Kani stops at

*Provisional name (`notes/verus.md`), written 2026-09-20 by the `maintainer/verus-versus-kani` lane.
Verus appears nowhere else in this tree except in one line of `script/verify`'s own header, which
says the script was named so that **"swapping Kani for Verus someday would cost nothing"**. This note
is the first attempt to price that someday.*

**Read this for the evidence, not for a verdict.** Adopting a verifier is a dependency and a
methodology decision, which AGENTS.md puts in calef's hands, and the section that would record it is
the maintainer's to mint.

## What was read and run, and when

| source | what it is | read or run |
|---|---|---|
| Chen, Li, Mesicek, Narayanan, Burtsev. *Atmosphere: Towards Practical Verified Kernels in Rust.* KISV '23, Oct 23 2023, 8 pages. DOI 10.1145/3625275.3625401. | the first Atmosphere paper | 2026-09-20, full text from `https://mars-research.github.io/doc/2023-kisv-atmo.pdf` |
| Chen, Li, Zhang, Narayanan, Burtsev. *Atmosphere: Practical Verified Kernels with Rust and Verus.* SOSP '25, Oct 13-16 2025. DOI 10.1145/3731569.3764821. | the grown-up one, CC-BY | 2026-09-20, from `https://mars-research.github.io/doc/2025-sosp-atmo.pdf`. **That copy is 10 pages and the ACM reference block says 16**; it ends at Section 5 with no evaluation, no related work and no bibliography. `dl.acm.org` returns 403 to an automated fetch of both the PDF and the full-text HTML. |
| `notes/redleaf.md` (branch `maintainer/redleaf-comparison`) | the sibling lane, which read the full text | 2026-09-20, for Table 3 only, and attributed as second-hand below |
| Lattuada, Hance, Cho, Brun, Subasinghe, Zhou, Howell, Parno, Hawblitzel. *Verus: Verifying Rust Programs using Linear Ghost Types.* OOPSLA 2023, 7(OOPSLA1):286-315. | the tool's own paper | 2026-09-20, from `https://matthias-brun.ch/assets/publications/verus_oopsla2023.pdf` |
| The Verus guide, reference, `vstd` docs, and the `verus-lang/verus` source | what the project says about itself | 2026-09-20, `https://verus-lang.github.io/verus/guide/` and the GitHub tree |
| Verus release `0.2026.09.20.aef82ed`, `verus-arm64-macos` | the tool itself | 2026-09-20, downloaded, installed and run on this Mac. Every Verus error message quoted below is output this lane produced. |
| `cargo kani` 0.67.0, in this worktree, against `kernel/src` | the tool we have | 2026-09-20, three throwaway harnesses, reverted. Every Kani message quoted below is output this lane produced. |

`script/citations` validates in-tree numbered records and cannot check any of the external half, so
the discipline is the author's alone, which is why this table exists. Nothing here is recalled.

## The premise this lane was given is out of date, and that is the first finding

The brief quoted `design/fatal-risks/README.md` risk 2 (the proofs prove trivia, and the real bugs live
where Kani cannot reach):

> **The cause is one line of `script/verify`'s own header**, verified rather than inferred:
> *"`cargo kani -p <crate>` never compiles the kernel, the user programs, or xtask."* So **64,818
> lines of `kernel/src` are out of reach by construction**.

**That has not been true since 2026-08-30.** Milestone 193 (put `kernel/src` within reach of the
prover, because today the proofs cannot see it) opened the door for four changes, three of them
one-line `cfg`s; milestone 255 (a quarter of `kernel/src/arch/` has no assembly in it, and none of it
is proved) put two harnesses inside `arch/aarch64/iommu.rs`; milestone 304 (`cargo kani -p kernel`
only ever compiled one architecture, and it was the runner's) put two inside `arch/x86_64/irq.rs` and
added an x86_64 verify job. `notes/kernel-proofs.md` is the record, and this lane confirmed it runs.

So the question is not *"can Kani reach `kernel/src`"*. It is **"what is left after milestone 193,
and does Verus reach that residue"**, a narrower question, and the residue is where Verus turns out
to stop too.

## What Kani actually stops at, measured rather than described

Three throwaway harnesses, added to `kernel/src`, run, and reverted. All three outputs are verbatim.

### 1. `asm!` is a loud refusal, and it reaches through a dependency

A harness calling `arch::aarch64::timer::rearm`, the re-arm drift from milestone 6 (threads, the
context switch, and preemption) that `notes/kernel-proofs.md` nominates as the first property worth
trying:

```
Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani. Please post your
example at https://github.com/model-checking/kani/issues/2
 File: ".../aarch64-cpu-11.2.0/src/registers/macros.rs", line 18, in
 <aarch64_cpu::registers::cntvct_el0::Reg as aarch64_cpu::registers::Readable>::get

VERIFICATION:- FAILED
** WARNING: A Rust construct that is not currently supported by Kani was found to be reachable.
```

Two things worth having in writing. The `asm!` Kani tripped over is **in a dependency**
(`aarch64-cpu`), not in this tree, so the boundary is the call graph and not the repository. And the
run reported **1 of 42 failed (41 undetermined)**: everything downstream of the unsupported construct
goes `UNDETERMINED` rather than silently `SUCCESS`. This is the good direction, and
`notes/kernel-proofs.md` already says so.

### 2. MMIO is also a loud refusal, which that note words more softly than the tool behaves

A harness doing a `read_volatile` at `0x0800_0000` (QEMU `virt`'s GICv2 distributor):

```
Failed Checks: dereference failure: pointer NULL
Failed Checks: dereference failure: pointer outside object bounds
Failed Checks: dereference failure: invalid integer address
 File: ".../library/core/src/ptr/mod.rs", line 2082, in core::ptr::read_volatile::<u32>
VERIFICATION:- FAILED
```

`notes/kernel-proofs.md` stub item 6 says MMIO addresses "are raw pointers to nothing under a model
checker", which reads as though a harness might quietly reason about a fiction. It does not: CBMC has
no object at that address and says `invalid integer address` three ways. Worth correcting there.

### 3. The silent one is the architecture `cfg`, and it is silent exactly as documented

A deliberately false harness (`assert!(1 + 1 == 3)`) added under `arch/riscv64/`, on this aarch64 Mac:

```
error: no harnesses matched the harness filter: `the_riscv_scratch_probe`
```

Nothing compiled it, so nothing could fail. That is stub item 3, reproduced. One correction to that
note's `BUGS`: `--ignore-global-asm` is **not** entirely undetected. Every run prints

```
warning: Ignoring global ASM in crate kernel. Verification results may be impacted.
```

which is per-run rather than per-item, so the `BUGS` entry's substance stands and its wording
("nothing detects that") is a little too strong.

### How large the residue actually is

Measured in this worktree on 2026-09-20, by walking `kernel/src` in Python:

| | lines |
|---|---|
| `kernel/src`, all `.rs` | 81,413 |
| of those, non-blank non-comment ("code") | 40,953 |
| `kernel/src/arch/` | 20,907 |
| lines in files containing an `asm!` call | **15,001**, and **every one of them is under `arch/`** |
| `arch/aarch64` / `arch/riscv64` / `arch/x86_64` | 6,052 / 5,766 / 8,953, of which 4,688 / 4,974 / 5,339 are in `asm!`-bearing files |

So **DECISIONS §4 (kernel shape: monolithic, deferred, with two cheap rules) rule 1,
architecture-specific code under `kernel/src/arch/`, has kept the construct-level residue to at most
18% of the kernel, and it is exactly the 18% you would expect.**
File granularity overstates it, because Kani refuses on the call graph rather than on the file. It is
still the right order of magnitude, and it is still where the timer re-arm drift lives. (This
sentence used to add "and the VisionFive 2 undelivered wake." That reading is **retracted**,
`notes/visionfive2.md`'s fifth bench stop, 2026-08-15: it was a completed tour's terminal state, not
a stranded receiver, and never happened. Found still repeating it here 2026-09-23.)

The 64,818 in risk 2 is also stale as a size: `kernel/src` is 81,413 lines today.

## What Verus is

**A rustc driver against a forked compiler, not a separate tool.** The OOPSLA paper, Section 7: *"We forked
the Rust compiler to introduce additional hooks and typechecking rules. We then implemented Verus as
a separate 'driver' that links against the Rust compiler."* Its `rust-toolchain.toml` asks for
`rustc-dev` and `llvm-tools`, which is the private-API build. The release ships a `verus` binary plus
that `rust_verify` driver, and it demands one exact toolchain:

```
verus: required rust toolchain 1.98.1-aarch64-apple-darwin not found
```

**Stable 1.98.1, pinned to the patch version.** This tree pins a nightly in `rust-toolchain.toml`
and uses nightly features (`custom_test_frameworks` among them, per AGENTS.md). Those are two
different compilers, and the verified crate has to build under the verifier's.

Releases are dated rather than numbered: the one this lane ran is `release/0.2026.09.20.aef82ed`,
published the day it was used, and the GitHub releases are marked pre-release. **There is no 1.0**,
and the project says so itself. `README.md`: *"Features may be broken and/or missing, and the
documentation is still incomplete."* The guide's feature table (last updated 2026-05-13): *"Note that
Verus is in active development. If a feature is unsupported, it might be genuinely hard, or it might
just be low priority."* And its `project-goals.md` lists among non-goals *"verify the verifier
itself"* and *"support all Rust features and libraries (instead, we will focus a subset that is easy
to verify)"*.

Compilation is a second step that the same tool can do: `cargo verus verify` checks without
producing a binary, `cargo verus build` *"Verifies all opted-in crates and compiles them to native
artifacts"*, and Verus-annotated code *"can also be built with a normal `cargo build` command"* once
the ghost code is erased.

### Deductive, not bounded, and that is the whole difference from Kani

Kani compiles a crate to a CBMC goto-binary and asks an SMT solver whether any input within a bound
reaches a failing assertion. Loops need unwind bounds; `notes/verification.md` and
`notes/user-proofs.md` are full of the consequences (a bound on a sum of 32 symbolic values that
finished on neither CaDiCaL nor Z3; a symbolic index into a 3.5 KB struct that ran out of memory; the
64-bit division in the calendar costing more than the logic around it).

Verus asks a different question. The human writes `requires`, `ensures`, `invariant` and `decreases`,
and Verus discharges the verification conditions in Z3. **There is no unwind bound**: a loop is
handled by the invariant the human supplies, and termination by the `decreases` clause the human
supplies. The OOPSLA paper, Section 2: *"Given preconditions, postconditions, and loop invariants, Verus
uses standard weakest precondition reasoning [Dijkstra 1975] to generate a verification condition for
`fibo_impl`. It then sends this verification condition to the Z3 SMT solver."* A loop invariant is
checked once, inductively, and holds for every iteration count; a bound on iterations never appears.
SOSP '25 Section 2: *"Verus translates the Rust program into SMT expressions, which are then
passed to a solver (Z3 in case of Verus)."* And: *"To improve verification speed, Verus leverages the
idea that it is possible to use the type system, and specifically the linear type system of Rust, to
reason about aliasing and memory instead of SMT solving. This reduces the complexity of the SMT
queries by orders of magnitude."*

**The trade is exactly the one that decides this whole question.** Kani's harnesses cost almost
nothing to write and are bounded by what the solver can chew. Verus's proofs are unbounded and cost
the specification. The tree's Kani harnesses are typically twenty lines against code nobody
annotated; Atmosphere paid 20.1K lines of proof for 6K lines of kernel.

### The human's side of the contract

From SOSP '25 Sections 2 and 4, and confirmed by running it here:

- **`spec fn`** defines the abstract model; **`proof fn`** carries the argument; **ghost** and
  **tracked** variables exist at proof time and are erased. *"Specifications and proofs (ghost code)
  are erased by Verus using the Rust procedural macro during the compilation time, and thus do not
  incur any runtime overhead."*
- **Verified code is a dialect.** *"Executable code is written in a subset of Rust, while
  specifications and proofs are written in a functional extension of Rust."* Everything to be
  verified lives inside a `verus!{ ... }` macro.
- **`int` and `nat` are not `i64` and `u64`,** and the first thing this lane wrote failed on it:
  `expected 'nat', found 'int'` for `va + (count - 1) * 4096`, because subtraction leaves `nat`. That
  is representative of the annotation burden: the specification is written in mathematics and the
  code in machine words, and the human closes the distance.
- **Raw pointers need permissions.** `PPtr<T>` is the pointer and `PointsTo<T>` is a linear ghost
  token granting access. SOSP '25 Section 2: *"To use a raw pointer in Verus, the code must prove that it
  possesses the corresponding tracked permission to the pointer, and the permission is
  initialized."*
- **Mutation through `&mut` is barely supported.** SOSP '25 Section 5, TCB item 7: *"Verus currently has
  very limited support for `&mut`. This adds 300 lines of executable code with 1900 lines of
  specifications."* KISV '23 says the same from the other side: *"At the moment, Verus does not
  support returning mutable references from functions and support for mutable references in function
  arguments is limited to special cases."*

### Concurrency, and the one sentence that matters most to this tree

Verus has real concurrency machinery: `tracked` ghost state, `LocalInvariant` and `AtomicInvariant`
containers opened around an operation, `atomic_ghost` pairing a hardware atomic with a ghost token,
and the VerusSync / tokenized-state-machine method for proving an inductive invariant over shared
ghost state rather than enumerating interleavings. `vstd::invariant`'s own doc comment states the
price: an `AtomicInvariant` *"can be only opened for the duration of a single sequentially consistent
atomic operation"*, and the block *"cannot contain any exec-mode code with the exception of a single
atomic operation"*.

**And there it is: sequentially consistent.** The OOPSLA paper, Section 9, comparing itself to RustBelt:

> RustBelt can also handle atomics with relaxed memory ordering [Dang et al. 2020], which Verus does
> not support.

`vstd::atomic`'s wrappers hard-code `Ordering::SeqCst` on every load, store, swap and
`compare_exchange`; the subagent that read that file found no `Relaxed`, `Acquire`, `Release` or
`AcqRel` in it.

**DECISIONS §4 (kernel shape: monolithic, deferred, with two cheap rules) rule 4, assume weak memory
ordering, is this project's fourth codebase rule, and its
stated reason is that being ARM-first is a gift because "we cannot develop hidden strong-ordering
assumptions the way an x86-first project would."** A verifier whose concurrency model is sequential
consistency would hand back exactly the assumption this tree wrote a rule to avoid acquiring. That
does not make Verus useless for concurrency here, and milestone 80 (loom: the hand-rolled atomic
protocols, model-checked) already points a tool at interleavings here. It does mean **a green Verus
proof over a `Relaxed` atomic in this kernel would not mean what a reader would take it**, and is the
sharpest nife-specific caveat in the note after the hardware boundary.

### What of this tree's Rust would not parse

The guide's feature table puts in **"not supported"**: `async`/`await`, function pointer types,
`transmute`, **hardware intrinsics**, printing and I/O, `Debug` derives, **user-defined `Drop`**, and
the standard `Mutex`/`RwLock` (vstd offers verified alternatives). **"Partially supported"** includes
`as` casts, `const` generics, `for` loops, trait objects (`dyn`), `impl` types, iterators, raw
pointers, closures (*"no mutable captures"*), and multi-crate projects.

This lane did not test any of it against nife code, and several of those are load-bearing here: the
capability lifecycle uses `Drop`, the driver layer is generic over traits, and `as` casts are
everywhere in page-table arithmetic. **"Hardware intrinsics: not supported"** is the same boundary as
`asm!` arriving by another door.

### What a pass means, and what it does not

A worked example this lane ran, which is the single most important thing in this note:

```rust
#[verifier::external_body]
pub fn read_counter() -> (r: u64)
    ensures r > 0,
{
    let v: u64;
    unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) v) };
    v
}

pub fn nonzero_counter() -> (r: u64) ensures r >= 1 { read_counter() }
```

```
verification results:: 1 verified, 0 errors
```

**`#[verifier::external_body]` makes a postcondition an axiom.** Verus checked nothing inside that
function, took `r > 0` on trust, and used it to discharge the caller. The claim happens to be *false*
(`CNTVCT_EL0` reads zero at reset, which is how this project learned QEMU does not pass a device tree
pointer in `x0`), and the run is green and silent about it. There is no warning, no count of trusted
items, nothing in the exit status.

That is the mirror image of Kani's behaviour. Kani refuses loudly and proves nothing; Verus proves
whatever you assert and says nothing. **Neither reaches the hardware. One tells you so.**

The project is honest about the mechanism. Its `tcb.md` enumerates how assumptions enter: an `assume`
statement, *"any proof function introduced with `#[verifier::external_body]`"*, *"any exec function
introduced with `#[verifier::external_body]` or `#[verifier::external_fn_specification]`"*, and
`#[verifier::external]`. `assume_specification`, which attaches a contract to an
already-compiled function, carries its own warning: *"assume_specification statement is unchecked, it
can easily be used to subvert Verus's guarantees."* And of `assume` itself: *"successful verification
using assume provides no guarantees about program correctness."* A `--no-cheating` flag forbids
`assume` outright.

**But `external_body` is the mechanism a kernel needs, and nothing forbids or counts it**, which is
why Atmosphere's TCB section had to be written by hand. `notes/kernel-proofs.md`'s enumerated stub
boundary is the same discipline, arrived at independently: *"a proof with an unexamined stub is worse
than no proof, because it reads as coverage."* If Verus were ever used here, that enumeration stops
being good practice and becomes the only thing standing between a green run and a fiction.

## Does Verus reach `asm!` and MMIO? No, and it says so

Both probes run by this lane, inside `verus!{}`:

```
error: The verifier does not yet support the following Rust feature: inline-asm expressions
 --> inside.rs:7:14
  |
7 |     unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) v) };
```

```
error: Verus does not support this cast: `usize` to `*const u32`
 --> inside2.rs:5:39
  |
5 |     unsafe { core::ptr::read_volatile(0x0800_0000usize as *const u32) }
```

**`asm!` is unsupported and an integer-to-pointer cast is unsupported**, and the second is the MMIO
idiom, not an incidental one: every fixed-address device register in this kernel is reached that way.
Outside the `verus!{}` macro, or under `external_body`, both compile and are simply not verified.

`no_std` works: the `run_end_va`-shaped function above is `#![no_std]`, verified, and took **0.83
seconds** of wall clock on this Mac.

### And the paper draws the boundary in the same place, by hand

This is the finding that answers the brief's question. SOSP '25 Section 5, its Trusted Computing Base section, item 8,
verbatim:

> Sequences of assembly and trusted Rust code. The fragments of assembly code (172 lines) implement
> entry and exit trampolines for system call and interrupt handlers. Trusted Rust code (total of
> around 3,000 lines) implementing hardware interface to configure IOMMU (465 lines), setup execution
> environment of the kernel (321 lines), initialize interrupt handling (293 lines), advanced
> programmable interrupt controller (287 lines), interrupt descriptor table (179 lines), initialize
> task state segment (TSS) and global descriptor table (172 lines), fast system call entry via
> sysenter (123 lines), initialize per-CPU data structures and application processes, etc.

**Read that list against this tree's `arch/x86_64/`.** IOMMU configuration, interrupt setup, the
APIC, the IDT, the GDT, the TSS, the syscall entry: it is the same file list, and in Atmosphere it is
**trusted, not verified**. A 6K-line verified kernel carries 3,172 lines of hand-trusted hardware
code beside it, and that trusted code is more than half the size of the verified part.

The same section is candid about the rest of the TCB: the Verus frontend, Z3 (*"any unsoundness in it
will compromise the soundness of verification"*), the Rust compiler and toolchain, the `core`
library, specifications of `core` routines, and *"Axioms missing in Verus... These axioms can be
verified in the future"* (around 700 lines of spec). KISV '23 adds one more that matters to a kernel:
*"Verus cannot guarantee the absence of stack overflows since it does not model the hardware and
relies on Rust to correctly abstract details of the machine executing the code."*

**So Verus does not reach the code Kani cannot reach.** It reaches *around* it, by letting a human
write down what the hardware seam promises and then proving the rest against that promise. Kani
cannot do that, and that difference is real and is the honest case for Verus. But it is a different
claim from "the prover reaches the driver", and the timer re-arm drift, which lives in that seam,
would have been on the trusted side of an Atmosphere-shaped boundary too. (This sentence named the
VisionFive 2 undelivered wake until 2026-09-24; that reading is retracted, as the residue section
above records.)

## The number that decides it: proof effort

All figures are quoted, with the paper each came from. The two papers disagree, because the second
supersedes the first.

| | KISV '23 | SOSP '25 |
|---|---|---|
| executable code | not stated | **6K lines** |
| proof code | not stated | **20.1K lines** (14.3K specification + 5.8K hints), of which 2.9K spec is the top-level abstract system-call specification |
| proof-to-code ratio | **7.5:1** | **3.32:1** |
| verification wall clock | *"Verus takes roughly 30 minutes to finish the proof and spends up to 850 seconds at most on a function"* (runtime checks off) | *"completes verification in less than 20 seconds on a modern laptop"* |
| total effort | *"less than 2.5 person-years, with only 1.5 years spent on verification"* | *"less than one and a half physical years and an effort of roughly two and a half person-years. But only one and a half person-years was spent on the development of the verified parts (we used the second person-year on unverified parts such as the boot and initialization infrastructure, user-level device drivers, application benchmarks, and the build environment)"* |

The 30-minutes-to-20-seconds swing over two years is the single most encouraging number in either
paper, and SOSP '25 draws the conclusion itself: *"it takes less time to finish verification than
compiling the kernel, which enables a truly interactive development cycle with a verifier as opposed
to traditional recompile, reboot, and run tests approach."*

Their own comparison table (KISV '23, Figure 2), for context on what that 3.32:1 is beating:

| system | language | spec language | proof-to-code |
|---|---|---|---|
| seL4 | C+Asm | Isabelle/HOL | 20:1 |
| CertiKOS | C+Asm | Coq | 14.9:1 |
| SeKVM | C+Asm | Coq | 6.9:1 |
| Ironclad | Dafny | Dafny | 4.8:1 |
| NrOS | Rust | Verus (Rust eDSL) | 10:1 |
| Atmosphere | Rust | Verus (Rust eDSL) | 7.5:1 (3.32:1 by SOSP '25) |

**That table disagrees with its own paper's prose**, which says Atmosphere's 7.5:1 beats *"SeL4 and
CertiKOS, which have proof-to-code ratio of 19:1 and 20:1, respectively"*, while the table gives 20:1
and 14.9:1. Both appear in KISV '23 and it does not reconcile them. Flagged rather than resolved.

**A second Verus data point, at a much smaller scale.** The OOPSLA paper's Table 1 gives per-example
spec, proof and exec lines with wall-clock verification times: a verified `RwLock` is 200 spec + 80
proof against 145 exec (1.9:1) in 4.44 seconds; an XOR doubly-linked list is 116 + 118 against 151
(1.6:1) in 5.03 seconds; a FIFO queue 220 + 119 against 138 (2.5:1) in 4.58 seconds. Those are
library-sized rather than kernel-sized, and the ratios are *better* than Atmosphere's, which is what
you would expect from a smaller unit with a simpler specification. They are the closest thing here to
a price for one subsystem.

And seL4's own cost, as SOSP '25 states it: *"development of the first formally verified microkernel,
seL4, required 11 person-years (an additional 9 person-years were needed for the development of
formal language frameworks, proof tools, and libraries)"*. KISV '23 puts the same thing as *"200,000
lines of proof code in the Isabelle/HOL theorem prover for 8,700 lines of C and required 22
person-years"*.

### Performance, second-hand

This lane could not read the evaluation section. `notes/redleaf.md` (branch
`maintainer/redleaf-comparison`, 2026-09-20) reports SOSP '25 Table 3 from the full text, on CloudLab
c220g5 (two Intel Xeon Silver 4114 10-core at 2.20 GHz): call/reply **1,058 cycles for Atmosphere
against seL4's 1,026**, and map-a-page **1,984 against 2,650**. Treated here as second-hand and
attributed; that note is the record for it.

## So: would Verus reach the 64,818 lines?

**The number is 81,413 today and the honest answer is in four parts.**

**1. Most of it is already reachable by Kani, so the question has changed.** Since milestone 193 the
prover compiles `kernel/src`. What stops a harness is a call graph reaching `asm!` (loud), an
integer-to-pointer MMIO read (loud), or an architecture the host does not compile (silent). Verus
refuses two of those three by construction and has the third problem in a different form. **On
reachability of the hardware seam, Verus and Kani are the same tool.** The six kernel harnesses in
this tree are six because nobody has written more, not because Kani cannot see the code.

**2. Where Verus is genuinely stronger is unboundedness, and this tree has the receipts for why that
matters.** Every wall `notes/verification.md` and `notes/user-proofs.md` record is a bounded-model-
checking wall: unwind bounds, a symbolic index into a 3.5 KB struct, counting, division chains. None
of those is a Verus wall. A loop invariant costs a human an afternoon and costs the solver nothing,
and `run_end_va`'s Verus version verified here in 0.83 seconds against a 42-minute `script/verify`.
**If proofs about `kernel/src` ever stop because the solver ran out of memory rather than because the
code touched hardware, that is the moment this question becomes urgent.** It has not happened yet:
the four kernel harnesses cost five seconds, almost all of it compiling.

**3. The cost, in this project's units.** Atmosphere verified 6K executable lines in **1.5
person-years** at **3.32 lines of proof per line of code**. `kernel/src` is **40,953 code lines**,
which is **6.8x** Atmosphere's kernel. A linear extrapolation gives roughly **136,000 lines of proof
and ten person-years**, and linear is the optimistic reading for three reasons the papers state
plainly:

- **Atmosphere was designed for its proof and nife was not.** Flat permission storage (*"push it to
  the extreme arguing that it is a key design choice critical for the scalability of the proof"*),
  manual memory management chosen so that memory is reasoned about explicitly, and a **big lock**:
  *"Atmosphere is a multiprocessor system, but to simplify verification we rely on a big-lock
  synchronization, i.e., all interrupts and system calls execute in the microkernel under one global
  lock and with further interrupts disabled."* This tree runs SMP with fine-grained locking and
  DECISIONS §4 (kernel shape: monolithic, deferred, with two cheap rules) rule 4, assume weak memory
  ordering, as a tenet. Atmosphere's design bought its ratio.
- **One architecture.** The SOSP '25 TCB list is x86_64 throughout. nife has three, and DECISIONS
  §19 (architectural parity is a tenet; the targets are aarch64, riscv64, and x86_64) makes that a
  gate rather than a choice. Verus
  would have the same one-architecture problem Kani has, for the same `cfg` reason.
- **Concurrency is named as future work by the paper itself**: *"I/O intensive shared device drivers
  will require proof about concurrent code, which we leave as future work."* Five of the eighteen
  defects in `notes/proof-retrospective.md`'s corpus are concurrency. **And Verus's concurrency model
  is sequentially consistent**, which the section above argues is the assumption DECISIONS §4's rule 4
  exists to keep this project from acquiring. So the defect class the prover misses most is the class
  where adopting Verus would buy the least and cost the most in false confidence.

Ten-plus person-years against one architect and a lane budget is not a schedule. **Whole-kernel
functional correctness in Verus is not available to this project at this size.** That is the answer,
and it is a no.

**4. A subset is a different question and is not obviously a no.** The papers' unit of work is a
subsystem with a written-down abstract state, and the ratio that matters for a first attempt is not
3.32:1 across a kernel but whatever one property costs. This lane wrote one, from a cold start,
knowing no Verus, in a scratch file: a `requires`/`ensures` pair for `run_end_va`'s exact shape,
`no_std`, verified in 0.83 seconds. It took one failed compile (`nat` versus `int`). That is not
evidence that a subsystem is cheap; it is evidence that the first hour is cheap, which is the only
thing a bounded experiment can establish and is exactly what one is for.

**The strongest single argument for looking harder is not a proof at all, it is the 20 seconds.** A
verifier that runs faster than the compiler changes what verification *is* in a workflow. This tree's
`script/verify` takes 42 minutes, runs on one heavy job at a time because a harness reached 3.5 GB,
is the merge queue's long pole, and needed an `--affected-since` mode to avoid paying for itself on
changes it could not possibly affect. None of that is a property of proving things; it is a property
of bounded model checking.

## What this does not settle

- **Nothing here compares soundness.** Both tools trust their solver; both have a TCB; this lane did
  not evaluate either one's.
- **Nothing here says the two are exclusive.** `script/verify`'s name was chosen for a swap, but the
  papers suggest a split (Verus where a specification is worth writing, Kani where panic-freedom over
  hostile input is the property) at least as strongly as a replacement. That is a design question and
  it is not this note's.
- **The tree's own residue is not priced.** What it would cost to write an abstract state for
  `kernel/src/sched.rs` is unknown and is the thing a bounded experiment would learn.

## EXAMPLES

Reproduce the `asm!` boundary in Kani. Append to `kernel/src/arch/aarch64/timer.rs`, run, then
`git checkout` the file:

```rust
#[cfg(kani)]
mod scratch_proofs {
    #[kani::proof]
    fn the_rearm_scratch_probe() {
        let interval: u64 = kani::any();
        kani::assume(interval > 0 && interval < 1_000_000);
        super::rearm(interval);
    }
}
```

```console
$ cargo kani -p kernel -Z unstable-options --ignore-global-asm \
      --harness the_rearm_scratch_probe
...
Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani.
VERIFICATION:- FAILED
```

Run Verus at all, from nothing, on this Mac (about 450 MB and one toolchain):

```console
$ curl -sL -o verus.zip https://github.com/verus-lang/verus/releases/download/\
release%2F0.2026.09.20.aef82ed/verus-0.2026.09.20.aef82ed-arm64-macos.zip
$ unzip -q verus.zip -d verus
$ rustup toolchain install 1.98.1-aarch64-apple-darwin
$ verus/verus-arm64-macos/verus probe.rs --crate-type=lib
verification results:: 1 verified, 0 errors
```

The probe, which is `run_end_va`'s property in `no_std` Verus:

```rust
#![no_std]
use vstd::prelude::*;
verus!{
pub open spec fn last_of(va: nat, count: nat) -> int { va + (count - 1) * 4096 }

pub fn run_end_va(va: u64, count: u64) -> (r: Option<u64>)
    requires count >= 1,
    ensures match r {
        Some(last) => last as int == last_of(va as nat, count as nat),
        None => va as nat + (count as nat - 1) * 4096 > u64::MAX as nat,
    }
{
    let pages = (count - 1) as u128 * 4096u128;
    let last = va as u128 + pages;
    if last > u64::MAX as u128 { None } else { Some(last as u64) }
}
}
```

Watch a false axiom pass, which is the caveat that matters most:

```console
$ cat seam.rs
#![no_std]
use vstd::prelude::*;
verus!{
#[verifier::external_body]
pub fn read_counter() -> (r: u64) ensures r > 0 {
    let v: u64;
    unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) v) };
    v
}
pub fn nonzero_counter() -> (r: u64) ensures r >= 1 { read_counter() }
}
$ verus/verus-arm64-macos/verus seam.rs --crate-type=lib
verification results:: 1 verified, 0 errors
```

`CNTVCT_EL0` is zero at reset. The green line does not know that.

## BUGS

- **The SOSP '25 copy this lane read is 10 pages of a 16-page paper.** It ends at Section 5 with no
  evaluation, no related work and no bibliography; `dl.acm.org` returns 403 to an automated fetch of
  both the PDF and the full-text HTML, and the paper is CC-BY, so a human with a browser can get it.
  **Everything in Section 6 onward is unread here**: the microbenchmarks, the comparison against seL4 and
  Linux, any scaling data for verification time against kernel size, and any statement of what the
  proofs failed to catch. The cycle counts above are second-hand from `notes/redleaf.md`.
- **No Verus *systems* paper other than Atmosphere was read.** The OOPSLA paper was, and its Table 1
  is library-scale. The KISV '23 comparison table gives NrOS as 10:1, but that figure is quoted from
  Atmosphere's table rather than from the NrOS paper, and this lane did not read Anvil, IronSync, the
  verified page-table work, or Microsoft's verified-storage. Those exist and would materially change
  the extrapolation in part 3; a reported 3.9:1 for Microsoft's verified log and roughly 4.5-7.4 for
  Anvil's controllers reached this lane only as search-result summaries, **were not read in the
  primary source, and are therefore not quoted above.**
- **The ten-person-year extrapolation is a ratio scaled linearly and nothing more.** It is not a
  measurement, it is not an estimate anyone with Verus experience produced, and the three reasons
  above say it is more likely to be low than high. Treat it as an order of magnitude that says "not
  this project at this size", which is all it can support.
- **Verus was run on scratch files, never on a nife crate.** Not one line of this tree was put
  through it. Whether `crates/capability` or `kernel/src/syscall.rs` would even parse under Verus's
  Rust subset is unknown: the subset's boundaries (traits, generics, closures, `dyn`, lifetime
  shapes) were not tested here at all, and this tree leans hard on all of them.
- **The build story after verification is quoted, not observed.** KISV '23 describes two passes (*"The
  build system first invokes the Verus toolchain on the verified components. Then a regular Rust
  toolchain is used to compile the entire kernel with ghost code erased"*), using *"the same Rust
  version that the Verus toolchain is based on"*. This lane never compiled a verified crate to a
  binary, never tried a custom target JSON, and never tried `aarch64-unknown-none-softfloat`. That is
  the first thing that would break and it is untested.
- **Verus's own concurrency story was not exercised.** `tracked`, `AtomicInvariant` and VerusSync are
  quoted from the papers and the `vstd` sources, not run. Given that five of
  `notes/proof-retrospective.md`'s eighteen defects are concurrency, this is the largest untested gap
  in the note, and SOSP '25 explicitly leaves concurrent drivers as future work. The
  sequential-consistency finding above is quoted from the OOPSLA paper and from a reading of
  `vstd::atomic`; **nobody here ran a concurrent Verus proof to see what it does with a `Relaxed`
  atomic**, and "not supported" could mean refused or could mean silently modelled as `SeqCst`. Those
  are very different, and this note does not know which.
- **Three things the Verus documentation does not state, found only in its source.** That `asm!` is
  rejected (`rust_to_vir_expr.rs`, `ExprKind::InlineAsm => unsupported_err!`); that `vstd` is
  `no_std`-capable (`#![cfg_attr(not(feature = "std"), no_std)]` and a default `std` feature); and
  anything at all about MMIO or a pointer at a fixed physical address. The first two this lane
  confirmed by running the tool. The third is unresolved in both directions: the docs neither permit
  nor forbid it, and the integer-to-pointer cast error above is this lane's only evidence.
- **Whether Verus builds for a bare-metal target was never tested and is not documented.** KISV '23
  asserts *"due to the lack of managed runtime, verified Rust code can be compiled and executed on
  bare metal"*, which is the authors' claim about their own x86_64 kernel, not a Verus project
  statement, and says nothing about a custom target JSON or `aarch64-unknown-none-softfloat`.
- **`design/fatal-risks/README.md` risk 2 and `notes/proof-retrospective.md` both said `kernel/src` was
  unreachable by construction** when this was written, three weeks stale. Risk 2 now says it in the
  past tense, its reach corrected 2026-09-24; the retrospective is a dated record of 2026-08-30 and
  reads as one.
- **This note names no winner and is not evidence for a decision either way.** It was written to
  price a question, and the pricing has a factor-of-several uncertainty that only running Verus on
  real code would remove.
