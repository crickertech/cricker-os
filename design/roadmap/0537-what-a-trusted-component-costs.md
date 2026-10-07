---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: what-a-trusted-component-costs
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 537. What a trusted component costs, in the only currency that travels

*(Number provisional until the merge queue lands it.)* Promoted from the proposal `what-a-trusted-component-costs`, filed 2026-09-21, on calef's instruction of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited except for this paragraph: the argument is its author's and promotion is not the moment to improve it. Raised by the maintainer, from calef's question about how cost per
trusted component would be measured and to what accuracy.

Everything it needs is already produced and thrown away.

## Why this is worth measuring at all

This tree measures scale: milestones, lines, harnesses, proofs, and since 2026-09-20 a velocity
chart. None of it answers the question an outsider actually asks, which is what a trustworthy
component costs to produce and to own.

That is the currency the literature is denominated in. Atmosphere reports 1.5 person-years on
verification alone at a 3.32:1 proof-to-code ratio; seL4 reports about eleven person-years,
plus nine more, at 20:1. Those numbers move institutions in a way a feature list does not, because
they are about what somebody must spend rather than about what somebody built.

**And this project has a denominator it has never written down.** calef works on nife full time, so
one calendar week is one person-week, and the whole project to 2026-09-21 is about ten
person-weeks of human effort. Every other number in the tree should be read against that, and none
of them currently is.

## What to measure: two currencies, and dollars is not one of them

Human effort, which is the scarce one. Decisions taken, reviews given, conflicts resolved,
briefs written, lane output repaired. This is the input a reader outside the project cares about and
the one nothing records.

Machine effort, which is already produced and discarded. Tokens, tool calls and wall clock per
lane, which the task records carry and nothing keeps: today's lanes ran between 184k and 641k tokens
and between 28 and 70 minutes each.

**Not dollars.** Under a fixed subscription the marginal cost of a lane is zero, so "this milestone
cost fourteen dollars" is both true and meaningless. Worse, it invites a comparison against salary
that this project cannot honestly make.

## The accuracy this needs, and it is less than it sounds

**Buckets, not decimals.** The comparisons that matter are 1.5 against 11 person-years, a factor of
seven. Anything that separates hours from days from weeks is enough, so plus or minus a third
is fine and anything tighter spends effort on methodology arguments rather than on the claim.

A published figure should carry no decimal point. "About ten person-weeks" is defensible; "10.4
person-weeks" invites a dispute about whether a Sunday counted, and loses it.

The one number that must be honest is human attention, because it is the scarcest input and the
easiest to under-report. A milestone that took a lane forty minutes, calef three decisions and the
maintainer an hour of conflict resolution cost that, not forty minutes.

## What is cheaply extractable today

- Machine effort per lane: the task records already hold tokens, tool calls and duration.
- calef's involvement, by proxy: `gh` reports review events, label changes, and comments per pull
  request. `needs-architect` labels and `design/decisions/` sections are a second proxy, and a better
  one for decisions than for time.
- Maintainer involvement, by proxy: commits authored on maintainer branches, conflict resolutions
  (a merge commit with conflict markers resolved), and repair commits on a lane's branch.
- What is missing is the join: nothing links a lane to a milestone, so no per-component figure can
  be assembled without one. That is the first thing to build and it is small: the claim commit
  already names the milestone.

## What "a trusted component" means, which must be settled first

Cost per *trusted* component only makes sense once the trusted set is defined, and this tree could
not state its own until 2026-09-21. `notes/trusted-base.md` settles the definition: in a capability
microkernel the trusted base is the kernel, so a trusted component is one inside it, and a userspace
server is not. That note is a prerequisite rather than a nice-to-have, because a cost figure
averaged over both populations answers nothing.

## What this does not propose

No time tracking for calef. A person who must log hours to satisfy a metric will stop
volunteering the honest ones, and the metric is worth less than what it displaces. The denominator
he has already given (one calendar week is one person-week, full time) is accurate enough for every
comparison named above, and the only thing that should ever be asked for is a correction when that
stops being true.

No dashboard. `script/metrics` already carries a weekly series and a chart nobody has to
maintain. If this lands anywhere, it lands there, as a column and a paragraph, with the same honesty
the page already applies to its own undercounts.

## Index row

This tree measures scale: milestones, lines, harnesses, proofs, and since 2026-09-20 a velocity
chart.
