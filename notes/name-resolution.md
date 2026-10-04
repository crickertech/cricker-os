# Name resolution: the wire format, and the three forks it leaves

Milestone 384 (in a capability system the resolver is a grant), lane of 2026-10-04 (UTC). Nothing in
this tree turned a host name into an address. This note records what the lane built, how to run
it, and the three questions it stopped at because each is an architect's call. The proposal below
is written for the maintainer to mint as a `design/decisions/` section marked PROPOSED.

## What is built

| Piece | Where | Evidence |
|---|---|---|
| The DNS wire format a stub resolver needs: a query for one name's A records, and `Query::accept`, the checks a reply must pass | `crates/domain_name_system` | 35 host tests, three of them on replies captured from 1.1.1.1 |
| The name decoder, header codec and TCP reassembly, proved | `crates/domain_name_system/src/proofs.rs` | 3 Kani harnesses, about 34 s; each falsified by a replayable patch |
| `accept` fuzzed with a property, not only for panics | `fuzz/fuzz_targets/domain_name_system_reply.rs` | 5.2 million runs in 120 s clean; deleting the owner check was caught |
| A name resolved end to end through `net_stack`, with lies refused, then connected to | `components/src/socket_test_client.rs`, `helpers/name-server-peer` | green on aarch64 and riscv64; the same deleted check fails it with `0xe14f` |

`Query::accept` believes an address only after the reply is a response with our transaction id, is
not truncated, echoes exactly our question and has response code zero. The address must also be
owned by the end of a CNAME chain of at most eight links starting from our name. Records for any other owner are
ignored, and the authority and additional sections are never read. The crate's own docs list the
checks in order and carry its `BUGS`.

The end-to-end test, `a_name_resolves_through_the_stack_and_a_real_one_when_the_host_answers`, asks
the runners' name server six names over TCP. `packages.nife.test` and `mirror.nife.test` (a
compressed CNAME) resolve to 10.0.2.9. `nosuch`, `forged`, `poisoned` and `loop` must each be
refused by its own check. The client then opens a TCP connection to the address it resolved and
reads an echo back. The second half of the same test is the old real-DNS check, now judged by the
crate, and it still does not gate.

### Why DNS over TCP in the test

slirp's `guestfwd` forwards TCP only. A UDP name server the test owned would have to bind a port on
the developer's machine, which the runners avoid everywhere else, and a fixed port collides between
two lanes running at once. So the gating exchange is DNS over TCP (RFC 7766), which every
general-purpose name server must serve, and the UDP path is exercised against the host's resolver
through slirp's 10.0.2.3, non-gating.

### EXAMPLES

```
$ cargo test -p domain_name_system
test result: ok. 35 passed
$ (cd crates/domain_name_system && cargo kani)
Complete - 3 successfully verified harnesses, 0 failures, 3 total.
$ script/fuzz --time 120 domain_name_system_reply
==> fuzz: no crashes in 1 targets at 120s each
$ script/test --arch aarch64 --test a_name_resolves_through_the_stack
test system_tests::user::tests::a_name_resolves_through_the_stack_and_a_real_one_when_the_host_answers ... ok
```

A client code from the gating half reads `0xE1`, then the case's index in `NAME_SERVER_CASES`, then
the stage: `1` to `7` are transport steps and `F` is a wrong verdict, a lie believed or the truth
refused.

## The proposal: three forks, stopped at

Each is a wire format two programs agree on or a choice 384 leaves open, so the lane built none of
them. What each blocks is at the end.

### Fork 1. Where the resolver lives

| Option | Shape | Cost, measured or read |
|---|---|---|
| R1. Inside `net_stack` | Enable `smoltcp`'s `socket-dns`; add a resolve verb to `socket_protocol` | 711 lines of `smoltcp/src/socket/dns.rs` plus 792 of `src/wire/dns.rs`, read 2026-10-04 |
| R2. Its own confined program | A resolver holding a `Stack` endpoint and an entropy endpoint, serving clients over an endpoint, using `domain_name_system` | A program, its spawn wiring and its protocol (Fork 3) |
| R3. A library in each client | Every client links the crate and holds a `Stack` endpoint | Nothing new to build |

Recommendation: R2. This fork is code and reversible, so it gets a recommendation.

What the tree does in the analogous case: `network_time_client` is its own program holding a
`Stack` endpoint and an entropy endpoint, and the clock service is a separate holder of the
authority. `net_stack` is a server whose verbs are sockets, not protocols above them.

The premise of R1 was checked against `smoltcp` 0.14.0's source rather than assumed:

- Its transaction id and source port come from `smoltcp`'s PCG32 (`src/rand.rs`), and `net_stack`
  seeds it with `config.random_seed = now()`, the boot-relative counter. An off-path forger who can
  estimate boot time can predict both. R2 draws them from the entropy service, as the NTP client
  draws its nonce.
- It has no TCP fallback, so a truncated answer is parsed as if whole.
- It follows a CNAME by renaming the query in place, and its comment says it relies on the CNAME
  preceding the A records in the packet.
- It puts a DNS parser in the process that holds the NIC.

Prior art, read 2026-10-04: Fuchsia's `fuchsia.net.name` library defines `Lookup` (`LookupIp`,
`LookupHostname`) and a separate `LookupAdmin` with `SetDnsServers`, a resolver reached as its own
protocol. That the component serving it is `dns-resolver`, apart from the netstack, is recalled and
not read. Plan 9's `ndb/dns` serves `/net/dns`: a client writes a name and type and reads answers,
and only a process whose namespace holds that file can resolve at all.

R3 is refused by 384's own argument: every client would hold the network, which is the ambient
shape the milestone exists to remove.

Would R2 still win at equal cost? Yes. It costs more than R1, and the case for it is the seed, the
parser's location and the grant Fork 2 needs a holder for, none of which is effort.

### Fork 2. The grant's shape, and what enforces it

Options are given rather than a recommendation, because a grant shape is what every client will be
written against.

| Option | A client may resolve | Note |
|---|---|---|
| G1. Any name | everything the resolver can | barely better than ambient, as 384 says |
| G2. Exact names | a list fixed at grant time | rigid: a CDN's CNAME chain leaves the list on the first hop |
| G3. A zone | the zone and every name under it | `Name::is_within` is built and tested, label by label, so `evilnife.test` is not within `nife.test`; a zone of one host name is G2 in effect |

G2's rigidity is narrower than it sounds if the check applies to the name asked and not to the
chain the answer follows: a client granted `packages.example.org` would still follow its CNAME to a
CDN. Which of the two the grant means is part of this fork.

The enforcement is a second question with two answers the tree already uses:

- One resolver per grant, the scope in its endowment. `net_stack` is spawned with its listen and UDP
  bind grants as a word, and `fs_subtree_caretaker` is a process per subtree.
- One resolver, a scope per badge. §230 (badged endpoint capabilities) delivers the badge with each
  request, and the system log stamps writers from it.

The lane's lean, not a recommendation: G3 by badge. One process serves every client, which matters
because every process here costs frames, and a zone covers the one real consumer, the package
client, whose grant is its package source's host.

### Fork 3. The client protocol

Options only, since this is a wire format two programs agree on, and by rule 7 it goes in a crate
(`name_resolution_protocol`, provisional).

- P1. The socket contract's shape: the client attaches a page, writes the name, calls a resolve verb,
  and reads up to eight addresses, a TTL and a status word from the page. One round trip.
- P2. Plan 9's shape over `filesystem_protocol`: the resolver serves a file the client writes a name
  to and reads answers from. No new protocol, and a grant is a directory capability, at the price
  of a file server's machinery for one exchange.
- P3. Names in message words. A name is up to 255 bytes, so it does not fit, which is why P1 and P2
  both use a page.

### A smaller piece: the nameserver from DHCP

`net_stack` reports its lease to its spawner as `send(REPORT, address, 0, 0)`, and `smoltcp`'s
`dhcpv4::Config` already carries the DNS servers. Putting the first one in the second word is a
change to what `net_stack` and every spawner of it agree on, so it waits for Fork 1: under R2 the
spawner hands that address to the resolver as its endowment.

### What is blocked until these are answered

The resolver program, and using the DHCP nameserver. So is the progenitor fetching a package by
host name, rung 3c of milestone 198 (a package manager, and the trivial install that makes a second
customer possible). The crate, its proofs and the gating test do not wait on any of it.

## BUGS

- x86_64 has no NIC under QEMU, so the end-to-end test runs on aarch64 and riscv64 only. The crate
  is portable and host-tested; milestone 494 (a driver for the network card a PC actually has) is
  the x86_64 leg's prerequisite.
- The gating exchange is TCP. UDP to a name server the test owns is not gated (see above), and the
  real-DNS half that covers UDP skips when the host's resolver does not answer.
- The test's transaction id is fixed, because the socket client holds no entropy endpoint. A real
  resolver must not do this, and R2 is written assuming it draws the id from the entropy service.
- `smoltcp`'s generator is seeded from `now()` for everything it randomises in `net_stack`, which
  includes TCP's initial sequence numbers (`socket/tcp.rs`, read); `net_stack` picks ephemeral
  ports with its own rotating allocator, not the generator. That is a finding about
  `net_stack` beyond this milestone, proposed in
  `design/roadmap/proposals/net-stack-seeds-its-generator-from-entropy.md`.
- `notes/fuzzing.md`'s table does not list `domain_name_system_reply`, as it does not list the
  service targets. Adding a row obliges that note to shed 47 bold spans under the prose ratchet,
  which is its own piece of work.

Name: provisional 2026-10-04 (milestone 384's lane).
