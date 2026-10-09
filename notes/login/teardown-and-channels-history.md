# Teardown and channels: the history the login module doc carried

<!-- writing-standards: exception. Marked by milestone 860 (comments state the constraint as it is
now) on 2026-10-09 (UTC). Reason: this file is a verbatim historical record moved out of a Rust
module doc, whose sentences were written to that medium's rules and none of these; rewording them
to meet the median would edit the record rather than move it. Splitting these sentences is fair
game for any later lane that touches them for a reason of its own. -->

Moved 2026-10-09 (UTC) from `components/src/login.rs`'s module doc by milestone 860 (comments
state the constraint as it is now), so the doc could state only the constraints as they are. The
entries below are the resolved records, kept with their dates; where a ruling holds the reason
now, it is linked rather than restated.

## Reclaiming a session, resolved 2026-08-23

The program's BUGS once named two candidate shapes for giving back a caretaker's construction
memory and picked neither: a principal's supervision endpoint reaching the login process, or a
caretaker `MemoryRegion::DESTROY`ed by name. Investigating both against the tree's own precedent
found a third, smaller than either, and it is what `mint` now builds: the fourth delegated
capability is `mint`'s own `region`, undropped and narrowed to `WRITE`, the client's logout
ticket.

Before that fix, a successful login ended with the process calling `cap_delete` on its own copy of
the caretaker's construction region the moment the caretaker confirmed descent, and a caretaker's
construction memory never came back at all: every successful login spent `CARETAKER_REGION_PAGES`
and `CLIENT_BUDGET_PAGES` out of `CONSTRUCTION_UT` for the rest of the process's life, with no
logout that gave the memory back.

Why this needed no session and no new supervision plumbing: the candidate this process never
supervises its caretaker at all (`mint` calls `cap_delete` on the caretaker's own TCB the instant
it starts, and sets no fault endpoint), which milestone 152 (durable delegation)'s own doc names as the gap §92 (a caretaker is supervised by the client it serves) left
open ("this says nothing about a caretaker with no client, because none exists yet"). Building a
supervision endpoint that reaches back into this process to ask for a specific principal's teardown
is exactly the durable-session machinery 152 is scoped to design in general; the caretaker's own
construction region is the only thing that ever needed tearing down, and its builder can hand the
means directly to the one party who should hold it, the client, without keeping anything itself.

Why `MemoryRegion::DESTROY` actually works here, checked against §32 (a supervisor may collect a corpse without being able to build one)'s own documented gap rather
than assumed: a supervisor's `Endpoint::REAP` only collects an *already-dead* thread (§32: "it
authorizes collecting a corpse, not killing"); killing a *live* one needs
`MemoryRegion::DESTROY`'s stronger right, and that refuses permanently against a thread `Blocked`
on an endpoint outside the region being destroyed (notes/hung-component.md's case (c), the open,
unsolved half of the hung-component taxonomy). The caretaker built by `mint` is never in that
shape: its own client-facing endpoint (`narrow_ep`, the fourth capability's sibling) is retyped
directly from `region`, so the caretaker's steady state (parked in `receive_cap` between requests)
is case (b), "blocked on an endpoint whose region the supervisor can destroy", which
notes/hung-component.md already documents as working, with collateral: destroying `region` drains
`narrow_ep`'s wait queue, aborts the caretaker's blocked receive, and the armed kill lands at the
caretaker's next scheduling. The one narrow exception is the instant the caretaker is mid-`forward`
to the file service (a `CALL` on `FS_EP`, which `region` does not own): a `DESTROY` attempted in
that exact window is refused, transiently. A client is expected to retry a bounded few times on
`NotPermitted` (`fixtures/src/login_test_client.rs`'s teardown role is the worked example).

`mint`'s failed-descent path used to leak too: the same `region` it abandoned on a refused descent
(the caretaker had already `exit()`ed, so nothing was running in it) is reclaimed right there,
with the same bounded retry, before returning `None`.

A full logout needed no fifth capability, because the third one already carried enough right:
`CLIENT_BUDGET_PAGES` is delegated with `WRITE | GRANT` (every principal's own spending money),
and `WRITE` is the one right `MemoryRegion::DESTROY` needs. Nothing before this fix had a reason
to call it, so the program's BUGS never named it, but any client holding `budget` could always
reclaim it the same way the logout ticket reclaims `region`. `fixtures/src/login_test_client.rs`'s
`LOGOUT` behavior does both, so a full logout gives back everything a session spent.

`CONSTRUCTION_UT` is still sized by whoever spawns this process, and running out (a client that
never logs out) still answers every further login with `login_protocol::DENIED` rather than a
distinguishable error, for `login_protocol`'s own stated reason (a caller must not learn "the
service is out of resources" by comparing outcomes across two attempts with the same identity); a
deployment that wants that not to happen relies on clients actually calling `DESTROY`, which this
program cannot compel and does not police (policing would need to know when a client is genuinely
done, which is exactly the session concept this fix avoided building).

## The destroy order, found by a test that passed

The order the two capabilities are destroyed in is load-bearing, and getting it wrong does not
fail loudly. `mint` splits `region` first and `budget` second, both off `CONSTRUCTION_UT`, so
`budget` sits at the top of its watermark. `crates/regions`' LIFO reclaim (the same rule §16 (object revocation)'s
object revocation and `job_undertaker`'s pool already live under, and the one §92 already named
for a caretaker's own region) only returns a freed child's pages to reusable capacity when it is
the current top; destroying `region` while `budget` is still alive still tears down the caretaker
correctly (`DESTROY` returns success either way) but strands `region`'s pages until
`CONSTRUCTION_UT` itself goes away.

This was found, not merely reasoned about: the first version of the fix's own test destroyed them
in the wrong order, every one of its own assertions passed, and it silently starved a later,
unrelated test in the same suite of real login attempts by leaving thirteen logins' worth of
stranded pages behind (see
`kernel::user::login_tests::caretaker_teardown_reclaims_a_full_session_worth_of_memory`'s own doc
comment). The fix is ordering, not a capability change: destroy `budget` (the third capability)
before `region` (the fourth). The client-facing version of this note, including why it holds
regardless of what other clients do (nothing else is ever split from `CONSTRUCTION_UT` between one
login's two capabilities) but does not generalize to reclaiming two different logins' memory out
of the order they were minted in, is `crates/login_protocol`'s module docs.

## The channel-per-client fix, and the two leaks it surfaced

Resolved, milestone 49 (users and attribution)'s channel-per-client update: `REQUEST` and `RESULT` used to be a single
endpoint pair carrying an actual login's identity and secret, on a single shared staging page
reused by every client this process ever spawned: two concurrent callers could interleave their
words on that one page, exactly the limit `credentialer.rs` still documents for its own verify
page. `filesystem_protocol`'s answer, a channel per client, was copied here: the front door's only
legal message is `CONNECT`, and `connect` answers it with a freshly minted, private request/result
pair and staging page, delegated to exactly the caller that asked. Proven by
`kernel::user::login_tests::two_clients_connecting_together_get_independent_channels_and_neither_observes_the_others_secret`.

Each channel is retyped from its own dedicated region and reclaimed by destroying it, not by
`cap_delete`. An earlier version of `connect` retyped the request/result rendezvous and the
staging page directly from `CONSTRUCTION_UT`, and `_start` answered a finished channel with
`cap_delete` on this process's own three capabilities. That removes the process's own *reference*,
not the underlying kernel objects: a rendezvous retyped by `RETYPE_OBJ` lives in the kernel's own
global registry (`kernel::sched::MAX_RENDEZVOUS`, 512 slots, shared by every process the machine
is running) until the *region* it came from is destroyed, so every connect leaked two of those
slots, permanently, machine-wide. This suite's own tests caught it (a later, unrelated test failed
with "out of rendezvous points" after this program's test-suite connects had quietly spent 58 of
the 512 the whole machine shares): `connect` now splits a small, dedicated region per channel,
retypes everything from it, and `_start` destroys that region once `serve_login` returns, which
reclaims the rendezvous objects, the page frame, and the process's own capability-table slots for
all three in one call. The one cost this still cannot avoid: a channel nobody finishes connecting
(`connect` succeeds but the caller never follows up) has no second party to trigger the destroy,
so its region's pages are abandoned the same way this program's other unreclaimed resources are.

Resolved, 2026-08-26: `MemoryRegion::DESTROY` does not free the destroyer's own capability table
slot, and this process leaked two slots per connect. The symptom was
`kernel::user::login_tests::caretaker_teardown_reclaims_a_full_session_worth_of_memory` refusing
its **second** of ten back-to-back connect-login-logout cycles with `login_protocol::DENIED`, as
though `chris`'s password were wrong, which it is not. The first cycle always succeeded, and an
earlier version of this entry recorded that later cycles succeed too; **that was wrong**, and
finding out cost nothing but letting the test run past its first failed assertion: cycles two
through nine all fail, the second inside `mint` and the rest earlier still, in `connect`.

The cause was found by instrumenting rather than by reasoning, in four steps, each narrowing the
previous one: which branch answers `DENIED` (`mint` returning `None`), which step of `mint`
(`supervision_protocol::build_child`), which step of `build_child_space` (`fill_and_map`'s own
`RETYPE`), and finally which half of the kernel's `memory_region_retype` refused it. That last
step is the one that mattered, because the syscall collapses two unrelated causes into one
`Error::OutOfMemory` (`kernel::memory_region`'s own BUGS says so): the region was **not**
exhausted, `sched::grant` had nowhere to put the capability. This process's capability table has
sixteen slots (`kernel::cap::CAPABILITY_TABLE_SLOTS`).

What filled it: `_start` destroyed each served channel's region and never `cap_delete`d its own
`channel.result` or `channel.region`. A comment here claimed the `DESTROY` covered them, and it
does not and cannot. `MemoryRegion::DESTROY` tears down the objects retyped from a region and
returns its pages, and `revoke_region` deletes every `PageFrame` capability naming a page it just
freed (which is why `channel.page` needed nothing). Neither touches a `Rendezvous` capability, and
nothing anywhere deletes the `MemoryRegion` capability *naming the region being destroyed*: both
stay as live table entries, now stale, until their holder clears them. Eight of sixteen slots are
spent at rest here, and a login at its peak needs six more, so two leaked slots per connect is
exactly one login's worth of headroom: the second login after this process starts gets through
`build_child`'s address space and fails on the next page.

The fix is `discard` (destroy *and* `cap_delete`), used at every site in this program that stops
wanting a region, plus a `cap_delete` for the channel's own result endpoint. It also closed the
same leak on six failure paths that had it silently, including the one `mint`'s own comment used
to describe as unfixable ("this process has no `DESTROY` capability on its own construction
budget's children today", which was never true).

The general fact worth carrying away, since nothing about it is specific to this program: a
long-lived server that destroys a region per request runs out of *capability table slots* while
its memory budget still looks healthy, and the failure surfaces as whatever that server says when
it cannot serve. Every one of the four things ruled out before this was found (`CONSTRUCTION_UT`
sizing to 16384, `OWN_UT_PAGES` to 8192, `kernel::sched::MAX_RENDEZVOUS`,
`kernel::memory_region::MAX_REGIONS`) was a *memory* hypothesis, and the sixteen-slot table was
looked at and passed over because tightening and restoring one slot of margin changed nothing: it
would not, against a leak that spends two slots per request.

A second, unrelated cost was measured while sizing the fix, and it is fixed too. A channel's
region is minted before the login it carries and destroyed after it, so a channel region split
from `CONSTRUCTION_UT` is never the LIFO top when it is destroyed. Every connect therefore
stranded `CHANNEL_REGION_PAGES` of `CONSTRUCTION_UT` permanently: **368 pages of holes** in one
suite run, against 1664 pages of real residents. `CHANNEL_UT_PAGES` is a budget with exactly one
spender, so a channel region is always its only live child and always comes home whole; see that
constant's own doc.

## The capability-table ceiling, and how the channel fix reopened it

This process's own capability table has sixteen slots and eight are spent at rest; `mint` used to
leak one of the remaining eight per successful login by keeping `region`'s capability past a
confirmed descent, which left room for exactly eight logins ever before the capability table
itself (not `CONSTRUCTION_UT`) answered every further attempt with `DENIED`. That was fixed
separately, by dropping `mint`'s own copy of `region` once the caretaker confirmed descent (a
`cap_delete`, not a `DESTROY`). A live login now costs this process's capability table nothing
beyond the width of one `mint` call, regardless of how many clients are logged in at once; see
`kernel::user::login_tests::the_login_service_serves_past_the_old_capability_table_ceiling`.

The channel-per-client update reopened that ceiling from a different direction and closed it
again: a per-connect channel is three more objects and a region, and two of those four
capabilities were never given back. The lesson this file states in two places is the one that
generalizes: `MemoryRegion::DESTROY` frees the region, never the destroyer's own table slot naming
it, so every abandon site goes through `discard`.
