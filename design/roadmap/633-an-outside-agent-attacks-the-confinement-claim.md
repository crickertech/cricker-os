---
status: NOT-STARTED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 633. An outside agent attacks the confinement claim

Raised 2026-10-03 (UTC) by calef, when he ruled risk 7 (the confinement claim is false) AMBER on
#1495. The number is minted by the maintainer; the title and slug are drafts. calef's ask: task an
agent, likely on Fable (model id `claude-fable-5-1`), to be risk 7's adversarial review.

## Why

Risk 7's amber has one named cause: every test and audit of confinement was written by the people
and the model family that built it. The in-house passes found real defects (tests that could not
fail, three times; claims false in milestone 313 (the security audit that was due since August) and on 2026-09-21) and no escape on a
component's own authority. What nobody has run is an attacker who was not in the room.
`design/fatal-risks/README.md` gates the human outsider behind milestone 198 (a package manager,
and the trivial install that makes a second customer possible). This milestone is the half of
that experiment that needs no stranger.

## The experiment

One reviewer agent, briefed with two things only: the 30 published claims in
`notes/confinement-claims.md` and the repository. Independence is the point, so these are the rules:

- No lane reports, no prior audit conclusions, and no hint of where defects were found before. The
  brief names no file under `design/audit-reports/` and none of the earlier falsification notes.
- A different model from the lanes that built and audited confinement. The first choice is Fable.
- It may write exploit programs and run them under QEMU on all three ISAs (aarch64, riscv64 and
  x86_64), through `helpers/qemu-bounded.sh`.
- It reports each attempt as exactly one of: an escape, a near miss, or a claim it found untestable
  (with the reason).

## Premise check

An agent from the same vendor is a weaker outsider than an independent human red team. Shared
training may mean shared blind spots, so a class of defect the builders could not see may be one the
reviewer cannot see either. That bounds what the result can do.

- A clean result moves nothing by itself. It supports that thirty claims survived a second, differently
  trained attacker, which is evidence of the same kind as the audits, a step further from
  self-review. It does not support green, and a green still needs the human half.
- An escape moves the verdict toward red, and is the more useful outcome.
- Untestable claims are findings: each becomes a rewritten claim or a recorded gap.

Record this as the first outsider pass and not the last. The stronger forms are an external human
review, or a public bounty once a stranger can install nife, which waits on milestone 198.

## Done means

- Every one of the 30 claims has an attack, or a written reason it was not attacked.
- Every escape becomes a failing test before any fix is written, in the shape milestone 202 (every
  confinement test is a ritual until somebody breaks the confinement and watches it fail) set.
- The results go into risk 7's appendix (`design/fatal-risks/the-confinement-claims.md`) through
  the maintainer, who may correct facts under §216 (fatal-risk facts are correctable, and verdicts
  are the architect's). Moving the colour stays calef's.

## Cost

An estimate, not a measurement: one long reviewer session per ISA plus the fixes it prompts. The
review itself is a few million tokens at most. The $200 monthly budget is the constraint, so run it
as one lane after the current queue drains and not beside other lanes. QEMU time is small.

## Dependencies and risk link

Nothing blocks it. It cites risk 7 and is the owner of that risk's remaining half in the running
order of `design/fatal-risks/README.md`.

## BUGS

- The reviewer reads the same repository the builders wrote, comments included. Comments that
  say where a check lives are a hint the brief cannot remove.

## Follow-on

- **Recorded.** The human and bounty forms are written in the premise check above, and wait on
  milestone 198.

## Index row

An agent on a different model, briefed with only the 30 published confinement claims, attacks each
one under QEMU on three ISAs and reports escapes, near misses and untestable claims. It is risk 7's
first outsider pass, not its last.
