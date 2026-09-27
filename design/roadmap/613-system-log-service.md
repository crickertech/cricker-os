---
status: NOT-STARTED
raised: 2026-09-27
milestone_dependencies: 126
decision_dependencies: 242
machine_requirements: none
specific_machine: none
needs_person: no
---
# 613. A system log service: the in-memory half

Minted 2026-09-27 by the maintainer against §242 (a system log),
`design/decisions/242-a-system-log.md`, which calef ruled the same day on pull request #1423 (now
merged). The title and slug are drafts; the number is not.

## Scope

One service. It holds a badged endpoint for writers and a separate endpoint for readers, and it
stamps every record's `seq`, `time`, `program`, `user` and `severity` from the writer's badge,
never from anything the writer sends. Writers speak F3, the byte sink framing with a body-kind byte
(§242, Question 4), with severity riding in `byte_sink_protocol`'s unused first-word bits. Readers
get JSONL, one JSON object per line, converted from F3 by the service. The service keeps a 64 KiB
in-memory ring, oldest out, with a dropped-count record on overflow. A per-user read filters that
one ring by the stamped `user` field; there is no per-user store. The service also drains the
kernel's own ring and forwards whole lines to the console, so it becomes the one thing writing
kernel output there.

Out of scope: persisting any of this to RedoxFS, which has its own proposal,
[`design/roadmap/proposals/a-system-log-on-redoxfs.md`](proposals/a-system-log-on-redoxfs.md); and
the kernel-side ring and drain syscall themselves, which are milestone 342 (the kernel and the
`console` server drive one UART from two address spaces).

## Dependencies

- §242 (a system log), DECIDED 2026-09-27.
- Milestone 126 (the `procps` package: who else is running), pull request #1360, which raises the
  progenitor's capability table to 32 slots. §242 counts the progenitor at 23 of 24 today; a
  minting endpoint for this service needs the spare slot #1360 adds.

## What this unblocks

- Milestone 342 (the kernel and the `console` server drive one UART from two address spaces), which
  names this service as the drainer its own kernel-side ring needs before it can build.
- A home for a scheduled job's output and refusals, milestone 152 (durable delegation), pull
  request #1377, whose Fork 6 option C grants a job's declared diagnostics stream to this log
  instead of discarding it.
- The notice board's history, §243, pull request #1424 (provisional).

## Done means

- Host tests, no emulator: stamping (the service, not the writer, fills `seq`, `time`, `program`,
  `user`, `severity`), F3 framing (including the severity bits amending `byte_sink_protocol`), and
  the F3-to-JSONL conversion.
- A QEMU test with two differently badged writers and one reader, asserting the reader sees both
  writers' lines correctly attributed to each, on aarch64, riscv64 and x86_64 per §19 (architectural
  parity is a tenet).

## Index row

§242 (a system log) ruled one service that the kernel and every program append to, stamping
attribution from the writer's badge because the writer's own claim cannot be trusted. This
milestone is that service's in-memory half: F3 binary framing in, JSONL out, a 64 KiB ring, and
per-user reads filtered by the stamped `user` field, with kernel lines forwarded to the console as
whole lines. RedoxFS persistence and the kernel's own ring are separate work; this is what unblocks
milestone 342's console arbitration, a home for a scheduled job's output, and the notice board's
history.
