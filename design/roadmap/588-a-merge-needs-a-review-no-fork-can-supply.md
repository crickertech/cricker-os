---
status: PARTIAL
raised: 2026-09-24
promoted_from: a-merge-needs-a-review-no-fork-can-supply
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 588. A merge needs a review no fork can supply

*(Number minted at promotion.)* Promoted from the proposal
`a-merge-needs-a-review-no-fork-can-supply`, filed 2026-09-24, after calef ruled on it the same
day: **options 2 and 3 adopted, option 1 not taken and still open.** Option 2 is set live (workflow
approval is required for all external contributors). Option 3's tree side is pull request #1215:
the three jobs that mint the App token (`merge-drain.yml`, `trunk-health.yml`, `toolchain-bump.yml`)
name the `automation` environment (name provisional), whose deployment branches are `main` only.
**Option 3 is not protective yet**: the secrets are still organisation-level, which every job can
read whatever its environment, so it becomes real only when calef stores them on the environment
and deletes the org copies (see `## Follow-on`). The text below is the proposal's own, unedited
except for this paragraph, the gate's first sentence, and the `## Follow-on` and `## Index row`
sections; the ruling paragraph under the gate was added to the proposal before promotion. As filed: Raised by the 2026-09-24 security audit
(`design/audit-reports/2026-09-24-new-trust-boundaries.md`, finding 1), which found that
`helpers/merge-drain.sh` armed auto-merge on every open, non-draft pull request against `main`
from any author, that the ruleset on `main` requires zero approving reviews, and that a merge-group
build runs a pull request's own workflow edits with this repository's secrets. The audit closed the
path in the script (`helpers/queue-eligible.jq` refuses a head in another repository); this is the
rung above it.

Option 1 is still an architect's to rule, and the secret move is an organisation
setting only he can make. As filed: every option below is a repository or organisation setting, which is a fact
that leaves the tree: no lane can change it, and every lane works under it from the moment it
changes. It is an architect's.

**Ruled the same day, in part (calef, 2026-09-24 UTC): options 2 and 3 are adopted.** Option 2 is
set: the repository's fork approval policy now reads `all_external_contributors`, so no outside
contributor's workflow runs without a click. Option 3 is pull request #1215 (the App's secrets
move behind a `main`-only environment), which touches the three scheduled workflows. Option 1 is
not ruled and is what this proposal still asks about; its cost stands as written below.

## What is true today

- The repository is public and forkable; 991 pull requests by calef's account, 9 by dependabot,
  none from a fork, ever.
- The `main` ruleset: `required_approving_review_count: 0`, merge queue on, required checks are the
  CI jobs. A pull request the queue can build green is a pull request the queue merges.
- `nife-smelter[bot]` arms auto-merge every five minutes from `merge-drain.yml`, and now only for
  heads in this repository. That is rung two of AGENTS.md's ladder: a gate somebody wrote, in a
  script, that a future edit can un-write. The self-test under `script/lint` is what stops that
  happening silently, and it is still a script.
- Workflows on a first-time contributor's fork wait for a click (`approval_policy:
  first_time_contributors`); a returning contributor's run automatically, with a read-only token
  and no secrets. A merge-group run is in this repository and sees the organisation's secrets,
  `AUTOMATION_APP_ID` and `AUTOMATION_APP_KEY` among them, and runs the workflow file as the pull
  request left it.

## Options, with what each costs

1. **Require one approving review, and let the App give it to a lane.** Set
   `required_approving_review_count: 1`. calef cannot approve his own pull requests, so every lane
   would stall unless something else approves them: `merge-drain.sh`, holding the App token, would
   approve a pull request that passes the same admission predicate before arming it. A fork's pull
   request then needs a person. Cost: the App's review is a rubber stamp by construction, and a
   reader of the pull request page sees "approved by nife-smelter" on work nobody read, which is the
   record lying in a new way. The platform enforces the rule, which is the point; the honesty cost
   is real and should be weighed against it.
2. **Require workflow approval for every outside contributor** (`approval_policy:
   all_external_contributors`). One setting. A fork's pull request then cannot go green without a
   person clicking, so it cannot enter the queue and cannot reach a merge-group run. Cost: a
   collaborator working from a fork waits for a click on every push. No lane is affected.
3. **Put the App's secrets in an environment the queue cannot use.** Move `AUTOMATION_APP_ID` and
   `AUTOMATION_APP_KEY` into an Actions environment whose deployment branches are `main` only, and
   have the scheduled workflows name it. A merge-group ref (`gh-readonly-queue/main/...`) is not
   `main`, so a job on it that named the environment would be refused, and a job that did not
   name it cannot read the secrets. Cost: three workflow files gain an `environment:` line; the
   toolchain-bump workflow, which opens a pull request under the App, keeps working because it runs
   from `main`.

## Recommendation

2 and 3 together, now: each is one setting, neither touches a lane, and between them a stranger's
code cannot run with a secret whether or not the drain is right. 1 is the only one that makes the
review a platform rule rather than a script's, and it is offered rather than recommended because
its cost is a review record that says something false. If calef takes 1, the App's approval
comment should say in its body that it is the admission predicate speaking and not a reader.

## What is blocked until it is answered

Nothing in the tree. The script-level fix holds today; this decides whether it is the only thing
holding.

## Follow-on

- **Outstanding.** calef stores `AUTOMATION_APP_ID` and `AUTOMATION_APP_KEY` as environment
  secrets on `automation`, then deletes the organisation-level copies. Until then option 3 is inert:
  an org secret reaches every job regardless of environment. After it, a dispatch of each of the
  three workflows confirms the jobs still mint the token. Checked 2026-09-24: the environment's
  secrets endpoint (`gh api repos/nifeos/nife/environments/automation/secrets`) lists none.
- **Outstanding.** Option 1, a required approving review the App would give. Not ruled; its cost is
  as written under Options. Checked 2026-09-24: the `main` ruleset's `pull_request` rule still
  reads `required_approving_review_count: 0`.
- **Done.** Option 2, a repository setting made 2026-09-24; the fork approval endpoint reads
  `approval_policy: all_external_contributors`.
- **Done.** Option 3's tree side, pull request #1215, merged 2026-09-24. Dispatch runs from `main`
  after the merge: merge-drain 36052027971 and trunk-health 36052031966 succeeded; toolchain-bump
  36052036224 failed on a quoting bug unrelated to the environment (fixed in #1237) and succeeded
  on re-run 36059259805.

## Index row

A merge-group build runs a pull request's own workflow edits with this repository's secrets, and the `main` ruleset requires no review. calef adopted two of three fixes on 2026-09-24: workflow approval for every outside contributor, and the App's secrets behind a `main`-only environment. The secrets have not yet moved out of the organisation, and a required review stays unruled.
