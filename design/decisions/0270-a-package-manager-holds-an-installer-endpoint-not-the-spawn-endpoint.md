---
status: DECIDED
raised: 2026-10-07
decided: 2026-10-10
ratified_by: calef
---

# 270. A package manager holds an installer endpoint, not the spawn endpoint

*Section number provisional until the merge queue lands it; 269 was the highest on `main` when this
was written. Raised by milestone 809 (the package client becomes a program)'s plan on 2026-10-07
(UTC), as its fork 1, and written by its building lane on 2026-10-10.*

calef ruled the package client a program, `jig`, on 2026-10-06 (UTC). The progenitor stays the
installer (§208 (installing a package is granting it, and the activation set is versioned)), so the
program needs a way to ask it. On 2026-10-10 (UTC) calef answered fork 1, the block's design for
that request, with *"Yes"*. This section records the method's semantics, which §10 (process model:
capability-based, microkernel) requires written here.

## The installer endpoint

A copy of the progenitor's spawn endpoint, badged for one job and narrowed to `WRITE`, on which the
progenitor serves install, remove and rollback and nothing else.

- What it is. `spawnproto::installer_badge(label)`: the job's label with
  `spawnproto::INSTALLER_BADGE` set. The kernel writes the badge (§230 (badged endpoint
  capabilities)) and `BADGE` needs `GRANT`, which the holder lacks, so no holder can forge another's
  badge or the shell's. Placed at `grant_plan::INSTALLER_SLOT` (15).
- What it serves. `spawnproto`'s activation request, unchanged: `Install` with the package's
  frames, `Remove` with a packed name, `Rollback` with nothing. Only the endpoint it arrives on is
  new. Anything else, a spawn, a `Vouch` or a verb this side does not know, is answered
  `ActivationStatus::NotServedHere` with nothing read past its first message.
- Where the answer goes. A request on the spawn endpoint is answered on the shell's result
  endpoint, which the shell is reading while the holder runs. So each holder also gets a reply
  endpoint of its own, made from its job's region when it is built. The holder has `READ` at
  `grant_plan::INSTALLER_REPLY_SLOT` (16). The progenitor keeps `WRITE`, filed by the job's label
  and deleted at its reap.
- Who grants it. The program's manifest declares it (`grant_plan::Manifest::installer`), and
  the progenitor places it only for a spawn request that arrived on the unbadged spawn endpoint.
  Only the boot prompt holds that endpoint, and §221 (the boot prompt is the owner's console) ruled
  that whoever holds that prompt is the owner. So only the owner's console grants it. No image may
  declare it (`grant_plan::image_can_carry`), and a manifest note cannot spell it
  (`manifest_note::encode` refuses it at compile time). By default only `jig` declares it.
- It cannot be passed on. The holder's copy carries `WRITE` alone, so the kernel refuses to
  delegate it. A `login` session holds no spawn endpoint, so it cannot spawn a holder either.
- What a row records. The progenitor writes the holder's program name in the activation set's
  new manager column (`activation_set::Entry::manager`; calef, 2026-10-06: "each activation set row
  records which manager installed it"). A remove or rollback that would undo a row another manager
  installed is answered `ActivationStatus::NotYours` (`activation_set::may_edit`). The owner's own
  rows, and rows from before the column, any manager may undo.
- A holder that dies mid-request. Its reaped message, which arrives on the same endpoint, ends
  the wait for frames it will never send; the request is abandoned and nothing is written.
- No new syscall and no new kernel method. A badge, a `WRITE`-only copy and an endpoint made
  from a region are all §230 and §16 (object revocation) as built.

`spawnproto`'s module documentation and `grant_plan::Manifest::installer` carry the same contract at
the definition. `system_initializer`'s `serve_installer` and `package_manager_parts` are the code.

## What was refused

- The whole spawn endpoint. The request already travels on it, so handing the program a copy
  would have cost nothing to build. But a holder of the spawn endpoint can spawn any program the
  image carries, with any grant the shell could plan, and is the owner by §221's definition.
  A package manager would be a second owner's console.
- `Vouch` on the installer endpoint. A vouch records a digest no catalog carries (§195 (a
  reviewed recipe vouches for a package) clause 3), so a holder could vouch for any bytes at all,
  its own included. Install is bounded by what a catalog vouches for; vouch is not. It stays the
  owner's console's builtin.
- A second endpoint object rather than a badge. The progenitor has one thread and no
  wait-on-many receive, so a second endpoint would need a second thread or a poll. A badge on the
  one endpoint it already reads costs neither, and `job_undertaker`'s reaped messages
  (`spawnproto::UNDERTAKER_BADGE`, milestone 685 (a job is finished when its memory is back)) set
  the precedent.

## Prior art, read 2026-10-10 (UTC)

- Android keeps installing behind one permission ordinary apps cannot hold.
  [`INSTALL_PACKAGES`](https://developer.android.com/reference/android/Manifest.permission):
  "Allows an application to install packages. Not for use by third-party applications." Other apps
  ask the system installer, which may stop for the user
  ([`PackageInstaller`](https://developer.android.com/reference/android/content/pm/PackageInstaller),
  `STATUS_PENDING_USER_ACTION`). The installer endpoint is that permission as a capability, and the
  owner's console is the only place it is granted.
- Fuchsia makes package resolution a protocol a component can reach only through a route.
  [`fuchsia.pkg.PackageResolver`](https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/sdk/fidl/fuchsia.pkg/resolver.fidl)
  is "intended to be implemented by package resolver components, and used by repository
  administration tools", and [capability
  routing](https://fuchsia.dev/fuchsia-src/concepts/components/v2/capabilities) says "there must
  also be a valid capability route from the consuming component to a provider." That the general
  rule governs this protocol is our reading; the reference page for it returned a 404 that day.
- Genode's `depot_deploy` changes what runs only by writing configuration for an `init` it was
  given: "it generates the configuration for the dynamic init instance", whose subsystems "access
  the depot content via mere ROM sessions"
  ([release notes 18.02](https://genode.org/documentation/release-notes/18.02)). That it acts only
  through sessions its parent routes is inferred from Genode's model and its example configuration;
  no sentence read says so of `depot_deploy` itself.

Each keeps the authority to change what runs with one trusted installer, and lets a client ask it
through a narrower channel. That is this section's shape: the progenitor decides, `jig` asks.

## What proves it

`script/swish-check`, on aarch64, riscv64 and x86_64. `jig install` from a file and by name, and
`jig remove` and `jig rollback` across a reboot, with no `package` builtin in the shell. `caps jig
install ...` prints the installer row and what it cannot do. And `installed/jig rollback`, the same
bytes run by path as unvouched installed bytes, holds no installer endpoint, and is refused before
anything reaches the progenitor. `activation_set`'s `a_manager_may_not_undo_another_managers_rows`
and `spawnproto`'s `an_installer_badge_carries_its_job_and_is_no_other_senders` are the host tests.

## BUGS

- "A `login` session cannot grant it" is proved by construction, not by a gate. Sessions hold no
  spawn endpoint and so spawn nothing, which is why no test runs one against the real progenitor.
  The badge check is the mechanism the day a session gets a spawn endpoint of its own, and nothing
  exercises it until then.
- **A holder that blocks without dying holds the progenitor**, mid-request, as any caller of this
  protocol that promises a message can.
- **Four holders at once**, and four first messages kept from senders not being served
  (`system_initializer`'s `INSTALLER_HOLDERS` and `STASHED`). One more holder is refused at spawn.
