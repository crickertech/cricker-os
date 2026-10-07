---
status: DECIDED
raised: 2026-10-04
decided: 2026-10-06
ratified_by: calef
---

# 252. A resolver grant is one zone per client badge, and a client speaks the socket contract's shape

calef, 2026-10-06 (UTC), on PR #1760: *"Yes on Fork 2"* and *"Yes on Fork 3"*. That ratifies what
milestone 384 (in a capability system the resolver is a grant) built provisionally for the two
forks §248 (the name resolver is its own confined program) left open. *(Section number provisional
until the merge queue lands it.)* Recorded by the maintainer session the same day, from Forks 2 and
3 of `notes/name-resolution.md`, which carries the options and the reasoning in full.

The names `name_resolver`, `name_resolution_protocol` and `name_resolver_test_client` stay
provisional. Ratifying them is a separate act under design/naming.md, and this section is not it.

## Fork 2: the grant

1. A client's authority to resolve is one zone, bound to its §230 (badged endpoint capabilities)
   badge. One resolver serves every client; the spawner, which holds the resolver's one unbadged
   capability, grants each badge its zone with `OPERATION_GRANT` messages, the shape the system log
   registers writers in.
2. The resolver checks the name the client asked against that zone, label by label
   (`Name::is_within`, so `evilnife.test` is not within `nife.test`), before it sends anything. It
   does not check the CNAME chain the answer follows: a client granted `packages.example.org` still
   reaches the CDN that name points at.
3. "Any name" is the root zone, and "exactly this name" is a one-host zone. They are grants of the
   same mechanism, not separate kinds. A client that needs every name says so in its grant.

Refused, one line each:

- G1 (any name) and G2 (exact name lists) as mechanisms of their own: the root zone and a one-host
  zone already express them.
- One resolver per grant: it costs a process per client, where a badge costs one table entry.
- Checking the CNAME chain: it breaks every name served through a CDN, and the asked name already
  carries the authority.

## Fork 3: the protocol

The client protocol is P1, the socket contract's shape, in `crates/name_resolution_protocol`. The
client attaches one page, writes the name into it, and makes one `CALL` carrying the name's length.
The reply carries a status word and the TTL; the addresses, up to `MAX_ADDRESSES`, are in the page.
One round trip, in the shape every network client here already speaks.

Refused, one line each:

- P2 (a Plan 9 file over `filesystem_protocol`): it wraps one exchange in a file server's open,
  write, read and close.
- P3 (the name in message words): a name of up to 253 bytes does not fit.

## What this changes

Nothing in code. Both were built this way by #1760 and its gate runs on aarch64, riscv64 and x86_64.
The ratification turns them from provisional into what a client may be written against. The first
is milestone 801 (packages over the internet)'s package client, which gets a one-host zone.

## Reversibility

Reversible until a client other than the test client is written against the protocol. After that,
changing the page layout or the resolve words means changing every client in the same pull request,
since the protocol is a crate under rule 7 and the compiler finds them.
