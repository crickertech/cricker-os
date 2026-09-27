---
status: PARTIAL
raised: 2026-09-25
promoted_from: the-x86-64-progenitor-serves-entropy-from-rdseed
milestone_dependencies: 205, 206
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 595. The shell runs a `std` program, and `rg pattern` works at the prompt

Minted 2026-09-25 by the maintainer's lane `maintainer/shell-runs-std`, from
the gap #1314 recorded in `notes/foreign-program-arguments.md`'s `BUGS` section. *(Number and title
provisional: the integrator mints the number at merge, and the title is a draft until an architect
names it.)* The progenitor's `std` layout was built on 2026-09-26 by lane
`milestone/595-std-layout`. Its x86_64 half was promoted from the proposal
`the-x86-64-progenitor-serves-entropy-from-rdseed` and built the same day by lane
`milestone/595-x86-std`. See "What is built" below.

No open fork stops a step between a typed line and a running `rg` any more. §219, §170 (how a
foreign program is told what to do) and §171 (where a program image starts, and where the stack
goes) were ruled on 2026-09-26. Milestones 205 and 206 build the last two. "What is left" says
which step waits on which lane.

## The gap, checked 2026-09-25

The shell cannot launch a `std` program at all. `swish` resolves a typed name through
`grant_plan::Prog` and sends its wire id to the progenitor, which indexes a table it filled at boot.
That enum has 16 variants, ids 0 to 15, and every one is a native program. No `std` binary has a row.

`ripgrep` does run on nife, on all three architectures, but only from the kernel test harness.
`kernel/src/user/ripgrep_tests.rs` spawns it through `fs_service::start_std_full`, which builds the
address space `std`'s runtime expects. The progenitor has no equivalent. Its loader,
`supervision_protocol::build_child`, has never produced a child in that layout. The layout is the
eight slots `patches/std-nife/overlay/std/src/sys/pal/nife/rt.rs` fixes, plus the file-service page
at `0x1100_0000`. The harness also maps 32 stack pages where a native child gets 12.

Nothing in `design/roadmap/` owns this. Milestone 121 (`ripgrep` on nife) owes a confined
demonstration, but its outstanding items are written against the harness. Milestone 198 (a package
manager) owes running an installed program, which is a different program arriving by a different
route.

## The goal

From the prompt, `rg needle docs` runs confined to the directories the line granted and prints its
matches. A path outside the grant is not found, because nothing names it. This holds on aarch64,
riscv64 and x86_64 per DECISIONS §19 (architectural parity is a tenet), or a scope note says which
architecture is missing and why.

## What is built (2026-09-26, lane `milestone/595-std-layout`)

The progenitor builds a `std` child, and `std_exerciser` runs from the prompt. Proven by two
`script/swish-check` lines on aarch64 and riscv64, through the real `crates/system_initializer`
rather than the kernel's harness:

- `caps std_exerciser` prints the slots a `std` child gets, which are not a native child's. The
  heap's budget is at 0, stdout at 1, the clock at 5, entropy at 6, the configuration page at 7.
- `std_exerciser` prints its offline transcript. Each asserted phrase is a slot or a page landing
  where `std` reads it: the heap at 0 is `vec sum`, the empty slots 4 and 2 are the two `honestly
  unsupported` lines, and so on down to `config seeded`. When one is missing the program panics,
  and the gate fails on the fault. The line's comment in `xtask/src/swish_check.rs` maps each
  phrase to its slot.

What it took:

- `crates/std_runtime_protocol` (provisional name): the eight fixed slots, the three shared
  pages' addresses and the 32 stack pages, in one crate. It is generated verbatim into the PAL as
  `runtimeproto`, which `rt.rs` now re-exports, and the kernel harness reads it too. Before this
  the numbers were written three times; the progenitor would have been a fourth.
- `grant_plan::Manifest::runtime` (provisional), `Runtime::Native` or `Runtime::Std`, and
  `Prog::StdExerciser` at wire id 16. The field is where a package's manifest will say the same
  thing under §219's option D. The enum variant is an archive program named the old way, and it
  decides nothing about D.
- `system_initializer::StdLayout` (provisional): the same decisions `spawn_service` already
  makes (which output, which directory, which of the manifest's pages), placed at `std`'s slots.
  A directory grant goes to slot 4 with the file page at `0x1100_0000`, though no manifest asks
  for one yet (see BUGS).
- One region per `std` job, which is also its heap: `grant_plan::STD_REGION_PAGES`, 384. A heap
  split off the job's region would stop `job_undertaker` reclaiming it (`SPLIT` pins a region
  until its children are destroyed), and a heap beside it would be a region nothing reclaims. The
  job pool grew by one such region.
- `caps` prints `std`'s slots for a `std` program. A `grant_plan` test,
  `a_std_program_declares_only_what_the_std_layout_can_hold`, refuses a `std` manifest that asks
  for anything the progenitor would plan and not deliver: a file, an input, `--mem`, a domain, a
  second stream or the network.

## What is built on x86_64 (2026-09-26, lane `milestone/595-x86-std`)

`std_exerciser` runs from the x86_64 prompt, and so do the four `uuid` lines that leg had omitted
since milestone 182 (x86_64's own interactive-boot entry point). `script/swish-check --arch x86_64` now types 81 of 85 lines on its first boot; the four it
omits need a NIC (two network runs, two package fetches). Green on patagonia under OVMF three
times, the last after rebasing onto #1318's merge (442 s for the first boot).

The gap was entropy. The progenitor built its entropy service only from a virtio-rng on a
virtio-mmio slot, and `q35` has no mmio bus.

### Who confirms `RDSEED` exists: the kernel

That was the proposal's open question, and it was
decided as a reversible choice, for three reasons that do not depend on effort:

- The tree already does it this way. `entropy_service::is_instruction_backend_available` reads the
  feature bit from `arch::isa`'s boot record, `components/src/entropy.rs` says it trusts its
  spawner's choice of mode, and the installer (`install_service`) already takes the instruction
  service on the booted x86_64 path.
- It is the only answer with an aarch64 twin. `ID_AA64ISAR0_EL1` is not readable at EL0, so a
  progenitor that ran `CPUID` itself would be an x86_64 special case (DECISIONS §19).
- Detection stays in `kernel/src/arch/`, and `crates/system_initializer` gains no
  `cfg(target_arch)`.

### How it reaches the progenitor: a built service, not a flag

`kernel::user::boot_instruction_entropy`
(provisional) calls `entropy_service::ensure(Bus::Instruction)` when no virtio-rng was granted,
reads the readiness report before the progenitor starts, and grants the request endpoint at slot 16,
`BootEndowment::entropy_ep` (provisional). This is the file service's shape (`fs_ep`, slot 5). The
three `START` words are spent, and a bit in a page would be a second format for one fact. No new
syscall, method or object type. The progenitor probes slot 16 before its first retype, because
after that a first-free object could sit there.

### A machine without the instruction refuses loudly

Falsified once with `NIFE_CPU='max,-rdseed'`.
The kernel printed `entropy : NONE. No virtio-rng device, and this CPU has no seed instruction
(RDSEED, RNDRRS), so nothing at the prompt can draw random bytes and there is no login.` Then the
gate's new entropy-source check failed on that sentence, and `std_exerciser` died with `std::random`'s
own refusal. A first draw of all zeros gets a `REFUSED` sentence and no grant. There is no fallback to
`RDRAND` or to software.

The gate now reads the source on every leg: swish-check asserts the progenitor's
`entropy service up` sentence, naming a virtio-rng on aarch64 and riscv64 and the seed instruction
on x86_64, with the kernel's refusal as the negative.

### The machines

QEMU's `-cpu max` (both x86_64 runners' default) implements `RDSEED`; the kernel's
tour prints `rdseed supported (cpuid leaf 7 ebx.18)`. No `-cpu` change was needed. xenon's
i5-7500T is Kaby Lake, and `RDSEED` arrived with Broadwell, so it has one (from Intel's
documentation, recalled, not read on xenon today); `design/fatal-risks/the-confined-driver.md`
already records xenon's entropy source as `rdseed`. A xenon boot is not part of this proof.

### What switching on the login stack changed

`have_login_stack` gates on entropy, so x86_64 now
builds `credentialer`, `identity_provisioner`, `login` and the audit receiver at boot, for the
first time. What the gate sees: one more line before the prompt, `progenitor: login credentials
provisioned -- identity 'operator' password '...'`. The prompt is not behind the login, on any leg,
so no typed line changed. No user thread was killed after the hand-over. The capability-slot peak
at the hand-over report is 22 of 24, read by a temporary instrument (`capability::highest_seen`
printed beside the report, not committed), against 17 of 24 before. That is aarch64 and riscv64's
figure, which is what the proposal predicted. The x86_64 gauge line is still stale for milestone
182's reason, so this number is not re-read by anything.

## What it waits on

Every fork is ruled. What remains is built by other lanes, or edits code they are rewriting.

- §219 (how the shell names an installed program to the spawner): option D with gate D2, built in
  milestone 198 (a package manager) by #1320 and #1334. The shell sends a binary's bytes as frames,
  which fits an `rg` built by `helpers/build-ripgrep.sh` and run by path. `rg` cannot be an archive
  row, because fetching its crates in CI is a §46 (thin primitives or whole subsystems) decision
  nobody has made. So it runs unvouched, under D2, holding only what the line delegates plus the
  clock and configuration pages. That is the confinement this milestone wants.
- Milestone 205 (how a foreign program is told what to do), in #1385, builds §170's ruling: a
  byte argv in one page, and what a word may touch from the manifest and the line's directories.
  Its list names `Prog::StdExerciser`'s manifest and the progenitor's `std_layout` arm, so the
  directory half of `StdLayout` is proven there. The image arm's refusal of a directory grant
  (`wiring.dir` in `spawn_service`) is the same designation question for an unvouched program.
- The manifest note (#1338, promoted from the proposal
  `a-program-carries-its-manifest-in-an-elf-note`, unmerged when this was written). Its note
  carries `runtime`, which is how the progenitor can learn that a file run by its path is a `std`
  program. Until then an image always gets a native layout, and `grant_plan::image_can_carry`
  refuses `Runtime::Std` on purpose. Its number belongs in `milestone_dependencies` once the
  block exists on `main`, where lint looks for it.
- Milestone 206 (a program image has under 896 KiB), in #1352, builds §171's option D. Corrected
  2026-09-26: this said a spawn from the shell cannot use the relink to `0x100_0000` that gets
  `rg` past the ceiling in the harness. It can. `supervision_protocol::build_child_space` maps each
  segment where its program header says, and nothing a `std` child holds sits below
  `std_runtime_protocol::FS_PAGE` at `0x1100_0000`. 206 is still owed for a program linked the
  stock way.

## What is left, checked 2026-09-26

Two steps are this milestone's own. Both are sequenced behind the lanes above rather than built
beside them.

1. An image that declares the `std` runtime runs in `std`'s layout. The progenitor splits an
   image's region at a native job's size (`JOB_REGION_PAGES`) before the bytes arrive, and never
   consults `manifest.runtime`. The choice must come from the note, before `receive_image`.
   `StdLayout` needs no change. `std_exerciser` can prove it under today's cap: after
   `llvm-strip --strip-all` it is 204 KiB on aarch64, 133 on riscv64 and 140 on x86_64 (release
   builds, measured 2026-09-26). Waits on #1338.
2. An image as large as `rg`, about 2.6 MiB against `spawnproto::IMAGE_MAX_PAGES`, 64 pages.
   Raising it is not one constant. The image is held three times while a child is built: the
   shell's staging frames, the progenitor's own copy (§219 hashes that), and the child's pages. The
   progenitor maps the caller's frames through its scratch window, which milestone 604 (the
   builder's scratch cursor is bounded) made wrap; its compile-time check will fail the build if
   the pool outgrows half the window. `IMAGE_VA` and `IMAGE_STAGING_VA` are fixed
   windows milestone 206 is moving onto its map, and the second ends where the scratch window
   starts. The jobs pool, `JOBS_BUDGET_PAGES` at 624 pages, cannot hold one `rg`. Waits on
   milestone 206 and #1338.

An unvouched `std` program starts without entropy. `hashmap_random_keys` falls back to a
counter-seeded stream when slot 6 is empty (`patches/std-nife/overlay/std/src/sys/random/nife.rs`),
so a `HashMap` works and only `std::random` refuses.

## What it unblocks

- Milestone 121's three outstanding items: the confined demonstration, the loud refusal of a
  directory lacking `ENUMERATE`, and the benchmark that prices the walk. Each becomes a typed line
  rather than a harness call.
- Milestone 123 (the demonstration: somebody else's software, running narrow). Its first element is
  a ported tool run here and on Linux over one corpus. A tool a person cannot type is a harness
  result, not a demonstration.

## Which fatal risks it serves

Risk 1 (only software written for nife runs on nife) is green in the harness and not at the prompt.
A stranger does not run the kernel test suite, so for them the risk is still open until this lands.
Risk 7 (the confinement claim is false) gets its most legible test: a search that cannot see outside
its grant, run by the person making the claim.

## The gate that proves it

A boot test, `shell_runs_std_tests.rs` (provisional name), drives a scripted shell the way
`pipeline_tests.rs` does. It has two halves, because `rg` is absent from CI.

- In every build, on all three architectures: the shell spawns an in-tree `std` program with a
  string argument and a directory grant. The program prints both back, and the transcript is
  asserted byte for byte.
- When `rg` is in the archive: `rg needle docs` prints the expected matches. `rg needle ..` with only
  `docs` granted finds nothing outside it. A grant without `ENUMERATE` is refused with `EPERM`,
  never an empty listing. It skips with the same reason `ripgrep_tests.rs` gives when `rg` is
  absent.

## BUGS

- The hardware bytes are served as the instruction returns them: unmixed, and not health-tested
  after the first draw. That is DECISIONS §137 (a hardware TRNG with no published health-test claim)'s option A, which every other backend here already
  does; B or C is calef's (§137 is `PROPOSED`). Until it is decided nothing may claim
  cryptographic-quality randomness on x86_64 either.
- aarch64 takes the same path on a CPU with `FEAT_RNG` and no virtio-rng, since the function is
  arch-neutral. That arm has not run at the prompt. Every aarch64 swish-check boot has a
  virtio-rng, and the default TCG CPU (`cortex-a72`) has no `RNDRRS`.
- On a boot where the installer ran first, `entropy_service::ensure` has already handed it the
  readiness report and the kernel grants the service unread. The installer does not check the
  verdict either, so a condemned source would reach the progenitor; it answers `NO_ENTROPY` to
  every request, the password draw fails, and no login stack is built. Loud enough to notice, not
  a sentence.
- A `std` child holds `WRITE` on the region it is built in, because that region is its heap. A
  program that `SPLIT`s it pins it, and `job_undertaker` can then never reclaim it: one region
  lost from the pool until reboot. nife's `std` never splits (its allocator only `MAP`s), so this
  takes a program written to do it. Closing it needs a right that allows `MAP` and not `SPLIT`,
  which is a syscall-surface question.
- The directory half of the layout is built and unproven at the prompt. No `std` manifest
  declares a directory. §170 ruled the designation half on 2026-09-26, and the manifest that would
  declare it travels in an ELF note (§197 (a package is one archive file), M2), which is unbuilt.
- The network half is not wired. Slots 2 and 3 exist in the contract; the progenitor does not
  mint the socket frames' budget slot 3 needs, so a `std` manifest may not declare the network yet.
- `STD_REGION_PAGES` is the harness's number, not a measurement. 256 pages of heap is what
  every `std` program here has been proven under; `rg` over a real tree needs more, and a budget a
  person sizes at the prompt is an argument, which milestone 205 carries.

- The CI half proves the mechanism with a program this project wrote. Only the second half answers
  risk 1, and it runs only on a machine that built `rg`.
- Designation is ruled, not built. Under §170, `docs` becomes a capability because the line
  grants it as a directory or because `rg`'s manifest says a resolved word may be one, never
  because the shell guessed. An unvouched `rg`'s own manifest grants nothing, so a named file
  reaches it read-only and a directory reaches it only as the line's grant.

## Follow-on

- **Done.** x86_64's missing entropy service, promoted from
  `design/roadmap/proposals/the-x86-64-progenitor-serves-entropy-from-rdseed.md` into this
  milestone. See "What is built on x86_64".
- **Recorded.** A `std` child's `WRITE` on its own region, the unexercised directory half and the
  unwired network half are in `crates/system_initializer/src/lib.rs`, in `StdLayout`'s BUGS.
- **Outstanding.** Everything past the layout. A `std` program the shell names by §219's option D,
  its arguments (§170), and an image over 896 KiB (§171 and milestone 206) are what `rg` needs.
  Rechecked 2026-09-26 at the §170 and §171 rulings: §170 is decided and milestone 205 builds
  it, §171 is decided and milestone 206 builds it, and option D and gate D2 are built.
  Rechecked again the same day by lane `milestone/595-shell-runs-std-program`: see "What is
  left", which also replaces this list's note on milestone 198's fixed installed manifest.


## Index row

The shell cannot launch a `std` program: all 16 programs `swish` can name are native, and `ripgrep`
runs only from the kernel test harness. This makes `rg needle docs` work at the prompt, confined to
the granted directories, on all three architectures. It builds on §219's option D for naming and
waits on milestone 205 for arguments (§170, decided) and milestone 206 (a program image has under
896 KiB) for image size. It unblocks milestone 121's remaining items and milestone 123's
demonstration.
