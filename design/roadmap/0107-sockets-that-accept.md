---
status: BUILT
raised: 2026-08-04
built: 2026-08-04
---
# 107. The socket contract learns to accept

Merged 2026-08-04 (pull request #117; `socket_proto`'s listen verbs are on main). The status said IN-PROGRESS for eleven days after the merge, found 2026-08-15 while ranking ready work: §76's defect class, and the finding that un-hid milestone 54`. Raised 2026-08-04 from `notes/net.md:250`, which states the gap and names
the test that would close it. Two milestones already on the roadmap need it and neither block says
so, which is why it is here rather than folded into one of them.

The finding. The socket contract has no listen verb. `notes/net/the-outbound-gates.md`, under "Not proven by the
gate": "inbound connections. A `LISTEN`/`accept` verb plus a QEMU `hostfwd` (host port to guest)
is the way to test the guest accepting a connection, and that is future work; the contract has no
listen verb yet."

Everything proved so far is outbound. The TCP gate opens a socket, connects to a slirp `guestfwd`
echo peer, sends, receives and closes: a complete round trip with the handshake and the teardown in
it, and every byte of it initiated by the guest. The UDP gate is a DNS query. The DHCP bring-up is a
client. nife can reach the network and cannot be reached.

**Why it is not a detail.** Milestone 54 (a network file service a Mac can actually mount) and
milestone 55 (Time Machine over SMB3 with Apple's extensions) are both **servers**. The mount is a
Mac connecting to us. Neither block mentions listen or accept anywhere, so the prerequisite is
currently invisible in both, and the first lane on either would discover it after designing
everything above it. That is the failure this entry exists to prevent.

The second limit, which arrives with the first. `notes/net.md` names the concurrency model in
the same breath: "one synchronous exchange at a time, no overlapping connections." A client can live
with that. A file server cannot: SMB3 to a Mac is several connections, and a server that serves one
at a time serves one Mac badly. So "learn to accept" is really two changes, and the second is the
larger one, because it touches how net_stack's single service loop relates to its clients rather
than adding a verb to a protocol.

**What the gate becomes.** `hostfwd` is the mirror of the `guestfwd` the outbound test already uses,
so the shape is known and costs no host setup: QEMU forwards a host port to the guest, the test
connects to it from the host side, and the guest accepts, reads and answers. That keeps the property
milestone 30 earned, a network gate with zero developer setup that runs in CI.

What it costs. A `LISTEN`/`accept` pair in `crates/sink_proto`'s sibling contract widens a
surface milestone 30 kept deliberately narrow ("TCP, UDP, DHCP, no sockets-API mimicry beyond what
the PAL needs"). Widening it is the right call when a real consumer arrives, and two have, but the
discipline that kept it narrow should decide the shape: the question is what a *file server* needs,
not what BSD sockets offer.

## Scope note

Accept is the verb; the concurrency model is the design work. Adding `LISTEN` to a stack that
handles one exchange at a time produces a server that accepts a connection and then cannot accept
the next one. Decide the model with the verb, or the verb is a milestone that ships something
nothing can use.

This also binds milestone 27's `std::net` PAL. `TcpListener` is currently unreachable for the
same reason `TcpStream` was before milestone 30; whether the PAL binding lands here or in 27's
follow-on is a scoping question worth settling early, because the answer decides whether a
gitoxide-style real workload (milestone 99) can serve anything.

## Follow-on

- **Milestone 64.** The `std::net` PAL binding this block flagged as a scoping question. It landed
  in milestone 64 rather than here or in 27: `TcpListener::bind` and `accept` map onto `OP_LISTEN`
  and `OP_ACCEPT`, and rank 21 of the crates.io gap list closed with it.
- **Recorded.** In `notes/net.md`, beside the verbs: the concurrency model is still phase one.
  `ACCEPT` re-arms, so a listener serves connections one after another indefinitely, but the backlog
  is one connection deep and two connections cannot be served at once, because the client blocks in
  one call at a time. Serving several at once wants userspace threads or a select-shaped wait.

## Index row

The contract has no listen verb and the gate proves outbound only: nife can reach the network and
cannot be reached. Milestones 54 and 55 are both servers and neither block mentions it, so the
first lane on either would find it after designing everything above it. The concurrency model (one
exchange at a time) is the larger half
