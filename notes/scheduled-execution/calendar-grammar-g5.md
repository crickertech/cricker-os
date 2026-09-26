# A calendar grammar in words: G5, priced against G3 and G4

**Status: PROPOSED 2026-09-26 (UTC).** Written by an agent on the lane
`proposal/129-calendar-grammar` for calef's ruling on milestone 129 (scheduled execution). It extends
[calendar-and-wall-clock.md](calendar-and-wall-clock.md), which recommended G3. calef answered: *"I
don't want to limit us to just nightly and weekly."* The maintainer proposed a G5 from memory; this
note checks it against its sources, designs it and prices it.

## What is being decided, and which half is irreversible

§122 (the on-disk, per-user schedule store) keeps a user's schedule as `timetable::parse`'s own
text, so the first calendar line a user writes fixes a meaning. The fork is irreversible, and per
`AGENTS.md` it gets options.

The irreversible part is the grammar's shape: the keyword that opens a line, the clause order,
where the command starts, and what an absent clause means. The word list inside the shape is
additive, because every word G5 would add later is a parse error today.

## The premise, checked against primary sources

Every source was read on 2026-09-26 (UTC): the RFC and man pages from source text, the vendor
pages live.

| source | what it actually says |
|---|---|
| App Engine `cron.yaml` | `every day 00:00`, `every monday 09:00`, ordinals `1st`..`31st` and `first`..`thirtyfirst`, `of month`, `of sep,oct,nov`. Sub-daily: `every N hours\|mins\|minutes`, optionally `from HH:MM to HH:MM` or `synchronized` (N must divide 24). Time zone is a separate field, UTC by default. |
| RFC 5545 section 3.3.10 | An invalid date such as February 30 "MUST be ignored and MUST NOT be counted". `-1FR` is the last Friday; BYMONTHDAY `-1` is the last day. UNTIL and COUNT "MUST NOT occur in the same recur". BYSETPOS picks the nth of a set, so "the last work day" is `BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1`. WKST changes which days an INTERVAL=2 weekly rule yields. |
| RFC 5545 section 3.8.5.3 | RRULE "SHOULD NOT be specified more than once", and the set from several is "undefined". A DTSTART not synchronized with the rule gives an "undefined" set. |
| systemd.time(7) | `~` counts from the month's end: `Mon *-05~07/1` is the last Monday in May. Shorthands `daily`, `weekly`, `monthly` and five more. No interval anchored to a start date. |
| systemd.timer(5) | Several elapses in one sleep yield "a single service activation". `Persistent=` fires "immediately if it would have been triggered at least once". An active unit is "simply left running". `RandomizedOffsetSec=` and friends spread fleets. |
| Temporal schedules | Spec: calendars, cron, intervals as "epoch + n * interval + phase", inclusive start and end, jitter, zone (UTC default). Policies sit apart: overlap (Skip default, BufferOne, BufferAll, CancelOther, TerminateOther, AllowAll), catch-up window (one year default), pause on failure, remaining actions. "Last day of the month or third Monday are not currently representable." Its cron has no day-field union. |
| Quartz `CronTrigger` | Seven fields with seconds and an optional year. `L` (last), `W` (nearest weekday to a day), `#` (nth weekday), `LW`. One of day-of-month and day-of-week must be `?`. |
| EventBridge Scheduler | `at(yyyy-mm-ddThh:mm:ss)`, `rate(value minutes\|hours\|days)`, `cron(` six fields with year `)`. `?` is required in one day field; `L`, `W`, `#` as in Quartz. A spring-forward time is skipped and a fall-back time runs once. |
| launchd.plist(5) | `StartCalendarInterval` keys Minute, Hour, Day, Weekday, Month; a missing key is a wildcard. Day and Weekday together mean either one. Missed intervals during sleep are "coalesced into one event upon wake". |
| crontab(5) | With both day fields restricted, a job runs "when either field matches": `30 4 1,15 * 5` is the 1st, the 15th and every Friday. |

Five corrections to the maintainer's summary follow from the table.

1. Three of the four example lines are not App Engine syntax. App Engine has no `weekday`, no
   `last`, and no `every 2 weeks`. Its own "every other week" example, `1st,third monday of month`,
   is not every other week in a month with five Mondays.
2. `every 2 weeks on mon 08:00` is not one RRULE as written. INTERVAL=2 counts from DTSTART, which
   the line does not carry, so the fortnight it picks depends on when it was registered. Temporal
   hides this with an epoch phase. An honest line has to carry its anchor.
3. `every 15 minutes from 09:00 to 17:00` is one RRULE only if 17:00 is excluded. BYHOUR and BYMINUTE
   form a product, so 09:00 through 16:45 is one rule and 17:00 on top is a second. The step must
   also divide 60. App Engine's plain `every 5 minutes` is not an RRULE at all: it waits five minutes
   after each run ends.
4. The run-policy words are Temporal's and systemd's, not App Engine's, which builds overlap into
   the interval type. RRULE itself has only UNTIL and COUNT.
5. `every 15 minutes` collides with a stored word. `every 15m` already means a monotonic interval
   from arming, so two lines one space apart would run on different clocks.

One correction to the lane's note, fixed there too: its S2 row said systemd catches up everything
that elapsed, "a stampede". systemd.timer(5) says it fires once. S2 differs from S3 in persisting the
last trigger across power-off, not in running a backlog.

## What the tree already does

- `timetable::parse` dispatches on the first word, `every` or `at-boot`. Any other first word is
  `Error::UnknownSchedule`, so a new one is additive.
- `every <digits><ms|s|m>` is taken. No calendar line exists in the tree, so nobody has acted on a
  calendar grammar yet.
- `crates/calendar` refuses tzdata and refuses month arithmetic, asking "what is one month after
  January 31?". G5's refusal of days 29 to 31 is the same answer.
- Its Kani harnesses prove the day-number conversions inverse over its whole range.
- `crates/timetable/src/proofs.rs` records that CBMC did not finish a second 64-bit modulo. That is
  the known risk for any next-occurrence proof.
- `clock_protocol::state` separates `SET` (an operator) from `SYNCED` (an accepted proposal, bounded
  to an hour forward and a second back). S3 can use that; see the finding under run policy.

## G5's shape: six rules

These are the irreversible part.

1. A calendar line opens with `every`, then a cadence word. `every` followed by a glued interval
   (`15m`) stays the monotonic line; a cadence word (`day`, `week`, a count and a plural) makes it a
   calendar line. Both are one token of lookahead.
2. The schedule ends at the time clause, which is last and ends on a fixed token. `at` takes a time
   list; `from HH:MM to HH:MM by Nm` takes a range and ends on the step. The command is what
   follows, and the parser never reads command words as schedule words. That is what keeps a later
   word from changing an old line whose program happens to share its name.
3. Clauses come in one fixed order, with one spelling per meaning. Lists are one token joined by
   commas (`mon,wed,fri`).
4. Each line is exactly one RFC 5545 RRULE with BYSECOND=0 and WKST=MO. Every field RFC 5545 would
   derive from DTSTART is written in the line. DTSTART therefore matters only as a lower bound, and
   as the phase when a count is above one, where the line must carry it.
5. An absent clause keeps its v1 meaning forever. No zone clause means UTC, even if tzdata ships
   one day. No `starting` means unbounded below. A system-wide zone setting may never reinterpret a
   stored line.
6. Run policy, if any is ever added, goes in a prefix slot before `every` or `at-boot`. It is not
   calendar grammar: overlap and jitter mean the same thing for an interval line.

## The v1 word list and each word's RRULE

All times are UTC. The RRULE column omits `BYSECOND=0;WKST=MO`, which every row carries.

| line | RRULE |
|---|---|
| `every day at 02:00` | `FREQ=DAILY;BYHOUR=2;BYMINUTE=0` |
| `every day at 09:00,21:00` | `FREQ=DAILY;BYHOUR=9,21;BYMINUTE=0` |
| `every weekday at 09:00` | `FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;BYHOUR=9;BYMINUTE=0` |
| `every weekend at 10:00` | `FREQ=WEEKLY;BYDAY=SA,SU;BYHOUR=10;BYMINUTE=0` |
| `every week on mon,thu at 08:00` | `FREQ=WEEKLY;BYDAY=MO,TH;...` |
| `every month on 1,15 at 00:00` | `FREQ=MONTHLY;BYMONTHDAY=1,15;...` |
| `every month on last day at 23:00` | `FREQ=MONTHLY;BYMONTHDAY=-1;...` |
| `every month on 2nd tue at 09:00` | `FREQ=MONTHLY;BYDAY=2TU;...` |
| `every month on last fri at 17:00` | `FREQ=MONTHLY;BYDAY=-1FR;...` |
| `every month on last weekday at 18:00` | `FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1;...` |
| `every month on 1 in jan,apr,jul,oct at 06:00` | `FREQ=MONTHLY;BYMONTH=1,4,7,10;BYMONTHDAY=1;...` |
| `every 2 weeks on mon starting 2026-10-05 at 08:00` | `DTSTART:20261005T080000Z`, `FREQ=WEEKLY;INTERVAL=2;BYDAY=MO;...` |
| `every 3 days starting 2026-10-01 at 04:00` | `DTSTART:20261001T040000Z`, `FREQ=DAILY;INTERVAL=3;...` |
| `every 3 months on 1 starting 2026-10-01 at 00:00` | `FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=1;...` |
| `every weekday from 09:00 to 16:45 by 15m` | `FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;BYHOUR=9,10,11,12,13,14,15,16;BYMINUTE=0,15,30,45` |
| `... through 2027-03-31 at ...` | adds `UNTIL=20270331T235959Z` |

A yearly date is a month rule narrowed by `in`: `every month on 15 in mar at 09:00`.

## The grammar

```ebnf
calendar_line = "every" , ws , cadence ,
                [ ws , "in" , ws , months ] ,
                [ ws , "starting" , ws , date ] ,
                [ ws , "through" , ws , date ] ,
                ws , times , ws , command ;

cadence   = "day" | "weekday" | "weekend"
          | "week" , ws , "on" , ws , day_list
          | "month" , ws , "on" , ws , monthday
          | count , ws , "days"
          | count , ws , "weeks" , ws , "on" , ws , day_list
          | count , ws , "months" , ws , "on" , ws , monthday ;
          (* a count requires "starting" and refuses "in" *)

count     = digit , { digit } ;                  (* 2 to 99 *)
day_list  = dow , { "," , dow } ;                (* one token *)
dow       = "mon" | "tue" | "wed" | "thu" | "fri" | "sat" | "sun" ;
monthday  = dom , { "," , dom }                  (* one token *)
          | "last" , ws , "day"
          | ( "first" | "last" ) , ws , "weekday"
          | ordinal , ws , dow ;
dom       = 1 to 28 ;
ordinal   = "1st" | "2nd" | "3rd" | "4th" | "last" ;
months    = month , { "," , month } ;            (* one token *)
month     = "jan" | "feb" | "mar" | "apr" | "may" | "jun"
          | "jul" | "aug" | "sep" | "oct" | "nov" | "dec" ;
date      = digit4 , "-" , digit2 , "-" , digit2 ;
times     = "at" , ws , hhmm , { "," , hhmm }    (* one token; one shared minute *)
          | "from" , ws , hhmm , ws , "to" , ws , hhmm , ws , "by" , ws , step ;
hhmm      = digit2 , ":" , digit2 ;              (* 00:00 to 23:59 *)
step      = digit , { digit } , "m" ;            (* divides 60, or whole hours *)
```

Two constraints sit outside the EBNF. A `starting` date must itself be an occurrence, because
RFC 5545 leaves an unsynchronized DTSTART undefined. A range must be exact: its start minute below
the step and its end the last step of its hour, so `to 16:45 by 15m` and not `to 17:00`.

## Refused in v1, and why

| refused | why |
|---|---|
| days 29, 30, 31 | RFC 5545 skips a month without them, so `on 31` fires seven months a year and says nothing. `last day` is month-end. |
| `5th <dow>` | The same silent skip, in four or five months a year. |
| a month day and a weekday together | Prior art gives union (crontab, launchd), intersection (RRULE, Temporal) and refusal (Quartz, EventBridge). Two lines give the union. |
| times with different minutes in one list | BYHOUR times BYMINUTE is a product, so `09:00,17:30` would also fire at 09:30 and 17:00. Two lines. |
| an inexact range, or a step not dividing 60 | Not one RRULE. The error names the last reachable time. |
| a count with `in` | A fortnightly rule limited to January can skip years, and the bound on the next fire is lost. |
| a count without `starting`, or a `starting` off the rule | The fortnight would depend on the registration date; RFC 5545 calls an unsynchronized start undefined. |
| zones, `TZ=`, zone names | UTC only. `crates/calendar` refuses tzdata. A fixed-offset clause can be added later and absence still means UTC. |
| COUNT (`times 10`) | Needs a durable counter, and the §122 store holds only the document. `through` needs no state. |
| seconds and sub-minute steps | `every 30s` already exists. |
| Quartz `W`, nearest weekday | Not expressible as an RRULE. |
| full day and month names | One spelling per meaning in a stored format. Aliases can be added later. |
| `every N minutes` spelled out | One space away from `every Nm`, which runs on the other clock. |

## Run policy and the S3 step rule

v1 ships no run-policy words; rule 6's prefix slot is where they go. Overlap today is uniform: an
entry fires beside its running children within the budget, and a `--mem` entry drains first.
Changing that is a timetable decision covering interval lines too. Jitter serves fleets, systemd's
own example. If it comes, it should be a fixed offset like `FixedRandomDelay=`, since random jitter
breaks the proof that each fire is an occurrence.

`starting` and `through` are RRULE bounds, evaluated in wall time, and they meet S3 like this:

- While the clock is `UNKNOWN` a calendar line is dormant. Leaving `UNKNOWN` arms the next
  occurrence strictly after now and fires nothing for the past. That transition is not a step,
  because nothing was armed.
- A forward step fires a line at most once, however many occurrences it covered. A range line
  jumped across three hours fires once, not twelve times. The entry's stamp becomes the latest
  occurrence jumped.
- A forward step past `through` fires the last covered occurrence once, and the line is then spent.
- After a backward step, the next fire is the first occurrence after both now and the stamp. Never
  twice.
- An occurrence before `starting` does not exist, so a backward step to before it leaves the line
  dormant until it.

One finding about S3 itself, for the same ruling. The stamp rule has a failure the lane's note does
not price. An operator `SET` typed as 2030 fires every daily line once and stamps it in 2030. A `SET`
back to 2026 then leaves those lines dormant for four years; the plausibility bound admits up to
2100. The page can tell the cases apart, since `state::SET` and `state::SYNCED` differ. A proposed
fix: a publication in state `SET` is a correction and clears the stamps, because they were taken on
a clock the operator has just called wrong. A `SYNCED` step, bounded to a second backward, keeps
them. The cost is that an operator stepping back an hour may see a line fire again, which the
operator asked for.

## What each option costs

Estimates are Rust code lines without comments or tests. The base is today's parser, about 70
lines, and `next_after`, 11. Neighbours were counted on 2026-09-26 (UTC) without comments.

| | G3: two words | G4: cron fields, no union | G5 v1 |
|---|---|---|---|
| covers | nightly, weekly | anything a crontab can, except "last" and fortnightly | the table above |
| parser, estimate | 40 to 60 | 150 to 200 | 250 to 350 |
| next occurrence, estimate | 30 to 50 | 120 to 180, a search over field bitmasks | 200 to 280, closed form per month |
| measured neighbours | none | cronie `entry.c`, 512 | Temporal `spec.go` and `calendar.go`, 746; systemd `calendarspec.c`, 1,059; dateutil `rrule.py`, 1,453 |
| silent traps left | none in scope | `0 0 31 * *` fires seven times a year unless refused | none: each is a refusal above |
| looks familiar to a stranger | yes | yes, and implies local time and the union, which it lacks | reads as English |

The Kani plan is the same for G4 and G5, and only the predicate differs. `matches(rule, t)` is the
specification, written straight from the RRULE fields over a `Civil` built from fields.
`next(rule, after)` is the implementation. Five harnesses:

1. `next` is strictly after `after`, so a polling pass cannot fire twice.
2. `matches(rule, next)`: every fire is an occurrence.
3. For a symbolic `t` between `after` and `next`, `!matches(rule, t)`: no occurrence is skipped.
4. Totality: for every rule the parser accepts, `next` exists within a bound: the count times the
   period, or twelve months for an `in` line. The refusals make this true, since every month has days 1 to 28, a 4th of each weekday, a last
   day and a last weekday.
5. With `through`, `next` is never after UNTIL.

Time would be a day number and a minute of day, both 32-bit, which keeps the modulo off the 64-bit
path that stalled CBMC. Whether harness 3 finishes is not measured, and it is the one to try first.
Host tests would add a fixture table of occurrences generated once by dateutil's `rrule`, checked in
as data with the command that made it, and no crate dependency.

## How each extends later without changing a stored line

- G3 extends by new first words, which are errors today. Reaching calef's range means either a
  family of one-off words, each a small grammar, or a second grammar with `daily` kept as an alias
  forever. The alias is the permanent cost.
- G4 extends by characters it refuses today (`L`, `#`, `W`), and a ruling on the day-field union
  can pick either answer later. Its stored-meaning hazard is familiarity: a line pasted from a Unix
  crontab means local time and the union, and here means UTC or a refusal.
- G5 extends by tokens that are errors today: a cadence (`year`), an `on` value (`5th`), a clause
  before the time (a fixed offset), a prefix policy word. Rules 2 and 5 keep old meanings.

## The options, and what the evidence favours

| option | shape | cost, estimated | reversibility once stored |
|---|---|---|---|
| G3 | `daily HH:MM`, `weekly <dow> HH:MM` | about 100 lines; Kani trivial | growing past it leaves `daily` as a permanent alias |
| G4 | five cron fields, UTC, day-field union refused | 300 to 350 lines; five harnesses | additive, but carries cron's implied meanings |
| G5 v1 | the rules and table above | 450 to 600 lines; five harnesses | additive by construction |
| G5, smaller first cut | G5's shape, v1 words `day`, `weekday`, `week on` | about G3's cost | additive; month words later |

The evidence favours G5's shape. Nothing has acted on a calendar grammar yet, so choosing it now
undoes nothing. How many words ship first is reversible, and the smaller cut buys G3's cost without
G3's dead end.

Question 7 of `AGENTS.md`. At equal cost we would still choose G5. We would not choose G3 at equal
cost: the lane's case for G3 rests in part on its being smaller, and that part of the recommendation
is about effort. G4 loses on meaning rather than cost, because its familiarity promises local time
and a day-field union that nife does not deliver.

## What a ruling unblocks

A yes on G5 lets the milestone 129 lane build the shape with calef's word list, the five harnesses
and the `clock` grant the lane's note lists. The S3 finding wants its own answer: whether a `SET`
clears the stamps.
