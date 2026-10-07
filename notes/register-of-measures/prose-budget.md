# The prose budget

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

calef ratified a prose budget on 2026-09-23 (UTC): 3,000 words of main body per document, enforced
as a ratchet. A document already over may not grow, and one under may not cross. The decision is
[§212 (a prose budget)](../../design/decisions/0212-a-prose-budget-for-every-document.md), minted on
2026-09-24; when this panel was written it was still a proposal in pull request #1187. A ratchet is
invisible without a graph, so calef asked for this panel the same day he ratified the cap.

## Two panels

The first chart is the debt: how many words would have to move into appendices for the tree to meet
its own rule. The second is how many documents that work sits in. They are two panels because the
series stack with nothing. They also move independently: splitting one long document cuts the debt
and leaves the count where it was.

## Scope

Both are measured over every `.md` directly under `design/`, `design/decisions/`,
`design/roadmap/`, `notes/` and `briefs/`, plus `AGENTS.md`, plus their appendices. That is the
ruling's own evidence scope with the appendices §212 puts under the same cap. It is one definition,
`documents()` in `helpers/prose_ratchet.py`, which the gate and this chart both read. Words are
whitespace-separated over the whole file, so each appendix is counted as its own document.

Every document is counted, including one carrying a marked exception. Exceptions belong to the gate,
milestone 586 (a prose ratchet in lint). A chart that subtracted exempted documents would hide the
debt, and the debt is what the chart is for.

## BUGS

- A word cap rewards moving prose rather than cutting it, as §212's own `BUGS` says. Every file can
  pass while the tree's total grows, and these series would show that as a falling debt. Neither
  chart measures whether anybody reads the words.
- Closed 2026-09-24 by milestone 586: appendices were not walked, so an appendix over the cap was
  invisible here (found the same day, when this register was split). The scope now includes every
  appendix beside a parent `X.md` and the thematic `design/tenets/`. The current week moved by one
  document.
- The numbers include `AGENTS.md`, while the ratified headline figures (569,775 words, 174
  documents, 2026-09-23) were measured without it. `AGENTS.md` alone is 10,967 words, so the two
  reconcile exactly at 7,967 words and one document.
