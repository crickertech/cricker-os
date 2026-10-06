# Call-frame information in hand-written assembly

*Name: provisional.*

Twelve hand-written `.s` files under `kernel/src/arch/` carried zero call-frame information (CFI)
before this milestone: no `.cfi_startproc`, no `.type`, no `.size`. A thirteenth,
`kernel/src/arch/x86_64/fp.s`, was missing from the milestone's own file list but is the same kind
of file and got the same treatment; see the BUGS section. This note records what CFI is, why these
files lacked it, what the directives say on each architecture, the before-and-after evidence, and
what is still not unwindable and why.

## What CFI is, in one paragraph

A compiled function's prologue usually does something like "push these registers, make room for
locals" and its epilogue undoes it. A debugger that wants to print a backtrace, or GDB's `finish`,
or a language's own unwinder, has to reconstruct, at an arbitrary instruction in the middle of that
function, three things: where the caller's stack frame starts (the canonical frame address,
CFA), where each register the function has saved lives relative to it, and where the return address
is. CFI is that description, written once per function, as a small program the debugger replays.
The compiler emits it automatically for every Rust function (that is what let the transcripts below
show line numbers and arguments for `kernel::sched::schedule` without any help from this milestone).
Hand-written assembly gets none of that for free: the assembler has no idea what a `stp x29, x30,
[sp, #-32]!` means about frames unless told, via `.cfi_*` directives.

The output format is `.eh_frame`. Despite the name (inherited from C++ exception handling, which
this kernel does not have and does not want), it is the ordinary ELF section a debugger reads CFI
from, and it is not tied to unwinding actually happening at runtime; `panic = "abort"` means this
kernel never unwinds its own stack, but a debugger attached from outside still can. That is the
whole reason this milestone is about tooling, not correctness.

## Why they lacked it

Nobody had disabled it on purpose in the assembly. `notes/scripts.md` and `xtask`'s `gdb` command
already existed, printing `break kernel_main` / `break _boot` instructions as though a debugger
session were a normal thing to run here. The gap was one line, in all three link scripts, discarding
`.eh_frame*` outright with the comment "we never unwind" -- true about the kernel's own runtime
behavior, and irrelevant to a debugger reading the ELF from outside. That line discarded not only
what this milestone was asked to add, but every CFI record the *compiler* had already been emitting
for ordinary Rust functions the whole time. See "The link-script discovery" below.

## The link-script discovery, and what was tried

Adding `.cfi_startproc`/`.cfi_endproc` to the twelve files compiled cleanly and produced correct
`.eh_frame` content -- but `llvm-objdump --dwarf=frames` on the linked kernel showed an empty
`.eh_frame` and an empty `.debug_frame`, for every architecture. `kernel/link-{aarch64,riscv64,
x86_64}.ld` each had:

```
/DISCARD/ : {
    ...
    *(.eh_frame*)         /* stack-unwinding tables; we never unwind */
}
```

Removing the line was not enough by itself. Three placements were tried, in this order, and the
first two were rejected:

1. Left unmentioned (default orphan placement). LLD keeps `.eh_frame*` ALLOC (the compiler
   marks it so; unlike `.debug_info`/`.debug_line`, which are never ALLOC and need no script
   attention) and slots it in with the other read-only data, on aarch64 landing between
   `.rodata` and `.data` -- before `__image_end`. Measured: `__image_size` (the value this
   kernel's own boot code writes into the arm64/RISC-V Image header for a real bootloader) grew by
   the CFI's own size, about 100 KiB, for a debugging convenience with nothing to do with how much
   RAM the image needs.
2. An explicit `(NOLOAD)` output section, to move it out of the way and off ALLOC. This turned
   it into `SHT_NOBITS`, the same representation `.bss` uses: LLD dropped the actual bytes, and
   `llvm-objdump --dwarf=frames` went back to empty. Wrong mechanism: NOLOAD is for reserving
   address space whose *content* does not matter, and CFI content is the entire point.
3. An explicit `(INFO)` output section, to keep the bytes and drop ALLOC. This let LLD place
   the section far from `.text` (multiple megabytes away in one measured build) and broke a
   `R_AARCH64_PREL32` relocation elsewhere in the image outright: `rust-lld: error: relocation ...
   out of range`.

**What worked**: an ordinary output section (no `NOLOAD`, no `INFO`), placed in the script text
*after* `__image_end`/`__image_size` are computed, rather than left to the orphan-placement
heuristic:

```
.eh_frame : { KEEP(*(.eh_frame)) KEEP(*(.eh_frame_hdr)) }
```

This stays ALLOC (so the bytes survive) and keeps LLD's normal sequential layout (so it lands right
after `.bss`/the per-core stacks, close enough to `.text` that nothing overflows), while costing
`__image_size` nothing, because that symbol is already assigned by the time this section exists.
**Verified, not assumed**: a from-scratch build of the commit immediately before this milestone
(`ece6d72c`) gives `__image_size = 0x350000` on aarch64; this tree, with CFI present, gives the
same `0x350000`. The flat `Image` binary the two builds' `objcopy -O binary` produce (aarch64's
QEMU/board boot artifact) is byte-for-byte identical, `cmp` confirmed.

**aarch64 also strips `.eh_frame`/`.eh_frame_hdr` back out of that flat binary**, in
`helpers/qemu-runner-aarch64.sh` and `xtask/src/inspect.rs`'s `image()`, with
`llvm-objcopy --remove-section`. This is the one place a real bootloader or QEMU's `-kernel Image`
path actually loads the bytes into memory; the full ELF `gdb <elf>` reads keeps the content either
way.

**riscv64 and x86_64 do not get the same strip**, and that is a real, if minor, gap: see BUGS.

## What the directives describe, per architecture

### The easy case: ordinary functions

`fp_save`/`fp_restore` (all three architectures) and `dispatch_on_interrupt_stack` (all three) are
reached by a real call and return by a real `ret`. Their CFI is the same shape a compiler would
emit: `.cfi_def_cfa_offset` tracking each push/pop, `.cfi_offset` naming where each saved register
lives, `.cfi_restore` undoing it. Nothing architecture-specific to say beyond what the code itself
already does.

### The interesting case: a context switch is a function that returns somewhere else

`switch_to` (aarch64, riscv64, x86_64) swaps `sp`/`rsp` to a different thread's stack mid-function
and then pops "its" callee-saved registers off that stack. The naive worry is that CFI can only
describe one function's frame, not a jump to an unrelated one. It turns out no special handling
is needed at the swap point, for a reason worth stating precisely: `.cfi_def_cfa_offset N`
defines the CFA as "the CURRENT stack-pointer register's value, plus N" -- a live formula, not a
frozen address. `next_context` is, by construction (`Context::for_kernel_thread`, `for_user_thread`,
or a previous `switch_to` call on that thread), exactly the stack-pointer value that thread's own
identical push sequence left behind. So the same formula, evaluated after the swap, correctly gives
*that thread's* call-site CFA, and the same `.cfi_offset` rules point at the very slots its own
prologue (or the synthetic frame) put values in. One continuous CFI program, no split, is both
simpler and more correct than trying to close and reopen the description at the swap.

`dispatch_on_interrupt_stack` needs a related but distinct trick: it swaps onto a per-CPU interrupt
stack that is not shaped like its own frame, runs a call there, and swaps back. Here the CFA is
restated in terms of a callee-saved register (`x19`/`s0`/`rbp`) that holds the pre-swap value and
does not move again until it is restored (`.cfi_def_cfa_register`), so the formula stays valid on
both sides of a stack that briefly isn't the one CFA was originally defined against.

`thread_trampoline`/`user_entry_trampoline` (and x86_64's pair) are the other half of the same
trick: a fake switch frame (`Context::for_kernel_thread`, etc.) whose "return address" is the
trampoline's own entry, so the very first `ret`/resume of a brand-new thread lands there. The link
register at that instant is self-referential, not a real caller -- `context.rs` already says so in
prose ("no caller: the backtrace ends here"); `.cfi_undefined` on the return-address register says
the same thing to the unwinder.

### The hard case: a trap is not a call, and two architectures cannot fully describe it

A vector entry (aarch64's `vectors.s`), a trap entry (RISC-V's `trap.s`), and an interrupt/exception
stub (x86_64's `trap.s`) all build a signal frame: the interrupted context's register file,
landed on the stack by hardware plus a macro, not by a `push` a debugger can walk backwards through
by convention. `.cfi_signal_frame` marks the FDE as one of these (GDB does not decrement the PC by
one when symbolizing it, and other tools know not to assume an ordinary call convention). The GP
registers are describable everywhere with plain arithmetic on the macro's own stores, so all three
recover x0-x29/a0-a7,s\*/rax-r15 correctly from any PC in the handler. The interrupted PC is the
part that differs by architecture:

- **x86_64 can describe it fully**, and this is the pleasant surprise of the three. The hardware
  frame puts the interrupted RIP and RSP at fixed offsets from the software frame's own CFA, and
  the SysV DWARF register mapping already treats register 16 (RIP) as the ordinary return-address
  column and register 7 (RSP) as an ordinary describable register. So `isr_common` states
  `.cfi_offset 16, 16` / `.cfi_offset 7, 40` and the unwind genuinely continues past the trap, into
  whatever Rust function was interrupted, and from there into *its* caller, transitively -- see the
  "hit 6" transcript below, which walks `switch_to -> schedule -> ipc_receive -> syscall::invoke ->
  syscall::dispatch -> exception_body -> exception_dispatch -> <signal handler called>` and stops
  there cleanly, the boundary correctly marked rather than silently wrong.
- **aarch64 cannot, today, though the DWARF spec has an answer.** The interrupted PC lives in
  `elr_el1`, a system register the hardware does not restore into `x30`/`lr` the way a `bl` does.
  AArch64's own DWARF register mapping anticipates exactly this: register 33 is `ELR_mode`, defined
  for describing an asynchronously-created (signal/exception) frame (`aadwarf64.rst`, ARM's DWARF
  for the ARM 64-bit Architecture). `vectors.s` states `.cfi_return_column 33` and
  `.cfi_offset 33, -24` -- spec-correct, and there for whichever tool honors it. GDB is not one
  of them, as of this writing: it hardcodes column 30 (`x30`) as the AArch64 return-address column
  and does not consult `.cfi_return_column`
  ([sourceware.org/pipermail/gdb/2023-January/050488.html](https://sourceware.org/pipermail/gdb/2023-January/050488.html)).
  So the honest choice, and the one this milestone makes, is `.cfi_undefined x30` at the trap
  boundary: not repurposing column 30 to smuggle `elr_el1` through it, which would make GDB's
  unwind *work* today at the cost of lying about where the real `x30` register lives (a query like
  `p $lr` in that frame would then read `elr_el1`'s value instead). Wrong CFI is worse than none;
  leaving the column undefined tells GDB, correctly, "stop here", which is exactly what the "after"
  transcript below shows it doing.
- **RISC-V cannot, and has no spec-level answer to reach for either.** The interrupted PC lives in
  `sepc`, and unlike AArch64, the base RISC-V DWARF register mapping has no reserved pseudo-register
  for a CSR the way `ELR_mode` exists. `trap.s`'s `.cfi_undefined ra` is therefore not a workaround
  for a tooling gap (as aarch64's is); it is the only currently-describable answer.

### `switch_to`'s epilogue restoring `x2`/`sp` itself (RISC-V)

One more RISC-V-specific note: `trap_return`'s very last register load, `ld x2, 2*8(sp)`, changes
the register the CFA formula is defined in terms of to an unrelated value (the interrupted context's
own `sp`), the same move `switch_to` makes deliberately. `.cfi_restore x2` there is correct (it
describes recovering the *value*, independent of the separate CFA-tracking mechanism), and it is the
last instruction before `sret`, so nothing after it needed a further rule.

### `image_header.s` is data, not code

Per the brief's own warning: `kernel/src/arch/aarch64/image_header.s` is the arm64 Image header, a
64-byte data structure the bootloader reads, with one instruction (`b _boot`) grafted onto its front
so the entry point can also be byte 0 of it. It carries no CFI, `.type`, or `.size`, and a
`CFI-EXEMPT:` comment says why, in the file, where a reader (or `script/lint`'s new check) meets it.
riscv64's `_start` has the identical shape (a Linux Image header with one leading `j`), for the same
reason and the same treatment.

### 16-bit and 32-bit code (x86_64 only)

`x86_64/boot.s`'s trampolines run in real mode and 32-bit protected mode for most of their length,
before the far jump into 64-bit mode. No CFI anywhere in those stretches, for two reasons: they have
no valid stack for most of it (no caller, nothing worth describing as a frame), and this ELF's own
`.eh_frame` is emitted and read as 64-bit DWARF throughout, so annotating a 16-/32-bit instruction
stream inside it would not obviously mean what the rest of the file's CFI means. `long_mode_entry`/
`_start_high` and `ap_long_mode_entry` (all `.code64`) get real CFI once execution is running in the
width the rest of the file is.

## Before and after: the evidence

Captured on aarch64 (the only architecture `cargo xtask gdb` currently drives), with GDB 17.2,
against two builds of the identical source tree at commit `ece6d72c` (immediately before this
milestone) and this branch's tip. `cargo xtask gdb` boots the kernel under QEMU with `-s -S` and
prints the same instructions `notes/scripts.md` already documents; a real `gdb <elf>` /
`target remote :1234` session, not a paraphrase.

### Case 1: a brand-new thread's first run (both builds agree, and should)

```
(gdb) break thread_entry
(gdb) continue
(gdb) bt
```

Before:
```
Thread 3 hit Breakpoint 1, kernel::thread::thread_entry (closure=..., call=...) at kernel/src/thread.rs:913
#0  kernel::thread::thread_entry (closure=..., call=...) at kernel/src/thread.rs:913
#1  0xffff0000400d94a4 in thread_trampoline ()
Backtrace stopped: previous frame identical to this frame (corrupt stack?)
```

After:
```
Thread 3 hit Breakpoint 1, kernel::thread::thread_entry (closure=..., call=...) at kernel/src/thread.rs:913
#0  kernel::thread::thread_entry (closure=..., call=...) at kernel/src/thread.rs:913
#1  0xffff0000400d94a4 in thread_trampoline ()
```

Both stop at `thread_trampoline`, and that is correct in both cases: `Context::for_kernel_thread`
fakes this thread's very first frame with no real caller (its own comment: "no caller: the backtrace
ends here"). The before transcript stops there by accident (no CFI existed to say anything); the
after transcript stops there on purpose (`.cfi_undefined lr`), and the difference shows up as GDB no
longer printing "corrupt stack?" about a frame that was never corrupt, only undescribed. This case
was chosen first and reported honestly even though it does not show an improvement in the *reach* of
the backtrace, because a fabricated frame that has no caller is not supposed to unwind further, and
claiming otherwise here would be exactly the invented-CFI failure this milestone was warned against.

### Case 2: resuming an already-running thread (the case that matters)

Breaking on `switch_to` itself and repeating `continue`/`finish`/`bt` across fourteen hits once the
boot tour's initial thread-spawning settles down. The compiler's own CFI for ordinary Rust
functions was ALSO being discarded before this milestone (the same one link-script line
discarded everything named `.eh_frame*`), so GDB's frames 0-3 below come from its frame-pointer
fallback heuristic, not from any FDE -- there was none. That heuristic is what makes the before
transcript dramatic rather than merely incomplete:

Before (second hit; this is not abbreviated for effect, the real output kept going until it was
killed at frame #9976, three minutes and forty-five seconds after the `bt`):
```
Thread 3 hit Breakpoint 1, 0xffff0000400d9450 in switch_to ()
#0  0xffff0000400d9450 in switch_to ()
#1  0xffff000040090970 in kernel::sched::schedule () at kernel/src/sched.rs:2145
#2  0xffff00004008c8e8 in kernel::sched::preempt_if_needed () at kernel/src/sched.rs:4639
#3  0xffff0000400b1718 in kernel::arch::aarch64::exceptions::exception_dispatch (frame=..., index=5) at kernel/src/arch/aarch64/exceptions.rs:351
#4  0xffff0000400d9ae0 in exception_vectors ()
#5  0xffff0000400d9ae0 in exception_vectors ()
#6  0xffff0000400d9ae0 in exception_vectors ()
[... repeats, identically, past #9976, never terminating on its own ...]
```

Frames 0-3 recover correctly because ordinary Rust functions still set up an AAPCS64 frame-pointer
chain (`x29`) regardless of CFI, and GDB's fallback walks that chain when it finds no FDE. Frame 4
is where the chain walks into `exception_vectors`, which builds a raw trap frame rather than a
conventional `x29` link -- and the heuristic, with nothing to tell it otherwise, reads some stale or
misinterpreted value as "the next frame pointer," gets the same address back, and loops forever.
This is the sharpest illustration in this whole milestone of why "no CFI" is not merely "less
information": GDB's own fallback is willing to trust a frame-pointer-shaped hand-written assembly
function that isn't one, and it does not know when to stop.

After (a representative hit, of a kernel thread parked in `sched::ipc_receive` waiting on an IRQ
endpoint, resumed and then re-scheduled by a timer preemption):
```
Thread 4 hit Breakpoint 1, 0xffff0000400d9450 in switch_to ()
#0  0xffff0000400d9450 in switch_to ()
#1  0xffff000040090970 in kernel::sched::schedule () at kernel/src/sched.rs:2145
#2  0xffff00004008fd70 in kernel::sched::ipc_receive (ep=2) at kernel/src/sched.rs:2704
#3  0xffff0000400bfdac in kernel::syscall::irq_wait (intid=79) at kernel/src/syscall.rs:1103
#4  0xffff0000400bf7fc in kernel::syscall::invoke (frame=..., slot=1, method=0, ...) at kernel/src/syscall.rs:452
#5  0xffff0000400bfcf0 in kernel::syscall::dispatch (frame=...) at kernel/src/syscall.rs:91
#6  0xffff0000400b15bc in kernel::arch::aarch64::exceptions::exception_body (frame=..., index=8) at kernel/src/arch/aarch64/exceptions.rs:432
#7  0xffff0000400b16f0 in kernel::arch::aarch64::exceptions::exception_dispatch (frame=..., index=8) at kernel/src/arch/aarch64/exceptions.rs:341
#8  <signal handler called>
Backtrace stopped: frame did not save the PC
```

This is the transcript that "proves it bought something." Frame 0 through 7 is new: seven real Rust
frames a debugger could not previously see past `switch_to` at all. `<signal handler called>` is
GDB recognizing `.cfi_signal_frame` on the vector-entry FDE and labeling it, rather than silently
misreading it as an ordinary call. And the stop after it -- "frame did not save the PC" -- is the
honest limit this note's "hard case" section describes: `.cfi_undefined x30` at the trap boundary,
because GDB does not honor the spec-correct `ELR_mode` column that would let it go one frame
further, into whatever kernel code took the IRQ. Other hits during the same run showed the same
shape resuming through `sched::preempt_if_needed`, `sched::yield_now`/`run_idle`, and
`sched::depart`/`exit`, each unwinding cleanly to its own real caller and stopping at the same kind
of boundary (a trap frame, or `kernel_main`'s own `-> !` entry, correctly reported as the bottom).

## Footprint

`script/fastpath-footprint` measures ELF symbol sizes in `.text`; CFI lives in `.eh_frame`, a
different section. Run before and after, on the same machine, same profile (release, which is what
the gate builds): byte-identical on every reported number (`ipc_send_receive` 6300, `ipc_call_reply`
8234, `ipc_fastpath` 8234, `syscall_entry` 1701). The gate's own printed "+N% against baseline" lines
are unchanged too, which makes sense: they compare against a stored reference figure from an earlier
point in the project's history that has nothing to do with this branch, and this branch moves
neither. The kernel's `__image_size` numbers (a different, and initially surprising, place this
milestone risked moving something) are covered in "The link-script discovery" above: verified
byte-for-byte, not merely argued.

## BUGS

- **The task that started this milestone named twelve files; there are thirteen.**
  `kernel/src/arch/x86_64/fp.s` (FXSAVE/FXRSTOR for the FPU/SSE register file) was omitted from the
  original enumeration. It is a real hand-written `.s` file under `kernel/src/arch/`, of the exact
  same shape as the other two `fp.s` files, and got the same CFI. Parity (DECISIONS's rule 5, cited
  in AGENTS.md) would have required it eventually regardless; this just does it now rather than
  leaving a gap the new `script/lint` check would otherwise have had to special-case.

- **riscv64 and x86_64 do not strip `.eh_frame` out of the image QEMU actually boots**, unlike
  aarch64. Neither port has a flat-Image/objcopy step at all: QEMU's `-kernel` loads each ELF's
  `PT_LOAD` segments directly (see each port's own `helpers/qemu-runner-*.sh` header comment), so
  the CFI (currently a few hundred bytes to ~30 KiB depending on how much the compiler emits for a
  given build) rides along into guest RAM. This is harmless in a QEMU VM with the usual hundreds of
  megabytes and was not deemed worth introducing a new strip-then-boot step for two ports that do
  not yet have a real board target the way aarch64's VisionFive 2 partly does. If either port grows
  a real flat-image / bootloader path, this is the thing to revisit.

- **GDB does not honor AArch64's spec-correct `ELR_mode` return column** (DWARF register 33;
  `.cfi_return_column 33` in `vectors.s`), so a backtrace on this architecture stops one frame short
  of RISC-V's and x86_64's theoretical ceiling: it cannot continue past a trap into the interrupted
  kernel code the way x86_64's transcript above does. This is a fact about the tool this project
  actually uses, cited to its own bug report
  ([sourceware.org/pipermail/gdb/2023-January/050488.html](https://sourceware.org/pipermail/gdb/2023-January/050488.html)),
  not a limitation this milestone introduced or could fix from the assembly side. Re-test if GDB
  ever gains AArch64 return-column support; the directive is already there, doing nothing useful
  today and no harm either.

- **RISC-V has no spec-level answer at all for the trap-PC case**, unlike AArch64's (merely
  unconsumed) `ELR_mode`. The base RISC-V DWARF register mapping has no reserved pseudo-register for
  `sepc`. `trap.s`'s trap boundary is therefore `.cfi_undefined ra` with no forward-looking
  alternative to offer, and closing this gap would need a convention this project cannot mint
  unilaterally (it would want to match whatever, if anything, the RISC-V toolchain ecosystem settles
  on, since a project-local DWARF register number would not mean anything to a debugger built
  against the standard mapping).

- **This is a presence check, not a correctness check** (the same limit `script/citations` and the
  roadmap's `## Revisit` check are honest about for their own claims). `script/lint`'s new "hand-
  written asm carries CFI" check confirms a `.s` file with an exported symbol also contains at least
  one `.cfi_startproc`, at file granularity. It cannot confirm the CFI is correct, that every
  function in a multi-function file has its own directives, or that a numeric offset is right rather
  than merely present. The evidence for correctness in this milestone is the transcripts above, not
  the gate; a future file that adds a `.cfi_startproc` and then gets the offsets wrong would pass
  this check and fail a debugger, silently.

- **Only one architecture's transcripts are captured.** `cargo xtask gdb` currently only drives
  aarch64 (`xtask/src/main.rs`'s `RUNNER`/`TARGET` are aarch64-only). x86_64 and riscv64's CFI is
  built and inspected via `llvm-objdump --dwarf=frames` (see the design work this milestone's pull
  request records) but not exercised against a live GDB session the way aarch64's is here. A
  `cargo xtask gdb --arch riscv64|x86_64` (or the equivalent) is a reasonable follow-on if this
  becomes a recurring need; today it would have to be a hand-rolled QEMU `-s -S` invocation per
  port, which is exactly the kind of thing worth a lane rather than a footnote.
