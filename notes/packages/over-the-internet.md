# Packages over the internet: the index, a host name and HTTPS

The appendix to [notes/packages.md](../packages.md) for milestone 801 (packages over the internet),
rung 3c of milestone 198 (a package manager). Built 2026-10-09 (UTC) by lane
`milestone/801-packages-over-the-internet`, under QEMU only. The exit criterion is xenon's installed
system, which needs a person, a host for `basalt.nifeos.org` and milestone 494 (a driver for the network
card a PC actually has) on silicon.

## What a fetch over the internet does now

From a program holding the network, a clock, entropy and a resolver grant:

1. `TcpStream::connect(("basalt.test", 8443))` resolves the name through the resolver badge.
2. TLS 1.3 to that host, trusting one pinned root (`pinned_tls_client`, milestone 501 (a TLS
   client that speaks to one pinned peer)).
3. `GET` the index, and `package_index::Index::parse` reads every line or refuses it whole.
4. `Index::find` picks the one entry a name asks for, and the client fetches its bytes from the
   location the entry names: another host, plain HTTP.
5. `package_index::accept` admits the bytes only if they are the package asked for and hash to the
   index's digest.

### EXAMPLES

`package_fetch_exerciser`'s transcript on aarch64, 2026-10-09 (UTC). riscv64 and x86_64 print the
same lines for their own architecture.

```
$ script/test --arch aarch64 --test package_is_fetched_through
package_fetch_exerciser start
index ok 12 packages from basalt.test over TLS
fetched greeting-0.1.0-aarch64 from packages.basalt.test:8080, 83875 bytes, program greeting
refused uptime-0.1.0-aarch64: NotCataloged
refused nosuch: not in the index
package_fetch_exerciser done
```

`std_resolve` shows the std half of the name, with a grant and then without one:

```
lookup packages.nife.test: 10.0.2.9:7777
lookup nosuch.nife.test: NotFound
lookup example.com: PermissionDenied
echo by name ok
...
lookup packages.nife.test: Unsupported
```

## The pieces

| Piece | Where | Gated |
|---|---|---|
| The index model, the stand-in reader, `accept` | `crates/package_index` (provisional) | host tests, in CI |
| What `name@version` means, written once | `package_archive::matching_stem` | host tests, in CI |
| A std program resolves through its grant | `lookup_host` in `patches/std-nife`, `std_runtime_protocol::RESOLVER_SLOT` (9, provisional) | `a_std_program_resolves_its_granted_zone_and_nothing_without_a_grant`, all three, in CI |
| The resolver's client words, copied into std | `crates/name_resolution_protocol/src/wire.rs` | the same test |
| The whole fetch | `pinned_tls_exerciser/src/bin/package_fetch_exerciser.rs` | `a_package_is_fetched_through_the_index_by_name_over_tls_and_judged_by_its_digest`, all three, skipped in CI |
| The test hosts | `helpers/tls-peer` (index), `helpers/name-server-peer` (two names) | the same test |

The whole-fetch test skips in CI on the terms milestone 501's does: no gate builds the TLS graph
yet, and whether one should is milestone 855 (the TLS graph enters the gated build).

### Why the resolver is a std slot

`std::net` is how a program written for any OS reaches a host by name, and `jig` (milestone 809
(the package client becomes a program)) is a std program. So the grant milestone 384 (in a
capability system the resolver is a grant) built became what `ToSocketAddrs` asks. A program
holding no resolver still connects to numeric addresses, because std parses those first. Each
refusal keeps its reason: `PermissionDenied` is the grant, `NotFound` is the name server.

The page the resolver writes answers into is minted from the socket budget (slot 3) on the first
lookup, so a resolver grant is only usable beside the network. A program with the slot empty pays
nothing; the probe is a refused method call.

## The index, split from where the bytes live

§250 (an image names its distribution's package index, and the bytes may live anywhere) separates
two things the image's catalog holds together today.

- `Index::parse` refuses a whole index at its first unreadable line, so a client never installs
  from an index it only partly understood.
- `Index::find` reads `name` and `name@version` by the catalog's own rule.
- `accept` hands the bytes to `package_archive::installable_as` with the entry as a one-line
  catalog. The progenitor's installer calls the same function with the image's catalog.

The format is not ruled, so the crate reads a stand-in: the catalog's line with a location after
it. A ruling replaces `Index::parse` and nothing above it. The survey and the five open questions
are [the-index-format.md](the-index-format.md).

```
greeting-0.1.0-aarch64 sha256:<64 hex> http://packages.basalt.test:8080/greeting-0.1.0-aarch64.nifepkg
```

## The seams, each stubbed behind a test host

| Production | Under QEMU | Whose |
|---|---|---|
| `basalt.nifeos.org` | `basalt.test`, `helpers/tls-peer` at 10.0.2.9:8443 | DNS and hosting: calef's hands |
| The index's path and file name | `package_index::STAND_IN_INDEX_PATH` | calef's ruling (§250's unwritten part) |
| ISRG Root X1 | the test authority in `pinned_tls_client/fixtures/` | ruled; X1 meets a real chain only in 501's ignored host test |
| The index's format | the stand-in encoding | the format proposal |
| The resolver started from the lease | the test harness starts it, pointed at `helpers/name-server-peer` | identified work (milestone 801's block) |
| `jig` installing what it fetched | the program fetches and judges, and installs nothing | milestone 809 |

## BUGS

- Nothing is installed. Taking an index's word for what may be installed is `jig` writing the
  index copy, milestone 809's ruling I2, and that program does not exist yet.
- No program at the prompt can do this yet. The booted system does not start the resolver, and the
  progenitor does not give a std program the network (milestone 595 (the shell runs a `std`
  program)'s BUGS).
- The stand-in has no "moved to" field, which §250 clause 4 requires, and no version, expiry or
  signature. Each is the format ruling's.
- No producer in this tree. The test host composes the stand-in from the build's catalog, in
  Python, apart from the reader on purpose.
- One location per package, and it must be plain HTTP. An HTTPS location would need a pin for its
  host, and §196 (nife carries TLS) holds one root per source.
- A location's host name goes through the resolver the client was granted. A location outside
  that grant's zone cannot be fetched, and the-index-format.md's Q5 holds the question.
- The peer's tampered copy flips one byte and leaves the package's own table of contents alone, so
  the member digest would also refuse it. The test asserts `NotCataloged`, which only the index's
  digest gives, and the falsification shows the difference.
- The resolver page is never returned. One page per program for its life.
