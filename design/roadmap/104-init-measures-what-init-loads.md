---
status: BUILT
raised: 2026-08-04
built: 2026-08-05
---
# 104. The measurement continues past init

Built 2026-08-05 (PR #138). Raised 2026-08-04. Milestone 22 (trusted init) is BUILT, so the sentence it
left behind is no longer a gap in work-in-progress; it is a permanent property of the shipped system
with nobody assigned to it.

Restated 2026-09-26 (UTC) by §219 (how the shell names an installed program to the spawner).
calef ruled option D with gate D2. "Init runs nothing it cannot vouch for" becomes "the progenitor
grants nothing of its own to what it cannot vouch for". Running unvouched bytes takes a capability
the owner gives a session. Such a child gets only the caller's grants and the clock and
configuration pages. Until D is built, the archive is the only source, so the refusal stands.

The finding, in the words of the note that owns it (`notes/trusted-init.md`, "Not covered,
deliberately"): "The kernel measures the program it loads. Every other program in the archive
(`console`, `input`, `shell`, `line_editor`, `net_stack`, `fs_server`, ...) is loaded by init in
userspace, and those bytes are not measured today, so the chain of trust stops at init's entry."

Measured against the archive, that is one program of many. The kernel hashes the boot program with
SHA-256, compares against a digest compiled into its own image, and refuses to enter anything else,
failing closed on a *missing* measurement too (`kernel/src/trust.rs`, `crates/measured_boot`,
milestone 22 phase B.1). Everything after that entry is unchecked bytes, loaded by a program whose
own bytes are checked.

Why the fix is init's and not the kernel's. The note names the crude alternative and rejects it
with a number: hashing the whole archive in the kernel covers everything with one value, and puts a
14 MB hash at every boot plus the policy that decides what is acceptable inside the trusted
computing base. The capability-correct extension is for init to measure what init loads, in
userspace, carrying its own table. That works precisely because milestone 22 already measured init:
init's bytes are vouched for, so init's table is vouched for by the same signature, and the chain
extends by induction rather than by widening the kernel.

This is the same argument supervision makes (§26: the kernel reports a fault, userspace decides what
to do about it), applied to loading instead of dying. Keeping policy out of the kernel is the reason
`crates/measured_boot` currently has no userspace consumer at all, and giving it one is most of the
work.

Named as the follow-up in three places: `notes/trusted-init.md`'s closing section, DECISIONS §26 (the fault endpoint)
phase B's record, and milestone 22's own block. None of them names an owner.

What it costs. A table of digests init carries, produced by the build the same way the kernel's
single digest is; a hash of each program image before `build_child` enters it; and a decision about
what init does with a mismatch. That last one is the only design content: refusing to boot a system
whose shell is unrecognized is the fail-closed floor milestone 22 argues for everywhere else, and it
is also a way to make a machine unbootable from a build defect. Say which, and say it in the block
that decides it.

## Scope note

Not the signature variant, which stays deferred. DECISIONS §26 records a signature over init in
place of a compiled-in digest, and it was deliberately not built: it puts keys, a certificate chain
and signature-verification code inside the trusted computing base, which is exactly what the digest
approach avoids. calef reaffirmed the deferral on 2026-08-03. This milestone extends the *reach* of
the measurement, not its *mechanism*, and the two are independent: a longer chain of digests is
still a chain of digests. The natural sequence §26 records still holds, signatures in addition to
measurement rather than instead of it, if they are ever wanted.

Both ISAs, and note that they load differently. `system_initializer` (riscv64) and `hello`'s
init role (aarch64) each carry their own copy of `build_child`, which is milestone 96's finding.
Measuring in two places is measuring twice; if 96 lands first, this has one site to change instead
of two, and sequencing it after 96 is worth more than the wait costs.

## Follow-on

- **Recorded.** In `notes/trusted-init.md`: the one piece of design content this block left open,
  what init does with a mismatch, was answered by the build. Init runs nothing it cannot vouch for,
  and a refused program is treated exactly as a missing one, so there is no new category. Halting
  the machine for a leaf program was rejected because it turns a build defect in `wc` into an
  unbootable machine, and spawning-and-recording was rejected harder, since there is no audit log to
  record into and a chain whose second link is advisory is not a chain.
- **Milestone 96.** The two copies of `build_child`, one in `system_initializer` and one in
  `hello`'s init role, which would have made this two sites to change instead of one. Sequencing
  after 96 was judged worth more than the wait cost.
- **Refused.** The signature variant stays deferred, reaffirmed by calef on 2026-08-03. A signature
  over init in place of a compiled-in digest puts keys, a certificate chain and
  signature-verification code inside the trusted computing base, which is exactly what the digest
  approach avoids. This milestone extends the measurement's reach, not its mechanism, and DECISIONS
  §26's natural sequence still holds: signatures in addition to measurement, never instead of it.
  This refusal is milestone 450 (design/roadmap/450-a-signature-over-the-init-image.md), which
  carries it with the condition that would change it.

## Index row

Init refuses to load what it cannot vouch for, which turned out to be the whole policy:
spawn-and-record and halt are not rival policies but what a refusal *costs*, and init already
answered that for an archive entry simply missing, so a refused program is treated as a missing
one and no new category exists. The table is an archive entry rather than compiled in, because
init is *in* the archive it measures; the kernel gains one digest and no policy
