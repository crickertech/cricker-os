---
status: NOT-STARTED
raised: 2026-09-05
milestone_dependencies: 66
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 387. A TLS stack, and the fork is not "OpenSSL or not" but what we are trying to prove

Filed 2026-09-05 as an unnumbered proposal, written after calef asked
whether a milestone covered OpenSSL; numbered 2026-09-19 by milestone 433's drain of the proposal
pile. **Premise re-read against the tree on 2026-09-19 and still true.** There is still no TLS: no
`rustls`, no `ring`, no `aws-lc-rs` and no OpenSSL anywhere in `Cargo.lock`, and the crypto surface
is still `argon2` and `subtle` in `crates/credentialer` plus the `aes` RedoxFS needs. The "no hash"
claim below wants one correction rather than a retraction: `blake2` and `digest` are in the lock
file, as argon2's own arithmetic, and nothing exposes a general hash. All four consumers named
below are still `NOT-STARTED` (66, 99, 174, 198), so the reason this was filed as a proposal rather
than a milestone, that nothing is blocked on it, is unchanged.
*(Number provisional until the merge queue lands it.)*

That gate covers the server half only; the client half is gated on nothing
and could start today, which the table below sets out. Neither is urgent, and this is a proposal
rather than a milestone for one reason: **nothing is blocked on it today.**

## The principle is already settled, so this is not a write-or-take question

[§46](../decisions/0046-dependency-rule.md) is explicit that crypto goes on the take side, and the
reason is stated: *"correctness there includes resistance to attacks not yet published and
side-channel behavior no specification states, and that is bought by years of exposure and review.
A proof that our AES matches the spec would not make it safe to use."*

So nife takes a TLS stack. The fork is which one, and it splits on what the choice is meant to
demonstrate rather than on engineering taste.

## Three answers, and they are not mutually exclusive

`rustls` is the engineering answer. Rust, no C build system, and it uses the entropy this tree
already has. Milestone 66 already assumes it by name: *"TLS: none. `rustls` needs entropy (have it)
and a large crypto surface."* [§83](../decisions/0083-rust-over-c-implementations.md) points the
same way, and its reasoning is specifically about hostile bytes: the vulnerability history of
comparable parsers *"is dominated by heap overflows and out-of-bounds access. That is the class Rust
removes by construction rather than by care."*

OpenSSL is the ecosystem answer, and the reason is risk 1: *only software written for nife runs
on nife*. Real software links OpenSSL. Choosing `rustls` gives nife TLS; it does not give nife the
ability to run a program that expects `libssl`. Those are different claims and only one of them is
about the ecosystem.

And the third is the interesting one: OpenSSL confined is a demonstration rather than a
dependency. It is the most security-critical C library in the world and the canonical large C blob
that everyone is obliged to trust. This tree has [§31](../decisions/0031-foreign-language-seam.md)'s
seam, `c_shim`, `c_confiner`, `c_swappable`, and milestone 202's 26 enumerated confinement claims
with replayable falsifications. Running OpenSSL where a compromise reaches nothing is
[§14](../decisions/0014-project-direction.md)'s thesis as a concrete object, and it is
[§145](../decisions/0145-compartmentalization-at-process-cost.md)'s argument with a name everybody
recognizes.

Milestone 36 already ranks foreign components and would place this: it calls **SQLite** the
canonical *"C you cannot beat"* and puts it at tier three. OpenSSL is the same tier and a better
demonstration, because unlike SQLite it has a credible Rust alternative, which means taking the C one
would be a choice about what to prove rather than a lack of options.

## The split nobody has written down, and it is the useful part

Client-side TLS and server-side TLS are different milestones with different consumers.

| | who needs it | blocked on |
|---|---|---|
| **client** | milestone 99 (`git` on nife), for the `clone` half; milestone 174 (nife as a thin development client), reaching its build service | nothing |
| **server** | milestone 66 (Vaultwarden) | 66's own concurrency gap: `ACCEPT` re-arms but the backlog is one connection deep |

Client-side needs `connect`, which has existed since the contract did. **So the half with three
consumers is the half that is not blocked**, and it is the one to build first if either is built.

Milestone 174 is explicit that it can wait: *"TLS is not a hard prerequisite for a first cut.
nife has no TLS/crypto stack today."*

## Milestone 198 was listed here as a consumer and should not have been

An earlier version of the table above named milestone 198 (a package manager) as needing client-side
TLS, to fetch what it installs. That is probably wrong, and the reason is worth keeping because it
also answers the question about `ca-certificates` that produced it.

Debian fetches packages over plain HTTP and verifies signed `Release` files with a project key.
Authenticity of the content, not confidentiality of the transport. That is a deliberate design and
not an oversight: it lets any mirror or cache serve the bytes, and it reduces the trust surface from
roughly 150 certificate authorities to one key the project controls.

**So a package manager needs signature verification, which it needs anyway, and does not obviously
need TLS at all.** Signature verification is a smaller piece of the crypto surface than a TLS stack,
it has no certificate parsing and no trust store, and 198 will need it whatever the transport is.

There is also a circularity that makes the TLS answer worse than it looks. A package manager that
fetched over HTTPS would need a certificate trust store; a trust store must be updatable
independently of the system, because authorities are compromised (DigiNotar, 2011) and roots expire
(DST Root X3, 2021, which broke a large fraction of the internet); and the mechanism for updating
data independently of the system is the package manager. Shipping the roots inside the image
resolves it and then freezes them until the next image.

None of that decides 198's transport, which is 198's to decide. It removes 198 from this proposal's
list of consumers, leaving two.

## `ca-certificates` is not a milestone here, and there is a nife-shaped reason

It is a data set rather than code, roughly 150 roots from Mozilla's NSS store, and on every Unix
it is a package. Whatever needs it will need it as data with an update path, which is the paragraph
above.

**And the interesting part is that a trust store should not be ambient here.** On Unix
`/etc/ssl/certs` is readable by every process, so a program that only ever talks to one host can
still verify against all 150 authorities. In a capability system a trust store is a capability,
and a client granted exactly the roots its one peer chains to cannot be induced to trust anything
else. That is a small, concrete instance of
[§145](../decisions/0145-compartmentalization-at-process-cost.md)'s argument, and it is the kind of
thing that is cheap to do here and impossible to retrofit into Unix.

Recorded rather than minted, because it has no consumer until this proposal does.

## Why this is a proposal and not a milestone

No consumer is blocked today. ~~198 has not chosen a format or a transport,~~ (Corrected
2026-10-06: 198 has both, §197 (a package is one archive file) for the format and §196 (nife
carries TLS) for HTTPS with `rustls`, so this section's premise for 198 is overtaken.) 99 is `NOT-STARTED`
and its local half needs no network at all, 174 says it can start without TLS, and 66 is the largest
single item on the roadmap and gated elsewhere. A milestone minted now would sit `NOT-STARTED`
behind four other things and teach nobody anything.

What would turn it into one is any of: 198 choosing a transport that needs HTTPS, 99 reaching the
`clone` half, or a decision that the confined-OpenSSL demonstration is worth doing for its own sake
rather than for a consumer. That last one is the most likely and it is
[§145](../decisions/0145-compartmentalization-at-process-cost.md)'s to trigger.

## What the tree has today

`argon2`, `subtle`, and `aes`. That is the whole crypto surface, and `aes` exists because RedoxFS
needs it. There is no hash, no signature, no key exchange, and no certificate parsing. A package
manager needs signature verification before it needs a transport, which is a smaller and nearer
piece of the same surface and may deserve its own proposal.

## BUGS

- Nobody has checked whether `rustls` builds here. `notes/crates-io-on-nife.md` probed 27 crates
  and `rustls` was not among them. Its dependency graph (`ring` or `aws-lc-rs` by default) is exactly
  the class that note calls C, and `ring` is recorded there as failing class C *"via C and assembly"*.
  **So "rustls is the easy answer" is an assumption this proposal is making and has not tested.**
- The confined-OpenSSL idea is priced nowhere. OpenSSL wants files, sockets, threads, time and a
  build system, and milestone 66 already says the smaller SQLite needs *"the §31 seam plus real
  filesystem locking"*.
- This proposal names no first consumer, which by AGENTS.md's ranking function is a reason to
  rank it below anything that has one.

## Index row

§46 already settles that crypto is taken rather than written, so the question is which stack, and it
splits on what the choice is meant to demonstrate rather than on engineering taste. `rustls` is the
engineering answer, Rust and no C build system over entropy this tree already has, and §83 points
the same way because the vulnerability history of comparable parsers is dominated by the class Rust
removes by construction. OpenSSL is the ecosystem answer, and it answers fatal risk 1 instead: real
software links `libssl`, and giving nife TLS is a different claim from giving it the ability to run a
program that expects OpenSSL. The third answer is the interesting one, since OpenSSL confined is a
demonstration rather than a dependency: the most security-critical C library in the world, run where
a compromise reaches nothing, is §14's thesis as a concrete object and §145's argument with a name
everybody recognizes. The useful half nobody had written down is that client-side and server-side
TLS are different milestones with different consumers: the client half has two (milestone 99's
`clone`, milestone 174's build service) and is blocked on nothing, while the server half has one
(milestone 66) and waits on 66's own one-deep accept backlog. Milestone 198 was listed as a consumer
and removed, because Debian fetches over plain HTTP and verifies a signed `Release` file, so a
package manager needs signature verification, which it needs anyway, and a trust store it could only
update through itself. A certificate trust store is recorded rather than minted here, with the
observation that it should be a capability rather than the ambient `/etc/ssl/certs` every Unix
process can read.
