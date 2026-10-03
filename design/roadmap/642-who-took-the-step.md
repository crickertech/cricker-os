---
status: NOT-STARTED
raised: 2026-09-23
promoted_from: who-took-the-step
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 642. The record should say whether a person or the machinery took a step

Promoted from `design/roadmap/proposals/who-took-the-step.md` on 2026-10-03 (UTC). The number 642 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->
<!-- prose-budget: exception. 4835 words (wc -w, this marker included) against a 3,000-word cap. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so its length predates the cap's reach. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by the `maintainer/what-the-machinery-did` lane, which was
sent to make the merge drain's actions countable and found that the logging half (landed in the same
pull request) cannot answer the question calef actually asked. **Name provisional**, this file's and
nothing else's: `who-took-the-step.md` is a lane's coinage and `design/naming.md` is the rule.

Every option below except the refusal puts a long-lived credential on patagonia
or adds an account to the organization. That is calef's call, it is a security decision, and it is
close to irreversible in the sense `AGENTS.md` means: a key that has been on a machine has been on
it. **Nothing here has been implemented and no credential has been placed anywhere.**

## The requirement, stated as a property so each option can be tested against it

calef, 2026-09-23: *"What I care about is attribution to automation versus myself so that we know
when automation is taking a step and when I'm acting manually or through the API on my own
behalf."*

**The property: can a reader of the record tell, for any action, whether a person or the machinery
took it?**

The word doing the work is *any*. A mechanism that labels what the machinery did is only half an
answer, because the other half is the negative: an action **not** labelled must be reliably a
person's. That is the test every option below is scored against, and it is what decides between
them.

## What is attributed to `calef` today, which is three different things wearing one name

Everything. `gh auth status` on patagonia reports one account, `calef`, with scopes
`admin:public_key, gist, read:org, repo, workflow`, and three distinct actors use it:

1. **`helpers/merge-drain.sh`**, running unattended under `launchd`: arms auto-merge, dequeues held
   pull requests, posts stall comments.
2. **A maintainer session and its lanes**: `gh pr create`, `gh pr comment`, labels, merges, and the
   lane worktrees' pushes.
3. **calef himself**, in the browser or from a shell.

GitHub records one actor for all three. So does this project's own log, and so does every pull
request timeline.

## Option (b), refused: attribute in our own logs and take no credential

**This is the cheapest option and it cannot satisfy the requirement**, which is worth writing down
because it is the one that looks like it does. It is what the logging half of this lane's pull
request already builds: `merge-drain: ARMED #N` says the drain enqueued something, countable,
free, no credential anywhere.

**It gives the positive half and not the negative half.** A local log records what automation did.
It cannot establish that an action *missing* from it was calef's, because every gap is ambiguous
between "the automation did not log this" and "calef did it by hand". And the ambiguity is worst
exactly where it matters: when something unexpected happened, which is the only time anybody reads
the record closely. A drain that died mid-pass, a line lost to a rotated log, a code path that
forgot to log, and a person acting by hand all produce the same evidence, which is none.

**Only distinct identities give the negative half**: once the machinery is `smelter[bot]` or
`smelter-bot`, anything still attributed to `calef` is genuinely calef, with no inference required.
That is the whole difference between rung three and rung two of `AGENTS.md`'s ladder, applied to
attribution.

So (b) is refused as an answer. It is kept as a *complement*: the log counts the drain's actions
over time, which no GitHub record does at all (see the audit-log finding below), and it costs
nothing to have both.

## Can `smelter` actually do the drain's work? Mostly, with one live risk

Verified against GitHub's documentation and by introspecting the live GraphQL schema, since the
fork dissolves if the answer is no. `smelter` has exactly **Contents: read/write** and **Pull
requests: read/write** on `nife`.

| What the drain does | Permission | App installation token? |
| --- | --- | --- |
| `gh pr list`, `gh pr view --json ...` | Pull requests: read | yes |
| `gh api repos/O/R/pulls/N`, `.../issues/N/timeline` | Pull requests: read | yes |
| GraphQL `repository.mergeQueue.entries` | repo read | yes |
| `enablePullRequestAutoMerge` (what `gh pr merge --auto` calls) | Pull requests: write | probably, see below |
| `dequeuePullRequest` | Pull requests: write | probably, see below |
| `gh pr comment` | Pull requests: write **or** Issues: write | yes |
| `gh pr create` | Pull requests: write + Contents: read | yes |

**"Probably" is honest rather than lazy, and the reason is structural**: GitHub publishes no
GraphQL-to-fine-grained-permission map, and says so explicitly (the permissions reference is REST
only). What was checked instead is that all three mutations exist in the live schema and that none
of them carries the *"GitHub Apps cannot use this mutation"* note the schema does attach to
restricted mutations. **That is the absence of a prohibition, not a documented grant.**

**One live risk, and it is specific:** `cli/cli#7213`, open and confirmed, reports
`gh pr merge --auto --merge` failing with an opaque *"Something went wrong while executing your
query"* under an App installation token where the same command succeeds under a personal token.
Calling `enablePullRequestAutoMerge` directly through `gh api graphql` may sidestep the CLI path.
**Whichever option is chosen, this is the thing to test first**, on one pull request, before the
drain is pointed at it.

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

**So there is no audit-log API on this organization**, and paying for Enterprise Cloud to answer a
metrics question is not a proposal anybody is making. What identities buy is therefore the **pull
request timeline**, which is per-pull-request and readable, plus whatever this project logs itself.
That is a real argument for keeping the local log whichever option wins: it is the only thing that
can be counted in aggregate.

## With more than one contributor: what is a singleton, what is per-developer

calef, 2026-09-23: *"We should also consider how this works when there are multiple contributors.
What do we run single instances of and what runs per developer. And how do we attribute
correctly?"* This is asked last and answered before the options, because it **repartitions the
problem**, and the partition changes which options are even live.

### Per-developer, and correctly so

Lane worktrees and branches, local gates, and the maintainer session are per-developer by design;
`AGENTS.md` says a lane's isolation *is* its worktree. **`nife-dev` is per developer too, and the
tree's current wording is not wrong but is about to read as if it were.** `AGENTS.md` calls it
"global to the *machine*", which is precise (one `rustup` symlink per user account) and will be
misread the day a second contributor exists, because the obvious next question is whether two
people's lanes fight over it. They do not: it is one symlink per user account per machine, so a
second contributor on a second machine has their own. It is a cross-*lane* hazard, never a
cross-*contributor* one. Worth a one-line clarification in that paragraph when somebody is next in
it; not this lane's to make.

**Plural maintainers are already solved and are not re-derived here.** `AGENTS.md`'s "sessions are
plural" passage holds it with three rules: the merge queue is the single merge authority, anything
minted stays provisional until the queue lands it, and a lane's branch is pushed the moment it is
cut so another session can see it. That is the model, and a second contributor inherits it
unchanged.

### Singleton, and singleton by accident

`helpers/merge-drain.sh`, `helpers/lane-claim-check.sh`, `helpers/trunk-health.sh`, and pull request
#1170's `held-for-red-trunk` hold must each run once. Two drains double-enqueue; two holds race.

**They are single only because they happen to run on one laptop, under one person's `launchd`, as
that person's token, writing to `~/Library/Logs/nife/` where nobody else can read them.** Nothing
enforces the singleness and nothing publishes the state. With a second contributor, none of the
obvious questions has an answer: whose machine runs them, what happens when it sleeps, who notices
when one is stopped.

**The exhibit is from the same evening this was written.** The maintainer unloaded the drain via
`launchctl` while eight pull requests were held out of the queue, deliberately and correctly, and
**no other person or session could have discovered that fact**. It exists in one transcript. That is
AGENTS.md's own rung-zero shape ("somebody will notice") wearing operational clothes, and it is a
worse failure than the attribution gap this proposal was opened for, because a stopped drain is
invisible rather than merely ambiguous.

### The split is cleaner than expected, and it was checked rather than assumed

What each singleton actually reads decides whether it can leave the laptop:

| Script | Reads | Can it move off a laptop? |
| --- | --- | --- |
| `merge-drain.sh` | `gh` against GitHub only; the one local call is `git rev-parse --git-dir`, and that is the guard refusing to *watch* from a worktree | **Yes**, with nothing lost |
| `lane-claim-check.sh` | `git ls-remote --heads origin`, `git rev-list origin/main..origin/<b>`, `gh pr list`: all remote refs | **Yes** |
| `trunk-health.sh`, trunk half | workflow runs on `main`, via `gh` | **Yes** |
| `at-risk-check.sh` (folded into `trunk-health.sh` 2026-09-23) | `git worktree list --porcelain`, then `git -C <path> status --porcelain`: **the laptop's own filesystem** | **No, and never** |

**So `at-risk-check.sh` is genuinely per-developer and the rest are not.** Uncommitted work in a
lane worktree is the one failure in this system that destroys rather than delays, and it is visible
only from the machine holding it. That check belongs on every contributor's machine, one per person,
and it needs **no GitHub write credential at all**: it prints to stdout and acts on nothing.

## Option (e): move the singletons into organization-owned infrastructure

Scheduled GitHub Actions workflows (or cordoba), rather than a contributor's laptop.

**For, and the first point may dissolve the rest of this proposal.**

- **There is no key at rest on anybody's laptop.** `smelter`'s App ID and private key are already
  organization secrets scoped to `nife`, and `actions/create-github-app-token` mints a fresh
  one-hour installation token per run. The credential question that makes (a) and (d) calef's call
  **does not arise** for anything that runs there.
- **It answers the singleton question by construction.** One workflow, one schedule, owned by the
  organization. `concurrency:` prevents overlap. Nobody's laptop sleeping stops it.
- **It publishes its own state.** The Actions run list is a log every contributor can read, with
  timestamps and exit codes, which is strictly more than `~/Library/Logs/nife/` on one Mac.
- **A stopped drain becomes visible.** A disabled workflow shows in the Actions tab; an unloaded
  `launchd` job shows in one transcript.
- **Attribution comes free and is the platform's**: everything the workflow does is `smelter[bot]`.

**Against, honestly, because this is not free.**

- **Cadence.** GitHub's shortest `schedule` interval is five minutes, and scheduled runs are
  **delayed or dropped under load**, which is a real behaviour and not a caveat. The drain currently
  polls every 150 seconds. Slower and lumpier, and probably fine for a queue that merges in groups.
- **`cli/cli#7213` still applies**, since the workflow would hold an installation token too. Same
  premise to test.
- **The comment spam guard is per-pull-request and survives**, since `notify` reads the pull
  request's own comments for its marker rather than any local state. Checked.
- **Two halves of `trunk-health.sh` would have to separate**, because the at-risk half cannot move.
  That is a real refactor of a script two lanes have touched this month, not a `cron` line.
- **cordoba as the alternative host buys the ownership without the cadence loss** and gives back the
  key-at-rest problem, since cordoba would hold a credential. It is a laptop that does not sleep,
  not an answer to the credential question.

**What (e) leaves unanswered**, and this is why it narrows the fork rather than closing it: **a
lane's and a maintainer session's own `gh` calls run on a developer's machine and cannot move.** If
those are also to be `smelter`, a credential has to be local, and (a) against (d) is decided on that
much smaller surface. If they are not, see the next section, where the multi-contributor answer is
better than it first looks.

## What stays a named human, which is the other half calef asked for

**With multiple contributors, each person's own account is their identity, and that is the answer
rather than a gap.** Anything not `smelter` is a named human, and the ambiguity that exists today
exists only because there is exactly one human.

Taking the three conflated actors in turn:

| Actor | Today | Under (e) | Under (e) plus a local `smelter` credential |
| --- | --- | --- | --- |
| The singleton watchers | `calef` | **`smelter[bot]`** | `smelter[bot]` |
| A lane's / session's `gh pr create`, comments, labels, merges | `calef` | `calef`, and with a second contributor, `<that person>`: correct attribution to the human accountable for the lane | `smelter[bot]`, which **loses** which human is accountable unless an instance tag carries it |
| calef by hand or by his own API calls | `calef` | `calef`, and now unambiguously so | `calef` |

**Read the middle row twice, because it reverses the obvious conclusion.** A lane authenticating as
its operator is not a failure of attribution; with several contributors it is the *better* answer,
because the accountable party for a lane is the person who briefed and reviewed it. Making every
lane `smelter[bot]` would flatten several humans into one bot, which is the same defect as today's
with the sign reversed.

What remains genuinely unseparated is **"X by hand" against "X's agent session"**, within one
person. No GitHub identity separates that unless each contributor holds a second credential, which
is a per-person credential-at-rest problem multiplied by the number of contributors. That is the
decision this proposal recommends **not** taking now. The `**Lane:**` convention line from
milestone 128 (the automation gets its own identity, and the agents get their own voice) is what
covers it in the meantime, at rung four and honest about it.

## Which instance acted, a field the model is missing

**"`smelter` did it" stops being an answer the moment automation runs in more than one place**, and
(e) makes that immediate: a scheduled workflow, a contributor's laptop and cordoba could all be
`smelter`. GitHub gives no sub-identity: an installation token carries the App, not the caller, and
a pull request comment posted from a workflow renders as `smelter[bot]` with no link back to the
run.

**So the instance tag has to live in the content, which is rung three, and there is nowhere higher
to reach.** Two places, both of which already exist:

- **The log line.** `merge-drain: ARMED #214 (...)` gains a tag: the workflow run id in Actions, the
  hostname on a laptop. Free, and the log is already the only thing countable in aggregate given
  that this organization has no audit-log API.
- **`notify`'s pull request comment**, which already embeds an invisible HTML marker for
  deduplication. The same marker can carry the instance, so a reader of the pull request can tell
  which drain spoke.

**Not proposed as part of the credential decision**, because it is worth doing under any of the five
options and needs nobody's ruling. Named here so it is not forgotten, in the shape `AGENTS.md` asks
for.

## The five options

### (a) A dedicated machine account, `smelter-bot`, with a PAT on patagonia

A real GitHub user added to `crickertech` with write access to `nife`; local automation
authenticates as it.

**For.** `gh` works unchanged, no JWT-minting step in `merge-drain.sh`. The credential is one
individually-revocable token that mints nothing else. GitHub's terms permit it explicitly: *"You may
maintain no more than one free machine account in addition to your free Personal Account."* It costs
nothing on a free organization.

**Against, and these are what decide it.**

- **A classic PAT is invisible and unrevokable to an organization owner.** GitHub: *"Organization
  owners can only view and revoke fine-grained personal access tokens in this UI, not personal
  access tokens (classic),"* and *"any personal access token (classic) can access organization
  resources until the token expires."* A fine-grained PAT is visible and revokable; a classic one is
  not. Removing the member cuts organization access but leaves the token alive for everything else
  it reaches. This is the single sharpest difference between (a) and (d).
- **Write access is broader than the App's two permissions.** Repository write carries issues,
  releases, wiki, Actions, projects and deployments. Merge-queue admission needs it (*"a user with
  write access to the repository can add the pull request to the queue"*), so it cannot be trimmed.
- **It reads as a human.** A machine account renders as an ordinary user with no badge; the
  `[bot]` suffix and badge are the App's. Distinguishing `smelter-bot` from a person is then a
  convention about a username, which is rung three, and milestone 128's second deliverable
  ("it looks like I'm talking to myself a lot") is exactly the problem conventions did not solve.
- **Two-factor is a new surface.** An unattended account under a 2FA requirement needs a TOTP seed
  stored somewhere, which is a second secret on the laptop rather than a replacement for the first.

**The milestone 128 block refuses a machine account for the toolchain-bump case on the ground that
it hits the same `GITHUB_TOKEN`-cannot-trigger-CI trap. That refusal does not transfer here, and the
claim was checked rather than trusted.** GitHub scopes the rule to that one ephemeral token: *"When
you use the repository's `GITHUB_TOKEN` to perform tasks, events triggered by the `GITHUB_TOKEN`
will not create a new workflow run"*, and the documented fixes are *"a GitHub App installation access
token or a personal access token"*. The drain and a lane's `gh` calls run on a laptop and are not
subject to it at all. Neither option has an edge here, and 128's refusal must not be cited against
(a) in this context.

### (c) Leave local automation as `calef` and accept the gap

**For.** No credential moves, nothing to secure, zero work. It is the status quo and the status quo
has not hurt anybody yet.

**Against.** It fails the property outright, and it fails the milestone this work belongs to. 128's
second deliverable is that the record should say who is talking, and the argument recorded there is
not about convenience: *"a timeline in which the architect appears to write, review and merge his
own work in a single voice is evidence against the claim it should be evidence for."* Accepting the
gap is choosing to keep that.

**It is not nothing, though**, and this is the honest version: with the logging half landed, the
drain's actions become countable for the first time, which answers the *metrics* question calef
started from. (c) is the option that says the metrics question was the real one and the attribution
question can wait. His sharpened wording says it cannot.

### (d) The `smelter` App's private key on patagonia

`merge-drain.sh` and lanes mint an installation token from the App's private key and use it.

**For.**

- **Attribution is rung two rather than rung three.** `smelter[bot]` carries a badge and a suffix
  GitHub renders; nothing has to be remembered or agreed.
- **The permission set is already exactly two and already scoped to `nife` alone**, narrower than
  the repository write (a) needs and far narrower than `calef`'s `repo` scope, which reaches every
  repository he can see.
- **An organization owner can see it, rotate it, suspend it or uninstall it.** Private keys are
  listed in App settings and deletable; a suspended installation *"cannot access resources owned by
  that installation account"*, effective immediately. Compare (a)'s classic PAT, which an owner
  cannot see at all.
- **It is the same identity `toolchain-bump.yml` already uses** once pull request #1167 lands, so
  the tree gains one automation identity rather than two.

**Against.**

- **The key at rest is the durable secret and it does not expire.** GitHub: *"Private keys do not
  expire and instead need to be manually revoked."* A leaked **token** is an hour of exposure and
  dies by itself; a leaked **key** mints fresh tokens indefinitely until somebody rotates it.
- **So the App's usual advantage is weaker on a laptop than it is in Actions**, and this is worth
  stating plainly because it is the part that is easy to get backwards. In a workflow, nothing
  durable is stored on the runner and the App's "nothing stored expires" argument is clean. On
  patagonia the durable thing is the key itself, and a key that does not expire is not obviously
  better than a token that can be revoked individually.
- **The blast radius is not zero.** Contents write on `nife` means force-pushing `main`, and pull
  requests write means opening, commenting and arming anything. Bounded, and narrower than what the
  same laptop already holds.

**The comparison that actually matters, and it is closer than it looks.** The credential already on
patagonia is calef's own token with `repo` and `workflow` across every repository he can reach. Both
(a) and (d) are **narrower than what is already there**, so neither is an increase in exposure on
this machine; the question is what is added, what an owner can see, and what can be revoked without
touching the architect's own access. On those three, (d) wins on visibility and revocation and loses
on the key's immortality.

## Recommendation: (e) first, which defers (a) against (d) rather than deciding it

**The recommendation changed when the multi-contributor question was answered, and saying so is the
point.** Before it, the fork was (a) against (d) and the answer was (d) on visibility and
revocation. After it, most of what needed a credential turns out not to need a **local** one.

### The recommendation

1. **Move `merge-drain.sh`, `lane-claim-check.sh` and the trunk half of `trunk-health.sh` into
   scheduled Actions workflows authenticating as `smelter`.** They read GitHub and nothing else,
   which was checked rather than assumed. This attributes every action they take to `smelter[bot]`
   at rung two, makes the singleton a singleton by construction, publishes the run log where every
   contributor can read it, and **puts no key on anybody's laptop**.
2. **Keep `at-risk-check.sh` per developer**, one per machine, because it reads that machine's
   worktrees and can read nothing else. It needs no credential.
3. **Leave lanes and maintainer sessions authenticating as their operator**, which is already the
   right answer for several contributors and becomes more right as contributors are added.
4. **Add an instance tag** to the drain's log lines and to `notify`'s marker comment, under whichever
   option wins, because "`smelter` did it" is unattributable once `smelter` runs in two places.

**(a) against (d) then applies only to step 3**, if calef decides a lane should be visibly not its
operator. That is a smaller surface, a later question, and the one where a credential at rest
actually bites. Deciding it now would be deciding it on the wrong facts.

### The reason, stated as judgment rather than as effort

**Would we still choose this if all options cost the same? Yes, and this one is not the cheap
option.** (c) is free and (d) is a `create-github-app-token` step; (e) is the most work of the five:
two or three new workflow files, a refactor splitting `trunk-health.sh` in half, a cadence change,
and the `cli/cli#7213` premise to test. Cost is pushing *against* this recommendation, not for it.

Three reasons, in order:

1. **It removes a decision instead of making one.** The credential-at-rest question is the
   irreversible part of this whole area, and (e) means it does not have to be answered for the
   automation that runs today. `AGENTS.md`'s own test is not "can I revert the commit" but "who else
   has already acted"; the best available move on an irreversible fork is the one that makes the
   fork unnecessary.
2. **It fixes a worse problem than the one asked about.** The attribution gap is ambiguity. A
   singleton that exists only because one laptop is awake, whose stopped state lives in one
   transcript, is invisibility, and `AGENTS.md` names that exact shape as the failure the whole
   steward-and-watcher apparatus exists to prevent. This lane found it while answering a different
   question, which is how that failure is normally found.
3. **It is the only option that gets better with a second contributor rather than worse.** (a), (c)
   and (d) all leave "whose machine runs the drain" unanswered, and each new contributor makes the
   question harder. (e) answers it once.

### What is lost, since this is not a clean win

**Cadence**, five minutes at best with delayed and dropped runs under load, against the current 150
seconds. **A refactor** of a script two lanes have touched this month. And **the local drain becomes
harder to run by hand** for a maintainer debugging the queue, though `--once` already works from any
checkout and would keep working. If calef weighs the cadence loss heavily, (d) plus the instance tag
is the fallback, and the ranking of the remaining three is unchanged: (d), then (a), then (c).

## Reversibility, and who has already acted

**The decision is reversible; the key's exposure is not.** Pointing the drain back at calef's token
is a one-line change. But `AGENTS.md`'s test is not "can I revert the commit", it is "who else has
already acted on this", and for a credential the answer is the machine: a key that has sat on a
laptop has sat on it, and reverting does not un-sit it. That asymmetry is why this is an architect's
call and not a lane's.

Who has acted so far: calef created the App, generated a key and stored it as two organization
secrets on 2026-09-23, so **a key file was downloaded to a browser's downloads folder and, per
notes/automation-identity.md's own instruction, deleted afterwards**. Whether that copy still exists
is a fact only calef holds. Nothing else has acted. The private key has not been placed on patagonia
and this lane has not touched it.

## What is blocked until this is answered

Nothing is blocked, and the honest version of that is two sentences rather than one. The logging
half landed independently and answers the metrics question on its own terms.

**If calef says no, or says nothing**, the watchers keep running on patagonia as `calef`, milestone
128's second deliverable stays half-built with the `**Lane:**` convention line doing rung-three
duty, and **the singleton problem keeps its current answer, which is that one laptop happens to be
awake**. That last one is the cost worth weighing, because it is not ambiguity, it is a thing that
can stop without anybody finding out, and it gets worse rather than staying flat as contributors are
added.

## What to do first whichever way it goes

**Test `enablePullRequestAutoMerge` under an installation token on one pull request**, because
`cli/cli#7213` says it may fail where a personal token succeeds. If it does fail and `gh api graphql`
does not sidestep it, **every option that authenticates automation as `smelter` loses the drain**,
(e) included, and the choice narrows to a machine account or the status quo. That single test is
cheap and it is the premise everything else here rests on.

## Index row

The merge drain's log cannot say whether a person or the machinery took a step, which is the question calef asked. Proposed: choose how the record carries that fact; every option except refusal puts a credential on patagonia or adds an account, which is a security decision.
