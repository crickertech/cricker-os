# RedLeaf, and the opposite bet about where isolation comes from

*Provisional name (`notes/redleaf.md`), written 2026-09-20 by the `maintainer/redleaf-comparison`
lane. RedLeaf is the closest academic relative this project has and it appeared nowhere in this
tree: not in `notes/prior-art.md`, not in a roadmap block, not in `design/decisions/`. Anyone who
knows the literature will make this comparison whether or not we have.*

*It is one note covering two papers, because they are one argument. Levy et al. (APSys '17) make
the case that a memory-safe language can replace hardware protection **inside** a kernel; RedLeaf
(OSDI '20) builds a whole system on that case. Reading either without the other gets the bet wrong.*

**Read this note for the mechanism, not for a verdict.** Where nife stands relative to RedLeaf, in
public, is a claim that leaves the machine, and AGENTS.md puts those in the irreversible category.
The evidence is laid out here and the sentence is deliberately not written.

## What was read, and when

| source | what it is | read |
|---|---|---|
| Levy, Campbell, Ghena, Pannuto, Dutta, Levis. *The Case for Writing a Kernel in Rust.* APSys '17, Mumbai, Sep 2 2017, 7 pages. DOI 10.1145/3124680.3124717. | the ancestor of the argument | 2026-09-20, full text from `https://patpannuto.com/pubs/levy17rustkernel.pdf` (the ACM DOI returns 403 to an automated fetch) |
| Narayanan, Huang, Detweiler, Appel, Li, Zellweger, Burtsev. *RedLeaf: Isolation and Communication in a Safe Operating System.* OSDI '20, Nov 4-6 2020, pp. 21-34. | the paper | 2026-09-20, full text from `https://www.usenix.org/system/files/osdi20-narayanan_vikram.pdf` |
| `https://github.com/mars-research/redleaf` | the implementation | 2026-09-20, depth-1 clone of `master` |
| Burtsev, Appel, Detweiler, Huang, Li, Narayanan, Zellweger. *Isolation in Rust: What is Missing?* PLOS '21, Oct 25 2021. | the authors' own retrospective on what RedLeaf cost them | 2026-09-20 |
| Chen, Li, Mesicek, Narayanan, Burtsev. *Atmosphere: Towards Practical Verified Kernels in Rust.* KISV '23, Oct 23 2023. | what the group did next | 2026-09-20 |
| Chen, Li, Zhang, Narayanan, Burtsev. *Atmosphere: Practical Verified Kernels with Rust and Verus.* SOSP '25, Oct 13-16 2025. | the same, grown up | 2026-09-20 |
| Heiser. *The seL4 Microkernel: An Introduction.* seL4 Foundation whitepaper, Revision 1.4.1 of 2026-07-17. | the reference point this project is measured against | 2026-09-20, for the TCB definition and size only |

Everything quoted below is quoted from one of those seven. Nothing here is recalled. `script/citations`
validates in-tree numbered records and cannot check any of this, so the discipline is the author's
alone, which is why the table exists.

## The ancestor: the 2017 case for writing a kernel in Rust

**This is the argument RedLeaf inherits, and it is worth reading first because it is narrower and
therefore clearer.** Levy et al. state it in one sentence: *"This paper takes a more extreme
approach: entirely throw away hardware protection within the kernel and, instead, write the kernel
in a memory-safe programming language."*

It positions against the obvious predecessors and rejects them on the runtime rather than the idea:
*"Both Spin and Singularity, however, use garbage-collected languages, which pose many problems for
kernels. Garbage collection complicates memory placement and layout, creates timing non-determinism
from background locks and introduces stop-the-world intervals that pause the entire OS."* A second
objection follows, and it is the one that sets up the whole contribution: *"Furthermore, both Spin
and Singularity depend on a large, unsafe code base: the Spin kernel and Singularity's runtime,
respectively."*

**So the contribution is a claim about how little must be trusted**, and it is made as two
enumerations rather than an argument. Four Rust-library abstractions that use `unsafe`: **bounds
checks**, **iterator optimizations**, **compiler intrinsics and primitive casts**, and **`Cell`**.
Six pieces of kernel code: **context switches**, **memory-mapped I/O and structures**, the **memory
allocator**, **userspace buffers**, **interrupt/exception handlers**, and **`TakeCell`**. Their own
framing: *"Together, they constitute the complete set of unsafe code in the kernel and are
surprisingly small, primarily consisting of mechanisms that any language or kernel needs to
provide."*

`TakeCell` is the one thing they had to invent, and the reason is a cost: *"Cell, unfortunately, is
only a partial solution. It imposes the significant cost of requiring memory copies, which is an
unacceptable overhead for complex or large kernel data structures."* `TakeCell` passes code in
through a closure instead of copying data out, its `map(f)` is a no-op when a reference already
exists (so it is *"a form of mutual exclusion ... unlike a mutex, it skips the operation instead of
blocking"*), and they show the generated ARM assembly to support the claim *"Once compiled, TakeCell
is just as fast as unchecked C code."*

**One line carries the paper's whole quantitative claim, and it is quoted here in full because
truncating it changes what it says.** *"The kernel's trusted computing base includes the Rust core
library as well as under 1000 lines out of over 6000 lines of kernel code."*

Two things about that sentence. It says **under** 1000, not "around" 1000. And the trusted computing
base it names is **not** 1000 lines: it is 1000 lines **plus the whole of `libcore`**, by the
sentence's own construction. The famous ratio is the *kernel-authored* part of the TCB, and a reader
who quotes "1000 of 6000" as the TCB size is dropping the larger and less auditable half.

**The paper never says "Tock."** The word does not appear in it; it says *"our kernel"* and *"a
kernel for low-power uniprocessors."* This is Tock's founding argument in substance, not in name.

### The three things the 2017 paper left open, and whether RedLeaf closed them

The paper is unusually clear about its own limits, which makes this answerable from the two documents
rather than by opinion.

**Multiprocessor: RedLeaf implements it and never evaluated it, so the open question is still open.**
Levy: *"we did not evaluate our design in a multi-processor setting"*, and its multicore subsection is explicit that
*"this paper has examined using Rust in a single-threaded setting."* Their ask is specific: *"Future
work should explore how to avoid growing the trusted computing base in service of concurrency."*
RedLeaf claims the capability (*"RedLeaf provides typical features of a modern kernel: multi-core
support, memory management, dynamic loading of kernel extensions, POSIX-like user processes, and
fast device drivers"*) but **every published number in it is single-core by construction**: *"In all
our tests, we pin the application thread to a single CPU core"*, *"All tests are limited to a single
CPU core"*, *"on all setups, we restrict execution to one CPU core."* And it says nothing at all
about what concurrency did to its trusted base, which is the half Levy actually asked about. So:
**not answered.**

**Page tables: RedLeaf dissolves the question rather than answering it.** Levy asks for future work
on *"data structures on disk or in hardware (e.g. the page table)"*, meaning: can a safe language
model a structure whose layout and semantics are fixed by something other than the compiler? RedLeaf
never has to, because it does not use hardware address spaces for isolation at all. That is a
legitimate move and it has a visible price, stated by RedLeaf itself: *"We do not support the full
semantics of the fork() system call as we do not rely on address spaces and hence cannot virtualize
and clone the address space of the domain."* The question Levy posed is untouched, and it is a
question this tree answers head-on: `crates/paging` is where the page-table math lives precisely so
a model checker can reach it, and milestone 193 (put `kernel/src` within reach of the prover) is about
getting the prover to the rest.

**Size of the trusted base: RedLeaf regressed from a number to a list.** This is the finding worth
carrying. Levy's whole contribution is a *measured* TCB ("under 1000 lines out of over 6000"), and
**the RedLeaf paper contains no such figure anywhere.** It enumerates instead: *"RedLeaf's TCB
includes the microkernel, a small set of trusted RedLeaf crates required to implement hardware
interfaces and low-level abstractions, device crates that provide a safe interface to hardware
resources, e.g., access to DMA buffers, etc., the RedLeaf IDL compiler, and the RedLeaf trusted
compilation environment."* That list is strictly larger in kind than Tock's: it adds an **IDL
compiler** and a **trusted build environment**, neither of which Tock needs, and both of which the
same authors later called *"incomplete and error prone"* (PLOS '21). So the trajectory from 2017 to
2020 is that the bet got bigger and the accounting for it got weaker, and nobody quantified the
difference.

### What this tree can and cannot say in their terms

**It cannot say it in their terms, and the reason is architectural rather than a gap in the
tooling.** `notes/trusted-base.md` is the worked version: the three definitions of "trusted", the
re-derived numbers, their six unsafe categories run against this tree file by file, and the
`TakeCell` question answered from the code. The short form:

- In Tock and in RedLeaf the *safe* part of the kernel is **not** trusted for isolation, so counting
  unsafe lines is a genuine TCB measurement.
- In nife the **whole** kernel is trusted for isolation whether or not it is safe, because it is the
  thing that programs the MMU and mints capabilities. The unsafe census measures proof burden, not
  TCB size.
- The tree's own numbers, from `script/metrics --table` on 2026-09-20: **39,892 kernel code lines**,
  **577 `unsafe` blocks inside the kernel** (314 of them in `arch/`), and an unsafe density of **77**
  per 10,000 non-arch code lines against a ceiling of 88. The headline `unsafe_outside_arch` of 824
  is misleading for this purpose, because **561 of those blocks are in userspace**, outside the
  trusted base entirely.
- The apples-to-apples comparison is with seL4, which draws the boundary in the same place:
  *"of the order of ten thousand lines of source code (10 kSLOC)"* (the seL4 Foundation whitepaper).

**And the meta-gap is the one worth closing.** `notes/tcb.md` is about the *Thread* Control Block,
and its own acronym-collision section says the Trusted Computing Base sense *"is unrelated"*. No
metric column tracks the trusted core's size, so the split above is a snapshot with nothing watching
it. Proposed as `design/roadmap/535-the-trusted-core-has-no-size.md` rather than fixed here.

## Corrections to the sketch this lane was briefed with

The maintainer sketched this comparison from memory and flagged it as a sketch. Three of its five
claims survived checking, one is wrong, and one is a third of the real answer. Recording the misses
is the point of having checked.

**Wrong: "the Rust operating system from the University of Utah."** The RedLeaf paper is **UC
Irvine**, with one author at VMware Research; every one of the six academic authors is listed
"University of California, Irvine" on the title page. The Utah association is real but belongs to a
*later* system: Burtsev's group moved, and the Atmosphere papers (KISV '23, SOSP '25) are
"University of Utah." The confusion is easy to acquire, because the most-linked copy of the RedLeaf
PDF is hosted at `users.cs.utah.edu/~aburtsev/doc/redleaf-osdi20.pdf`.

**Incomplete: "termination with heap ownership tracking."** Ownership tracking is one of **five**
named principles, and on its own it does not deliver termination. The paper lists them as heap
isolation, exchangeable types, ownership tracking, interface validation, and cross-domain call
proxying. Ownership tracking only covers the *shared* heap; what makes termination safe is the
**private** heap invariant, and the paper is explicit that the private heap is freed without running
anything: *"the microkernel walks the registry of all untyped memory regions allocated by the
allocator assigned to the domain and deallocates them without calling any destructors. Such untyped,
coarse-grained deallocation is safe as we ensure the heap isolation invariant."*

**Right: single address space, ring 0.** Stronger than the sketch, in fact. *"As RedLeaf does not
rely on hardware isolation primitives, all domains and the microkernel run in ring 0."*

**Right: domains must be Rust, and zero-copy `RRef`.** *"Domains, however, are restricted to safe
Rust (i.e., microkernel and trusted libraries are the only parts of RedLeaf that are allowed to use
unsafe Rust extensions)."* `RRef<T>` is the shared-heap pointer: *"RRef<T> consists of two parts: a
small metadata and the value itself. The RRef<T> metadata contains an identifier of the domain
currently owning the reference, borrow counter, and type information for the value."*

**On the 2017 paper the sketch was accurate except in one word, and the word matters.** The thesis
quote, the garbage-collection objection, the four library abstractions, the six kernel pieces, the
`TakeCell` rationale and all three stated limits check out verbatim against the text. The size figure
was given as *"around 1000 lines out of over 6000"*; the paper says **under** 1000, and says it
inside a sentence whose subject is a TCB that also *"includes the Rust core library"*. Both details
cut the same way, which is that the headline ratio is smaller and the actual trusted base is larger
than the short form suggests. Detail is in "The ancestor" above.

**Right, with a caveat the paper does not make: x86_64 only.** The paper never states an
architecture claim. It does not have to: every experiment is on Intel, the interrupt entry discusses
*"the x86-interrupt function ABI"*, and the repository contains exactly one target
(`domains/x86_64-unknown-redleaf.json`, `kernel/x86_64-unknown-none.json`) and no other. So the
sketch is right about the artifact and is reading something into the paper that is not written there.

## What RedLeaf is

A microkernel where the boundary between components is enforced by the Rust type system instead of
by the MMU. *"RedLeaf is a new operating system developed from scratch in Rust to explore the impact
of language safety on operating system organization. In contrast to commodity systems, RedLeaf does
not rely on hardware address spaces for isolation and instead uses only type and memory safety of
the Rust language."*

Its unit is a **domain**: *"units of information hiding, fault isolation, and composition. Device
drivers, kernel subsystems, e.g., file system, network stack, etc., and user programs are loaded as
domains."* A cross-domain call is *"normal, typed Rust function invocations. Upon cross-domain
invocation, the thread moves between domains but continues execution on the same stack."* That is
the migrating-threads model, chosen deliberately over messages *"to avoid a thread context switch on
the critical cross-domain call path."*

Two things in it will read as familiar here. It is capability-shaped: *"In RedLeaf references to
objects and traits are capabilities"*, and a domain's default authority is exactly one thing,
*"the microkernel system call interface ... the only interface through which the domain can affect
the rest of the system."* And it has a supervision story: shadow drivers, *"lightweight shadow
domains that mediate access to the device driver and restart it replaying its initialization
protocol after the crash"*, which is DECISIONS §26 (the fault endpoint: thread death becomes a message) wearing
different clothes.

On top of it they built **Rv6**, a POSIX-subset personality following xv6, as a collection of
domains. Its one loud gap is the one a single address space forces: *"We do not support the full
semantics of the fork() system call as we do not rely on address spaces and hence cannot virtualize
and clone the address space of the domain."*

## Bet one: what each system has to trust

*`notes/trusted-base.md` is the measured companion to this section: the three incompatible
definitions of "trusted", the re-derived numbers for this tree, and their six unsafe categories run
against it. This section is the argument; that note is the units.*

This is the interesting axis, and both systems name their own trusted base honestly, which makes the
comparison possible at all.

**RedLeaf trusts the Rust compiler, and says so first.** *"The core assumptions behind RedLeaf are
that we trust (1) the Rust compiler to implement language safety correctly, and (2) Rust core
libraries that use unsafe code."* Its TCB then enumerates: *"the microkernel, a small set of trusted
RedLeaf crates required to implement hardware interfaces and low-level abstractions, device crates
that provide a safe interface to hardware resources, e.g., access to DMA buffers, etc., the RedLeaf
IDL compiler, and the RedLeaf trusted compilation environment."*

Two admissions sit beside it. On unsafe code: *"At the moment, we do not address vulnerabilities in
unsafe Rust extensions, but again speculate that eventually all unsafe code will be verified for
functional correctness."* On hardware: *"We trust devices to be non-malicious. This requirement can
be relaxed in the future by using IOMMUs to protect physical memory. Finally, we do not protect
against side-channel attacks."*

**What happens when the trust is misplaced is where the two systems diverge, and it is structural
rather than a matter of degree.** If `rustc` miscompiles a bounds check in a RedLeaf domain, or an
`unsafe` block inside a whitelisted core library has a soundness hole, the isolation is gone and
nothing underneath notices, because there is nothing underneath: one ring, one address space, one
page table. If `rustc` miscompiles a nife program, the program is wrong and the kernel's page tables
are unmoved; the confinement claim is made by hardware the compiler did not emit. `notes/c-seam.md`
puts the arithmetic plainly for the worst case it could construct, which is C: *"what a bad index
reaches: any physical memory, any device register [in a monolith] | one page it was granted [confined
here]."*

**Neither trust is free and this tree's version can come back red, which is the asymmetry worth
recording.** nife's claim is DECISIONS §14 (a verified-Rust capability microkernel that runs real
workloads), and `design/fatal-risks.md`'s risk 2 exists precisely to ask whether the proofs prove
anything: its answer is AMBER, and milestone 191 (did the proofs catch the bugs?) found that *"no
Kani harness in this tree has ever caught a defect after the day it was written"* because
`cargo kani -p <crate>` never compiled the kernel. That is a worse-sounding sentence than anything in
the RedLeaf paper, and it is a *better* epistemic position, because it is a falsifiable claim that
was falsified and then acted on (milestone 193 (put `kernel/src` within reach of the prover)). RedLeaf's
trust in `rustc` is not the kind of claim an experiment in the RedLeaf tree can return red on.

**And the trusted compilation environment is the part that is easiest to underrate.** RedLeaf's
safety across separately compiled domains depends on it: *"RedLeaf relies on a trusted compilation
environment. This environment allows the microkernel to check that domains are compiled against the
same versions of IDL interface definitions, and with the same compiler version, and flags. When a
domain is compiled, the trusted environment signs the fingerprint that captures all IDL files, and a
string of compiler flags. ... Additionally, we enforce that domains are restricted to only safe Rust,
and link against a white-listed set of Rust libraries."* By their own PLOS '21 assessment, this is
where the design is weakest: *"development of a trusted build environment is incomplete and error
prone"*, and their own prescription is **"Research: Support typed assembly language for Rust"** and
**"Ecosystem: Support trusted build environments."**

**The public tree does not enforce the safe-Rust rule.** Checked rather than assumed, on the
2026-09-20 clone: of 23 domain crate roots outside `domains/lib/`, **seven** carry an active
`#![forbid(unsafe_code)]`, **four** have it present but commented out, and **twelve** never had it.
The four commented-out ones include both flagship drivers (`domains/sys/driver/ixgbe/src/lib.rs`,
`domains/sys/driver/nvme/src/lib.rs`), the shadow block driver
(`domains/usr/shadow/bdev/src/lib.rs`), and the Rv6 network domain. Six domain files outside the
trusted `domains/lib/` tree contain real `unsafe` blocks, `ixgbe` among them. Nothing in the
`Makefile`s forbids it, and the `tools/signer` binary is 31 lines that append an Ed25519 signature to
an ELF, which is the *version* half of the fingerprint and not the *safety* half. This is a research
prototype and the gap is unsurprising; it is recorded because the paper's central invariant and the
artifact's enforcement of it are not the same thing, and a comparison that took the paper's word for
it would be comparing against a system that does not exist.

## Bet two: the cost per crossing

**This is where RedLeaf's numbers will be quoted against `design/fatal-risks.md`'s risk 4** (*"the
architecture imposes a per-crossing cost that cannot be engineered away"*), and against milestone 168
(a multi-tasking workload benchmark) when it produces one. So the conditions matter more than the
figures.

RedLeaf's Table 1, on CloudLab **c220g2**: two Intel E5-2660 v3 10-core Haswell CPUs at 2.6 GHz,
160 GB RAM, bare metal, with hyper-threading, turbo boost, CPU idle states and frequency scaling all
disabled, and *"seL4 configured without meltdown mitigations"*:

| operation | cycles |
|---|---|
| seL4 | 834 |
| VMFUNC (the instruction alone) | 169 |
| VMFUNC-based call/reply invocation | 396 |
| RedLeaf cross-domain invocation | **124** |
| RedLeaf cross-domain invocation (passing an `RRef<T>`) | **141** |
| RedLeaf cross-domain invocation via shadow | 279 |
| RedLeaf cross-domain via shadow (passing an `RRef<T>`) | 297 |

Their breakdown: *"a null cross-domain invocation via a proxy object ... introduces an overhead of
124 cycles. Saving the state of the thread, i.e., creating continuation, takes 86 cycles as it
requires saving all general registers. Passing one RRef<T> adds an overhead of 17 cycles."*

**The quotable number is the ratio on one machine, not the absolute.** 124 against 834 is roughly
**6.7x**, measured by one team on one box, and that is the comparison that survives being moved
between papers. The absolute 124 should never be set beside a nife figure directly: different ISA,
different decade, different measurement instrument.

**Two caveats on their own table, in their favour and against.** The shadow row disagrees with their
own prose, which says *"in case of a shadow the invocation crosses two proxies and a user-built
shadow domain and takes 286 cycles"* against the table's 279. Seven cycles, and it is worth noting
only because it is the kind of thing a reader quoting one of the two will not know about. And the
seL4 row is seL4's IPC, which is a *different guarantee*: a RedLeaf crossing does not change address
space, does not change privilege level, and does not schedule.

**What nife measures, for the shape rather than for a race.** `notes/benchmarks.md`, under HVF on an
Apple M3, median of five boots: `ipc_rtt_el0` (EL0 to EL0, two rendezvous, two address spaces, four
`svc`s) at **350 ns**, which that note converts to **~960 to ~1,420 cycles** across the M3's E-core
and P-core clocks because *"cycles here are arithmetic, not a reading."* The same note anchors
against seL4's published Jetson TX1 figures, 413 + 426 = ~839 cycles for a round trip, and concludes
*"roughly 1.1x to 1.7x an L4-lineage round trip."*

**So the honest statement of the gap is a chain of ratios rather than a subtraction**, and the chain
has a missing link this tree already knows about: `notes/benchmarks/calibration-against-sel4.md` says `call_reply` *"has no EL0
twin. So the structurally matched comparison to seL4's published pair is not measured at all"*, and milestone 188 (the IPC fastpath) owns closing that. Until it closes, nife's number and
RedLeaf's number are both roughly-6x-to-8x away from an L4-lineage crossing in opposite directions,
each measured against a different seL4 run on a different machine.

**And the strongest evidence on this axis was published by RedLeaf's own authors, against
themselves.** Atmosphere (SOSP '25) is a verified Rust microkernel from the same group that *does*
use hardware isolation and a conventional syscall interface. Its Table 3, on CloudLab c220g5 (two
Intel Xeon Silver 4114 10-core at 2.20 GHz):

| system call | Atmosphere | seL4 |
|---|---|---|
| Call/reply | 1,058 | 1,026 |
| Map a page | 1,984 | 2,650 |

**A verified Rust microkernel with hardware isolation, built by the RedLeaf team five years later,
pays about a thousand cycles for call/reply and lands within 3% of seL4.** That is the same shape as
nife's *"1.1x to 1.7x"* against seL4, arrived at independently, on x86_64, on silicon. The ~6-8x
that language isolation buys is therefore not a RedLeaf-specific artifact and not a tuning failure on
anyone's part: **it is what a privilege and address-space crossing costs**, and both halves of that
sentence have now been measured twice by the same group. Risk 4 says the per-crossing cost *"cannot
be engineered away."* On this evidence, that is not a fear. It is measured, and the question a
customer-facing workload actually asks is how many crossings the workload makes, which is exactly
what milestone 168 was minted to answer.

## Bet three: what each system can isolate

**RedLeaf can isolate only code it compiled, from source, in its own build environment, in a language
it chose.** That is not an oversight; it is the direct consequence of the mechanism. Isolation is the
type system, so anything outside the type system is outside the isolation. The paper's own dynamic
loading section states the invariant this rests on: *"types of all data structures that cross a domain
boundary, including the type of the entry point function, and all types passed through any interfaces
reachable through the entry function are the same, i.e., have identical meaning and implementation,
across the entire system."* A C binary cannot satisfy that. Neither can a Rust binary built somewhere
else.

Their own evaluation contains the cost of this, stated plainly, and it is the most honest paragraph in
the paper. On the key-value store: *"Despite our optimizations, RedLeaf achieves only 61-86%
performance of the C DPDK version"*, because safe Rust forced `Vec<T>` and three
`extend_from_slice()` calls where C used `memcpy`. And then: *"As an exercise, we implemented the
packet serialization logic with unsafe Rust typecast that allowed us to achieve 85-94% of the C's
performance. However, we do not allow unsafe Rust inside RedLeaf domains."* **They measured the
speedup, and refused it, because taking it would have cost them the isolation.** That is the bet
being paid for, visible in a single sentence.

**nife's version of this axis is `design/fatal-risks.md`'s risk 1, and it has been run.** Unmodified
`ripgrep` 14.1.1 from crates.io, forty transitive crates, zero patches, builds and runs on all three
architectures (milestone 121 (`ripgrep` on nife: enumeration as a capability), and the three transcripts are byte
for byte identical). DECISIONS §31 (the foreign-language seam) confines C that holds no capabilities and
makes no syscalls, demonstrated by faulting it deliberately and restarting it. A language-isolated
system has no move available on either.

**The symmetric honesty: that door swings both ways.** nife's isolation does not care what language a
component is written in, and it also does not give a nife program the intra-component memory safety
that a RedLeaf domain gets for free. A bad index inside a nife component corrupts that component; a
bad index inside a RedLeaf domain does not compile. RedLeaf buys fine-grained safety *within* a
component and forfeits the ability to run anything it did not build; nife buys the ability to confine
anything at all and leaves within-component safety to whatever language the component chose. Neither
is a subset of the other.

## Where RedLeaf is ahead, without hedging

Four places, and the first two are not close.

1. **Per-crossing cost.** 124 cycles against seL4's 834 on the same machine. Nothing in a hardware-
   isolated design reaches that, and the Atmosphere numbers above are that statement made by the
   people with the most incentive to find otherwise.
2. **Drivers measured against the fastest thing available, on real hardware.** Their ixgbe driver
   against DPDK: *"On a batch of one, DPDK achieves 6.7 Mpps and is 7% faster than RedLeaf (6.5
   Mpps)"*, and at batch 32 *"both drivers achieve the line-rate performance of a 10GbE interface
   (14.2 Mpps)."* Their NVMe driver against SPDK: *"the RedLeaf driver is 1% faster (457K IOPS
   per-core) than SPDK (452K IOPS per-core)."* This tree has no driver benchmarked against a
   user-space framework at all, on any architecture.
3. **A recovery number taken under a crash loop.** *"we trigger a crash of the block device driver
   every second ... For reads, the throughput with and without restarts averages at 2062 MB/s and
   2164 MB/s respectively (a 5% drop in performance). For writes, the total throughput averages at
   356 MB/s with restarts and 423 MB/s without restarts (a 16% drop)."* nife has the mechanism
   (DECISIONS §26 (the fault endpoint: thread death becomes a message), DECISIONS §41 (the endpoint is the broker)) and
   `notes/live-replacement.md` records that live replacement costs zero in steady state, but nobody
   here has published what a component dying repeatedly costs a workload.
4. **Application-level numbers against commodity baselines.** Maglev, a network key-value store, and
   an httpd measured against Linux sockets, DPDK and nginx: *"On Linux, Nginx can serve 70.9 K
   requests per second, whereas our implementation of httpd achieves 212 K requests per second."*
   Whatever the caveats, that is a workload a stranger recognises, and milestone 168 (a multi-tasking
   workload benchmark) is still PARTIAL.

A fifth, which is a design observation rather than a result: **RedLeaf's IDL generates the proxy, the
entry point, the create trait and the microkernel-side create function from one interface definition**,
and enforces the exchangeability invariant as a static analysis pass over the resulting type graph.
That is rung one of AGENTS.md's ladder (make the wrong state unrepresentable) applied to an IPC
surface, and it is further up that ladder than anything in this tree's component contracts. Milestone
23 (a capability-routed component OS with live replacement) is where it would be relevant.

## What happened after 2020

**The repository is public, unarchived, and stopped.** `github.com/mars-research/redleaf`, read
2026-09-20 through the GitHub API: last push **2022-05-09**, and that commit is `add paper link`; the
last substantive commit is **2022-01-09**. 144 stars, 35 open issues, `archived: false`, and the API
reports **no license**, which for a reuse question means "all rights reserved" rather than
"permissive". The pinned toolchain is `nightly-2021-12-15`, so a build today needs that exact nightly.
Measured on the clone: 48,749 lines of `.rs` across the tree (kernel 10,364; domains 19,818; interface
2,499; lib 14,621; tools 1,370), which **excludes the `redIDL` compiler**, a git submodule this lane
did not fetch.

**The authors published their own critique a year later.** *Isolation in Rust: What is Missing?*
(PLOS '21) reads as a list of the places RedLeaf had to work around the language, and it is more
useful to this tree than the OSDI paper in one respect: it says what the design cost. On the IDL:
*"lacking support to cleanly express the necessary invariants for exchangeable types in the Rust type
system, RedLeaf relies on a separate, complex interface definition language (IDL) that enforces these
isolation invariants outside of Rust."* On unwinding: *"RedLeaf implements costly continuations to
unwind execution across domain boundaries."* On the type identifiers their shared-heap destructors
depend on: *"this implementation of RTTI does not guarantee collision freedom, an attacker can
generate a type with a colliding identifier to trigger an unsafe deallocation of object."* Their asks
of the language are concrete: trait bounds on function pointers of any arity, type information in
procedural macros, a collision-free unique type identifier, and a `no_std` extendable unwind library.
**As of this reading none of the four has landed**, which is the honest reason RedLeaf's mechanism
has not been picked up elsewhere.

**And the group's next kernel took the other bet.** Atmosphere is *"a full-featured microkernel
developed in Rust and verified with Verus"*, *"conceptually similar to the line of classical L4
microkernels, i.e., without the capability interface"* (KISV '23). It uses hardware isolation, a
conventional syscall interface, and a proof effort rather than a compiler assumption: SOSP '25 reports
*"6K lines of executable code"* against *"20.1K lines of proof code"*, a **3.32:1** proof-to-code
ratio, *"less than 2.5 person-years"*, and full verification *"in under 20 seconds"* on an i9-13900hx.
Their own Table 1 puts that beside seL4's 20:1 and CertiKOS's 14.9:1.

That is the single most consequential fact in this note, and it is about neither paper's benchmarks.
**The team that built the strongest language-isolated OS in the literature spent the following five
years building a hardware-isolated, machine-verified Rust microkernel instead.** Nothing in either
Atmosphere paper says RedLeaf was abandoned or why, and this note does not claim it: the two lines
share authors and could be complementary. What can be said is what the artifacts show, which is that
the Verus-verified L4-shaped kernel is the one still being published and the RedLeaf tree has not
moved since January 2022.

## What could not be established

- **Why RedLeaf stopped.** Nothing in the repository, the papers, or the group's pages says. The 35
  open issues were not read. **Unknown, and it should stay unknown in this note rather than be
  inferred from a commit date.**
- **Whether the safe-Rust-only invariant was ever enforced anywhere.** The measurement above is of the
  public tree at its last commit. A stricter internal build may have existed. `redIDL` was not
  fetched and is the most likely place for such a check to live.
- **Whether RedLeaf boots on anything current.** Not attempted. It wants `nightly-2021-12-15`, `nasm`,
  `grub-mkrescue`, and CloudLab-class Intel hardware for anything but QEMU.
- **The SOSP '25 Atmosphere evaluation beyond Tables 1-3.** Its driver and application benchmarks were
  not read in full; only the verification-effort and syscall-latency tables were.
- **How Atmosphere's authors would characterise the relationship between the two systems.** Not asked,
  and not inferable from the text.
- **Whether anyone has independently reproduced RedLeaf's 124-cycle figure.** Not searched
  exhaustively.

## BUGS

- **This note was written from seven documents and one shallow clone. It is not a survey.** Citing
  papers is not the same as tracking a field, and the isolation-in-Rust literature since 2021
  (KSplit, Theseus, Netbricks, Splinter, VeriSMo, NrOS, Asterinas) was seen only through these
  authors' related-work sections, which are not a neutral source about their neighbours.
- **Nothing was read about what Tock became.** The 2017 paper is treated here as an argument, which
  is what it is, and the nine years of Tock since it are out of scope. A claim in this note about
  the 2017 position is not a claim about Tock today, and if anyone wants the second thing it is a
  separate reading.
- **The "RedLeaf did not answer the 2017 questions" finding is an argument from absence**, which is
  the weakest shape of finding there is. It rests on the RedLeaf paper containing no TCB size figure
  and no multi-core measurement, both of which this lane established by reading the full text and
  grepping it. An absence in a ten-page paper is not an absence in the work.
- **No RedLeaf number in this note was reproduced.** Every figure is quoted with its stated
  conditions, and every one comes from a paper's own evaluation of its own system. That is the weakest
  class of benchmark evidence there is, and it is the only class available without CloudLab access.
- **The `#![forbid(unsafe_code)]` audit is a grep over crate roots**, so it undercounts (a crate could
  forbid unsafe in a submodule) and overcounts (a crate with no `unsafe` needs no attribute). It
  establishes that the invariant is not *mechanically* enforced. It does not establish that any
  shipped domain violates it in a way that matters.
- **The line counts are raw lines including comments and blanks**, on both sides. `notes/counted-claims.md`
  and `design/fatal-risks.md` both record that `kernel/src` is 40% comment by measurement, so any
  size comparison drawn between this tree and RedLeaf's 48,749 lines is invalid as written. It is
  included for order of magnitude only.
- **The PLOS '21 reading of what Rust still lacks was not re-checked against current Rust.** "As of
  this reading none of the four has landed" is this lane's belief from the language's own tracking
  history and was not verified against a 2026 nightly. Treat it as unconfirmed.
- **Nothing here is a positioning statement, on purpose.** The comparison is laid out and the
  concluding sentence is not written, the same way
  `design/roadmap/528-cheri-capabilities-are-not-these-capabilities.md` stops short of one.
  Where nife stands relative to RedLeaf is an architect's to say.
