---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A mapping cannot outlive its frame's revoke

Raised 2026-10-04 (UTC) by the revocation-race lane (#1640), which closed the delegation side of a
revocation window and recorded the use side without driving it: `PageFrame::MAP` and
`AddressSpace::MAP_INTO` read the frame, then map and record, and a sweep unmaps only what its log
held when it scanned, so a late mapping survives. Fatal risk 7's "a revoked frame is unreachable"
holds for capabilities and not yet for mappings, and under `MemoryRegion::DESTROY` the survivor is
a mapping of a page the allocator reuses (DECISIONS §13 (capability revocation and untyped
reclamation), which named it in 2026-07 as "the one honest race").

The work: drive a sweep into each map path with the revocation-race lane's seam, prove it red, close
it without changing the ABI (a `MAP` that loses its source answers as if it had started after the
revoke), and leave a replayable falsification per path.
