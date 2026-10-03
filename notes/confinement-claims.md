# What nife claims a confined component cannot do, and which of those claims is tested

Milestone 202, building `design/fatal-risks/README.md`'s risk 7. The enumeration is the first
deliverable and this note is it. What follows the table is what happened when each claim's test
was broken on purpose.

Risk 7 is *"the confinement claim is false."* The evidence against it is a set of tests this
project wrote about attacks this project chose, and a passing confinement test is consistent
with two very different worlds: the component was stopped, or it never reached the address and
the assertion is decorative. Milestone 194 built the mechanism that tells those apart for a Kani
harness (`Falsification:`, `script/falsifications`, a recorded patch that must turn one harness
red). This milestone points it at the security claims.

Nothing here supports "the confinement holds." What it supports is narrower and is the
sentence to quote instead: *these named claims are tested, and each test has been shown to fail
when the claim is broken.* It cannot reach a claim nobody made, and that is where real escapes
live.

## The claims

Assembled from DECISIONS §14, §20, §31, §32, `notes/untrusted-input-audit.md`, and the tests
themselves. The last column is this milestone's result.

| # | The claim | Stated in | Tested by | Falsified |
|---|---|---|---|---|
| 1 | A component cannot widen its own rights: a derive holds no more than its source | §14, `crates/capability` | `capability::derive_never_widens_rights` | milestone 194 |
| 2 | Userspace cannot forge a right out of a syscall register | §14 | `capability::from_bits_cannot_forge_a_right` | milestone 194 |
| 3 | A budget that cannot delegate cannot split itself a child that can | §16, `Cap::mint_child` | `capability::split_never_widens_rights` | **yes** |
| 4 | A consumed capability cannot be used again | §12 | `capability::a_deleted_capability_stays_deleted` | **yes** |
| 5 | Dropping one capability does not disturb the others | §12 | `capability::delete_touches_only_its_slot` | **yes** |
| 6 | A supervisor cannot collect a corpse it does not supervise | §32 | `capability::reap_is_permitted_only_to_the_supervising_rendezvous` | **yes** |
| 7 | A refusal about a stranger's thread discloses nothing about it | §32 | `capability::a_stranger_reveals_nothing_about_its_liveness` | **yes** |
| 8 | A process view shows exactly the rendezvous's own children | milestone 126 | `capability::a_survey_shows_exactly_the_endpoints_own_children` | **yes** |
| 9 | What a supervisor may see and what it may reap are one domain | milestone 126 | `capability::the_view_and_the_reap_have_the_same_scope` | **yes** |
| 10 | A user virtual address is in the low half and page-aligned, on every ISA | §19 (architectural parity is a tenet) | `paging::{aarch64,sv39,x86_64}::the_user_va_gate_admits_only_the_aligned_low_half` | **yes, three times** |
| 11 | No page is both writable and executable | §19 | `paging::x86_64::no_encoded_leaf_is_both_writable_and_executable` | **yes** |
| 12 | An IOMMU entry sets no bit the hardware treats as reserved | §20 | `paging::x86_64::no_vtd_entry_ever_sets_a_reserved_bit` | **yes, since 2026-09-16, and see below** |
| 13 | A device cannot touch memory outside its driver's granted region | §20 (IOMMU-backed DMA isolation) | `direct_memory_access_validator::in_region_is_sound`, `an_accepted_descriptor_is_confined`, `validate_and_shadow_confines_every_chain` | **yes, three** |
| 14 | A driver cannot send its device to descriptors nothing validated | §20 | `direct_memory_access_validator::an_accepted_descriptor_is_confined` (the indirect refusal) | **yes** |
| 15 | A driver cannot make the validator walk outside the rings, or forever | §20 | `direct_memory_access_validator::the_outer_walk_stays_inside_the_rings_and_terminates`, `an_oversized_batch_is_refused` | **yes, two** |
| 16 | One queue's validation cannot touch another queue's rings | §20 | `direct_memory_access_validator::distinct_queues_occupy_disjoint_blocks` | **yes** |
| 17 | A descriptor changed after validation cannot reach the device | §20 | `direct_memory_access_validator::a_descriptor_mutated_after_validation_cannot_reach_the_device` | **no, and see below** |
| 18 | A wiring plan never grants a right the declaration did not ask for | §41 | `component_plan::a_plan_never_grants_a_right_the_declaration_did_not_ask_for` | **yes** |
| 19 | A directory capability reaches its subtree and nothing above it | §50 | `filesystem_protocol::attenuate_never_widens`, `a_grandchild_is_bounded_by_the_root`; `kernel::user::dir_capability_tests` | milestone 194 (the proofs) |
| 20 | A memory-unsafe C component faults on an out-of-bounds write and changes nothing outside its grant | §31 | `kernel::user::c_seam_tests::a_c_out_of_bounds_write_faults_and_changes_nothing_outside_its_grant` | **yes, by hand** |
| 21 | A user program cannot read a kernel address, on every ISA | §19 (architectural parity is a tenet) | `kernel::user::tests::a_user_program_cannot_read_a_kernel_address`, `the_hardware_says_el0_cannot_read_the_kernels_memory`, `riscv_virtio_tests::the_page_tables_say_u_mode_cannot_read_the_kernels_memory` | **yes: one record on aarch64 and x86_64, and riscv64's is the software walk** |
| 22 | An ELF cannot ask to be loaded over the kernel, or for a writable executable page | §15 | `kernel::user::tests::an_elf_that_asks_to_be_loaded_over_the_kernel_is_refused`, `..._for_a_writable_executable_page_is_refused` | **yes, two** |
| 23 | The progenitor cannot rebuild after dropping its construction authority | §26 | `kernel::user::authority_tests::init_drops_its_construction_authority_and_cannot_build_again` | **yes, and see below** |
| 24 | Two shells with different roots cannot name each other's files | §50 | `kernel::user::shell_navigation_tests::two_shells_with_different_roots_cannot_name_each_others_files` | **yes, and see below** |
| 25 | A client cannot reach its neighbour's pixels or read the screen | §66 | `kernel::user::compositor_tests::a_client_holds_no_capability_for_its_neighbours_pixels_or_the_screen` | **yes, one of its four** |
| 26 | A client of a rendezvous cannot become its server | §41 | `kernel::user::live_swap_tests::a_client_of_the_stable_rendezvous_cannot_become_its_server` | **no, and see below** |
| 27 | A thread holding no port capability cannot touch a port, and a holder's ports do not leak across a context switch (`x86_64`) | §121, milestone 299 | `kernel::user::x86_port_tests::port_holder_transmits_then_a_non_holder_faults` | **yes, milestone 313, and see below** |
| 28 | A revoked port holder faults on its next `in`/`out` (`x86_64`) | §121, milestone 299 | `kernel::user::x86_port_tests::a_revoked_holder_faults_on_its_next_port_write` | **yes, milestone 313** |
| 29 | A thread that deletes its own port capability faults on its next `in`/`out` (`x86_64`) | §12, milestone 313 | `kernel::user::x86_port_tests::a_holder_that_deletes_its_port_capability_faults_on_its_next_port_write` | **yes, milestone 313, and it was false in the tree** |
| 30 | A revocation reaches a capability in flight, not only the ones sitting in capability tables | Nowhere until 2026-09-21; now `sched::delete_page_frame_caps_where` | `kernel::user::revocation_in_flight_tests::a_capability_revoked_while_it_is_in_flight_does_not_reach_the_receiver` | yes, 2026-09-21, and it was false in the tree |

## Five claims that are stated nowhere, which is what step 1 was for

A confined component's *timing* is not confined.
Added 2026-09-02 with DECISIONS 139 (how a saturated workload is made to hand threads across
cores is a different section; this is 139, who may read the cycle counter and by what authority).
The words `timing`, `side channel` and `covert` appeared zero times in this note, in
`DECISIONS.md` and in `design/fatal-risks/README.md` before that decision, so nothing here was falsified
by it; the absence was the finding. seL4 states its own position in one clause, that exporting the
PMU to user level "opens the possibility of timing channels", and this tree intends to publish
cycle-denominated numbers against seL4's while saying nothing.

The reason it cannot be claimed is measured rather than assumed. Two threads and a shared word
reconstruct a fine clock with no privileged instruction: 6.8 ns of usable resolution on cordoba
under load, matching Schwarz et al. (FC 2017). That holds on all three architectures, so gating a
cycle counter cannot deliver timing isolation on any of them. What the grant in DECISIONS 139
buys is accountable authority: the cheap accurate path is granted rather than ambient, and the
kernel knows which threads hold it. It belongs in this section so nobody reads the capability rows
as covering timing.


A confined device's *values* are not confined, only its *reach*.
`notes/untrusted-input-audit.md` says this in its own words ("the IOMMU confines placement, not
values") and no test asserts it, because it is a limit rather than a guarantee. It belongs in
this table as a claim the system does not make, so that nobody reads row 13 as covering it.
The audit's finding 1 is the live consequence: the NVMe driver panics on a device-written index
it does not check, which the IOMMU cannot prevent and does not claim to.

A `SURVEY` cursor counts threads the viewer cannot name.
`kernel::user::survey_tests::the_survey_cursor_counts_threads_the_viewer_cannot_name` is a test
that *states a limit*, found by a 2026-08-17 audit. Row 8's claim is about which threads are
shown; nothing claims the count is confined, and one is not.

A confined component's *interrupt target* is not confined, and nothing here has ever asked.
Added 2026-09-03 by DECISIONS §86's research pass, which went looking for what a userspace NVMe
driver would need and found this underneath every option it was pricing. An MSI or MSI-X message is
a memory write to an architecturally special address, so an IOMMU doing DMA remapping alone does not
confine it: a component that can write a device's MSI-X table can aim an interrupt at a vector it
was never given. Linux refuses to hand a device to an untrusted userspace driver on a machine
without interrupt remapping for this reason, and names its escape hatch `allow_unsafe_interrupts`.

The absence is the finding, and it is three-deep. Row 12 and row 13 are about where a device may
*read and write*, and neither covers where it may *interrupt*. When this was written, no boot this
tree runs could exercise the question even if a claim existed: `helpers/qemu-runner-x86_64.sh`
attached `-device intel-iommu` with no `intremap=on`, and `helpers/qemu-runner-aarch64.sh` used
`gic-version=2`, which has no ITS. And no driver touches an MSI-X table (notes/non-volatile-memory-express.md's `BUGS`: the
NVMe controller is brought up with `IEN=0` and no MSI-X table is touched), so nothing has ever come
near it.

**The first of those three was false, and the machine is what said so** (milestone 317,
2026-09-17). The x86_64 sentence above is a true reading of the runner script and a wrong
conclusion, reached without booting it. QEMU's `intremap` property is tri-state and defaults to
`auto`, which resolves ON when there is no in-kernel irqchip, so on patagonia `ECAP.IR` reads
set on the default machine (`ECAP = 0xf00f4a`) and clear only under an explicit `intremap=off`
(`0xf42`). Interrupt remapping has been offered in every x86_64 boot this tree has ever run,
and nothing read the bit, so nobody noticed. The guest now reports it,
`NIFE_INTREMAP=off` is the flag that reaches the machine without it, and one test asserts
`GSTS.IRES` stays clear whatever `ECAP.IR` says. `design/decisions/86-el0-nvme-driver.md` still
carries the uncorrected sentence; a lane may not edit that file, so it is flagged rather than
fixed.

The second is measured, and it holds. aarch64 is not one flag away: `gic-version=3` gives
QEMU's `virt` an ITS, and it also
moves `reg[1]` from the CPU interface to the redistributor, which `memory::init`'s `intc@`
name-prefix match hands to a GICv2 driver without ever reading `compatible`. The measured result is
a boot that says `GICv2` while printing a redistributor base, with zero timer ticks. That is
milestone 227's bill; see design/roadmap/317-interrupt-remapping-flags.md.

And there is a third architecture the paragraph above never mentions, which is the sharpest part.
riscv64 was not surveyed, and it is the one where the mechanism is closest to hand: the RISC-V IOMMU
puts MSI confinement *inside the device context this kernel already writes on every attach*. Measured
from a boot, `CAPS = 0x78c2cf4f10` has `MSI_FLAT` set, so every context is the extended 64-byte
format, and `attach` writes all four MSI words zero (`msiptp.MODE = Off`). No flag is needed there
and none is available. It is also the only one of the three that can never get a second witness: no
silicon ships the ratified RISC-V IOMMU (milestone 143), which inverts x86_64, where xenon is
waiting.

So MSI confinement lives in three different places and none of the three is exercised: a separate
IOMMU feature on x86_64, a separate device (the GICv3 ITS) on aarch64, one mode field on riscv64.
Each machine description now reports its own position, which is what makes this checkable at all.

The claim itself stays stated nowhere. Nothing writes an `IRTE`, nothing programs an MSI page
table, and nothing forges an MSI to see where it lands.

It is latent rather than false, and it stays latent exactly as long as every component that can
reach a BAR is the kernel. It goes live the first time a driver leaves the kernel and wants
interrupts instead of polling, which is what §86 decides. Whatever §86 settles has to say who owns
the page holding the MSI-X table; the cheap first move was two runner flags, and milestone 317 took
it. x86_64's boot path was already running with the hardware present, so what the flag buys there is
the machine *without* it; aarch64's does not reach the question at all. (`kernel-irqchip=split`
turned out not to be needed on patagonia: it is a KVM constraint and there is no KVM here. 317's
block has the three invocations.) The hazard is the one milestone
202 (every confinement test is a ritual until somebody breaks the confinement) already caught in
§31's headline assertion: green after turning the flags on proves nothing by itself, and the
falsification has to be a driver aiming an interrupt where it was not given one, coming back red.

The progenitor's bytes are unsigned.
§14 says so plainly in its own honest caveat and it is not in the table because it is not a
confinement claim; it is the reason the confinement has an unverified component inside it.

The kernel cannot execute a confined component's code.
Added 2026-09-17 by milestone 313's audit, which found it stated nowhere and true on one
architecture only by accident. No row above says it and no test asks. On aarch64 it is `PXN`, which
the encoder sets on every user page; on riscv64 it is the hardware, which refuses a supervisor
fetch from a `U` page unconditionally; on `x86_64` it is `CR4.SMEP`, which nothing in this kernel
set until that audit, so a user code page with `XD` clear was executable at ring 0, whatever
`crates/paging`'s decoder reported. It is not a userspace escape on its own (a kernel control-flow
bug has to come first), which is why it belongs in this section rather than in the table: it is
the thing that turns any such bug into a full one. `arch::x86_64::init` now sets the bit on every
core whose CPUID offers it and prints a line either way. There is still no test, because a
falsification would need ring 0 to survive its own page fault, and `SMAP` (its sibling, per-access
and not free) stays off with the reason `mmu::permit_kernel_access_to_user_pages` records; both are
milestone 424 (design/roadmap/424-a-ring-0-that-provably-cannot-execute-ring-3-pages.md).

## What breaking them found

Twenty-five Kani harnesses now carry a recorded patch that turns them red, up from six.
`script/falsifications --sweep` runs all twenty-five in about 30 seconds and every one goes red.
Three results are worth more than the count.

**Milestone 305 added the kernel half** (2026-09-16), which milestone 202 could not: ten kernel
`#[test_case]`s now carry a record and `--sweep` replays each by booting one architecture. Its own
results are in the section after this one. Read them first if you only read one: the headline is a
confinement test that stayed green under a patch that broke the thing it claims, and had been
unable to fail since milestone 41.

### §31's headline sentence is not what catches a broken confinement

Row 20 is the roadmap's own worked example: map `WITNESS_RO` read/write into the C component,
rebuild, run, and the test must go red. It does. It does not go red on the assertion anybody
would name.

The obvious answer is the verdict equality, `assert_eq!(v[2], CONFINED, ...)`, which prints all
four bits including `read-only witness intact`. That assertion never runs. A component that is
not confined does not fault; a component that does not fault produces no death report; and
`run_seam` collects every report before the test inspects any of them, so the run stalls at the
collection. The witness check, which is the sentence §31 leads with, is reached only by an
escape that faults anyway.

**And the first run of it failed for the wrong reason**, which is the hazard milestone 202's
block names. `run_seam`'s blocking receive had nothing to take, so the break surfaced as a
watchdog timeout at 234 seconds reading `a livelock, not a lost wakeup`. Right answer, useless
diagnostic: nothing in it says the word confinement. `wait_for_report` (provisional name) now
bounds that wait at 30 seconds against a 90-second budget, and the second run fails at report 4
of 12 with a sentence about what a missing death report means.

### A proof can be blind to the predicate it is stated in, twice

`component_plan::a_plan_never_grants_a_right_the_declaration_did_not_ask_for` asserted
`p.caps()[i].1 == reqs.caps[i].direction.rights()`, which is stated *through* `rights()`, so a
`rights()` that adds `GRANT` to everything satisfies it. Only the explicit
`& abi::rights::GRANT == 0` beside it caught the defect. That is the same shape milestone 194
measured in `capability::derive_never_widens_rights`, one crate over, and it was the argument for
keeping an assertion that looks redundant.

**Milestone 307 found that this paragraph had inverted, and the inversion is a better lesson than
the original.** Milestone 211 fixed the harness by writing the expected rights out in literals
(`Direction::Serve => READ`, `Direction::Use => WRITE`) instead of calling `rights()`. Since `READ`
is `1 << 0`, `WRITE` is `1 << 1` and `GRANT` is `1 << 2`, that equality now *implies* both lines
below it, so `& GRANT == 0`, the assertion this note credits as the only thing catching the
defect, became the one that could not fail. The rescue became the decoration, the note went on
describing code that had changed, and nothing gated the drift. The two implied lines are removed in
307 and the live assertion carries the sentence; the argument for keeping a redundant-looking
assertion survives, with the caveat that which one is redundant moves when the other is repaired.

### One claim has no falsification and the reason is structural

Row 17, the time-of-check/time-of-use property, stayed `unfalsified` on purpose. It holds because
the driver's descriptor table and the shadow are two disjoint arrays in the harness's memory model,
and no line of `direct_memory_access_validator` can make them one. Aiming the copy back at the
driver's table does turn the harness red, but through `ChainMem::write64`'s address arithmetic
rather than through the post-mutation assertion, which is a red for the wrong reason and so is not
recorded as evidence. The harness proves a property of the *design* rather than of code that could
regress, and its honest denominator is that state and not a patch.

## What breaking the kernel tests found (milestone 305)

Rows 21 to 26 are six claims and nine tests, and a falsification is per test. Eight of the nine
now carry a recorded patch; the ninth is row 26 and it cannot carry one. Five results are worth
more than that count.

### A confinement test that could not fail, on RISC-V, since milestone 41

Row 21's RISC-V twin, `the_page_tables_say_u_mode_cannot_read_the_kernels_memory`, is the one place
this milestone found the thing it was looking for. The patch removes the `U`-bit check from
`mmu::user_can_read` outright, which is as complete a break of that test's stated property as can be
written, and the first sweep reported SURVIVOR: the test ran, and passed.

The reason was not a weak defect. `user_can_read` walked through `translate_user`, and
`translate_at` builds its `Mapper` with `Half::Low`, always, so the high-half kernel address the
test asks about came back `None` before any leaf was read. The headline assertion, `assert!(
!mmu::user_can_read(kernel_va), "the page tables say U-mode could read the kernel's own memory")`,
answered "no" by refusing to look. It could not fail, and the test's own doc comment calls that walk
"the thing under test" and calls the `U` bit RISC-V's single line of defence.

The tree had already written the cause down and nothing connected it.
`is_mapped_in_current_space`, forty lines away in the same file, exists for exactly this case and
says so in its doc comment: *"a user thread reaching for the kernel's memory names a high-half
address ... `translate_user` alone would say 'not mapped' for it and turn the most interesting case
into the wrong answer."* `user_can_read` went on calling `translate_user`.

Milestone 305 fixed it (`translate_in_either_half`) and the patch is recorded against the fixed
function, so the row is evidence now rather than ritual. Two things follow. A green confinement
test is consistent with the assertion being unable to fail, which is this note's opening sentence
arriving from a direction nobody had checked. And the only instrument that could find it was a
falsification: every gate in this tree was green throughout, because a vacuous assertion is a
passing assertion.

### The §31 assertion-order hazard recurs, in row 24

Milestone 202 found that §31's leading sentence, the witness-page equality, is reached only by an
escape that faults anyway. Row 24 is the same shape in a different subsystem, and here it is
structural rather than incidental.

`two_shells_with_different_roots_cannot_name_each_others_files` states its property twice: once as
the per-shell bitmap equalities in `assert_report`, and once as the crossing, `assert_eq!((a &
nb::REACHED_SECRET, b & nb::REACHED_INNER), (0, 0), "a shell named a file in the other shell's
root")`. The second is the sentence the milestone makes and the one a reader would quote. It sits
below both per-shell checks, and it cannot run: any defect that causes a crossing sets a
forbidden bit in one of the reports, and `assert_report`'s first direction catches that one call
earlier. No patch tried in milestone 305 made the crossing fire, and none can.

The quotable sentence is documentation; the bitmap equalities are the mechanism. Here that costs
nothing, because `assert_report`'s messages name the offending or missing bit, so a reader learns
as much as the crossing would have told them. §31's instance cost a 234-second watchdog timeout
reading "livelock". Two instances found the same way promotes it from an anecdote about §31 to a
thing to look for: in a test that states its property twice, the readable statement is usually the
unreachable one.

And row 24's own record is weaker than the row looks, which the patch says where a reader meets
it. The recorded defect (the caretaker serving the filesystem root instead of its narrowed handle)
turns the test red through `assert_report`'s *second* direction, the vacuity guard: the shell could
no longer reach its own files, so every refusal it reported would have proved nothing. Nothing
crossed. It proves the test is wired to the real root handle; it does not demonstrate a crossing.

### Row 21's "on every ISA" is not evidenced the same way on every ISA

The aarch64 falsifications map the kernel's own memory EL0-readable, one flag at one call site, and
the tests watch the hardware refuse. That defect cannot be booted on RISC-V. `crates/paging`'s
Sv39 encoder turns `CAP_USER` into the `U` bit, and S-mode access to a `U` page faults unless
`sstatus.SUM` is set, which this kernel sets only inside a test helper. A kernel that marked its own
`.rodata` user-readable there would not be a kernel userspace can read; it would be a kernel that
cannot read itself, dead before the first test. So the RISC-V evidence is against the software walk
instead, and the row's three "yes"es are not three of the same thing. DECISIONS §19 makes parity a
gate for the capability; this is a gap in the evidence.

The `x86_64` leg had no evidence until milestone 323 (the falsification record is incomplete in five ways). The same one-flag defect works there:
`arch/x86_64/mmu.rs` maps `.rodata` with `Flags::kernel_rodata()`, and `user_rodata()` writes the
`U/S` bit while SMAP stays off. Replayed 2026-10-03 (UTC), red at `tests.rs:257` on both aarch64 and
`x86_64`. The record now names `Architecture: aarch64, x86_64`, with a hunk per ISA's file.
Read the row as aarch64 twice, `x86_64` once, riscv64 once (the software walk).

### Row 26 cannot be falsified as written, because a real escape hangs the run

`a_client_of_the_stable_rendezvous_cannot_become_its_server` asserts `attack[1] ==
-NotPermitted`, and the honest defect is the one that breaks the claim: delete the kernel's
`Rights::READ` check on `RECV_CAP`, so a client really can receive on the stable rendezvous. That
patch was written and run on 2026-09-16, and the result was a 60-second watchdog reading "no
progress ... a lost-wakeup hang", with a thread dump and not one word about impersonation.

The reason is structural. `RECV_CAP` is a blocking receive. An attacker the kernel fails to
refuse does not come back and report an escape; it takes the message the honest server was waiting
for, or parks on the rendezvous, and the run deadlocks. So the assertion that states the claim is
reachable only when the kernel *does* refuse, and the case it is written about cannot reach it.

**That is milestone 202's wrong-reason red, on a third claim, and a red for the wrong reason is not
evidence.** The row is `unfalsified` on purpose, the same disposition as row 17's, rather than being
filled with the easier defect that *does* fire the assertion: changing which error the refusal
returns makes `attack[1]` wrong while leaving the claim entirely intact, which would put a false
claim in the record whose whole job is saying what is known.

**What would close it** is the same move milestone 202 made for §31: give the attacker a bounded
wait so that "the server never answered me" is reported rather than waited out. That needs a
non-blocking or timed receive, which is the syscall surface, so it is a proposal rather than a fix.
See milestone 305's block.

### One row's falsification proves less than the row looks like it proves

Row 23 is falsified in the kernel rather than in the fixture, by making `CapabilityTable::delete`
not take the capability out of the slot, which is the right place because the claim is the kernel's.
But rows 4 and 5 are `capability::a_deleted_capability_stays_deleted` and
`capability::delete_touches_only_its_slot`, both already `replayable`, and both catch that same
defect. So the honest reading is narrower: it proves row 23's kernel test is wired to the kernel's
own delete, not that row 23's test is the only thing watching it. Rows 21, 24, 25 and 26 have no
such overlap.

## Which assertion actually fires (milestone 307)

Milestone 305 found two independent cases where the assertion a reader would quote is not the
assertion doing the work, and said a sweep would probably find more. This is that sweep, over all
26 rows. The question asked of every test and harness: when the claim is broken, which assertion
fires, and is the one a reader would quote reachable at all?

Three verdicts, and the first is not padding: saying plainly that most rows are exactly what they
look like is what makes the rest worth reading.

| Verdict | Rows | Count |
|---|---|---|
| Fires as advertised | 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 16, 18, 19, 21, 22, 23, 25 | 17 |
| The quotable assertion cannot run | 4, 13, 14, 15, 20, 24 | 6 |
| Answered by refusing to look | 12 | 1 |
| Deliberately `unfalsified`, so neither | 17, 26 | 2 |

Two of the seventeen carry an unreachable restatement *below* a headline that does fire, which is
the same shape doing less damage: row 18's `& GRANT == 0` and row 25's two
`rendezvous_waiting_senders` checks. They are counted where their headline is and described below.

Row-by-row notes on the seventeen, so a reader can tell which fact is which rather than inferring it
from a count: rows 1, 3, 5, 6, 8, 10, 11 and 16 state their property independently of the code under
test and each has a recorded patch that fires on the assertion its prose names. Row 2's
`from_bits_cannot_forge_a_right` is sound for the claim it makes and blind to the adjacent hazard its
own doc comment names, a wrong `Rights::ALL`, which drops rights rather than forging them and is a
different claim. Row 7's `kani::assume` narrows *to* the adversarial case rather than away from it.
Row 9 is stated through both functions it compares, which is the claim (that they agree) rather than
a defect. Row 16's two assumes plus `MAX_QUEUES = 2` admit exactly one pair, `(0, 1)`, so "any two
distinct queues" is one concrete case; its patch says so. Rows 19, 21, 22 and 23 are the kernel
tests, and 23's evidence is about the test rather than the kernel for the reason 305 recorded.

### Row 12's proof could not fail, and the tree had written down that it could

`paging::x86_64::no_vtd_entry_ever_sets_a_reserved_bit` is row 12's whole evidence: §20's claim that
an IOMMU entry sets no bit the hardware treats as reserved. It stated all three of its assertions,
and its `kani::assume`, through `VTD_ADDR_MASK`, `VTD_R` and `VTD_W`. Those are the three
constants `Vtd::leaf_entry` builds its result out of:

```rust
fn leaf_entry(pa: u64, flags: Flags) -> u64 { (pa & VTD_ADDR_MASK) | bits }   // bits ⊆ {VTD_R, VTD_W}
assert_eq!(leaf & !(VTD_ADDR_MASK | VTD_R | VTD_W), 0);
```

`(pa & M) | bits` sets no bit outside `M | VTD_R | VTD_W` for every value of M, and the assume
admitted exactly the addresses `M` allowed, so widening the mask moved the encoder, the input space
and the assertion in lockstep. The address half of the claim was a tautology.

Measured, and this is the part that makes it a finding rather than an argument. With
`VTD_ADDR_MASK` widened to bits 62:12, a range a VT-d second-level entry really does reserve
(patagonia, 2026-09-16, kani 0.67.0):

| Harness | Result |
|---|---|
| as it stood before 307 | SUCCESSFUL, 0 of 45 failed. A survivor. |
| as it stands after 307 | FAILED, 1 of 68 |
| after 307, honest tree | SUCCESSFUL, 0 of 68 |

The defect is not exotic. `VTD_ADDR_MASK`'s own doc comment says VT-d's real width is
`CAP_REG.MGAW`-defined and this driver has never narrowed to it, so the mask is a number somebody
could plausibly change. And the consequence is not cosmetic: QEMU's model and real silicon fault a
transaction over a reserved bit rather than ignoring it, so the failure mode is an IOMMU whose every
translation fails, presenting as broken hardware rather than as a bad table.

The tree had already recorded the opposite, in writing, forty lines up. The comment on
`the_leaf_keeps_address_and_permissions_apart` explains this exact trap correctly and then says
*"`no_vtd_entry_ever_sets_a_reserved_bit` in this crate already works this way; this is the same move
on the portable leaf."* It did not work that way. A lane that had just avoided the trap cited, as its
precedent, the one harness in the crate still caught in it. That comment is corrected rather than
deleted, because the citation is the interesting half.

Fixed here: the permitted bits are a literal (`VTD_PERMITTED_BITS`, `cfg`-gated to the test
configurations so no implementation can reach it and reintroduce the coupling), and the assume is
gone, because masking the address down is `leaf_entry`'s own job and a claim about what it does with
an arbitrary address may not assume the address is already in range. The host twin
`a_vtd_leaf_sets_no_bit_outside_read_write_and_address` was blind for a second, independent
reason on top of the first, and it is worth naming because a literal alone would not have fixed it:
its one concrete address `0x10_0000` has no bits above 51, so the encoder's masking was never
exercised and a wider mask changed nothing it could observe. It now runs three addresses and catches
the same defect in microseconds.

### The 305 shape recurs six more times, and it recurs in proofs as well as in kernel tests

Milestone 305 promoted "in a test that states its property twice, the readable statement is usually
the unreachable one" from an anecdote about §31 to a thing to look for. It looks for well. Eight rows
carry an assertion that states the claim in the claim's own vocabulary and cannot run:

- Row 13. `in_region_is_sound` ended with `assert!(addr >= base && end <= limit)`, carrying the
  comment *"no byte the device would touch lies outside the granted region"*, directly below the two
  assertions it is the conjunction of. Removed in 307; the sentence moved onto the live pair.
- Row 14. `an_accepted_descriptor_is_confined` kept `assert!(!d.is_indirect())` and
  `assert!(is_in_region(base, size, d.addr, d.buf_len()))` **below** the assertions of milestone 211
  (a harness that states its property through the function under test) that replaced them. Inside
  `if check_descriptor(..)` those are the same two calls with the same arguments the guard returns
  false on, so neither can fail. 211 added the working phrasing and left the blind one underneath
  holding both readable messages. Removed in 307.
- Row 18. The inversion described in the section above.
- Row 4. `a_deleted_capability_stays_deleted`'s `get`/`delete` re-use refusals sit below the
  storage check `assert!(cs.slots[slot].is_none())`, which catches the same defect one line earlier.
  Benign and already named in an in-code comment; left alone, because the accessors are the claim's
  vocabulary and the storage line is its mechanism, and here that costs nothing.
- Row 15. `an_oversized_batch_is_refused`'s single `assert!(!ok)` is never reached; the panicking
  closures are the mechanism. Its own patch says so. Left alone for the same reason.
- **Row 20.** §31's headline, and 307 sharpens what 305 recorded. `assert_eq!(v[2], CONFINED)` is not
  simply unreachable: it is reachable only through the two bits that are not the confinement
  claim. A broken `IN_GRANT_WRITE_LANDED` or `FAULT_ADDR_AS_EXPECTED` still faults, still produces
  a death report, still produces a verdict, and fires it. A broken `WITNESS_RO_INTACT` or
  `WITNESS_FAR_INTACT` means the store landed instead of faulting, so no death is reported and the
  run stalls at `wait_for_report`. The assertion that prints *"read-only witness intact"* can fire
  for everything except a broken witness.
- Row 24. 305 recorded the relative-path crossing as unreachable. It is the same for the
  absolute-path crossing twenty lines further down, and for all four `assert_ne!` lines beside the
  two: every one of them restates a bit `assert_report` has already checked in one direction or the
  other. Six assertions, all of them the readable half, none of them able to run.
- Row 25. The two `assert_eq!(sched::rendezvous_waiting_senders(..), 0)` lines, whose messages
  read *"the write did not fault"* and *"a client read a pixel of the screen it holds no mapping
  of"*, sit below a `wait_for` on the fault counter that catches exactly that defect two seconds
  earlier. The quotable assertion for row 25 is the fault wait itself, and that one fires.

### Two limits the sweep found that are not assertion order

An assertion can be live on one architecture and structurally dead on another, and row 21 is not
the only place. `a_read_only_segment_is_mapped_read_only` asserts
`!flags.is_kernel_executable()` on a user `.rodata` page. On aarch64 that is a live check: `PXN` is
a bit independent of `AP_USER`, and a kernel-executable user page is a real hazard the
`Flags::user_code` doc calls out by name. On riscv64 and x86_64 it cannot fail, because both
decoders reach `CAP_KERNEL_EXEC` only through an `else` branch that requires the user bit clear
(`sv39.rs` `leaf_flags`, `x86_64.rs` `leaf_flags`), and the assertion two lines up has already
established the page is user-accessible. ~~The decoders are faithful: on those two ISAs the hardware
really does make a user page non-executable in supervisor mode~~ **Half of that sentence was false,
and milestone 313's audit found it** (2026-09-17). On riscv64 the hardware does refuse a supervisor
fetch from a `U` page, unconditionally. On `x86_64` it does so only while `CR4.SMEP` is set, and
this kernel had never set it: `XD` is the one execute bit and applies at every ring, so the decoder
was reporting a user page as not kernel-executable on a machine where ring 0 could execute it. The
bit is set now (`arch::x86_64::init`, on every core whose CPUID offers it), which makes the decoder
true on the hardware rather than in principle; the encoder's own comment carries the correction. The
structural point survives with that caveat: the assertion still cannot fail on those two ISAs, and
what is wrong is reading one portable test as three ISAs' worth of evidence. Same distinction 305
drew for row 21: a gap in the evidence, not in the capability.

Row 19's attackers cannot tell a refusal from a probe that was never sent. `fs_test_client`'s
`dir_attacker` sets `REACHED_PARENT` and its siblings only on *success*, so a fixture that stopped
attempting the parent open would report a clean verdict and the test would pass. `OPENED_ITS_OWN`
and `GRANTED_ACCESS_FAILED` guard the other direction (a capability that reaches nothing is
trivially confined) and they do that job well; nothing guards this one. It is the note's own opening
sentence, one level out: a passing test is consistent with the component being stopped and with the
component never having asked. Recorded rather than fixed, because the fix is a per-probe
"attempted" bit in a wire-format bitmap two programs agree on.

### And where the three instances did not generalise, which is worth as much

The predicate class that produced 305's survivor was checked on every architecture and found sound
elsewhere. aarch64's `user_can_read` asks the silicon (`AT S1E0R`) and has no half to get wrong;
x86_64 has no such predicate at all. The DMA attackers' descriptors reach `is_in_region` rather than
being turned away by an earlier check (`check_descriptor` tries `is_indirect` first, and the direct
attacker's descriptor is not indirect; the indirect attacker's *is*, and its own comment says so).
The compositor test's vacuity guard, `neighbour_probe_phys(ATTACKER) == client[VICTIM] +
FRAME_SIZE`, compares two different allocation records and is a real fact about adjacency rather
than a restatement. `reap_tests::assert_can_only_supervise` walks every slot and checks both
directions. All of these fire as advertised.

## What attacking them found (risk 7's adversarial pass, 2026-09-21)

Every pass before this one read the claims and asked whether each was tested. This one took the
other posture `design/fatal-risks/README.md`'s risk 7 has asked for since it was written: assume a claim is
false and go looking for the case that makes it so. One claim was false in the tree, on a path any
two cooperating programs can take, and the tree had already written down the rule it broke. The
rest of this section is what was attacked and held, because a pass that reports only its hits is
indistinguishable from one that stopped early.

### A capability revoked while it is in flight was delivered anyway

The attack. Ask where a capability can live that is not a capability-table slot, because every
revocation sweep in the kernel walks tables. There is exactly one such place and it is not obscure:
`Thread::outgoing_cap`, the hand-off slot `sched::ipc_send_cap` writes when a `SEND_CAP` finds no
receiver waiting, and `sched::ipc_recv_cap` takes when one arrives.

It was false. A sender parks `PageFrame(p, 1)` there and blocks. `PageFrame::REVOKE` then runs
over that frame: `sched::delete_page_frame_caps_where` deletes the capability from every table
including the sender's own, and `revoke::unmap_under_object` unmaps every page the log records. The
hand-off slot is read by neither. The next `RECV_CAP` files the surviving capability in the
receiver's table, and the receiver may `MAP` a page the revoker believes it took back. Measured, not
argued: `kernel::user::revocation_in_flight_tests::
a_capability_revoked_while_it_is_in_flight_does_not_reach_the_receiver` went red on the tree as it
stood, on aarch64, with its vacuity guard and its premise check both green first.

`MemoryRegion::DESTROY` is the sharper case. `revoke::revoke_region` sweeps capabilities by
overlap precisely so that "no capability still names a page this allocator is about to hand out" is
true before the region's pages go back, which is the hole the review in milestone 142 (a text display good enough that people use it
instead of a GUI) found and closed.
An in-flight capability reopens it: the use-after-free DECISIONS §13 (capability revocation and
untyped reclamation) exists to prevent, through the one slot nobody
swept.

The tree had written the rule down, once, for a different object.
`sched::delete_reply_caps_naming` sweeps `outgoing_cap` beside the tables and its doc comment says
why in exactly the words this defect needed: *"`outgoing_cap` goes too, and it is the half a second
copy would forget ... a live `Reply` in a hand-off slot is the same forgery one step earlier."* The
one sweep without the defect is the one that states the rule. That is the same shape as milestone
307's row 12 finding, where a comment correctly explained a trap and then cited, as its precedent,
the one harness still caught in it. The failure is not that nobody knew; it is that knowing lived
in a doc comment on one call site, which is AGENTS.md's ladder rung four wearing the clothes of a
design.

Fixed here, in the three sweeps that lacked it (`delete_page_frame_caps_where`, which is both
`PageFrame` policies; `delete_device_frame_caps_from_others`; and `x86_64`'s
`delete_port_range_caps_impl`), by dropping the parked capability. Dropping rather than failing the
send is the behaviour `ipc_send_cap` already documents for the other way a hand-off comes up empty,
a receiver whose table is full: the data word still arrives and the receiver sees `NO_CAP`. No new
error reaches userspace, so nothing about the syscall surface moves. The falsification is recorded
and replayed red.

What it does not settle. Only the `PageFrame` sweep is driven by a test; the other two carry the
same two lines and nothing exercises them through a parked hand-off, so read those as reasoned from
the code. And the test stops at delivery rather than at exploitation: it proves the receiver holds a
capability naming the revoked run, not that it then read a page somebody else owns.

### A mapping the kernel wires at boot is invisible to revocation

`user::AddressSpace::map_physical` did not call `revoke::record_mapping`, and every unmap sweep in
`crate::revoke` is driven by that log. So a page placed by kernel wiring (the serial driver's UART
registers, a `Spawn::maps` entry, a `DeviceRun`, the initrd, the `x86_64` timebase page) survived
`DeviceFrame::REVOKE`, `PageFrame::REVOKE` and `MemoryRegion::DESTROY`: the capability went and the
mapping stayed, which is a take-back doing half its job.

This was first recorded here as latent, and that was wrong. The paragraph this replaces said
nothing reached it, on the grounds that every `map_physical` call site is boot wiring and a driver a
userspace supervisor builds takes `MAP_INTO`, which records. Both halves of that are true and the
conclusion does not follow, because it asks only where the *mapping* comes from and never asks
whether anything holds a *capability* to the same page. Something does, in an ordinary boot, and the
two halves are wired by different modules, which is why nobody had put them side by side:
`user::fs_service::spawn_fs_server` puts the file channel's shared pages into the FS server through
`Spawn::maps`, and `user::boot_progenitor` hands the progenitor `PageFrame(file_shared, 1)` with
`GRANT` over the first of exactly those pages. `PageFrame::REVOKE` on that slot is a syscall the
progenitor may make: it deleted every capability naming the run and unmapped every mapping recorded
under it, and the FS server kept its writable mapping of a page the progenitor had just un-shared.

Fixed, by making the record a required argument rather than a call to remember. `map_physical`
now takes `revoke::PageMapSource` and records, exactly as `record_mapping` has required an answer
since §132 (what `PageFrame::REVOKE` owes an overlapping run); it was the one mapping site
in the kernel that never had to answer, because it never recorded. Every kernel-wiring caller passes
`NoCapability`, truthfully, so the page stands as its own object and a single-page revoke of it
finds the record. `user_address_space_map` was the one caller that remembered to record, and is now
the one caller with nothing extra to remember. Nothing about the syscall surface moves.
`kernel::user::spawn_mapping_revocation_tests::a_page_the_kernel_wired_is_unmapped_when_its_frame_is_revoked`
is the falsification, replayed red on aarch64 with its vacuity guard and premise check green first.

What it costs, since a record is not free and these are boot paths. One `LogEntry` per mapping,
170 to a page, paid out of the mapped space's own backing region. Every caller but one maps a
handful of pages; the exception is the initrd read-only window, 2837 pages on aarch64 today, which
is 17 log pages per space that maps it. The suite was green before those budgets were widened, which
means the existing slack absorbed it with roughly 8% to spare, which is the reason the term is now
explicit (`revoke::log_pages_for`) rather than a reason it did not need to be: the archive grows
every milestone and the first budget to blow would have done it on an unrelated change.

What it does not settle. The test drives the `PageFrame` sweep. `revoke_device_from_others` and
`revoke_region` read the same log and are fixed by the same record, but nothing drives a wired
mapping through either, so read those as reasoned from the code. `revoke_region`'s limb is the
narrower one and worth stating as a genuine "narrower than recorded": no `map_physical` call site in
the tree maps a page that came from a `MemoryRegion` (they come from `memory::alloc`, from MMIO, or
from the initrd), so `MemoryRegion::DESTROY` never had a wired mapping to walk past. And
`AddressSpace::map_new` still records nothing, deliberately: its frames are retyped from the space's
own region and freed with it, so no capability names them.

### What was attacked and held

Stated one attack per line, because the misses are what make the hit worth believing.

| # | The attack | Result |
|---|---|---|
| 1 | Is there any other authority-bearing field on `Thread` that a table sweep cannot see? | **Held.** `port_range_grant` (milestone 313's find, fixed) and `cycle_counter_grant` are the only two, both are cleared where they must be, and `mailbox` carries scalars. `outgoing_cap` was the third and is the finding above. |
| 2 | Does a recycled TCB slot inherit a dead thread's authority? | Held, structurally. `Thread` has no `Default` and all four constructors (`boot`, `adopt_current`, `spawn_into`, `embryo`) write every field by name, so a new authority-bearing field cannot be added without four decisions. Rung one. |
| 3 | Is the cycle-counter grant enforced on every architecture? | **Held, as a stated exception.** `x86_64`'s `set_cycle_counter_grant` is an empty function: `rdtsc` is ambient in ring 3 and DECISIONS 139 part 3 kept it that way, because `user_mode_runtime`'s `now()` *is* `rdtsc` there and closing it would take out `Instant`, `sleep`, the seed and the benchmark harness at once. It says so in its own doc, at length, and `notes/x86-port.md` carries it. Not a gap. |
| 4 | Can a thread read another thread's FP/SIMD registers? | Held. `fp::hand_over` scrubs to `FpState::INITIAL` on the live-to-not-live switch, `sched::schedule` is the only switch site in the kernel, and the module header names `LazyFP` (CVE-2018-3665) as the reason it is eager rather than lazy. |
| 5 | Can a name escape a directory capability's subtree? | Held. `fs_subtree_caretaker` performs no checks by design, so the whole claim rests on `redoxfs_server::check_component`, which refuses `.`, `..`, the attribute store's directory, and any name containing `/`, `\`, `:` or NUL. All twelve name-taking server verbs call it, `open_dir` and `make_dir` through `resolve_child_dir`. The cross-directory `rename` checks both names and both handles. |
| 6 | Is `record_mapping`'s failure ignored anywhere, so that a mapping is unrecorded? | Held. Three call sites, all three check the return and unmap what they just mapped. The unrecorded mappings are the ones that never call it, which is the section above. |
| 7 | Does a revocation reach the IOMMU's device domain? | **No, and it is a documented limit rather than a live defect.** `iommu::confine` has no inverse and its own doc says it runs once per device per boot. Only kernel drivers call it and the regions it maps are kernel-owned, so there is nothing a userspace revoke is failing to undo. It goes live the day a driver leaves the kernel and programs its own device, which is the question DECISIONS §86 (whether an NVMe driver can leave the kernel, and what
capability would let it) answers. |
| 8 | Can an ELF segment whose `p_vaddr + p_memsz` overflows be laid over the kernel? | **Held.** `map_segments` maps page by page through `AddressSpace::map_new`, whose `Mapper` is built `Half::Low`, so the refusal is per page rather than per segment and a run that walks out of the low half is refused where it walks out. This is the defect class milestone 142's review found in the two syscall map paths, which check the run's last page as well as its first. |

### What this pass could not reach

- The falsification for the finding above was aarch64's until milestone 323 replayed it on riscv64
  and `x86_64` on 2026-10-03 (UTC), red on both. Nothing in `outgoing_cap`, the sweeps or the
  rendezvous is architecture-specific, so the record now names all three.
- The claims whose enforcement is a userspace program rather than the kernel. The caretakers
  were read, not attacked from a hostile client. A hostile client is a fixture and a boot, and it is
  the shape `design/fatal-risks/README.md` says wants outside eyes anyway.
- Anything needing hardware this project does not own. MSI confinement stays exactly where
  milestone 317 (the interrupt-remapping flags, and where MSI confinement actually lives) left it.

## BUGS

- ~~**Six kernel confinement tests in the table are marked "no", and there is no mechanism to
  change that.**~~ **Closed by milestone 305, and the mechanism it needed had existed since
  2026-08-31 without anyone connecting the two.** The sentence this replaces said that automating a
  kernel falsification "needs a way to run one kernel test by name, which does not exist". Milestone
  210 built exactly that (`cargo xtask test --test <substring>`, with the filter baked into the test
  binary by `kernel/build.rs` and read by `kernel/src/testing.rs`'s runner) and this note went on
  saying it did not exist for a fortnight. `script/falsifications` now reads a `Falsification:` block
  above a `#[test_case]` and replays it by booting one architecture; `kernel/falsifications/` is the
  path §134 already spells, and row 20's patch is swept rather than remembered.
- Rows 22, 23, 24 and 30 now carry a record on all three ISAs; row 19's kernel tests carry none.
  Milestone 323 replayed the four aarch64 records on riscv64 and `x86_64` on 2026-10-03 (UTC), each
  red. Row 19's two Kani harnesses are ISA-neutral and now both have a
  record, but `dir_capability_tests` has none. The `attenuate` defect turns its
  read-only test red on aarch64 only through the vacuity guard at
  line 974 of `kernel/src/user/fs_service.rs`, like row 24's, so none was written.
- A kernel row's evidence is re-checked far less often than a harness row's, and on one
  architecture. A Kani record costs a second, so `script/falsifications --affected-since` re-checks
  it on every pull request that can reach it. A kernel record costs a boot per leg, so it is re-checked only
  by a full `--sweep`, which nobody runs per commit, and only on the architectures its patch names.
  Both limits are in `script/falsifications`' own `BUGS`. Read the "Falsified" column accordingly: a
  yes on a kernel row is a machine-replayable fact that nothing replays on a schedule.
- This table is a floor and its own worst failure is invisible. It cannot list the claim
  nobody made. Every row here was found by reading what this project already wrote, so the
  enumeration inherits exactly the blind spots the tests have. §31's `BUGS` and
  `design/fatal-risks/README.md` both say the decisive experiment is adversarial and by somebody else;
  this is not that, and calef's position gates outside eyes behind milestone 198.
- A recorded falsification proves the harness catches *that* defect, not the class. The
  `the_view_and_the_reap_have_the_same_scope` record carries a prediction that was measured
  false: its defect was claimed to be caught by that harness alone and it also reaches
  `reap_is_permitted_only_to_the_supervising_rendezvous`. Both the prediction and its correction
  are in the patch, which is the point of writing the prediction down.
- **Nothing gates which assertion a row's evidence comes through, and milestone 307 is a manual
  sweep rather than a mechanism.** `script/falsifications` checks that a recorded defect turns the
  harness red; it cannot check that the red came through the assertion the patch's prose predicts,
  which is its own `BUGS`' standing entry, and it has nothing at all to say about an assertion that
  is *unreachable while the harness is green*. Row 12's survivor was found by reading the encoder
  beside the assertion and then breaking a constant on purpose. Nine rows' worth of that reading is
  recorded above and it will rot the moment somebody rewrites one of these harnesses. Read the
  verdicts as dated 2026-09-16.
- An assertion that is unreachable because a guard above it is correct becomes reachable the day
  the guard is wrong, so "cannot run" is not "delete it". Rows 4 and 15 are left exactly as they
  are for this reason: the unreachable assertion is the claim in the claim's own words, and it costs
  nothing but a line. Where 307 did remove such a line (rows 13, 14, 18) it was because the assertion
  was a *restatement of the guard itself*, so no defect anywhere can separate them. The distinction
  is worth keeping: one is redundancy, the other is decoration.
- The sweep read every assertion and broke exactly one. Reading is how all nine unreachable
  assertions were found and it is cheap; breaking is the only thing that can find a survivor and it
  costs a solver run or a boot per defect. Row 12 was broken because the reading predicted a
  tautology and a prediction about a proof is worth confirming. The other 25 rows' verdicts are
  **reasoned from the code, not measured**, and milestone 305's own headline is the standing warning
  about what that is worth: `user_can_read` had been readable for four weeks.
- **Rows 27 to 29 were added by an audit and their verdicts are dated 2026-09-17.** Milestone 313
  found the tree's newest device object (the `x86_64` `PortRange`, milestone 299) absent from this
  table, its two tests unable to go red in the direction they exist for (a wrongly permitted `out`
  was followed by a `SEND` nobody received, so the escape hung the run: row 26's shape, one object
  over), and a third property, self-deletion, false in the tree. The fixtures were reshaped so an
  escape exits and arrives as `EVENT_EXIT` where the test wants `EVENT_FAULT`, and all three rows
  carry a record replayed on `x86_64`. Row 27's record also names the defect that did not fire,
  because the hand-off is protected twice (the bitmap bits and the `iomap_base` word) and only a
  defect that defeats both turns the test red. The audit is
  [design/audit-reports/2026-09-17-userspace-confinement.md](../design/audit-reports/2026-09-17-userspace-confinement.md).
- The rows citing kernel tests are still not evidence at the same grade as the rows citing
  harnesses, and the reason changed. It used to be that nothing could replay a kernel
  falsification at all. Since milestone 305 a machine can, so the gap is narrower and it is now
  about what the replay *proves*: a Kani harness is checked by a solver over every input in its
  bound, and a kernel test is one boot of one machine with one fixture attached, on one
  architecture. A kernel row that skipped for want of a disk or a device page is not evidence at
  all, which is why `--sweep` reports a skipped test as an error rather than as either colour.
