---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 824. A retype mints no right its budget lacks

*(Minted 2026-10-07 (UTC) by the maintainer session at the merge of PR #1798, milestone 800 (a
non-Anthropic model attacks the confinement claim), on calef's ruling of decision 6 in that thread.
The number is provisional until the merge queue lands it; the title and slug are drafts.)*

## In brief

`MemoryRegion::RETYPE` hands back a `PageFrame` with `Rights::ALL`, whatever the budget's rights.
calef ruled on 2026-10-07 (UTC), option (a): the frame's rights are the intersection of the budget's
rights and `Rights::ALL`. A `WRITE`-only budget mints `WRITE`-only frames, and `GRANT` can be
delegated but never minted.

## Why

Milestone 633 (an outside agent attacks the confinement claim)'s second pass found it, and its third
booted it as a characterization on all three ISAs: a holder of a budget that cannot delegate retypes a
page and then delegates it. It is the holder's own memory, so it is a scope gap, not a reach into
another domain. Claim 3 of `notes/confinement-claims.md` covers `SPLIT` and says nothing about
retype. Two options were refused with the ruling. Revoking after the fact comes too late, because
the delegation window opens at the mint. Narrowing `GRANT` alone is the intersection with a special
case in it.

## The work

- Intersect the rights at every retype arm in `kernel/src/syscall.rs` that mints from a budget
  (`RETYPE`, and `RETYPE_OBJ` if the same reasoning applies; say which and why).
- Flip `confinement_attack_tests::a_grant_less_budget_mints_a_grant_bearing_frame` from a
  characterization to a held assertion, renamed to say what it now holds, with a replayable
  falsification that restores `Rights::ALL`.
- Extend claim 3's row, or add one, so the table states the retype half.
- Record the method's new semantics in `design/decisions/`, as §10 (process model:
  capability-based, microkernel) requires of a method's behavior. Check every caller that
  retypes from a narrowed budget and then delegates, since it will now be refused.
- Remove the ruling's note from `cap::memory_region_cap`'s `BUGS`.

## Done when

The test holds on aarch64, riscv64 and x86_64, goes red under its record, and the claims table and
the decision record say what a retype mints.

**Reuse:** seL4's retype gives the new object full rights over itself (recalled, not read); this
ruling deliberately departs from it, so the decision record should say why.

## Index row

A frame retyped from a budget carries only the budget's rights, so `GRANT` is delegated and never
minted. Closes the near miss on claim 3 that milestone 633's passes carried.
