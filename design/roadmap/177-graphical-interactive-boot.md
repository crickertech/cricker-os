---
status: BUILT
raised: 2026-08-26
built: 2026-09-19
---
# 177. Wire the graphical terminal stack into the real interactive boot

Built 2026-09-19. Minted 2026-08-26, from tracing the user story "boot to a login
prompt, land in a `swish` prompt on a real terminal" against the actual code rather than the
roadmap's own framing, and finding no milestone owns the gap this surfaced. **Pieces 1-4 built and
merged 2026-08-27** (`milestone/177-boot-wiring-build`): the kernel-side graphical stack, the direct
`kbd` -> `line_editor` grant (option A, decided), `line_editor`'s `display_terminal` output adapter,
and device attachment. **The prompt reached the screen on 2026-09-19** (`milestone/177-second-flush`),
once the hang that had stopped it for three weeks was root-caused: not the GPU driver's interrupt
handling, as recorded at the time, but the boot never receiving two drivers' one-time status
reports, so each sat in a blocking `SEND` (see "The second flush, root-caused" below).
`script/swish-check --graphical` and `--graphical-serial` are green on aarch64 and riscv64 and run
in CI. **Piece 5 (x86_64's entry point) split off as its own milestone**,
[182](182-x86-64-interactive-boot.md), once the lane found it needs a from-scratch ELF-loading boot
path, not wiring.

The input-routing fork was `Gate: DECISION` (2026-08-27, an investigation lane found piece 1's own plan
does not fit and piece 2 cannot be built until the input-routing fork below is answered); decided
the same day, calef: **Option A**, a direct `kbd` -> `line_editor` grant, fixed at spawn, no
compositor in this path. See "The investigation, 2026-08-27" below for the full reasoning and the
two options not taken.

## What actually blocks this

The real interactive boot (`crates/system_initializer::boot`, reached by
`kernel::user::spawn_init` on aarch64 and `riscv_shell_boot` on riscv64) spawns exactly `console,
input, line_editor, swish, job_undertaker`. `console` and `input` are the plain UART pair (DECISIONS
§26): `input.rs`'s real-boot path reads only the UART register layout, with no code path to a
virtio device at all outside `cargo xtask test`. Checked directly, not assumed: neither the GPU nor
the keyboard is in `system_initializer::BootEndowment`'s device grants, and `NIFE_GPU`/`NIFE_KBD`
join `NIFE_RNG`/`NIFE_NVME` on the list of flags this boot's QEMU invocation never sets, which
`cargo xtask test` sets unconditionally for every leg. This is the same shape DECISIONS §120 already
found and named for the RNG case on milestone 49's boot-wiring fork: a deliberate, existing
minimal-device-surface choice for the interactive/demo boot, not an oversight.

**x86_64 has neither of the two entry functions named above, at all** (found 2026-08-27, tracing
`printenv`'s own "both boards" verification against `script/swish-check`, which names only aarch64
and riscv64: *"x86_64 has no shell leg."* That comment's own justification, "there is no userspace
on that target at all yet," is stale, since x86_64 has had real userspace running for most of this
session; the fact itself is not stale, for a narrower and still-current reason). Nothing boots
straight to a shell prompt on x86_64 today; `kernel/src/user.rs` has no third function beside
`spawn_init`/`riscv_shell_boot`. This is milestone 49's gap as much as this milestone's: neither
currently plans it.

This milestone was originally scoped as three joined pieces; **the third split off as its own
milestone, [182](182-x86-64-interactive-boot.md), 2026-08-27**, once the build lane found it needs
a from-scratch ELF-loading boot path rather than wiring. What remains here, built (see "What was
built" below):

1. **Attach the devices.** GPU and keyboard grants added to `BootEndowment`, and the interactive
   boot's QEMU invocation (`helpers/qemu-runner-*.sh`'s non-test path, or a new demo-boot flag)
   attaching the virtio-gpu and virtio-keyboard devices the test harness already exercises.
2. **Swap the programs.** Replace `console`/`input` in `system_initializer::boot`'s spawn list with
   `display_terminal`/`compositor`/the virtio keyboard client, the same components milestone 23's
   own text already names as proven but "not running under the test harness."

## The investigation, 2026-08-27: why "just wire it up" does not work

A lane investigated all three pieces before writing code, and reverted the one piece it did write
(the grant code below) once the reason became clear. Recorded here rather than only in the pull
request that carried it (merged with no code changes; the PR is not where a future reader meets
this).

**Finding 1: `BootEndowment` has no room for GPU/keyboard grants as piece 1 describes them
(mechanical, not a fork).** Counted the actual slots rather than assumed them: aarch64's
`spawn_init` (`fixtures/src/hello.rs`'s `init_boot`) fills capability-table slots 0-11 of 16 already
(`untyped`, the report endpoint, `uart_dev`, the test IRQ, `uart_irq`, `clock_page`, `config_page`,
`fs_ep`, `fs_page`, the virtio-rng trio); 3 free. riscv64's `riscv_shell_boot` fills 0-9; 5 free. A
virtio-gpu device needs 11 (`display_service.rs`'s own shape: transport, irq, and a nine-page DMA
region, one `PageFrame` capability per page, because the ABI's `MAP_INTO`/`CAP_INSERT` are strictly
one-capability-per-physical-page). A virtio-keyboard device needs 3-4. Neither fits what remains on
either board, let alone both together. The lane found this by writing the grant code first and
watching it try to `grant_at` a slot past 16, then reverted it (`kernel/src/user.rs` is unaffected
by this milestone as of this writing).

The fix is mechanical: build the graphical stack **kernel-side**, called from `spawn_init`/
`riscv_shell_boot` before init exists, the same shape `fs_service::root_directory` already uses for
the filesystem pair (the block server and FS server are built kernel-side; init receives only two
capabilities, `fs_ep`/`fs_page`, regardless of how many processes or pages sit behind them).
Kernel-side spawning maps physical pages directly and is not bound by any one process's
sixteen-slot table, so it sidesteps the wall entirely. The kernel-side wiring functions already
exist and are proven under test (`kernel/src/user/display_service.rs`, `compositor_service.rs`,
`keyboard_service.rs`); this piece is calling them (or code shaped identically) from the real boot
path instead of only from tests.

**Finding 2, the real fork: there is no path for a keystroke to reach the shell in the graphical
stack, and closing it means choosing how.** `swish` calls `OP_READLINE` on its terminal capability,
and only `line_editor` answers that opcode; `display_terminal`'s own module doc is explicit that it
is not a line discipline and expects `line_editor` in front of it. So `line_editor` has to stay in
the graphical stack. Two problems:

1. Output. `console.rs` speaks a bespoke two-endpoint request/reply protocol plus a shared
   page; `display_terminal` speaks `line_editor::proto`'s `OP_WRITE`/`OP_BYTES` over one `CALL`
   endpoint. `line_editor` needs a code change (or a second output path) to print through
   `display_terminal` instead of `console`.
2. Input, the harder one. `kbd.rs` (the only proven virtio-keyboard driver) speaks only the
   compositor's ring-and-doorbell protocol. This is not an accident of what has been built so far;
   it is DECISIONS §33's own security property, stated in `kbd.rs`'s own module doc: the driver
   "cannot name a client at all," and holds no client's endpoint, precisely so a compromised or
   buggy keyboard driver cannot forge a keystroke to a client it was never meant to reach. Focus is
   entirely the compositor's decision. `display_terminal`'s own `OP_BYTES` handler, when it is the
   compositor's focused client, only feeds its local renderer (`term().feed(&b)`) and forwards
   nothing onward. So there is no route from a real keystroke to `line_editor` in the
   compositor-mediated model as it exists today, by design rather than by gap.

Whether the compositor belongs in this path at all, checked directly rather than assumed
(calef, 2026-08-27: *"I'm concerned we're leveraging the compositor because we started a GUI and
then paused."*). The concern is correct and sharpens the fork below. `kbd`'s inability to name a
client is not a general security property of keyboard drivers; it is the specific answer to a
multi-client problem, "which of several competing windows should this keystroke reach," stated
in DECISIONS §33's own reasoning. A single-terminal boot has exactly one possible destination for
every keystroke, always: there is no second window to misdirect to, so the problem the compositor's
focus arbitration solves does not exist in this journey's actual scope. `display_terminal`'s
`MODE_DISPLAY` already reaches the equivalent conclusion on the output side (no compositor, direct
GPU ownership, because a single terminal does not need multiplexing); the options below differ on
whether the input side follows that same logic or not.

Three shapes, priced. Decided 2026-08-27, calef: A.

- A. Give `kbd` a second delivery mode: `CALL` `line_editor`'s endpoint directly, fixed at
  spawn, for the boot's own single-terminal case, the same shape `display_terminal`'s own
  `MODE_DISPLAY`/`MODE_WINDOW` split already uses. Not a security exception to DECISIONS §33 (the compositor's authority is memory, not messages),
  on the reasoning above: it is this codebase's own standing pattern (`AGENTS.md` rule 2, "a driver
  takes what it needs, passed in") applied consistently, and it is *narrower* authority than the
  compositor-mediated model, not looser, because `kbd` would hold exactly one fixed capability
  instead of "whichever client the compositor currently focuses." Cheapest change, and the one that
  matches what this journey's scope actually needs.
- B. Make `line_editor` itself a compositor-window-shaped client (hold a control page and
  doorbell, receive `OP_BYTES` from the compositor as the focused client, forward rendered output to
  `display_terminal` via `OP_WRITE`). Buys the compositor's real value, multi-window/multi-login
  arbitration, before this journey needs it (milestone 49's login wiring is still single-session
  today). Real new work for a problem not yet in scope: a second blocking endpoint on a process, in
  a system DECISIONS §33 already found has exactly one blocking wait point per process.
- C. Have `display_terminal` relay (already the compositor's focused client, already receives
  `OP_BYTES`; forward to `line_editor` instead of only feeding its own renderer). Raised in the same
  conversation as an alternative to A/B before the reframing above; superseded by it, since C keeps
  the compositor load-bearing for input in exactly the case that does not need multiplexing at all,
  the pattern the reframing names as the thing to be wary of. Kept here rather than deleted so the
  reasoning that ruled it out stays visible.

Decided: A (calef, 2026-08-27, "Go with option A, build it"). Not merely cheapest; per the
reframing above it is the design this journey's actual scope calls for, and B's real benefit
(multi-window arbitration) is not yet needed by anything in this tree. B remains buildable later,
additively, whenever a second concurrent session actually needs the keyboard; A does not foreclose
it.

Finding 3 corrects this doc's own sequencing claim. The original text (below, in "what this
unblocks") said piece 3 (x86_64's entry point) is provable against the plain `console`/`input` pair
independent of pieces 1-2. Checked against
[DECISIONS §121](../decisions/121-port-io-capability.md) (ratified permanently 2026-08-25) and
found false: x86_64's UART console is permanently kernel-resident, a closed question rather
than an unbuilt feature ("this is not an interim stance to be revisited on a schedule"). x86_64 has
no working userspace console at all, on either side of this milestone; its only possible route to
an interactive shell is through the graphical stack, which means piece 3 depends on finding 2's
fork being answered the same way pieces 1-2 do, not independent of it.

## What was built, 2026-08-27

Pieces 1-2, both on aarch64 and riscv64: `kernel::user::boot_graphical_terminal` (kernel-side,
mirroring `fs_service::root_directory`'s pattern, per finding 1), the direct `kbd` -> `line_editor`
grant (`MODE_DIRECT`, option A, decided), `line_editor`'s `MODE_DISPLAY` output adapter, and
`crates/system_initializer::boot()` branching on whether the graphical grants are present. A real,
reproducible capability-table-exhaustion bug was found and fixed along the way (the new grants
inflated `boot()`'s resting baseline enough to push the *entropy* build past the sixteen-slot wall;
fixed by freeing `uart_dev`/`uart_irq` at the top of `boot()` on a graphical boot, since they are
dead weight there). `script/swish-check --graphical` (a new leg, verifying via decoded screendump
since there is no UART to pipe a transcript from) does not yet reach a working prompt: a second
`FLUSH` through the real boot's own driver instance hangs, diagnosed as likely a pre-existing
characteristic of `components/src/gpu_driver.rs`'s completion-IRQ handling rather than something this
milestone's wiring introduced, and recorded in `notes/framebuffer-contract.md`'s own BUGS section
rather than held on. (That diagnosis was wrong; see "The second flush, root-caused" below.) The existing plain-console boot is unaffected and re-verified working on both
architectures throughout.

## What this does not decide

Whether the swap is unconditional (the graphical stack becomes the only interactive boot) or a
runtime choice (both paths coexist, selected by a flag); which of milestone 55's storage or
milestone 49's login pieces this should sequence before or after; and whether `line_editor` itself
needs any change to run as a `display_terminal` client rather than a `console` client, or already
does (checked in milestone 29's own text as a client of the framebuffer contract, not re-verified
against the real boot path here).

## What this unblocks

The graphical half of the login-to-`kilo` user story ([DECISIONS
§131](../decisions/131-hold-at-rung-two.md)'s "kick-ass terminal, something I'll love working
with"), now that the display-driver hang above is resolved (2026-09-19). Independent of [milestone
169](169-kilo-editor.md) (`kilo`'s raw-keystroke primitive sits at the `DECISIONS §21`
line-discipline contract level, which both `console` and `display_terminal` already speak
identically) and of milestone 49's login-boot-wiring piece (unblocked 2026-08-26, DECISIONS §120
amended to grant the stopgap; the piece itself is a separate, ongoing build). x86_64's own route to
an interactive shell, split off as [milestone 182](182-x86-64-interactive-boot.md), still depends
on the input-routing fork this milestone already answered (option A), the same dependency the
investigation found before the split: x86_64 has no fallback UART path at all (DECISIONS §121,
permanently kernel-resident), so its only possible route is through the graphical stack this
milestone builds.
## The second flush, root-caused (2026-09-19)

What was recorded, and why it was wrong. The 2026-08-27 lane saw `display_terminal` blocked in
`CALL` to the driver with nothing receiving, and wrote down the best-supported guess: the driver
stuck on its own completion interrupt. Milestone 400's `framebuffer_driver` then served the same
contract to the same terminal with no interrupt at all, and that looked like confirmation.

What the boot was doing, read from a thread dump rather than inferred. `gpu_driver` sends
`graphics_protocol::status::FLUSHED` once, after replying to its first flush, and `SEND` blocks until
received. The terminal's first flush is its blank grid, painted *before* it sends `TERM_UP`, and
`kernel::user::boot_graphical_terminal` took `UP` and `TERM_UP` and nothing else. So the driver
parked in that `SEND`, and the terminal's second `FLUSH` queued behind it for ever. A kernel thread
dumping every thread twenty seconds into the boot showed the driver `Blocked` as a `Sender` on its
report endpoint and the terminal awaiting a `Reply` on the display endpoint, each endpoint with one
sender and no receiver; receiving on the report endpoint returned `FLUSHED`, and ten seconds later
the driver was back in `RECV` and the terminal's flush had been served.
`notes/framebuffer-contract.md`'s BUGS has the dump verbatim. `framebuffer_driver` works because it
sends no `FLUSHED`, not because it has no interrupt.

The keyboard had the same bug one step later. `keyboard_driver` sends `KEYBOARD_UP` before its
first `WAIT`, and the boot dropped the report endpoint `keyboard_service::start_direct` returned.
With the display fixed, the prompt appeared and `sendkey` still never echoed.

The fix, at each cause. The boot receives `FLUSHED` after `TERM_UP` and asserts it.
`keyboard_service::start_direct` receives `KEYBOARD_UP` itself and no longer returns the endpoint, so
no caller can drop it (rung one: there is nothing left to forget). The GPU side stays at rung three,
on purpose and marked as a foot gun at `display_service::start_terminal`, at `FLUSHED`'s own
definition and at the driver's `send`: the three test spawners read the digest `FLUSHED` carries, so
the wiring function cannot swallow it for them. No wire change, no new barrier, no driver change.

What the leg proves, and its cost. `script/ci-build`'s `swish-check-graphical` row runs
`--graphical` (a virtio keyboard, `sendkey`) and `--graphical-serial` (no keyboard, the byte typed
down the UART) on both architectures: a `$ ` prompt decoded off a `screendump`, then `$ a` after one
key. 47 seconds for all four boots against a warm target directory on the dev Mac, appended to CI's
`build + test` job beside `swish-check`, whose `--features shell` kernels it reuses. It proves one
key, not `SWISH_CHECK_SCRIPT`; that limit is recorded in the leg's own doc.

## The boot half, reversed (2026-09-30)

calef, 2026-09-30: *"I don't want graphics at boot and won't for a long time."* Every
`--features shell` boot is the minimal UART system again. The shell holds the device grants until a
person launches `graphical_terminal` from the prompt, and [milestone 632 (graphics on demand)](632-graphics-on-demand-screen-launched-from-the-prompt.md) builds the stack at that launch. The input-routing
decision (option A) survives unchanged inside the session, and so does the second-flush lesson:
`build_graphical_terminal_session` takes the drivers' one-time reports in the order they send them. The legs
keep their names, retargeted from "the boot brings up the stack" to "the launch does".

## Follow-on

- **Done.** (2026-09-19) The second flush through `components/src/gpu_driver.rs`'s real boot path
  returns; root-caused and fixed above.
- **Done.** (2026-09-19) `script/swish-check --graphical` reaches and types at a working prompt on
  aarch64 and riscv64, and runs in `script/ci-build` and CI as `swish-check-graphical`.
- **Done.** Whether the swap is unconditional or a runtime choice is settled by what was built:
  `crates/system_initializer` branches on whether the display-terminal endpoint was granted, and
  the kernel's graphical boot returns nothing when the bus has no GPU, so both paths coexist and
  device presence picks.
- **Done.** Whether `line_editor` needs a change to run as a display client rather than a console
  client is answered yes and built: `components/src/line_editor.rs` carries a second output arm chosen at
  spawn and nowhere else.
- **Done.** The sequencing question against milestone 55's storage and milestone 49's login is
  moot. 55 is REMOVED as of 2026-08-30 and 49 is BUILT and already wired into the real interactive
  boot on both ISAs.
- **Milestone 182.** Piece 5, x86_64's own entry point, is
  `design/roadmap/182-x86-64-interactive-boot.md`, which this block already names; the earlier
  sentence saying neither architecture plans it is superseded by the split recorded lower down.

## Index row

The real interactive boot brings up the graphical terminal stack whenever a GPU is attached:
`gpu_driver`, `display_terminal`, and a keystroke source (a virtio keyboard in `MODE_DIRECT`, or the
board's UART) wired kernel-side straight to `line_editor`, with no compositor in the path. A prompt
reaches the screen and a typed key echoes on aarch64 and riscv64, proven in CI by
`script/swish-check --graphical` and `--graphical-serial`. The three-week hang that blocked it was the
boot never receiving two drivers' one-time status reports, not the GPU's interrupt handling. x86_64's
own entry point split off as milestone 182 (x86_64's own interactive-boot entry point). *Reversed in part 2026-09-30 by milestone 632: the stack
is built at a launch from the prompt, not at boot.*
