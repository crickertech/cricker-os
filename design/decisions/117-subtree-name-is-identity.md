---
status: DECIDED
raised: 2026-08-23
decided: 2026-08-23
ratified_by: calef
---

# 117. A principal's subtree is named by its identity string, created at provisioning time

calef, 2026-08-23, on milestone 49 (what identity is for once it stops being authority)'s subtree-scoping fork: *"Agree, go with
provision-time creation."*

## The question

`login` attenuates every successful login to the same fixed subtree (`SUBTREE_NAME`), because
wiring an authenticated identity to a *specific* subtree needs a lookup this milestone had not
built, and its own text said it "does not want to guess the shape of." Two things needed deciding:
what the lookup is, and when the subtree comes to exist.

## The decision

The identity string is the subtree name, used directly, with no separate lookup table. The
subtree is created at provisioning time (when an identity and secret are first `PUT` into the
credential store), not auto-vivified the first time someone logs in.

## Why, checked rather than assumed

Identity is already the right shape for this. `cred_proto`/`login_proto` already represent
identity as a plain byte string, up to `MAX_IDENTITY = 64` bytes (`"chris"`, `"corinne"` in the
existing tests), not an opaque numeric id needing a separate mapping. `fs_subtree_caretaker`
already takes an arbitrary name at construction; today it is hardcoded to a constant, and using
the identity instead is passing a different byte string through the same parameter, not new
mechanism.

It is safe to use directly, checked against the fs model rather than assumed. `fs_proto`'s
`valid_name` and the directory-entry model treat a name as one opaque key (up to `MAX_NAME = 255`
bytes) with no separator parsing at all -- a `/` or `..` inside an identity string is just a byte
in one entry name here, not a path-traversal vector, because this filesystem does not interpret
names for structure the way a string-parsed path would. `MAX_IDENTITY`'s 64 bytes fits comfortably
under the 255-byte cap.

**Prior art:** this is Unix's `/home/$USER` convention, minus the indirection through a numeric
uid, which is consistent with how this milestone already avoids uid/gid everywhere else.

**Provision-time over auto-vivify, and why that is not a detail.** Auto-vivify (create the subtree
the first time an identity successfully logs in) is simpler to build but means any identity that
clears the credential store's `PUT` silently acquires a filesystem subtree with no separate,
deliberate act. Provision-time creation keeps identity-creation and resource-allocation as one act,
the same way `useradd` creates `/home/$USER` at account-creation rather than at first login. Given
this system's own stated position that authority should never be ambient or silently acquired
(DECISIONS §10, §82), auto-vivify would be an odd exception to grant to the one place a principal's
resources actually come into being.

## What this decision surfaced, not itself

Provision-time creation requires a provisioning tool that does two things (credential `PUT` and
subtree creation), and none exists. Checked: nothing outside test harness code
(`credentialer_test_client.rs`, `kernel/src/user/credential_tests.rs`) calls the credential
service's `PROVISION` endpoint today. This decision assumes that tool into existence rather than
leaving the assumption implicit; milestone 155, a provisioning tool that creates an identity and
its home subtree together (minted alongside this decision, provisional), is where it is tracked.

## What this does not decide

The provisioning tool's own design (a program, its capability contract, whether it is interactive
or scripted) is milestone 155's, not this entry's. Reclamation of a subtree when a principal is
deprovisioned is not addressed here either; it is downstream of the reclamation bound
`components/src/login.rs`'s own BUGS already names as open.

## What it unblocks

`login` can attenuate each successful login to that identity's own subtree instead of the shared
fixture subtree, which is milestone 49's largest remaining scoping gap. Building it needs milestone
155's provisioning tool to exist first, so the subtree actually has something to be created by.
