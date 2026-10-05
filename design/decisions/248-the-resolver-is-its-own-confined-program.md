---
status: DECIDED
raised: 2026-10-04
decided: 2026-10-04
ratified_by: calef
---

# 248. The name resolver is its own confined program, not a verb inside the network stack

calef, 2026-10-04 (UTC): *"the resolver should be its own confined program"*. *(Section number
provisional until the merge queue lands it.)* Recorded by milestone 384's lane on the maintainer's
delegation, from Fork 1 of `notes/name-resolution.md`, which carries the measurements in full.

## The ruling

A program that needs a host name turned into an address asks a resolver that runs as its own
process. That process holds an endpoint to `net_stack` (a socket grant, like any other client), an
endpoint to the entropy service, and the nameserver it is told to use. `net_stack` gains no DNS
verb and runs no DNS parser. The parser is `crates/domain_name_system`, which the resolver links.

## The options

| Option | Shape | Outcome |
|---|---|---|
| R1. Inside `net_stack` | Enable `smoltcp`'s `socket-dns` and add a resolve verb to `socket_protocol` | Refused, below |
| R2. Its own confined program | A client of `net_stack` holding a `Stack` endpoint and an entropy endpoint, serving its own clients | Ruled |
| R3. A library in each client | Every client links the parser and holds a `Stack` endpoint of its own | Refused, below |

## Why R2

Capability separation first. Under R2 a client's authority to resolve is an endpoint to the
resolver and nothing else, so a program can be allowed to resolve without being allowed to send a
packet, and the reverse. It also keeps the DNS parser, which reads bytes any forger chooses, out of
the process that holds the NIC. The tree already works this way for time: `network_time_client` is
its own program holding a `Stack` endpoint and an entropy endpoint, and the clock service holds the
authority it proposes to.

## Why R1 was refused

R1 is cheaper: a feature flag on a dependency already in the graph and one verb. It was refused for
the separation above and for three gaps found by reading `smoltcp` 0.14.0's source (2026-10-04):

1. Its transaction id and source port come from `smoltcp`'s PCG32 (`src/rand.rs`), which `net_stack`
   seeds with `config.random_seed = now()`, the boot-relative counter. An off-path forger who can
   estimate the uptime can predict both. R2 draws the id from the entropy service.
2. It has no TCP fallback, so a truncated answer is parsed as if it were whole.
3. It follows a CNAME by renaming the query in place, relying (by its own comment) on the CNAME
   preceding the A records in the packet. `Query::accept` follows the chain in any order.

## Why R3 was refused

Every client would hold the network itself. That is the ambient Unix shape milestone 384 (in a
capability system the resolver is a grant) exists to remove. A program that should talk to one host
could look up, and then reach, any other.

## What this unblocks

The nameserver DHCP hands out has a consumer: the resolver's spawner reads it from `net_stack`'s
lease report and hands it to the resolver as its endowment. The same pull request as this section
puts it in the report's second word, provisionally (`socket_protocol::lease`).

## What is still blocked

- Fork 2 of `notes/name-resolution.md`: the shape of a resolver grant, which is any name, exact
  names or a zone. It also asks what enforces it: one resolver per grant, or a badge per client
  under §230 (badged endpoint capabilities). The resolver's endowment depends on it.
- Fork 3: the protocol between a client and the resolver, a crate under rule 7. The resolver's entry
  point depends on it, so the program itself is not built yet.
- Testing the program end to end also wants either a stack shared by two clients in the kernel
  harness or a virtio slot past `MAX_DEVICES`, which `kernel/src/virtio.rs` calls a foot gun.

## Reversibility

Reversible until a client is written against the resolver's protocol. Before Fork 3 is answered
nobody can have acted on it.
