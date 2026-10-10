---
status: PROPOSED
raised: 2026-10-09
milestone_dependencies: 384, 595
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A std program at the prompt holds the network and a name resolver

Raised 2026-10-09 (UTC) by milestone 801 (packages over the internet)'s lane, which built a std
program resolving a name through `std::net` and fetching the package index over TLS, and could only
start it from the kernel test harness.

## The gap

Three pieces exist and are not joined at the prompt.

- The name resolver runs only when a test starts it. Milestone 384 (in a capability system the
  resolver is a grant)'s BUGS gave starting it from the lease to 801. 801 built the client half
  and left this, because nothing at the prompt could use it.
- A std program cannot hold the network at the prompt. Milestone 595 (the shell runs a `std`
  program)'s BUGS: the progenitor does not mint the socket frames' budget slot 3 needs, so
  `grant_plan` refuses a std manifest that declares the network.
- A std program's resolver is a badge at `std_runtime_protocol::RESOLVER_SLOT` (slot 9,
  provisional), placed today only by `system_tests`' `std_service`.

`jig` (milestone 809 (the package client becomes a program)) needs all three to reach
`basalt.nifeos.org` by name, so rung 3c's exit criterion on xenon waits on this.

## The work

1. The progenitor starts `name_resolver` once the lease arrives, told the lease's name server
   (`socket_protocol::lease`'s second word), as the test harness does with the runners' peer.
2. The progenitor mints a socket frames' budget for a std program whose manifest declares the
   network, and places it at slot 3. 595's `a_std_program_declares_only_what_the_std_layout_can_hold`
   then admits the network.
3. A manifest names the zone it may resolve; the progenitor grants a fresh badge that zone and
   places it at slot 9. Whether the zone is the manifest's word or the owner's is a fork: a manifest
   can only ask, and §252 (a resolver grant is one zone per client badge) says a spawner grants.
4. A `script/swish-check` line: an in-tree std program run at the prompt resolves a name in its zone
   and is refused one outside it, on all three architectures.

Reuse: everything named above is in the tree; this is wiring.

## BUGS

- Item 3's fork is real. `jig` fetching bytes from anywhere wants the root zone, which
  `notes/packages/the-index-format.md` (Q5) recommends and which an architect has not ruled.
