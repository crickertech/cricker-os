---
status: BUILT
raised: 2026-09-20
built: 2026-09-20
---
# 447. A thread's vector registers are its own

Built 2026-09-20. Minted 2026-09-20 by the maintainer, on calef opening **Route 2** of
milestone 164 (x86_64 userspace can't build `aes`: no SSE, no scalar fallback), whose
[block](164-x86-64-fs-server-aes.md) refused that route and priced it, after a conversation about
why SSE is switched off. *(Number provisional until the merge queue lands it.)*

## What was true before this, and it was true on purpose

The kernel saved no floating-point or vector state anywhere, on any of the three architectures. Not
a gap: a decision, recorded three times and enforced by construction.

- Every target in `targets/` is soft-float. `x86_64-unknown-nife.json` carries
  `"features": "-mmx,-sse,-sse2,...,-avx,-avx2,+soft-float"` and `"rustc-abi": "softfloat"`; the
  other two are the `-softfloat`/`-neon` equivalents.
- Milestone 184 (extend the `std` port to x86_64), in [its own block](184-std-x86-64.md), calls
  that feature string **"a correctness requirement"** in its target table, with the reason beside
  it: "`kernel/src/arch/x86_64/` saves no FPU or SSE state on a context switch."
- §31 (the foreign-language seam: C holds no capabilities and makes no syscalls), in
  [its own section](../decisions/0031-foreign-language-seam.md), says the same about the C boundary:
  "the kernel never enables FP/SIMD for EL0, and the context switch" saves nothing, so a vector
  register in a confined component would be a trap or a corruption depending on which of those two
  bit first.
- 164's own refusal priced exactly this milestone and declined to build it: an SSE-enabled x86
  userspace "would mean an `FXSAVE` area per thread and save/restore in the context-switch path,
  and none of that is needed to compile `aes`." That was the right call then. It is what calef
  reopened.

**So the interesting thing here is not that a kernel can save registers.** It is that a kernel
built this far on the assumption that it never had to now does, without the switch getting slower
for the threads that do not use it, and that the three ISAs' mechanisms are different enough to be
worth writing down.

## The rule, which is one sentence

> **The FP/SIMD register file holds the running thread's data, or it holds the initial state. It
> never holds a thread's data while a different thread runs.**

That is `kernel/src/fp.rs`'s header, and everything else is that sentence made true at the one
instant it could stop being true. `crate::fp::hand_over` is called by the outgoing thread, on the
outgoing thread's stack, with `IPC_TABLES` already released and interrupts masked, immediately
before `arch::switch_to`. Four cases:

| outgoing | incoming | what happens |
|---|---|---|
| not live | not live | nothing. Two loads and a branch. **Every switch in this tree today.** |
| live | live | save, restore |
| not live | live | enable, restore |
| live | not live | save, **scrub to the initial state**, disable |

**The switch itself is untouched.** `context.s` still saves what a function call may destroy, on
all three architectures, and this runs beside it rather than inside it. That was a deliberate
choice over the tidier-looking one of growing `Context` and letting `switch_to` do the whole job:
the register file is not a calling convention's callee-saved set, it is 512 bytes that cross a
*thread* boundary, and keeping it in Rust keeps it typed, testable, and out of three files of
delicate hand-computed offsets.

**And it lives in the free space of the thread's own TCB page**, not in the `Thread` struct and not
on the kernel stack. That was the second shape rather than the first, and `script/stack-frame-check`
is the reason; see *What the gate caught*, below.

## Why eager, and why "lazy" is a warning word here

The cheap-looking scheme is to leave the outgoing thread's registers in the hardware, trap when the
incoming thread touches them, and only then swap. It is what several kernels did until 2018, when
**LazyFP, CVE-2018-3665**, showed that on x86 the trap is not a boundary: speculative execution past
the `#NM` reads the registers the trap was supposed to protect, and one thread recovers another's
AES round keys. The mechanism was `CR0.TS`, which is the very bit this tree's x86 half now uses.

So the three control registers here answer **"has this thread ever wanted FP"**, which is a
performance question, and never **"whose data is in the registers"**, which is a confidentiality
one. The second question is answered by the registers always being right, which is what the fourth
row of the table above is for: when a thread that used the unit hands off to one that has not, the
file is scrubbed to the architectural initial state *before* the trap is re-armed, so there is
nothing behind the trap to speculate at.

## Three architectures, three mechanisms, one rule

| | gate | first-use trap | what moves | bytes |
|---|---|---|---|---|
| aarch64 | `CPACR_EL1.FPEN`, `0b00` (traps EL0 **and** EL1) against `0b11` | its own exception class, `ESR_EL1.EC == 0x07` | `q0`-`q31`, `FPCR`, `FPSR` | 512 + 16 |
| riscv64 | `sstatus.FS`, Off against anything else | an **ordinary illegal instruction**; nothing distinguishes it | `f0`-`f31`, `fcsr` | 256 + 8 |
| x86_64 | `CR0.TS`, with `CR4.OSFXSR`/`OSXMMEXCPT` set and `CR0.EM` clear | `#NM`, vector 7 | the `FXSAVE` area: x87, `MXCSR`, `xmm0`-`xmm15` | 512 |

Two of those rows cost this milestone real time, and both are written down where the next reader
meets them rather than only here.

### RISC-V does not tell you, so this retries rather than decodes

`sstatus.FS` is the best control register of the three: a four-state machine (Off / Initial / Clean
/ Dirty) with a hardware-maintained dirty bit, designed by people who had context switching in
mind. What it does not do is say why an instruction was illegal. An FP instruction under `FS == Off`
arrives as `scause` 2, exactly like a bad opcode, and `stval` is permitted to be zero for that cause
and is on some parts.

The handler therefore does not decode the instruction, it retries it. `FS == Off` in the frame
is the whole guard: open the unit, leave `sepc` where it is, and let the `sret` re-execute whatever
trapped. A genuinely illegal instruction traps a second time with `FS` no longer Off, fails the
guard, and falls through to the fault it deserved. The cost of being wrong is one extra trap on a
path that is already killing a thread. The alternative was reading user memory at `sepc` (with
`SUM`, and a page fault to handle) to decode seven major opcodes plus the compressed forms, to
answer a question the retry answers for free.

### And `sstatus` travels in the trap frame, which the other two do not

`TrapFrame::sstatus` is captured on trap entry and written back by `trap.s`'s `csrw sstatus, t0` on
the way out. A handler that set `FS` in the live CSR and returned therefore had its own write undone
by its own return. The first-use handler sets the frame's copy as well; every later switch is safe
without thinking about it, because `hand_over` sets the live CSR from the incoming thread's `live`
and that thread's frame carries the same value for the same reason.

## What it found: one architecture's boot path does not run `arch::init`

`arch::fp::init` shuts the unit for a core before any thread exists. The obvious home was
`arch::init`, and that is where it went first. It was green on aarch64 and x86_64 and silently
wrong on RISC-V, and the test that caught it is the one written to catch exactly this.

`main`'s RISC-V tour installs `stvec` by calling `arch::exceptions::init()` directly. It reaches
`sched::init` and never passes through `arch::init` at all. OpenSBI hands the kernel a hart with
`sstatus.FS` already set, so no thread ever took the first-use trap, `live` stayed false on every
thread, `hand_over` took its early return every time, and two threads shared a register file with
nothing in the system saying so.

The fix is not a third call site. The invariant is about threads, so it lives where threads begin
to exist: `sched::init` and `sched::adopt_secondary_idle`, both portable, both on the path of
every core that will ever own a thread. That is AGENTS.md's ladder read downward: the rung-four
version (remember to call it from each architecture's bring-up) is what failed, and the property
and the mechanism are now in the same function.

`the_first_floating_point_instruction_takes_a_trap` exists because of this, and it does not merely
assert that FP works. It distinguishes three outcomes and names each one, because "a brand-new
thread started with the unit already open" and "an FP instruction executed without opening the unit"
are different bugs with the same symptom.

## What it costs, measured

### The IPC fastpath: within bound on every ISA, after a correction

`script/fastpath-footprint`, before and after, on this tree:

| ISA | `ipc_send_recv` | `ipc_call_reply` | `syscall_entry` |
|---|---|---|---|
| aarch64 | 6256 → 6300 (+0.7%) | 8190 → 8234 (+0.5%) | 1701 → 1701 (0) |
| riscv64 | 4632 → 4700 (+1.5%) | 5936 → 6004 (+1.1%) | 1870 → 1912 (+2.2%) |
| x86_64 | 5356 → 5432 (+1.4%) | 7028 → 7104 (+1.1%) | 1504 → 1504 (0) |

**The first measurement was not that**, and the correction is the useful part. Inlined into
`schedule`, `hand_over`'s expensive half put riscv64's `ipc_send_recv` 7.5% over the 5% bound
and `ipc_call_reply` 5.8% over, for code that no thread in this tree executes. The gate measures the
transitive closure of non-cold calls, because Liedtke's argument is about what a round trip
evicts from L1i, and five hundred bytes of register-file machinery that no IPC runs evicts nothing.
So the half that touches registers is `#[cold]` and out of line, and so is `enable_for_current`.

`#[cold]` here is a claim about this tree rather than a hint about taste, and it is checkable:
`crate::fp::ENABLES` counts the threads that have ever asked for the unit and is zero on every
shipping boot. `enable_for_current` being cold matters a second time on RISC-V, where it is called
from `riscv_trap_body`, one of the symbols this gate measures flat: the trap decoder's bytes are
on every `ecall`, so an arm that inlined a register-file load there would be charged to every
syscall this kernel serves. That is the +2.5% row above, and it is the decoder's own compare and
branch.

### The benchmarks: the switch costs one to three percent more instructions

`script/bench`, deterministic icount, against the committed baselines. Re-recorded in the commit
that moved them, per `bench/baseline-*.txt`'s own header.

| | aarch64 | riscv64 | x86_64 |
|---|---|---|---|
| `yield_switch` | 1101149 → 1134719 (+3.0%) | 184875 → 188666 (+2.1%) | 18903108 → 19288726 (+2.0%) |
| `ctx_switch` | 2922971 → 3007320 (+2.9%) | 495050 → 504958 (+2.0%) | not measured on this ISA |
| `ipc_rtt` | 1026311 → 1045670 (+1.9%) | 169382 → 172373 (+1.8%) | 17068218 → 17252344 (+1.1%) |
| `ipc_rtt_el0` | 10755621 → 10949404 (+1.8%) | 1829296 → 1847912 (+1.0%) | not measured on this ISA |
| `null_syscall` | 405004 → 410004 (+1.2%) | 72200 → 73029 (+1.1%) | not measured on this ISA |
| `spawn_reap` | 210224 → 214922 (+2.2%) | 33756 → 34277 (+1.5%) | 2779705 → 2830510 (+1.8%) |
| `coremark` | 20915599 → 20913256 (**-0.01%**) | 3654349 → 3654349 (**0**) | 306261408 → 306259744 (**-0.001%**) |

Every figure is inside the 10% tripwire. Read the table as two facts rather than one:

- A context switch costs about seventeen more instructions on aarch64 (550.6 → 567.4 per
  `yield_switch` iteration) and about 1.9 on riscv64. That is `hand_over`'s two loads and branch,
  plus the two `fp_state_of` offsets `schedule` computes under the lock.
- `spawn_reap` went the other way when the register file moved out of `Thread`: 215841 with the
  field inline, 214922 with it in the page. Spawning copies a `Thread` and a smaller struct is a
  smaller copy, so the shape the stack gate forced is also the cheaper one to spawn. Still +2.2%
  against `main`, because the initial write of the register file is real work a spawn did not do
  before.
- **`coremark` did not move, on any architecture**, and on riscv64 it is identical to the digit. A
  compute workload switches rarely, so the per-switch cost is invisible to it. The two numbers that
  went very slightly *negative* are code layout, not an improvement; nothing here makes arithmetic
  faster.

`null_syscall`'s +1.2% on aarch64 is the new `ec::FP_SIMD_ACCESS` arm in the exception decoder: a
quarter of an instruction per syscall, which is one compare amortized over the arms that precede
it. It is identical between the two shapes, as it should be: nothing about where the register file
lives is on the syscall path.

### The memory: none, and that is not a rounding of 544 down

The register file costs no memory at all. A `Thread` is always constructed at the start of a
whole 4096-byte TCB page it exclusively owns: `Threads::insert_at` and `insert_at_in_place` both
take `phys_to_virt(page) as *mut Thread`, and every route into the table goes through one of the
two. `size_of::<Thread>()` is 1152, so 2,944 bytes of that page were already allocated and
idle, and the register file (544 on aarch64, 528 on x86_64, 272 on riscv64) goes there.

`thread::fp_state_of` is the accessor, and one line is the whole of the mechanism that keeps the
fit true:

```rust
const _: () = assert!(FP_STATE_OFFSET + size_of::<FpState>() <= PAGE_SIZE);
```

Grow `Thread` past the point where the register file no longer fits beside it and the kernel does
not build. The one thing the page does not do for free is zero itself, so `Threads`' two inserts
call `thread::init_fp_state` right after the `Thread` is written: a `kmem` page carries whatever its
last owner left, and `live` is the first thing `hand_over` reads.

**It is reached through the page pointer, never through a `&Thread`.** A reference's provenance
stops at the end of the struct and this address is past it, so `Threads::pointer` hands back the
page cast unnarrowed and both the context slot and the register file fall out of that one pointer.
That is one table probe per side where the first shape's `get_mut` plus a field borrow was one as
well, so nothing was paid for the move.

### What the gate caught: a struct that gets copied through a caller's frame

The first shape put `FpState` inline in `Thread`, and `script/stack-frame-check` failed it. Not
a flake and not a tuning problem: the gate exists because a frame larger than the 4096-byte guard
page can move `sp` from inside a stack to below the guard in one step, touching nothing in between,
so the guard never faults and the write lands in the neighboring thread's stack.

| | on `main` | inline `FpState` | in the TCB page |
|---|---|---|---|
| `spawn_into::<std_service::start_on_full>` | 3552 | 4672 | 3552 |
| `spawn_into::<fs_service::spawn_fs_server>` | 3536 | 4656 | 3536 |
| `Thread::spawn::<sched::init>` | 3504 | 5152 | 3504 |

The delta is 1120 for a 544-byte field, which is the finding. An unoptimised build materializes
the `Thread` value and then copies it, so a byte added to the struct costs two bytes of frame.
`Thread::spawn` wraps `spawn_into` and paid 1648, half again as much, because the value passes
through one more frame on the way. Milestone 124 (a thread is born where it
lives: the spawn path's copies) is the block that already measured this shape, took
`sched::spawn_on` from 3888-4592 down under the guard by handing the destination pointer down, and
stopped one hop short of removing the last temporary. 447 spent that remaining headroom in one
field.

Building the `Thread` in place, field by field, was priced and refused; it is the `## Follow-on`
entry. It would fix the rest of the 3552 as well as this milestone's 1120, and it converts a
struct literal the compiler checks for completeness into twenty-five raw writes it does not. That
is the wrong direction on `AGENTS.md`'s ladder, in a lane about floating point, on the spawn path.
The register file moving one struct outwards into memory that was already there gets the same
numbers for none of that risk, which is why the table's third column is the first column exactly.

**`EXCEPT` was never a candidate.** The check offers it for a function that provably cannot run on a
kernel thread stack, and `Thread::spawn` is the opposite of that case.

### The instruction clock: unchanged, byte for byte

`script/icount` boots under `-icount shift=0,sleep=off` and asserts the timer handler's instruction
count. Measured at this branch's base commit (`01d1cbf3`) and at its tip:

| | before | after |
|---|---|---|
| aarch64 `arrival_instructions` | min 1040 mean 1040 max 1040 | identical |
| aarch64 `handler_instructions` | mean 1088 max 1088 | identical |
| riscv64 `arrival_instructions` | min 300 mean 300 max 400 | identical |
| riscv64 `handler_instructions` | mean 800 max 900 | identical |
| missed ticks, early arrivals | 0, 0 | 0, 0 |

Not a surprise, and worth recording as the control it is. The timer handler does not switch
threads: §9 (locking: `IrqSafeMutex`, plus a discipline) puts the rule in a table row, "interrupt
handlers record and defer; they do not do work", so the switch happens a frame out, on the
interrupted thread's stack. Nothing this milestone added is inside the window that gate measures,
and a number that *had* moved would have meant something was on a path it had no business being
on.

## The proof, and it fails against a kernel without this

Five `#[test_case]`s in `kernel/src/fp.rs`, running on all three architectures.

- `two_threads_doing_vector_work_do_not_see_each_others_registers` is the milestone. Two threads
  fill the whole register file with distinct patterns, yield two hundred times each, and check their
  own values after every turn. They are placed with `spawn_on(cpu::id(), ..)` and **not** with
  `spawn`, because §28 (SMP placement: two random choices at spawn) and its "spawn placement: the
  power of two choices" would put them on two cores with two register files, where the test would
  pass without the kernel doing anything at all. On one core they interleave over one file, with
  this thread (which never touches FP) between them, so the scrub-and-disable arm runs between
  every pair of turns as well.

  Falsified on purpose: with `save` removed from `hand_over` and nothing else changed, it fails
  with "a thread found another thread's values in its own vector registers". Run on aarch64 before
  the other two architectures existed.
- `a_thread_with_no_vector_state_finds_the_registers_scrubbed` drives the fourth row directly.
  Two threads both using FP catch a missing *save* loudly; a thread that stops using FP while its
  values sit in a file nobody scrubs fails silently, forever, and is what CVE-2018-3665 was. It
  reads the hardware back through a helper that deliberately does not go through the enable trap,
  because that trap installs the initial state itself and would prove nothing.
- `the_whole_register_file_survives_a_save_and_a_restore` checks every register by name. A save
  that copies the right bytes to the wrong lane passes the concurrency test whenever both threads
  are preempted at matching offsets; the per-register pattern fails here.
- `the_first_floating_point_instruction_takes_a_trap` is above: it is why the RISC-V bug was
  found rather than shipped.
- `two_threads_that_never_used_the_unit_leave_it_shut` asserts the case every switch in this
  tree actually takes. If it stopped being true the cost would be a kilobyte of memory traffic per
  switch and the only symptom would be a slower benchmark.

**The tests are kernel threads, not user programs, and that is forced rather than chosen.** Every
target in `targets/` is soft-float, so no userspace binary in this tree can execute an FP
instruction to be tested with. That is why all three first-use handlers serve the kernel's own
exception level as well as userspace: refusing EL1 would have meant a mechanism whose only possible
test was a different mechanism. §31's C seam is untouched for the same reason, and is the first
thing that changes if the flip below is taken.

## What this does NOT do: the target flip is not taken

No target JSON is changed and no force-soft flag is removed. Turning `+soft-float` off changes
the calling convention for every userspace binary and every `std` crate built against it, which is
an ABI two programs agree on, which AGENTS.md's *move fast on what can be undone* tenet puts in the
irreversible category, and which §22 (Rust `std` on the native ABI, the Hermit way)
chose deliberately when it specified nife's target JSON as "softfloat, and `singlethread = true`".

This milestone makes that flip possible. It is an architect's to make, and the point of the
section below is that it can now be made on evidence.

### The proposal

What it would take. Four edits and a rebuild. Drop `+soft-float` and the `-sse`/`-neon` feature
strings from the three files in `targets/`, drop `"rustc-abi": "softfloat"` from the two that carry
it, rebuild the `std` farm (`xtask std-src`, which every lane already takes the `nife-dev` toolchain
link for), and rebuild every user program. Nothing in the kernel changes: it stays `softfloat`, and
should, because a kernel that emits FP into its own fastpath would pay the save on every switch
rather than on the switches of threads that asked.

What it would buy. Less than the framing suggests, and the honest accounting is worth having
before the decision rather than after:

- `aes_force_soft` could go, and it is currently the *only* force-soft flag in this tree. 164's
  block and the brief for this milestone both suggest a family of them; there is one, in
  `.cargo/config.toml`'s `[target.x86_64-unknown-none]` block, and `grep` finds no other. Worth
  saying plainly because the flip's case is weaker than "six flags disappear".
- AES-NI against the bitsliced software backend. Upstream RustCrypto puts the hardware path
  roughly an order of magnitude ahead. 164 refused to measure this and was right to: nothing on
  x86_64 mounts an encrypted RedoxFS volume, so there is no workload and a synthetic number would
  be a fact leaving the machine with nothing behind it. That refusal still stands after this
  milestone. What changed is that the number is now *obtainable* rather than blocked.
- The second failure class of milestone 442 (a crypto provider `rustls` can use on all three
  bare-metal targets).
  Its block records that `sha2` and `polyval` "fail on soft-float x86_64", met through
  `embedded-tls`, and immediately warns that the probe behind that finding "ran against stock bare
  targets on the stable host toolchain, not against nife's own target specifications". 442 is
  NOT-STARTED and owes that re-measurement. **It should be run before the flip, not after**, because
  if those crates build against nife's targets on the pinned nightly then this half of the case
  evaporates, and if they do not, 442 has the specific list the flip would have to fix.
- §31's seam widens. A C component
  compiled by bare-metal clang currently cannot use vector registers at all. After the flip it can,
  and §31's sentence about "a trap or a corruption depending on which of those two bit first" stops
  being true, which is a real gain for the vendored-component rung the seam exists to de-risk.

What it would cost. Every thread that then executes an FP instruction takes one trap and pays
512 bytes of save and restore on every subsequent switch for the rest of its life, because `live`
never clears (this is `crate::fp`'s first `BUGS` entry). Soft-float userspace pays nothing today
because nothing takes the trap; hard-float userspace means `memcpy`, `std` formatting and anything
LLVM feels like vectorising will take it, so most threads become live, and the common case in the
table above stops being the common case. Nothing in this tree measures that, and the measurement
does not exist until there is a hard-float userspace to run.

What would have to be rebuilt. The `std` farm at minimum, and therefore every user program and
every archive; `nife-dev` is one symlink per user account, so the rebuild is machine-global and has
to be sequenced against other lanes (AGENTS.md's `std_src` rule, and notes/std.md's 2026-08-18
cross-contamination).

**What is not known, and would decide it.** Whether 442's crates actually fail against *our*
targets, and what a hard-float userspace does to the switch cost when most threads are live. Both
are measurements rather than arguments, and neither needs this decision made first.

## BUGS

- **`live` never clears.** A thread that used FP once saves and restores the whole file on every
  switch for the rest of its life. RISC-V's `sstatus.FS` could answer the narrower question in
  hardware and this does not use it: a uniform rule across three ISAs was judged worth more than one
  ISA's optimization while no workload exists to measure the difference on. Recorded in
  `kernel/src/fp.rs` and in `arch/riscv64/fp.rs`, which has the four-state table it declines to use.
- **x86_64 uses `fxsave`, not `xsave`.** A thread using AVX would have `ymm` upper halves this
  does not move. Safe only because `CR4.OSXSAVE` is clear, so every VEX-encoded instruction raises
  `#UD` and no thread can get into that state. The day this kernel sets `XCR0`,
  `arch/x86_64/fp.rs` has to grow an `xsave` path with it, and nothing enforces that coupling; it
  is stated in that file's `BUGS` where the next reader meets it. `xsave`'s init optimization
  (skipping components in their initial configuration) is left on the table with it.
- **aarch64 does not disable SVE or SME.** `CPACR_EL1.ZEN` and `SMEN` are left at their reset
  values, which on every machine this kernel has run on means trapped. A part that reset them open
  would let a thread keep vector state this file does not move. Nothing in this tree emits SVE and
  the check belongs with `arch::isa`'s feature reading; recorded, not built.
- A RISC-V hart without the D extension refuses rather than loops, and nothing tests it.
  `sstatus.FS` is hardwired to zero on such a part, so the enable does not take, `is_enabled` says
  so, and the first-use trap becomes an ordinary fault. Every machine in this tree's matrix
  (`qemu-system-riscv64 -cpu rv64`, the JH7110's U74) has D. F without D is worse: 32-bit
  registers where `fp.s` writes 64, so the save would be wrong rather than refused.
- A migrating thread carries its register file through memory, saved on the core it leaves and
  restored on the core it arrives at. Correct, and a kilobyte of traffic a same-core switch does not
  pay. Nothing measures it.
- This is proved under QEMU only. No board has run it. The one line most likely to be wrong on
  silicon is the one QEMU cannot falsify: `arch::fp::init` writes `CPACR_EL1` to a value QEMU's
  reset already supplies, so on the emulator it could be deleted without a test noticing, and the
  architecture says the reset value is UNKNOWN.

## Follow-on

- **Milestone 534.** milestone 534 (the soft-float targets could now be flipped),
  `design/roadmap/534-the-soft-float-targets-could-now-be-flipped.md`: the target flip, what it
  would take, buy and cost, and the two measurements that should come before it. Written up rather
  than recommended, because it is an ABI two programs agree on and §22 chose the current one
  deliberately; the *fork reaches calef with its questions already answered* tenet asks for options
  and costs on an irreversible fork and explicitly not for a winner. A `design/decisions/` section
  is owed when calef rules on it; this lane does not write one, per its brief.
- **Milestone 534.** First on that same file's list, in
  `design/roadmap/534-the-soft-float-targets-could-now-be-flipped.md`: re-measure milestone
  442's soft-float x86_64 failures against nife's own target specifications on the pinned nightly,
  which 442's block already says it owes in its own first item. It is a prerequisite for pricing
  the flip rather than a consequence of it, and it is cheap.
- **Recorded.** `arch::fp::init` is called from `sched::init` and `sched::adopt_secondary_idle`
  rather than from `arch::init`, because RISC-V's boot hart never calls the latter. The reason is at
  the call site in `kernel/src/sched.rs`, and all three `arch/*/fp.rs` point at it.
- **Recorded.** The `xsave`/`XCR0` coupling above, in `arch/x86_64/fp.rs`'s `BUGS`.
- **Refused.** Building a `Thread` in place, field by field, instead of writing a struct literal
  through a pointer. Priced under the gate rather than in the abstract, because
  `script/stack-frame-check` is what raised the question. It would take `Thread::spawn_into`'s
  frame well below the 3552 bytes it has carried since milestone 124 (a thread is born where it
  lives: the spawn path's copies), because an unoptimised build materializes the value and then
  copies it, so every byte of the struct costs two bytes of frame. What it costs is the thing the
  ladder ranks highest: a struct literal is checked for completeness by the compiler and
  twenty-five `addr_of_mut!` writes are not, and a forgotten field is uninitialised memory in a TCB
  with nothing to catch it. 124 stopped one hop short of this for the same reason. It is the right
  next move if the frame ever needs more headroom, and it was not needed here: the register
  file moved out of the struct instead and the numbers went back to 124's exactly.
- **Refused.** Growing `Context` so that `switch_to` saves the register file with the callee-saved
  set. It looks tidier and keeps `thread.rs`'s line that a thread's whole saved state is one stack
  pointer, and it is worse: an uninitialised region in every thread's kernel stack frame, a new
  frame-size constant leaking into portable code, three files of hand-computed offsets to keep in
  step, and a save/restore that Rust cannot test without an emulator. The cost of the chosen shape
  is two field reads in `schedule`, measured above.
- **Refused.** Using RISC-V's `sstatus.FS` dirty bit to skip a save whose registers are unchanged.
  It is the one place any of the three ISAs offers something better than "has this thread ever
  asked", and taking it would mean three different policies to reason about for a saving no workload
  can currently demonstrate. First thing to try when one exists; in `crate::fp`'s `BUGS`.
- **Refused.** Decoding the faulting instruction on RISC-V to tell an FP trap from a bad opcode.
  Above: `stval` may be zero for that cause, so it would mean reading user memory at `sepc` and
  decoding seven major opcodes plus the compressed forms, to answer a question one retry answers.

## Index row

Milestone 164's refused Route 2, reopened by calef. The kernel saved no FP or vector state anywhere,
which is why every target in `targets/` is soft-float as a correctness requirement rather than a
preference; it now saves the whole register file across a context switch on all three architectures,
under one rule: the file holds the running thread's data or the initial state, never a stranger's.
Eager and not lazy, because the trap this uses on x86 is `CR0.TS` and deferring the restore behind
it is CVE-2018-3665. The expensive half is `#[cold]` and off the IPC fastpath, so a soft-float
thread pays two loads and a branch, and the register file costs no memory: it lives in the 2,944
bytes of a thread's TCB page that were already allocated and idle, which is where
`script/stack-frame-check` sent it after the inline shape put `Thread::spawn` 1,056 bytes over the
guard page. `script/fastpath-footprint` within bound on every ISA, a context switch 2-3% more
instructions, `coremark` and `script/icount` unmoved. Proved by two threads
doing vector work on one core, falsified against a kernel with the save removed. The target flip is
**not** taken: it is an ABI and calef's, and the block ends with what it would take, buy and cost.
