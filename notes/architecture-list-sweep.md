# Every place in the tree that enumerates architectures, and whether the list is complete

<!-- writing-standards: exception. Marked 2026-09-29 (UTC) by the audit lane for the documentation
audit of that date. Reason: the one correction below closes a recorded-gap row that closed without
the row knowing, and bringing a 6,200-word sweep to 4 bold spans per 1,000 words is a rewrite for
this note's owner, not something to hide inside a correction, the same line
notes/timed-wait.md's marker takes. Remove this marker when that de-bold pass lands. -->

Swept 2026-08-27, prompted by one instance. `script/stack-frame-check` line 59 read
`arches="aarch64 riscv64"` and exited 2 on anything else. That gate was written 2026-08-14, when
there were two architectures; x86_64 became a `script/test` target on 2026-08-24 (commit
`357a226f`, milestone 161 (the x86_64 kernel port)), ten days later. The premise holds fine on
x86_64: the target is `x86_64-unknown-none`, `kernel/src/arch/x86_64/mmu.rs` prints "guard pages
are holes" from its own map builder, and `sched::every_thread_stack_has_a_guard_page` is
architecture-independent. Nothing technical stopped the third entry going in. Nobody added it, and
the script's `BUGS` section, which honestly records four other limitations, did not record this
one, so a reader met a section that looked complete.

That is the rung-2 failure `AGENTS.md`'s ladder describes. The coverage decision lived only in a
hardcoded string, and when a third architecture arrived nothing fired. The question this note
answers is how many more of those there are, which is `notes/arch-audit.md`'s question in a
different domain and is asked here for the same reason: the interesting output is not the bug that
prompted the sweep, it is the count of its siblings.

## The short answer

Eleven silent gaps, in nine files. Seven are lists that were correct when written and were never
widened; two are content (an audit's scope, a script's `EXAMPLES`) rather than lists; one is a
class of four `#[cfg]` pairs whose failure mode is a silently empty function rather than a missing
entry; one is a tripwire that exists, is committed, and is not armed.

Findings 1 through 10 are the sweep as it was first run. Finding 11 was added when the lane holding
`script/fastpath-footprint` turned out to have assessed and not changed it, so it stopped being
somebody else's and became one more of these. The prompting instance, `script/stack-frame-check`,
is deliberately **not** in the table: another lane converted it into a recorded gap the same night,
and that conversion is described below because it is the model for how the rest should end.

Nine recorded gaps, which rule 5 explicitly allows and which are working as designed. They are
listed below so a later reader does not re-derive them as findings. `script/lint`'s x86_64 block is
the model and is worth reading before writing another one.

Fifteen-odd things that are legitimately one architecture's, which are not findings and are
listed only so the next sweep does not spend an hour rediscovering that a device tree does not
apply to a PC.

## Disposition, 2026-09-23, by milestone 186

Twenty-seven days after the sweep. Seven of the eleven are closed, one is now a recorded gap, one
is another milestone's, and two were already closed by other lanes before this one looked, which
is the finding worth carrying: a sweep's table is a snapshot, and a lane that acts on it without
re-checking each row will fix something twice and miss what moved.

| # | Where | Disposition |
|---|---|---|
| 1 | `script/stack-depth-check` | **Open.** Phase 3, and not a widening: its `TRAP_FRAME`, `DISPATCH` and `BODY` tables each need an x86_64 row measured against that architecture's own `size_of::<TrapFrame>()` and symbol names, and then the gate may find a real offender. Recorded in its own `BUGS`, which is where it already was |
| 2 | `script/bootstrap` | **Closed.** Derived from `rust-toolchain.toml`'s `targets`, the way `channel` above it already was |
| 3 | `script/drift` | **Was already closed**, by milestone 312 (`script/drift`'s lists are derived), which derives both the target list and the bare-metal subset. Its comment is the model the other two copied |
| 4 | `script/toolchain-bump` | **Closed**, and `.github/workflows/toolchain-bump.yml` with it, which the sweep did not list. Both install against the **new** toolchain, so the pin's array does not cover them; this was the copy that actually bit |
| 5 | `deny.toml` | **Closed.** `x86_64-unknown-none` added and the stale scope note replaced with what happened. `script/supply-chain` is green with the target in, so nothing was hiding in that graph. TOML cannot call a script, so it stays hand-kept and its `BUGS` now says so |
| 6 | CI's bench job | **Was already closed**, on 2026-09-15: `script/ci-build`'s `bench` row runs three legs |
| 7 | `script/bench`'s `EXAMPLES` | **Closed.** All three architectures and their baselines are in the header, with `--real` named as aarch64-only by construction rather than by omission |
| 8 | `notes/arch-audit.md`'s scope | **Another milestone's**, 187 (read the x86_64 arch tree through the lens the first arch audit used), which was minted for it |
| 9 | five driver `barrier()` functions | **Closed, and it was five rather than four.** See below |
| 10 | `components/src/pgrep.rs` | **Was already closed** with the sweep itself |
| 11 | `script/fastpath-footprint` | **Was already closed**: `arches` is three and x86_64 is measured. CI's job for it still installed two targets a month later, which this milestone fixed |

### Finding 9 was five copies, not four, and the fix is one function

The sweep named `crates/virtio`, `gpu_driver.rs`, `keyboard_driver.rs` and `net_transport.rs`. The
fifth is **`components/src/entropy.rs`**, found by the fatal-risk-9 lane on 2026-09-23 and confirmed
by re-running the sweep's own method 3: it had the same two arms, no third, and no `BUGS` note, so
it was the one copy nobody had even recorded. A sixth `barrier()` exists in
`components/src/non_volatile_memory_express.rs` and was never in this class: it has all three arms.

They are now one function, `user_mode_runtime::virtio::virtio_ring_barrier` (name provisional),
in the module where those four programs already get `virtio_read_reg` and its siblings. That is a
higher rung than five `compile_error!` arms would have been, which is what option B priced: the
fallback now exists once, and a seventh driver gets the third architecture by calling the
function rather than by remembering to write an arm.

What x86_64 needed, which the sweep left open as a rule 4 call. A compiler fence, and the
reasoning is per-site rather than a default. Every call site in those five programs asks for one of
two orderings: store-store (publish the descriptor before the available index; publish the index
before the notify) or load-load (read the used index before the payload it gates). x86_64 gives both
for the write-back memory a coherent DMA agent shares. The store-load case TSO does not give is
unreachable here, because every notify and every register access in these drivers leaves the process
through a syscall instruction, which is serializing. So the compiler is the entire exposure.

`non_volatile_memory_express.rs` is the control that makes this argument falsifiable rather than
convenient: it keeps `dmb sy` and a real `mfence`, because its doorbell is a direct store to a
device-typed page in this address space with no syscall between the ring stores and it. Same
hardware, different answer, for a stated reason.

### What is still open after this milestone

- Finding 1, `script/stack-depth-check`, which is phase 3 work that may surface an offender.
- Finding 8, which is milestone 187 (read the x86_64 arch tree through the lens the first arch
  audit used).
- The rung-1 limit this note already stated and which still holds: deriving the list removes the
  copy, not the incompleteness. Nothing stops a new gate writing `for arch in aarch64
  riscv64`, and `deny.toml` cannot read the derivation at all. `virtio_ring_barrier`'s
  `compile_error!` is the one place in this sweep's territory where a missing architecture now
  fails to build rather than failing to order.

## The three classifications, and why the classification is the deliverable

Rule 5 in `AGENTS.md`, from §19 (architectural parity is a tenet):

> A kernel capability ships on every supported architecture, proven by the same suite, or a scope
> note records the gap and the plan. If a feature works on one ISA and silently not another, that
> is the bug.

The word doing the work is silently. A hit is a finding only when all three of these hold:

1. An architecture is absent from the list, match, table, file set, or matrix.
2. The premise would hold there. A stack-frame ceiling means something on x86_64; a device tree
   fixture does not.
3. Nothing in the tree says so. Not a `BUGS` entry, not a `skip!()` reason, not a scope paragraph
   beside the code, not a roadmap block.

Fail 2 and it is architecture-specific. Fail 3 and it is a recorded gap. A sweep that reports the
raw count instead of the split is worse than one that reports four real gaps, because the reader
has to redo the judgment.

## Silent gaps

| # | Where | What enumerates | Missing | Evidence that the premise holds |
|---|---|---|---|---|
| 1 | `script/stack-depth-check:89`, and its `TRAP_FRAME`, `DISPATCH`, `BODY` dicts at `:156`, `:161`, `:170` | `arches="aarch64 riscv64"` plus three two-key dicts | x86_64 | `kernel/src/arch/x86_64/` has `exceptions.rs` and `trap.s`, so both a trap-frame size and a dispatch symbol exist to name. Its `BUGS` section carries six entries and none is this one. |
| 2 | `script/bootstrap:32` | `rustup target add --toolchain "$channel" aarch64-unknown-none-softfloat riscv64imac-unknown-none-elf` | `x86_64-unknown-none` | The next paragraph in the same file checks for all three QEMU emulators and says "`cargo xtask test` now needs the full set". The bootstrap installs two of the three targets that sentence requires. |
| 3 | `script/drift:33`, and `:44` | `rustup target add` list; and a bare-metal build that is aarch64 only | `x86_64-unknown-none` in the list; riscv64 and x86_64 in the build | The job's whole purpose is "does a toolchain still build us". A nightly that breaks only the x86_64 kernel is invisible to it. Note the second half independently: riscv64 is installed on line 34 and never built. |
| 4 | `script/toolchain-bump:38` | `rustup target add --toolchain "$new"` list | `x86_64-unknown-none` | A bump installs two of three targets against the new pin, so the first x86_64 build after a bump is the one that discovers it. |
| 5 | `deny.toml`, the `targets` array | two bare-metal targets and three hosts | `x86_64-unknown-none` | `cargo-deny` resolves the dependency graph per target, so a crate reached only under `cfg(target_arch = "x86_64")` gets no advisory, license, or ban verdict. **This one had a scope note and the note went stale**, which is the worse variant: it said x86_64 bare metal "is declared in §19 but not started; it joins this list when it does", and it started. A reader who checked was told the gap was tracked. |
| 6 | `.github/workflows/ci.yml:390` | the bench job's two legs, `script/bench --check` and `script/bench --riscv --check` | an x86_64 leg | `cargo xtask bench --x86 --check` exists (`xtask/src/bench.rs`, `bench_x86`) and `bench/baseline-x86_64.txt` is committed. The tripwire is built, the baseline is recorded, and nothing pulls it. |
| 7 | `script/bench:5` | the header's `EXAMPLES`, which names only `bench/baseline-aarch64.txt` | `--riscv` and `--x86` | CI runs `--riscv` and the script's own documentation does not mention it exists. Two of three architectures are undiscoverable from the front door. |
| 8 | `notes/arch-audit.md:41` | "Everything under `kernel/src/arch/`, read in full, both ISAs", and a two-column table | x86_64 | The note contains zero occurrences of "x86". `kernel/src/arch/x86_64/` is 18 files and 6,797 lines, which is **larger than the 6,200-line two-ISA tree that audit read in full**. The audit's own argument is that hand-written arch assembly is the least-verified code in the trusted computing base; a third of it is now unread. |
| 9 | `crates/virtio/src/lib.rs` `barrier`, `components/src/gpu_driver.rs` `barrier`, `components/src/keyboard_driver.rs` `barrier`, `components/src/virtio_net_transport.rs` `barrier` | two `#[cfg(target_arch)]` arms with no third arm and no fallback | x86_64 | All four compile for x86_64 (`initrd_x86` builds `-p user` for the target; `script/lint` clippies `-p user -p user_mode_runtime --target x86_64-unknown-none --all-targets -- -D warnings` and passes). On x86_64 both arms compile out and the function body is empty, so it emits no compiler fence at all. TSO covers the machine half and nothing covers the compiler half. |
| 10 | `components/src/pgrep.rs`, its `#[panic_handler]` | the same two-arm shape, for a deliberate trap (`brk #0` / `ebreak`) | x86_64 | On x86_64 neither arm compiles, control reaches the spin loop, and a panic becomes a thread burning CPU forever instead of the kill the comment above it promises. `pgrep` is the **only** program in `user/` with a hand-written panic handler of this shape; every other one goes through `crates/user_mode_runtime`, whose arms cover all three ISAs. **Fixed on this branch**, since it is a live defect rather than a coverage gap. |
| 11 | `script/fastpath-footprint`, its `arches` and its `ROOTS` table | `arches="aarch64 riscv64"` plus a two-key `ROOTS` dict | x86_64 | The file has no occurrence of "x86" at all. The IPC fastpath is exactly the kind of thing §19 says must not fit on one ISA and silently not another. Not a one-word widening: it needs `bench/fastpath-x86_64.txt` and `kernel/src/arch/x86_64/fastpath_pad.rs`, neither of which exists, plus an x86_64 `ROOTS` row. |

### The one to read first, because it names its own trap

`script/lint`'s x86_64 user pass carries this sentence, written to justify its own existence:

> Without it, a lane could add an aarch64/riscv64 pair to a user program and never learn that x86
> fell through to nothing until `cargo xtask test --arch x86_64` failed to pack an archive.

Findings 9 and 10 are exactly that, and that gate does not catch them, because falling through to
nothing compiles clean. An empty function is not a warning. This is the strongest argument in the
sweep for the `compile_error!` arm proposed below: the gate written to catch this class cannot,
and the author knew what to look for.

### Two notes on severity, because overclaiming would waste the reader's time

Finding 9 is latent; finding 10 is live but rarely reached. The x86_64 QEMU runner attaches no
virtio disk, NIC, GPU, or RNG (`helpers/qemu-runner-x86_64.sh` says so in its own header), so no
x86_64 boot reaches a virtio ring today; the exposure is that the code is compiled, is shipped in
the archive, and is wrong the day a device is attached, which is a day the roadmap plans for.
Finding 10 needs no device: any panic in `pgrep` on x86_64 hangs the thread now. It is small only
because that panic handler is the path the comment above it calls "nothing here should panic".

Findings 2 through 5 are mitigated by `rust-toolchain.toml`, which carries
`targets = ["aarch64-unknown-none-softfloat", "riscv64imac-unknown-none-elf", "x86_64-unknown-none"]`
and is the reason CI's clippy job installs only aarch64 explicitly and still lints x86_64
successfully. rustup reads that array. So the stale `rustup target add` lines are dead weight
against the pinned toolchain rather than a live breakage. They are still findings, for two
reasons: `script/drift` and `script/toolchain-bump` both add targets to a different toolchain
than the pin, where the file's array does not apply; and a list that is wrong and harmless today is
the exact shape of `script/stack-frame-check` on 2026-08-24.

### The two that were another lane's while this ran, and how they ended differently

Both are the same shape as finding 1 and both were held by the lane on pull request #567 while this
sweep ran. That pull request landed 2026-08-27, and it settled one of them and not the other, which
is worth recording because the split is instructive.

`script/stack-frame-check` is now a recorded gap, and is the best example in the tree of the
conversion this note keeps asking for. The lane widened it to accept `--arch x86_64`, ran it, and
found a real offender the first time: `kernel::arch::x86_64::iommu::init` at 12,504 bytes
against a 4,096-byte ceiling, from an `Iommu { ctx: [Option<u64>; 256], .. }`, which is the same
"`[T; MAX]` local sized to a table maximum" shape that gate's own first `BUGS` entry describes. So
x86_64 is reachable but deliberately out of the default set, with a `BUGS` entry saying so, saying
what was found, and saying that whether to fix the offender or except it is not that gate's call.
`main` stays green by naming the finding rather than by not looking. That is exactly the phase-3
outcome milestone 186 plans for, arrived at a day early.

`script/fastpath-footprint` was assessed and left alone, and it is still
`arches="aarch64 riscv64"` with no x86 mention anywhere in the file. So it is not a claimed item
any more; it is finding 11, unclaimed, and it is recorded in its own `BUGS` on this branch. It has
two absent companions worth naming for whoever takes it: `bench/fastpath-x86_64.txt` and
`kernel/src/arch/x86_64/fastpath_pad.rs` do not exist, where both other architectures have both.
The two tripwire file sets diverged: `bench/baseline-<arch>.txt` is complete at three and
`bench/fastpath-<arch>.txt` is at two.

The ten-versus-eleven count. The table above is the sweep as it was run, when
`script/stack-frame-check` was the prompting instance and `script/fastpath-footprint` was somebody
else's. Read forward from today it is eleven silent gaps, one of which (`stack-frame-check`) was
converted to a recorded gap by another lane before this note landed. The table is not renumbered,
because an inventory records what the sweep found rather than what survived it.

`.github/workflows/ci.yml` installs two bare-metal targets for each of those two jobs, under a
comment that reads "Both bare-metal targets, because this gates both (§19 parity: a frame that fits
on one ISA and not the other is exactly the asymmetry that rule exists to catch)". That comment
cites the parity rule as the reason for enumerating two of three architectures. It follows the
scripts and should be fixed with them, not separately.

## Recorded gaps, which are fine

Listed so the next reader does not count them twice.

| Where | Missing | Where the reason is written |
|---|---|---|
| ~~`xtask/src/main.rs` `STD_TARGETS: [&str; 2]`, and `targets/` holding two `.json` specs~~ | ~~`x86_64-unknown-nife`~~ | **Closed** by milestone 184 (extend the `std` port to x86_64): `STD_TARGETS` has three entries and `targets/` three specs |
| `xtask/src/suite.rs`, the vendored RedoxFS build loop `for target in [TARGET, RISCV_TARGET]` | x86_64 | milestone 164 (x86_64 userspace can't build `aes`). The reason is written at `initrd_x86` and at `fs_service.rs`'s `NO_FS_SERVER`, **not at the loop itself**, which is the one weakness in an otherwise clean record |
| ~~`xtask/src/swish_check.rs` `swish_check`, refusing `--arch x86_64`~~ | ~~an x86_64 leg~~ | **Closed**: the leg runs (`swish_check.rs:59`), milestone 182 (x86-64's own interactive-boot entry point) built it, and milestone 177 (wire the graphical terminal stack into the real interactive boot) is BUILT |
| `xtask/src/icount.rs` `icount`, which refuses `--arch x86_64` | an x86_64 icount leg | The real reason is at `bench_x86`'s doc comment (the LAPIC timer is a periodic hardware reload with no re-armed deadline to compare against). **The inline comment at the match arm gives a different and now-stale reason** ("needs a userspace this port cannot build"), which stopped being true when milestone 161 item 4 landed. Worth a one-line correction by whoever next touches it |
| `script/lint`, the boot-mode feature loops `for tgt in aarch64... riscv64...` | x86_64 | Thirty lines above the loop, naming each narrowing, why it is a scope gap rather than an oversight, what is still covered, and **the trigger that retires the suppression**. This is the model |
| `kernel/src/user/fs_service.rs` `NO_FS_SERVER`, `NO_MKFS`; `clock_service.rs` `NO_RTC`; `user.rs` `NO_UART_PAGE`; `std_service.rs` `NO_STD_EXERCISER` | per-test x86_64 coverage | The `skip!()` mechanism, whose whole design is that a skipped fixture prints why |

## Legitimately architecture-specific, which are not findings

`script/board-image` (riscv64 only; it builds a U-Boot `booti` image for a StarFive board).
`script/cpu-matrix` (riscv64 only, and its first line says so; it sweeps QEMU's riscv CPU models).
`script/ci-build`'s `hvf` check (aarch64 only by construction: Hypervisor.framework runs the host's own
ISA). `crates/device_tree_blob/tests/qemu_aarch64_virt.rs` and `qemu_riscv64_virt.rs` with their `.dtb` fixtures
(q35 has ACPI and no device tree). `notes/riscv-tlb-shootdown.md` and `notes/x86-tlb-shootdown.md`
with no aarch64 twin (aarch64 broadcasts `TLBI` in hardware, so there is no software shootdown to
document). `notes/riscv-port.md` and `notes/x86-port.md` with no aarch64 twin (aarch64 is the
original, not a port). The per-architecture assembly whose names differ because the hardware does
(`vectors.s` against `trap.s`, `image_header.s`, `segments.rs`, `port.rs`, `rtc.rs`, `ap_boot.rs`,
`machine.rs`). `script/qemu-check`'s device probes, which ask each emulator for the device that
emulator needs.

## The things that are complete, because they are the answer to the design question

These are worth more than the findings, because each one is a shape that did not go stale.

- **`kernel/build.rs`**: `match arch` over three arms with `other => panic!("nife has no linker
  script for target arch {other}")`. A fourth architecture fails the build with a sentence naming
  the file to edit.
- **`kernel/src/arch/mod.rs`**: three `#[cfg]` module arms and three flat re-exports, and the
  module's own comment says a new ISA is a new directory rather than a diff.
- **`crates/elf`'s `EXPECTED_MACHINE`**: three explicit arms because it was two and the default
  arm was a bug. It read `#[cfg(not(target_arch = "riscv64"))] EM_AARCH64`, so the x86_64 kernel
  was compiled to accept aarch64 binaries and refuse its own. Its comment states the general lesson
  in one sentence: "A default arm that names one architecture is a trap the moment a third exists."
- **`xtask`'s `ArchLegs`**: an enum whose doc records the same correction. It was `Both`, its two
  predicates were written as `self != the_other_one`, and that answers `true` for every leg the
  moment a third variant exists. Now `All` with explicit `matches!`.
- **`rust-toolchain.toml`'s `targets`**: three entries, and the only architecture list in the tree
  that a tool reads rather than a human copies.
- **`.cargo/config.toml`**: three `[target.*] runner` blocks. **`crates/paging/src/`**: `aarch64.rs`,
  `sv39.rs`, `x86_64.rs`. `crates/machine_discovery/src/`, `bench/baseline-<arch>.txt`,
  `helpers/qemu-runner-<arch>.sh`, `kernel/link-<arch>.ld`, and twelve files under
  `kernel/src/arch/<arch>/`: all complete at three.

The pattern. Everything that stayed complete is either a Rust `match` the compiler pushed on,
or a per-architecture file whose absence a build notices. Everything that went stale is a
space-separated string in a shell script, a YAML step, a TOML array, or a sentence in a note.
Ten of the eleven silent gaps are in the second group and the eleventh is a `#[cfg]` pair with no `else`.

### One hazard that is not a finding today

Ten sites spell `#[cfg(not(target_arch = "x86_64"))]` or `#[cfg_attr(not(target_arch = "x86_64"),
...)]`, in `kernel/src/console.rs`, `memory.rs`, `pci.rs`, `smp.rs`, `user/clock_service.rs`, and
`user/supervision_tests.rs`. Every one is correct today, because "not x86_64" is exactly
"aarch64 or riscv64" while there are three architectures. Every one is the shape that made
`EXPECTED_MACHINE` wrong, and each will silently include a fourth architecture on the day one
arrives. Named here rather than filed as ten bugs, because rewriting correct code on a hypothetical
is the gold-plating the tenets refuse. What it argues for is that a fourth port's first task is a
grep for `not(target_arch`, which is the sort of thing a note is for.

## The design question: is there a rung-1 answer

Widening ten lists by hand is rung 2 done ten times. It fixes 2026-08-27 and does nothing on the
day a fourth architecture arrives, which the roadmap contemplates. So: what would make an
incomplete list hard or impossible to write? Three candidates, priced rather than asserted, with
the six questions `AGENTS.md` asks of a fork answered where they apply.

### Option A: one derived list of supported architectures and their triples

What. A single place naming each supported architecture and its triple, that every shell script
and every `xtask` path reads instead of spelling.

What the tree already does here, which is most of the answer. `rust-toolchain.toml` already
carries the list, complete at three, and is already the authority for the pinned toolchain: rustup
reads that array, which is why CI's clippy job explicitly installs only aarch64 and still lints
x86_64 clean. `script/bootstrap` already parses that same file with a one-line `sed` to get
`channel`. So the mechanism, the file, and the parsing technique all exist, and the gap is that
nobody extended the parse by one field.

What it costs, measured. The arch-to-triple table is currently written out four times:
`script/stack-frame-check`, `script/stack-depth-check`, `script/fastpath-footprint` (all three as
`case "$arch" in ... esac`), and `xtask`'s `TARGET` / `RISCV_TARGET` / `X86_TARGET` constants. The
bare triple list is written out four more times: `script/bootstrap`, `script/drift`,
`script/toolchain-bump`, and `deny.toml`. A `script/` entry point printing `arch<TAB>triple` per
line is about fifteen lines of POSIX sh reading `rust-toolchain.toml`. Each `case` block becomes a
lookup; each `rustup target add` becomes a command substitution. `deny.toml` is TOML and cannot
call a script, so it stays a hand-maintained list and needs option C or a counted claim to keep it
honest.

**What it does not do**, and this is the honest limit. It removes the copy, not the
incompleteness. A new gate can still write `for arch in aarch64 riscv64` and nothing stops it.
It also does nothing for findings 6, 7, and 8, which are a missing CI step, a stale `EXAMPLES`
block, and an unaudited directory. Those are content, not lists.

Reversibility. Total. It is one script and its callers, nothing leaves the machine, no wire
format and no name a stranger has learned. By the *move fast on what can be undone* test, nobody
outside this repository has acted on it.

### Option B: a type with no default arm

What. An enum the compiler forces exhaustive matching on, and a `compile_error!` arm where
`#[cfg]` does the dispatching.

This half already exists and has already paid for itself. `ArchLegs` is that enum, and its own
doc records the bug it was created by: two variants, predicates written as `self != the_other_one`,
and every leg answering `true` once there was a third. `crates/elf`'s `EXPECTED_MACHINE` is the
same correction in the same week. Its reach is the problem: no shell script, YAML step, or TOML
array can see a Rust enum, and nine of the eleven silent gaps live in exactly those three languages.
So B is already taken where it applies and cannot apply where the gaps are.

Where it extends, and this is the half worth building. Findings 9 and 10 are five `#[cfg]`
pairs whose x86_64 behavior is an empty function body. Adding

```rust
#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "x86_64")))]
compile_error!("barrier(): this architecture has no ordering instruction named here");
```

turns "fell through to nothing" into a build failure. Cost: five sites, three lines each,
mechanical. Precedent in the tree: `entropy_backend`'s backend ladder, whose own module doc says it
"ends in `compile_error!` rather than a fallback", which is the only existing use of the macro here.
Reversible.

**The caveat that makes this a partial answer rather than a fix.** `compile_error!` catches the
*fourth* architecture. It does not catch x86_64 today, because x86_64 is inside the `any(...)` and
the arm still emits nothing for it. Closing finding 9 properly means deciding what a barrier is on
x86_64: `compiler_fence` for the compiler half, since TSO covers the machine half. That is a
memory-ordering decision under rule 4, not a sweep's call, and it is the same argument in four
copies, which is `AGENTS.md` rule 7's own case for one crate rather than four functions. Flagged as
a fork rather than answered here.

Finding 10 is not this option's shape at all and should not be bundled with it. `crates/user_mode_runtime`
already has a panic handler with arms for all three ISAs, and `pgrep` is the only program in
`user/` that hand-rolls its own. The fix is to delete the hand-rolled one, which is deletion rather
than mechanism.

### Option C: a gate comparing each per-architecture file set against the list

What. For every family of files whose name carries an architecture token, assert the set equals
the supported list.

What the tree already does here. `script/lint`'s counted-claims check is exactly this shape one
domain over: it re-derives a number from the tree and fails when a written claim disagrees, with
the marker at the claim (`<!--count:sh-scripts-->`, `notes/counted-claims.md`). A file-set check is
the same primitive over globs instead of counts.

What it costs, measured, and this is why it loses. There are sixteen file-set families in the
tree today. Three are legitimately incomplete and would need an exception on day one (the `dtb`
fixtures, the port notes, the shootdown notes). Three more are recorded gaps and would need an
exception until their milestones land (`targets/*.json`, `bench/fastpath-*.txt`,
`fastpath_pad.rs`). That is six exceptions against sixteen families: the exception table would
be more than a third of the check on the day it shipped. `script/stack-frame-check`'s own `BUGS`
already records what that costs, in the entry that says its exception table is maintained by hand
and that a function which moves keeps its exemption with nothing noticing.

**And it would not have caught the finding that prompted this sweep.** `script/stack-frame-check`'s
gap was a string inside a script, not a missing file.

### The recommendation

This fork is reversible in every part, so it gets a recommendation rather than options, which is
the line `AGENTS.md` draws.

**Take A, take B's `compile_error!` arm, do not take C.**

A, because five of the ten findings are a copied list that went stale and A is the only option that
removes the copying. Its own limit is real and worth saying in the same breath: it prevents
divergence, not omission.

B's arm, because findings 9 and 10 are the only ones whose failure mode is silence rather than
absence, and silence is the mode no reader and no reviewer catches. Three lines at five sites, with
an in-tree precedent.

Not C, because a check that ships with an exception table a third its own size is a check whose
exceptions become the artifact, which is the failure `stack-frame-check` already documents about
itself.

What none of the three fixes, stated plainly so it is not mistaken for covered: finding 6 wants
a CI step, finding 7 wants two lines of `EXAMPLES`, and finding 8 wants somebody to read 6,797
lines of x86_64 assembly and Rust the way `notes/arch-audit.md` read the other two. Those are work,
not mechanism.

On prior art outside this tree: not researched. `AGENTS.md` asks for it read rather than
recalled, and this lane did not go outside the repository, so nothing is claimed. If it is worth an
hour, the question to ask is how a multi-target project keeps one target list authoritative across
a build system, a CI matrix, and a package manifest, since that is the general form of option A.

## Where the work went

Closing the eleven silent gaps is a real body of work rather than a sed. Finding 1 needs an x86_64
trap-frame size and dispatch symbol names; finding 6 needs a CI leg that may find real drift the
first time it runs; finding 8 is an audit. It is **milestone 186 (derive the architecture list, and
close what it does not reach)**, minted on calef's question against this sweep, and this note is
that milestone's worklist. The block carries the phasing and the scope refusals; the table above
carries the items.

Two things the milestone is explicit about, because both were decided here.

**Do not do it as one commit.** Widening an architecture list newly gates that architecture, and a
gate that has never run against x86_64 may have real offenders. It already does:
`script/stack-frame-check --arch x86_64` surfaced `kernel::arch::x86_64::iommu::init` at 12,504
bytes against a 4,096-byte guard page, found by the lane on pull request #567 the same night. Ten
widenings in one commit turns `main` red for a reason unrelated to whoever pushed and leaves nobody
able to tell which one did it. Milestone 96's loader commit is the precedent: separateness is the
argument.

**Finding 10 was not scheduled, it was fixed.** `components/src/pgrep.rs`'s panic handler was a live
defect rather than a coverage gap: on x86_64 neither arm compiled, control reached the spin loop,
and a panic burned a thread forever. It was the last hand-rolled panic handler left outside
`user_mode_runtime` after milestone 130 swept forty-eight of them, and the fix was to use
`user_mode_runtime::panic_handler!()` like every other program in `user/`. Kept in the table above as a
finding, since the point of an inventory is what the sweep found rather than what survived it.

**Finding 8 is deliberately not in milestone 186 either**, and for the opposite reason: it is too
large, not too small. Reading 6,797 lines of x86_64 assembly and low-level Rust on
`notes/arch-audit.md`'s terms does not fit beside a `rustup target add` line. Proposed there,
provisionally, as its own milestone.

## EXAMPLES

Reproduce the sweep, or run it again after a fourth architecture lands.

The list that should be authoritative:

```sh
$ grep '^targets' rust-toolchain.toml
targets = ["aarch64-unknown-none-softfloat", "riscv64imac-unknown-none-elf", "x86_64-unknown-none"]
```

Every hand-copied triple list, to diff against it:

```sh
$ git grep -n 'aarch64-unknown-none-softfloat' -- script scripts .github '*.toml' \
    | grep -v 'x86_64-unknown-none'
```

Each hit is a place that spells the list and does not mention the third target. Read each one:
some are legitimately single-target (`script/crate-probes` builds one spec on purpose).

Per-architecture file sets, and which are incomplete:

```sh
$ git ls-files | grep -E 'aarch64|riscv64|x86_64' | grep -v '^vendor/' \
    | sed -E 's/(aarch64|riscv64|riscv|x86_64|x86)/<ARCH>/g' | sort | uniq -c | sort -rn
```

A family with a count of 3 is complete. A count of 2 is either a recorded gap, a legitimate absence,
or a finding, and only reading it tells you which. This is the method that found
`bench/fastpath-<ARCH>.txt` at two beside `bench/baseline-<ARCH>.txt` at three.

Two-arm `#[cfg]` blocks with no fallback, which is finding 9's shape:

```sh
$ git grep -l 'target_arch' -- '*.rs' | grep -v '^vendor/' | while read f; do
    a=$(grep -c 'target_arch = "aarch64"' "$f")
    r=$(grep -c 'target_arch = "riscv64"' "$f")
    x=$(grep -c 'target_arch = "x86_64"' "$f")
    [ "$a" -gt 0 ] && [ "$r" -gt 0 ] && [ "$x" -eq 0 ] && echo "$f"
  done
```

Default-arm traps, which is what made `EXPECTED_MACHINE` wrong:

```sh
$ git grep -n 'not(target_arch' -- '*.rs' | grep -v '^vendor/'
```

## Methods, and what each one cannot see

Four methods, run in this order. Stated because a later reader needs to know the shape of the hole
rather than trust the count.

1. Grep for architecture names and triples across `script/`, `helpers/`, `xtask/src/`,
   `.github/`, `bench/`, every `Cargo.toml`, `.cargo/config.toml`, `rust-toolchain.toml`,
   `deny.toml`, `kernel/build.rs`, and `notes/`, ranked by hits per file, then read every file with
   a hit. Found findings 1 through 7.
   **Blind to**: a list that derives its members instead of spelling them, and a list that spells
   them in a language I did not think to grep (there is no Makefile or Dockerfile matrix here, but
   a future one would be missed by this method).
2. Per-architecture file-set completeness, by normalizing every tracked path's architecture
   token and counting the families. Found the `bench/fastpath-` divergence and confirmed twelve
   families complete at three.
   **Blind to**: a per-architecture thing whose files do not carry the token in the name.
   `crates/paging/src/sv39.rs` is riscv64's page-table format and this method scored that family as
   incomplete until it was read by hand. So the method produces false positives, which is safe, and
   would produce a false negative for any family named the way `sv39` is.
3. `#[cfg(target_arch)]` arm counting per file, then reading every file with a nonzero count
   and a zero in one column. Found findings 9 and 10, and the `not(target_arch` hazard class.
   **Blind to**: a runtime dispatch on architecture rather than a `cfg`, and a `cfg` written through
   a build-script-generated `cfg` name. `kernel/build.rs`'s `cfg(initrd)` is exactly the second
   shape, and it is complete, but this method would not have told me that; reading `build.rs` did.
4. **Reading the prose claims**: every "both ISAs", "both architectures", "two architectures", and
   "both bare-metal" in the tree, filtered to those with no x86 mention nearby. About forty hits,
   and almost all of them are correct: a note recording that something was proven on both ISAs
   on a given day is a record of what happened, not a list a gate reads, and rewriting it would be
   falsifying a record. Only two were coverage claims rather than history: `notes/arch-audit.md`
   (finding 8) and the CI comments cited above.
   **Blind to**: a claim phrased without those words.

What no method here covers. This sweep read `notes/`, `script/`, `helpers/`, `xtask/`,
`.github/`, the manifests, and the `#[cfg]` sites. It did **not** read `design/roadmap/` or
`design/decisions/` for incomplete architecture lists, on the ground that a roadmap block is intent
rather than a gate and a decision records what was decided when it was decided. If a decision's
list is load-bearing for a current gate, this sweep missed it. It also did not run any of the gates
against x86_64 to see whether widening them would pass, which is deliberate: that is the proposed
milestone's first job and doing it here would have turned a sweep into a fix.

## BUGS

- It is a snapshot, and the thing it measures moves. Every finding is against the tree at
  commit `c4854083`, on 2026-08-27. Findings 2 through 5 in particular are one-line edits that
  somebody may land the same week, and this note will not know.
- "Complete" here means three, and three is today's number. Every judgment in this note assumes
  the supported set is aarch64, riscv64, and x86_64, which is what §19 says today. A fourth
  architecture invalidates the file-set counts and the `not(target_arch = "x86_64")` hazard becomes
  ten live bugs rather than a note.
- **Severity is not ranked.** The table orders findings by where they live, not by what they cost.
  Finding 8 (an unaudited 6,797-line arch tree in the trusted computing base) and finding 7 (a
  stale `EXAMPLES` block) sit in the same list and are not the same size of problem.
- The three classifications are a judgment and two of them are contestable. In particular,
  findings 9 and 10 could be argued as legitimately architecture-specific on the ground that x86 is
  TSO and needs no machine barrier. This note calls them silent gaps because the compiler-reordering
  half is uncovered and because an empty function body records nothing either way, but a reader who
  disagrees is disagreeing with an argument rather than with a count.
- No gate enforces any of this. The sweep is a one-time read, like `notes/arch-audit.md` and
  `notes/untracked-work-sweep.md` before it, and it will go stale the same way both of those did.
  That is the argument for option A rather than for a longer note.
