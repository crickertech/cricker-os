---
status: BUILT
raised: 2026-09-14
built: 2026-10-05
milestone_dependencies: none
decision_dependencies: 185
machine_requirements: none
specific_machine: none
needs_person: no
---
# 404. Composing a process from two capabilities is proved for two verbs and no more

Built 2026-10-05 (UTC) by the §185 build lane, as §185 (what carries the claim that userspace
composes a process from an authority you can count on one hand) ruled: option (a), and the fixture
renamed `process_composition_witness`. Filed 2026-09-14 as an unnumbered proposal by milestone
295 (retire `components/src/builder.rs`)'s lane and numbered 2026-09-19 by milestone 433 (drain the proposal pile to zero, and keep it
there). The full argument as filed, and the four options it priced, are in git history
and in §185. *(Number provisional until the merge queue lands it.)*

## What was built

`fixtures/src/process_composition_witness.rs` holds exactly the two capabilities `builder` held, a
memory region in slot 0 and a report line in slot 1, plus the archive mapped read-only, which is
what the kernel gave `builder` and gives the progenitor. From those it:

1. reads `least_authority_demo` out of the archive by name and parses its ELF,
2. mints a rendezvous out of its own memory for the child to answer on,
3. builds the child through `supervision_protocol::build_child_space`, the loader every composer
   in the tree shares, with one capability (`WRITE` on that rendezvous) in the child's slot 0,
4. maps a frame of its own into the same space and is refused the same address twice, which keeps
   milestone 19b (run a real workload)'s break-before-make claim,
5. configures the thread at the child's entry and starts it with an input,
6. and receives the input squared on the rendezvous it minted.

The verdict is one word with a bit per step, in `capability_witness_protocol::process_composition`.
`system_tests::user::tests::a_process_composed_from_two_capabilities_runs_in_the_space_it_built`
asserts the whole word, on aarch64, riscv64 and x86_64, under `script/test` on every pull request.
It waits for the verdict with a 30-second deadline, so a child that never runs fails as a sentence
rather than as a watchdog dump.

It carries a replayable falsification:
`system_tests/falsifications/user.tests.a_process_composed_from_two_capabilities_runs_in_the_space_it_built.patch`
makes `ThreadControlBlock::START` from userspace report success and start nothing. Every step up to
`STARTED` still passes, and only the deadline catches it, which is the step this milestone added.

## What this closes

The gap was a join, not a hole. Two verbs were proved from userspace at a two-capability floor, and
the whole sequence from the kernel side
(`a_process_can_build_start_and_run_a_child_thread`, which calls the kernel entry points directly).
`builder` was the only thing that was both, and nothing on a pull request ever ran it. Now one
fixture is both, on every pull request.

Milestone 19b's "nothing runs in the space it built" reading no longer holds, which §185 chose
knowingly. The fixture's header says so; 19b's account stays as it was written.

## The rename

Performed in its own commit, under the procedure in `design/naming.md`. The fixture's `Name:` block
lists what moved and what kept `address_space_witness` as an account. `script/names` refusals held
at 400 across it. The test wiring module moved with it, to
`system_tests/src/user/process_composition_service.rs` (a derived name, provisional), and the test
was renamed for the claim it now makes.

## Follow-on

- **Recorded.** The witness's receive has no deadline, so under a broken kernel it stays parked
  until the test kernel exits. In `fixtures/src/process_composition_witness.rs`'s `BUGS`;
  supervising the child would cost the floor.
- **Recorded.** The child is loaded unmeasured, as `builder` loaded it, in the same `BUGS`.
- **Milestone 794.** Milestone 794 (one progenitor-shaped spawn for the system tests). The harness's progenitor-shaped spawn is the fifth copy of one sequence in
  `system_tests`: `design/roadmap/0794-one-progenitor-shaped-spawn-for-the-system-tests.md`.
- **Refused.** Option (c), a host-side proof that a capability set suffices for a verb sequence:
  §185 did not choose it. It produces a proof but does not witness the kernel, and stays a
  possible addition beside this one.

## Index row

A process composed from two capabilities now runs in the space it built, on every pull request.
`process_composition_witness` (renamed from `address_space_witness`, calef's ruling in §185) holds a
memory region and a report line, reads a child out of the archive by name, builds its space through
the shared loader, keeps milestone 19b's break-before-make probe in that space, starts the thread
and receives its answer. The test asserts the whole verdict on all three architectures, waits with a
deadline, and carries a falsification that breaks userspace `START`.
