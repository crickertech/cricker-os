---
status: BUILT
raised: 2026-08-02
built: 2026-09-19
---
# 64. Enough `std` to run somebody else's crate

Built on 2026-09-19 (PR #984), after five passes: the first on 2026-08-04 (PR #113),
the second on 2026-08-17, three on 2026-08-18, and the last on 2026-09-19, which bound file times
and closed the one item the Follow-on still called outstanding (below, "The last pass: file
times"). Every other row on the ranked list is closed, declined with a recorded reason, or owned by
another milestone or a proposal; the Follow-on names each home. The measurement's deliverable, the prioritised gap list that milestones 99 and 66
consume, is in `notes/crates-io-on-nife.md`: 50 crates.io crates, 43 built, 7 failed, where 43 is
against this tree as it ships and 39 is against it without `entropy_backend`, which is the number
this block carried until the third pass.

No gate held it back, and the fourth pass had to establish that rather than assume it, because the third
pass's report said the opposite: *"the ranked list is now genuinely exhausted for a lane under these
constraints: every remaining row is a decision."* That sentence was wrong twice, and this block
carried a version of it too:

- `std::process::exit` was a trap instruction, found by building the gate the third pass said
  was missing. Not a decision, and not on the list, because the list cannot see a function that ends
  the process. Fixed here.
- **`TcpListener` is a binding, not a decision.** Rank 21 was a contract gap when it was written and
  stopped being one at milestone 107; `sys/net/connection/nife.rs`'s own doc comment has said so
  since, in the words *"small and mechanical"*. It was the largest piece of ordinary work left in
  this milestone and needed no decision from calef; it is bound now (below, "The fourth pass:
  `TcpListener`", PR #341, 2026-08-18).

The block's own sequencing still holds: run the measurement phase first and independently, pick the
probe crates, build them, and let the failures name the work. The `File::open` resolution question is
a design fork inside it, to be raised before code and answered jointly with milestone 47's namespace
half rather than twice.

The measurement is a script now, `script/crate-probes` (name provisional), and that is the third
pass's first deliverable. It had been a prose recipe re-derived by hand three times, and two of those
three produced a wrong headline. A probe is a `[[bin]]` whose `main` calls the crate, both halves
learned by recording `diesel` as a pass twice; a probe that fails is rebuilt for the **host** and
reports `BODY` rather than `FAIL` if the host fails too, which is the check fifty hand-written call
sites could not give anyone.

That split was recorded as 35/15, then 39/11, and is 43/7. The first correction (2026-08-17) was
arithmetic: the headline summed the four failure-class headings, and four crates are in two classes
at once (`zip` and `ring` in A and C, `gix-config` and `gix` in A and B). The second (2026-08-18) is
that 39/11 describes the tree *before* `entropy_backend`, which had landed the same day the number
was re-derived; `rand`, `uuid`, `gix-object` and `gix-actor` build now, and the second run confirming
they still fail without it is one flag rather than an argument (`script/crate-probes --no-backend`).

What is closed. Five ranks turned out to be bindings rather than verbs (`create_dir`, `read_dir`,
`remove_file`, `remove_dir`, `rename` were all dispatched by the FS server since milestones 47 and
48; only the client refused). The second pass added four more, working the list from the top:

- Rank 1, the `getrandom` backend, which was 8 of the 11 build failures and is the one gap on the
  list that is not a `std` gap at all. `entropy_backend` is the fix, on `getrandom`'s own
  documented custom-backend hook over `std::random::SystemRng`. `rand`, `uuid`, `gix-object` and
  `gix-actor` build for nife now, and the other four no longer fail on `getrandom` at all.
- Rank 4, `env`, and it is the sting in a second place: `env::var` was recorded as "no PAL at
  all", which sounded harmless because `getenv` answered `None` honestly. The same fallback's `env()`
  is a `panic!`, so `std::env::vars()` aborted the process, and it compiled perfectly.
- Rank 8, `File::set_len` and **rank 26, `fs::copy`**. Rank 9 (`symlink_metadata`) needed nothing:
  the row was stale, std routes it to `lstat`, which this PAL binds.

The third pass asked a different question, and it is the one worth carrying. Not "what is next on
the ranked list" but "what do the neighbouring functions in that module do", which is where the
second pass's best finding had come from by accident. Run across every module the PAL falls through
rather than binds, it found three more std calls that abort a nife process, all of which compiled
perfectly: `std::env::temp_dir()`, `std::env::split_paths()` and `std::process::id()`. Two are now
answered and one is a constant with its reasoning recorded (`sys/paths/nife.rs`, `sys/process/nife.rs`);
`std_exerciser` asserts both the answers and the four neighbouring refusals on both ISAs.

None of the three could have appeared on the ranked list, and that is the finding rather than the
functions. The list is built from PAL functions that answer `Unsupported`, and a function that aborts
never answers. `env::temp_dir` did have a row, at rank 16, reading "no PAL at all ... needs a
namespace answer, not a PAL one", which is how a fatal defect got filed as a design question; its
demand was undercounted by four probes too. It also corrects this note's most-quoted sentence:
`tempfile` did not "build, link, and return an error", it **died inside `std::env::temp_dir`** before
reaching its own refusal.

The fourth pass turned the third pass's method into a check, and the check found a fifth.
`cargo xtask std-aborts` (name provisional) asks cargo's dep-info which `library/std/src/sys/**`
sources rustc compiled for the nife targets and greps exactly those for bodies that end a process,
against a list carrying a reason per entry. It runs inside `script/test`. That is rung two of
AGENTS.md's ladder where this block's own BUGS section had rung four, and it was worth building
rather than reading a fourth time because **`std::process::exit()` was `crate::intrinsics::abort()`
and no amount of reading would have found it.** `sys/exit.rs` is not a `sys/<module>/mod.rs`
backend; its `cfg_select!` lives inside a function, so "read every module the PAL falls through"
has nothing to read. It hid behind a second thing too: `_start` calls the PAL's `rt::exit` on
`main`'s return directly, and `std::process::exit` is std's **only** caller of `sys::exit::exit`, so
every std test in this tree ended by the path that worked. A clean exit was arriving at its
supervisor as `EVENT_FAULT` with a pc and a faulting address.

What that leaves open, and it is a wire format rather than a PAL arm. `sched::exit()` is
`depart(EVENT_EXIT, 0, 0)`: the §26 message has no field for an exit code, so a supervisor can tell
exit from crash and cannot tell `exit(0)` from `exit(1)`. That is the expensive category (something
two programs agree on) and is recorded in notes/std.md rather than guessed at here.

What remains, and why each one stops here rather than being unfinished. Rank 2, the
`std::os::unix` fallthrough, wants a **uid** and a **file mtime set** that this system does not have
in the form the crates ask for, and answering would be a Unix fiction over a capability refusal.
Rank 3, `thread::spawn`, is this block's own scheduling question, now decided (§105): declined for
now, for want of a customer (see BUGS), and has no build failures behind it. *(The next two sentences are the 2026-08-18 reading and are
wrong now: milestone 47's `touch` added the verbs on 2026-08-24, and both ranks were bound by path
on 2026-09-19; see "The last pass: file times".)* Rank 19,
`Metadata::modified`, is one field in `FSTAT`'s reply away and that makes it a wire-format change; it
wants a `DECISIONS` section and calef, same as rank 2. Rank 28, `File::set_times`, is the same
shape as rank 19 and was never triaged by name in an earlier pass: setting an mtime needs a verb
the FS server does not have any more than reading one does, so it is a second wire-format row behind
the same decision rather than a second decision. Rank 21, `TcpListener`, is closed (below, "The
fourth pass: `TcpListener`"): it was a contract gap when the list was written and stopped being one
at milestone 107, and what was left was a PAL binding (`bind` -> `OP_LISTEN` on a socket id this PAL
allocates, `accept` -> `OP_ACCEPT` into a second id with a frame attached) plus a listen grant on the
stack the program is spawned with, which this milestone's fourth pass bound. `sys/net/connection/nife.rs`
called it *"small and mechanical"* in its own doc comment and notes/net.md's "The inbound half" said
what it took. It was also on the customer path: milestone 55 wants Samba-shaped code, and nothing
that serves a share accepts no connections. Everything else that resolves a path waits on the
`File::open` fork, now milestone 154's tier two, and the third pass drew the line at exactly that
edge: fix the ones that abort, leave the ones that refuse. `current_dir`, `current_exe`, `chdir`
and `home_dir` can each say no in their own signature and still do; `temp_dir` and `split_paths`
could not, so they were answered from §27's existing decision (`.` is the granted directory,
`one_name`'s own words) rather than from a new one.

The sting the measurement found is the one worth carrying forward: **a green build is not evidence.**
`tempfile` compiles, links, and returns "operation not supported" at run time. Raised 2026-08-01,
from a question with a number behind it: does milestone 27 mean ordinary Rust programs run here?

## What 27 actually delivered, and where it stops

`std` on the native ABI is **BUILT**, and the proof program is real: `println!`, `Vec`, `String`,
`Instant`, `SystemTime` and `std::random` all work through the PAL in `patches/std-nife/`.

The bound is in the PAL's own answers:

| module | functions | answering `Unsupported` |
|---|---|---|
| `time` | 8 | 0 |
| `stdio` | 5 | 3 |
| `thread` | 6 | **4** |
| `fs` | 54 | **32** |

`std::fs` has the metadata surface (`size`, `perm`, `modified`, `is_dir`, `read`, `write`, `append`,
`truncate`) and answers `Unsupported` for most of the rest. That is honest rather than broken
(§42: declare what you offer), and it is exactly what milestone 27's own text claims: it widens real
workloads to *"most of crates.io that stays off fs and threads"*. The qualifier is doing the work
in that sentence, and this milestone is about removing it.

## Why now rather than at 27

The pieces that were missing then exist now. The FS service and its wire contract (§27), the three
caretakers and their verb table (§56), extended attributes (§54), and `fs_test_client`'s worked grant
path all landed after 27 did. `std::fs` could not have been backed by a capability-shaped filesystem
that did not yet exist.

And it is on the critical path in a way the roadmap does not currently say: milestone 55 wants
Samba-shaped code, and nothing realistic in that space stays off `fs` and threads.

## How to scope it, which is the whole method

**Do not fill in functions by guessing which matter.** Pick real crates, build them, and let the
failures name the work. The gap that matters is the one a chosen dependency actually hits, and a PAL
completed by inspection would be a large amount of code justified by nobody's use.

Candidate probes, roughly in order of how much they would teach:

- a pure-computation crate with no IO, to establish the floor,
- a serialization crate, which pulls in `alloc` patterns and trait-heavy generics,
- something that opens a file by path, which is where the capability question bites: `File::open`
  takes a path and this system has no ambient authority, so either the PAL resolves against a
  granted directory or the call must keep answering honestly,
- something that spawns a thread, which is the other half.

**The `File::open` question is a design fork, not an implementation task**, and it should be raised
before code is written. §50 chose `bind` over stored paths and §48 settled resolution; how a
`std::fs::File::open("config.toml")` finds its directory capability, or refuses to, is the same
question one layer up. It may be that the honest answer is a program namespace (milestone 47's `PATH`
analysis) rather than a PAL trick.

## The relationship with milestone 47, in both directions

64 needs 47, in tiers rather than all at once.

- Tier one, a bare name against one granted directory, needs nothing from 47's remaining work.
  `File::open("config.toml")` where the program holds a directory capability resolves the way
  `fs_test_client` and the caretakers already resolve names, on machinery that exists: §27's
  contract, §47's rights ladder, §56's verb table.
- Tier two, anything that traverses, needs a namespace to resolve *against*, and that is 47's
  unbuilt half. `Path::new("assets").join("x.png")`, an absolute path, or a program wanting two
  directories all land here. Minted as milestone 154 (2026-08-23): a process that holds two
  directory capabilities, the same gap 47's `bind` independently named.

So 64 can start and get a useful distance before it blocks. It will block sooner than tier one
suggests, because real crates rarely open a bare name in a single directory; they join paths.

And 47 may need 64 more than the reverse. `bind` is a decided mechanism with no forcing use case:
§50 records it as unbuilt, needing "a mount table per process and resolution through it", and nothing
in the shell strictly requires one. A `std` program calling `File::open` with a path is a concrete
demand for exactly that machinery. The same is true of `PATH`, where 47 concluded there is no search
because there is no ambient namespace to search, and that a program namespace **is** an endowment.
64 would be its first real customer.

Sequencing that follows from this. Run 64's measurement phase first and independently: pick the
probe crates, build them, let the failures name the work. It costs 47 nothing and produces the
evidence for how much namespace 64 actually needs, which is the question 47's remaining scope should
be sized against. Then answer `File::open`'s resolution once, as a fork spanning both, rather
than twice. Answered inside 64's PAL it will be a trick; answered as 47's namespace it is the design
both milestones already point at.

## The fourth pass: `TcpListener`, which was a binding and not a decision (2026-08-18)

**Status does not move: still PARTIAL, gate still NONE.** The ranked list is not exhausted (rank 2's
`std::os::unix` fallthrough, rank 3's threads and rank 19's `modified` all still want answers calef
has not been asked for), so a flip to BUILT would be a claim this pass did not earn. What this pass
did earn is the largest piece of *ordinary* work the list had left.

`std::net::TcpListener` was `Unsupported`, and it had stopped being a contract gap at milestone 107:
`LISTEN` and `ACCEPT` have been on the wire, with a listen grant behind them, since that milestone
landed. The PAL is now bound to them, and nothing about it needed a decision. `bind` is
`OP_LISTEN`, `accept` is `OP_ACCEPT` into a second socket id with a frame attached, and a listener
carries no frame at all, because a listener carries no bytes (DECISIONS §25).

The authority is the interesting half, and it is what the gate is built around. A listening port
is a grant `net_stack` was spawned with, so the same `std_exerciser` binary does two different things
on two boots and neither of them is a fallback:

| the stack it was spawned over | what `TcpListener::bind` answers | what the program prints |
|---|---|---|
| `NO_LISTEN_GRANT` | `PermissionDenied` | `listen refused`, then the outbound transcript |
| `listen_grant(7778, 7778)` | granted | `listen ok`, and it serves two host connections |

Both transcripts are pinned byte for byte on both ISAs, so a change that widened the grant check
would turn `listen refused` into `listen ok` and fail in a diff rather than pass with more authority
than it was given. The granted run also asserts the two refusals inside its own grant: a port
outside the range is `PermissionDenied` and the granted port asked for twice is `AddrInUse`.

What this unblocks is milestone 55, and it is not a small step: nothing that serves a share
accepts no connections. A `std` program that can listen is the difference between "a crate compiles"
and "a server runs".

## The last pass: file times, which were a binding for three weeks (2026-09-19)

Ranks 19 (`Metadata::modified`) and 28 (`File::set_times`) were carried here as wire-format
decisions until milestone 47's `touch` lane added `GETMTIME`, `SETMTIME` and `SETMTIME_AT` on
2026-08-24 (DECISIONS §112). From then on they were a PAL binding, and the PAL's own comments went
on saying no verb existed. That is the fifth time this milestone has recorded a refusal outliving
its reason.

What is bound, in `patches/std-nife/overlay/std/src/sys/fs/nife.rs`:

| `std` call | verb | answer |
|---|---|---|
| `fs::metadata(p).modified()`, `DirEntry::metadata().modified()` | `GETMTIME` by name, after the existing `OPEN`/`FSTAT`/`CLOSE` | bound, files and directories |
| `fs::set_times(p, ..)`, `set_times_nofollow` | `SETMTIME_AT`, walking with `dir::WRITE \| dir::SETTIME` | bound, whole seconds |
| `File::metadata().modified()`, `File::set_times`, `File::set_modified` | none: the verbs take a name and a handle has none | `Unsupported`, with a message naming the path form |
| an access time in `FileTimes` | none | `Unsupported`, and the mtime beside it is not written |
| `accessed()`, `created()` | none | `Unsupported` |

The premise checked first, per AGENTS.md's fourth question: open, read, write, stat, list and
remove were already bound (milestones 27, 31, 64's first and second passes, 122) and proven by
`std_exerciser` on all three architectures. The outstanding item was exactly the mtime surface, and
this pass bound only that.

Why the `File` forms refuse rather than remember the name they were opened by: the answer would
describe whatever holds that name now, so after a rename it would report, or stamp, another file,
and succeed. That is a wire question (a handle-taking form, which runs into §112 because a file
handle can never carry `dir::SETTIME`), proposed in
`design/roadmap/504-an-mtime-for-an-open-file.md` rather than decided here.

**Why `set_times` never falls back to `SETMTIME`**: the caller's `SystemTime` is an assertion, which
§112 put behind `dir::SETTIME`; writing the server's own "now" in its place and reporting success
would be exactly the silent substitution this PAL refuses everywhere else.

Proved by `std_exerciser`'s `file_times`, six new transcript lines pinned byte for byte on
aarch64, riscv64 and x86_64: the host-made `motd` reads a wall-clock second inside the same window
`SystemTime::now()` is checked against; a write moves a made file's time forward; a time set by
name reads back in whole seconds (the half-second is truncated); a directory's time reads back by name and through a
listing entry; and four refusals (handle metadata, handle set, an access time with nothing changed,
the granted directory itself) plus a `NotFound` for an empty set on a missing name.

## BUGS

- "Runs unmodified" is the claim to be careful with. A crate that compiles is not a crate that
  works, and a crate that works under one grant may fail under another, because on this system what a
  program can do depends on what it holds. The acceptance evidence has to be a crate doing its job
  with a stated endowment, not a green build.
- The PAL patches std's own source, so every function added here is more surface for
  `toolchain drift` to break against a future nightly. That is a real recurring cost and the reason
  to add only what a probe demands.
- Threads opened a scheduling question this project has now answered: declined, for now.
  `std::thread::spawn` implies a thread the program owns; the kernel has TCBs and a budget model,
  and which of those a `std` thread is stayed open after the second pass, because all four of the
  crates behind rank 3 (`rayon`, `crossbeam-channel`, `tokio`, `ignore`) already compile and link, so
  nothing was blocked on the code and everything was blocked on the answer. The fork is written up
  in full in notes/thread-spawn-fork.md (pull request #394, 2026-08-22): `Tcb::CONFIGURE` consumes
  the `Aspace` capability it binds and the `Thread` struct owns it outright, so no two TCBs can share
  one address space today, which is what a `std` thread actually needs. Two real shapes a fix could
  take were costed (a kernel-level shared VSpace, seL4's own answer and the lineage
  `objtype::ASPACE` already cites, versus sibling processes kept aliased by replicated frame
  mappings, whose apparent cheapness, no syscall touched, does not survive contact with what a
  growing heap needs from an allocator) alongside declining outright. calef decided 2026-08-22
  (§105): decline. No customer for real shared-memory threads exists yet; `thread::spawn` stays
  `Unsupported` permanently rather than pending, and milestone 149's Rayon-parallel NPB-Rust variants
  stay out of scope until one does. §105 is the record; nothing here forecloses building the
  seL4-shaped option later.
- `env` is a table nobody seeds. The environment backend added in the second pass is honest and
  it is also a stub in one direction: a program can set its own variables and cannot be *given* one,
  because there is no endowment to carry it. That is milestone 47's namespace, and until it lands
  every `env::var` a crate reads is `None`. `chrono`'s `TZ` and `clap`'s colour detection both take
  that path, which is fine and is not the same as working.

- A fifth check of the ranked list (2026-08-22) found no genuinely buildable row. After §105
  closed rank 3 and PR #341 closed rank 21, every remaining row is either already closed, already
  declined with a recorded reason (ranks 7, 10, 14, 15, 20, 23, 29, 30), a documented contract
  property rather than a gap (`set_nonblocking`, read/write timeouts and `lookup_host` all say so in
  `sys/net/connection/nife.rs`'s own doc comment), part of the `File::open` fork now held by
  milestone 154, or a wire-format change wanting a `DECISIONS` section (rank 2, rank 19, and now
  rank 28, `File::set_times`, named alongside rank 19 for the first time here). This pass's only
  change was correcting the two stale mentions of rank 21 as still open and naming rank 28. Do not
  read that as the list being exhausted the way the third pass wrongly said it was: a `panic!` or a
  trap instruction hiding behind a green build would look exactly like this too, and only reading
  the PAL's neighbouring functions (the third and fourth passes' method) or building a new gate would
  find one.
- File times are only as true as the server's clock, and it has none. `modified()` reports what
  `GETMTIME` says, and for a file written on this system that is the FS server's per-mount counter,
  so it reads as early 1970, orders wrongly against files the host tool stamped, and restarts each
  boot. And because the engine only moves an mtime forward, a nife write to a host-made file does
  not change its time at all. Proposed as `design/roadmap/497-a-filesystem-server-that-knows-the-time.md`; the
  details are in notes/std/file-times.md's `BUGS`.
- The open-`File` time forms refuse (`File::metadata().modified()`, `File::set_times`), because
  the verbs take a name. `set_times` truncates to whole seconds and refuses an access time whole.
  None of this is exercised through a grant narrower than the mount root from `std`.
- The sweep is a gate now (`cargo xtask std-aborts`, fourth pass), and it is honest about its
  boundary. It covers `library/std/src/sys/` only, because `sys` is the platform layer and a panic
  above it (`path.rs`, `thread/scoped.rs`) is a caller's bug that behaves identically on Linux; a
  first version that swept all of std found about forty of those and none of the platform kind. The
  cost of that boundary is real: **portable std code that is only reachable on a platform this thin
  is invisible to it, and finding those still needs somebody reading. It also proves reachability
  of a *body*, never of a *call*. See notes/std.md, "What still ends a nife process".

**Effort: not estimated**, deliberately. The measurement is the first deliverable: pick the probes,
build them, and report what breaks.
## Follow-on

- **Refused.** Rank 2, the `std::os::unix` fallthrough, stays absent: a uid and a Unix mtime-set
  would be a fiction laid over a capability refusal, and `notes/crates-io-on-nife.md`'s second row
  already reads declined.
- **Refused.** Rank 3, `thread::spawn`, is declined for want of a customer, written up in
  `design/decisions/105-thread-spawn-decline-for-now.md` and in `notes/thread-spawn-fork.md`. It
  stays unsupported permanently rather than pending.
- **Done.** Ranks 19 and 28 no longer want a decision: milestone 47's mtime lane landed the get,
  set and set-at verbs in `crates/filesystem_protocol`, on
  `design/decisions/112-touch-mtime-authority.md`, decided 2026-08-23.
- **Done.** Ranks 19 and 28 are bound by path (2026-09-19, "The last pass: file times" above):
  `metadata(p).modified()` on `GETMTIME`, `fs::set_times` on `SETMTIME_AT`, and the shim's comments
  that said no verb existed are corrected.
- **Milestone 504.** The open-`File` forms (`File::metadata().modified()`, `File::set_times`) still refuse,
  because the verbs take a name; a handle-taking form is a wire change and is
  milestone 504 (an mtime for an open file), `design/roadmap/504-an-mtime-for-an-open-file.md`.
- **Milestone 497.** A file written on nife reads as early 1970, because the FS server stamps its own
  per-mount counter rather than a wall-clock second (notes/touch.md's `BUGS`, and notes/std/file-times.md's
  `BUGS` where a std reader meets it). Giving the server the clock page is
  milestone 497 (a filesystem server that knows what time), `design/roadmap/497-a-filesystem-server-that-knows-the-time.md`.
- **Recorded.** The exit event still carries no exit code, so a supervisor can tell exit from crash
  and cannot tell `exit(0)` from `exit(1)`. The reasoning lives in `notes/std.md`.
- **Done.** `env` is seeded now: `design/decisions/111-inert-config-is-a-validated-page.md` and
  `crates/environment_protocol` define a validated read-only page, and the environment shim under
  `patches/std-nife/overlay/std/src/sys/env/nife.rs` fills `TZ`, `LANG` and `TERM` from it before
  `main` runs.
- **Milestone 154.** Tier-two `File::open`, anything that joins or traverses, still waits on a
  second directory capability reaching a real process. 154 has the mechanism host-tested and
  guest-proved, and nothing in the PAL resolves against two roots.
- **Recorded.** A green build is still not evidence, and the acceptance bar stays a crate doing its
  job under a stated endowment. `script/crate-probes` reports the split and cannot report
  behaviour.
- **Recorded.** The PAL patches std's own source under `patches/std-nife/overlay/`, so every bound
  function is more surface for toolchain drift, which is the reason to add only what a probe
  demands.
- **Recorded.** The abort scan in `xtask/src/main.rs` covers std's platform modules only and proves
  a body reachable, never a call, so portable std code reachable only on a platform this thin stays
  invisible to it.
- **Recorded.** `script/crate-probes` and the abort scan both ship under provisional names, said so
  where they live, and naming is an architect's.

## Index row

Fifty crates.io crates measured: **43 build, 7 fail** (2026-08-18; 39 without `entropy_backend`),
ranked by demand, which is the list milestones 99 and 66 consume and is `script/crate-probes` rather
than a prose recipe. **A green build is not evidence**: four std calls aborted a nife process while
compiling perfectly (`env::temp_dir`, `env::split_paths`, `process::id`, and `process::exit`, a trap
instruction that `cargo xtask std-aborts` found), and none could appear on a list built from
`Unsupported` counts. Closed: the namespace verbs, `getrandom`, `env`, `File::set_len`, `fs::copy`,
`TcpListener`, the four aborts, and last (2026-09-19) file times by path, `modified()` on `GETMTIME`
and `fs::set_times` on `SETMTIME_AT`, which had been a binding for three weeks while the PAL said no
verb existed. Declined with reasons: the `std::os::unix` fallthrough and `thread::spawn` (§105).
Proposed: an mtime through an open `File` (a wire change), and an FS server that stamps a real time
rather than a per-mount counter
