---
status: NOT-STARTED
promoted_from: net-stack-seeds-its-generator-from-entropy
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 783. The network stack seeds its random generator from the clock, and TCP sequence numbers come from it

<!-- writing-standards: exception. Granted 2026-10-06 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by the lane for milestone 384 (in a capability system the resolver is a grant) (`lane/384-name-resolver`) on 2026-10-04 (UTC), while reading
`smoltcp` 0.14.0's DNS socket to price putting a resolver inside `net_stack`. Title and slug are
drafts.

## The defect

`components/src/net_stack.rs` builds its interface with `config.random_seed = now()`, the
boot-relative counter. `smoltcp`'s generator (`src/rand.rs`, a PCG32) is the only randomness the
stack has, and `src/socket/tcp.rs` draws every connection's initial sequence number from it. So an
off-path attacker who can estimate how long the machine has been up can predict the sequence
numbers of `net_stack`'s connections, which is what blind TCP injection needs. `notes/entropy.md`
already calls a stream seeded off the counter "predictable to anyone who could guess boot-relative
time", for the same reason.

Ephemeral ports are not affected: `net_stack` assigns them with its own rotating allocator.

## What it would take

`net_stack` holds no entropy capability today. The fix is one: grant it the entropy service's
endpoint at spawn, draw eight bytes before the interface is built, and refuse to start without
them, which is the NTP client's rule (`network_time_client`'s `RPT_NO_ENTROPY`). That is a change to
`net_stack`'s endowment in every spawner of it, so it touches the kernel's test wiring and the
progenitor.

## BUGS

- Not measured how much of the seed an attacker actually has to guess; the counter's rate is known
  and the boot time is the unknown.

## Index row

`net_stack` seeds smoltcp's generator from the boot-relative counter, and every TCP initial sequence number comes from it, so an off-path attacker who can estimate uptime can predict them. The fix is an entropy capability granted to `net_stack` at spawn and a refusal to start without it.
