---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: a-risk-entry-should-say-who-argues-it-is-real
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 562. A risk entry should say who published the argument that it is real

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `a-risk-entry-should-say-who-argues-it-is-real` on 2026-09-22, filed 2026-09-21. Raised by the `incremental-path` lane, which noticed the convention
forming and had nowhere to state it.

One line in a file that already exists, and both instances of the pattern it
describes are already written.

## What happened twice

`design/fatal-risks/README.md` risk 4 (a per-crossing cost that cannot be engineered away) grew a paragraph
naming the published counter-thesis, quoting it, and saying what it would mean if it were right. Risk
8 (nobody needs it) has now grown the same shape for a different argument. Risk 1 points at the same
note.

**Two instances are a coincidence and three are a pattern**, and this file's own rule section says
what an entry must contain without mentioning this at all.

## Why it is worth stating rather than leaving to recur

The file's purpose is to hold claims that, if true, mean the project should stop. **An entry that
only this project has ever argued is weaker than one a stranger has published and measured**, and a
reader has no way to tell them apart today. Risk 8's new paragraph carries 1475 bucketed CVEs from
people with no stake here; risk 5's entry carries our own worry. Both are legitimate and they are not
the same kind of evidence.

## What to add, and it is small

One line in that file's `## The rule an entry has to meet`: where a published argument exists that
the risk is real, the entry names it, quotes it rather than paraphrasing, and says what it would take
for the argument to be wrong. Where none exists, the entry says that too, because **"nobody outside
this project has made this case" is itself information** about how seriously to take it.

## What this must not become

**Not a requirement.** An entry with no literature behind it is not thereby less real; risk 5
(multicore reliability) is the hardest thing on the list and its evidence is a board. A convention
that implied otherwise would rank risks by how well studied they are rather than by how fatal.

## BUGS

- **Nothing can gate this.** No check can tell a cited counter-thesis from a paraphrase of one, which
  is the same limit `AGENTS.md` records for routing identified work. It is rung three: a written
  record at the thing itself.

## Index row

Risk 4 grew a published counter-thesis paragraph, then risk 8 did; the file's rule section should say
that an entry names who else argues the risk is real, and says so when nobody does.
