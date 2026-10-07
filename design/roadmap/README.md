# The roadmap: one file per milestone

`script/roadmap` is the index, computed from these files, so nothing here lists them. A block's
filename carries its number zero-padded to four digits, `0007-user-mode.md`, so a listing sorts in
milestone order. Prose and headings keep the plain number. calef ruled this on 2026-10-07 (UTC),
choosing four digits over three so the rename is never redone at a thousand blocks, and
`script/lint` fails any other width.
A block opens with frontmatter holding its status, dates and dependencies, then its number and
title, and its `## Follow-on` bullets each carry a disposition. [`notes/roadmap.md`](../../notes/roadmap.md) has the
keys, `script/roadmap --check` says which forms it accepts, and
[`notes/follow-on-work.md`](../../notes/follow-on-work.md) says why. Work with no number yet goes in
[`proposals/`](proposals/README.md).

For the shape rather than the list: milestone 7 (user mode: EL0, capabilities, the ELF loader, and
IPC) is the dividing line between a Rust program that boots and an operating system.

*Name: recorded (milestone 294). Milestone 294 (`design/roadmap/README.md`'s index is generated,
not hand-maintained) retired the hand-kept index this file used to be. The stems are milestone
slugs and are not tracked by `script/names`: calef ruled milestone titles drafts on 2026-08-04.*
