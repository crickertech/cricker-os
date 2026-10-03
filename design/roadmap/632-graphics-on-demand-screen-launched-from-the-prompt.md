---
status: BUILT
raised: 2026-09-30
built: 2026-10-03
milestone_dependencies: 177, 192, 600
decision_dependencies: 26, 227
machine_requirements: none
specific_machine: none
needs_person: no
---
# 632. Graphics on demand: `screen`, launched from the swish prompt

*(Number and title provisional; the integrator confirms the number at merge.)* calef, 2026-09-30:
*"I don't want graphics at boot and won't for a long time. Change to launching a program from the
swish prompt to get graphics; graphics is going to sit there largely unused for some time."*

The ruling reverses the boot half of milestone 177 (wire the graphical terminal stack into the real
interactive boot). Every `--features shell` boot is now the minimal UART system of DECISIONS §26
(the fault endpoint: thread death becomes a message a supervisor holds). A gpu on the bus changes
only who holds the grants. The answer is the shell, until a person at the prompt delegates them
into a session. Milestone 600's mechanism survives whole; only its moment moved, from the boot to
the launch.

## What changed

- The protocol carries devices (`crates/grant_plan/src/spawnproto.rs`). `GRAPHICS_BIT` and
  `KEYBOARD_BIT` announce the gpu's four caps and the keyboard's three. They are delegated after
  the `--mem` untyped and before the machine statistics page. `SHELL_GPU_SLOT` (22) names the
  seven-slot block 22-28 in the shell. Const assertions fence it against `SHELL_CONFIG_SLOT` below
  and `RUN_UNVOUCHED_SLOT` above. The shell probes fixed numbers, and a probe is sound only for a
  slot nothing else allocates into.
- `screen` is a program (`components/src/screen.rs`, `Prog::Screen`, id 20). Its manifest declares
  nothing a line designates, so every operator on it is a refusal and a session is always a plain
  line. It has two arms, chosen by `x0`. With a keyboard, the session's own discipline answers
  cooked `OP_READLINE` and does the terminal's whole job. Without one, the program reads the boot's
  own discipline raw over the UART (`OP_RAWMODE`/`OP_READRAW`) and paints its own echo. That is the
  shell's shape under §227 (how Tab reaches the shell: the shell edits its own line), and it is
  milestone 192 (a keyboard on real silicon)'s option A at launch rather than at boot. `quit` and
  `^C` end either arm.
- The boot hands the devices to the shell and builds nothing from them
  (`crates/system_initializer`). `screen_grants` replaces `graphical_verdict`. The console pair and
  the UART `input` driver are built unconditionally again, the pre-177 shape, and the UART is never
  freed early. The seven grants land in the shell's slots 22-28 beside the machine page, after the
  login block's peak. The progenitor keeps no copy.
- The shell refuses what it cannot launch, loudly. `screen` at a prompt that holds no display (no
  gpu, a `login` session, every witness wiring) prints a sentence. It is never a spawn that quietly
  drops the authority the user asked for. On a launch the shell delegates the seven caps, narrowed
  to the rights the boot endowment carried. It keeps its own copies, so a session can run again.
- The launch belongs to the spawn service (`build_screen_session`). Milestone 600 (the graphical
  terminal stack is built in userspace)'s builder moved from the boot into the `Prog::Screen`
  branch. It refuses anything but the exact shape: the program and the caps together, and the
  keyboard bit exactly when the boot's verdict says. Everything is carved from one 464-page job
  region. The session program is born supervised on `deaths` like any job, so its one reap sweeps
  the drivers with it. The drivers are unsupervised for `build_caretaker`'s recorded reason: a death
  message for a tid the sweep already collected would trap `job_undertaker`. Failure is `Err` all
  the way down, not a trap. A launch that cannot come up is a command that failed.

## What proves it

`script/swish-check --graphical` and `--graphical-serial`, retargeted, on aarch64 and riscv64. Each
boots the normal UART system and asserts the swish prompt on the serial console first. That
assertion is the claim that the boot stayed minimal. Each then types `screen` over the UART. It
requires the session's `$ ` prompt on the screen and one keystroke echoed back. The keystroke goes
by `sendkey` through the session's keyboard in one arm and by the raw UART round trip in the other.
The old graphical boot legs are retired: the boot they booted no longer exists, and the retarget
subsumes their claim. CI's `swish-check-graphical` job runs both legs. The plain legs on all three ISAs attach no gpu
and type `screen` too, requiring the refusal sentence.

## BUGS

- `SCREEN_SESSION_PAGES` (464) is counted from the constants, not measured. The counting is spelled
  out at the constant. Too small is a launch that fails every time; too large spends headroom.
  While a session runs, its region is 464 of the pool's 672 pages, so a `std` job and a second
  session are refused until it ends.
- A session whose build fails after its first driver started may keep those pages. The failure
  path reclaims the region and the sweep wakes the half-built drivers. No leg has yet forced
  whether they exit on a swept endpoint.
- A session cannot be interrupted under §24 (interrupting the foreground process). It ends on its
  own, so a second `^C` cannot escalate and a hung session holds its region until reboot.
- The boot now carries the seven device grants from kernel spawn until the shell's build. The
  previous shape deleted them mid-boot. With a gpu, a keyboard, a virtio-rng and a NIC attached,
  that resting baseline is counted from the code, not measured. No gate boots all four devices.
- x86_64 has no `screen` session. No virtio-gpu is wired there, the verdict answers empty, and
  `screen` is refused with the sentence, which the plain swish-check legs assert on all three ISAs.
  The plan for the launch itself is milestone 270 (wire `virtio-gpu-pci` and `virtio-input` into the
  x86_64 test runner): the drivers are PCIe and need no new capability, so a graphical leg follows. The firmware-screen terminal (milestone 400 (the shell on
  the firmware screen)) is untouched and still mirrors the boot console; it is the x86_64 display
  path, and milestone 624 (the x86_64 paint path stops repainting the world) is its paint path.
  Whether that console should also become a launch is calef's call, not decided here.

## Follow-on

- **Proposed.**
  `design/roadmap/proposals/a-screen-session-can-be-interrupted-and-torn-down-like-any-job.md`.
  The §24 job-frame shape for `screen`, so a second `^C` escalates and a hung session can be torn
  down. The manifest already records the cost: `interruptible: false` is a first cut, not a design.
- **Proposed.** `design/roadmap/proposals/a-local-editor-for-the-screen-sessions-uart-arm.md`. The §227
  engine behind the raw arm's reads, so backspace works before `quit` is typed.
- **Recorded.** The rest of `SWISH_CHECK_SCRIPT` against a graphical prompt is still scoped-out
  follow-on work, carried unchanged from milestone 177's own scoping.

## Index row

calef's 2026-09-30 ruling reverses milestone 177's boot half. Every boot is the minimal UART system,
and the shell holds the display devices' seven grants at slots 22-28. A new `screen` program,
launched from the prompt, takes them by delegation. The progenitor builds the session's driver,
terminal and (with a keyboard) discipline from one supervised region. Launch proven on aarch64 and riscv64
by the retargeted `swish-check --graphical` legs; the refusal, on all three. Without a keyboard, a session reads the boot
discipline raw over the UART: milestone 192's option A at launch.
