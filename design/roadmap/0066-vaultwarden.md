---
status: NOT-STARTED
raised: 2026-08-02
milestone_dependencies: none
decision_dependencies: 161
machine_requirements: none
specific_machine: none
needs_person: no
---
# 66. Vaultwarden: somebody else's real application, running here

Not started, and this is the largest single item on this roadmap. It is recorded as a
target rather than a plan, and its value today is that it converts "runs real workloads" from a claim
into a checklist.

(The MILESTONE 107 half cleared 2026-08-04, found stale 2026-08-15; the MILESTONE 64 half cleared 2026-09-19, when 64 turned BUILT.) 64 was named as the prerequisite and this is its
extreme case; 107 owns the listen and accept that head the gap table. The decision is the block's
own and is now §161 (which subset counts
as running Vaultwarden), written up 2026-09-19 by milestone 435's lane: which subset counts has to
be settled before the work starts, or the goalposts move to wherever the effort lands. Until then
this gate had named no decision anywhere a reader could open, while the ask sat in this block's own
`BUGS` section addressed to one person.

## Why this application

Vaultwarden is a Bitwarden-compatible server written in Rust: self-hosted, widely deployed, and the
kind of thing calef actually runs. It is not a benchmark or a demo. Getting it working would mean
this system runs software written by people who have never heard of it, which is the difference §14
draws between a demonstrator and a curiosity.

It also lands on the same board as milestones 53 to 55. A VisionFive 2 serving the family's Time
Machine backups and their passwords is a home server, not an exhibit.

## What is actually missing, measured

| Gap | State today |
|---|---|
| **TCP listen and accept** | **built, and this row said otherwise until 2026-09-05.** `OPERATION_LISTEN` and `OPERATION_ACCEPT` have been on the wire since milestone 107 (the socket contract learns to accept), 2026-08-04, and bound into `std`'s PAL by milestone 64 (enough `std` to run somebody else's crate). What remains is **concurrency, not the contract**: the backlog is one connection deep and two cannot be served at once, because the client blocks in one call at a time. For a web application that is the real limit. |
| `std::thread` | 4 of 6 PAL functions answer `Unsupported` |
| `std::fs` | 32 of 54 answer `Unsupported` (milestone 64) |
| async runtime | none. Vaultwarden uses Rocket, which uses tokio: timers, wakers, and a reactor |
| TLS | none, and there is still no crypto stack beyond `argon2`, `subtle` and `aes`. `rustls` needs entropy (have it) and a large crypto surface. The fork between `rustls` and a confined OpenSSL is milestone 387, `design/roadmap/387-a-tls-stack-and-which-one.md`; **server-side TLS is this block's, and client-side has two other consumers that do not need it to be a server** (387 removed the third, milestone 198, on the ground that a package manager wants signature verification rather than a transport) |
| SQLite | a **C library**, so the §31 seam plus real filesystem locking |

The listen/accept question was the interesting one and it has been answered, which is why the
row above changed. A listening socket is a *capability to accept connections on a port* and `accept`
mints a new capability per connection, and milestone 107 settled the shape: `bind` is `OPERATION_LISTEN`,
`accept` is `OPERATION_ACCEPT` into a second socket id with a frame attached, and a listener carries no
frame at all because a listener carries no bytes ([§25](../decisions/0025-socket-identity.md)). The authority
is a listen grant `net_stack` is spawned with, so the same binary is a client or a server depending
on what it was given, and neither is a fallback.

This block had it wrong in two places and half-right in a third, which is worth recording rather
than quietly fixing. The gate line was corrected on 2026-08-15 (*"the MILESTONE 107 half cleared
2026-08-04; found stale"*), and the gap table and this paragraph were not. That is §76's defect class
exactly: a status fixed where somebody happened to look and left wrong where they did not. The gate
gets read when ranking work and the table gets read when scoping it, so the two audiences saw
different answers for three weeks.

The honest remaining gap is smaller and more specific than the one this block claimed. Serving
two connections at once wants userspace threads or a select-shaped wait, which is recorded in
`notes/net.md` beside the verbs and is phase one of the concurrency model rather than a hole in the
contract.

## Its relationship to the rest

- Milestone 64 is the prerequisite and this is its extreme case. 64 measures with small probe
  crates; this is what the measurements are eventually for.
- Milestone 65 (A secrets service) is a different thing wearing a similar word, and conflating them would be a
  mistake worth naming: 65 is a secrets service for the system (keys the OS computes with);
  Vaultwarden is a secrets service for a human (passwords a person retrieves). Different layers,
  different threat models, no shared machinery.
- Milestones 53 to 55 share the board and the thesis.

## BUGS

- **This is a target, not a plan.** Every row in the table above is milestone-sized on its own, and
  several are unsequenced. Treating it as scheduled work would be dishonest about the distance.
- **"Runs Vaultwarden" is not one bit.** It could run with SQLite on a real filesystem and no TLS, or
  behind a TLS terminator, or single-threaded. Which subset counts should be decided before the
  work starts, or the goalposts will move to wherever the effort lands.
- **A capability system may not want to run it unmodified.** Vaultwarden expects ambient filesystem
  and network access. Running it here may mean granting it a directory and a listening socket and
  finding out what it does when it asks for more, which is a more interesting result than success.

**Effort: not estimated, and deliberately not.** The first honest deliverable is the sequence, not a
date.

## Index row

the north star for "runs real workloads". Names the gaps concretely rather than aspirationally: no
TCP listen or accept in the socket contract, threads mostly stubs, most of `std::fs`
unsupported, no async runtime, no TLS, and SQLite is a C library. Largest single item on this
roadmap
