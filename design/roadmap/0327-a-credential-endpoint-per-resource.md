---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 327. A credential endpoint that is the credential for one resource

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 54's block; numbered 2026-09-19 by milestone 433. Premise re-checked 2026-09-19 and
it holds. `crates/credential_protocol`'s `verify::VERIFY` is still one opcode whose request carries
the identity in the shared page (`place(&mut page, b"corinne", b"hunter2", verify::VERIFY)`), so a
holder of a verify endpoint still chooses which record to test. Nothing has narrowed the wire since;
the only change in that crate was the 2026-08-30 removal of `NTLM_PROOF` with the SMB
implementation, which reduced the store to one kind of secret and left the addressing alone.

It changes `credential_protocol`, a contract two programs agree on, so it is
calef's the same way every wire decision in this tree is. The request currently carries the identity
it asks about; removing that word from the wire is not something a lane can decide.

**In brief.** A `VERIFY` request names the resource it is asking about, so the caller chooses which
record to test. That is one authority more than a caller needs. The endpoint should *be* the
credential for one resource, with the name implied by which endpoint you hold and therefore
unforgeable. This is DECISIONS §27's argument, which the tree already accepted elsewhere, applied to
`credential_protocol`.

## Why this matters

The wire is wider than the design. A holder of a verify endpoint can ask about any resource in the
store, so the confinement claim for a program that verifies credentials is "it only asks about its
own" rather than "it cannot ask about anything else", and the difference between those two sentences
is the whole point of a capability system. The second is checkable from outside; the first is a
promise about a branch the program is trusted to take.

**The consumer that motivated it is gone, and that is the honest reason this may sit unpromoted.**
Milestone 54's SMB adapter was deleted on 2026-08-30 with the rest of the network file service, and
it was the caller that named a resource it was configured with. What remains is `credentialer`,
`login`, `identity_provisioner` and their test client, plus `login_protocol` and `system_initializer`
on top. The extra authority is still on the wire and still unchosen, but nothing today is visibly
harmed by it.

What makes it worth doing anyway is the direction of the cost. Every program written against the
current shape makes the change more expensive, and the next thing that authenticates anything will
be written against it. Fixing a wire while it has four callers is a morning; fixing it after a
rebuilt file service is a project. `notes/smb.md` recorded this as a next step rather than as an
accepted limitation, which means nobody ever chose to carry it.

## Where it came from

Milestone 54's block: *"Replace the SMB adapter's resource-name configuration with a narrower
`cred_proto` capability, so the endpoint is the credential for one resource and the name is implied
and unforgeable, which is DECISIONS §27's argument applied to `cred_proto`."* (`cred_proto` has
since been renamed `credential_protocol`.)

`notes/smb.md`'s BUGS section states the shape: *"The right fix is not a configuration string, it is
a narrower capability: a request that names its resource is the adapter choosing which record to ask
about, which is one authority more than it needs, and the endpoint should be the credential for one
resource so the name is implied and unforgeable."*

## Index row

A holder of a `credential_protocol` verify endpoint names the identity it asks about, so the
confinement claim for a program that verifies credentials is "it only asks about its own" rather than
"it cannot ask about anything else", and only the second is checkable from outside. This applies
DECISIONS §27's argument to `credential_protocol`: the endpoint should be the credential for one
resource, with the name implied by which endpoint you hold and therefore unforgeable. The consumer
that motivated it is gone, since milestone 54's SMB adapter was deleted on 2026-08-30, so nothing
today is visibly harmed by the extra authority; what makes it worth doing is the direction of the
cost, because the next thing that authenticates anything gets written against the current shape and
fixing a wire with four callers is a morning.
