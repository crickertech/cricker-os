# The FS server's stack is sized by measurement, because guessing it cost a day

An appendix to [notes/fs-server.md](../fs-server.md). It holds the stack overflow that took a day
to find, what it taught about the instruments, and the measurements the grant now rests on.

## Why the stack is deep

RedoxFS recurses. A single `Transaction::read_block::<TreeList<..>>` activation carries a whole
4096-byte block plus scratch, so one frame is 8 KiB, and a tree walk stacks a dozen or more of them.
The FS server therefore gets a deep stack. `run` maps one page at `USER_STACK_VA`, and
`fs_service::wire_servers` maps `FS_STACK_PAGES` more directly below it, out of fresh frames. So the
process sees one contiguous run down from `USER_STACK_TOP`.

## How the old number failed

That number used to be 32 (33 pages, 135,168 bytes), chosen to be comfortably above the
read-and-write path. Adding `CREATE` and `TRUNCATE` in milestone 31 (a capability shell) phase 2 took
one more level of tree recursion, and it was 528 bytes short. The FS server ran off the bottom of its
stack mid-request, and the kernel killed it, correctly and legibly:

```text
  user thread 8589934629 killed: Data abort from a lower EL
    pc 0x00000000004000b0   far 0x00000000004dfe90   user sp 0x00000000004dfe90   esr 0x92000047
```

`far == sp`, one page below the bottom of the mapping. `pc` disassembles inside
`read_block::<TreeList<TreeList<TreeList<BlockRaw>>>>`, in the middle of its two 4 KiB `sub sp`
instructions. Nothing about that is ambiguous once you look at it. What was not legible was anything
downstream, and that is the part worth carrying off:

- The std client sat `Blocked` on a `CALL` that nobody would ever answer, because the endpoint's only
  receiver had just died. Blocking IPC has no "the server is gone" reply. So a dead server is
  indistinguishable, from a client, from a slow one.
- The suite's no-progress heartbeat credits work by *any* running thread. Earlier tests had left
  processes spinning on other cores, so it saw a healthy system for as long as you cared to wait.
- The only instrument that could fire was the per-test wall-clock ceiling. It fired at the budget,
  and a ceiling failure reports the budget, not the cost. "std_fs ran 914 s against a 900 s budget"
  was read as evidence of honest slowness. It sent an investigation looking for a slow path in a test
  whose server had been dead since second three.

## What came out of it

Two things. First, the thread dump now prints each thread's address-space root
(`sched::dump_threads`). Every user program links at `0x40_0000`, so a bare `pc` resolves plausibly
against several binaries at once. Threads sharing a root are one process, and distinct roots are
distinct processes. That is what separates a leftover spinner from the process under test.

Second, the stack size is now a measurement. The kernel fills every FS-server stack page with a
poison word before the process starts. `fs_service::fs_stack_used` reports the deepest word that no
longer reads as poison. Measured across a mount, reads, writes, a create and two truncates:

| leg | high-water | of grant | headroom | when |
|---|---|---|---|---|
| aarch64 | 135,696 bytes | 397,312 | 66% | 2026-07-30, milestone 31 phase 2 |
| riscv64 | 135,824 bytes | 397,312 | 66% | 2026-07-30, milestone 31 phase 2 |
| aarch64 | 127,408 bytes | 397,312 | 68% | 2026-07-30, milestone 37 |
| riscv64 | 127,536 bytes | 397,312 | 68% | 2026-07-30, milestone 37 |

Both of the first pair were over the old 135,168-byte grant, so both legs were broken. The riscv leg
needs slightly more for the same recursion, which is why the number is measured per ISA rather than
assumed to transfer. `the_redoxfs_servers_stack_still_has_headroom` (both ISAs) prints it every run
and fails under a quarter left. So the next verb that deepens a tree walk fails with a number instead
of a mystery.

The second pair is 8 KiB lower, and the cause is not attributed. That is recorded rather than
smoothed over, because an unexplained move in a safety instrument is worth more attention than a
comfortable one. 8 KiB is exactly one `read_block::<TreeList<..>>` activation, so it reads like one
less level of tree recursion on the deepest path. Milestones 41 through 45 landed between the two
measurements, and any of them could have changed codegen. Nothing in milestone 37 (prove RedoxFS's
crash consistency) touches the read path. The crash test's own servers run against a shallower
image, so they can only lower the maximum by not raising it. The number to trust is whichever the
gate last printed. The assertion that matters (a quarter of the grant still free) is unaffected
either way.

Since milestone 37 the high-water is a maximum over every FS server a boot starts. That now includes
the process that mounts a crashed disk. A mount that has to walk back a generation is the case most
likely to recurse further than a clean one. So it is exactly the case this instrument should be
watching, and until then it was not being watched at all.

The grant is 96 extra pages. That is deliberately well above the measurement rather than just above
it. Recursion depth here tracks the *tree* depth, which grows with the image, so a size proven on a
16 MiB fixture is not proven on a real disk. 384 KiB of frames once per boot is cheap beside the FS
server's 8 MiB heap budget.

## Still open

A client of a dead server blocks forever, named rather than fixed. The fault endpoint of §26 (thread
death becomes a message a supervisor holds) is the mechanism that would turn that into a message a
supervisor can act on. Wiring the FS service into a supervision tree is the problem of milestone 23
(a capability-routed component OS with live replacement), not this one's. Until then, "the server
died" presents to a client as "the server is taking a while". That is the same shape of invisibility
this whole appendix is about.
