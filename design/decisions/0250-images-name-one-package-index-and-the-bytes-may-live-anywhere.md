---
status: AMENDED
raised: 2026-10-03
decided: 2026-10-06
ratified_by: calef
---

# 250. An image names its distribution's package index, at `basalt.nifeos.org`, and a package's bytes may live anywhere

calef, 2026-10-06 (UTC), refining the maintainer's recommendation himself: *"can't we plan to be
able to change where the packages are served from? We just can't change where the list of packages
is served from?"* Then: *"yes, index at packages.nifeos.org."* Later the same day he changed
the host: *"confirm basalt.nifeos.org"*, one host per distribution. *(Section number provisional until
the merge queue lands it.)* Recorded by the maintainer's lane from the milestone 198 (a package
manager) decision review, question 1, researched 2026-10-05 against `main` at `8848a701f`.

*Amended 2026-10-07 (UTC) by calef, on #1805: basalt's index is an image's default source, not its only one, and CI never deploys. See the amendment at the end.*

## The ruling

1. **An image carries one fixed name: its own distribution's package index, which for basalt
   (§247 (the split begins with basalt holding nife)) is at `basalt.nifeos.org`.** The index lists
   packages, their versions, their digests, and where each package's bytes live. The index's path
   and file name on that host are not ruled yet; calef is deciding them, and until he does they are
   pending, not provisional.
2. A package's bytes may be served from any host: GitHub Releases, a CDN, a mirror, plain
   HTTP. They are verified by the digest the index lists, which is §195 (a reviewed recipe vouches
   for a package) and §197 (a package is one archive file) applied unchanged. The package host is
   neither trusted nor pinned, so moving it is an edit to the index and ships no image.
3. The pinned root of §196 (nife carries TLS) clause 4 applies to the index host only. That
   root is ISRG Root X1 (Let's Encrypt), chosen because it survives a change of who serves the
   name: any host that will present a Let's Encrypt certificate for `basalt.nifeos.org` can take
   the name over without an image changing.
4. The index carries a "moved to" field, so the index itself can migrate while the old name
   still answers.

Name: ratified 2026-10-06 (calef, this section). `basalt.nifeos.org` is basalt's index host.
Refused `packages.nifeos.org` (one name for every distribution, so a second distribution could not
have its own pin or migration path), a GitHub host name (H2 below), and no built-in name (H3 below).

## Why one host per distribution

An image names its own distribution's host. A later distribution that is not rolling gets its own
name, its own certificate pin and its own migration path, and none of them is shared with basalt's.
GitHub Pages allows one custom domain per repository, so `nifeos/basalt` can own
`basalt.nifeos.org` outright, and the same host can serve basalt's web page as well as its index.
`packages.nifeos.org`, ruled first the same day, was one name for every distribution and is
refused for those reasons.

## Why the split, and why it is the elegant shape

A name baked into a shipped image is the irreversible part of hosting: it is fixed on every machine
installed from that image until the machine changes its own source. The ruling makes exactly one
thing irreversible, the index's name, and leaves everything else movable. Authenticity of a
package never depended on where it came from (§195's digest decides), so pinning the package host
bought nothing. Authenticity of the *index* does depend on its host, because the index is where the
digests come from, so that is where the pin goes.

Prior art, read 2026-10-05: Debian ships `deb.debian.org`, a project name that is a CNAME to
Fastly; Rust ships `static.rust-lang.org` and `static.crates.io`, both CNAMEs to Fastly. Debian also
splits the signed index (`Release`, `Packages`) from the bytes any mirror serves, which is this
shape.

## What was refused

| Option | Why it lost |
|---|---|
| H2. A GitHub host name baked into images (`github.com/.../releases/download/...` or `ghcr.io`) | The host and its certificate chain are GitHub's to change, and they already have. Measured 2026-10-05: a Release asset now redirects to `release-assets.githubusercontent.com` (Let's Encrypt, ISRG Root X1); milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets) measured the asset host as `objects.githubusercontent.com` on 2026-09-20; `ghcr.io` is Sectigo. Two hosts, two CAs, and one name moved in two weeks. Under §196's one root per source, every shipped image would pin something GitHub can change. GitHub Releases remains a fine place for the *bytes* under clause 2. |
| `packages.nifeos.org`, ruled first and then replaced the same day | One name shared by every distribution: a second distribution would share basalt's pin and could not migrate on its own. See above. |
| H3. No built-in host; the page tells the owner what to type | Adds a step to §157 (a trivial install is a web page, a USB drive, and packages over the internet)'s stranger path, which §157 exists to remove, and a typed URL is a phishing surface the image cannot check. |

At equal cost the answer is the same: the argument is about who controls the name, and H2 was the
cheap option.

## What this unblocks

- Milestone 198's rung 3c has a host to fetch the index from by name, once the index's path and file name are ruled.
- Milestone 501 (a TLS client that speaks to one pinned peer) has its one root: ISRG Root X1, for
  the index host.
- The GPL repository's org (owed to §135 (running GPL software is aggregation)'s amendment) becomes reversible, because no image names an
  org.

## BUGS

- **Choosing a host for the index is now constrained.** Whatever serves `basalt.nifeos.org` must
  present a certificate chaining to ISRG Root X1. A host that issues through another CA (some CDNs
  default to their own) breaks every shipped image until it is reconfigured. Nothing checks this;
  this line is the record.
- **The index's format is not ruled.** It is a wire format two programs agree on, so by rule 7 it
  goes in a crate, and it must be fixed before anyone outside this repository fetches one. The
  "moved to" field's semantics (how often a client follows it, whether it rewrites its own source)
  belong to that ruling.
- The index is authenticated by TLS alone. A per-source signature over the index, which §195
  leaves room for, would let the index itself be mirrored. Not ruled; nothing is blocked on it.
- Root rotation. ISRG Root X1 expires 2035-06-04 (UTC), read from the certificate itself by the
  milestone 501 lane on 2026-10-06; this replaces the earlier *recalled* "2035". A date earlier
  than that one governs: Let's Encrypt now issues from `Root YE` and `Root YR`, which chain to X1
  only through cross-signatures that expire 2032-09-02, three years before X1. A client pinned to
  X1 alone stops verifying Let's Encrypt certificates on that date unless the pin changes. The
  evidence is in milestone 501's BUGS (branch `lane/501-tls`, PR #1759); the rotation itself is
  not solved here.
- Renewing `nifeos.org` (2027-10-03) gates every image that names it. Milestone 198's BUGS carry
  the same line; neither enforces it.
- The index's path and file name on `basalt.nifeos.org` are pending calef's ruling.
- No DNS record or host exists for `basalt.nifeos.org` yet. Creating them is calef's act; agents
  publish nothing under `nifeos.org`.

## Amendment, 2026-10-07 (UTC): the owner chooses the source and the moment

calef ruled two things on pull request #1805, reading the lab self-update proposal
(`design/roadmap/proposals/lab-machines-update-themselves.md`). They are recorded here because this
is the section that rules where an update comes from, and its first clause is the one they change.
§208 (installing is granting) rules what an install changes, and §157 (a trivial install) rules how
a stranger starts; neither names a source.

On when a machine updates: *"CI should not be part of deployment. Owners should decide when and if
to update a machine and can enable automated updates if they choose."*

1. CI is never part of deployment. A gate may publish a release; nothing pushes it onto a machine.
2. The owner decides whether and when a machine updates. Automatic updates are a setting the owner
   may turn on, and a machine with it on pulls from its channel. The lab machines update
   automatically because their owner turned that on.

On where it updates from: *"I want to make certain owners can update off of their own media or
servers. I'm thinking of air gapped deployments for example."* Then *"Yes"* to the following.

3. The source is the owner's setting: basalt's index at `basalt.nifeos.org` (clause 1 above, now the
   default), a mirror or server the owner runs, or local media, air-gapped machines included.
4. The index is a TUF repository (the same day's Fork 9 on #1805). A mirror copies it without
   re-signing, so every copy verifies against basalt's root. An owner may also trust a further
   repository under a root key of their own, as §221 (the boot prompt is the owner's console) lets
   an owner vouch today.
5. Freshness, TUF's timestamp expiry, is enforced by default, and the owner may switch it off per
   source, so older media still installs. Rollback protection, versions that never decrease, is
   always on and has no switch.

Prior art named with the ruling, recalled rather than read for this amendment: Debian's
`apt-offline`, `apt-cdrom` and `Check-Valid-Until=false`; Red Hat Satellite's disconnected mirrors;
WSUS; and Uptane, whose Director repository is a per-fleet source. This binds milestone 809 (the
package client becomes a program), whose source is compiled in today as QEMU's `10.0.2.9:8080`.

