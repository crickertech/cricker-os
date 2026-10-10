# The index's format: a survey, and what is left to rule

An appendix to [notes/packages.md](../packages.md), owed by milestone 801 (packages over the
internet) under §46 (thin primitives or whole subsystems). Its block said nothing outside the tree
was surveyed for the index format and the building lane owes that survey. Written 2026-10-09 (UTC)
by lane `milestone/801-packages-over-the-internet`. This lane made no network call, so every source
is marked: read in this tree, read from a crate's source in the local Cargo registry, or recalled.

## The answer most of this was asking for is already ruled

Two rulings of 2026-10-07 (UTC) on #1805 settle the family. §250 (an image names its distribution's
package index)'s amendment, clause 4: "The index is a TUF repository." And milestone 858 (lab
machines update themselves)'s Fork 9: TUF, with `jig` fetching on rust-tuf over RustCrypto and a
small proven verification core in the progenitor. So the format is TUF's metadata, and the crate
rule 7 asks for is TUF's own definition plus basalt's fields inside it.

What TUF does not say, and what §250 asks of the index, is the open part. Five questions follow the
survey. Two are a format fork (where a package's bytes are named, and "moved to"). One is a
contradiction between rulings. Two are calef's by name.

## The survey

What each system's index lists, where it says a package's bytes are, and how the index itself moves.
"Recalled" means from memory and unchecked; treat those rows as leads.

| System | Index | Digest | Where the bytes are | Index moves by | Authenticity, freshness | Source |
|---|---|---|---|---|---|---|
| Debian apt | `Release` signs `Packages` per suite | SHA-256 per file | `Filename:`, relative to the archive root of whichever mirror the client chose | The client's `sources.list`; `deb.debian.org` is a CNAME | OpenPGP over `Release`, `Valid-Until` | §250 read the split; fields recalled |
| Alpine apk | `APKINDEX.tar.gz` | SHA-1 of the control part, recalled | Relative to the repository URL in `/etc/apk/repositories` | Client configuration | RSA signature inside the archive | recalled |
| Arch pacman | `<repo>.db`, one tarball | SHA-256 and a detached signature per package | `Server =` in the mirror list, relative | Client configuration | Packager keys, database signature optional | §195 read; layout recalled |
| FreeBSD pkg | `packagesite` (a catalog in a tarball) | SHA-256 per package | Relative to the repository URL | `pkg.conf`; SRV records once, recalled | Fingerprint per repository | §195 read; layout recalled |
| Homebrew | Formula JSON from `formulae.brew.sh` | SHA-256 per bottle | An absolute URL per bottle, on GHCR | Changing the API's base URL ships a new `brew` | TLS, and the reviewed formula's digest | §195 and §197 (a package is one archive file) read the trust; URL recalled |
| Haiku | `repo` file plus `repo.info` | SHA-256 per package, in the index only | Relative to the repository's base URL | Client configuration | Not signed, recalled | §197 read the `.hpkg` format |
| Nix | `.narinfo` per store path | NAR hash, and a file hash | `URL:`, relative to the cache, recalled | `substituters` setting | Ed25519 over the narinfo | §195 read |
| crates.io | Sparse index, one file per crate | SHA-256 per version | `dl` in `config.json`, a template | `config.json` at the index root | TLS only | recalled |
| TUF (PyPI's PEP 458) | `targets.json`, delegated roles | Hash and length per target | A targets base URL the client is configured with; per-target `custom` is opaque | A new root can be fetched and verified; repository moves are TAP 4 and TAP 5, recalled | Root threshold, snapshot, timestamp expiry, versions never decrease | 858's appendix read; client source read below |

Two client implementations, read in the local registry this session:

- `tough` 0.24.0 (`src/schema/mod.rs`): a target is `length`, `hashes` and `custom`, and `custom` is
  "opaque to the framework". The repository loader takes `metadata_base_url` and `targets_base_url`
  as two arguments (`src/lib.rs`). Where bytes live is the client's configuration, not metadata.
- `tuf` 0.3.0-beta9 (`src/metadata.rs`): `TargetDescription` is the same three fields.

Neither builds for nife's targets yet (858's appendix: `aws-lc-sys` and `ring`).

## What the survey says about §250

- Every system splits a signed list of digests from the bytes, and every one lets the bytes come
  from more than one host. §250 clause 2 is the mainstream shape.
- Only Homebrew writes an absolute location per package in the index. Everyone else writes a path
  relative to a base the client configures, so moving the bytes means changing every client. TUF is
  in the second group. §250 clause 2 says moving the bytes "ships no image", which the second group
  meets only if the base is not compiled in.
- Nobody but TUF has a "moved to" for the index itself. The rest move the index by editing every
  client's configuration, and Debian hides moves behind a CNAME.

## What calef ruled, 2026-10-10 (UTC)

All five questions were put on #1884 with options, and calef ruled each there; those comments are
the record and quote him. In short, with what milestone 801 built under QEMU and where the rest
lives:

| Question | Ruling | Built in `crates/package_index` | Not 801's, and where it goes |
|---|---|---|---|
| Q1. Where a package's bytes are named | "L1, with the safeguards": a signed, ordered `custom.locations` list per target as additional sources; basalt always keeps a copy in its own `targets/`, the fallback; HTTPS only; no private or link-local addresses; an owner may pin one mirror | `Entry::locations` (HTTPS only), `Index::sources` (listed, then the repository; a pinned mirror alone), `public_address` | Who signs location changes: decided with the signing setup (milestone 858) |
| Q2. What "moved to" means | "M1 plus a backup address": only a signed root names a new location, followed once the new place serves a root chaining to the trusted one; each image carries a second index address | The client tries the image's addresses in order (`package_fetch_exerciser`) | Following a root's move is the TUF client's (milestone 858); §250's wording is question A below |
| Q3. basalt's root in an image | "K4, a default owner trust line": the root's SHA-256 in the form of §220 (signed builds: a vendor signs, a developer self-signs, and trusting a key is scoped), removable by the owner; §220 clause 1 and §195 (a reviewed recipe vouches for a package) clause 4 amended | nothing | The amendments are the integrator's text; the trust line is proposed as a milestone (`design/roadmap/proposals/an-image-carries-its-distributions-root-as-a-default-trust-line.md`) |
| Q4. The path on the host | "Channel prefix, no architecture": `/<channel>/metadata/`, `/<channel>/targets/`, each channel its own TUF repository, all three architectures in one index | `Repository` and its paths; `PROVISIONAL_CHANNEL` is `rolling`, a name calef has not given | Naming the channel is calef's |
| Q5. `jig`'s resolver grant | "Yes, root zone, any repository": `jig` holds the root zone, recorded where granted, with Q1's safeguards; any repository whose root is an owner trust line with a §220 ceiling | The test grants the root zone, and `public_address` is what bounds listed locations | Granting it at the prompt is the proposed std-at-the-prompt milestone; installing from any repository is milestone 809 (the package client becomes a program)'s `add-index` |

The stand-in encoding stays until the TUF client exists: a line is the catalog's line followed by
zero or more HTTPS locations.

## Two questions the rulings raise

**A. Does the backup address touch §250's wording?** Yes, in two places. Clause 1 says "An image
carries one fixed name", and Q2 gives it two. Clause 3 pins ISRG Root X1 "to the index host only".
Recommended: amend clause 1 to one index at up to two addresses, the second under a different
registrable domain so one domain's loss cannot take both, and let clause 3's pin cover both
addresses. If no: the backup cannot be on another domain, and a domain loss still strands a machine.

**B. Which root must a listed HTTPS location's certificate chain to?** §196 (nife carries TLS)
clause 4 holds one root per source and no system store, and Q1 makes every location HTTPS.

| Option | What it costs |
|---|---|
| R1. The index's own pin, ISRG Root X1 | A location on another authority fails and the client falls back to `targets/`. GitHub's release asset host was measured on Let's Encrypt (§250) |
| R2. A small root store for package bytes only | The store §196 clause 4 refused, and the circularity it names |
| R3. Encryption without verifying the certificate | The digest already decides integrity, so TLS buys only privacy from a passive observer, and a client that accepts any certificate is a pattern worth not having |

Recommended: R1. It keeps one root per source and costs nothing but a fallback. The exerciser pins
listed locations to the test authority now, which is R1's shape. If no: R2 or R3 is a decision on
§196 clause 4 first.

## What this does not decide

The TUF client and verifier are milestone 858's items 4 and 5, and the trust table is milestone
666 (a signed build installs up to its key's ceiling)'s. `crates/package_index`'s stand-in
encoding waits on the TUF client, and nothing above `Index::parse` should change when it lands.
