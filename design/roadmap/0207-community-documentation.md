---
status: PARTIAL
raised: 2026-08-31
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 207. The four things GitHub asks for, and which of them this project actually wants

Minted 2026-08-31 by calef, from GitHub's Community Standards checklist. The
pull request template and the issue templates shipped the same day, and the code of conduct and
the content-reports setting were delivered within the week (see `## Follow-on`). One item is left:
a gate that checks the issue forms render. *(Number provisional until the merge queue lands it.)*

This carried `DECISION` until 2026-09-19 and both things it named had been done
for weeks, which is recorded here rather than silently fixed. The gate said two policy questions
were open: who a code of conduct names as its enforcement contact, and the content-reports setting.
`CODE_OF_CONDUCT.md` shipped on 2026-08-31 (commit `7ffa8f3b`) with an enforcement section naming an
address and a subject line, saying outright that it is one person with no rota and that it becomes a
role address when there is a second maintainer; and GitHub's community profile reports
`content_reports_enabled` true, so calef delivered the one item no pull request could. Both are
recorded as `Done` in this block's own Follow-on section, and the gate was left pointing at
them. That is [§76](../decisions/0076-roadmap-status-versus-tree.md)'s defect class again, and it is
the second time this block has had it: its gap table and its gate disagreed for three weeks about
TCP listen and accept. The gate is read when ranking work and the follow-on list is read when
picking it up, so the two audiences saw different answers. What is left needs nobody: nothing
verifies the issue forms render, which is a gate to write rather than a decision to take.

In brief. Measured against `repos/nifeos/nife/community/profile` on 2026-08-31: `README`,
`CONTRIBUTING` and `LICENSE` are present; `CODE_OF_CONDUCT`, an issue template and a pull request
template are missing. GitHub's checklist also offers a fourth item, *Repository admins accept
content reports*, which is a setting rather than a file and cannot be delivered by a pull
request.

This matters for AGENTS.md's third principle rather than for tidiness. A newcomer must be able to
succeed without asking anyone, and these are among the first things a stranger meets.

## The pull request template is the one with a mechanism behind it

AGENTS.md requires that every pull request an agent writes opens by saying so, one line, first
thing in the body. Today that is prose in a brief, remembered by whoever is writing, and lanes have
already failed the neighboring instruction twice this week (milestone 204 (pushed): two lanes pushed
branches and opened no draft pull request, though both briefs named it as the first act).

A template makes the line the default rather than a thing to remember, which is rung two replacing
rung four, and it is the cheapest item here.

## What shipped, and what the premise turned out to be

`.github/pull_request_template.md`. The lane line as default text, then what changed and why
(the diff shows what), the five gates named individually so "not run" is a visible answer rather
than a silence, where identified work went, and a `## Needs an architect` section deleted unless it
applies. The gates are listed by name rather than as one "did you run the gates" line because a
checklist somebody ticks without reading is the thing this was replacing.

`.github/ISSUE_TEMPLATE/`, three forms and a `config.yml`. The section below was written when
issues were disabled and its premise expired within hours of being written, which is the useful part
of the record rather than an embarrassment: calef enabled issues on 2026-08-31, and that picked
the second of the two answers it lays out. So the forms route, which is what that option asked
for:

- A bug report is welcome and prompts for the commit sha, the architecture, and how it was run.
  `CONTRIBUTING.md`'s own `BUGS` section had recorded that nothing prompted for the sha while its
  body told people to supply one, and it had been pointing at a disabled tracker the whole time.
- A feature or scope request says outright that the outcome is a `design/roadmap/` block, links
  the directory, and asks for the material a block needs, including which existing blocks were
  checked.
- A design argument says the outcome is a `design/decisions/` file and asks that file's own
  questions: what is being decided, what the tree does today, the options with a refusal for each,
  and how reversible it is.
- A vulnerability is a `contact_link` to private reporting rather than a form, so it is visible
  on the chooser before somebody opens the wrong thing. A template that says "do not file this
  here" has already failed by being opened.

Blank issues are off. A blank issue is a way to skip every prompt, and the prompts are the
entire reason the forms exist.

Labels are existing ones only (`bug`, `enhancement`, `needs-architect`). A lane does not mint
names global to the tree, and a label is one.

## The issue template was premature, and then it was not

*(Written when `has_issues=false`. Kept because the question it raises is real and was answered
rather than defaulted, and because the answer is what the shipped templates implement.)*

Issues are disabled on this repository (`has_issues=false`). A template would be a file nobody
can reach.

Behind that is a real question this project has never answered: is an issue a channel here at
all? Work lives in `design/roadmap/` and in pull requests, deliberately, and milestone 203 (RedoxFS)'s
(nothing will ever tell us RedoxFS moved) own option list refused an issue as an output on the
grounds that this project does not read them. Two coherent answers:

- Keep issues off, and say so where a stranger looks, so the absence reads as a decision rather
  than neglect. `CONTRIBUTING.md` is the place.
- Turn them on with a template that routes: a bug report is welcome, a feature request belongs
  against the roadmap, and a design argument belongs in `design/decisions/`.

The second becomes materially more attractive once milestone 198 (a package manager, and the trivial
install that makes a second customer possible) exists, because that is when a third party can appear.
**Today there are none**, by calef's own position, so nothing is lost by deciding late.

## The code of conduct is a commitment, not a file

Adopting the Contributor Covenant takes a minute. **What it costs is the promise to enforce it**, and
a project that ships one without meaning it has published a claim it will not honor, which is worse
than silence and is exactly the failure this tree's `BUGS` convention exists to avoid elsewhere.

What has to be decided: **who the enforcement contact is.** Today that is one person, and naming him
is honest; a role address is what a project with contributors would use. §109's (attribution is a
channel property) instinct applies, that a mechanism spelling one person's name has that name as its
failure mode.

## Content reports: calef's, and it is a setting

Enabling *Repository admins accept content reports* sends reports of disruptive content to repository
admins rather than only to GitHub staff, into a Reports tab and optionally an email. It costs
nothing, it cannot be done in a pull request, and its moderation load today is zero because the
project has no third-party contributors.

## BUGS

- The three routing forms are more work to fill in than an issue elsewhere would be, on purpose,
  and that is a cost rather than a free win: a stranger with a small good idea may not write four
  paragraphs about reversibility, and this project will never hear it. The alternative was hearing
  it and having nowhere to put it, which is worse, but the trade is real and unmeasured.
- Nothing verifies the forms render. GitHub validates issue-form YAML when it lands on the
  default branch, so a schema mistake surfaces as a broken chooser rather than as a red check. They
  parse as YAML locally and follow the documented schema, which is not the same claim.
- Three of the four are checklist items and one is a mechanism. The pull request template earns
  its place on its own; the rest are worth doing because a stranger meets them, which is a weaker
  reason and should be said out loud rather than dressed up.
- A code of conduct with an unresponsive contact is worse than none, and nothing in this
  milestone can make anybody responsive.
- **This block does not write any of the text.** Choosing the Contributor Covenant version, the
  template fields and the routing language is the work, and it is all reversible except the
  enforcement promise.
## Follow-on

- **Done.** The code of conduct shipped the same week it was named: `CODE_OF_CONDUCT.md`,
  Contributor Covenant 2.1, commit `7ffa8f3b` dated 2026-08-31, with `CONTRIBUTING.md` pointing at
  it.
- **Done.** The enforcement contact is decided and written where a reporter meets it. The
  enforcement section of `CODE_OF_CONDUCT.md` names an address and a subject line, says outright
  that it is one person with no rota, and records that it becomes a role address when there is a
  second maintainer.
- **Done.** The content-reports setting is on. GitHub's community profile API reports
  `content_reports_enabled` true and the repository at 100%, so the one item no pull request could
  deliver has been delivered by calef.
- **Done.** Choosing the Covenant version was the work this block said it was, and it was done in
  the same commit: 2.1, three routing forms and a chooser config under `.github/ISSUE_TEMPLATE/`,
  and `.github/pull_request_template.md`.
- **Outstanding.** Nothing verifies the issue forms render, and nothing blocks writing the gate (no
  decision, person or hardware): it is a lane's work. None of the workflows under
  `.github/workflows/` reads `.github/ISSUE_TEMPLATE/` and no gate under `script/` parses the YAML,
  so a schema mistake surfaces as a broken chooser rather than as a red check. Checked 2026-09-03.
- **Recorded.** The three routing forms are more work to fill in than an issue elsewhere would be,
  on purpose, so a stranger with a small good idea may not write four paragraphs. The trade is
  stated in this block's own `BUGS` and in `CONTRIBUTING.md`.

## Index row

Minted by calef on 2026-08-31 from GitHub's Community Standards checklist. Measured: `README`, `CONTRIBUTING` and `LICENSE` present; `CODE_OF_CONDUCT`, issue template and pull request template
missing. The PR template is the one with a mechanism behind it, making AGENTS.md's required lane
line a default rather than a thing to remember. Shipped by 2026-09-03: the pull request template and three routing issue forms. They are a bug
report asking for the sha, the ISA and how it was run, a feature request whose outcome is a roadmap
block, and a design argument whose outcome is a decisions file. Also shipped: a vulnerability
contact link, blank issues off, `CODE_OF_CONDUCT.md` with a named enforcement contact, and the
content-reports setting. Issues were never premature, because calef enabled them on 2026-08-31.
Open: nothing checks that the issue forms render, a gate that needs no decision, no person and no
hardware.
