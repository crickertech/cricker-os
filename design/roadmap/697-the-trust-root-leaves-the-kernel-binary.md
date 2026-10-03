---
status: NOT-STARTED
raised: 2026-09-27
promoted_from: the-trust-root-leaves-the-kernel-binary
milestone_dependencies: 609
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 697. The trust root leaves the kernel binary

Promoted from `design/roadmap/proposals/the-trust-root-leaves-the-kernel-binary.md` on 2026-10-03 (UTC). The number 697 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

calef ruled this on 2026-09-27, beside the ruling that moved the system tests out of the kernel
crate (milestone 609 (the system tests leave the kernel crate)). Today `kernel/build.rs` compiles
the progenitor's digest and the base manifest's digests into `TRUST_ROOT`, so a kernel binary is a
function of every base program's bytes, and no kernel build can happen before userspace is built
and the archive packed. Milestone 609 removed the crate coupling; this is the build coupling that
is left.

The ruled shape: the loader hands the kernel the progenitor's digest and the base manifest's digest,
the kernel checks only the progenitor, and the progenitor checks the base set. What it needs
decided first is the handoff (a boot-protocol field on three architectures, which is a wire format)
and where the loader's own copy of the digests comes from.

The measurements behind the coupling are in pull request #1389's note on packages and divisions,
under "the trust root".

## Index row

A kernel binary is a function of every base program's bytes because the trust root is compiled in. Ruled by calef on 2026-09-27: the loader hands over the digests, the kernel checks the progenitor and the progenitor checks the base set.
