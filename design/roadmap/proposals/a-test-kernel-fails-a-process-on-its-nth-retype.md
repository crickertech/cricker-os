---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A test kernel fails a process on its Nth retype

Raised 2026-10-04 (UTC) by the lane for milestone 745 (count the error paths no test reaches),
provisional. *(Title and slug are drafts.)*

## Why

Milestone 745 found 75 cleanup paths, error paths whose body gives something back, and every one is
in code no coverage run reaches: 27 in `system_initializer`, 17 in `login`, 10 in `swish`, 13 in the
kernel ([`notes/untested-error-paths.md`](../../../notes/untested-error-paths.md)). They are the
rollbacks fatal risk 7 depends on. `login`'s `connect` and `mint` say in their own comments that some
failures leak capability slots. Nothing fails them on purpose, so nothing says which.

## What to build

A test-kernel feature (in the `system_tests` family, never in a shipping build) that makes a chosen
process's Nth `RETYPE` or `MEMORY_REGION_SPLIT` return its out-of-memory error. A system test then
sweeps N over one service start, `login`'s `connect` first, and checks the process's capability
table count and its region's usage before and after. The kernel already keeps both counters.

## Why it is an architect's call

It adds a knob to the test kernel's syscall path, and the syscall surface is a boundary under §10 (process model: capability-based, microkernel). It
should not change the surface a shipping program sees, but whether a test-only knob there is
acceptable, and how a test names the target process, are not a lane's to decide. Do
[a page-table allocator that fails on its Nth call](a-page-table-allocator-that-fails-on-its-nth-call.md)
first: it needs no decision.
</content>
</invoke>
<invoke name="Bash">
<parameter name="command">cd /Users/calef/projects/nife-worktrees/error-paths && ls design/roadmap/proposals | grep -E 'nth|qemu|retype'; ls design/roadmap/745*; script/roadmap --check 2>&1 | tail -5