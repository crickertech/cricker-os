---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: a-tls-client-that-speaks-to-one-pinned-peer
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 501. A TLS client that speaks to one pinned peer

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-tls-client-that-speaks-to-one-pinned-peer`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Written by the lane for milestone 442 (a crypto provider `rustls` can use on all three bare-
metal targets), which carried that block's clauses 1 and 2 and repriced this one out of it rather
than leaving it unnamed.

Which provider the client is built on is an architect's, stated under "The decision
this leaves" in 442's block. The handshake code is the same either way, so a lane could start
against either and rewire; naming the gate is honest about the fact that it would be building on a
crate nobody has agreed to take.

## What 442 left standing, and what it did not

442 produced a provider that builds and runs on all three architectures and computes what the
specifications say. It produced **no handshake**. `cryptography_exerciser` constructs the provider,
asks what it can negotiate, and stops there, deliberately: there is no peer, no certificate and no
socket in that program at all.

Two things are missing before a client exists, and neither is small.

**An HTTP client, which the tree does not have.** DECISIONS §196 (nife carries TLS: `rustls` for
the protocol, and a crypto provider we make work) says so in its own `BUGS`: a `git grep` for an
HTTP request line in `components/` and `crates/` finds none. `std::net`'s `TcpStream` is bound
(milestones 27 and 64), so it can be an ordinary `std` program, which is the cheap half.

**Certificate verification, which nothing has exercised.** `rustls-webpki` builds on all three
(442's table) and 442 never called it. There is no ECDSA or RSA signature vector in that
milestone's program, which its `BUGS` says plainly, so the largest remaining piece of a handshake
is proven only to compile.

## The shape §196 already chose

**One root, or one pinned key, held as a capability**, for the one repository this client talks to,
rather than a system trust store. §196's clause 4 gives the reason and it is a circularity rather
than a preference: a system-wide store has to be updated independently of the system, and the thing
that updates it is the package manager.

## What it would prove, and what it would not

It closes rung 3c of DECISIONS §157 (a trivial install is a web page, a USB drive, and packages
over the internet): a package fetched by host name from a source somebody else operates. It does **not** buy integrity, which §195 (a recipe vouches,
and the owner may overrule) already gives by digest over any transport; TLS here buys
confidentiality and knowing which host answered.

## BUGS

- **No rotation story.** 442's block carries this and it does not get smaller here: when the one
  pinned key rotates, every installed client is talking to a peer it no longer recognises.
- **No wall clock a stranger's machine can trust**, so certificate expiry is unenforceable in the
  ordinary way. 442's block names this too.
- **No cost is known.** A handshake on a board with no hardware crypto may be slow enough to
  matter, and 442 made it slower by forcing portable implementations on all three architectures.

## Index row

442 produced a provider that builds and runs on all three architectures and computes what the
specifications say.
