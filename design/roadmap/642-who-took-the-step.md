---
status: BUILT
raised: 2026-09-23
built: 2026-10-04
promoted_from: who-took-the-step
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 642. The record should say whether a person or the machinery took a step

Promoted from `design/roadmap/proposals/who-took-the-step.md` on 2026-10-03 (UTC). The number 642 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

Raised by the `maintainer/what-the-machinery-did` lane, which was
sent to make the merge drain's actions countable and found that the logging half (landed in the same
pull request) cannot answer the question calef actually asked. **Name provisional**, this file's and
nothing else's: `who-took-the-step.md` is a lane's coinage and `design/naming.md` is the rule.

Every option below except the refusal puts a long-lived credential on patagonia
or adds an account to the organization. That is calef's call, it is a security decision, and it is
close to irreversible in the sense `AGENTS.md` means: a key that has been on a machine has been on
it. The one credential this milestone caused was the App user token (`ghu_`) calef authorized on
2026-10-04 (UTC) for the #1580 measurement. calef revoked that authorization, which kills its
refresh token too, and disabled Device Flow on `nife-smelter` on 2026-10-04 (UTC), after the test;
the local token file was deleted.

## Built 2026-10-04 (UTC): the machinery is separable, a session is reconstructed, and an agent account is refused

Written by the milestone 642 lane, after the 2026-10-03 correction of error (the queue judged one
pull request at a time, #1564) made this its action item: it could not tell calef's own dequeues
from a maintainer session's. Everything below "The requirement" is the 2026-09-23 proposal, kept as
written. Two of its facts are stale: the organization is `nifeos`, not `crickertech`, and its plan
is now `team` (`gh api orgs/nifeos`). The audit-log API still returns 404, but calef's token lacks
the `read:audit_log` scope, so that 404 no longer proves the plan rule by itself.

### What the 2026-09-23 recommendation decided, and what has happened since

| Step | State |
| --- | --- |
| 1. The watchers move into scheduled Actions as `smelter` | Done (calef, 2026-09-23; AGENTS.md records it) |
| 2. The at-risk check stays per developer | Done |
| 3. Lanes and sessions keep authenticating as their operator | Ruled: they do (below) |
| 4. An instance tag on the drain's lines and comments | Done: `merge-drain[$INSTANCE]` in `helpers/merge-drain.sh` |
| The `cli/cli#7213` premise test | Answered: on 2026-10-03, 10 `auto_merge_enabled` and 24 `added_to_merge_queue` events carry the actor `nife-smelter[bot]` |

### Measured on this repository, 2026-10-03 and 2026-10-04 (UTC)

- **`performed_via_github_app` does not separate the queue steps.** Under the App's installation
  token it is set on comments only, and null on every issue event, the App's own `labeled`,
  `auto_merge_enabled` and `added_to_merge_queue` included. Under an App user token (`ghu_`,
  device flow on `nife-smelter`, measured by the maintainer session on scratch pull request #1580)
  the actor is `calef`. The field reads `nife-smelter` on `labeled` and on comments, but null
  on `unlabeled`, `ready_for_review`, `convert_to_draft` and `auto_merge_enabled`/`disabled`.
  Enqueue and dequeue were not exercised there; auto-merge, the nearest step, carries no mark.
  `calef`'s own `gh` token is `gho_`, an OAuth App's, and marks nothing.
- **GraphQL has nothing.** The timeline event types expose `actor` (and `enqueuer`), never an app.
- **`gh` already tells GitHub that an agent is driving.** `GH_DEBUG=api` shows
  `User-Agent: GitHub CLI 2.101.0 Agent/claude-code_2-1-287_agent`, from the `AI_AGENT` variable
  Claude Code sets. GitHub keeps no user agent anywhere this organization can read.
- **The `**Lane:**` line covers less than half.** Of 836 `calef` comments, 390 open with it, and
  dequeues carry no body at all.

So the only change that would make a session's steps read differently is its credential, and the
only credential measured to separate every step is a separate account.

### The ruling

calef, 2026-10-04 (UTC, after 00:16), on #1575, asked whether agent sessions should act as a
separate account of his, provisionally `calef-agent`: *"I don't want to create calef-agent."* No
credential option is taken. The (g) App user token is not taken either: the measurement above
shows it marks labels and comments but not the queue steps the COE needed.

### What was built

`helpers/session_steps.py` (name provisional), with a fixture self-test in `script/lint`. It credits
a past day's deliberate `calef`-account steps to the agent session whose transcript holds the `gh`
call that took them. On 2026-10-03 it credits 229 of 250 to a session; the 10 unmatched dequeues
include all four calef recalls taking by hand, and it claims one #1530 dequeue the COE gives to
calef, which is its coincidence limit showing. It is option (b) below, refused there as the answer.
With the ruling, it and the `**Lane:**` line are what this project attributes with.

## BUGS

- **The record cannot say, for any action, whether calef or an agent session took it.** That is
  the property this block set, and it is not met. An agent session acts with calef's token, so
  GitHub credits it to `calef`. Attribution is the `**Lane:**` line (rung four, on 390 of 836
  comments, and on no event) plus `helpers/session_steps.py`'s reconstruction from transcripts
  (positive half only, coincidence-prone, Claude Code only, gone when the transcripts age out).
  The next correction of error that needs to know who dequeued inherits this limit. What would
  change it is a separate credential for sessions, which calef refused on 2026-10-04.
- What a transcript match does and does not prove is in `helpers/session_steps.py`'s own BUGS.

## The requirement, stated as a property so each option can be tested against it

calef, 2026-09-23: *"What I care about is attribution to automation versus myself so that we know
when automation is taking a step and when I'm acting manually or through the API on my own
behalf."*

The property: can a reader of the record tell, for any action, whether a person or the machinery
took it?

The word doing the work is *any*. A mechanism that labels what the machinery did is only half an
answer, because the other half is the negative: an action **not** labeled must be reliably a
person's. That is the test every option below is scored against, and it is what decides between
them.

## What is attributed to `calef` today, which is three different things wearing one name

Everything. `gh auth status` on patagonia reports one account, `calef`, with scopes
`admin:public_key, gist, read:org, repo, workflow`, and three distinct actors use it:

1. `helpers/merge-drain.sh`, running unattended under `launchd`: arms auto-merge, dequeues held
   pull requests, posts stall comments.
2. A maintainer session and its lanes: `gh pr create`, `gh pr comment`, labels, merges, and the
   lane worktrees' pushes.
3. calef himself, in the browser or from a shell.

GitHub records one actor for all three. So does this project's own log, and so does every pull
request timeline.

## Option (b), refused: attribute in our own logs and take no credential

This is the cheapest option and it cannot satisfy the requirement, which is worth writing down
because it is the one that looks like it does. It is what the logging half of this lane's pull
request already builds: `merge-drain: ARMED #N` says the drain enqueued something, countable,
free, no credential anywhere.

**It gives the positive half and not the negative half.** A local log records what automation did.
It cannot establish that an action *missing* from it was calef's, because every gap is ambiguous
between "the automation did not log this" and "calef did it by hand". And the ambiguity is worst
exactly where it matters: when something unexpected happened, which is the only time anybody reads
the record closely. A drain that died mid-pass, a line lost to a rotated log, a code path that
forgot to log, and a person acting by hand all produce the same evidence, which is none.

Only distinct identities give the negative half: once the machinery is `smelter[bot]` or
`smelter-bot`, anything still attributed to `calef` is genuinely calef, with no inference required.
That is the whole difference between rung three and rung two of `AGENTS.md`'s ladder, applied to
attribution.

So (b) is refused as an answer. It is kept as a *complement*: the log counts the drain's actions
over time, which no GitHub record does at all (see the audit-log finding below), and it costs
nothing to have both.

*Cut 2026-10-03 (UTC), because they were carried out: the sections on whether `smelter` can do the drain's work, which watchers are singletons, option (e) itself (move the singleton watchers into scheduled Actions workflows as `smelter`), and the instance tag. They read in full at commit e48faf22a.*

## Would GitHub's audit log answer the question once actions are attributed? No

Checked rather than assumed, and it changes the shape of the answer.

The events exist: `merge_queue.pull_request_dequeued`, `merge_queue.pull_request_queue_jump`,
`merge_queue.queue_cleared`, `merge_queue.update_settings`, with 180-day retention. But GitHub's own
sentence is *"Organizations that use GitHub Enterprise Cloud can interact with the audit log using
the GraphQL API and REST API."* Measured on this organization:

```console
$ gh api /orgs/nifeos -q .plan
{"filled_seats":1,"name":"free","private_repos":10000,"seats":0,"space":976562499}
$ gh api /orgs/nifeos/audit-log -X GET
{"message":"Not Found", ..., "status":"404"}
```

So there is no audit-log API on this organization, and paying for Enterprise Cloud to answer a
metrics question is not a proposal anybody is making. What identities buy is therefore the pull
request timeline, which is per-pull-request and readable, plus whatever this project logs itself.
That is a real argument for keeping the local log whichever option wins: it is the only thing that
can be counted in aggregate.

## What stays a named human, which is the other half calef asked for

With multiple contributors, each person's own account is their identity, and that is the answer
rather than a gap. Anything not `smelter` is a named human, and the ambiguity that exists today
exists only because there is exactly one human.

Taking the three conflated actors in turn:

| Actor | Today | Under (e) | Under (e) plus a local `smelter` credential |
| --- | --- | --- | --- |
| The singleton watchers | `calef` | **`smelter[bot]`** | `smelter[bot]` |
| A lane's / session's `gh pr create`, comments, labels, merges | `calef` | `calef`, and with a second contributor, `<that person>`: correct attribution to the human accountable for the lane | `smelter[bot]`, which **loses** which human is accountable unless an instance tag carries it |
| calef by hand or by his own API calls | `calef` | `calef`, and now unambiguously so | `calef` |

Read the middle row twice, because it reverses the obvious conclusion. A lane authenticating as
its operator is not a failure of attribution; with several contributors it is the *better* answer,
because the accountable party for a lane is the person who briefed and reviewed it. Making every
lane `smelter[bot]` would flatten several humans into one bot, which is the same defect as today's
with the sign reversed.

What remains genuinely unseparated is "X by hand" against "X's agent session", within one
person. No GitHub identity separates that unless each contributor holds a second credential, which
is a per-person credential-at-rest problem multiplied by the number of contributors. That is the
decision this proposal recommends **not** taking now. The `**Lane:**` convention line from
milestone 128 (the automation gets its own identity, and the agents get their own voice) is what
covers it in the meantime, at rung four and honest about it.

## The five options

The option-by-option argument, (a) a machine account, (c) leave it as is, and (d) the App key on patagonia, is in [the options appendix](642-who-took-the-step/options.md). In short: (a) and (d) are both narrower than the credential already on patagonia; (d) wins on visibility and revocation and loses on the key never expiring; (c) fails the property outright.

## Recommendation: (e) first, which defers (a) against (d) rather than deciding it

The recommendation changed when the multi-contributor question was answered, and saying so is the
point. Before it, the fork was (a) against (d) and the answer was (d) on visibility and
revocation. After it, most of what needed a credential turns out not to need a local one.

### The recommendation

1. Move `merge-drain.sh`, `lane-claim-check.sh` and the trunk half of `trunk-health.sh` into
   scheduled Actions workflows authenticating as `smelter`. They read GitHub and nothing else,
   which was checked rather than assumed. This attributes every action they take to `smelter[bot]`
   at rung two, makes the singleton a singleton by construction, publishes the run log where every
   contributor can read it, and puts no key on anybody's laptop.
2. Keep `at-risk-check.sh` per developer, one per machine, because it reads that machine's
   worktrees and can read nothing else. It needs no credential.
3. Leave lanes and maintainer sessions authenticating as their operator, which is already the
   right answer for several contributors and becomes more right as contributors are added.
4. Add an instance tag to the drain's log lines and to `notify`'s marker comment, under whichever
   option wins, because "`smelter` did it" is unattributable once `smelter` runs in two places.

(a) against (d) then applies only to step 3, if calef decides a lane should be visibly not its
operator. That is a smaller surface, a later question, and the one where a credential at rest
actually bites. Deciding it now would be deciding it on the wrong facts.

### The reason, stated as judgment rather than as effort

**Would we still choose this if all options cost the same? Yes, and this one is not the cheap
option.** (c) is free and (d) is a `create-github-app-token` step; (e) is the most work of the five:
two or three new workflow files, a refactor splitting `trunk-health.sh` in half, a cadence change,
and the `cli/cli#7213` premise to test. Cost is pushing *against* this recommendation, not for it.

Three reasons, in order:

1. It removes a decision instead of making one. The credential-at-rest question is the
   irreversible part of this whole area, and (e) means it does not have to be answered for the
   automation that runs today. `AGENTS.md`'s own test is not "can I revert the commit" but "who else
   has already acted"; the best available move on an irreversible fork is the one that makes the
   fork unnecessary.
2. It fixes a worse problem than the one asked about. The attribution gap is ambiguity. A
   singleton that exists only because one laptop is awake, whose stopped state lives in one
   transcript, is invisibility, and `AGENTS.md` names that exact shape as the failure the whole
   steward-and-watcher apparatus exists to prevent. This lane found it while answering a different
   question, which is how that failure is normally found.
3. It is the only option that gets better with a second contributor rather than worse. (a), (c)
   and (d) all leave "whose machine runs the drain" unanswered, and each new contributor makes the
   question harder. (e) answers it once.

### What is lost, since this is not a clean win

Cadence, five minutes at best with delayed and dropped runs under load, against the current 150
seconds. A refactor of a script two lanes have touched this month. And the local drain becomes
harder to run by hand for a maintainer debugging the queue, though `--once` already works from any
checkout and would keep working. If calef weighs the cadence loss heavily, (d) plus the instance tag
is the fallback, and the ranking of the remaining three is unchanged: (d), then (a), then (c).

## Reversibility, and who has already acted

**The decision is reversible; the key's exposure is not.** Pointing the drain back at calef's token
is a one-line change. But `AGENTS.md`'s test is not "can I revert the commit", it is "who else has
already acted on this", and for a credential the answer is the machine: a key that has sat on a
laptop has sat on it, and reverting does not un-sit it. That asymmetry is why this is an architect's
call and not a lane's.

Who has acted so far: calef created the App, generated a key and stored it as two organization
secrets on 2026-09-23, so a key file was downloaded to a browser's downloads folder and, per
notes/automation-identity.md's own instruction, deleted afterwards. Whether that copy still exists
is a fact only calef holds. Nothing else has acted. The private key has not been placed on patagonia
and this lane has not touched it.

## What is blocked until this is answered

Nothing is blocked, and the honest version of that is two sentences rather than one. The logging
half landed independently and answers the metrics question on its own terms.

If calef says no, or says nothing, the watchers keep running on patagonia as `calef`. Milestone
128's second deliverable stays half-built, with the `**Lane:**` convention line doing rung-three
duty. The singleton problem keeps its current answer, which is that one laptop happens to be
awake. That last one is the cost worth weighing, because it is not ambiguity, it is a thing that
can stop without anybody finding out, and it gets worse rather than staying flat as contributors are
added.

## Follow-on

- **Refused.** A separate GitHub account for agent sessions (provisionally `calef-agent`, an outside collaborator on `nife`). calef, 2026-10-04 (UTC): *"I don't want to create calef-agent."* The limit it would have removed is in this block's BUGS.
- **Refused.** An App user token for sessions. Measured on #1580 on 2026-10-04 (UTC): it marks labels and comments, not `ready_for_review`, `convert_to_draft` or auto-merge, so it does not separate the queue steps this milestone was opened for.
- **Done.** The watchers act as `nife-smelter[bot]` from scheduled Actions workflows, and the drain's lines and comments carry an instance tag (`helpers/merge-drain.sh`).
- **Done.** Past days are reconstructed by `helpers/session_steps.py`, with its self-test in `script/lint`.

## Index row

The machinery is `nife-smelter[bot]`; an agent session and calef by hand are both `calef`, and calef refused a separate agent account on 2026-10-04. Built: `helpers/session_steps.py`, which reconstructs a past day's session steps from transcripts. The remaining gap is this block's BUGS.
