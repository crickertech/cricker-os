---
status: DECIDED
raised: 2026-08-24
decided: 2026-08-24
ratified_by: calef
---

# 124. Ratify the x86_64 syscall ABI

calef, 2026-08-24, ratified as written: `rax` carries the syscall number;
arguments ride `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`. Raised the same day by calef, after milestone
161's ring-3 lane (pull request #464, merged) made the x86_64 syscall ABI genuinely **spoken** rather
than only written down: a real `syscall` executed from CPL 3 reaches the portable dispatcher through
`TrapFrame::{syscall_nr, arg, set_arg}` and a real answer comes back. The ABI itself had never been
formally decided, and #464 had already merged, so there was no open pull request left to carry a
`needs-architect` label. The number is **provisional**, minted against the current
`design/decisions/` index (highest existing was 123 at the time of writing).

What is blocked: nothing today. Milestone 161's item 4, the kernel test suite, has not
started. What this decision prevents is a *later* fork of that work treating an unratified ABI as
settled by compiling real programs against it, with nobody having looked at the choice first.

## What is being decided

Whether the x86_64 syscall ABI is ratified as written: `rax` carries the syscall number; arguments
ride in `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`. This is exactly what's already implemented in
`kernel/src/arch/x86_64/exceptions.rs`'s `TrapFrame::{syscall_nr, arg, set_arg}`, and it is the
syscall surface: DECISIONS §10 and §16 already establish that surface as "a boundary rather than a
habit," not an implementation detail a lane settles on the way past.

## What this tree already does in the analogous case

Both other architectures faced the identical question and answered it the same way: adopt the
platform's own real hardware/OS syscall convention rather than invent one.

- aarch64: `TrapFrame::syscall_nr`'s own doc comment states it plainly: *"The syscall number the
  caller passed. aarch64 ABI: register `x8` (DECISIONS §10)."* Arguments ride `x0`..`x5`.
- riscv64: `TrapFrame::syscall_nr`'s doc comment: *"The syscall number the caller passed. RISC-V
  `ecall` ABI: register `a7` (`x17`)... This, with `arg`/`set_arg`, is the resolution of the
  syscall-ABI leak flagged during the port (DECISIONS §17): aarch64 uses x8 + x0..x5, RISC-V uses a7
  + a0..a5, and each maps its own registers."* Arguments ride `a0`..`a5` (`x10`..`x15`).

Both are the same registers Linux uses for the syscall number and arguments on those ISAs, not
something invented for this kernel. `x86_64`'s own code comment already states the same move for the
third architecture, and is worth quoting in full since it is the actual case being ratified:

> `rax`, following Linux, and that choice is worth stating because it is the one place a third
> architecture could gratuitously invent a third convention. The tree's shape is one number register
> plus six argument registers, with argument 0 doubling as the return (aarch64: `x8` + `x0`..`x5`;
> RISC-V: `a7` + `a0`..`a5`). Applying that shape to `x86_64`'s own `syscall` convention gives `rax`
> + `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`.
>
> **`r10` rather than `rcx` is not a preference**, it is the instruction: `syscall` overwrites `rcx`
> with the return address and `r11` with the caller's RFLAGS, so the fourth argument cannot ride in
> the C ABI's fourth register. Every `x86_64` kernel makes the same substitution for the same reason.

(`kernel/src/arch/x86_64/exceptions.rs`, `TrapFrame::syscall_nr`'s doc comment.)

So the pattern across all three architectures, without exception, is: adopt the ISA's own standard
syscall calling convention, unmodified. `rax` + `rdi`/`rsi`/`rdx`/`r10`/`r8`/`r9` is x86-64 Linux's
own `syscall(2)` convention, the same one glibc and musl use for a raw syscall, not invented for
this port; the `r10`-not-`rcx` substitution is not a choice at all, it is what the `syscall`
instruction's own side effects force on every x86_64 kernel that uses it.

## Recommendation

Ratify as written. This is not a novel design choice needing scrutiny; it is the third
architecture adopting its own platform's standard convention, the identical move already made (and
already decided, under DECISIONS §10) for the first two. There is no alternative on the table because
none of the reasoning above leaves room for one: the register assignment for the number and the
first three arguments follows the same one-number-plus-six-arguments shape both other architectures
already use, and the `r10` substitution is the `syscall` instruction's own behavior, not a design
fork.

## How reversible is this, and who has already acted on it

The syscall surface is exactly the category `AGENTS.md` calls out as expensive to change once
something depends on it ("anything two programs agree on"). Nothing currently depends on this ABI
beyond the one hand-assembled probe program milestone 161's ring-3 lane wrote
(`kernel/src/arch/x86_64/ring3_probe.s`), which item 4's own lane is expected to delete. So this is
cheap to decide now and expensive to leave ambiguous once item 4 starts compiling real userspace
programs against it, exactly the moment this decision should already be settled, not still open.
