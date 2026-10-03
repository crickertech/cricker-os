---
status: PARTIAL
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 719. Compositor confinement claim 25 is attacked part by part

Raised 2026-10-03 (UTC) by calef as risk 7's compositor pass, the claim milestone 633 (an outside
agent attacks the confinement claim) did not attack because a userspace server enforces it rather
than the kernel. The number 719 is provisional until the queue lands it, and the title and slug are
drafts.

## Index row

Risk 7's compositor pass: the one confinement claim enforced by a userspace server gets an attack and
a replayable falsification for each of its four parts, instead of one.

## Why

`notes/confinement-claims.md` row 25 says *"yes, one of its four"* in the falsified column. The one
test behind it proves four things in sequence, and a patch recorded against it reaches exactly one
(the neighbour write faults). Three parts have no test that could fail on its own, and a first read
of each says why:

1. The input-slot refusal is reachable but has no recorded patch.
2. The neighbour probe is a **write**, so a read-only exposure of the neighbour's pixels (the
   confidentiality breach) leaves it green: a write to a read-only page faults at the same address.
3. The victim's witness digests sit behind the fault wait, so they cannot fire unless that wait
   already has.
4. The screen read is only attempted at an address the fixture never maps, so a screen capability
   leaked to every client leaves it green.

## What is built

New attack roles in `fixtures/src/window.rs` and one test per part, each with a replayable
falsification under `system_tests/falsifications/`. The attacks that went through the compositor's
only untrusted input (the damage rectangle in a client's control page) are recorded with their
outcome in `notes/compositor-claim-25.md`.

## Done means

- Each part has a test that fails when only that part's enforcement is removed, and a patch
  `script/falsifications` replays to show it.
- Any escape is fixed with its test or written up as a proposal under `design/roadmap/proposals/`.
- Row 25's falsification column says what is and is not covered.

## BUGS

- The six patches were replayed by hand (apply, run the one test, require red, reverse), not by
  `script/falsifications --sweep system_tests`, which this lane did not run on the whole package.

## Follow-on

- **Recorded.** A client spawned into a reused window slot would see the previous occupant's pixels:
  `notes/compositor-claim-25.md`, BUGS. Read from the code and not run; nothing respawns a
  client today.
- **Outstanding.** `design/fatal-risks/README.md` and `the-confinement-claims.md` still say row 11 is
  proved on x86_64 only; they change to all three once PR #1534 (W^X on aarch64 and riscv64) merges. Not done
  here because #1534 had not merged.
- **Recorded.** Claim 25's falsifications are recorded on aarch64 only: `notes/compositor-claim-25.md`.
