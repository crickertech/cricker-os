---
status: BUILT
raised: 2026-10-04
built: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 745. Count the error paths no test reaches

Raised and built 2026-10-04 (UTC) by the lane `lane/error-paths`, at calef's request, as the
measurement before any fault-injection work. The number 745 is provisional until the queue lands
it. *(Title and slug are drafts.)*

## Why

Fatal risk 3 asks whether the tests test anything, and risk 7 whether confinement holds. Both lean
on error paths: a refusal nobody tested may not refuse, and a rollback nobody ran may leak. Nothing
counted them.

## What shipped

- [`notes/untested-error-paths.md`](../../notes/untested-error-paths.md): the method, the answer by
  crate and by kind, the twenty unreached paths that release memory or authority, the blind spot,
  the classifier's measured error rate and the cost. Tables in its appendix.
- `helpers/error_paths.py`: reads an llvm-cov HTML report (the CI `coverage-report` artifact) and the
  unmeasured tree, and prints the tables. Provisional name.
- In the host crates, 586 of 1,150 Result-family error paths (51%) never ran. Of 636 `?`, 490 never
  took their error side. A further 1,159 are in the kernel and services, unmeasured, and so are all
  75 cleanup paths found.

## Follow-on

- **Milestone 774.** Milestone 774 (a page-table allocator that fails on its Nth call). `design/roadmap/0774-a-page-table-allocator-that-fails-on-its-nth-call.md`,
  the recommended first pilot.
- **Milestone 757.** Built as milestone 757 (a test kernel fails a process on its Nth retype), for the 75 cleanup paths
  (approved by calef 2026-10-04 UTC, #1591).
- **Milestone 781.** Milestone 781 (measure kernel and service coverage under QEMU). `design/roadmap/0781-measure-kernel-and-service-coverage-under-qemu.md`, to
  turn the blind spot's count into a measurement.
- **Recorded.** Not reached is not reachable, the Option family overstates, and a bare `.ok_or` is
  unmeasurable: the BUGS section of `notes/untested-error-paths.md`.

## Index row

BUILT on `lane/error-paths`. Half the host crates' error paths never run under a test, mostly `?`
that never took its error side, and every cleanup-after-failure path sits in kernel or service code
no coverage run reaches.
