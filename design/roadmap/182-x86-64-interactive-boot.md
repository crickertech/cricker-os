---
status: BUILT
raised: 2026-08-27
built: 2026-09-19
---
# 182. x86_64's own interactive-boot entry point

Built 2026-09-19. Split off from [milestone 177](177-graphical-interactive-boot.md),
2026-08-27, once that milestone's build lane found piece 3 (originally scoped as "build x86_64's
own interactive-boot entry point first") needs a from-scratch ELF-loading boot path, not wiring: a
substantially larger, separate undertaking than pieces 1-2's device attachment and program swap.
Built in part on 2026-09-14, inside [milestone 268](268-the-boot-ladder.md)'s lane, because 268's
top rung on x86_64 is not reachable any other way; the prompt came with
milestone 299 (the x86 port-range capability) on 2026-09-15; the third `script/swish-check` leg,
the last outstanding item, on 2026-09-19 (below, "The third leg").

DECISIONS §149 was resolved 2026-09-15 by reversing DECISIONS §121 (`AMENDED`):
x86's console is a userspace driver holding a port-range capability, not a kernel thread. The
narrative from the next paragraph down to "The third leg" predates that ruling and still weighs
§149's options; it is kept as the record, and its BUGS entries that 299 answered are marked so.

Amended 2026-09-09: this block's central premise no longer holds, and the milestone is smaller
than it reads. It says the graphical stack is x86_64's *only possible* route to an interactive
shell, because DECISIONS §121 makes the console permanently kernel-resident and there is therefore
no userspace console server to talk to. §121 does make the driver kernel-resident, and that part
stands. What was missed is that `swish` never talks to a UART on any architecture: it talks to a
console server over an endpoint. So the question was never "can x86 have a userspace console
driver" (it cannot) but "can something else answer on that endpoint", and a kernel thread can:
`inter_process_communication::Rendezvous` is generic over `T: Node`, privilege-free, and kernel threads already exist.

Two consequences. Milestone 177 (wire the graphical terminal stack into the real interactive boot) is no longer a prerequisite for this milestone, which
restores the agreed order of software parity before hardware parity. And the route is serial,
which is what a bench session needs, since `board_console` reads a wire and cannot read a monitor.

DECISIONS §149 carries that decision. This milestone waits on it, and on nothing else it did not
already wait on.

## What was missing (as of 2026-08-27)

x86_64 had no third function beside `spawn_init`/`riscv_shell_boot`. What existed instead,
`kernel::user::x86_userspace_demo`, is a fixture: it builds two children directly from a hand-built
region, each carved from a budget with every kernel object built in place, with no ELF parsed and
no archive loaded (its own doc: "the loader-shaped path minus the ELF"). It proves the scheduler and
the fault path can build and run real EL0-equivalent (ring 3) processes; it does not prove a real
`init` reached from a real archive, the thing `spawn_init`/`riscv_shell_boot` each already are for
their own architecture.

Milestone 177 corrected its own earlier claim that this piece was independent of the others.
x86_64 has no fallback UART path at all (DECISIONS §121, permanently kernel-resident), so this
milestone's only possible route to an interactive shell is through the graphical stack milestone
177 builds, not a plain-console alternative the way aarch64/riscv64 each have one. *(Superseded by
the 2026-09-09 amendment above; kept as the record of what was believed.)*

## What this needs

1. A real ELF-loading boot entry, matching `spawn_init`/`riscv_shell_boot`'s shape: parse the
   initrd archive, measure and verify `init` against the boot's own measurement table (DECISIONS
   §104's discipline, the same check the other two architectures' entries already make), build its
   address space, grant it the boot's own capability set, and dispatch it to ring 3. Built
   2026-09-14.
2. The x86_64-specific capability grants the other two boots each hand-assemble for their own
   architecture. Built for everything but the console, which is the question of §149 (may the kernel answer on an endpoint, where §121 leaves no userspace holder?).
3. A third `script/swish-check` `--arch` leg. Built 2026-09-19; see "The third leg".

## What was built (2026-09-14)

### The entry point, and the progenitor at ring 3

x86_64's default boot used to end in `nife x86_64: boot complete, halting.` after a green self-test
verdict. It now ends by handing the machine to the progenitor, through the same function
riscv64's boot uses, `kernel::user::riscv_shell_boot`, rather than a third copy of it. That
function parses the archive, measures `progenitor` against the trust root and the program table
against the kernel's vouching (`trust::require`, `trust::require_program_measurements`), builds the
address space with the archive mapped read-only, endows the progenitor, and starts it. Three
changes made one body serve both architectures:

- Every grant names its slot (`thread_control_block_insert_cap(.., Some(n))`), so an
  architecture that grants differently cannot renumber the rest.
- The x86 timebase page is mapped, the way `load` maps it for every process it builds; a
  hand-built address space has to do it itself (`map_x86_timebase_page`'s six earlier call sites
  each found that as a page fault).
- The interrupt-controller arming at the end stays RISC-V's. On x86_64 there is no input
  driver that could use COM1's line.

`main.rs`'s new `x86_hand_over` calls it and then watches, bounded, because on this architecture a
progenitor that cannot reach a console has nothing to print through, and a silent machine reads as
a hang. The transcript, QEMU `q35`, one core, the x86 archive attached:

```
nife self-test: 5 of 5 passed
  kernel task : a spawned thread ran and carried its captured state (0x16100004)
  ...
  initrd      : 4954112 bytes at 0xfb1e000, 87 programs, from the PVH module list

nife: handing the system to the userspace progenitor.
  uart irq: line 4 (machine description)

  user thread 6 killed: vector 6 (invalid opcode)
    rip 0x00000000004001c0   addr 0x0000000000000000   user rsp 0x0000000000500fa0   err 0x00000000
  the kernel is fine.

  user thread 4294967300 killed: vector 6 (invalid opcode)
    rip 0x0000000000400398   addr 0x0000000000000000   user rsp 0x0000000000500f40   err 0x00000000
  the kernel is fine.
nife x86_64: the progenitor is running at ring 3; 2 of the processes it built stopped on purpose.
  no prompt  : the console server cannot reach COM1 from ring 3 (§121); how a shell gets a console here is §149, not yet decided.
```

What those two faults are, identified rather than assumed. A temporary dump of the bytes at
each faulting `rip` and of the user stack, symbolized against the x86 ELFs: both are
`user_mode_runtime::trap`. Thread 6 is `input` (`rip` is `trap`, called from `input::uart::rx_pending`
via `input::drain`), and the other is `console` (`trap` at `0x400398` in that binary, its
`uart_put`). Both are the `x86_64` arms those programs have carried since milestone 161, which trap
on first use because a ring-3 process cannot reach port I/O (DECISIONS §121). They are the
progenitor's children, so the progenitor built the console server, the line discipline and the
input driver before they stopped, and it is still alive after the bound. Under OVMF with two cores
(`cargo xtask uefi-boot`) the same boot also prints the kernel's `capability slots: 13 of 24 at
peak`, which is the progenitor's own table gauge.

### A correction found by measurement: the empty slot

The first version left slot 1 empty on x86_64, because slot 1 is the UART page on RISC-V and
there is no page to grant here. The boot then failed in a way that had nothing to do with §149:
the progenitor trapped in `must(build_child(swish))` (`crates/system_initializer/src/lib.rs:1374`).
A temporary print of every syscall error named it: `MAP_INTO` of the shell's output page returned
`WrongObject`. The progenitor's first-free retype had put `term_out` into the empty slot 1, and once
the drivers existed it deleted slot 1 as `uart_dev`, deleting its own output page.

So slot 1 holds a placeholder on x86_64: a freshly zeroed frame, `READ | GRANT`, which nothing
else owns and which the console and input drivers' x86 arms trap before touching. It is marked in
the code as the foot gun it is. A device capability over physical page zero, which is what
`spawn_progenitor`'s aarch64-shaped path grants on x86, was refused for this: page zero is real
memory, and the console server would have written to it.

### The gates that keyed on the halt line

Three things waited for `nife x86_64: boot complete, halting.`: `cargo xtask uefi-boot`'s serial
check, its screen marker, and `script/netboot-rehearsal`. All three now wait for the self-test
verdict (`boot_ladder::SELF_TEST`), the last line every boot prints whatever it hands over to next,
and `uefi-boot` also requires the hand-over line, since that boot carries the archive. `uefi-boot`
passed under OVMF after the change.

### What §149 would take from here, per option

Priced against the tree as it stands, so the ruling can be made without reading the diff.

- Option 1, a kernel thread answering on an endpoint (recommended in §149). Smaller than §149
  reads, because the progenitor already has the shape. Its graphical branch (milestone 177,
  `has_graphical`) takes three kernel-created capabilities at slots 10 to 12 and builds no console
  and no input driver at all: `disp_term_ep`, which `line_editor` `CALL`s with `OP_WRITE` after
  filling `disp_term_page`, and `kbd_ep`, which a keystroke source `CALL`s with `OP_BYTES`. Under
  option 1 the kernel would run two kernel threads, one serving `OP_WRITE` by copying the page to
  COM1 through the direct map and one taking COM1's receive interrupt and `CALL`ing `kbd_ep`, and
  grant the three slots. The kernel already has the IPC for a kernel-side server
  (`sched::ipc_recv_cap`, `sched::ipc_reply`, `sched::ipc_call`). No change to the progenitor,
  `line_editor` or `swish`, and no syscall surface. What it costs is §149's own objection, a
  message parser in the kernel, and one naming wrinkle: the slots are called `disp_term_*` and would
  hold a serial terminal.
- Option 2, a console syscall. One or two new syscall numbers (the surface is four today), the
  `x86_64` arms of `components/src/console.rs` and `components/src/input.rs` rewritten to call
  them, and a blocking read, which needs a kernel wait queue for keystrokes. The progenitor would
  be unchanged. Every program on every architecture is then written against a surface that exists
  for one of them.
- Option 3, the graphical stack. `boot_graphical_terminal` is already portable and
  `pci::find_gpu_device` is PCI, which x86 has. The q35 runner would need a `NIFE_GPU` block
  (`virtio-gpu-pci` behind VT-d) and a keystroke source: the UART fallback in
  `boot_graphical_terminal` is `input`, which traps on x86, so a virtio keyboard is required too.
  xenon has neither device, and `board_console` cannot read a monitor, so the bench loses its
  serial path.

## The third leg (2026-09-19)

`script/swish-check --arch x86_64` boots x86_64 to its prompt and types the same
`SWISH_CHECK_SCRIPT` the other two legs type, over COM1, and checks every answer. `script/swish-check`
with no arguments now runs all three.

### Which boot it drives, and what that costs

The UEFI image under OVMF: `cargo xtask uefi-image`'s `target/esp/EFI/BOOT/BOOTX64.EFI`, the file
a customer copies to a USB stick (DECISIONS §157, milestone 198's rung 1), booted by
`helpers/qemu-uefi-x86_64.sh`. Not QEMU's PVH `-kernel` path, which `script/test`'s x86_64 suite and
`script/boot-check` use.

Measured on patagonia, 2026-09-19, one core, the same kernel and archive, a temporary switch in the
leg selecting the runner (not committed):

| | PVH (`qemu-runner-x86_64.sh`) | UEFI (`qemu-uefi-x86_64.sh`) |
|---|---|---|
| runner start to the shell's banner | 2.0 s, 1.9 s | 6.0 s |
| the 60 typed lines | 78 s, 79 s | 325 s |
| whole leg, warm target directory | 90 s | 341 s |

So the UEFI leg costs about four minutes more per run, and the cause is known rather than
guessed: under firmware the kernel hands the screen to a userspace terminal (milestone 400), and
the console server writes every byte to COM1 and then waits for the terminal to draw it before
acknowledging the write (`components/src/console.rs`, milestone 400's BUGS). Under TCG that draw
is a CPU copy through an emulated framebuffer, once per write.

Why UEFI anyway. It is the one a customer boots, and it carries four things PVH does not: the
loader, the firmware's memory map and ACPI tables (2 GiB of RAM, tables above 1 GiB where a real
machine puts them), OVMF's placement of the PCI functions' BARs, and the screen tee. A shell
regression on any of those passes a PVH leg. The four minutes are also a finding rather than
overhead: they are what milestone 400's "the serial console now waits for the screen" costs, now
measured at the prompt instead of described. Would UEFI still win if both cost the same? Yes; this
was not decided on effort, and the cost it does carry is stated so it can be weighed.

What it costs CI, and the per-line bound it needed. `script/ci-build`'s `swish-check` row runs
every leg, so CI's `build + test` job gains this one with no workflow change. The first CI run
(35463884897) went red on it: `caps ps`, 16.7 s on patagonia, did not finish inside the 30 s
per-line bound the other legs use. Nothing was wrong but speed, so the leg's bound was measured
rather than raised by feel (every leg now prints its line count, total and three slowest lines):

| leg | lines | total | slowest line |
|---|---|---|---|
| aarch64 | 64 | 6.9 s | 0.3 s |
| riscv64 | 64 | 7.3 s | 0.6 s |
| x86_64 (OVMF) | 60 | 321.1 s | 24.7 s (`xargs caps rm globmany/m-*.txt`), 16.7 s (`caps ps`) |

That CI runner was 1.5x to 1.8x slower than patagonia on this leg, so the x86_64 bound is 90 s
(`SWISH_CHECK_X86_LINE_SECS`): 3.6x the slowest local line, 2x that line at CI's worst ratio. The
cost is the screen path, milestone 400's console waiting for each write to be painted and copied
into an uncacheable aperture, as debug builds under TCG; the other two legs run the same shell
over TCG in under a second a line. A real PC pays that copy in native stores (milliseconds per
scroll, not measured on silicon). Recorded in 400's BUGS, where the blocking design is.

Expected CI runtime. In that run the x86_64 leg started 10.5 minutes into the job. At 1.75x
patagonia it takes about 10 minutes, and `boot-check` about one more, so the job should finish near
22 minutes against its 30-minute timeout: 8 minutes of margin, measured by one run, and the
first thing to watch. If it closes, the leg moves to a job of its own (`.github/workflows/ci.yml`
says so where the step is), or the flush cost comes down in milestone 400's code.

### What differs from the other two legs, and why

- The default kernel, tour first. x86_64 has no early hand-over under `--features shell`; every
  boot runs the tour and then hands over (milestone 268), and `uefi_image` builds exactly that.
- The checks read from the hand-over on. The tour's userspace demonstration kills two threads on
  purpose (`x86_userspace_demo`'s supervised deaths), so the killed-thread check and the
  "was the kernel writing during the boot" test start at `nife: handing the system to the userspace
  progenitor.` The first version read the whole transcript and failed on those two deaths, which is
  how this was found.
- It waits for the kernel's hand-over report, then presses Enter. `x86_hand_over` watches the
  progenitor for ten seconds and prints two lines after the prompt, so the transcript does not end
  in `$ ` and a line typed in that window could have the report spliced through its echo. The leg
  waits for the report's last line and presses Enter once for a fresh prompt. This is ordering the
  gate's own reads, not a readiness signal for input: typing before the prompt works (below).
- The RedoxFS disk is attached under firmware too, on request. `helpers/qemu-uefi-x86_64.sh`
  attaches `nifefs-redoxfs.img` as a second `virtio-blk-pci` function when `NIFE_UEFI_REDOXFS` is set
  (name provisional), because `>`, `<`, `ls` and `rm` need a filesystem. The leg sets it; nothing
  else does. It is opt-in rather than the PVH runner's attach-when-present because attaching it
  unconditionally turned `uefi-test` red (see BUGS).

### The script lines, and the four it omits

60 of 64 lines run. The omitted four are `uuid > id.txt`, `wc < id.txt`, `uuid 2> ent.txt` and
`wc < ent.txt`, each carrying its reason in `swish_check_omits(arch, line)`
(`xtask/src/swish_check.rs`). The reason: `uuid` draws from the entropy service, which the
progenitor builds only from a virtio-rng the kernel found, and the kernel finds one only on a
virtio-mmio slot (`kernel::user::boot_virtio_rng_device`); `q35` has no mmio bus. `caps uuid` still
runs, because it is a preview of the manifest and needs no device. A line added to the script runs
on x86_64 unless someone argues it out.

### Capability slots at peak

17 of 24, read by a temporary instrument (a kernel loop printing `capability::highest_seen()`
whenever it rose, not committed). That is five below aarch64 and riscv64's 22, consistent with the
three virtio-rng slots x86_64 does not grant and the entropy endowment it does not build. Not tighter.

The number the leg itself prints is 5 of 24, and it is wrong, which is the finding worth more
than the number. See BUGS.

### Verified

`script/swish-check --arch x86_64` green on seven runs on 2026-09-19: four at one core (the default),
two at `NIFE_SMP=2`, one with the final code. Typing before the prompt was also probed directly, at
two cores: a line written to COM1 the moment the serial log showed `handing the system`,
`every program measured` and `nife capability shell` was answered every time, within a second. So a
keystroke that arrives before the input driver is polling waits in the 16550 and is not lost, under
QEMU at least.

## What this does not decide

Whether x86_64's own boot path needs anything architecture-specific beyond the ELF-loading
mechanism itself (interrupt routing, device discovery specifics already covered by milestone
176's own work) is not assessed here; check that milestone's own text and `kernel/src/arch/x86_64/`
before assuming parity with aarch64/riscv64 on every point.

## What this unblocks

The top rung of milestone 268 (every architecture boots the same way: describe the machine, test yourself, hand over) on x86_64: a default x86_64 boot reaches `swish`, and now a gate types
at it. What 268 still owes there is its own item, `script/boot-check` asserting the prompt, which
this milestone does not build.

## BUGS

- Answered by milestone 505 (an x86_64 input driver that never lets the core idle), kept as the
  record: "the x86_64 capability-slot gauge reports the hand-over, not the peak" and "an x86_64
  prompt holds a host core at 100%". Both had one cause: x86_64's input driver polled COM1 and
  yielded, so the idle loop never ran again after the hand-over (5 of 24 printed against a real peak
  of 17; 76 CPU-seconds in 78 wall-seconds at an idle prompt). It now waits on IRQ 4.
- The x86_64 leg is about forty times slower per line than the other two, because the console
  waits for the screen (above), and so it carries a 90 s per-line bound where they carry 30 s: a
  real hang on that leg is reported a minute later. A wedged screen terminal would stall this leg's serial transcript as well, which is
  milestone 400's BUGS entry, not a new one.
- `x86_hand_over` still watches for ten seconds before its summary, and on this path the summary
  lands after the prompt. The leg orders around it; a person at a serial console sees two kernel lines
  appear under the `$ ` ten seconds after it.
- `uefi-test` fails with the RedoxFS disk attached, which is why that runner attaches it only on
  request. With it attached, `dir_capability_tests::a_full_directory_capability_does_everything_inside_and_nothing_outside`
  failed under OVMF ("the full directory capability could not do what it was granted"), one run,
  2026-09-19. The PVH leg a minute earlier passed the same test against the same image and wrote to
  it, so the likeliest reading is `mkredoxfs`'s own second-boot case (a mount of an image a previous
  boot wrote), not firmware. Not investigated. Recorded at the runner's opt-in.
- Answered by milestone 299, kept as the record: "there is no prompt on x86_64" (there is), "slot
  1 is a placeholder frame" (it holds COM1's `PortRange`), and "two of the progenitor's children die
  at every x86_64 boot" (zero do; the report says `0 of the processes it built stopped`).
- Answered by a ratification: `riscv_shell_boot` is `user::boot_progenitor` (ratified
  2026-09-15). `riscv_hand_over`/`x86_hand_over` are still provisional.

## Follow-on

- **Milestone 505.** milestone 505 (an x86_64 input driver that never lets), `design/roadmap/505-an-x86-64-input-driver-that-never-lets-the-core-idle.md`:
  interrupt-driven x86_64 input. Milestone 299 recorded the poll as a latency and CPU limitation; this
  milestone measured that it also starves the idle loop, which takes the slot gauge and the core's
  halt with it.
- **Done.** The stale gauge and the 100% core, by milestone 505 (an x86_64 input driver that never
  lets the core idle); BUGS above keeps the record.
- **Recorded.** `uefi-test` red with the RedoxFS disk attached, in BUGS above and at
  `helpers/qemu-uefi-x86_64.sh`'s `NIFE_UEFI_REDOXFS`.
- **Recorded.** The leg's CI cost, in `script/swish-check`'s header and `.github/workflows/ci.yml`;
  measured in CI by this pull request's first green run.
- **Milestone 268.** Its x86_64 top rung is reachable and gated by this leg; 268's own
  `boot-check`-asserts-the-prompt item is 268's.
- **Milestone 192.** Milestone 192 (a keyboard on real silicon) has no x86_64 graphical leg; 632 lists its needs.

## Index row

x86_64's interactive boot: the kernel hands over to the progenitor through the shared
`boot_progenitor`, the console is a userspace driver on a port-range capability (milestone 299), and
`script/swish-check`'s third leg boots the customer's UEFI image under OVMF and types the shared
script at the prompt, 60 of 64 lines, the four `uuid` lines omitted for want of an entropy device.
