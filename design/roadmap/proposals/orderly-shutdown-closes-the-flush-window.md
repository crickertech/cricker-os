---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: 805
decision_dependencies: 251
machine_requirements: none
specific_machine: none
needs_person: no
---
# Orderly shutdown closes the flush window

**Reuse:** the design is borrowed from the prior art below; no code is taken, because nothing
outside a service manager can own the order, and nife's would be written here.

calef asked for this on 2026-10-06 (UTC), ruling on #1783, milestone 805 (`reboot` at the prompt).
He approved the flush-only binding (option A in §251 (restarting the machine is a kernel object the
progenitor hands out)'s amendment) and asked that option D be filed as follow-on work. Written by
lane/805-reboot, which built nothing for it.

## The gap

`reboot` sends `fs::SYNC` on its flush-only capability, waits for the reply, then invokes the reboot
object. A write another job makes after that reply and before the reset is lost on a device with a
volatile write cache. `components/src/reboot.rs`'s `BUGS` records it. The window is short, and it
only holds a write from a job somebody left running in the background, but no flush issued by one
program can close it. Something has to stop the writers first.

## What closes it

An orderly shutdown. Before the reset, the reboot path tells every stateful server to stop taking
writes, flush and acknowledge. Only after every acknowledgement does it invoke the reboot object.
That is the only option in §251's amendment that closes the window rather than narrowing it.

It needs an owner for shutdown order, and neither `reboot` nor the progenitor is the right one.
`reboot` should not know which servers exist. The progenitor holding that list is the god-process
drift calef ruled against on the same pull request. The owner is a service manager: the process that
knows what was started, in what order, and what depends on what. nife has none yet. Its supervision
tree (`root_supervisor`, `sub_server_supervisor`) restarts what dies and does not order a stop.

So this proposal has two halves, and the first may be its own milestone:

1. A service manager owns start and stop order for the stateful servers (the file server, the
   block servers, the system log once it is durable).
2. Shutdown is a request to it. `reboot` asks it to stop the system, it stops servers in reverse
   dependency order and waits for each to flush and acknowledge, and then the reboot object is
   invoked. The flush-only capability stays as the last step, or goes, depending on whether the
   file server's own stop includes the flush.

## Prior art (recalled, not re-read)

- Fuchsia's `component_manager` stops components in dependency order, and `fshost` flushes on
  `fuchsia.process.lifecycle` `Stop` before power control resets.
- systemd stops units in reverse dependency order, remounts file systems read-only, syncs, then
  resets.

## Open questions for whoever promotes it

- Who may ask the service manager to stop the system, and is that the reboot object's holder or a
  separate capability?
- What a server that does not acknowledge in time gets: a timeout and a forced reset, or a refusal.
- Whether power-off (excluded from milestone 805) rides the same request.
