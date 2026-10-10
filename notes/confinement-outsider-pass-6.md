# Confinement outsider pass 6 (2026-10-10 UTC)

The sixth outsider pass at risk 7 (the confinement claim is false), milestone 871 (a sixth outsider
pass attacks the confinement claim), whose number is provisional (minted as 868, renumbered 871 on
2026-10-10 because relibc's seed took 868 on main). By GLM 5.3, the non-Anthropic
model that ran pass 4, run by calef's opencode. It serves criterion (c)'s non-Anthropic half: if
the fifth pass (PR #1895, Anthropic) lands clean, this pass can be the second of the two
consecutive clean passes, and a booted escape here restarts the count instead.

Informed, the posture milestone 800 (a non-Anthropic model attacks the confinement claim) set: the
whole tree and its git history were in hand. The counting rule, the report format and the refusal
log are that milestone's, unchanged. An attack counts when it boots. A verdict from reading alone
is graded `read`, the weakest grade. Only a new finding counts; a re-found fixed escape is a
re-discovery, scored apart.

**Status: one booted escape on the newest shipped surface, ruled an escape by calef, and fixed.**
The failing test was committed before the fix, which turned it green on all three ISAs. The 34-row
sweep and the variant work did not run in this milestone; the block's follow-on holds them.

## The escape: the package client's address check and its connection disagree

Milestone 801 (packages over the internet) shipped its fetch path on 2026-10-10 (PRs #1884 and
#1890). Q1's safeguard, calef's ruling of the same day: *a listed location may never reach a
private or link-local address*, and a client applies `package_index::public_address` to every
address a listed host resolves to. The client
(`pinned_tls_exerciser/src/bin/package_fetch_exerciser.rs`, `from_location`) does exactly that,
on one resolution of the name, and then hands the name, not the checked address, to
`TcpStream::connect((host, port))`, which resolves the name a second time inside the connect.
The resolver has no cache (components/src/name_resolver.rs: "every resolve is a query"), so the
two resolutions are two independent queries, and a rebinding name server answers them
differently: public for the check, private for the connect.

What that reaches: the check approved 192.0.2.1 (RFC 5737 documentation space, public to
`public_address`, never connected to in this attack), and the connect went to 10.0.2.9. The
client completed a TCP connection and a TLS handshake attempt with the peer there. Q1's sentence
is about reach, and the reach happened: a signed index's listed location, checked and approved,
connected to a private address the ruling forbids it from reaching. In production the same shape
is a mirror's DNS rebinding the package client into the owner's own network, with the client's
printed refusals and timings as a probe.

### The boot (aarch64, 2026-10-10 UTC, the commit "871: boot the rebinding reach")

`script/test --arch aarch64 --test a_package_is_fetched_through_the_index_by_name_over_tls_and_judged_by_its_digest`,
with `helpers/name-server-peer` answering `rebind.basalt.test` public on a boot's first query and
at the private peer on every later one, and `helpers/tls-peer` listing a `rebound` twin of
`greeting` under that name. The client printed:

```
passed over https://rebind.basalt.test:8443/rolling/targets/rebound-0.1.0-aarch64.nifepkg: Tls(InvalidCertificate(NotValidForNameContext { expected: DnsName("rebind.basalt.test"), presented: ["DnsName(\"basalt.test\")"] }))
```

The reason is the proof. A certificate name check runs on a certificate a **server sent over a
completed TCP connection**; slirp has exactly one TLS server, the guestfwd peer at the private
10.0.2.9:8443; the check had approved 192.0.2.1. So the connection reached 10.0.2.9, and the
refusal the client reports is the private peer answering. The digest admission held afterwards
(the fetch fell back and was judged), so no untrusted bytes were taken; the escape is the reach
itself, the exact sentence Q1 was ruled to prevent.

The test asserts the confinement (`system_tests/src/user/package_index_tests.rs`: no `Tls(` reason
may appear for a listed location) and is red on the vulnerable tree, committed before any fix
per the standing rule. Any sound fix (connect by a checked address; or re-resolve and re-check
inside the connect) keeps it red-free, because no correct client can complete a TLS exchange with
a host the resolver rebound past the check.

The first answer's state is keyed per boot: the runners export `NIFE_BOOT_TAG` (a fresh value per
emulator start) and the name server keys its first-answer file on it. That is because slirp runs each
guestfwd connection through a short-lived shell whose parent chain is gone before the query is
read. Two parent-chain keyings were tried and broke first (pid reuse poisoned the demo); the tag
is deterministic.

### What this is and is not

- It is a client defect (check-then-use on two resolutions), not a kernel or resolver defect:
  the kernel granted exactly what was asked; the resolver answered what it was asked, twice.
- It is on the fetch path as shipped on main. The boot skips in CI (milestone 855 (the TLS graph
  enters the gated build) owns that). Whether that made it an escape "on a shipped path" was
  calef's verdict, given below.

## The verdict and the fix (calef, 2026-10-10 21:04 UTC, on #1901)

The ruling, recorded on #1901 with `script/record-ruling`:

1. "Yes, it is an escape on a shipped path. It was literally just shipped, but counts." Criterion
   (c)'s count of consecutive independent passes with no escape restarts at zero.
2. "Yes, launch the fix." Resolve once and connect to the exact address that was checked, keeping
   the host name for TLS (the server name and the certificate check).

The fix has the shape of curl's `CURLOPT_RESOLVE` (a name pinned to addresses decided before the
connect) and Go's `net.Dialer.Control` (the check runs on the address actually dialed).
`package_index::Location::check` takes one resolution and returns a `CheckedLocation` holding that
same slice, with private fields, so the only addresses a client can pair with a listed location are
ones the check passed. The exerciser's `get` takes addresses and a TLS server name and never a name
to resolve, so connect-by-name is gone from the fetch path, index and repository included. The
check also refuses an IPv6 answer instead of passing it unjudged, which closes the IPv6 finding
below.

Where the pattern lived: once. The listed-location path was the only place a check came before a
connect. The index addresses and the repository fallback are the image's or the owner's choice,
held to no address check, so connecting by name there was not a check-then-use gap; they now
resolve once anyway, through the same `get`. `pinned_tls_client` takes a connected stream and never
resolves. `package_index` has no network. `jig` (milestone 809 (the package client becomes a
program), PR #1903) dials a fixed address today and gets `Location::check` when it adopts listed
locations.

The red test went green unchanged on aarch64, riscv64 and x86_64. The fixed client dials the
checked public address 192.0.2.1, slirp has nothing there, and the reason it prints is
`ConnectionRefused`. Falsified: with `get` patched back to `TcpStream::connect((server_name, port))`
and the exerciser rebuilt, the test went red on all three ISAs with the private peer's certificate
error, and green again with the patch reverted (patagonia, 2026-10-10). The record is `attested`,
not `replayable`, because no gate builds the exerciser until milestone 855.

## Read-grade findings, homed

- **The address check skipped IPv6.** `from_location`'s check loop only inspected
  `SocketAddr::V4`, so a `SocketAddr::V6` answer passed it untouched, while the ruling's words are
  "a private or link-local address" with no address family named. Unreachable (the tree carries no
  IPv6), so graded `read`. Closed by the fix: `Location::check` refuses an address it cannot judge.
- **`public_address` omits ranges beyond the classics**: 192.0.0.0/24 (RFC 6890 special purpose)
  and 198.18.0.0/15 (RFC 2544 benchmark) are admitted. Both are "not the owner's public internet"
  in spirit; whether Q1's "private" reaches them is a ruling, not a fact. Read, homed here for
  the ruling to cite.

## Machine findings this pass hit (same family as the falsifications grep)

- The TLS peers need a `python3` whose OpenSSL speaks TLS 1.3. Apple's CLT Python 3.9 (what a
  bare `env python3` resolves to in some sessions) raises `ValueError: Unsupported protocol
  version 0x304` at startup, and the guest sees a dead peer. Homebrew's python3.14 works; there
  is no unversioned Homebrew `python3`. Recorded in `helpers/tls-peer`'s BUGS. Boots in this pass
  ran with a session-local `python3` shim to python3.14 on `PATH`.

## Refusal log

Grows with the pass. Format per milestone 800's standing rule; each entry names the claim, says
what was tried in one sentence, and whose refusal it was.

1. **Claim 19/24, the redoxfs name-window TOCTOU.** Declined to boot it here, exactly as passes 4
   and 5 did: a disk fixture plus a racing writer is a milestone of its own, milestone 825 (a
   hostile client races the file server's name window), NOT-STARTED. This pass's own refusal;
   a shipped-path target already homed, so examined, not open.

## BUGS

- This note covers the pass's first day: the booted escape and its fix, two read-grade findings,
  one machine finding. The 34-row table, the variant work against the fixed escapes, and the
  re-discovery count are unwritten. The escape restarted criterion (c)'s count, so the rest of
  this pass could not have made it a clean one; the remaining sweep is the block's follow-on.
- The fixed client sends a real SYN to 192.0.2.1 (documentation space) through slirp, which
  either refuses at once or leaves the net server's connect to give up at its 15 s bound. It was
  refused at once on patagonia on all three ISAs. A hermetic fixture would need a public address
  slirp answers locally, and slirp refuses a `guestfwd` outside its own network.
- The UEFI x86_64 leg (`helpers/qemu-uefi-x86_64.sh`) did not export the boot tag at first, so
  every UEFI boot after the first shared one state file and saw the private answer first. The
  probe there proved nothing and still passed. Found 2026-10-10 on the fix's last boot; that
  runner now exports the tag, and the test fails when the check saw the private answer. Re-booted
  the same day: green with the fix, red with connect-by-name patched back.
- The rebound twin is served `greeting`'s bytes under a `rebound` stem, so the digest admits the
  fetch but the member check refuses it (`NotRequested`). That is harmless for the reach proof,
  since the reach precedes admission, and visible as a line in the exerciser's report.
