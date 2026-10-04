---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Fuzz the surface a confined process can reach: the services' decoders, and the syscalls

calef asked for this on 2026-10-04 (UTC), and asked that it be a proposal, not a build. Written by
the proposal lane `lane/fuzz-surface-proposal`. It proposes two milestones, (a) and (b), with
provisional numbers the integrator mints at promotion; it may split them into two files then. Title
and slug are drafts.

## The premise, checked

§60 (fuzzing complements the proofs) put `cargo-fuzz` over parsers of bytes we did not write. Its
six targets in `fuzz/fuzz_targets/` read a device tree, an ELF, a GPT, an HTTP head, a nifefs
archive and a package archive. None reads an IPC message, and nothing makes a syscall. The two
audits that cover this ground read it once: 2026-08-15 (`notes/untrusted-input-audit.md`) and
2026-09-29 (`design/audit-reports/2026-09-29-syscall-surface-whole.md`). A reading does not recheck
after the next change, and the 2026-09-29 audit was followed four days later by three kernel
confinement defects on the same surface.

The census, run on `1a145fcaa`:

- 19 `*_protocol` crates. Most are constants and pack/unpack helpers for a three-word message. The
  decoding that matters happens inline in each server, after the unpack.
- No protocol crate, and no server, is fuzzed. Nothing in the tree derives `Arbitrary` or uses
  proptest, quickcheck or bolero.
- Three decoders are partly proven. `credential_protocol::read` has 3 Kani harnesses (it stays in
  the page). `network_time_protocol::Packet::parse` has 7 (parse is total). `filesystem_protocol`'s
  3 cover rights attenuation, not decoding.
- On the syscall side, `crates/capability` carries 14 Kani harnesses and
  `crates/inter_process_communication` 14 (the endpoint, notification and timer state machines).
  `kernel/src/syscall.rs` and `kernel/src/sched.rs`, which join them to threads and mailboxes, carry
  none that reach the dispatcher.

That last point decides part (b). The three confinement defects found on 2026-10-03 were all in
`kernel/src/sched.rs`: a plain `SEND` delivering badge 0 (#1494), a plain `SEND` handing a
`RECEIVE_CAP` receiver a sender-chosen slot (milestone 634 (a plain SEND received by RECEIVE_CAP never hands the receiver a sender-chosen slot)), and a staged `outgoing_cap` surviving a
plain `RECV` (#1525). None was in the proven crates.

## (a) Host fuzz targets for the services' request handlers

### Ranked by exposure to untrusted senders

| rank | handler | who can send | ready for host? |
|---|---|---|---|
| 1 | `redoxfs_server` (`src/bin/redoxfs_server.rs:413-700`, 24 verbs) | any std program; bound badges are confined clients | the `Server` API is; the per-request dispatch is in the binary |
| 2 | `net_stack` (`components/src/net_stack.rs:227-360`, 11 ops) | any program granted network | no; an EL0 binary with no sans-IO core |
| 3 | `system_log::Log::handle` (`crates/system_log/src/lib.rs:163`) | every program's stdout | yes, pure |
| 4 | `fs_file`, `fs_nameset`, `fs_subtree` caretakers | the clients a grant fronts | no; inline in `components/src/` |
| 5 | `compositor::serve_frame` (`components/src/compositor.rs:253-306`) | any graphical client | the clip, `crates/compositor/src/lib.rs:516`, is |

Lower: `login` (terminals only), `credentialer` (login and the provisioner only, and `read` is
proven), `entropy`, `console`. The file server is first because it is a confinement boundary as
well as a parser: a bound badge must reach only its subtree, and #1494 was an escape through exactly
this server's admission.

### What a target is

A session, not a message. The input is a sequence of `(badge, w0, w1, page bytes)` requests applied
to one host-built server, which is how §60's `nifefs_roundtrip` already works (structured input
through `arbitrary`, which `libfuzzer-sys` brings, so no new dependency). Each target carries an
oracle beyond "no panic". For the file server it is the subtree rule: a bound badge is admitted only
to `ROOT` or a handle it minted, and a revoked badge to nothing.

### Measured cost

A scratch target was written and run once, outside the tree (not committed): a four-badge session
against `Server<DiskMemory>` with the subtree oracle above.

- 74 lines. It built against `redoxfs_server`'s library unchanged, with `redoxfs` at
  `default-features = false, features = ["std"]` (the default pulls `fuser`, which failed to build).
- 60 seconds on this machine ran 11,461 sessions at about 190 a second, reaching 1,438 coverage
  edges, with no finding. §60's parsers sustain at least 19,470 a second. The difference is setup:
  every input formats a fresh 4 MiB image. A template image copied per input, through a small `Disk`
  over a `Vec`, should remove most of it. Not measured.
- It cannot reach the binary's own dispatch, so it mirrors the admit-then-dispatch order by hand.
  The real target needs the `match code` moved from the binary into the library as one
  `Server::handle` function. That is about 285 lines moved, not written, and it is also what the
  host tests would want.
- `net_stack` and the caretakers need the same extraction, with no library half to move into today.
  That is the bulk of (a)'s cost, and it is not measured.

Runner minutes: the CI `fuzz` job runs 60 seconds a target. Its median over 25 merge-group runs was
7.0 minutes (milestone 721 (each merge-group CI job has a 20-minute budget)'s table), against a 20-minute budget. Four targets add about four
minutes.

### Proof condition: BUILT when

1. Targets for ranks 1, 3 and 5 are in `fuzz/Cargo.toml`, in `script/fuzz --list`, and in the CI
   sweep, each with a stated oracle.
2. Each target is shown to find one planted defect, and the time it took is recorded. For the file
   server, the planted defect is the fail-open arm that milestone 726 (an unknown badge fails closed in subtree_scope) removed, restored. A target that cannot find
   its plant within ten minutes on a developer machine is not BUILT.
3. Ranks 2 and 4 have targets, or a recorded limitation beside each server saying what extraction
   blocks it.
4. The CI `fuzz` job stays inside its 20-minute budget, measured from the first merge-group runs.

Estimate: one lane. Ranks 1, 3 and 5 are about two days of agent time; rank 2's extraction is
unknown and may want its own milestone.

## (b) A syscall fuzzer

### The hard part

Under QEMU, a fuzzer outside the kernel sees only the semihosting exit and the console. A kernel
panic is visible. A confinement violation is not: #1494, 634 and #1525 each delivered something
wrong and crashed nothing. So the oracle matters more than the generator.

Milestone 615 (a QEMU boot matrix) deferred snapshot-based fault injection because "it fights the
semihosting exit and the icount determinism conventions". Any snapshot design inherits that.

### Prior art, read

- syzkaller describes syscalls in syzlang, with handles as typed resources a call produces and a
  later call consumes. It detects bugs by matching console output (`BUG: KASAN:`, panics, hangs)
  and needs KCOV for coverage. Its one microkernel port, Fuchsia, is documented as incomplete, with
  "coverage feedback is not supported" and "crash parsing does not work reliably"
  (`docs/fuchsia/README.md`).
- seL4: no syscall fuzzer found; its assurance is the proof. The FAQ lists what the proof assumes:
  the assembly, the hardware, TLB and cache management, and the boot code.
- Hubris faults a task that misuses `SEND` or `RECV` rather than returning an error, and idol
  servers answer a malformed message with `REPLY_FAULT`. No fuzzing found.
- Redox: no fuzzing found. Its GitLab code search needs a login, so the scheme repositories were
  not searched.
- HYPER-CUBE (NDSS 2020) boots a small guest that interprets a seeded byte stream as interface
  calls. It is coverage-blind, catches crashes, hangs and assertion failures, and reproduces from
  the PRNG seed. 26 of its bugs were assertion failures.
- Not read, search snippets only: Fuchsia's FIDL fuzzing page (now 404) and the kAFL and Nyx
  repositories.

### Options

1. Port syzkaller. Lost: it needs an executor, descriptions for about 77 ABI constants, a VM
   backend and console parsers, and its oracle is a crash. The defects this surface has actually
   had do not crash.
2. Host model only: `cargo-fuzz` over operation sequences on `capability` and
   `inter_process_communication`. Lost: cheap and needs no kernel change, but those crates already
   carry 28 Kani harnesses, and none of the three recent defects was in them.
3. An in-kernel invariant checker alone. Lost as a first step: a checker with nothing random driving
   it checks what the existing tests already drive.
4. Recommended: an in-guest seeded driver with a shadow model, HYPER-CUBE's shape. One EL0 system
   test program holds a small endowment (two rendezvous, a notification, a timer, a memory region,
   a child thread). From a seed, it issues a random sequence of `SYS_INVOKE`, `SYS_CAP_DELETE`,
   `SEND_CAP` and `RECEIVE_CAP` with random slots, methods and arguments. It predicts each result from
   a model of its own table: rights, badges, consumed Replies. Witness threads endowed with nothing
   assert that they receive nothing and that every badge they see is the one their sender's
   capability carries. A kernel panic ends the run through semihosting as any test does.

Option 4 needs no snapshot, so it does not meet 615's objection. The seed is the reproducer, and
the run is deterministic under icount. A later phase may add an invariant walk at the tail of
`syscall::dispatch`, where `ipc_stack_depth`, from milestone 134 (the register of measures), already hooks, under the same
`any(test, ...)` gating, so no shipping kernel carries it. That is worth building only if phase 1
measurably misses a class.

Cadence follows milestone 616 (constrained hardware fuzzing): a short fixed seed list in the suite,
and a long sweep on the weekly `falsifications.yml` schedule, where a finding is a report and not a
red trunk. Syscalls per second under QEMU are not measured; that is the lane's first number.

### Proof condition: BUILT when

1. The driver runs in the suite on aarch64, riscv64 and x86_64, as §19 (architectural parity is a tenet) requires, with its seed list committed.
2. Six replayable falsifications in `system_tests/falsifications/` break kernel IPC or capability
   code: the five `recv_cap_attack_tests` patches and `revocation_in_flight_tests`. With each
   applied, the driver's committed seeds turn red on aarch64. Any it misses is recorded, with the
   reason, in the driver's `BUGS`.
3. Its wall time in the suite is measured, and the weekly sweep's seed count is chosen from that
   number.

Estimate: one lane, about three days of agent time for phase 1. It lands in `kernel/src/user/tests.rs`
and the QEMU runners, the test-wiring hotspot, so it should not run beside another lane there.

## Reversibility, and what is calef's

- Neither part needs a new syscall or a new method. (b) uses only the existing surface.
- Neither needs a new dependency. `arbitrary` is already in `fuzz/Cargo.lock` through
  `libfuzzer-sys`, and the driver's PRNG is a few lines.
- (b)'s phase 2 is test-build code. Putting it in a release kernel would need a cargo feature, which
  is a name, and calef's call. Nothing proposed here does that.
- The names of the new targets, the driver program and its test are provisional and calef's.
- Everything else is code, CI wiring and a seed list: reversible, and nobody has acted on it.

## Would we choose this at equal cost?

(a): yes. A host target runs two orders of magnitude faster than anything under QEMU and gets
libFuzzer's coverage guidance, and the extraction it needs is what the host tests want anyway.

(b): yes for the oracle, and no for the blindness. If a coverage-guided port cost the same as
option 4, its coverage would be worth having. Option 4 is coverage-blind partly because coverage
under QEMU costs more than it does, and that part of the choice is about effort. The shadow model
would still be chosen at equal cost, because a crash-only oracle would have missed all three
defects that motivated this.

## Index row

Nothing fuzzes what a confined process can reach: the services' request handlers or the syscalls.
(a) puts host fuzz targets over the handlers, file server first, each with an oracle and a planted
defect it must find. (b) runs a seeded in-guest driver whose shadow model of its own capability
table predicts every result, and it must turn red under six recorded kernel falsifications.
