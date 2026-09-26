---
status: AMENDED
raised: 2026-08-22
decided: 2026-08-22
ratified_by: calef
---

# 108. Disabling a user's login credentials kills their durable session

calef, 2026-08-22, on milestone 152 (authority that outlives the session that requested it)'s durable-delegation design (worked out in
conversation, not yet built): *"disabling a user's login credentials should kill their durable
session. That seems like the right consequence."*

## The question

Milestone 152 gives a scheduled job's authority a durable supervisor: the user's own login session,
kept alive past a disconnect by the same rule `Untyped::DESTROY` already has (§16), a parent refuses
to be destroyed while it has live children, and becomes destroyable once it does not. That answers
what keeps a durable session alive. It does not answer what should tear one down: if a user's login
credentials are disabled, should their already-running durable session, and every job it supervises,
stop, or should already-registered work keep running on the authority it was granted at registration
time until something explicitly sweeps it?

Both are defensible. Revoking credentials could plausibly mean only "you may not authenticate
again," leaving existing delegated work alone; disabling an account is also plausibly meant to stop
everything acting on that person's behalf, immediately.

## The decision

**Disabling credentials kills the durable session.** One action, one consequence: revoking a user's
ability to log in also revokes everything currently running on authority derived from an earlier
login of theirs. This reuses §40's subtree-death rule rather than inventing a second revocation
path that would have to be kept in sync with the first: killing the durable session (a supervised
subtree) cascades to every scheduled job it was supervising, the same mechanism that already tears
down a component's whole subtree when its supervisor dies.

Why this over the alternative: a second, independent revocation path (credentials disabled, but
delegated authority left running until an admin separately notices and sweeps it) is exactly the
kind of two-places-recording-one-fact shape this project's own ladder (CLAUDE.md, "nobody remembers,
so build the mechanism that does not need them to") warns against. Coupling them means there is
nothing to forget.

## Amended 2026-09-26: `user suspend` and `user resume`

*Amendment provisional until the merge queue lands it, since it was recorded in one pull request
with four provisionally numbered sections. Recorded by the maintainer at 18:35Z and corrected at 18:37Z;
those are times of recording, not of the rulings.*

calef, 2026-09-26 (UTC): *"user suspend and user resume"*. This answers the trigger this section left
open below. The fork was raised by the lane for milestone 152 (authority that outlives the session
that requested it) as the second question in `notes/durable-delegation.md`. That note lands with
#1347 and is not on `main` yet.

- The names are ratified. Both are owner-console-only commands at the boot prompt, the console
  §221 (the boot prompt is the owner's console) gives the machine's owner.
- The suspended mark is a list file: `suspended` (name provisional) at the root of the file
  service. calef, 2026-09-26: *"store the suspended mark as a list file"*. It has the shape of
  `may-run-unvouched` from §221, one identity per line, and the same reader,
  `login_protocol::lists`.
- `user suspend <name>` appends the name and sends `login` a front-door word that fires the cascade
  above at once. `user resume <name>` removes the name.
- `login` refuses a suspended identity with a distinct code. `session_reviver` reads the file at
  boot and skips a listed identity, so a reboot does not bring its jobs back.
- The stored schedule is untouched by a suspension, so it resumes at the identity's next login.
- The credential store is unchanged and stays sealed.
- The front-door word and the refusal code are provisional wire items.
- Deleting an identity is a separate act, and it is not built.

A correction, recorded as one. The maintainer first proposed keeping the mark in the credential
store, and first recorded this amendment that way. The lane for milestone 152 checked and corrected
it. That store is memory only and reprovisioned every boot, sealed after provisioning, and
unreadable by `session_reviver`, so a mark there could neither persist nor be read where it is
needed. calef then ruled the list file.

Why "suspend". It is the identity provider's word, as Okta, Google Workspace and GitHub use it, for
an act that is instant, total and reversible. "Disable" (Windows, macOS) and "lock" (Unix, where it
covers the credential only) were weighed. Unix splits locking the password, disabling the account
and ending its sessions into three acts, and forgetting one is the failure this avoids. That prior
art is recalled from memory, not re-read.

This also settles the note's fork 2 without its wire question. `session_reviver` reads the list
file at boot, so it never has to ask the credential service whether an identity is provisioned.

## What this does not decide

How credentials get disabled (the credentialer's own revocation mechanism, whether that is
deleting a store record, marking it inactive, or something else) is milestone 56/49's territory, not
named here. This decision only fixes the *consequence*, once revocation happens by whatever
mechanism those milestones build.

## What it unblocks

Milestone 152's design no longer has an open revocation fork: killing a durable session is the one
and only way delegated authority (including every scheduled job it supervises) stops, and it is
triggered exactly by disabling the login credentials that authenticate it.
