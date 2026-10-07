---
status: BUILT
raised: 2026-09-05
built: 2026-10-06
milestone_dependencies: none
decision_dependencies: 248, 252
machine_requirements: none
specific_machine: none
needs_person: no
---
# 384. Nothing here resolves a hostname, and in a capability system the resolver is a grant

Filed 2026-09-05 as an unnumbered proposal, written after asking whether a
milestone covered `curl` or `wget`; numbered 2026-09-19 by milestone 433's drain of the proposal
pile. Premise re-read against the tree on 2026-09-19 and still true, with one path correction:
the `smoltcp` 0.14 dependency moved from `user/Cargo.toml` to `components/Cargo.toml` when milestone
175 split the program directories, and `socket-dns` is still not among its features. Nothing in
`crates/` or `components/src/` resolves a name, and the DHCP client still receives a nameserver
nothing reads. The one thing that changed since filing supports the proposal rather than undercutting
it: milestone 298 retired the multicast DNS responder on 2026-09-15, and its block cites this
proposal as the reason the link-local responder was not the answer here.
*(Number provisional until the merge queue lands it.)*

`smoltcp` already ships the socket; enabling it is a feature flag and a program.

## Built 2026-10-06: the resolver, the grant and the gate on three architectures

A second lane (`lane/384-resolver`, PR #1760) built what the first one stopped at, after calef ruled
Fork 1 (§248 (the name resolver is its own confined program)). Forks 2 and 3 were built on the first
lane's leaning answers. calef ratified both on 2026-10-06 (UTC). §252 (a resolver grant is one zone
per client badge) records them.

- `components/src/name_resolver.rs` (name provisional): §248's confined program. It holds a `Stack`
  endpoint, an entropy endpoint and the name server its spawner names, and serves clients over its
  own endpoint. Each transaction id comes from the entropy service, and it asks nothing without one.
- `crates/name_resolution_protocol` (name provisional): Fork 3 as P1, the socket contract's shape.
  The client attaches a page, writes a name, `CALL`s resolve, and reads the addresses from the page.
  It also carries Fork 2 as G3 by badge: the spawner grants each client's badge a zone, and the
  resolver judges the name asked against it **before anything is sent**. 11 host tests.
- `a_granted_client_resolves_inside_its_zone_and_nothing_outside_it`, on aarch64, riscv64 and
  x86_64 over the `e1000e` every runner attaches. A client granted `nife.test` resolves two names,
  gets each lie refused for its own reason, is denied `example.com` and `evilnife.test`, and
  connects to what it resolved; a client with no grant is denied every name. Deleting the zone
  check turns it red (`system_tests/falsifications/`). The x86_64 and UEFI runners gained the
  name-server peer.

Reuse: the parser is `domain_name_system`, kept after the 2026-10-04 survey of six crates in
`notes/name-resolution.md` (§46 (thin primitives or whole subsystems; we write everything in
between)). The resolver program and its protocol are this tree's shapes
(`network_time_client`'s endowment, the socket contract's page, the system log's badge
registration), so nothing outside the tree fit; `hickory-resolver` and `domain`'s stub need `tokio`,
whose poller the PAL lacks.

## Partial as of 2026-10-04: the wire format and an end-to-end test, the forks not taken

A lane (`lane/384-name-resolver`, PR #1634) built the part that is not a fork and stopped at the
three that are. `notes/name-resolution.md` has the whole account and the proposal.

- `crates/domain_name_system` (name provisional): a query for one name's A records, and
  `Query::accept`, which believes an address only at the end of a bounded CNAME chain from the name
  asked. 35 host tests, three on replies captured from 1.1.1.1; three Kani harnesses, each falsified
  by a replayable patch; a fuzz target whose property is that no address is believed that the
  answer does not carry.
- An end-to-end test on aarch64 and riscv64: six names asked over TCP through `net_stack` of
  `helpers/name-server-peer`, two resolved and four lies refused, then a connection to the resolved
  address. Deleting the owner check fails it with `0xe14f`, the poisoned case.
- The premise of putting the resolver in `net_stack` was read in `smoltcp`'s source: its DNS
  transaction id and source port come from a generator `net_stack` seeds with `now()`, it has no
  TCP fallback, and it follows a CNAME only when the CNAME comes first.

## The gap, measured

`user/Cargo.toml` builds `smoltcp` 0.14 with `alloc`, `medium-ethernet`, `proto-ipv4`,
`proto-dhcpv4`, `socket-udp`, `socket-tcp` and `socket-dhcpv4`. `socket-dns` is not among them,
and nothing else in `crates/` or `user/src/` resolves names: the only greps that match "resolve" are
about capability names and generational tables.

So a program cannot turn a hostname into an address. That is upstream of everything else a
network client needs. `curl` with a perfect HTTP implementation and a perfect TLS stack still cannot
fetch `https://example.com`, and DHCP already hands us a nameserver we throw away.

It is also upstream of two proposals and two milestones. The TLS proposal's remaining consumers
are milestone 99's `clone` half and milestone 174, and both of those reach a host by name.

## Why this is worth its own piece rather than a line in a bigger one

Because the interesting question is not the resolver, it is who holds it, and that question does
not arise on Unix.

On Unix, name resolution is ambient. `/etc/resolv.conf` is readable by every process, `getaddrinfo`
is in libc, and any program can resolve any name against whatever the system decided. A program that
should only ever talk to one host can silently look up any other, and the first sign is usually in a
packet capture.

Here it should be a capability. "Which resolver may this program use, and for what" is a grant,
the same shape as every other authority in this system:

- A client granted a resolver capability that answers for exactly one domain cannot be induced to
  look up anything else, whatever bug or injection it carries.
- A resolver is itself a network client, so it is a confined component with a socket grant, not a
  library linked into everyone.
- A program with no resolver grant and a literal address still works, which is what the tree does
  today and should keep working.

This is the same observation as the trust-store one in
milestone 387, `design/roadmap/387-a-tls-stack-and-which-one.md`, and the two belong to the same
family:
the ambient parts of a Unix network client are exactly the parts a capability system should make
explicit. It is also a small, concrete instance of
[§145](../decisions/145-compartmentalization-at-process-cost.md)'s argument, which is otherwise
stated at the scale of a whole operating system.

## What it would take

- Enable `socket-dns` in `smoltcp`, which already implements the client.
- Decide where the resolver lives: inside `net_stack` behind a verb on `socket_protocol`, or as its own
  confined program that holds a socket grant. The second is more in keeping with the tree
  (`net_stack` is already a userspace server and this would be a client of it), and it is the more
  expensive one, so it should be argued rather than assumed.
- Decide the capability's shape, which is the part worth thinking about: a resolver grant that is
  "any name" is barely better than ambient, and one that is "exactly these names" may be too rigid
  to use. A domain suffix is the obvious middle and obvious is not the same as right.
- Take the nameserver from DHCP, which `proto-dhcpv4` already receives and nothing currently reads.

## What it is not

It is not `curl`. A resolver plus HTTP plus TLS is three pieces and this is the first. There is
no consumer today for any of them, which is why this is a proposal.

And it does not need mDNS. The multicast DNS responder answered for names on the local link, which
is a different protocol solving a different problem, and reusing it here would have been a category
error. It was retired on 2026-09-15 by milestone 298 (retire the multicast DNS responder and its
two crates), in notes/mdns.md. Its general half is worth
reading before writing a parser: DNS header, record and name decoding with compression pointers,
Kani-proven not to loop or overrun, is in `crates/multicast_dns_protocol` at commit `0652c981`
(`git show 0652c981:crates/multicast_dns_protocol/src/lib.rs`, with `src/proofs.rs` beside it).

## BUGS

- `socket-dns`'s cost was read rather than built (2026-10-04): 1,503 lines of `smoltcp` source, and
  the three findings above. An enabled feature nothing calls links to nothing, so a size delta
  without a caller would measure nothing.
- The resolver is started only by the test harness. The booted system starting it from the lease is
  milestone 801 (packages over the internet)'s, whose package client is its first real client.
- The resolver's UDP path, with its source check and its keep-listening on a forged reply, has run
  on no boot: slirp forwards UDP to no process a test owns, so the gate asks over TCP.

## Follow-on

- **Decision.** `design/decisions/248-the-resolver-is-its-own-confined-program.md` holds Fork 1.
  calef ruled it on 2026-10-04. The resolver is its own confined program.
- **Done.** The resolver program, `components/src/name_resolver.rs`, by #1760.
- **Done.** The grant's shape (Fork 2) is a zone per client badge, built by #1760. The client
  protocol (Fork 3) is P1, in `crates/name_resolution_protocol`. calef ratified both on 2026-10-06
  (UTC). `design/decisions/252-a-resolver-grant-is-a-zone-per-badge-over-the-socket-contracts-shape.md`
  holds them.
- **Milestone 801.** Starting the resolver on the booted system, with the lease's name server, and
  granting the package client its source's host: milestone 801 (packages over the internet), whose
  package client is the resolver's first real client.
- **Done.** The DHCP nameserver rides the lease report's second word, which §248 unblocked. The
  layout is `socket_protocol::lease`, provisional. Both ISAs' lease tests assert slirp's 10.0.2.3.
- **Done.** The x86_64 leg, by #1760: the resolver's gate runs over the `e1000e` on all three
  architectures, and the x86_64 and UEFI runners carry the name-server peer.
- **Milestone 783.** Milestone 783 (the network stack seeds its random generator from the clock, and TCP sequence numbers come from it). `design/roadmap/783-net-stack-seeds-its-generator-from-entropy.md`: seed
  `smoltcp`'s generator in `net_stack` from the entropy service rather than `now()`, since it
  chooses TCP initial sequence numbers today, whatever happens to DNS.


## Index row

`components/Cargo.toml` builds `smoltcp` 0.14 without `socket-dns`, and nothing else in the tree
turns a hostname into an address, so a program cannot reach a host by name even with a perfect HTTP
implementation behind it, and the nameserver DHCP already hands us is thrown away. The work is worth
its own block because the interesting question is not the resolver, it is who holds it. On Unix name
resolution is ambient: `/etc/resolv.conf` is readable by everyone, `getaddrinfo` is in libc, and a
program that should only ever talk to one host can silently look up any other, with a packet capture
as the first sign. Here it should be a grant, which makes three things true that are not true on
Unix: a client granted a resolver that answers for one domain cannot be induced to look up anything
else whatever injection it carries, the resolver is itself a confined network client rather than a
library linked into everyone, and a program with no grant and a literal address still works. That is
the same observation as the trust-store half of milestone 387's TLS fork, and a small concrete
instance of §145's argument. What it takes: enable `socket-dns`, decide whether the resolver lives
inside `net_stack` or as its own confined program holding a socket grant (the second is more in
keeping with the tree and more expensive, so it is argued rather than assumed), decide the
capability's shape, which is the real design work, and read the nameserver DHCP already receives. It
has a consumer now, milestone 801 (packages over the internet), whose package client fetches the
distribution's index by host name. Built 2026-10-06: the resolver is its own confined program, and
each client's badge is granted a zone it is judged against before anything is sent.
