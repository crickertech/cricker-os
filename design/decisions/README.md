# nife: Architecture Decisions

*Name: ratified 2026-08-04 (calef, §75 (directories under `design/` and `notes/` carry provenance in their own README) covers this directory). `decisions` names what the files are, one each, and the tree cites them as `§N` regardless of the directory's name; the plural is deliberate, since a singular would read as one decision about design rather than a place where decisions live.*

Decisions made 2026-07-12, before any code was written. Each entry records what we
chose, what we rejected, and why. Revisit these deliberately, not accidentally.

## How this directory works

**The table under `## The decisions` is generated.** Run `script/decisions --write-index` after
adding a decision; the prose around it is written by hand and is not touched.

One decision, one file, named `NN-slug.md`. The number is the identity: `§14` is
`14-project-direction.md` and nothing else, and the 2,000-odd `§N` citations spread across the
kernel, the crates, the notes and the roadmap resolve here.

This was one 5,320-line file until milestone 114 (split `DECISIONS.md`, and give a decision a status). Splitting it does the same three things the
roadmap split (milestone 76) did one directory over:

- **A number cannot be claimed twice by accident.** Two lanes both wrote `## 30.` into the single
  file on 2026-07-30 and it merged clean, because both sides' prose survived. As two files claiming
  one number it is a gate failure, which is what `script/decisions --check` now reports.
- **Text cannot land under the wrong heading.** §67 was appended into the middle of §61's BUGS
  section on 2026-08-03, because the insertion anchored on the string `## Reading` and the first
  occurrence of that string in the file was a wrapped prose line inside §61. Every gate reported
  clean for a day. `cat >>` into `67-second-stream.md` can only add text to §67.
- **A status flip stops being a conflict.** Marking one decision superseded used to edit the file
  every other lane was also editing.

Milestone 582 finished that argument one step further. The index table was still hand-maintained, so
every lane minting a section edited one sorted file and collided with every other lane in flight,
always, there. `briefs/rebase-onto-main.md` names it as case 4's worked example. Generating the
table from the files removed the last additive-index conflict in this tree, the same move
milestone 294 (`design/roadmap/README.md`'s index is generated, not hand-maintained) made for `design/roadmap/README.md`.

Do not renumber. A number that moves breaks citations that no gate can see are wrong, because a
well-formed citation to the wrong section still resolves. Milestone 97 is where that check gets
built, and this directory is what makes it cheap: once a decision is a file with a title, a
citation's parenthetical name can be compared against that title.

## Frontmatter, which is where the status lives

Every decision opens with frontmatter, and it is the only record of its status. The prose may not
restate it, because two copies of one fact in one file is what the schema was ratified to remove.
The table below is generated from these keys by `script/decisions --write-index`, so a lane minting
a section writes one file and nothing else.

```yaml
---
status: DECIDED
raised: 2026-09-23
decided: 2026-09-23
ratified_by: calef
---
```

| key | values | required when |
|---|---|---|
| `status` | `PROPOSED`, `DECIDED`, `AMENDED`, `SUPERSEDED` | always |
| `raised` | `YYYY-MM-DD`, UTC | always |
| `decided` | `YYYY-MM-DD`, UTC | `status` is `DECIDED` or `AMENDED` |
| `ratified_by` | a GitHub username listed in [`ARCHITECTS.md`](../../ARCHITECTS.md) | `status` is `DECIDED` or `AMENDED` |
| `superseded_by` | a section number | `status` is `SUPERSEDED` |

The schema was ratified 2026-09-23 (calef); a lane does not extend it. Keys are snake_case,
values uppercase, dates UTC like every date in this tree.

This was `**Status: DECIDED.**` in prose until milestone 582 (a decision's status becomes a field,
and the index becomes generated). Two failures came from reading a field out of a sentence. A file
said `**Status: NOT YET`, the regex captured `NOT`, dropped `YET`, and the report printed a word
nobody had written, which is §211 (what a fatal-risk verdict says, and what the chart can plot as a result)'s finding. `design/fatal-risks.md`'s risk 7 carried two status
lines eighteen lines apart, and the first one won, so `AUDITED` was invisible to every consumer for
weeks.

Where the dates came from: most of this corpus predates the schema and states neither date, so
milestone 582 filled them from the file's own prose where it says, and from git where it does not:
`raised` is the first commit that wrote the decision down, followed back through milestone 114's
split into the single `DECISIONS.md` it came from, and `decided` the first commit whose text reads
DECIDED or AMENDED. Both are author dates in UTC. Where the two sources disagree the earlier wins,
since prose written before 2026-09-13 dates by calef's local day.

A git date is the date, not a bound on it: calef, 2026-09-24, *"We write things down when we raise
them. There is no gap between the commit and when it was raised."* The split commit's own date,
2026-08-04, dates nothing here. The block for milestone 582 (a decision's status becomes a field,
and the index becomes generated) has the counts and the three decisions whose prose says otherwise.

`ratified_by` names an architect listed in [`ARCHITECTS.md`](../../ARCHITECTS.md).

| Status | Means |
|---|---|
| `PROPOSED` | Raised, not yet decided. Options and a recommendation are in the file; nothing is built on it, and nothing should cite it as settled. Waiting on an architect. |
| `DECIDED` | It holds as written. |
| `AMENDED` | It holds, but part of it was revised or overtaken by later work. The file names what changed, and the amendment is in the file or in the decision it names. |
| `SUPERSEDED` | A later decision replaces it, and `superseded_by` names which. Kept, never deleted, because the reasoning is the record: §8 (DEFERRED to a hard decision point)'s deferral was correct and §10 (process model: capability-based, microkernel) is what it deferred to. The table below shows it as `SUPERSEDED BY N`, which is the two keys read together. |

`AMENDED` is the token that pays for the vocabulary. Eleven decisions carry a revision that a reader
of the opening paragraph would otherwise miss, and §26 is the sharpest: its first line still says
"not yet built" while three blocks below it record milestone 22 building it. Nothing flagged that,
because a decision had no status to contradict.

**A decision waiting on an architect is `PROPOSED`, and lives here, not in a queue of its own.**
`design/open-decisions.md` was that queue for one day, and it existed for a good reason: a decision
that lives only in a conversation's scrollback is in the medium milestone 94 was written to abolish.
But a proposal is the same object one lifecycle step before `DECIDED`, with the same shape (what is
being decided, the options, the recommendation and its reason, what is blocked until it is
answered). So it gets a file and a number when it is raised, and answering it flips a status in
place instead of moving text between two systems. Numbers 68 to 73 are the six that queue held.

## The decisions

| # | Status | Decision |
|---|---|---|
| 1 | DECIDED | [Target architecture: aarch64](01-target-architecture.md) |
| 2 | DECIDED | [Primary target: QEMU `virt`, Raspberry Pi as a later port](02-qemu-virt-primary.md) |
| 3 | DECIDED | [Use the crate ecosystem](03-crate-ecosystem.md) |
| 4 | DECIDED | [Kernel shape: monolithic, deferred, with two cheap rules](04-kernel-shape.md) |
| 5 | AMENDED | [Execution model: preemptive threads with real stacks](05-preemptive-threads.md) |
| 6 | SUPERSEDED BY 11 | [SMP: single-core, refactor when it hurts](06-single-core-first.md) |
| 7 | DECIDED | [Testing: QEMU harness + host-testable crates, from commit one](07-testing-harness.md) |
| 8 | SUPERSEDED BY 10 | [Process model / syscall ABI: DEFERRED to a hard decision point](08-process-model-deferred.md) |
| 9 | DECIDED | [Locking: IrqSafeMutex, plus a discipline](09-irq-safe-locking.md) |
| 10 | DECIDED | [Process model: capability-based, microkernel. Untyped memory deferred.](10-capability-microkernel.md) |
| 11 | AMENDED | [SMP: per-CPU run queues, message-based migration. §6, reopened.](11-per-cpu-run-queues.md) |
| 12 | DECIDED | [Call/Reply IPC: a one-shot reply capability](12-call-reply-ipc.md) |
| 13 | DECIDED | [Capability revocation and untyped reclamation (frames)](13-frame-revocation.md) |
| 14 | AMENDED | [The project's direction: a verified-Rust capability microkernel that runs real workloads](14-project-direction.md) |
| 15 | DECIDED | [The native ABI: formalize the convention, defer the BootInfo (milestone 19e)](15-native-abi.md) |
| 16 | AMENDED | [Object revocation: reclaim the objects a process built (extends §13)](16-object-revocation.md) |
| 17 | DECIDED | [The second architecture: RISC-V, and the page-table format trait](17-riscv-second-architecture.md) |
| 18 | DECIDED | [The PCIe transport: one driver, two buses, the seam in the kernel](18-pcie-transport.md) |
| 19 | DECIDED | [Architectural parity is a tenet; the targets are aarch64, riscv64, and x86_64](19-architectural-parity.md) |
| 20 | DECIDED | [IOMMU-backed DMA isolation: one seam, two arch drivers (milestone 16b (real hardware and IOMMU-backed driver isolation))](20-iommu-dma-isolation.md) |
| 21 | AMENDED | [The terminal is a userspace component, and the kernel is out of the shell business (milestone 28)](21-terminal-in-userspace.md) |
| 22 | AMENDED | [Rust `std` on the native ABI, the Hermit way (milestone 27)](22-rust-std-on-the-native-abi.md) |
| 23 | DECIDED | [Multi-queue DMA confinement: the validator's second direction (milestone 30 (the network stack as a confined component))](23-multi-queue-dma-confinement.md) |
| 24 | AMENDED | [Interrupting the foreground process: two-tier, shell-held, no new kernel surface](24-interrupting-the-foreground.md) |
| 25 | DECIDED | [Socket identity: a socket id in phase one, minted endpoints as the tracked later step](25-socket-identity.md) |
| 26 | AMENDED | [The fault endpoint: thread death becomes a message a supervisor holds](26-fault-endpoint.md) |
| 27 | AMENDED | [The filesystem service: a capability-shaped contract over a component we did not write (milestone 32 phase 2)](27-filesystem-service.md) |
| 28 | AMENDED | [SMP placement: two random choices at spawn, message-shaped stealing, local wakes](28-smp-placement.md) |
| 29 | DECIDED | [The framebuffer is a bigger grant, not an exemption (milestone 29 (a display terminal), the display ladder's rung one)](29-framebuffer-grant.md) |
| 30 | DECIDED | [The DMA boundary is proved for descriptors, and the proof says where it stops (milestone 35 (prove the DMA-confinement boundary))](30-dma-boundary-proof.md) |
| 31 | DECIDED | [The foreign-language seam: C holds no capabilities and makes no syscalls (milestone 36 (a foreign-language component, seam first))](31-foreign-language-seam.md) |
| 32 | DECIDED | [A supervisor may collect a corpse without being able to build one](32-reap-without-build.md) |
| 33 | DECIDED | [The compositor's authority is memory, not messages (milestone 33 (a compositor), the display ladder's rung two)](33-compositor-authority.md) |
| 34 | AMENDED | [RedoxFS is the primary filesystem, on three conditions](34-redoxfs-primary.md) |
| 35 | DECIDED | [What a scanner is for here, and how its findings get dispositioned](35-scanner-findings.md) |
| 36 | DECIDED | [The repository is part of the TCB (milestones 44 and 42)](36-repository-in-the-tcb.md) |
| 37 | DECIDED | [Text is a value three witnesses compute, not a screenshot (the remaining increment of milestone 29 (a display terminal))](37-text-as-a-value.md) |
| 38 | DECIDED | [A suppression is scoped to an item and carries a reason, or it does not ship (milestone 41 (dead code: triage the suppressions))](38-scoped-suppressions.md) |
| 39 | DECIDED | [A component is named for what it is, and nothing is named for a daemon](39-component-names.md) |
| 40 | DECIDED | [A supervisor's death is its subtree's death; there is no reaper of last resort](40-no-reaper-of-last-resort.md) |
| 41 | DECIDED | [The endpoint is the broker, and a device is revoked by taking it back (milestone 23 (a capability-routed component OS with live replacement))](41-endpoint-as-broker.md) |
| 42 | AMENDED | [A filesystem declares what it offers and must be truthful; it is not required to be capable](42-truthful-filesystem.md) |
| 43 | DECIDED | [Reading the clock is a page, setting it is a page you may write, proposing is an endpoint](43-clock-authority.md) |
| 44 | DECIDED | [Entropy is a capability, `std::random` improves transparently, and the refusal is loud](44-entropy-capability.md) |
| 45 | DECIDED | [A nife partition is `EC5CC08B-D749-4434-AC38-A274C50385BA`, and that never changes](45-partition-guid.md) |
| 46 | AMENDED | [Thin primitives or whole subsystems; we write everything in between](46-dependency-rule.md) |
| 47 | DECIDED | [A directory capability carries six rights, and a child can never exceed its parent](47-directory-rights.md) |
| 48 | DECIDED | [Navigation is the shell rebinding what it holds, and every shell has its own root](48-shell-navigation.md) |
| 49 | DECIDED | [Removal is a directory operation, and `-r` widens the grant rather than setting a flag](49-removal-and-recursion.md) |
| 50 | DECIDED | [Namespace composition (`bind`), not stored paths](50-namespace-composition.md) |
| 51 | DECIDED | [The sink protocol: a writer must not be able to tell what it is writing to](51-sink-protocol.md) |
| 52 | DECIDED | [A set of names is a namespace, and that is how a glob is granted](52-nameset-glob-grant.md) |
| 53 | DECIDED | [Parity is a matrix, not a pair](53-parity-matrix.md) |
| 54 | DECIDED | [Recovering a backup includes its metadata, and formatting a disk needs entropy](54-host-recovery.md) |
| 55 | DECIDED | [The file behind a `>` is the shell itself, because one page cannot serve two clients](55-shell-holds-the-redirect.md) |
| 56 | DECIDED | [The filesystem contract describes its own verbs, so a caretaker is written once](56-verb-table.md) |
| 57 | DECIDED | [Extended attributes forward through the caretakers, and the server enforces direction](57-xattr-forwarding.md) |
| 58 | DECIDED | [A wider archive name, and the one format change that had to bump the magic](58-wider-archive-name.md) |
| 59 | DECIDED | [Append is an open mode, so `>>` costs a character and a flag](59-append-mode.md) |
| 60 | DECIDED | [Fuzzing complements the proofs, and the parsers are exactly where it wins](60-fuzzing-the-parsers.md) |
| 61 | DECIDED | [A lint is adopted on evidence from this tree, not on its description](61-lints-on-evidence.md) |
| 62 | DECIDED | [Nothing of yours lives below the live stack pointer](62-below-the-stack-pointer.md) |
| 63 | DECIDED | [The line between a program and its crate is "does this need a capability"](63-program-versus-crate.md) |
| 64 | DECIDED | [A per-file coverage number counts where tests are written, not what they reach](64-per-file-coverage.md) |
| 65 | DECIDED | [A refusal that is not passive cannot be used as a question](65-active-refusal.md) |
| 66 | DECIDED | [A refusal is a non-zero status, and not the same one an error gets](66-refusal-status.md) |
| 67 | DECIDED | [A program's second stream is a declaration, not a number](67-second-stream.md) |
| 68 | DECIDED | [`BootEndowment::unused` wants a truer name: it is `for_test_roles`](68-boot-endowment-name.md) |
| 69 | DECIDED | [`Endow` is a verb, and names the same idea as `Endowment`: it is `ChildEndowment`](69-endow-versus-endowment.md) |
| 70 | DECIDED | [BUILT measures the end-state, and a false premise is rewritten in as the finding](70-built-when-nothing-to-fix.md) |
| 71 | DECIDED | [A limitation is promoted when it stops being a fact and becomes a plan](71-recorded-limitations.md) |
| 72 | DECIDED | [`time` needs no clock: duration is ambient, wall-clock identity is authority](72-time-command-clock.md) |
| 73 | DECIDED | [Milestone 44's ten admin minutes, which only calef can spend](73-repository-admin-steps.md) |
| 74 | DECIDED | [Audits run on change, not on the calendar: events first, then a count](74-audit-cadence.md) |
| 75 | DECIDED | [Directories under `design/` and `notes/` carry provenance in their own README](75-naming-covers-directories.md) |
| 76 | DECIDED | [What catches a milestone status that is wrong in both places?](76-roadmap-status-versus-tree.md) |
| 77 | DECIDED | [The branch-prefix list now describes the tree](77-branch-prefixes.md) |
| 78 | DECIDED | [Signed commits: worth doing, and not as a side effect](78-signed-commits.md) |
| 79 | AMENDED | [Holding password-equivalent material, and what a session key release means](79-password-equivalent-material.md) |
| 80 | DECIDED | [One build for the kernel and everything that runs on it](80-one-build-for-everything.md) |
| 81 | DECIDED | [A dependency stays upgradable; we suppress churn, never the upgrade](81-dependency-upgrades.md) |
| 82 | DECIDED | [Ambient authority is the problem; replacing the ecosystem, not confining it, is the end state](82-ambient-authority-and-the-rewrite.md) |
| 83 | DECIDED | [When the same thing exists in C and in Rust, take the Rust one](83-rust-over-c-implementations.md) |
| 84 | DECIDED | [How we port: prefer software that has already dropped ambient authority](84-how-we-port.md) |
| 85 | DECIDED | [What we port is evidence and must not be ours; what we ship is product and must be](85-evidence-and-product.md) |
| 86 | DECIDED | [Whether an NVMe driver can leave the kernel, and what capability would let it](86-el0-nvme-driver.md) |
| 87 | DECIDED | [MIT OR Apache-2.0, and why the GPL's lesson does not transfer](87-permissive-license.md) |
| 88 | DECIDED | [`needs-architect` as a required check, rather than as a script's restraint](88-needs-architect-as-a-check.md) |
| 89 | DECIDED | [`provisional` becomes the fourth provenance state](89-provisional-versus-unrecorded.md) |
| 90 | DECIDED | [The claim is a draft pull request; the status flip is a gate](90-claiming-and-closing.md) |
| 91 | DECIDED | [A region's endpoints are swept before its refusal, not after](91-endpoints-before-the-refusal.md) |
| 92 | DECIDED | [A caretaker is supervised by the client it serves](92-caretaker-lifetime.md) |
| 93 | DECIDED | [The filesystem wire protocol is ours, and 9P is an adapter at the edge](93-filesystem-wire-protocol.md) |
| 94 | DECIDED | [What may live in a userspace library, and what must be per-binary](94-what-may-live-in-a-library.md) |
| 95 | DECIDED | [A hand-written IPC fastpath, and whether it can stay proven](95-a-proven-ipc-fastpath.md) |
| 96 | DECIDED | [Process kernel or event kernel, and how to decide it](96-process-kernel-or-event-kernel.md) |
| 97 | DECIDED | [Six gates run on every pull request and none of them can stop one](97-advisory-checks.md) |
| 98 | PROPOSED | [`OPENDIR` cannot be asked to attenuate, so a held directory probes for its own rights](98-opendir-cannot-attenuate.md) |
| 99 | DECIDED | [Where Apple's metadata lands: stream or sidecar](99-apple-metadata-at-rest.md) |
| 100 | AMENDED | [The terminal font](100-the-terminal-font.md) |
| 101 | AMENDED | [Notification objects: async multiplexing without wait-any](101-notification-objects.md) |
| 102 | DECIDED | [A Frame names a run of pages](102-frame-names-a-run.md) |
| 103 | SUPERSEDED BY 102 | [What a `Frame` names](103-what-a-frame-names.md) |
| 104 | DECIDED | [The rich-text font is DejaVu Sans Mono, and the palette is Solarized](104-the-font-and-the-palette.md) |
| 105 | DECIDED | [`std::thread::spawn` stays declined, until a customer needs it](105-thread-spawn-decline-for-now.md) |
| 106 | DECIDED | [Take the `terminal_sink_caretaker` narrowing: an unredirected tail stage's output goes to the screen, not the shell](106-tail-output-narrowing.md) |
| 107 | DECIDED | [`missing_docs` moves to `workspace.lints.rust`, opt-out rather than opt-in](107-missing-docs-workspace-wide.md) |
| 108 | AMENDED | [Disabling a user's login credentials kills their durable session](108-credential-revocation-kills-durable-session.md) |
| 109 | DECIDED | [Attribution is a property of a channel, not of a capability](109-attribution-is-a-channel-property.md) |
| 110 | DECIDED | [Hard links are declined, for want of a customer](110-hard-links-declined.md) |
| 111 | DECIDED | [Inert configuration is a read-only page, and each declared key is validated against a closed domain](111-inert-config-is-a-validated-page.md) |
| 112 | DECIDED | [`touch`'s two behaviors need two rights: write covers "now", a separate right covers "arbitrary"](112-touch-mtime-authority.md) |
| 113 | AMENDED | [Eleven kernel object and identifier names move from contraction or borrowed jargon to the plain, standard term](113-kernel-object-plain-names.md) |
| 114 | DECIDED | [`pmap` gets its listing: `ENUMERATE` extends to the address-space object](114-aspace-enumerate.md) |
| 115 | DECIDED | [No `sysctl`: each subsystem's tuning goes through its own service, not a bolted-on aggregator](115-no-sysctl.md) |
| 116 | SUPERSEDED BY 209 | [Live component state handoff is declined, for want of a customer](116-state-handoff-declined.md) |
| 117 | DECIDED | [A principal's subtree is named by its identity string, created at provisioning time](117-subtree-name-is-identity.md) |
| 118 | DECIDED | [`Scheduler`/`SCHED` rename to `IpcTables`/`IPC_TABLES`](118-ipc-tables-name.md) |
| 119 | DECIDED | [Splitting `OutOfMemory`'s three causes is declined for want of a customer](119-oom-causes-declined.md) |
| 120 | AMENDED | [A QEMU-only virtio-rng stopgap for the interactive boot](120-boot-entropy-stopgap-declined.md) |
| 121 | AMENDED | [What a device capability is when the device has no page: x86 port I/O](121-port-io-capability.md) |
| 122 | DECIDED | [The on-disk, per-user schedule store: format, write path, read-at-boot path](122-durable-schedule-store-format.md) |
| 123 | AMENDED | [Boot-time re-derivation: what grants the privilege, and how it dies after one use](123-boot-time-rederivation-privilege.md) |
| 124 | DECIDED | [Ratify the x86_64 syscall ABI](124-x86-64-syscall-abi.md) |
| 125 | DECIDED | [What tells boot-time re-derivation which identities have pending work](125-durable-schedule-manifest.md) |
| 126 | AMENDED | [A process holding two directory capabilities gets a real, single, moving `cwd`](126-two-directory-cwd.md) |
| 127 | DECIDED | [x86_64's `now()`/`cntfrq()`: PIT-calibrated `rdtsc`, ratifying what PR #476 already built](127-x86-64-timer-rdtsc.md) |
| 128 | DECIDED | [What enforces the git-clobber rule, now that it has crossed its own threshold](128-git-clobber-enforcement.md) |
| 129 | DECIDED | [Whether `filesystem_proto::fs::RENAME` grows a `NOREPLACE` flag, revisiting §42](129-rename-noreplace-flag.md) |
| 130 | DECIDED | [How the kernel-resident CMOS RTC reaches the userspace clock service](130-cmos-rtc-delegation.md) |
| 131 | DECIDED | [The competitor question: hold at rung two, prove text-mode usefulness first](131-hold-at-rung-two.md) |
| 132 | DECIDED | [What `PageFrame::REVOKE` owes an overlapping run](132-overlapping-page-frame-runs.md) |
| 133 | DECIDED | [Whether an idle core should drain its own inbox before parking](133-idle-core-self-rescue.md) |
| 134 | AMENDED | [A harness carries a machine-replayable falsification record, or it is not evidence](134-harness-falsification-record.md) |
| 135 | AMENDED | [Running GPL software is aggregation, the capability boundary is what makes it so, and packages are how it arrives](135-running-gpl-software.md) |
| 136 | DECIDED | [A mature foreign implementation earns its place as an oracle, not as a dependency](136-foreign-implementation-as-oracle.md) |
| 137 | PROPOSED | [A hardware TRNG with no published health-test claim](137-trng-health-tests.md) |
| 138 | DECIDED | [How a saturated workload is made to hand threads across cores](138-cross-core-handoff-under-load.md) |
| 139 | DECIDED | [Who may read the cycle counter, and by what authority](139-cycle-counter-authority.md) |
| 140 | DECIDED | [The words a finished milestone may use to say what happened to the work it named](140-follow-on-disposition-vocabulary.md) |
| 141 | DECIDED | [Application is grant: what a command line means in a capability system](141-application-is-grant.md) |
| 142 | DECIDED | [What a spawner retains over a child after `START`](142-spawn-retention.md) |
| 143 | DECIDED | [A machine's name is not a hardware fact, and source comments should say the hardware](143-machine-names-in-source.md) |
| 144 | DECIDED | [The fastpath footprint gate gets a delta and a ceiling, and the ceiling is 16 KiB](144-fastpath-footprint-ceiling.md) |
| 145 | PROPOSED | [Compartmentalization at process cost: is Qubes' mission the reason the world needs this OS?](145-compartmentalization-at-process-cost.md) |
| 146 | PROPOSED | [Archive and compression: which pieces we write, which we take, and which we refuse](146-archive-and-compression-write-or-take.md) |
| 147 | AMENDED | [A timer a userspace service cannot hold](147-a-timer-a-userspace-service-cannot-hold.md) |
| 148 | AMENDED | [Milestone 105's two forks: a supervisor restarts by asking, and resolves by asking the kernel](148-reap-and-thread-identity.md) |
| 149 | DECIDED | [May the kernel answer on an endpoint, where §121 leaves no userspace holder?](149-kernel-served-console-endpoint.md) |
| 150 | DECIDED | [How does a thread's CPU time reach userspace?](150-per-thread-cpu-accounting.md) |
| 151 | DECIDED | [The goal of the repository split is independent release and third-party programs](151-repository-goal-is-independent-release.md) |
| 152 | DECIDED | [The port-range capability: object and method semantics on the syscall surface](152-port-range-capability.md) |
| 153 | DECIDED | [How a two-core x86_64 test earns its place, when two-core x86_64 is not yet trustworthy](153-two-core-x86-test-sequencing.md) |
| 154 | DECIDED | [The acronym test is whether the phrase is spoken, applied recursively](154-the-acronym-test-is-whether-the-phrase-is-spoken.md) |
| 155 | DECIDED | [The naming conventions move out of the constitution, and the note becomes the rule](155-naming-conventions-move-out-of-the-constitution.md) |
| 156 | DECIDED | [What the package manager waits on: a decision, not milestone 23 and not the repository split](156-the-package-manager-waits-on-a-decision-not-milestone-23.md) |
| 157 | DECIDED | [A trivial install is a web page, a USB drive, and packages over the internet](157-a-trivial-install-is-a-web-page-a-usb-drive-and-packages.md) |
| 158 | DECIDED | [A program is declared once: the archives read `Cargo.toml`, and the shell's table is one macro](158-a-program-is-declared-once.md) |
| 159 | DECIDED | [Lab machines upgrade like user machines, and only a new kernel needs a reboot](159-upgrades-without-a-reimage.md) |
| 160 | PROPOSED | [What a subshell copies, given that a capability set cannot always be copied](160-what-a-subshell-copies.md) |
| 161 | PROPOSED | [Which subset counts as running Vaultwarden](161-what-counts-as-running-vaultwarden.md) |
| 162 | AMENDED | [Whether a holder can give up a mapping, and what gives it up](162-giving-up-a-mapping.md) |
| 163 | DECIDED | [Where a confined device's IOMMU fault is delivered](163-where-a-device-fault-is-delivered.md) |
| 164 | SUPERSEDED BY 148 | [Whether the kernel resolves a tid it already sent to the supervisor that received it](164-resolving-a-tid-a-supervisor-holds.md) |
| 165 | PROPOSED | [Where a stored secret comes from on a boot that is not a test](165-where-a-stored-secret-comes-from.md) |
| 166 | DECIDED | [The rasteriser dependency, and whether the glyph atlas ships one face or four](166-the-rasteriser-and-how-many-faces.md) |
| 167 | PROPOSED | [What a profiling session's grant names](167-what-a-profiling-session-may-name.md) |
| 168 | PROPOSED | [Where a proof-gated credential rotation verb lives](168-where-a-rotation-verb-lives.md) |
| 169 | PROPOSED | [Whether a clipboard exists here, and what it is scoped to](169-whether-a-clipboard-exists.md) |
| 170 | AMENDED | [How a foreign program is told what to do](170-how-a-foreign-program-is-told-what-to-do.md) |
| 171 | DECIDED | [Where a program image starts, and where the stack goes](171-where-a-program-image-starts.md) |
| 172 | PROPOSED | [Whether a `credential_protocol` verify endpoint names the identity it asks about](172-credential-endpoint-per-resource.md) |
| 173 | PROPOSED | [Narrowing the root of the shell's namespace: a verb on the wire, or a shallower root](173-narrowing-the-namespace-root.md) |
| 174 | PROPOSED | [Which caller each of the three uncalled instruments gets](174-a-caller-for-the-three-uncalled-instruments.md) |
| 175 | DECIDED | [Where the kernel's own output goes once userspace owns the console](175-kernel-console-arbitration.md) |
| 176 | PROPOSED | [Offering the two RedoxFS patches upstream, and under whose name](176-offering-the-redoxfs-patches-upstream.md) |
| 177 | DECIDED | [Whether AGENTS.md quotes measured numbers at all](177-measured-numbers-in-the-front-door-file.md) |
| 178 | PROPOSED | [Where the timer re-arm seam goes, and which miss behavior the kernel tick is meant to have](178-timer-rearm-seam.md) |
| 179 | PROPOSED | [Whether the tour boot keeps starting a console server that has no client](179-console-server-with-no-client.md) |
| 180 | PROPOSED | [Whether `components/` splits again, for the tools a person invokes](180-a-third-program-directory.md) |
| 181 | PROPOSED | [May `script/bootstrap` say "installed everything I could, and this machine is still not good enough" without failing?](181-bootstrap-provisions-and-judges.md) |
| 182 | PROPOSED | [Is a string two binaries agree on a name for `script/names`' purposes, or is it data?](182-provenance-for-wire-visible-names.md) |
| 183 | PROPOSED | [What `script/ci-build` with no arguments means, and what the two tiers are called](183-what-no-arguments-means.md) |
| 184 | PROPOSED | [Does the host test pass run on a second architecture, and at what cadence?](184-an-x86-64-host-in-the-host-pass.md) |
| 185 | DECIDED | [What carries the claim that userspace composes a process from an authority you can count on one hand](185-composing-a-process-from-two-capabilities.md) |
| 186 | PROPOSED | [Where a riscv64 tour-boot check runs, what it asserts, and what it is called](186-where-the-riscv-tour-check-runs.md) |
| 187 | PROPOSED | [One crate per kernel-test pair, or one crate for all of them?](187-crates-for-the-numbers-a-kernel-test-and-its-program-agree-on.md) |
| 188 | PROPOSED | [What the lifted `fn check(ok: bool)` is called, now that nine programs write it out by hand](188-one-home-for-the-trap-on-false-helper.md) |
| 189 | PROPOSED | [Which of two definitions `caretaker` carries, and what the translating shape is called](189-what-a-caretaker-is-when-it-translates.md) |
| 190 | DECIDED | [Must an icount baseline save record why it moved, and does a second fixed anchor earn its cost?](190-what-a-baseline-save-must-record.md) |
| 191 | DECIDED | [Whether the job mix reports the spread rather than the best, and whether `REPEATS` varies by sweep point](191-job-mix-repeats-and-what-the-line-reports.md) |
| 192 | PROPOSED | [Does the ACPI walk's direct-map read take a bound, and is the bound per-read or a region it holds?](192-a-checked-direct-map-reader-for-the-acpi-walk.md) |
| 193 | PROPOSED | [What a block-roster entry calls an NVMe disk, and whether it carries more than virtio does](193-nvme-in-the-block-roster.md) |
| 194 | DECIDED | [Sessions interleave rather than serialize, and a renumber is the price](194-sessions-interleave-rather-than-serialize.md) |
| 195 | DECIDED | [A reviewed recipe vouches for a package, and the machine's owner may overrule it](195-a-recipe-vouches-and-the-owner-may-overrule.md) |
| 196 | DECIDED | [nife carries TLS: `rustls` for the protocol, and a crypto provider we make work](196-nife-carries-tls-and-builds-the-provider.md) |
| 197 | DECIDED | [A package is one archive file, named and vouched for by its recipe](197-a-package-is-one-archive-file.md) |
| 198 | DECIDED | [The glue is ours, the primitives are not](198-the-glue-is-ours-the-primitives-are-not.md) |
| 199 | DECIDED | [The screen check asks instead of sampling](199-the-screen-check-asks-instead-of-sampling.md) |
| 200 | DECIDED | [Two crates the documentation system refused, and why each loses on its own terms](200-two-crates-the-documentation-system-refused.md) |
| 201 | DECIDED | [One roadmap until a citation has to cross, and the blocked side declares the dependency](201-one-roadmap-until-a-citation-has-to-cross.md) |
| 202 | AMENDED | [Mechanical work goes to a cheaper model, and the gates are why that is safe](202-mechanical-work-goes-to-a-cheaper-model.md) |
| 203 | AMENDED | [Capacity is rented rather than bought, and what each of the three benches is still for](203-capacity-is-rented-not-bought.md) |
| 204 | DECIDED | [How userspace asks where a thread runs](204-how-userspace-asks-where-a-thread-runs.md) |
| 205 | DECIDED | [The subscription stays, and rented models fill the mechanical tail](205-the-subscription-stays-and-renting-fills-the-tail.md) |
| 206 | PROPOSED | [Filing a lane's findings is a step, not a duty somebody remembers](206-filing-a-lanes-findings-is-a-step-not-a-duty.md) |
| 207 | AMENDED | [The roadmap is a graph, and the block says so in fields a script can walk](207-the-roadmap-is-a-graph-and-says-so.md) |
| 208 | DECIDED | [Installing a package is granting it, and the activation set is versioned](208-installing-is-granting.md) |
| 209 | DECIDED | [State handoff is an opaque blob over a granted frame, and it is optional](209-state-handoff-is-an-opaque-blob-and-it-is-optional.md) |
| 210 | DECIDED | [A correction of error, and its action items are decisions, proposals or milestones](210-a-correction-names-its-action.md) |
| 211 | DECIDED | [What a fatal-risk verdict says, and what the chart can plot as a result](211-what-a-fatal-risk-verdict-says.md) |
| 212 | DECIDED | [A prose budget: 3,000 words of main body, with appendices under the same cap](212-a-prose-budget-for-every-document.md) |
| 213 | DECIDED | [Writing standards: three countable rules, one review rule, and a ratchet](213-writing-standards.md) |
| 214 | DECIDED | [The Team plan buys runner concurrency, and merge throughput is no longer third](214-the-team-plan-buys-runner-concurrency.md) |
| 215 | DECIDED | [The second RISC-V machine is a rented Scaleway Elastic Metal RV1](215-the-second-risc-v-machine-is-a-scaleway-rv1.md) |
| 216 | DECIDED | [Fatal-risk facts are correctable, and verdicts are the architect's](216-fatal-risk-facts-are-correctable-verdicts-are-the-architects.md) |
| 217 | DECIDED | [Every architect holds the whole role](217-every-architect-holds-the-whole-role.md) |
| 218 | DECIDED | [Carry a Kani patch so riscv64 is proved, and send it upstream](218-carry-a-kani-patch-so-riscv64-is-proved.md) |
| 219 | DECIDED | [How the shell names an installed program to the spawner](219-naming-an-installed-program-to-the-spawner.md) |
| 220 | DECIDED | [Signed builds: a vendor signs, a developer self-signs, and trusting a key is scoped](220-signed-builds-and-scoped-key-trust.md) |
| 221 | AMENDED | [The boot prompt is the owner's console](221-the-boot-prompt-is-the-owners-console.md) |
| 222 | AMENDED | [Who holds a user's schedule, and how it is changed while their session lives](222-who-holds-a-users-schedule.md) |
| 223 | DECIDED | [The process view is the supervision domain](223-the-process-view-is-the-supervision-domain.md) |
| 224 | DECIDED | [No `pwdx`: only the shell has a working directory, so there is nothing to report](224-no-pwdx.md) |
| 225 | DECIDED | [`free` sees the machine and your share: a region method and a withholdable memory page](225-free-sees-the-machine-and-your-share.md) |
| 226 | DECIDED | [`pidwait` takes tids and composes with `pgrep`, because a program does one thing](226-pidwait-takes-tids.md) |
| 227 | DECIDED | [How Tab reaches the shell: the shell edits its own line, and the terminal wire does not change](227-the-shell-edits-its-own-line.md) |
| 228 | DECIDED | [How a set of matched names reaches the progenitor: in a page the shell fills, copied and checked](228-how-a-set-of-names-reaches-the-progenitor.md) |
| 229 | DECIDED | [How a bare name at the prompt reaches an installed program: the activation set, and never an owner's vouch](229-how-a-bare-name-reaches-an-installed-program.md) |
| 230 | AMENDED | [Badged endpoint capabilities: how a server learns which client's frame a request is in](230-badged-endpoints-name-a-callers-frame.md) |
| 231 | DECIDED | [A swap's warning to a dependent is advisory, and the supervisor never waits for it](231-a-dependents-warning-is-advisory.md) |
| 232 | DECIDED | [The `line_editor` swap contract: a quiesce opcode on the served endpoint, and a retry flag](232-the-line-editor-swap-contract.md) |
| 233 | DECIDED | [`MemoryRegion::RETYPE` takes a page count](233-a-region-retypes-a-run-of-pages.md) |
| 234 | DECIDED | [The prompt shows one tree, and other trees are mounted at names in it](234-the-prompt-shows-one-tree.md) |
| 235 | AMENDED | [The OS is built and updated from packages, and the tree divides by what releases together](235-packages-build-the-os-and-the-tree-divides-by-release.md) |
| 236 | DECIDED | [`OPEN`/`OPENDIR` take a relative path, and `OPEN`'s reply carries the size](236-open-opendir-relative-path-and-size-hint.md) |
| 237 | DECIDED | [Subtree grants may skip the caretaker, per filesystem](237-subtree-grants-skip-the-caretaker-per-filesystem.md) |
| 238 | DECIDED | [The system tests leave the kernel crate as a second image, and a new test defaults to userspace](238-system-tests-leave-the-kernel-crate.md) |
| 239 | DECIDED | [Four package kinds, and TOML for package declarations and recipes](239-package-kinds-and-toml-declarations.md) |
| 240 | DECIDED | [§46 (thin primitives or whole subsystems): `toml` in `xtask`](240-toml-in-xtask.md) |
| 241 | DECIDED | [A threadbare base: the boot slot holds the kernel and what boots and repairs, and every other program is a package](241-a-threadbare-base.md) |
| 242 | DECIDED | [A system log: one service that programs and the kernel append to](242-a-system-log.md) |
| 243 | DECIDED | [Notices for people: programs publish, users curate, displays show](243-notices-for-people.md) |
| 244 | DECIDED | [The installed disk has four partitions, and a boot slot is 64 MiB](244-the-installed-disk-has-four-partitions.md) |
| 245 | DECIDED | [A `CALL` server tells a Reply from a delegation by a kernel tag in `x4`, read through a typed runtime helper](245-a-call-server-tells-a-reply-from-a-delegation.md) |
| 246 | DECIDED | [A plain `RECEIVE` never takes a capability, whichever side reaches the rendezvous first](246-a-plain-receive-never-takes-a-capability.md) |
| 247 | DECIDED | [The split begins with basalt holding nife, and `procps` moves first](247-the-split-begins-with-basalt-and-procps-moves-first.md) |
| 248 | DECIDED | [The name resolver is its own confined program, not a verb inside the network stack](248-the-resolver-is-its-own-confined-program.md) |
| 249 | DECIDED | [A running address space stays nameable, and a capability never decides when it dies](249-a-running-address-space-stays-nameable.md) |
| 250 | DECIDED | [An image names its distribution's package index, at `basalt.nifeos.org`, and a package's bytes may live anywhere](250-images-name-one-package-index-and-the-bytes-may-live-anywhere.md) |
| 251 | DECIDED | [Restarting the machine is a kernel object the progenitor hands out](251-restarting-the-machine-is-a-kernel-object.md) |

Two blocks that lived among the decisions are not decisions and moved out with the split, the same
way milestone 76 moved four essays out of the roadmap: [the open design
ideas](../open-design-ideas.md), which are proposals parked against a trigger rather than choices
made, and [the original eleven-milestone plan](../original-milestone-plan.md), which the roadmap
backfilled into `design/roadmap/` and which is kept here as the record of what was planned before
anything was built.

## Reading

- The seL4 manual, and Klein et al., *seL4: Formal Verification of an OS Kernel* (SOSP'09)
- Liedtke, *On µ-Kernel Construction* (SOSP'95): why Mach was slow and why that was not a law
- The xv6 book (MIT, ~100pp) for how a real Unix-shaped kernel is structured. Read it as the
  road not taken (§10), not as a template.
- `rust-raspberrypi-OS-tutorials` for the aarch64-specific mechanics
- OSDev wiki as a reference, not a tutorial
- *Operating Systems: Three Easy Pieces* for the theory
