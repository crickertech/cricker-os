# The network stack as a confined component (milestone 30)

Milestone 30 is three pieces in order: multi-queue transport confinement, a userspace virtio-net
driver behind it, and a TCP/IP stack (smoltcp) in a net server speaking a capability-shaped socket
contract.

Pieces 1 and 2, and phase A of piece 3, are built on both ISAs. The kernel confines the NIC's
DMA to N queues, a userspace driver runs it, and smoltcp takes a DHCP lease over it. Their records
are in [the-driver-and-dhcp.md](net/the-driver-and-dhcp.md). What a client uses is the socket
contract below, decided in §25 (socket identity) and built as phase B.

## Piece 3 phase B: the client-facing socket contract (built, both ISAs)

The §25 contract, so a process other than net_stack can open sockets. net_stack, after DHCP, serves requests
on a `Stack` endpoint; a client holds `WRITE` on it plus its own untyped budget. Files:
`crates/socket_protocol/src/lib.rs` (the wire format), the serve loop in `components/src/net_stack.rs`, and the client in
`components/src/socket_test_client.rs` (a module of the net_stack binary, dispatched by the entry role, see the archive
note below).

### A socket is a socket id

Open returns a small integer, carried in the request word of every
later call; the per-connection shared frame is the real granted resource, delegated once at
open via `SEND_CAP` and mapped by net_stack at a per-socket VA. No ambient network: the client acts only
through the `Stack` capability it was granted, and bytes cross in the shared frame, never in a
message.

### Operations

`ATTACH_FRAME` is a `SEND_CAP` (it carries the frame). The rest are `CALL`s (which
mint the reply cap net_stack answers on), the socket id packed into the request word: `OPEN_UDP` /
`OPEN_TCP`; `SENDTO(len)` and `RECEIVE() -> len` for UDP (destination and payload in the shared
frame); `CONNECT` / `SEND(len)` / `RECEIVE()` for TCP; `CLOSE`. A blocking `RECEIVE` is net_stack driving the
smoltcp poll loop (WAIT on the NIC interrupt) until the socket has data, then replying, the disk
driver's discipline one layer up.

### Frame layout, pinned

One data region reused per operation, NOT a split TX/RX ring: the
phase-one contract is one *synchronous* exchange per `CALL` (the client blocks in the CALL while
net_stack drives the network), so a request's payload and its reply never coexist. A split ring becomes
necessary only with asynchronous or streaming sockets, deferred with the concurrency model.

### Concurrency model, phase one

net_stack is single-threaded, with one synchronous exchange per request. net_stack
blocks on the `Stack` endpoint between requests and drives the network inside handling one. This
suits the `std::net` PAL's blocking calls; concurrent connections and listening sockets want either
userspace threads (the TCBs of milestone 19c (run a real workload)) or a select-like wait, the phase-two extension.

### One binary, one archive entry

The client rides in the net_stack binary (a nonzero entry role runs
it) rather than a separate binary, because the nifefs archive directory held at most 15 files at
the time and the initrd was already near that ceiling. (The ceiling is `nifefs::MAX_FILES`, 76
since 2026-08-01; see [nifefs.md](nifefs.md). The decision stands on its own merits, but the
pressure behind it is gone.) A subtlety worth recording: net_stack reports its DHCP
lease with a *blocking* `send`, so the spawn service drains that report before returning, or net_stack
never reaches its serve loop and the client's first request hangs. That was the one real bug in
bring-up, caught by a watchdog hang.


## The inbound half: the guest can be connected to (milestone 107)

Everything above is the guest as a client. The TCP gate connects out to a slirp `guestfwd` peer, the
UDP gate sends a request, DHCP is a client protocol. nife could reach the network and could not
be reached, and the contract had no listen verb to fix that with.

Milestone 107 adds `LISTEN` and `ACCEPT` (`crates/socket_protocol`, opcodes 9 and 10, **names
provisional**), the smoltcp side in `components/src/net_stack.rs`, and a gate in which a **host process
connects into the guest** and gets an answer the guest composed. Two design questions came with the
verbs, and neither was copied from POSIX.

### EXAMPLES: serving a port, end to end

#### Spawn side

Whoever wires the pair decides the inbound authority, and the client never asks for
it. `socket_protocol::listen_grant(lo, hi)` packs an inclusive range into the one word `arg2` carries:

```rust
// A stack whose client may listen on 7778 and nothing else.
let report = virtio_service::start_net_stack(
    image,
    NET_TEST_TCP_ACCEPT,                                    // which exchange the client drives
    false,                                                  // mmio, not PCIe
    socket_protocol::listen_grant(NET_LISTEN_PORT, NET_LISTEN_PORT),
)?;

// A stack that serves nobody inbound, which is every other net test in the tree.
let report = virtio_service::start_net_stack(image, NET_TEST_TCP_ECHO, false,
                                             socket_protocol::NO_LISTEN_GRANT)?;
```

#### Client side

Bind the port, then accept into a *different* socket id that already has a frame.
The listener never gets a frame, and `ACCEPT` refuses to install a connection at the listener's own
id:

```rust
// 1. Bind. No frame is attached anywhere yet, because a listener carries no bytes.
match call(STACK, req(OP_LISTEN, LISTEN_SID), 7778).0 {
    LISTEN_GRANTED => {}
    LISTEN_DENIED  => /* this stack was never granted 7778: ask the spawner, not again */,
    LISTEN_IN_USE  => /* somebody already holds it: pick another port */,
    _ => unreachable!(),
}

// 2. The connection id is what carries data, so it is what gets the frame.
attach_frame(CONN_SID);

// 3. Accept, use, close, repeat. The listener re-arms inside ACCEPT, so this loop keeps working.
loop {
    if call(STACK, req(OP_ACCEPT, LISTEN_SID), CONN_SID).0 != REP_OK { break; }
    let (len, _) = call(STACK, req(OP_RECEIVE, CONN_SID), 0);
    // ... read the request out of the frame at FRAME_VA + OFF_PAYLOAD, write the answer back ...
    let _ = call(STACK, req(OP_SEND, CONN_SID), reply_len);
    let _ = call(STACK, req(OP_CLOSE, CONN_SID), 0);   // the listener is untouched by this
}
```

`components/src/socket_test_client.rs::tcp_accept_inbound` is that sequence with the assertions in it.

#### Running the gate

It is part of the ordinary suite and needs no host setup; xtask picks a free
loopback port, hands it to the runner as `NIFE_HOSTFWD_PORT`, and runs the prober thread itself:

```console
$ script/test                                  # both ISAs, inbound gate included
$ cargo xtask test --arch aarch64              # just the aarch64 leg
```

A plain `cargo xtask run` sets no `NIFE_HOSTFWD_PORT`, so nothing binds a port on your machine
outside a test run.

## The appendices

Everything else that used to live on this page moved into [`notes/net/`](net/README.md) on
2026-09-25 (UTC), verbatim apart from the links that had to follow it, under §212 (a prose budget).
None of it is needed to use the contract.

| appendix | what it holds |
|---|---|
| [the-driver-and-dhcp.md](net/the-driver-and-dhcp.md) | Multi-queue confinement, the virtio-net driver and its DHCP proof, the smoltcp pin, and smoltcp's first lease. |
| [prior-art-and-the-contract.md](net/prior-art-and-the-contract.md) | seL4, Fuchsia, Plan 9, POSIX, systemd and Capsicum, read before the contract; where this differs from seL4; and the §25 design record. |
| [the-outbound-gates.md](net/the-outbound-gates.md) | What the UDP and TCP gates prove, the ephemeral-port fix, the riscv lost wakeup, and why the UDP gate stopped using the host's resolver. |
| [the-inbound-half.md](net/the-inbound-half.md) | Why a listener is not a connection, who grants a port, what `ACCEPT` re-arming buys, and the host-to-guest gate. |
| [the-inbound-check.md](net/the-inbound-check.md) | The inbound check's intermittent red, as it was investigated before the cause was found. |
| [std-tcp-listener.md](net/std-tcp-listener.md) | `std::net::TcpListener` on this contract, and why the prober requires three rounds of four. |
| [memory-and-reclamation.md](net/memory-and-reclamation.md) | How net servers ran the aarch64 test boot out of memory, and what reclaiming them returned. |

## BUGS

- The DMA region is one 4 KiB page, so the MTU is 576 and a full 1514-byte frame does not fit. See
  [the-driver-and-dhcp.md](net/the-driver-and-dhcp.md).
- `net_stack` handles one synchronous exchange at a time. A listener serves connections one after
  another, never two at once, and its backlog is one connection deep. See
  [the-inbound-half.md](net/the-inbound-half.md).
- While one of smoltcp's timers is pending, `net_stack` yields and polls instead of sleeping, so it
  spins a hart until the timer is due. The clean fix is a timed wait. See
  [the-outbound-gates.md](net/the-outbound-gates.md).
- A listen grant belongs to the `Stack` endpoint, not the client, because an endpoint carries no
  sender identity. Two clients sharing one endpoint share its grant. See
  [the-inbound-half.md](net/the-inbound-half.md).
- An accepted stream reports its peer as `0.0.0.0:0`, because `OP_ACCEPT`'s reply carries no peer.
  Both fixes change what two programs agree on, so both are calef's. See
  [std-tcp-listener.md](net/std-tcp-listener.md).
- `virtio::MAX_DEVICES` never reuses a slot, and a dead net service's DMA page and shadow ring are
  not returned. See [memory-and-reclamation.md](net/memory-and-reclamation.md).
- No artifact says which components may serve inbound ports, where a static seL4 system description
  would. Nothing in `crates/socket_protocol` is machine-checked. See
  [prior-art-and-the-contract.md](net/prior-art-and-the-contract.md).

### BUGS: the check fails intermittently, and the mechanism is still not identified

*Identified 2026-09-24: `read-failed` is `EINTR`, a signal on the held read, and the prober now reads
again instead of dropping the connection. The errno, the base rate and the fix are in
[the inbound EINTR appendix](load-sensitive-assertions/inbound-eintr.md). The history is kept as it was
written, in [the-inbound-check.md](net/the-inbound-check.md).*

### What is still not proven, and what is deliberately out of scope

- **`std::net::TcpListener` is bound** (milestone 64, 2026-08-18; this bullet used to say it was
  still `Unsupported`). The prediction above was right in every particular: `bind` is `OP_LISTEN` on
  a client-allocated socket id, `accept` is `OP_ACCEPT` into another one the PAL allocates and
  attaches a frame to, and the std client's stack is now spawned with a real listen grant. See "The
  std client is a server too" in [std-tcp-listener.md](net/std-tcp-listener.md), and notes/std.md.
- **Only the mmio transport carries the inbound gate.** A PCIe twin would need a second host port,
  and the transport is orthogonal to the accept path, which the outbound gates already prove over
  both buses. This is the same reasoning that retired the PCIe DNS variant.
- **Inbound UDP is now built, and it is a grant of its own** (milestone 55's mDNS stack half; this
  bullet used to say "not built"). `BIND_UDP` claims a fixed UDP port the way `LISTEN` claims a TCP
  one, checked against a UDP bind grant the spawn site packs into the high half of the same
  spawn word the listen grant rides in (`socket_protocol::udp_bind_grant`; the halves are independent
  authorities, and the zero word still grants nothing anywhere). It answers with `LISTEN`'s own
  vocabulary because the three outcomes are properties of claiming a port, not of TCP. In the same
  change, a UDP `RECEIVE` reply now carries the datagram's source endpoint in the frame's dst
  fields (dead space on a reply), because a responder must see who asked and RFC 6762 §6.7 turns on
  the querier's source port; the TFTP gate consumes it by ACKing to the DATA packet's reported
  source, which is what TFTP's TID scheme wanted all along. The stack also joins 224.0.0.251 at
  startup (smoltcp's `multicast` feature, switched on in the same milestone). The whole story,
  including what QEMU could and could not prove about multicast, is notes/mdns.md's. **The
  multicast half was retired on 2026-09-15** (milestone 298): the group join, the feature and the
  runners' injection hub went with the responder, so nothing proves multicast now. The UDP bind
  grant and the `RECEIVE` source endpoint stayed, still proved by the accept test and the TFTP gate.
