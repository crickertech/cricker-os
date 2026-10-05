---
status: PROPOSED
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A destroyed region cannot free the root a running thread walks

Raised by the page-tables-outlive-destroy lane on 2026-10-05 (UTC), while cutting intermediate page
tables out of live walks when their region is destroyed. The root is the one table that cannot be
cut, so it needs a different answer, and every answer changes what `MemoryRegion::DESTROY` does to a
thread. That makes it a question for §16 (object revocation: reclaim the objects a process built)
and an architect's call.

**Reuse:** none exists to take; this is §16's teardown path in the kernel, where §46 (thin
primitives or whole subsystems) says write it. Each option below reuses an existing mechanism (A the
resident kill, C the `CONFIGURE` refusal shape).

## The hole

`RETYPE_OBJ(ADDRESS_SPACE)` retypes a space's root out of region R. `CONFIGURE` binds that space to
a TCB, and nothing requires the TCB to come from R. `sched::reclaim_region(R)` reaps threads whose
TCB page is in R and unbound spaces whose root is in R (`user::reap_address_spaces_in_region` reads
only `USER_SPACES`). A bound space whose thread lives elsewhere is neither, so `DESTROY` succeeds,
`unpin` clears R's pin, and R's pages, the root included, go back to the allocator while the thread
still runs on that root. The next owner of the page writes that thread's top-level translations,
which is fatal risk 7's confinement claim broken outright.

**Is the premise true?** Driven once on aarch64 by a scratch test that was not kept: a kernel
thread adopted a space built by `user_address_space_create(R)`, the test called
`sched::reclaim_region(R)`, and it answered `Ok(())` with R no longer resolving and the thread still
on root `0x4001c000`, inside R's old span. The userspace route (`CONFIGURE` with a TCB retyped from a
second region) is reasoned from `configure_thread_control_block`, which checks only that the TCB is
an embryo.

**Who builds this shape today?** Nobody. Every `ADDRESS_SPACE` retype in the tree takes its TCB from
the same region: `supervision_protocol::build_child_space` (`build_ut` for both),
`fixtures/src/job_mix_task.rs` and `os_primitives_benchmarker.rs` (the child's region for both),
`system_tests/src/user/reap_tests.rs`, and `address_space_witness`, which binds nothing. So the hole
is reachable only by a program that sets out to build it.

## The options

All three keep the intermediate-table fix as it is; they differ only in the root.

**A. `DESTROY` treats a thread whose bound space is rooted in the region as a resident.** The finish
and refuse phases of `reap_region_objects` test "TCB page in the span"; this adds "or the bound
space's root in the span". Such a thread is killed exactly as a resident is (§16's amendment that
`DESTROY` force-kills a live resident: refuse this pass, kill, the retry reclaims). What changes: a
holder of R can end a thread whose TCB it does not own. It already owns the memory that thread
translates through, so the thread could not have kept running whatever the answer.

**B. `DESTROY` refuses, and kills nothing, while a bound live space is rooted in the region.** The
lender cannot get its memory back until the borrower's thread ends of its own accord. That is the
pre-amendment shape that §24 (interrupting the foreground process)'s `^C` needed removed: a runaway
that never finishes would pin R forever.

**C. `CONFIGURE` refuses a space whose root is not in the TCB's region.** Rung one of the ladder:
the shape that cannot be reaped stops being constructible, and the TCB-based reap already covers
the space. Every binding in the tree today passes. It adds a refusal to `CONFIGURE`, which is a new
answer on the syscall surface, and it forbids a split some future loader might want (a supervisor
holding the TCB budget, a child holding its own space).

## The seven questions

1. **Considered and lost.** Cutting the root, the answer the intermediate tables got: nothing points
   at a root, so there is no entry to clear; the closest equivalent is switching the thread to the
   reserved root, which leaves a thread that faults on its next instruction and is option A with
   extra steps. Recording the root in the mapping log like a table: the registry already knows
   every root, and what is missing is a reaction, not a record.
2. **What the tree does in the analogous case.** A thread whose TCB is in the region is killed and
   the destroy retried (§16's force-kill amendment; `reclaim_region`'s `BUGS` calls a refused
   reclaim destructive). Option A extends that to the second page a thread cannot live without.
3. **Prior art.** From memory, not re-read for this file: in seL4, revoking an untyped deletes
   every object derived from it, including a VSpace root, and a TCB whose VSpace capability is
   deleted is left with a null root and faults to its handler on its next run. That is closest to
   option A: the thread survives as an object and cannot run.
4. **Premise.** True on aarch64 as above; the other two ISAs share the code path
   (`reclaim_region`, `reap_address_spaces_in_region`) and were not run.
5. **Cost.** A: one more comparison per thread in two scans that are already a walk of the thread
   table, on `DESTROY` only; not measured. B: the same comparison, plus a pinned region for as long
   as the borrower's thread lives. C: one region lookup per `CONFIGURE`, which is spawn-time; not
   measured.
6. **Reversibility.** All three are §16 semantics, which every future program is written against.
   Nobody has acted on the current behaviour, because nothing in the tree builds the shape.
7. **Effort.** Each is small, and C is the smallest. That is an effort argument, said as one; it is
   not a reason to prefer C.

Options only, because this is on the irreversible list. What is blocked until it is answered: only
this hole. The intermediate-table fix landed without it.
