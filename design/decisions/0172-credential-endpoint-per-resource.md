---
status: PROPOSED
raised: 2026-09-19
---

# 172. Whether a `credential_protocol` verify endpoint names the identity it asks about

Raised 2026-09-19 by milestone 435 (forty-five milestones are gated on a decision nobody wrote down)'s slice B, which read milestone 327 (a credential endpoint that is the credential for one resource)'s
`DECISION` gate and found it naming no section. The question itself is older: milestone 54's block
posed it, `notes/smb.md` recorded it as a next step rather than as an accepted limitation, and the
2026-09-03 proposal sweep carried it forward. *(Section number provisional until the merge queue
lands it.)*

## What is being decided

One question, and it is a wire question: does a `verify` request keep carrying the identity it
asks about, or does the endpoint become the credential for one resource, with the name implied by
which endpoint you hold?

Today `place(&mut page, b"corinne", b"hunter2", verify::VERIFY)` puts an identity and a secret in
the shared page, so a holder of a verify endpoint chooses which record to test. The alternative is
an endpoint minted against one identity, where a caller can present a secret and cannot say whose.

## The tree as it stands, read rather than recalled

`crates/credential_protocol/src/lib.rs` fixes the layout: `ID_OFF = 0`, `SECRET_OFF = 64`,
`MAX_IDENTITY = 64`, `MAX_SECRET = 256`, `LAYOUT_LEN = 320`. The identity is *"an opaque byte string
and nothing more"*, and `place` refuses an empty one.

The extra authority is narrower than "read the store", and the crate says so where a reader meets
it. `MISMATCH` is returned both for the wrong secret and for an identity that is not present,
deliberately: *"Distinguishing them would turn the verify endpoint into an identity"* oracle. So
what a holder can do today is test a guess against any name, not enumerate names. That is the
honest size of the gap and it is smaller than milestone 327's block implies.

The senders, counted rather than estimated (2026-09-19). `verify::VERIFY` leaves exactly two
programs: `components/src/login.rs` at one site, and `fixtures/src/credentialer_test_client.rs`,
the test client, at six. `components/src/credentialer.rs` is the server that answers it, and
`kernel/src/user/credential_tests.rs` asserts the opcode-space property below. Five crates and
program trees depend on `credential_protocol` at all (`components`, `fixtures`, `kernel`,
`crates/system_initializer`, `crates/login_protocol`).

## What this tree already does in the analogous case, which is the strongest argument here

The crate is already built on the rule this proposal extends. `provision::PUT` and
`verify::VERIFY` are both opcode 1, and the module header says why in words this decision can
lean on directly: *"the endpoint gives a number its meaning, not the number itself"*, so a program
holding the verify endpoint that sends `PUT` gets `MISMATCH` rather than a refusal, because *"there
was never a privileged request to refuse"*.

So the question is not whether this tree believes an endpoint should carry authority that a word on
the wire does not. It already does, in this crate, and milestone 65 demonstrated it by collision.
The question is whether the identity is one of those words, or whether it is data the holder is
entitled to choose.

[§27](27-filesystem-service.md) is the argument milestone 327 cites and it is the same
shape one subsystem over. `crates/block_roster`'s header states the general form most sharply:
*"Deliberately not a handle. Holding this tells you a device exists; it does not let you touch it."*

## The options

| | shape | cost, measured where it can be |
|---|---|---|
| **A** | **Keep the identity on the wire.** No change. | Nothing to build. The standing cost is that a program which verifies credentials can be described only as *"it asks about its own"*, which is a promise about a branch rather than a property checkable from outside. |
| **B** | **The endpoint is the credential for one resource.** The identity comes off the request; a verify endpoint is minted per identity and the page carries a secret alone. | Two senders and one server change together (measured above). `LAYOUT_LEN` drops from 320 to 256 and `ID_OFF`/`MAX_IDENTITY` leave the wire. The real cost is on the minting side and is not priced here: something has to hand out one endpoint per identity, and `login` today resolves the identity a person types at a prompt. |
| **C** | **Both**, a narrowed endpoint kind beside the general one. | The general shape survives, so nothing is unbuilt and nothing is un-shipped; two shapes exist forever and a reader has to learn which one a given endpoint is. This tree refuses that kind of doubling elsewhere. |

No recommendation, deliberately. This is a contract two programs agree on, which AGENTS.md puts
in the irreversible column beside names and the syscall surface, and its own limit says a
syscall-surface decision arriving with a recommendation *"is already most of the way made"*. What is
offered instead is the premise check below, which is the thing that would change the answer.

## Is the premise true?

Partly, and the part that is false is the part that would decide B. Milestone 327's block says
the change is cheap now because there are four callers, and that is right as far as the *request*
goes. It does not price the minting side, and B is not a change to a message shape alone: it is a
change to who hands out verify endpoints and on what basis. `login` currently takes an identity from
a person at a prompt and asks the credentialer about it; under B it would have to obtain the
endpoint for that identity from somewhere, and that somewhere is a piece of design this block does
not have.

The consumer that motivated it is gone. Milestone 54's SMB adapter was deleted on 2026-08-30
with the rest of the network file service, and it was the caller configured with a resource name.
Nothing in the tree today is visibly harmed by the extra authority.

## What is blocked until this is answered

Milestone 327, and nothing else. No other block cites it. The argument for answering it anyway
is the direction of the cost rather than a live defect: every program written against the current
shape makes B more expensive, and the next thing that authenticates anything gets written against
whatever is there.

## What this does not decide

Nothing about `provision`. The provisioning endpoint genuinely does name the identity it writes,
because minting a record is the act of naming one, and no reading of this changes that.
