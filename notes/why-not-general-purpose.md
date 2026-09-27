# Why nife isn't suited to general-purpose applications

**Rewritten 2026-09-05, because most of what this page said had stopped being true and it was
telling newcomers so with confidence.** The version before this one is in git. It was written when
the project's stated goal was understanding rather than demonstration, and it grounded itself in
that goal by quoting it; AGENTS.md names that framing and says what to do with it: *"This began as a
learning project and pivoted to a demonstrator, deliberately and on the record (2026-07-26). If you
find the old 'understanding is the goal, explain every line as we build it together' framing
anywhere, it is stale."* This page was where it was.

**It also asked to be updated and was not.** Its closing line said to add to it *"when a subsystem
that would move the needle (a writable FS, a POSIX shim, a net stack) actually gets built."* Two of
those three were built and the page kept saying they did not exist. That is AGENTS.md's ladder
working as advertised: a note is rung four, it fires only when somebody remembers, and nobody did.

**The title now overstates the case** and is left alone on purpose, because note names are an
architect's (AGENTS.md's naming rule) and a rename is a naming decision with extra steps. Read it as
*"what a general-purpose application still hits"*, which is what the page is actually about.

## The part that has not changed, and it is the interesting half

**Porting is deliberately hard, and that is a design choice rather than an omission.** nife is
capability-based with **no `open(path)` and no ambient authority**
([§10](../design/decisions/10-capability-microkernel.md)). A program cannot name a file it was not
given, cannot open a socket it was not granted, and cannot widen its own authority. Every assumption
a Unix program makes about reaching for things by name is an assumption this system refuses on
purpose. §10's own words: we are not building the back door.

**Drivers are pushed to userspace for isolation rather than throughput**, which costs crossings and
buys the thing the project exists to demonstrate.

**And the model is not the barrier.** Fuchsia's Zircon is a capability microkernel shipping as a
general-purpose OS on real devices. Nothing here is unsuited *because* it uses capabilities. What is
missing is the userspace built on top, and [§4](../design/decisions/04-kernel-shape.md)'s rules were
chosen to keep those additive rather than blocked: capabilities to a Unix-shaped API is additive,
and the reverse is a rewrite. That asymmetry is what decided §5 and §10.

## What an application actually hits today

The previous version of this table is a useful record of how fast this moves, so what changed is
shown rather than quietly replaced.

| Gap | Then | Today |
|---|---|---|
| **No POSIX, no libc, no `std` target** | *"The big one... You cannot drop in existing software; every program is hand-written against our ABI"* | **Substantially false.** There is a `std` target (the `nife-dev` toolchain and milestone 64), `notes/crates-io-on-nife.md` probed 27 crates against it, and **milestone 121 ran unmodified `ripgrep` with zero patches** on 2026-08-31. That was fatal risk 1's experiment and it came back green. What remains is narrower and is below. |
| **No writable filesystem** | *"nifefs is read-only, one block, built at compile time"* | **False.** Milestone 57's write half landed. |
| **No networking** | *"No TCP/IP, no sockets"* | **False.** `net_stack` (milestone 30) with TCP, UDP and DHCP; listen and accept since milestone 107. (It answered mDNS too, for Time Machine, until milestone 298 retired that on 2026-09-15.) |
| **No display, GUI, or input beyond a serial console** | *"The only I/O to a human is a UART"* | **False.** The compositor landed 2026-08-26, and [§131](../design/decisions/131-hold-at-rung-two.md) is a decision about which rung of the display ladder to stop at rather than an absence. |
| **Tiny, fixed platform** | *"QEMU virt, 128 MiB, single-core until the §11 SMP work lands"* | **False.** Three architectures, four cores on radon, and single-core is now an opt-in `single_hart` feature. |
| **No dynamic linking** | *"Static only; no shared libraries, no `dlopen`"* | **Still true**, and see below. |

## What is genuinely missing, as of 2026-09-05

This is the list to trust, and it is much shorter than the one above it.

- **No DNS.** `smoltcp` is built with `socket-udp`, `socket-tcp`, `socket-dhcpv4` and no
  `socket-dns`, and nothing else in the tree resolves names. **A program cannot turn a hostname into
  an address**, so nothing here can fetch a URL however much HTTP it has. See
  `design/roadmap/384-a-name-resolver-and-who-holds-it.md`.
- **No HTTP, and no TLS.** No client, no server, and a crypto surface of `argon2`, `subtle` and
  `aes`. The TLS fork is milestone 387, `design/roadmap/387-a-tls-stack-and-which-one.md`.
- **No dynamic linking.** Static only, every program a standalone ELF in the archive. This is the
  one row of the old table that survived intact, and it is a real barrier for software that expects
  to `dlopen` a plugin.
- **No async runtime.** Milestone 66 names it: Vaultwarden uses Rocket, which uses tokio, which
  wants timers, wakers and a reactor.
- **Concurrency at the socket layer is phase one.** `ACCEPT` re-arms, but the backlog is one
  connection deep and two connections cannot be served at once.
- **`std` is honest rather than complete.** Milestone 64 measures it: of the PAL's own functions,
  `thread` answers `Unsupported` for 4 of 6 and `fs` for 32 of 54. That is
  [§42](../design/decisions/42-truthful-filesystem.md)'s posture working as designed, and it is still a
  ceiling on what will run.
- **No SQLite, and no drop-in Rust replacement**, which
  [§83](../design/decisions/83-rust-over-c-implementations.md) calls its own limiting case.

## So what it is suited for

**A demonstrator, which is what [§14](../design/decisions/14-project-direction.md) says it is**: a
verified-Rust capability microkernel that runs real workloads, built to stand next to Linux, macOS
and seL4 on the primitives that define an OS. The honest claim is no longer "it cannot run other
people's software", because it ran `ripgrep` unmodified. It is that **the set of other people's
software it can run is bounded by the list above**, and every item on that list is a milestone
somebody could build rather than a consequence of the model.

**What has not changed is the ranking.** AGENTS.md ranks work by the shortest path to a system a
customer runs, and that path has been vacant since 2026-08-30. None of the gaps above is being
worked because an application demanded it.

## BUGS

- **Nothing gates this page.** It went stale in five rows at once and was corrected only because
  somebody asked an unrelated question about `curl`. There is no check comparing a note's claims
  against the roadmap, and the same thing will happen again.
- **The "today" column is a snapshot** taken on 2026-09-05 and will rot the same way. Where a row
  names a milestone or a decision, follow it rather than trusting the summary.
- **"Substantially false" is doing work in the first row.** `ripgrep` ran and reached its own
  argument parsing; that is not the same as a browser running, and milestone 121's own block is
  careful about the difference.
