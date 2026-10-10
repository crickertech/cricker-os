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

## What is left to rule

Each question says what is blocked, and what a "no" would mean.

### Q1. Where a package's bytes are named (a format fork)

| Option | Shape | Cost |
|---|---|---|
| L1 | `custom.locations` per target: an ordered list of absolute URLs, the targets role signs it | Moving the bytes re-signs `targets.json`, which a rolling release does every day anyway |
| L2 | TUF's own: bytes at `<targets base>/<hash>.<path>`, and the targets base compiled into the image | Moving the bytes ships an image, which §250 clause 2 refuses |
| L3 | The targets base in a signed root's `custom` field | Moving the bytes is a root rotation, the heaviest act TUF has |

Recommended: L1, with TUF's own layout as the floor. A client tries the listed locations in order
and then the repository's `targets/`, so a mirror that copies the repository (cordoba, or local
media for an air-gapped machine) works with no locations at all. This is §250 clause 1 as written,
and it is the one place basalt's index differs from a plain TUF repository. It would be the same
choice at equal cost: the reason is clause 2, not effort.

Blocked on it: replacing `package_index::Index::parse`. If no, L2 needs §250 clause 2 amended.

### Q2. What "moved to" means (a format fork)

§250 clause 4 asks for the field, and its BUGS line leaves the semantics to this ruling.

| Option | Who may move the index | Client behavior |
|---|---|---|
| M1 | Root keys: a signed root carries the new location | Every update reads it; the client rewrites its own source only after the new place serves a root chaining to the trusted one |
| M2 | The timestamp key, online, in `timestamp.json`'s `custom` | Same, signed by the key most exposed to theft |
| M3 | The host, by an HTTP redirect | TUF still verifies the content, but nothing signed says the move was meant |

Recommended: M1. The index's identity is what root keys exist to vouch for. A client that follows
without verifying would let anyone holding the old name move every machine. Blocked on it: nothing
until the first move. If no, the old name must answer forever.

### Q3. An image would carry basalt's TUF root, and two rulings say no key ships

The amendment says every copy of the index verifies "against basalt's root", so an image carries
that root or something that pins it. §220 (signed builds: a vendor signs, a developer self-signs, and trusting a key is scoped) clause 1 says "No
key ships in any image", and §195 (a reviewed recipe vouches for a package) clause 4 that no
long-lived signing key is held for now.

| Option | What the image carries |
|---|---|
| K1 | `1.root.json` itself |
| K2 | Nothing new: the first root is fetched over the pinned TLS connection and trusted on first use |
| K3 | The SHA-256 of `1.root.json`, beside the catalog in the measured archive |

Recommended: K3, said plainly: it is K1 in effect, since pinning the root's digest pins its keys.
The question for calef is whether §250's amendment amends §220 clause 1 and §195 clause 4 for a
distribution's root. K3 keeps the image a list of digests, as every other trust root here is. K2
makes the WebPKI the root of package trust, which TUF exists to avoid. Blocked on it: any TUF
verification at all, and the key custody it implies (who holds basalt's root keys, and how many).

### Q4. The index's path on `basalt.nifeos.org` (calef's, already pending)

Under TUF the file names are TUF's (`root.json`, `timestamp.json`, `<N>.snapshot.json`,
`<N>.targets.json`), so what is left is a prefix. P1 is the host's root (`/metadata/`,
`/targets/`). P2 is a channel prefix, `/<channel>/metadata/`, which the cordoba spec already uses
as `/lab/` (`notes/lab-index-on-cordoba.md`). Recommended: P2, so a second channel needs no new name.
The stand-in path is `package_index::STAND_IN_INDEX_PATH`, one line to change.

### Q5. A resolver grant is a zone, and bytes may live anywhere

Not a format question, found while building item 3. §252 (a resolver grant is one zone per client
badge) bounds which names a client may resolve, and L1's locations are on any host. Z1 grants the
package client the root zone, which §252's protocol crate names as the grant for a client that needs every name. Z2 requires
locations under the index's own zone. Z3 has the client ask its spawner for each location's zone,
which the spawner cannot judge before the index is verified. Recommended: Z1 for `jig`, recorded
where the grant is made. The resolver grant then bounds nothing the socket grant does not, which is
the honest statement of what "bytes anywhere" costs.

## What this does not decide

The TUF client itself is 858's items 4 and 5 and milestone 809 (the package client becomes a
program)'s. Signing and key custody are 858's and milestone 666 (a signed build installs up to its
key's ceiling)'s. This note decides nothing; the stand-in in `crates/package_index` waits on Q1 and Q2.
