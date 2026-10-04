---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The noteless launch that prints nothing

Raised by the lane that recorded the signature in
[the swish-check flake note](../../../notes/swish-check-flake.md). The number is provisional and the
integrator mints it.

## The problem

Twice in four hours on 2026-10-03 and 2026-10-04 (UTC), the aarch64 leg of swish-check saw
`packages/noteless/0.1.0/noteless` answer an empty string. One of the two evicted #1573 (docs only)
from the merge queue. The cause is unmeasured.

## The work

- Reproduce on aarch64 under `helpers/qemu-bounded.sh`, looping the noteless step, and count
  occurrences with a denominator.
- Make the failure self-describing: say whether the launch ran, whether it wrote, and whether the
  console had the bytes.
- Decide the remediation only with that data in hand, and record it in the flake note.

## Done when

The note names the mechanism with a measured rate, and the swish-check failure line distinguishes a
launch that never ran from one that ran silently.
