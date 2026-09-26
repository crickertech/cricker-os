---
status: PARTIAL
raised: 2026-09-26
promoted_from: a-frame-per-filesystem-client-channel
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 599. A frame per filesystem client channel

Minted 2026-09-26 by lane `milestone/599-fs-client-page`, promoted from the
proposal `a-frame-per-filesystem-client-channel`, which milestone 47 (navigation and naming)'s lane
wrote on `milestone/47-navigation` (#1343). The work was first named as case A of
`notes/shared-page-audit.md` by milestone 43 (a second security audit) and never got a number.
calef ruled option A (badged endpoints) on 2026-09-26; the same lane built the kernel mechanism, the
file server's K windows, the kernel-harness pool, and the witness that proves isolation on all three
architectures. What is left is the production progenitor's pool, which has a design question of its
own and its own gate; see "What is left". *(Number and title provisional: the integrator mints the
number at merge, and the title is a draft until an architect names it.)*

The proposal said the work had no gate, "a wiring change inside the tree". Reading the server said
otherwise: it receives on one endpoint, learns nothing about its caller, and reads every request from
one window. A per-client frame it can read needed badged endpoints, a kernel remap at the
rendezvous, or a token convention among the page's writers, and calef chose badged endpoints. The
second fork, how the progenitor holds the windows, is open; see "What is left". Neither has a section
in `design/decisions/` yet; the integrator mints them from the note.

## The finding, checked 2026-09-26

The file service shares one read-write staging channel (`fs::TRANSFER_PAGES`, 16 pages, 64 KiB)
with every client a boot wires. In the kernel harness it is `FILE_SHARED`, memoised by
`fs_service::ensure`. On the interactive boot it is the progenitor's `BootEndowment::fs_page`,
which the shell maps at `SH_FS_VA` and every `build_caretaker` chain maps at `FS_CLIENT_PAGE_VA`,
caretaker and confined program both (`crates/system_initializer/src/lib.rs`). What keeps clients
apart is which of them is blocked, not what they can map.

It is a correctness bug as well as a confinement one. A client stages its bytes and then calls, so
two honest clients active at once overwrite each other; `notes/pipes/the-file-end.md` is why the
shell writes a `>` file itself rather than letting a second process do it. Today every path keeps
one FS client active at a time. The set grant at the prompt would not.

## What is built (2026-09-26, option A)

calef ruled option A: a server tells its clients apart by a badge the kernel delivers.

- **The kernel mechanism.** A capability's badge rides on `Object::Rendezvous(id, badge)` (no size
  growth; `Cap` stays 32 bytes). `abi::rendezvous::BADGE` (method 7, provisional) mints a badged
  copy of an endpoint, GRANT-gated, set once. `RECV_CAP` returns the sender's badge in x3, carried
  in the delivered mailbox's word 3, which was already stored as a zero on the CALL and SEND_CAP
  fastpaths, so the only added instruction is `set_arg(3)` on the RECV_CAP return.
  `crates/user_mode_runtime` gains `badge` and `recv_cap_badged`.
- **The file server's K windows.** `filesystem_protocol::fs::CLIENT_WINDOWS` (8, provisional) staging
  windows; `redoxfs_server` reads window `badge` per request. Window 0 is the unbadged default, so
  single-client paths are unchanged.
- **The kernel-harness pool.** `fs_service` allocates the K frames once, maps them into the server,
  and hands them out with `claim_window`/`release_window` (release zeroes the frame: the take-back
  half). The witness gives each client its own window and a badged endpoint, so the two map
  different frames; its assertion flipped from substitution to isolation. It exercises both shapes:
  the victim mints its own badge, and the attacker is handed a pre-badged endpoint.

## What is left

- **The production progenitor's pool.** `crates/system_initializer` still hands every client
  `BootEndowment::fs_page`, window 0, so the shell and its caretakers share one window on a real
  boot. The fix is proven in the harness but not yet wired into the progenitor, and this is the piece
  the set grant at the prompt actually needs. It carries a design question of its own. The progenitor
  cannot hold a frame capability per window without blowing its 24-slot capability table
  (DECISIONS §102 (a Frame names a run of pages), which calef has ruled on before). So how it addresses K windows wants deciding
  first. Six options are priced in `notes/a-frame-per-filesystem-client-channel.md`, under "The
  progenitor's pool: the fork". The short answer: one run capability covers the pool, so the slot
  problem goes away, but handing a client only its window still needs one new operation (a
  `PageFrame` slice or a `MAP_INTO` offset), and that is calef's call. It is gated
  by `script/swish-check` (which boots the real progenitor), not `script/test`, so it is a separate,
  carefully validated piece rather than more of this one.

## What it costs

From constants, not a boot: 64 KiB of contiguous memory per concurrent client channel, where the
whole boot shares one 64 KiB channel today. Carved from a directory job's region
(`DIR_JOB_REGION_PAGES`, 96 pages), that adds about 17% to each job. Per-request cost depends on the
option: nothing added under A, a window remap under B, two rendezvous and a copy under C.

## What it unblocks

Milestone 47 (navigation and naming)'s set grant at the prompt, which names this as its
prerequisite. A ruling on option A would also decide option 1 of the proposal
`every-client-of-a-network-stack-shares-its-socket-numbers`, which is the same question for sockets.

## Follow-on

- **Done.** The kernel badge mechanism (`BADGE`, the badge on `RECV_CAP`), the file server's K
  windows, the kernel-harness window pool, and the witness that proves isolation on all three
  architectures. See "What is built".
- **Recorded.** The residue is in `redoxfs_server`'s module `BUGS`: the production progenitor still
  hands its clients window 0, so on a real boot they share one window until the progenitor pool
  lands.
- **Decision.** calef ruled option A (badged endpoint capabilities) on 2026-09-26; a maintainer is
  recording it under `design/decisions/`. The syscall-surface fork is answered.
- **Outstanding.** The production progenitor's per-client pool (see "What is left"), with its own
  design question (how the progenitor addresses K windows within its 24-slot table, §102) and its
  own gate (`script/swish-check`). Checked 2026-09-26: the harness proves the mechanism; this is the
  boot-path application of it, which the set grant at the prompt needs.

## Index row

The file service shares one read-write staging channel with every client, so clients are kept
apart by scheduling rather than by mapping. A per-client frame needs the server to know its caller,
which is a kernel or protocol fork for calef.
