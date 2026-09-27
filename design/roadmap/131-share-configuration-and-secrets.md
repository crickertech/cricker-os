---
status: NOT-STARTED
raised: 2026-08-17
milestone_dependencies: none
decision_dependencies: 165
machine_requirements: none
specific_machine: none
needs_person: no
---
# 131. A share is configured, not compiled, and its secret arrives from somewhere

**The subject was removed on 2026-08-30**: this milestone was about
configuring an SMB share and provisioning its secret, and calef decided that day to remove the SMB
implementation (notes/smb.md). There is no share to configure. The status word is unchanged because
the vocabulary has no word for a block whose subject is gone, and minting one is an architect's.

The configuration half needs nothing and has a precedent to copy. The secret
half is a fork calef must rule on, stated in full below and now in
§165 (where a stored secret comes from on a
boot that is not a test), written up 2026-09-19 by milestone 435's lane: **where a secret physically
comes from on a real boot.** That section carries the second question this block leaves to calef as
well, which is whether the block is re-aimed or retired now that its subject is gone. The two are one milestone because they are one act, and because splitting them would
ship a document with a hole where the only value that matters goes.

**What is still live in it, and why it is not simply deleted.** Two of the four deliverables were
never really about SMB. The **configuration document** half generalizes to any service that is
currently a set of compile-time constants, and the retired multicast DNS responder's configuration document is still the shape (notes/mdns.md;
the file itself is at commit `0652c981`). The
**provisioning path** fork is still unanswered and still matters: nothing in the tree can tell a
running system a secret, and the only provisioner is a test program carrying a published fixture.
That is a real gap in the credential service regardless of who its clients are. The other two
deliverables (per-resource credential endpoints for the SMB adapter, and the boot that stops
admitting guests to the share) have no subject left. **Re-aiming or retiring this block is an
architect's call.**

## What "configured" means today, measured rather than characterised

Nothing is configurable. Changing anything about a share means editing Rust and rebuilding.

| Fact about the share | Where it lives today |
|---|---|
| Which behaviour (fixture, read-only, read-write, authenticated) | a `--features` flag choosing one of four numbered constants in `user/src/smb_server.rs` |
| What is served | the whole RedoxFS image; there is no "this directory is a share" notion at all |
| The identity | four constants in `cred_proto::fixture`: resource `backups-chris`, user `User`, domain `Domain`, password `Password` |
| Which resource the adapter authenticates against | the same constant, compiled in and **named in every request** |
| The port | `127.0.0.1:10445`, fixed, which is why two serve boots on one machine collide |

`--features smb_serve` wires `SHARE_FS_READ_WRITE` rather than `SHARE_FS_AUTHENTICATED`, and
`smb_server.rs`'s own header says why: **there is no way to tell that boot a password.** The
identity constants are [MS-NLMP] §4.2.1's published account on purpose, so the gate asserts against
numbers Microsoft printed rather than arithmetic this tree performed, and `cred_proto::fixture`'s
doc is explicit that they are a fixture and not a deployment: *"a real share's account and password
are somebody's, arrive through a provisioning path that does not exist yet, and must never be
these."*

**So this milestone is not "add a config file". It is the first time anything in this system is told
a fact at runtime that it currently learns at compile time**, and the secret is the hardest instance
of that.

## The precedent to copy rather than invent

**Milestone 55's mDNS responder already did the configuration half**, and it is the shape to follow
even though the code is gone (retired 2026-09-15 by milestone 298): a document described what the
machine advertised, `multicast_dns_config` parsed it host-tested with line-numbered errors, and
`notes/mdns.md` records the property that mattered, that what it advertised lived in a document a
person edits rather than in constants. All of it is at commit `0652c981`, and `crates/timetable` is
a live copy of the same shape. A share document is the same move one service over.

## The four deliverables, in order

1. **A share configuration document.** Which directory, read-only or read-write, which resource
   authenticates it, and which port. Parsed by a host-tested crate in `timetable`'s shape (which copied `multicast_dns_config`'s), read at
   boot by whoever wires the adapter.

2. **A provisioning path**: how a secret reaches a running system's credential store. This is the
   fork, and it is below.

3. **Per-resource credential endpoints**, which fall out of (1) rather than needing their own
   argument. Today the adapter names its resource **in the request**, which is one authority more
   than it needs: it is choosing which record in the store to ask about, and nothing structural stops
   it asking about another. `caps` on that adapter cannot show which credential it reaches, because
   the answer is "whichever one it types". If a document says share `backups` uses resource
   `backups-chris`, then **init mints an endpoint that means exactly that credential**, the request
   carries only a challenge and a proof, and the name is not refused but unsayable, because there is
   no field to put it in. That is §27's "the endpoint IS the capability" carried through the one
   place `cred_proto` does not yet carry it, and its own header already argues the general case:
   *the endpoint gives a number its meaning, not the number itself.*

   **It retires a recorded limit for free**, which is usually the sign a shape is right. The
   one-verify-frame limit exists because one endpoint serves everyone; per-resource endpoints want a
   per-client frame anyway.

4. **`smb-serve` stops admitting guests.** Trivial once 1 and 2 exist, and **it is the deliverable
   calef asked to be tracked** (2026-08-17): *"Leave it guest-writable for now. We want to come back
   when we can to locking this down."* Until then the demo boot is guest-writable and says so in its
   banner, and identity is proven in the gate rather than in the demo.

## The fork: where does a secret come from

Milestone 56 built the store and the entropy; it is `BUILT` and it is not this. What is missing is
the path **into** it on a boot that is not a test. Four answers, with what each costs:

- **Typed at a console.** Honest, needs no new authority, and matches how a person expects to set a
  password. It requires an interactive boot to reach the credentialer, and it cannot serve a machine
  that boots unattended, which a backup target eventually is.
- **Read from a separate device** (a USB key, a second partition). Works unattended. Moves the
  question rather than answering it: the secret is now at rest somewhere this system does not
  control, and "who may read that device" is a capability question of its own.
- **Provisioned at image build.** Simplest, and it is what the test path does today. It means the
  secret is in the image, so the image becomes the secret, and every copy of it is a copy of the
  password.
- **Bound by measured boot**, sealed to the measurement milestone 22 already takes. Strongest, and
  the largest: it needs a sealing story this tree does not have, and it fails closed in ways that are
  hard to recover from at 2am.

**No recommendation is recorded here on purpose.** The choice is an architect's, it is a fact that
leaves the machine (a secret's resting place is not un-decidable later), and this block exists so
the options are written down rather than settled by whoever implements first.

## Why it matters

**It is the head of the customer path.** Milestone 54 is `BUILT`: a real Mac mounts the share, reads
and writes byte-correct, sees real free space, walks subdirectories, and proves an identity against a
server that never holds the key. What stands between that and a machine calef's family backs up to is
not more SMB. It is that the share he would actually mount admits guests, because nothing can tell a
running system a real password.

## BUGS

- **This block names four deliverables and only the first is cheap.** Do not read the ordering as an
  estimate; (2) is a design fork and (3) is a wire change to `cred_proto`, which is the irreversible
  category.
- **A configuration document is a new parser reading attacker-adjacent bytes** if it is ever read
  from anywhere but the image. The retired `multicast_dns_config` was host-tested and fuzzed; a share document
  should be held to the same standard on day one rather than after.
- **Nothing here fixes the fixed port.** `smb-serve` binds `127.0.0.1:10445`, so two serve boots on
  one machine still collide; the document is the natural place for it and this block does not require
  it.

## Index row

every fact about a share is a compile-time constant today: a `--features` flag picks one of four
numbered behaviours, the whole image is the share, and the identity is four constants holding
Microsoft's published test account. calef asked for a configuration document and a stored secret
while ruling that `smb-serve` stays guest-writable for now; this tracks the lock-down he asked to
keep. Four deliverables: the document (milestone 55's retired `mdns_responder.conf` is the shape), the
provisioning path (the fork, four options recorded and none chosen), per-resource credential
endpoints (which retire the request-carries-a-name authority and the one-verify-frame limit for
free), and the boot that stops admitting guests. **Subject removed 2026-08-30** with the SMB
implementation (notes/smb.md): there is no share to configure. The configuration half generalizes
to any service and the provisioning fork is still unanswered for the credential store; the block
needs re-aiming or retiring, which is an architect's
