---
status: NOT-STARTED
raised: 2026-09-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 379. Select E3's padding at boot rather than at compile time, so the experiment is one binary

Named by calef on 2026-09-04 while planning the first E3 bench session, on
learning it wanted two cards and he has one; promoted by milestone 433 on 2026-09-19. Checked
against the tree that day: `fastpath_pad` is still a Cargo feature (`kernel/Cargo.toml`) with no
runtime selector anywhere, so E3's two arms are still two binaries. The session this was written to
spare went ahead in the expensive shape on 2026-09-04, six card rewrites to interleave six boots,
which is milestone 375, so the cost this names is now measured rather than predicted.

The mechanism it would copy shipped the same week.

What the work is. `fastpath_pad` is a Cargo feature, so E3's padded and un-padded arms are two
different kernels. The comparison therefore rests on trusting that two builds differ only where
intended, and the bench session pays for it twice over: six card rewrites to interleave the boots,
and no way for a booted machine to say which arm it is running.

If the padding were selected at boot instead, a token on the command line, the way milestone
243's `screen=` now carries the framebuffer geometry from `uefi_loader` to the kernel, the session
becomes one card, one write, six boots, and the two arms are provably the same binary.

Why the mechanism is already proven. Milestone 243 established the whole path on 2026-09-04:
PVH's `cmdline_paddr` is a field the format has always had, `machine_discovery::x86_64::BootInfo` has
decoded it since milestone 87, and nothing read it until 243 did. On riscv64 the boot script is
milestone 218's `boot.scr`, which is ours to write, so the token has an obvious home.

What it would cost, and the honest part. Resident dead code selected at runtime is not the same
experiment as resident dead code linked in: a branch that skips the padding still has the padding in
the image, which is the point (E3 perturbs the *footprint*, and an un-fetched byte still occupies a
cache line only if it is on the same line as a fetched one). **Whoever takes this has to say whether
a runtime-selected pad perturbs what E3 means to perturb**, and the answer is not obviously yes.
That is the design question and it is the whole of the work; the plumbing is a token and a branch.

What it would buy beyond this one session. Every future A/B on the board has the same shape, and
this tree now has several build-time flags that select an experiment rather than a product:
`soak`, `job_mix`, `reboot_soak`, `single_hart`, `fastpath_pad`. A proposal already records that
**nothing in CI compiles any of them**
(`design/roadmap/373-board-only-features-nothing-compiles.md`, whose own status line records that
`script/lint` has since closed half of that claim and which half is left), which is the same
brittleness from the other side.

## Index row

`fastpath_pad` is a Cargo feature, so E3's padded and un-padded arms are two different kernels, and
the comparison rests on trusting that two builds differ only where intended while the bench session
pays for it twice: six card rewrites to interleave the boots, and no way for a booted machine to
say which arm it is running. Selecting the padding from a boot token instead, the way milestone
243's `screen=` carries framebuffer geometry from `uefi_loader` to the kernel, makes it one card,
one write, six boots, with both arms provably the same binary. The mechanism shipped the same week
on both the PVH path and milestone 218's `boot.scr`. The design question is the whole of the work
and it is not obviously answerable yes: resident dead code selected at runtime is not the same
experiment as resident dead code linked in, and whoever takes this has to say whether a
runtime-selected pad perturbs what E3 means to perturb. Every future A/B on the board has the same
shape.
