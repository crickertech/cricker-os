---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# Concurrent login sessions: more than one person on the machine at once

calef asked for this on 2026-10-06 (UTC), in a maintainer session, and later the same day added why:
*"it seems like we have programs that not every user on a multiuser system should be able to run.
which means we're possibly looking at some sort of sudo escalation mechanism."* A writing-only lane
wrote it and built nothing. Six forks below are his to rule on.

Every name here is provisional: `greeter`, `console_multiplexer`, `grant_broker`, the policy files
and the gate. They are an architect's call.

This is the revisit of milestone 481 (real terminal multiplexing), which milestone 49 (users, login, and attribution) refused with
one condition: "somebody needing two sessions at once." calef's request meets it. If promoted, 481
should point here rather than be reopened as a second record.

## Why, and its honest rank

Milestone 49 built the parts: identity, a per-identity subtree (§117 (a principal's subtree is named
by its identity)), a budget, a per-caller channel. It stopped at one live session on purpose.

Against principle 1: the customer path is vacant, and none of the candidates in milestone 530 (name
a customer) needs two operators. Remote login ranks higher than concurrency does. A rented machine
(§203 (capacity is rented rather than bought)) has no console a person can reach, so even its one
operator needs a network login. calef also ruled no second customer before milestones 801 and 802.

Against the fatal risks: no verdict moves. Concurrency adds claims to risk 7 (the confinement claim
is false), because user against user is a confinement boundary the 33-row table in
`notes/confinement-claims.md` does not yet contain. Each new row needs a replayable falsification
before it counts, so this work enlarges risk 7's surface rather than greening it. Risk 8 (nobody
needs it) is untouched.

Its claim on a lane is calef's ask, and the escalation question, which bears on every privileged
program the tree adds (`reboot`, `vouch`).

## What the tree has, read on 2026-10-06 (UTC)

No person can log in on a real boot today. `crates/system_initializer` builds `login`, then deletes
its own copies of `login`'s front door. `components/src/swish.rs` says plainly that the shell "holds
no capability to `login`'s front door." The only clients are `kernel::user::login_tests`. Milestone
49's "a successful login receives the terminal" is true of the plumbing, proven in tests, and
reachable by nobody at a prompt.

A login session cannot run a program. It receives a directory, a budget, a logout ticket, the
terminal and, if listed, the run-unvouched capability. It gets no shell and no spawn endpoint, and
`login.rs` says so. `grant_plan::spawnproto`'s BUGS adds why: every activation verb (`install`,
`remove`, `rollback`, `vouch`) is open to whoever holds the spawn endpoint. "A session given a spawn
endpoint would be the owner too."

`login` blames its stuck-terminal bug on having "no wait-any primitive." Notification objects
(§101 (notification objects: async multiplexing without wait-any)) have since landed as
`abi::objtype::NOTIFICATION`, so that reason is stale.

**Reuse:** `login`, the credential service, `fs_subtree_caretaker`, the system log and the spawn
presentation are the tree's. The greeter and multiplexer are new glue over them;
`getty` and `screen` lend a shape, not code. Remote login reuses an SSH library (fork 6).

## The forks

Six, in the order calef should rule on them. The first three shape the first slice.

### 1. What a session is, and who mints its terminal

Today `login` holds the one terminal and lends it. That is why concurrency is a design question
rather than a loop: the terminal is a singleton inside the authenticator.

| option | what it is | verdict |
|---|---|---|
| A | A `greeter` per terminal. It owns one terminal, reads an identity and secret, asks `login`, builds a shell from the capabilities `login` returns, supervises it, and logs out when it dies. `login` holds no terminal. | recommended |
| B | `login` mints one terminal object per session from terminals it is handed, `terminal_held` becomes a set | refused |
| C | A `session_manager` process between every source and `login` | refused for now |
| D | One shared terminal endpoint, sessions told apart by badge | refused |

A is Unix's `getty` and `login` split, from memory rather than re-read: one process per line, and
the authenticator never owns the line. It removes the singleton instead of widening it. Liveness falls out, because the greeter is the shell's supervisor
and receives its death on the fault endpoint (§26 (the fault endpoint)). That fixes `login`'s stuck-terminal bug with no
new mechanism. It also answers what builds the shell, which nothing does today.

B leaves the authenticator holding every terminal, a wider grant than authentication needs.

C is A with one more process. It earns its place when something needs a view of all sessions at once,
which `w` (milestone 681 (`w`: who is logged in)) might. Today nothing does, and A does not foreclose it.

D is the shared-endpoint pattern §109 (attribution is a property of a channel) refused three times.

Cost of A: one small program, and a change to `login_protocol` that drops the terminal from the
reply. That wire change is the expensive part, since two programs agree on it. Nothing outside the
tree has acted on it. Would A still win at equal cost? Yes. It has fewer things holding a terminal.

If calef says no: B, with its wider grant written into `login`'s BUGS.

### 2. Which programs a user may run, and how a user gets more

calef's question. The capability model reframes it. Running a binary confers nothing here. What a
program can do is what it is granted at spawn (§208 (installing is granting)). `reboot` (#1766,
option A, a reset object) is the live example: a session that holds the object can reboot, and one
that does not, cannot, whatever program it runs.

So "who may run `reboot`" becomes two questions. Which capabilities does a session get at login?
And how does a session obtain one it was not given?

The tree already answers the first once. `may-run-unvouched` is an owner-written list at the
file-service root that `login` reads at every login (§221 (the boot prompt is the owner's console) ruling 2). A session's confinement to its
own subtree (§117) is what keeps the list out of its reach. And the spawn protocol has a
"presentation" for this: a request that claims a privileged endowment must prove it holds the
capability by sending on it (`RUN_UNVOUCHED_BIT`).

For the second question:

| option | what it is | verdict |
|---|---|---|
| A | setuid or `sudo`: a binary carries authority to whoever runs it | refused |
| B | Login-time only. To do more, log in again as an identity the policy grants more. | recommended for the first slice |
| C | A `grant_broker` holds privileged capabilities and hands a session an attenuated, time-limited or single-use copy after re-authentication | refused for now |
| D | The broker spawns one job with the extra endowment after re-authentication and a policy check. The session never holds it. | recommended as the follow-on |

A is ambient and binary-scoped. Whoever can name the binary gets its authority, which is the confused
deputy milestone 49 names in setuid. §85 (what we port is evidence) already says `sudo` exists because
there is an ambient root to escalate to, and may have no successor here. §84 (how we port) puts it in
its third tier: take the interface, write the thing.

B generalizes what the tree does now: each privileged authority gets an owner-written list, and
`login` delivers it only to listed identities. With concurrent sessions, "log in again as `admin` on another terminal" is `su` without
the ambient part. Its cost is friction, and its benefit is that nothing new holds authority.

C hands a session a live capability and promises to take it back. The kernel has no time-limited
capability. Expiry would need the broker to revoke on a timer (§16 (object revocation))
or to proxy every call, and a holder can copy a `GRANT`-able capability before it expires. §139 (who
may read the cycle counter) ruled the stronger statement for its case: authority given at creation,
never acquired by a live thread.

D keeps §139's statement. The session asks; the broker re-authenticates (through `login`'s
credential path), checks the owner's policy, and spawns that one command with the extra endowment.
Single use falls out, because the authority dies with the job. Revocation is tearing down the job
(§16), and §108 (disabling a user's login credentials kills their durable session) extends to it unchanged. Every grant
is one audit record naming identity, command and authority. This is polkit's shape (a privileged mechanism asks a policy before each action) without its root
daemon. From memory, not re-read: Fuchsia and Genode route capabilities statically by component
manifest and have no runtime escalation at all, and seL4 leaves policy entirely to the system built on
it. D is closer to them than to `sudo`, because nothing a session holds widens.

Precedent: §123 (boot-time re-derivation privilege) built an authority that
dies after one use, and the spawn presentation exists. §65 (a refusal that is not passive cannot be used as a question) bears only on the
broker's deny path.

Where the policy lives, for both B and D: owner-written files at the file-service root, one per
authority, edited at the owner's prompt. That matches the two lists that exist. A single table of
identity to authorities would read better, and it is reversible later.

Would B then D still win at equal cost? Yes; C's expiry is the part that fails silently. If calef
says no to D, B alone is a complete system.

### 3. Where sessions come from first

| option | what it is | works under QEMU on all three today? | verdict |
|---|---|---|---|
| A | Several virtual terminals on the boot console: a `console_multiplexer` switches the one line between terminals on an escape key | yes | recommended |
| B | A second serial line | no: `virt` gives aarch64 and riscv64 one UART each | refused |
| C | The compositor's windows | no: no x86_64 graphical leg (milestone 270 (the x86_64 test runner's gpu)), and calef ruled graphics stay unused at boot (milestone 632 (graphics on demand)) | refused for now |
| D | A network login | not safely: no TLS server, and milestone 649 (every client of a network stack shares its socket numbers) so sessions could reach each other's sockets | follow-on (fork 6) |

A puts two sessions on one machine, not two people: whoever holds the serial line holds every
terminal on it. Two people arrive with fork 6. A wins because the session object is the same
whatever the source, and A is the only source a gate can drive on three architectures. That is a
parity argument, not an effort one.

The risk in A is x86_64, whose console is kernel-resident (§121 (x86 port I/O)). The multiplexer has to sit above
whatever endpoint the shell reads there. Nobody has checked that it can.

If calef says no to A: the first slice waits for 649 and a network source.

### 4. Isolation between users, and the owner's console

A session already has its own subtree, budget and channel. It shares the file server, the network
stack, the log, the scheduler and the memory pool. Recommended for the first slice:

- Memory. A fixed per-session budget from `login`, as now. `login`'s construction budget is sized for
  "a handful" of logins, and milestone 632 shows the pool already refusing a second graphical session.
  The building lane measures how many sessions fit, and records it.
- CPU. Not isolated. The gate measures how long one session answers while another spins, and records
  it without a threshold.
- The owner's console stays §221's: terminal 1 on the boot console is the owner's, with no login.
  Anyone who holds the serial line can switch to it, which §221 already accepted. A session from any
  other source, the network above all, must never reach it. Owner authority from elsewhere is fork 2's
  broker, not a remote terminal 1.

Requiring the owner to log in on terminal 1 would reverse §221. Not recommended while possession of
the console is the machine's only root of trust.

### 5. Attribution and audit with two people at once

Today `login` sends one `ATTRIBUTED` record per login and `login_audit_receiver` throws it away.
Yet the system log (milestone 613 (a system log service), §242 (a system log)) already stamps each record's `user` from the
writer's badge and filters reads by it. It is a multi-principal server that attributes by channel.

| option | verdict |
|---|---|
| `login`'s records and the greeter's logouts go to the log; each session's log writer is badged with its identity | recommended, first slice |
| Build §109's second half (milestone 480 (a server that logs which channel a request arrived on)): a multi-principal server logs which channel a request came on | when fork 2's broker or a session spawn endpoint exists |
| Stamp identity onto every capability | refused by §109 |

A spawn service that sessions reach also meets 480's revisit condition. The log is memory only
until milestone 687 (the system log persists through RedoxFS), so the trail dies at reboot: a `BUGS` line, not a blocker.

### 6. Remote login

| option | what it is | verdict |
|---|---|---|
| A | An SSH server, reused: `sunset`, a `no_std` Rust SSH library by dropbear's author (from memory, not built here) | recommended, follow-on |
| B | An SSH server written here | refused: rule 6 says reuse first, and this is security protocol code |
| C | A nife protocol over TLS (`rustls` server, §196 (nife carries TLS); milestone 501 (a TLS client) is a client only) | refused |
| D | Plaintext over TCP | gate fixture only, loopback through QEMU's port forward |

A's case is the stranger, whose every machine has an SSH client. C needs a nife client installed
first, plus a TLS server and a certificate story the tree lacks.

A is a dependency ruling (§46 (thin primitives or whole subsystems), rule 6). The lane checks that `sunset` builds on all three bare-metal
targets and which crypto it takes, against §198 (the glue is ours, the primitives are not). `russh` (Tokio) and
dropbear (C, through §31 (the foreign-language seam)'s seam) are the fallbacks. It is a follow-on, depending on this slice, 649
and milestone 783 (the network stack seeds its random generator from the clock), since a
remote login must not ride guessable TCP sequence numbers.

## The first slice

One milestone, shippable alone: two identities' sessions, concurrent and isolated, on the boot console.

1. `greeter`, one per terminal, as fork 1 A.
2. `console_multiplexer` on the boot console with three terminals. Terminal 1 is the owner's prompt.
3. `login` loses the terminal and `terminal_held`, and `LOGOUT` moves to the session's private
   channel.
4. A session gets a spawn endpoint that serves plain programs and refuses every activation verb and
   every owner-only endowment, unless the session presents the matching capability (fork 2 B).
5. `login`'s audit records reach the system log.

### Exit criteria a stranger could check

Under QEMU on aarch64 (`virt`), riscv64 (`virt`) and x86_64 (`q35`), one `cargo xtask` gate that
`script/test` runs, exiting 0 on all three:

1. Two at once. The gate logs `chris` in on terminal 2 and `corinne` on terminal 3. Both run a program
   at the same time, and each transcript shows its own output.
2. Files are isolated. Each writes a marker in its own subtree. Each then tries to open the other's
   marker by path and is refused.
3. Terminals are isolated. No byte `corinne`'s session writes appears anywhere in terminal 2's
   transcript.
4. Memory is isolated. `corinne` runs a program that allocates until refused. `chris`'s next program
   still runs.
5. The owner's authority stays the owner's. `vouch` from either session is refused with a named code.
   The same verb at terminal 1 succeeds.
6. A listed identity gets the authority. After the owner adds `chris` to `may-run-unvouched`,
   `chris`'s next login holds it and `corinne`'s does not.
7. Liveness. Killing `chris`'s shell returns terminal 2 to the greeter's prompt with no restart of
   `login`.
8. Audit. The system log's reader, at terminal 1, shows both logins and the logout, each stamped
   with its identity.
9. Measured, not gated: how long `chris`'s session takes to answer while `corinne` spins.
10. `login_hands_out_the_terminal_once_and_denies_a_concurrent_second_login_until_logout` is
    retired, and its replacement proves two concurrent logins get distinct terminals.
11. New rows in `notes/confinement-claims.md` for user against user, each with a replayable
    falsification.
12. A `design/decisions/` section records the session object and the spawn endpoint's restriction,
    minted by the integrator.

## Follow-ons, proposed, unnumbered

- The broker of fork 2 D. Depends on this slice and on calef's ruling on D.
- Remote login over SSH, fork 6 A. Depends on this slice, 649, 783 and a dependency ruling.
- Graphical sessions in compositor windows, after milestone 270 and when calef wants graphics.
- More than one durable session at a time (`login.rs` names the limits).

## What is blocked until the ruling

Forks 1 to 3 block the first slice. The slice builds on 4 and 5's recommendations unless calef says
otherwise. Fork 6 blocks only its follow-on.

## BUGS

- Virtual terminals on one serial line are one person's sessions. Two people arrive with fork 6.
- `#1766`'s reset object is cited as ruled on the maintainer's report. No ruling was on the pull
  request when this was written.
