---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: 121
machine_requirements: none
specific_machine: none
needs_person: yes
---
# A read-only port range still drives the hardware

Raised by lane/633-outsider-2, milestone 633 (an outside agent attacks the confinement claim)'s
second pass, on 2026-10-05 (UTC). Title and slug provisional. It needs a person because the answer
is the x86 port syscall surface, which DECISIONS §121 (what a device capability is when the device
has no page: x86 port I/O) owns.

**Reuse:** the rights check answer 1 adds is the one every other object's syscall arm already
performs (`cap.rights.allows(Rights::WRITE)` in `kernel/src/syscall.rs`), applied at the port
grant's insert site; the test reuses `x86_port_tests::build_child` and the stub builders in
`kernel/src/user/x86_programs.rs`, with one builder added beside them. `cycle_counter_grant` was
considered as a model for "a creation-time grant that ignores rights" and rejected, because it
gates a read-only counter and this gates a device write.

## The defect

On x86_64 a `PortRange` capability is enforced by `Thread::port_range_grant` and the TSS I/O
bitmap the context switch installs from it, not by the capability table. The one place a port
capability enters a thread, `sched::thread_control_block_insert_from`, installs that grant for any
`PortRange` object and never reads `cap.rights`. So a capability narrowed to `READ`, or to no
rights at all, opens the bitmap exactly as a `WRITE` one does.

The rest of the tree treats `WRITE` as the right that drives a port. `x86_port_tests::build_child`
grants the driver `WRITE` with the comment "the rights a driver gets", and `component_plan` maps a
`Use` component to `WRITE` and a `Serve` component to `READ`. A supervisor that hands a component a
`READ` view of a range, to let it observe and not drive, finds the component drives the hardware.
A port write is a device command, so the gap is authority to act.

`notes/confinement-claims.md` rows 27 to 29 say what holding, revoking and deleting a port
capability mean. None says what its rights mean, which is why the test suite was green.

## The failing test

`system_tests::user::x86_port_tests::a_read_only_port_capability_must_not_grant_port_output`,
committed in the lane's pull request before any fix. It builds a child whose only `PortRange` is
`READ`, executes one `out` through `x86_programs::port_out_then_exit`, and asserts the child faults
at the `out`. On the tree as of 2026-10-05 the supervision message is `[EVENT_EXIT, ..]`: the `out`
was permitted. The test is opt-in (`run_was_filtered`), so the default suite stays green; run it
with `script/test --arch x86_64 --test a_read_only_port_capability`. It carries
`Falsification: unfalsified` because it pins a live defect rather than guarding a passing claim.

## The design question

The TSS bitmap is one bit per port and cannot grant `in` without `out`. So "`READ` means `in` only"
is not implementable on this mechanism, and the question is what a non-`WRITE` port capability
should grant. Three answers, with a recommendation:

1. **Nothing.** A `PortRange` without `WRITE` installs no grant. `READ` and `GRANT` keep their
   table meanings (`GRANT` already gates `REVOKE` and delegation). This is one `if` at the insert
   site, matches `component_plan`'s reading, and makes the failing test pass. Recommended: it is
   the smallest change, it is what every existing consumer already satisfies (every driver is
   granted `WRITE`), and it is the answer the capability model gives for every other object.
2. **Everything, by design.** Declare that a port range is an object whose only sub-right is
   `GRANT`, and that holding it is the authority. Then the test is wrong, `component_plan`'s
   `Serve => READ` must never be applied to a port, and a `BUGS` entry says so where the rights are
   read. Cheaper still, but it makes `READ` on this object a lie a reader has to know about.
3. **Split the object.** An `in`-only and an `out`-only `PortRange` kind. Not implementable on the
   TSS without trapping every port access, which is the cost this mechanism exists to avoid.

Whichever answer is chosen, the sentence belongs in `notes/confinement-claims.md` as a row: "port
I/O honours the capability's `WRITE` right" for answer 1, or its negation for answer 2.

## Acceptance

- The ruling recorded, as a `design/decisions/` section or an amendment to §121.
- The failing test either passes (answer 1) or is rewritten to assert the ruled behaviour and the
  `BUGS` at the insert site says why (answer 2).
- A `Falsification:` record for the test once it guards a claim, replayed on x86_64.
