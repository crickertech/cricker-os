# 604. The builder's scratch cursor is bounded

**Status: BUILT.** Promoted on 2026-09-26 from the proposal `the-builders-scratch-cursor-is-bounded`,
which the lane for milestone 206 (a program image has under 896 KiB) filed while siting every window
on the address-space map, and built the same day by lane `milestone/scratch-cursor`. *(Number
provisional: minted by the lane, to be confirmed at merge. 596 to 603 were held on `main` and open
pull requests when it was taken.)*

## The finding, and a correction to it

`supervision_protocol`'s loader maps every page it fills for a child into the builder's own address
space, at a cursor that started at `0x1000_0000`, advanced one page per page built, and never came
back, because nothing in the ABI unmaps (DECISIONS §162 (whether a holder can give up a mapping) is
open). The proposal said the progenitor's cursor reaches the kernel's initrd window at `0x2000_0000`
after about a hundred `ripgrep`-sized spawns, and every build fails after that. That is on the
customer path: `rg` at the prompt is milestone 595's (the shell runs a `std` program) goal.

**The arithmetic, computed from the tree's constants rather than measured on a progenitor:**

- `ripgrep`'s image spans 668 pages on aarch64 (`0x40_0000..0x69C_000`, notes/ripgrep-on-nife.md),
  and 669 scratch pages a spawn on `x86_64` and `riscv64`, which add the timebase page. Stack pages go
  straight into the child and cost the builder nothing.
- The window to the initrd is 65,536 pages, less the boot servers', which is about 97 spawns.
- **But the page tables run out first.** Every 512 scratch pages need a new last-level table from the
  progenitor's own budget, `INIT_OWN_PAGES`, 128 pages. `job_undertaker` is built out of the same
  budget (about 24 to 28 pages: its image, twelve stack pages, its tables, address space and thread),
  and the first image request takes two more tables. That leaves about 100 tables, 51,000 pages:
  **about seventy-five spawns**, failing as out of memory rather than already mapped. The records
  that said "a hundred" now say this.

The count is reasoned, not reproduced on a progenitor: `rg` is in no CI archive (milestone 121's
reasons), and the job pool cannot yet hold a program that size (`grant_plan::STD_REGION_PAGES` is
384). The same failure was reproduced in a builder the test controls, below.

## The fix: the kernel was already giving the pages back

The proposal's gate was `DECISION`, on the reasoning that bounding the cursor needs an unmap. It
does not. A scratch page maps a frame retyped from the child's region, and destroying that region
revokes every mapping of its pages, the builder's included (DECISIONS §13 (frame revocation)). So a
scratch page is free again once its child is reaped. Only the cursor did not know it.

- **The cursor lives in a fixed window and wraps.** `supervision_protocol::SCRATCH_WINDOW`,
  `0x1000_0000..0x1400_0000`, 64 MiB, 16,384 pages, whose last-level tables cost a builder at most
  `SCRATCH_TABLE_PAGES`, 32.
- **The kernel's page table is the free list.** `supervision_protocol::map_scratch` tries
  `PageFrame::MAP` at the cursor. The kernel refuses a page that is already mapped, and refuses it
  whole, so that refusal (`BadPointer`) means "taken" and the next page is tried. Any other refusal
  is returned. A whole lap with nothing free is `Err`, which means the builder holds 16,384 pages of
  live children at once. Pages that stay mapped for good (the boot servers') cost one refused probe
  per lap each.
- **The image request moved onto it.** The progenitor's `receive_image` took a contiguous run with
  `scratch_pages`, which is gone; it maps each of the caller's frames with `map_scratch`. The shell
  destroys its staging region once the answer is in, so those pages come back too.
- **The progenitor's numbers are checked against the window at compile time.** `INIT_OWN_PAGES` must
  be at least twice `SCRATCH_TABLE_PAGES`, and the job pool plus an image request's pages must fit in
  half the window, leaving the other half for the boot servers' permanent pages. Milestone 595 will
  grow the pool for `rg`, and a pool grown past that fails the build rather than a spawn.

No syscall surface, no new method, no capability slot: the progenitor's table stays at 23 of 24.

**Would we still choose this if the alternatives cost the same?** Yes. The alternatives were the two
the proposal named, an unmap after each build and a fixed per-build window. Both need §162's unmap,
which is a permanent addition to the syscall surface and calef's call. The window needs nothing new
and uses a guarantee the kernel already makes and already tests. What it does not do is remove the
builder's mappings of a *live* child, which is §162's question and stays open.

## The test

`kernel::user::scratch_window_tests::a_builder_reuses_scratch_its_reaped_children_gave_back`, with
`fixtures/src/scratch_window_exerciser.rs` (name provisional). The fixture forges a 668-page ELF and
builds a child from it 110 times, each out of a region it destroys before the next: 73,480 scratch
pages, more than the old cursor's 65,536 and four and a half laps of the window. Its own page tables
come from a 48-page region, which covers the window's tables and nothing like the 144 a climbing
cursor needs.

**The negative control, measured** on aarch64 on 2026-09-26, with the wrap taken out of
`map_scratch`: 36 builds, 24,048 scratch pages, then the 37th refused. With the wrap: all 110, in 35
seconds on a host 5.6 times oversubscribed.

Cross-ISA: one portable test body on all three architectures. The loader is portable userspace code
and the revocation is portable kernel code (DECISIONS §19).

## BUGS

- **The count above is computed, not measured on a progenitor.** The boot servers' share of the
  window was never measured; the whole archive is under 2,400 pages of file, so the compile-time
  check leaves half the window for them.
- **The probe is one thread's.** `map_scratch` reads, probes and writes the cursor without a lock.
  Every builder in the tree is one thread; two threads building at once would waste a probe, not
  share a page, because the kernel refuses the second mapping.
- **A builder still holds a mapping of every live child's pages**, readable and writable. Bounding
  the cursor does not change what DECISIONS §162 is about, and `notes/trusted-init.md`'s honest
  limits still apply.
- **`BadPointer` means "taken" only because the address is always valid here.** The kernel returns
  the same error for a misaligned or kernel-half address. A distinct `AlreadyMapped` error would make
  the probe say what it means; it is an ABI change and not worth one on its own.

## Handoff

- Unblocks milestone 595's `rg` at the prompt from failing after a fixed number of runs; its pool
  growth is now checked against the window at compile time.
- No new work found that wants a lane of its own.
