# The NTLM half went with it, and that is the transferable lesson

An appendix to [notes/smb.md](../smb.md), which keeps the lesson in short form. This file holds what
was removed with the SMB implementation on 2026-08-30, why the removal is worth a section, and the
two facts that made it safe.

## What was removed

Removed in the same pass, 2026-08-30:

- `crates/ntlm` entirely;
- the NTLM path through `crates/credentialer`: `Record`'s `nt` field and `has_ntlm` flag,
  `derive_ntlm`, `put_ntlm`, `ntlm_proof`, and the `NTLM_CHALLENGE_LEN`/`NTLM_KEY_LEN` re-exports;
- the `provision::PUT_NTLM` and `verify::NTLM_PROOF` opcodes in `crates/credential_protocol`, with
  their request accessors;
- the four dependency crates that existed only underneath them: `md4`, `md-5`, `hmac` and `digest`.

## Why this is worth a section rather than a line in a commit message

DECISIONS §79 (holding password-equivalent material) approved holding password-equivalent material,
and it approved three known-broken hash functions to go with it. The justification was NTLMv2
protocol compliance. MD4 and MD5 are what the specification names; nothing here chose them, and
shipping them was the same act as implementing DES to talk to old hardware. That reasoning was sound,
and it was entirely contingent on there being a protocol to comply with.

With the SMB implementation gone there is no such protocol. So between the moment the customer moved
and the moment this was noticed, the tree was carrying an `NTOWFv2` and two broken hash functions in
its shipping dependency graph, for a consumer that no longer existed. An `NTOWFv2` is a password
equivalent, crackable at roughly the speed of MD4, sitting beside an Argon2id tag that is not. Nothing
was wrong with the code. What went stale was the *reason*. A reason going stale is invisible in a way
that a broken build is not: every gate stayed green, `cargo-deny` stayed happy, and the security
property the crate documented was still true of the crate.

The lesson for a future reader: a dependency taken for a stated reason should be re-checked when that
reason changes. The place that check can happen is a decision record naming its own premise. §79 named
its premise plainly, which is what made this removal easy to argue for. It is now stale and needs
amending, and the amendment is an architect's.

## Two facts that made the removal safe

Both were verified in the tree rather than assumed.

- Nothing was ever stored. `crates/credentialer`'s own module docs say it outright: *"No persistence.
  A `Store` is memory only, and everything in it dies with the process."* No checked-in fixture and
  no disk image ever held an encoded `Record`, so changing `Record`'s layout migrated nothing.
- The opcode spaces are not positional. `verify::VERIFY` is 1 and `verify::NTLM_PROOF` was 2;
  `provision::PUT` is 1, `SEAL` 2 and `PUT_NTLM` was 3. Removing the NTLM opcodes renumbered nothing.
  It is still a change to a wire contract two programs agree on (AGENTS.md rule 7). It was only safe
  because both programs are in this tree and nothing outside speaks it.

## One casualty worth naming

It was a good piece of writing. `credential_protocol`'s module docs argued "an opcode is not an
authority" using the *collision* between `provision::SEAL` and `verify::NTLM_PROOF` at opcode 2 as
its worked example. They recorded that milestone 65 (a secrets service) demonstrated the principle by
breaking a kernel test when the collision appeared. The principle is unchanged and the example is
gone. The argument now stands on `PUT`/`VERIFY` at 1, which is a weaker illustration of the same true
thing.
