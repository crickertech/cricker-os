---
status: PROPOSED
raised: 2026-09-27
---

# 243. Notices for people: programs publish, users curate, displays show

*Section number provisional: §241 is claimed by #1421 and §242 by #1423, so this took 243 on
2026-09-27 (UTC) and may move at merge. The slug, the word "notice", the service name
`notice_board`, every wire value and every constant below are provisional.*

Raised 2026-09-27 (UTC) by calef in the maintainer session: *"We want components to be able to
publish notifications for systems users. We want users to be able to curate the types of
notifications they receive. We want different displays to be able to display the notifications."*
His examples: a notice in every session that the system is going down for reboot, an update poller
telling an appropriate user that packages are out of date, and a background job that ended.
Written by the lane `proposal/user-notifications` (pull request #1424), which builds nothing.

## The name comes first, because the obvious one is taken

"Notification" is a kernel object: §101 (notification objects), milestone 151 (notification objects), `objtype::NOTIFICATION
= 4` in `crates/abi/src/lib.rs:512`, with `SIGNAL`, `WAIT`, `POLL` and `BIND`, and
`Received::Notification` in `crates/user_mode_runtime`. The design below uses that object as a
display's doorbell, so the two would meet in one sentence. The person-facing thing needs another
word. Names are calef's call (`design/naming.md`, and CLAUDE.md's naming authority), so these are
options with a leaning:

| option | concept, service, contract crate | collides with |
|---|---|---|
| N1 | notice, `notice_board`, `notice_protocol` | syslog's severity 5 is named "Notice" (RFC 5424), and §242 ruled syslog's eight levels. It is a value in one field, not a type. As a noun it is unused in code; the 41 hits are the verb. |
| N2 | bulletin, `bulletin_board`, `bulletin_protocol` | nothing in the tree or in RFC 5424; reads as a broadcast, which two of three examples are not |
| N3 | memo, `memo_board`, `memo_protocol` | nothing; reads as addressed to someone, which fits, and as informal |
| refused | alert | RFC 5424 severity 1, "action must be taken immediately"; a job that ended is not an alert |
| refused | message | an IPC message is `Received::Message` beside `Received::Notification` |

Leaning: N1. It parses without Unix exposure, which `design/naming.md` asks of a component name,
and its collision is a severity value rather than a second kind of object. This file uses
"notice" throughout, provisionally.

## What is being decided, in the order it should be answered

1. A separate service, or a view over the §242 system log.
2. The name (above).
3. The capability shape: what a publisher holds, how it names an audience, and who may bypass curation.
4. The wire and read formats. Irreversible, so options with a leaning.
5. The urgency scale. Irreversible once a publisher ships.
6. How a console shows an urgent notice. Reversible, recommended.

Curation storage, retention, table sizes and defaults are reversible and recommended below; none
blocks.

## What the tree has today

Nothing person-facing. Lookups on base `995ac5bce`:

- No `motd`, banner, `wall` or broadcast concept exists.
- No program reboots the machine. The first example has no publisher yet.
- No update poller. §241's P5 (an OS update is a set of packages through a slot) is unbuilt.
- A job's end is already observed. On #1377 the session process "blocks on one endpoint that
  carries both the timetable's death and every job's report" and, until milestone 152 (durable
  delegation)'s Fork 6 is built, "receives and drops reports".
- One interactive session at a time: `login` holds a single `terminal_held` flag
  (`components/src/login.rs:159`) and refuses a second login with `NO_TERMINAL`.
- Nobody may list sessions. "List sessions and match" was refused as "enumeration is authority"
  (`notes/durable-delegation.md:48`, milestone 126 (who else is running)).
- Per-user state lives at `<principal root>/<identity>/`, beside `schedule`
  (`crates/schedule_store/src/lib.rs:79`). There are no home directories (`notes/credentials.md:372`).
- The compositor draws a fixed scene of three windows with `MAX_WINDOWS = 4`
  (`crates/compositor/src/lib.rs:160`) and has no overlay or popup (`notes/compositor.md:327`).

## Prior art, read

Read on 2026-09-27 from primary sources by this lane; "(memory)" marks what was not verified.

| system | who names the sender | who controls loudness | replace or dedupe | no display attached |
|---|---|---|---|---|
| freedesktop Notifications (D-Bus `Notify`) | the caller's own `app_name` string | the sender's `urgency` hint: low, normal, critical | `replaces_id` from an earlier `Notify` | not handled; a `persistence` capability is optional |
| Apple UserNotifications | the app bundle | user per app; `interruptionLevel` passive, active, timeSensitive, critical; critical needs an entitlement | a request with the same identifier replaces the pending one | not handled |
| Android channels | the package | the user, per channel: "After you create a notification channel, you can't change the notification behaviors" | same `notify()` id updates; `setOnlyAlertOnce`; `setTimeoutAfter` is a TTL | not handled |
| `wall`, `write`, `mesg` | the kernel's uid | the recipient, with `mesg n`; "only the superuser" writes past it | none | lost |
| systemd `shutdown` and logind | the privilege to power off | none; `--no-wall` is the sender's | warnings repeat as the deadline nears (`logind-wall.c`'s interval table) | lost |
| Ubuntu `update-notifier` and `update-motd` | root scripts at login | none | regenerates only when the package lists or dpkg status change | the next login's message of the day |
| Fuchsia | | | | no user notification library found in the FIDL index |

What carries over: Android's user-owned channel is curation by (program, class); Apple's
entitlement makes bypass a granted right; `update-notifier` republishes only when its inputs change.
Every desktop system assumes an attached display, so the headless case has only `motd`.

## Question 1: a separate service or a view over the log

Priced both ways.

A view over §242 would make a notice a log record with a new body kind (F3 has the byte for it).
It would need four changes to §242, each measured against what was ruled today:

- Audience. The log's badge maps to (program, user, process). A notice also needs an audience,
  so registration grows a field.
- Per-user read. §242's per-user scope filters by who wrote. A notice is read by whom it is
  addressed to, so the filter grows a second axis.
- State. A notice is replaced, dismissed and expires. An append-only ring expresses those only as
  tombstone records, which every display must fold back into state.
- Retention. The service's 64 KiB ring holds about 24 minutes of a chatty reporter (§242's own
  figure). A job that ends at 03:00 is evicted by kernel chatter before its owner wakes.

The fourth is decisive: a notice must outlive the traffic around it, and a log is sized by traffic.

Recommended: a separate service, `notice_board` (provisional), that holds the live notices as
state and appends every accepted notice to the log as history. The log is the backstop the question
asks for, and it costs the log nothing: the board is one more writer with its own badge. Its records
carry an `audience` field, which calef's JSONL ruling already allows ("structured fields can join
the object later").

Refused: folding the board into the log service as a second front door. It saves a process and
couples a fan-out service to the one calef just ruled must fall back to the UART when it wedges.

Refused: writing into each session's terminal, as `wall` does. That needs every terminal, which
is enumeration, and has no curation.

## Question 3: the capability shape

Recommended: a publisher holds a badged endpoint to the board, and the badge names its audience.
The writer never states its audience, program or time.

- Minting mirrors §242. Whoever spawns a publisher holds the board's unbadged endpoint, mints a
  badged copy (`rendezvous::BADGE`, §230 (badged endpoint capabilities)) and registers it: badge to (program digest, audience,
  bypass). The board stamps the rest.
- Three audiences. One identity; the owners, meaning §221 (the boot prompt is the owner's console)'s owner-written list plus the
  console owner; and every attached display.
- `login` registers per-user badges. It mints one publish badge (audience: that identity) when it
  builds a session process, and one read badge per login.
- The spawner of a system service registers the others. The reboot program gets "every display,
  may bypass". The update poller gets "owners".
- A user cannot address another user. `write(1)` has no analogue until someone asks for one.
- The board keeps a list of who is subscribed, not who is logged in. A display subscribes
  with its read badge, so "every display" is the board's own subscriber set. No publisher learns
  who else is on, which keeps "enumeration is authority" intact.
- Bypass is registered, never sent. Only a badge registered with the bypass right may publish
  at critical urgency, and the board refuses critical from any other. That is Apple's entitlement
  as a capability, and the analogous case in the tree is §221's owner-written list.

A display is woken with a §101 notification object: it hands the board one with `SIGNAL` right
when it subscribes, and binds it (`BIND`) so a new notice wakes it out of `RECV`. That is the
tree's doorbell, "a doorbell, not a meeting" (`crates/abi/src/lib.rs:520`).

## Curation

Recommended: a per-user document, stored at `<principal root>/<identity>/notices` (provisional)
beside `schedule`, written by the user's own session, and handed to the board whole. That is §222 (who holds a user's schedule)'s
shape: one document in one shared page, parsed whole or refused, and the session writes the store
(sub-rulings 1, 2 and 5).

- A rule is (program, class, least urgency shown). Program is the board's stamp, class is the
  publisher's label (`job.ended`, `package.updates`). The pair is Android's channel.
- A publisher cannot change a user's rule. A rule can hide a class, lower it to the list only,
  or raise it. It cannot silence critical.
- Defaults are compiled into the board: show normal and above, keep low in the list, always show
  critical. Before a user's document arrives, or when it is absent, the defaults apply.
- Whoever can write that identity's subtree may change it. Nothing new.

## Question 5: the urgency scale

Leaning: four levels, low (listed, never presented), normal (presented), high (presented and kept
until dismissed) and critical (presented, cannot be curated away, bypass right required). They map
onto Apple's four and onto freedesktop's three with high folded into critical. Android's five include
`NONE`, which is a curation outcome rather than something a sender asks for.

Freedesktop's three would lose the difference between "keep this until I see it" and "this
overrides my settings", which the update and reboot examples need.

Urgency is not §242's severity. A job that failed is severity error and urgency normal; a reboot is
severity notice and urgency critical. The board writes both into the log record.

## Delivery

Recommended:

- Held until dismissed or expired, in memory. Each notice carries a time to live the publisher
  sets, capped by the board (7 days, provisional). Nothing persists across a reboot; the log's
  RedoxFS half, when built, holds the history.
- Replacement by (program, key). A notice with the same publisher-chosen key replaces the old
  one, as `replaces_id` and Android's id do. A key is scoped by the stamped program, so no program
  can replace another's.
- No nagging. A replacement whose text is byte-identical does not alert again (Android's
  `setOnlyAlertOnce`). A dismissed key stays dismissed until its text changes; the board keeps an
  8-byte digest per dismissed key.
- Retraction. A publisher may withdraw its own key, which is how a cancelled reboot disappears.
- Dismissal is per user. Dismissing on one display dismisses on all of that user's displays.
- At least once per display. A display reads from the last sequence number it saw, so a display
  that attaches late gets the backlog.
- No attached display. The notice waits in the user's table until it expires. A table holds 32
  live notices per identity (provisional); overflow drops the oldest lowest-urgency one and counts
  it.
- Refused for now: actions (freedesktop's `ActionInvoked`). An action is a reverse capability to
  the publisher, a design of its own.

## Question 4: the formats, which are irreversible

Two surfaces become wire formats.

| surface | who agrees on it | options |
|---|---|---|
| publish, publisher to board | every program that publishes | W1: a 16-byte binary header (urgency, flags, class length, key length, time to live in seconds, spare), then class, key and text as UTF-8, 224 bytes in total, in a shared page like §222's `REPLACE`. W2: one JSON object per notice, parsed by the board. |
| read, board to display | every display, including a remote one | R1: JSONL, one object per notice, with the board's stamped `seq`, `time`, `program` and `audience`, then `class`, `key`, `urgency`, `expires`, `text`. R2: the binary record as stored. |

Leaning: W1 and R1. It is §242's split, which calef ruled today for the log: binary where programs
write, JSON where people and tools read, with an in-tree JSON writer and no dependency, under §46 (thin primitives or whole subsystems). W2
would put a JSON parser in the board, where every publisher's bytes reach it, and would make
every publisher link a JSON writer. R2 would make the remote client a second binary parser for
nothing. Keeping W1's total at 224 bytes means a notice's text always fits F3's 224-byte text bound
when the board logs it.

## Displays

Recommended, and reversible:

- A console line in a swish session. Before drawing each prompt, swish reads new notices and
  prints one line each. That is how Bash reports a finished job before the prompt (memory). It
  costs one read badge in swish's table.
- Critical on the console. It cannot wait for a prompt when a full-screen program holds the
  terminal. Recommended: the terminal server prints a critical notice at the next line boundary,
  accepting what `wall` accepts, a corrupted screen, for critical only. The alternative, prompt
  only, is question 6: a user inside an editor would miss the reboot.
- A compositor toast. A small client with the user's read badge and a fourth window drawn last
  in stacking order, which is the one spare window of four. It needs only that scene entry.
- A remote client later reads the same JSONL with a read badge carried off the machine.

## The three examples, end to end

A reboot. A reboot program (unbuilt; name unminted) holds a badge registered "every display, may
bypass". It publishes class `system.shutdown`, key `reboot`, urgency critical, and a time to live
ending at the deadline: "Rebooting at 14:05 UTC". The board signals every subscribed display's
doorbell. The console's terminal prints it at the next line boundary, and the toast shows it. The
program republishes under the same key as the deadline nears, which is logind's repeating wall
without its interval table in the protocol. A cancel withdraws the key.

An update poller. It holds a badge registered "owners". Each hour it computes the set of
out-of-date packages and the set's digest, and publishes class `package.updates`, key `updates`,
urgency normal only when the digest changed, which is `update-notifier`'s rule. An owner who
dismisses it is not told again until the set changes, because the board's dismissed digest
matches. A non-owner never receives it.

A job that ended. The timetable reports the job's exit to the session process, which already
receives that report (#1377). The session process holds the publish badge `login` registered for
its identity, and publishes class `job.ended`, key the entry's name, urgency low when the job
succeeded and normal when it failed. If the user is away, it waits in their table. At their next
login swish prints it before the first prompt. A user who wants only failures sets `job.ended`'s
least urgency to normal.

## What it costs

Derived from the tree's counts; nothing is built, so nothing is measured.

- Slots. `login` gains one (the board's unbadged endpoint). Its own comment counts eight at rest
  and six more at a login's peak (`components/src/login.rs:367`), against a table now of 24. A session process, swish and each display gain one badge. The
  board holds one doorbell per subscribed display. The spawner of system services gains one; if
  that is the progenitor, it is at 23 of 24 (`kernel/src/cap.rs:309`) and §242 takes the last
  slot, so this waits on #1360's 32.
- Pages. A stored notice is at most 272 bytes: 48 the board stamps (sequence, time, program,
  audience, expiry and digest, eight each) and the 224-byte publish record as sent. A 32-notice
  table is 8,704 bytes, three pages per identity.
- Per notice. W1 is one `CALL` carrying a page. Notices arrive at most hourly in every example, so
  the per-notice time is irrelevant; the IPC round trip is 350 ns (`notes/benchmarks.md`).

## The seven questions

1. Considered and lost: each refusal is above, with its reason.
2. The analogous cases: §242's badge-and-register, §222's whole-document replace, §221's
   owner-written list, and §101's doorbell.
3. Prior art: read, above; the Bash prompt behaviour is from memory.
4. Premise: checked under "What the tree has today".
5. Cost: derived, since nothing is built.
6. Reversibility: nobody has acted on any of it. The name, the badge's audience and bypass, W1,
   R1 and the urgency scale become irreversible when the first publisher or remote reader ships.
7. Equal cost: yes. The view over the log is the least work, and it loses on retention, not effort.

## What it unblocks, and what waits on calef

It gives milestone 152's dropped job reports a consumer, gives §241's P5 a way to tell an owner,
and gives a future reboot program its warning. Blocked until calef answers questions 1 to 3:
building `notice_board` and its protocol crate. Blocked until 4 and 5: any publisher outside the
board's own tests. Question 6 and everything under Curation and Delivery can be built on the
recommendation.
