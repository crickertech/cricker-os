---
status: PROPOSED
raised: 2026-10-07
---

# 255. Each socket is its own capability, and the network stack needs a way to hand it over

*Section number provisional until the merge queue lands it; 254 was the highest on `main` when this
was written. Raised by lane/649-sockets-are-capabilities for milestone 649 (every client of a
network stack shares its socket numbers), on 2026-10-07 (UTC).*

calef ruled the shape on 2026-10-07 (UTC). Building it found that the kernel cannot do one step the
shape needs, so the section is `PROPOSED`: the ruling stands, and the fork below is what has to be
answered before milestone 649 can be built.

## The ruling

Recorded by the maintainer session on PR #1798 at 15:04Z on 2026-10-07 (UTC), superseding calef's
per-caller-windows ruling of about 03:50Z the same day:

> milestone 649's fix makes each socket its own capability. On open, the network stack mints a
> badged endpoint for that one socket (milestone 606's badge model); every later call (send,
> receive, attach a page, close) is made on that capability; the stack identifies the socket by the
> kernel-stamped badge, never by a client-supplied `sid`, so `socket_protocol` drops the `sid`
> argument; close unbinds the badge; a socket can be handed to another program by passing the
> capability. This is §27's directory-capability model applied to sockets.

calef's words: *"Naming socks as capabilities seems right. Can we do that?"*, then *"Yes"*.

## What it changes in `socket_protocol`

This part holds under every option in the fork below.

- The request word is the opcode alone. `req`, `req_sid` and `MAX_SOCKETS` as a client-visible
  number go. A socket is named by the badge on the capability the request arrived on. The kernel
  writes it (`x3` of `RECEIVE_CAP`, §230 (badged endpoint capabilities) and its milestone 613 (a
  system log service: the in-memory half) amendment) and no client can choose.
- The capability a client is spawned with is the **front door**. On it, only the operations that
  make a socket are accepted: `OPEN_UDP`, `OPEN_TCP`, `BIND_UDP` and `LISTEN`. Every other operation
  on the front door is refused, because the front door names no socket.
- A socket capability accepts `ATTACH_PAGE_FRAME`, `SENDTO`, `RECEIVE`, `CONNECT`, `SEND` and
  `CLOSE`. A listener's capability accepts `ACCEPT` and `CLOSE`. `ACCEPT` makes a socket, so it
  hands over a new capability the way `OPEN_TCP` does, and its `target_sid` argument goes.
- The stack mints each socket's badge from a counter that is never reused. `CLOSE` drops the badge
  from the stack's table, so any copy of a closed socket's capability, held by anyone, reaches
  nothing (`REP_ERR`). That is the same fail-closed reading §230's amendment gave `subtree_scope` in
  milestone 726 (an unknown badge fails closed in subtree_scope): a badge the server has no entry
  for reaches nothing.
- The stack holds its own endpoint with `READ` (to serve) and a second copy with `WRITE | GRANT` (to
  mint from). `BADGE` keeps the source's rights, so a copy minted from the `WRITE | GRANT` one gives
  a client what a socket holder needs. That is `WRITE` to use it and `GRANT` to pass it on, and no
  `READ`, which would let a client take other clients' requests off the endpoint. `BADGE` refuses an
  already-badged source, so a client holding `GRANT` on its socket cannot re-badge it into another.
- Cost: one capability slot per open socket in the client, out of `abi::CAPABILITY_TABLE_SLOTS`
  (64). The stack's own table does not grow with sockets once the minted copy has been handed over
  and deleted.

## The fork: a `CALL`'s reply cannot carry a capability

The ruling's costs, as recorded, were "a contract change to `socket_protocol`, one capability slot
per open socket in the client". That premise is not true of this kernel. The stack can mint the
badged copy (`BADGE`, once it holds `GRANT` on its endpoint), but it has no way to give it to the
client that asked:

- A reply is two words. `abi::reply::REPLY` takes `r0, r1`, and the kernel's `ipc_reply`
  (`kernel/src/sched.rs`) writes the caller's mailbox as `[r0, r1, 0, 0, 0]`. No capability moves.
- `SEND_CAP` is the only IPC that moves a capability, and it moves one from sender to receiver. For
  the stack to send to a client, the client must be receiving on an endpoint the stack can write,
  and `SEND_CAP` blocks until it does.
- There is no non-blocking send (milestone 235 (a command that faults hangs the prompt, because
  nothing has a word for it)'s block and `notes/supervision.md` both record
  wanting one and not having it).

So the step "on open, the stack mints a badged endpoint and the client gets it" needs either a
kernel change or a protocol that pays for the missing step some other way. The brief for this lane
says a new kernel method is raised, not invented. These are the options.

### Option A: a reply carries one capability

A new method on the Reply object (name provisional, `REPLY_CAP`): `invoke(reply, REPLY_CAP, r0,
r1, cap_slot)`. The kernel moves the server's `cap_slot` into a free slot of the caller's table and
delivers `r0` and `r1` as `REPLY` does. The caller finds the new slot in `x2` (`NO_CAP`
when the caller's table was full, with the copy dropped, which is how a `CALL` whose Reply could
not be installed already behaves). The delegated capability needs `GRANT` in the server's table,
exactly as `SEND_CAP` requires. Plain `REPLY` is untouched, so the shape every service already runs
pays nothing.

- Prior art, read: seL4 lets a reply carry capabilities, gated on the `Grant` right of the endpoint
  capability the server received on (seL4 reference manual; seL4 PRs #874 and #945 tightened the MCS
  variant). Fuchsia's `zx_channel_call` reads its reply as `zx_channel_read` does, handles included.
- What it keeps: the ruling, literally. The stack mints on open, the client gets one slot, nothing
  blocks the stack on a client.
- What it costs: a new method on the syscall surface (§10 (process model: capability-based,
  microkernel), §12 (call/reply IPC)), built and proved on aarch64, riscv64 and x86_64, and priced
  with `script/fastpath-footprint` and `script/bench`. Not measured yet; the plain `REPLY` path does
  not change, so `ipc_rtt` should not move.
- Open within A: whether a caller must opt in to receiving a capability. §246 (a plain `RECEIVE`
  never takes a capability) refused filling the table of a receiver that never asked. A `CALL`
  caller did ask for a reply, and it can delete what it gets, so this section does not propose an
  opt-in; seL4 has none on the caller's side either.

### Option B: the client brings a return endpoint

No kernel change. Each client retypes a rendezvous, sends the stack a `WRITE | GRANT` copy, and
waits in `RECEIVE_CAP` on it after each open; the stack `SEND_CAP`s the minted socket onto it.

- What it loses: the stack is one thread, and `SEND_CAP` blocks until the client receives. A client
  that opens and never receives parks the stack for the life of the machine, and every other client
  loses the network. That is a new denial of service any network program could perform, added by
  the fix for a confinement escape.
- The tree's one precedent is `login`'s `RESULT` endpoint, which accepts the same hazard only
  because it assumes no hostile co-tenant at the front door (`components/src/login.rs`, module doc,
  "What this does not build"). The network stack's clients are exactly the programs risk 7 is
  about, so that assumption does not transfer.
- It also costs every client a rendezvous page and two more rendezvous per open.

### Option C: the progenitor mints a pool of socket capabilities at spawn

No kernel change. The progenitor already mints badged copies for the file service (§230's pool) and
the resolver (§252 (a resolver grant is one zone per client badge)). Here it would mint `K` badged
copies of the stack's endpoint for each child that declares `network`, and register them with the
stack the way `BIND` registers a filesystem badge. Each copy is a socket slot: `OPEN` on it makes
the socket, `CLOSE` ends it.

- What it loses: the stack does not mint, so this is not the ruling's shape. Every network child
  pays `K` slots at spawn whether it opens anything. And "close unbinds so a stale capability
  reaches nothing" holds only if a closed badge is retired for good: otherwise a copy passed away
  earlier reaches the next socket opened on that badge. Retiring means `K` opens per program life,
  unless a refill channel is added, which is option B's problem again.

### Refused within the fork

- A non-blocking `SEND_CAP` on its own. It is a kernel change as well, and it does not deliver to
  an honest client either: a single-threaded client is still inside its `CALL` when the stack would
  send, so the send finds nobody parked and fails.
- The client minting its own badges. A client holding `GRANT` on an unbadged copy of the stack's
  endpoint can mint any badge, so it could name any socket, which is the escape this fixes.

### What each option preserves of the ruling

| | stack mints on open | nothing blocks the stack on a client | stale copy reaches nothing | kernel change |
|---|---|---|---|---|
| A | yes | yes | yes | a reply method |
| B | yes | **no** | yes | none |
| C | no | yes | only by retiring badges | none |

A is the only option that keeps every clause of the ruling, and it is the most work of the three,
so the lean toward it is not about effort. It is a syscall-surface change, which is on the
irreversible list, so this records the options and the lean rather than a recommendation to build
without a ruling.

## Alternatives the ruling refused

- **`(badge, sid)` keying**, the 03:50Z per-caller-windows ruling read across the whole socket
  table, as the review on #1798 recommended. It closes the cross-client escape, but a socket is
  still a number its holder chose, so it cannot be passed to another program, and closing it revokes
  nothing a copy could hold. A second table keyed by caller is also more state than a badge that is
  the socket.
- **Per-caller frame windows only**, the 03:50Z ruling as worded. It closes the frame capture and
  leaves sending to, reading and closing another client's socket open.
- **A stack per client** (option 3 in milestone 649's block): a NIC backs one stack, so it needs a
  multiplexer that does not exist.

## Prior art

- Fuchsia: a socket is a `zx_socket` handle, a kernel object a process holds and passes over a
  channel; nothing names it by a per-server integer. (Recalled, not re-read for this section.)
- Genode: a client's connection to a service is a session capability returned by the server through
  the parent's routing; every later RPC is on that capability. (Recalled.)
- Capsicum: in capability mode a socket is a file descriptor with rights, and it moves between
  processes by `SCM_RIGHTS`; no global name reaches it. (Recalled.)
- In this tree: §27 (the filesystem service)'s directory capability, bound to a badge by
  milestone 606 (a directory walk costs what it does on Linux), and §252's resolver grant. In both, authority
  is a capability the server recognizes by its badge, never an argument the client fills in.

## What is blocked until this is answered

All of milestone 649's build: the `socket_protocol` change, `net_stack`, every client, and the
tests. Under option A the kernel method comes first, on all three ISAs. Under option C the
progenitor's pool and the stack's registration verb come first. Option B needs nothing first, and
is not recommended.

Also ruled on #1798 the same day, and recorded there by the GLM lane: per-caller scoping becomes a
written rule for every server that keeps windows for many clients, and milestone 649's lane audits
the rest, `system_log`'s reader windows first. It is folded in here when this section is decided.
That first audit is done and found nothing to fix: `system_log` answers `OPERATION_READ` with the
window registered for the reader's badge (`crates/system_log/src/lib.rs`, `Log::handle`), and only
badge 0, the spawner, can register one (`OPERATION_READER`). A reader names no window, so it cannot
name another's. The remaining servers are audited with the build.