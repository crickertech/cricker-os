# The progenitor, and loading a program from userspace

*(Milestone 19d. `kernel/src/user.rs` `spawn_hello`, and the `init`/`child` roles in
`fixtures/src/hello.rs`. The loader described here lived in that file until milestone 96 gave the tree
one of them; it is `crates/supervision_protocol`'s `build_child` now, and every caller reaches it there.
What the loader does is unchanged, and the steps below are still the steps.)*

*This note said, until milestone 266, that the archive entry named `init` meant a different program
on each architecture and pointed at [milestone 166](../design/roadmap/166-init-boot-parity.md) as the
open question. That is settled: the entry is `progenitor` on all three boards and it is one program,
`components/src/progenitor.rs`. Where the text below says init it is describing milestone 19d, when the
role and the demo catalog were the same binary.*

## The one thing 19d moves, and why it matters

Until 19d, when a program ran on nife the kernel read its file and set it up: it parsed
the ELF (the standard "here is a program: code here, data there, start at this address" format),
copied the pieces into memory, and started it. That parser lived inside the kernel.

Parsing a program file means processing bytes an attacker may have crafted, and a bug in a parser
is where exploits live. A bug in a parser *inside the kernel* is the worst kind: it compromises
the trusted core the whole §14 (project's) thesis rests on. So 19d moves the parser out, into an ordinary
confined program where a parser bug is that program's problem, confined by the same
capability walls as any workload.

That program is the progenitor: the first process, the one the kernel starts and from which
every other descends. It was called init until milestone 266.

## What still loads the progenitor (the honest residue)

Something has to load the *first* program, so the kernel keeps exactly enough loader for one: it
loads the progenitor through `boot_progenitor` and nothing else. The progenitor loads every
*other* program.
"The kernel loads exactly one program" is not a slogan we rounded up to; it is literally one call
site. (19d.2 removes the kernel's other loaders, the ones that wire up the console and shell
services today, by moving that wiring into the first process.)

## How the progenitor loads a child (the loader, in userspace, through the verbs)

It is handed three things by `spawn_hello`: a building untyped budget (slot 0), a report
endpoint (slot 1, with `GRANT` so it can endow a child), and the whole initrd mapped read-only
at `INITRD_VA` so it can read the ELF. Its length arrives in `x1`.

`build_child` then does, entirely through the milestone-19 granular verbs, what the kernel's
`map_segments` used to do privileged:

1. `RETYPE_OBJ(ASPACE)`: a fresh address space out of init's budget.
2. For each ELF segment, page by page: `RETYPE` a frame, `frame::MAP` it read/write into init's
   *own* scratch window to fill it (zero it, free `.bss`, then copy the segment's bytes), then
   `MAP_INTO` the child at the segment's own virtual address with the segment's permissions
   (executable code via the `MAP_CODE` mode 19d added). Then `cap_delete` the frame cap so the
   16-slot capability table recycles the slot: a loader retypes hundreds of frames, so slot recycling is
   why `SYS_CAP_DELETE` exists.
3. A stack frame, mapped read/write.
4. `RETYPE_OBJ(TCB)`, `CAP_INSERT` the report endpoint as the child's slot 0, `CONFIGURE` (entry
   from the ELF, the child's stack top, the aspace), `START`. `START` carries the child's first
   three registers: `x0` is the role (which of the multi-role binary this instance is), and `x1`,
   `x2` are data the child needs before it can run. See "The argument to START" below.

The child is a second instance of the same multi-role binary, entered at the `CHILD` role. It
SENDs one word through the capability init granted it, and exits. Receiving that word is the proof:
init parsed a real ELF and produced a running thread, and the kernel never looked at the child's
bytes.

## The argument to START (milestone 19e)

Through 19d, `START` handed the child exactly one word, its role in `x0`, and that was enough:
every child was pure code selected by identity. A *least_authority_demo* breaks that. It computes `n*n`, and `n`
has to reach it before it runs. So 19e widened `START` to carry `x0`, `x1`, `x2`, the way a
function call carries arguments, and the loader passes the role in `x0` and the input in `x1`.

The plumbing is one value carried through the whole thread-creation path. `START`'s three
arguments land in `Thread::start_args` (`kernel/src/thread.rs`); `arm_for_start` writes them into
the faked switch frame as `x21/x22/x23`; the EL0 trampoline (`context.s`
`user_entry_trampoline`) moves those into `x0/x1/x2` right before dropping to the child's entry.
The child sees them as the arguments to its `_start(x0, x1, x2)`.

init's spawn service (the shell's `run <n>`) is the payoff: the shell SENDs `n`, init builds a
least_authority_demo endowed with a result endpoint and `START`s it with `n` in `x1`, the least_authority_demo squares it and
SENDs the answer straight to the endpoint the shell is waiting on. init only builds the pipe; it
never sees the number. The kernel test `init_builds_the_demo_and_passes_it_an_argument` proves the
argument survives the crossing: the least_authority_demo reports `n*n`, not `n` and not garbage.

## Two hardware details a userspace loader must respect

- **The instruction cache is not coherent with the data cache** (aarch64). init writes a child's
  code as ordinary data; the CPU's instruction fetcher has never heard of those bytes. So when
  `MAP_INTO` maps a page executable, the *kernel* makes it coherent (clean to the point of
  unification, invalidate the I-cache) for that physical page. Without it, the child fetches
  whatever was in the frame before the program was written into it. This is the same
  `sync_icache` the kernel's own loader always did; 19d moved *when* it happens.
- **W^X across two address spaces.** init keeps a code frame writable in its own scratch window
  while the child maps it executable. A trusted loader mapping pages writable to fill them is the
  standard shape (seL4's does the same); the child's mapping is never writable, so the child
  cannot rewrite its own code.

## The initrd is an archive now (milestone 19f.1)

Through 19d/19e the initrd *was* one ELF: the kernel parsed the whole blob as the init program, and
init, to load a child, parsed that same blob again (children were roles of the one binary). 19f
turns the blob into a nifefs archive, the same named-file format the virtio disk uses, so one
parser serves both the RAM archive and the disk. `cargo xtask` packs it (`initrd_aarch64`, renamed
from `mkinitrd` 2026-08-27); it holds one
entry today, `progenitor`.

Two readers changed, each in its own domain:

- The kernel (`boot_progenitor`) reads the superblock, looks up the `"progenitor"` entry
  (`"init"` until milestone 266), and loads *that*
  as the ELF. This is the same honest residue as before ("something has to load the first program"),
  now naming that program through a fixed archive index instead of assuming it sits at offset 0. The
  kernel gains a nifefs read, which is proportionate to the ELF parse it already does for init and
  is bounded (a 512-byte superblock, count capped at 15). The milestone tour and the kernel-wired
  demos load a program the same way, through `user::program(user::HELLO_ENTRY)`.
- **init** (`hello.rs` `program()`) parses `INITRD_VA` as a nifefs archive and looks up a program
  by name, rather than treating the whole blob as one ELF.

Why the archive rides *beside* the kernel (handed in via the device tree) rather than baked into
init's own image: this is the QNX-IFS / old-Linux-initrd lineage, and it keeps the "delivery, not a
rebuild" property. You can swap userspace without rebuilding the kernel. seL4, our nearest relative,
instead embeds a cpio of the app ELFs into its root task; we chose the delivered-archive side so the
RAM boot and the eventual disk boot share one parser.

### The archive carries stripped ELFs (2026-07-30, milestone 23)

**The initrd is reserved RAM**: `memory::init` tells the frame allocator to keep its hands off it, so
every byte in the archive is a byte the running system does not have, for the whole boot. That was
free until it was not.

A debug build is almost entirely debug information. `rust_swappable` is 720 KB, of which 3 KB is
`.text` plus `.rodata` and the other 717 KB is `.debug_*`. Twenty-odd programs like that made a 26
MB archive out of well under a megabyte of code, on a 128 MB machine. Milestone 23 (capability-routed component) added five
programs, the archive went to 30.7 MB, and a *later, unrelated* test stopped being able to find a
contiguous eight-megabyte run for init's building budget: `no building budget for init`, in a test
that had nothing to do with the change, which is the usual signature of a resource the whole suite
shares. (That message reads `no building budget for the progenitor` today; it is quoted here as it
read on the day.)

So `initrd_aarch64` (`mkinitrd` before 2026-08-27) now strips each ELF (`llvm-objcopy --strip-debug`) before packing, and the archive is
**4.3 MB**. Nothing lost anything: `crates/elf` parses program headers only and has no
section-header code at all, so no loader on either side of the boundary could ever see a debug
section; the kernel prints a raw `pc` on a fault, and symbolizing it is done offline against the
unstripped binary still sitting in `target/`.

Two deliberate choices. `--strip-debug` rather than `--strip-all`, so the symbol table survives for
anything that later wants to read it out of the archive. And a missing `llvm-objcopy` is a hard
failure rather than a quiet fallback to unstripped bytes, because the measured-boot digest
(DECISIONS §26 phase B.1) is taken over exactly these bytes: a build that packed different bytes
depending on which tools were installed would be a build whose trust root meant something different
on each machine.

## The first distinct binary: the least_authority_demo (milestone 19f.2)

The least_authority_demo is the first program that is its own binary, not a role of `hello`. It lives in
`components/src/least_authority_demo.rs`: its own `_start`, its own panic handler, ~30 lines, and not one line of hello's
code. It shares the `user` package's `link.ld` (so it links at `0x40_0000` like hello), which is not
a conflict because each program runs in its own address space. `initrd_aarch64` packs it as a second
archive entry, `"least_authority_demo"`, beside the boot program's.

Every consumer that used to spawn "a role-6 least_authority_demo of hello" now loads `"least_authority_demo"` by name and starts
it with `x0 = 0` (a standalone binary needs no role selector) and the input in `x1`:

- init's `init_least_authority_demo` and the initboot spawn service (`hello.rs`), for `run <n>`.
  (`initboot` is the name that path had when this was written. It was a kernel Cargo feature
  and a `script/initboot`, both deleted by milestone 296, which found the feature selected nothing
  `shell` did not. Read `initboot` here as today's `shell`; the account is otherwise unchanged.)
- the kernel-side `shell_service` (the pre-initboot interactive shell), same `run <n>`. (Retired as
  a boot path by DECISIONS §28 and deleted by milestone 41; it is described here as it was.)

Removing the least_authority_demo from `hello` is what proved the split was real: it broke every one of those call
sites (a role-6 spawn fell through to hello's default arm and *faulted*), and fixing each to load
`"least_authority_demo"` is the migration. The hello binary no longer contains a least_authority_demo at all. Two headless tests
pin it: `a_spawned_least_authority_demo_computes_and_reports` (kernel spawns the least_authority_demo binary, gets 81)
and `init_builds_the_demo_and_passes_it_an_argument` (init loads it by name, gets 49).

The tiny syscall runtime in `least_authority_demo.rs` (`invoke`/`send`/`exit`) is duplicated from hello on purpose.
When 19f.3 splits the next binary, that second copy is the signal to lift a shared user-runtime
crate, with the requirements known rather than guessed (DECISIONS: don't build the abstraction before
the requirements are).

## The console server, its own binary (milestone 19f.3)

"Console server as its own binary" was the headline 19f was aiming at, and here it is:
`components/src/console.rs`, a distinct ELF init loads by the name `"console"`. It owns the UART and one
request/reply channel, loops (receive a length, copy that many bytes from the shared page to the
UART, ack), and holds nothing else. Same shape as the least_authority_demo split: every consumer that entered
hello at the console role now loads `"console"` and starts it with `x0 = 0`. There were three, in
two domains:

- the kernel-side `console_service::start` (`kernel/src/user/console_service.rs`), used by the milestone-8 tour and the
  pre-initboot shell;
- init's `init_console` (the userspace-built-console test) and the `init_boot` console child.

hello lost the console entirely: the `console_server` function, `uart_put`, and the PL011 register
constants are gone from it. hello keeps only the *printing client* that drives a console (a role, for
the tour), because a client is agnostic to who serves it. The test
`userspace_init_brings_up_the_console_server` still passes, now proving the `"console"` binary works
end to end: init builds it, wires a channel, delegates the UART, and the line comes out.

## The input driver, its own binary (milestone 19f.4)

The receive half of the terminal, `components/src/input.rs`, lifted out of hello the same way. It owns the
PL011 receive side and its RX interrupt, assembles a line character by character (echoing as it
goes), and hands each completed line to the shell over IPC. Its consumer is init's `init_boot` child, which loads
`"input"` by name and starts it with `x0 = 0` (a kernel-side `input_service::spawn_wired` did the
same until milestone 41 deleted it with the rest of the retired kernel-wired shell);
`hello` lost the `input` module and role entirely.

With this, the whole interactive stack runs on distinct binaries. Verified end to end by piping real
keystrokes into QEMU's serial: typing `run 6` at the prompt, the input binary read and echoed the
line, the shell (still a hello role) parsed it, init built the least_authority_demo binary with the argument 6,
the least_authority_demo computed 36 and reported, and the console binary printed `6*6 = 36` back. Input,
least_authority_demo, and console, three separate programs, plus init and the shell, cooperating through
capabilities and shared pages.

## The shell, its own binary (milestone 19f.5): the split is complete

The last and most-wired program, `components/src/swish.rs`, lifted out of hello. It holds five capability
slots (console request/reply, the input line endpoint, and the spawn/result endpoints) and two
shared pages, reads a line, and prints. Its consumer is init's `init_boot`, which loads `"swish"` by
name and starts it with `x0 = 0`.

With the shell out, hello contains none of the system's programs. Every service is its own binary
in the archive: `least_authority_demo`, `console`, `input`, `swish`. hello keeps only init and the milestone-tour
demo roles (the printing client, the virtio driver, the capability demos). On the `initboot` path
init loads nothing of hello into a child at all; it builds the whole system from the four distinct
binaries.

Proven end to end by piping keystrokes into QEMU's serial on both interactive paths (kernel-side
`shell` feature and `initboot`): typing `run 9` at the prompt, the input binary read and echoed it,
the shell parsed it, init built the least_authority_demo binary with 9, the least_authority_demo computed 81, and the console
binary printed `9*9 = 81`. Four separate programs plus init, cooperating through capabilities.

One honest wrinkle surfaced: a line burst-piped before the prompt appears loses its first
character, because the input driver arms its RX interrupt a few instructions after it starts and a
pre-armed poll only narrows that window, it does not close it. The input driver's comment used to
claim it "never loses the first character"; it does, under burst-piping. A user typing after the
prompt never hits it, and every line after the first is interrupt-driven and intact. The comment is
corrected; fully closing the window is a separate input-driver fix, not part of the split.

## The shared runtime, `user_mode_runtime` (milestone 19f.6)

With the split done, the `invoke`/`send`/`receive`/`exit` runtime was copied verbatim into five binaries
(hello and the four it shed). `crates/user_mode_runtime` is that runtime, lifted into one library crate all
five now depend on: one `invoke` (the single syscall), and `send`/`receive`/`exit` built on it. The
extraction waited on purpose until the split was complete, so the shared surface was known rather
than guessed (the DECISIONS rule about not building an abstraction before its requirements exist).

Two things deliberately stayed out of `user_mode_runtime`:

- The `#[panic_handler]`. A panic handler is per-final-binary, and one in the shared library would be
  forced on every program that links it and collide with any program (like hello) that wants its own.
  Each binary keeps its own one-line handler; it is trivial, and it keeps the linking simple.
- Device helpers (the UART `putc` and echo logic in the console and input drivers). Those are not
  runtime, they are the program: they belong to the driver that owns the hardware.

## Where the claim lives now that `builder` is gone (milestone 295, 2026-09-14)

**The claim is "userspace, not the kernel, composes a process", and it used to be made twice on
riscv64.** `components/src/builder.rs` made it in miniature as a step in the boot tour: the kernel
loaded that one program out of the archive, granted it a budget and a report endpoint and nothing
else, and it parsed `least_authority_demo` out of the same archive in userspace, built a child from
its own budget and started it. The kernel never touched the child's bytes. The progenitor makes the
same claim at the scale of a whole system, and since milestone 268 item 4 the default riscv64 boot
reaches it too, so calef retired `builder`.

**Split the claim in two, because only one half moved cleanly.**

*Userspace composes a process.* Carried by the progenitor on every architecture that runs one
(`boot_progenitor`, one function on all three architectures since milestone 166),
on the boot a card performs. On `x86_64` the progenitor composes the system but its console
server and input driver stop on first use, because a ring-3 process cannot reach port I/O (DECISIONS
§121); how a shell gets a console there is DECISIONS §149.
This half is better off than it was: the progenitor composes the console server, the line
discipline, the input driver and `swish`, and a person can then type at the result.

*...from an authority you can count on one hand.* This is the half `builder` carried alone. The
progenitor does not carry it: it is granted the NS16550 and the UART's interrupt line as well,
because it is building a system rather than demonstrating a floor.

What carries it since 2026-10-05 is `fixtures/src/process_composition_witness.rs`, built for §185
(what carries the claim that userspace composes a process from an authority you can count on one
hand) as milestone 404 (composing a process from two capabilities is proved for two verbs and no
more). It holds the same two capabilities `builder` held, a memory region in slot 0 and a report
line in slot 1, plus the archive mapped read-only. From those it reads `least_authority_demo` out of
the archive by name, mints a rendezvous, builds the child through
`supervision_protocol::build_child_space`, maps a frame of its own into the child's space and is
refused the same address twice, then configures and starts the thread and receives its answer.
`system_tests::user::tests::a_process_composed_from_two_capabilities_runs_in_the_space_it_built`
asserts every step on all three architectures under `script/test`, which is more than `builder` ever
had: nothing on a pull request executed `builder` (milestone 406 (nothing on a pull request boots
the riscv64 tour, so its userspace step is unasserted),
`design/roadmap/406-nothing-in-ci-boots-the-riscv-tour.md`).

Until then the fixture was `address_space_witness` and stopped where milestone 19b (run a real
workload) stopped, with nothing running in the space it built, while
`system_tests::user::tests::a_process_can_build_start_and_run_a_child_thread` drove the whole
sequence from the kernel's side. The gap was a join rather than a hole, and the block of milestone
295 (retire `components/src/builder.rs`) carried it as its handoff until 404 closed it.

## What is not here yet

**Resolved since this was written.** The kernel's own pre-init service wiring is gone: §28 retired
`shell_service` as a boot path and milestone 41 deleted it and `input_service` outright, because they
had no caller in any configuration. `console_service` remains, spawned by the milestone tour only.
Every interactive build now reaches userspace through `boot_via_progenitor`, so the kernel has one way in
rather than two.
