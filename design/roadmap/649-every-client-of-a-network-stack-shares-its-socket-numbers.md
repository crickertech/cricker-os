---
status: NOT-STARTED
raised: 2026-09-24
promoted_from: every-client-of-a-network-stack-shares-its-socket-numbers
milestone_dependencies: none
decision_dependencies: 255
machine_requirements: none
specific_machine: none
needs_person: no
---
# 649. Every client of a network stack shares its socket numbers

Promoted from `design/roadmap/proposals/every-client-of-a-network-stack-shares-its-socket-numbers.md` on 2026-10-03 (UTC). The number 649 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

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
3. A stack per client. One `net_stack` per declaring job. No wire change, but a NIC can back one
   stack, so this only works behind a multiplexer that does not exist.

Recommendation, from reading rather than measurement: option 2, because it is the shape the tree
already uses for the analogous file case (a narrowed endpoint per grant), and a wire change is owed
either way. What each costs has not been measured.

Option 1 is now buildable without its own §10 fork. calef ruled the analogous filesystem
question on 2026-09-26 (milestone 599 (a frame per filesystem client channel), a frame per filesystem client channel) in favor of badged
endpoint capabilities, which is exactly the "endpoint badges" option 1 says it needs. Milestone 599
builds the badge machinery on the shared `INVOKE` surface (a `BADGE` method to mint a badged
endpoint, and the badge as a fourth `RECEIVE_CAP` return value). So option 1's prerequisite is being
built, and its cost here is `net_stack` keying its socket table by the badge the kernel already
delivers, with no `socket_protocol` change. Whether to take option 1 (reuse the badge) or option 2
(an endpoint per socket) is still open and still a wire decision, but the badge no longer has to be
argued for from scratch.

## Ruled, and held on a kernel fork (2026-10-07 UTC)

calef ruled the shape on 2026-10-07 (UTC), recorded on PR #1798 at 15:04Z: **each socket is its own
capability**. The stack mints a badged endpoint per socket on open, and every later call is made on it.
The stack names the socket by the kernel-stamped badge, so `socket_protocol` drops the `sid`. Close
unbinds the badge, and a socket moves to another program by passing the capability. That supersedes
his per-caller-windows ruling of about 03:50Z the same day, and it refuses this block's option 1 as
written ((badge, sid) keying) and its option 3.

lane/649-sockets-are-capabilities (PR #1817) found that the kernel cannot hand the minted
capability over: a reply is two words, and `SEND_CAP` would park the one-threaded stack on any
client that never receives. §255 (each socket is its own capability), PROPOSED and provisionally numbered,
records the protocol change and three ways to deliver the capability: a reply that carries one, a
client-supplied return endpoint, or a progenitor-minted pool. It leans toward the first, which is a
new kernel method and so an architect's call. Nothing is built until it is answered.

Exit, once built (from the review of #1798, finding 8). The pinned test in
`system_tests/src/user/net_confinement_tests.rs` loses its opt-in skip and runs in the default suite
on all three ISAs. Its aarch64 region budget is measured or raised. A replayable falsification
record that restores a client-named socket turns it red.

## Index row

`net_stack` names sockets by a small integer every client shares, so two network programs can
operate each other's connections. Proposed: an endpoint per socket, a `socket_protocol` change.
