---
status: DECIDED
raised: 2026-10-06
decided: 2026-10-06
ratified_by: calef
---

# 253. Concurrent login sessions: a greeter per terminal, authority granted at login, memory from a broker

calef ruled the six forks of the concurrent-login-sessions proposal on PR #1769 on 2026-10-06
(UTC), one at a time, each recorded verbatim in a maintainer comment there. *(Section number
provisional until the merge queue lands it.)* Recorded by the maintainer session the same day.
Milestone 806 (concurrent login sessions) is the proposal promoted, and it builds forks 1 to 5;
fork 6 and the follow-on half of fork 2 are unnumbered follow-ons. Nothing is built yet.

Every name here is provisional: `greeter`, `console_multiplexer`, `grant_broker`, `memory_broker`
and the policy files. Naming them is a separate ratification under design/naming.md.

This section builds on §221 (the boot prompt is the owner's console), amended the same day on
#1776. Every console shows a greeter, and the only unauthenticated owner shell is a recovery boot.
The owner sets a password before any shell exists.

## The rulings

1. What a session is. calef: *"A on Fork 1."* One `greeter` per terminal owns it, authenticates
   through `login`, builds and supervises the shell, and logs out when the shell dies. `login`
   holds no terminal. Milestone 49 (users, login, and attribution)'s single-session rule is retired.
2. What a user may run. calef: *"B now, D as a follow-on."* First, authority is granted at login
   from owner-written lists at the file-service root, the `may-run-unvouched` pattern; to do more, a
   person logs in as an identity granted more. Later, a `grant_broker` re-authenticates, checks the
   owner's policy and spawns one job with the extra endowment, audited. A session never holds it.
3. Where sessions come from. Virtual terminals first. calef: *"be able to switch between
   sessions at a single keyboard. I believe early linux with text mode had this when hitting the
   function keys to switch between different sessions. You would get a fresh login prompt, could
   sign in, and that session would continue running and you could switch between sessions."* So
   Alt+F*n* selects console *n*, each console opens on a fresh greeter, and a session switched away
   from keeps running.
4. Memory. calef: *"D on Fork 4."* A `memory_broker` holds the pool left after the boot's carve.
   A session gets a small first increment at login and asks for more, in geometric increments, when
   a split is refused. The owner holds a revoke capability over what the broker handed out. An
   optional per-identity cap is an owner-written file; no file, no cap. CPU stays unisolated, with
   the delay measured. The broker is designed so programs, not only sessions, can ask it for memory;
   a growable program heap is a separate proposal (#1777).
5. Audit. calef: *"Yes on Fork 5, with the owner-credential events."* `login`'s records and the
   greeter's logouts go to the system log of milestone 613 (a system log service), which stamps
   `user` from the writer's badge. The owner password being set, a recovery boot and the refusal of
   an unreadable credential store are audited too. Milestone 480 (a server that logs which channel
   a request arrived on) returns when sessions reach the spawn service.
6. Remote login. calef: *"Sunset."* A follow-on reuses `sunset`, a `no_std` Rust SSH library, as
   the remote-login server. SSH logins take the same `greeter` and `login` path, so rulings 2 and 5
   apply, and recovery stays local-only.

## The conditions on ruling 6

- The dependency (§46 (thin primitives or whole subsystems)) is approved on the condition that
  `sunset` builds on all three bare-metal targets. If it does not, this approval does not stand.
- Before any nife machine exposes SSH to a network, a security gate passes: fuzz `sunset`'s packet
  parser and review the glue.
- dropbear through the C seam (§31 (the foreign-language seam)) is the fallback.
- calef, the same day: *"We should seriously consider contributing any hardening we do to sunset as
  a push upstream if possible."* Hardening goes upstream where it can. A security bug goes to
  `sunset`'s maintainer privately first, never as a public pull request first. Each upstream
  submission is an outward-facing act calef approves.

## What was refused

| Fork | Option | Why it lost |
|---|---|---|
| 1 | `login` holds a set of terminals | A wider grant than authenticating needs. |
| 1 | One shared terminal told apart by badge | §109 (attribution is a property of a channel) refused it three times. |
| 1 | A `session_manager` in front of every source | For now: nothing yet needs every session at once. |
| 2 | A. setuid or `sudo` | Ambient and binary-scoped, the confused deputy milestone 49 names. |
| 2 | C. A broker hands out a time-limited or single-use copy | The kernel has no expiring capability, and a holder can copy a `GRANT`-able one first. |
| 3 | A second serial line | `virt` gives aarch64 and riscv64 one UART each. |
| 3 | Compositor windows first | For now: x86_64 has no graphical leg, and graphics stay off at boot. |
| 4 | A. A fixed budget per session | calef: "we're just limiting a single session to how much memory can be used." |
| 4 | B. One region of everything but a reserve | A region cannot shrink, so a second session starves. |
| 4 | C. `login` as the memory broker | The authenticator holding all memory is ruling 1's wider-grant argument again. |
| 5 | Identity stamped onto every capability | §109 refused it. |
| 6 | B. An SSH server written here | Rule 6 says reuse first, and this is security protocol code. |
| 6 | C. A nife protocol over TLS | A stranger's machine has an SSH client and no nife client. |
| 6 | OpenSSH | Its fork and uid privilege separation does not survive a port to a capability system. |

## Open

- The session object, the spawn endpoint's restriction and the broker's request and reply are wire
  formats milestone 806 has to record in their own section when it builds them.
- The follow-ons (the `grant_broker`, remote login over `sunset`, memory for running programs)
  carry no milestone numbers until someone takes one.
