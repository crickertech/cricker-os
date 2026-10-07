# The five options, as argued in 2026-09

This appendix belongs to [milestone 642 (the record should say whether a person or the machinery took a step)](../0642-who-took-the-step.md); it holds the option-by-option argument behind that block's recommendation.

### (a) A dedicated machine account, `smelter-bot`, with a PAT on patagonia

A real GitHub user added to `crickertech` with write access to `nife`; local automation
authenticates as it.

For. `gh` works unchanged, no JWT-minting step in `merge-drain.sh`. The credential is one
individually-revocable token that mints nothing else. GitHub's terms permit it explicitly: *"You may
maintain no more than one free machine account in addition to your free Personal Account."* It costs
nothing on a free organization.

Against, and these are what decide it.

- A classic PAT is invisible and unrevokable to an organization owner. GitHub: *"Organization
  owners can only view and revoke fine-grained personal access tokens in this UI, not personal
  access tokens (classic),"* and *"any personal access token (classic) can access organization
  resources until the token expires."* A fine-grained PAT is visible and revokable; a classic one is
  not. Removing the member cuts organization access but leaves the token alive for everything else
  it reaches. This is the single sharpest difference between (a) and (d).
- Write access is broader than the App's two permissions. Repository write carries issues,
  releases, wiki, Actions, projects and deployments. Merge-queue admission needs it (*"a user with
  write access to the repository can add the pull request to the queue"*), so it cannot be trimmed.
- It reads as a human. A machine account renders as an ordinary user with no badge; the
  `[bot]` suffix and badge are the App's. Distinguishing `smelter-bot` from a person is then a
  convention about a username, which is rung three, and milestone 128's second deliverable
  ("it looks like I'm talking to myself a lot") is exactly the problem conventions did not solve.
- Two-factor is a new surface. An unattended account under a 2FA requirement needs a TOTP seed
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

For. No credential moves, nothing to secure, zero work. It is the status quo and the status quo
has not hurt anybody yet.

Against. It fails the property outright, and it fails the milestone this work belongs to. 128's
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

For.

- Attribution is rung two rather than rung three. `smelter[bot]` carries a badge and a suffix
  GitHub renders; nothing has to be remembered or agreed.
- The permission set is already exactly two and already scoped to `nife` alone, narrower than
  the repository write (a) needs and far narrower than `calef`'s `repo` scope, which reaches every
  repository he can see.
- An organization owner can see it, rotate it, suspend it or uninstall it. Private keys are
  listed in App settings and deletable; a suspended installation *"cannot access resources owned by
  that installation account"*, effective immediately. Compare (a)'s classic PAT, which an owner
  cannot see at all.
- It is the same identity `toolchain-bump.yml` already uses once pull request #1167 lands, so
  the tree gains one automation identity rather than two.

Against.

- **The key at rest is the durable secret and it does not expire.** GitHub: *"Private keys do not
  expire and instead need to be manually revoked."* A leaked token is an hour of exposure and
  dies by itself; a leaked key mints fresh tokens indefinitely until somebody rotates it.
- So the App's usual advantage is weaker on a laptop than it is in Actions, and this is worth
  stating plainly because it is the part that is easy to get backwards. In a workflow, nothing
  durable is stored on the runner and the App's "nothing stored expires" argument is clean. On
  patagonia the durable thing is the key itself, and a key that does not expire is not obviously
  better than a token that can be revoked individually.
- **The blast radius is not zero.** Contents write on `nife` means force-pushing `main`, and pull
  requests write means opening, commenting and arming anything. Bounded, and narrower than what the
  same laptop already holds.

The comparison that actually matters, and it is closer than it looks. The credential already on
patagonia is calef's own token with `repo` and `workflow` across every repository he can reach. Both
(a) and (d) are narrower than what is already there, so neither is an increase in exposure on
this machine. The question is what is added, what an owner can see, and what can be revoked without
touching the architect's own access. On those three, (d) wins on visibility and revocation and loses
on the key's immortality.
