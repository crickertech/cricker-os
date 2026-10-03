---
status: PROPOSED
raised: 2026-09-24
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# Every client of a network stack shares its socket numbers

Raised by milestone 590 (the booted system starts its network
stack), whose number is provisional, which gave programs at the prompt a `WRITE` view of one
`net_stack` endpoint and had to say in `caps` what that view does not narrow.

The fix changes `crates/socket_protocol`, which two programs agree on, so it is
a wire format and calef's call.

## The finding

`socket_protocol::req(op, sid)` names a socket by `sid`, an integer below `MAX_SOCKETS` (6), and
`net_stack` keeps one table of them for every caller of its endpoint. Nothing ties a `sid` to the
client that opened it. So two programs holding the network capability at once can each send, read
and close the other's sockets, and one that attaches a page to a `sid` another is using redirects
that socket's bytes into its own page. Under the kernel harness this never mattered, because every
stack had one client. At the prompt it is the difference between "may reach the network" and "may
reach every connection any other program on this machine has open", and `caps` now prints the
second sentence because it is the true one.

## Options

1. **A badge per client.** The progenitor mints each declaring child its own badged view of the
   stack's endpoint, and `net_stack` keys its socket table by badge. Needs endpoint badges, which is
   a question about §10 (process model: capability-based, microkernel) before it is one about
   sockets.
2. **An endpoint per socket.** `OPEN` answers with a fresh endpoint capability for that socket, the
   `fs_subtree_caretaker` shape; the `sid` disappears from the wire. The larger change and the one
   that matches how this tree already narrows files.
3. **A stack per client.** One `net_stack` per declaring job. No wire change, but a NIC can back one
   stack, so this only works behind a multiplexer that does not exist.

Recommendation, from reading rather than measurement: option 2, because it is the shape the tree
already uses for the analogous file case (a narrowed endpoint per grant), and a wire change is owed
either way. What each costs has not been measured.

**Option 1 is now buildable without its own §10 fork.** calef ruled the analogous filesystem
question on 2026-09-26 (milestone 599 (a frame per filesystem client channel), a frame per filesystem client channel) in favour of badged
endpoint capabilities, which is exactly the "endpoint badges" option 1 says it needs. Milestone 599
builds the badge machinery on the shared `INVOKE` surface (a `BADGE` method to mint a badged
endpoint, and the badge as a fourth `RECV_CAP` return value). So option 1's prerequisite is being
built, and its cost here is `net_stack` keying its socket table by the badge the kernel already
delivers, with no `socket_protocol` change. Whether to take option 1 (reuse the badge) or option 2
(an endpoint per socket) is still open and still a wire decision, but the badge no longer has to be
argued for from scratch.

## Index row

`net_stack` names sockets by a small integer every client shares, so two network programs can
operate each other's connections. Proposed: an endpoint per socket, a `socket_protocol` change.
