# The process view: a listing is a capability, not a fact about the machine

Milestone 126 (the `procps` package), the view stratum. `ps`, `pgrep` and `top` work here, and none of them can enumerate
the machine. What each can
see is one supervision domain, because somebody handed it the endpoint that supervises one. There is
no `/proc` to open, no pid space to scan, and no call in a program's reach that takes a process id it
was not shown.

This note is why that shape, what it costs, and the one place the authority is currently wider than
the job.

## The problem, which the reader already agrees with

`ps aux` on Linux reads `/proc`, and `/proc` is ambient: any process gets it with no grant from
anyone. So the listing is every process on the box, including the ones with secrets in `argv`. Nobody
defends that design; they live with it, and `hidepid` exists because enough people stopped wanting
to.

That is what makes this a good first demonstration of the argument milestone 121 makes for
directories. Enumeration is a larger power than reading something you were handed, and the claim
needs no setup: the reader already knows the Unix behaviour is wrong.

## The design: a view over a supervision subtree

The scope is the supervision subtree, because the kernel already maintains it. A thread's
supervision endpoint is recorded at `START` (`Thread::fault_ep`, DECISIONS §26) and never changes.
The set of threads whose deaths arrive on one endpoint is therefore a set the kernel keeps for its
own reasons, exactly maintained, and it costs nothing to read.

So the domain a viewer sees is the endpoint it holds. Same move `rm -r` makes with a directory
subtree: authority is a subtree, not a global. A scope the system already keeps cannot drift out of
agreement with reality, which is the property a registry would not have had.

The wide grant is not forbidden. It is nameable. An operator's monitor over the whole machine is
a program handed the endpoint that supervises the whole machine, and `caps ps` prints which one that
is *before* anything is spawned. On Linux there is no such distinction to print, and that is the
entire difference. This is Fuchsia's job-handle shape: their `ps` needs a handle to the root job to
see everything, and a handle to a smaller job to see less.

## The surface: one new method, no new syscall

`abi::endpoint::SURVEY`, method 6 on an endpoint capability. A new method inside the established
capability model, not a new syscall number.

```text
invoke(cap, SURVEY, cursor, record, 0) -> (next_cursor, tid, word)
```

- `next_cursor` returns in x0 (a0 on RISC-V), `tid` in x1, and the selected record's word in x2.
- Start with `cursor = 0`. Feed each `next_cursor` back. `abi::survey::DONE` (zero) means finished.
- A negative first word is an `abi::Error`.
- Needs `ENUMERATE`, and pointedly not `READ`. `READ` on a supervision endpoint is what `RECV`
  and `endpoint::REAP` take, so a viewer holding it could reap a child; a domain names its members
  and does not act on them (calef, 2026-08-17). See `capability::Rights::ENUMERATE` and the first
  `BUGS` entry below, which is the finding this right came from.

`state` is one of `abi::survey`'s four codes: `READY`, `RUNNING`, `BLOCKED`, `DEAD`. Only four,
because a supervised thread cannot be found in the other two. `Embryo` has not run, so it has no
recorded supervision endpoint yet; `Finished` is what *un*supervised death looks like, and a
supervised thread dies into `DEAD` and waits for its supervisor.

### `record` is a selector, and a new fact is a new value here

calef ruled on 2026-09-21 that observing another thread is a selector. The argument to `SURVEY`
that was always a zero now names which per-thread record the caller wants, and that record's word
comes back in x2. A new fact becomes a new `abi::survey::record` value; it never becomes a fourth
return register.

The deciding input was a forecast rather than an argument about elegance. A register row holds a
fourth word with no new mechanism at all, and appending one is the fewest moving parts *if the row
stops*. calef's answer was that he expects a third and a fourth fact, which settles it: a mechanism
that has to be redesigned at the sixth field is the wrong mechanism at the fourth. The systems that
already went through this agree, and were read rather than recalled: Linux reached 52 fields in
`/proc/[pid]/task/[tid]/stat` behind a pseudo-file, and Zircon reached 41 topics behind
`zx_object_get_info(handle, topic, buffer, size)`, which is this selector. Nobody grows a register
row.

x0 and x1 are the frame; only x2 belongs to the record. The cursor walk and the tid are the same
whichever record is asked for, so a caller that wants two facts walks the domain twice and joins on
the tid, and a caller that wants one is unaffected by every record it does not ask for. That is what
makes adding a record cost an existing reader nothing, and it is asserted rather than assumed
(`kernel/src/user/survey_record_tests.rs`), because the cheap way to build a selector is to let each
record drive its own walk, and that version makes the tids unjoinable while passing everything else.

`record::STATE` is 0, which is the backward-compatibility claim and not a coincidence. Every
caller written before the selector existed passed a zero into an argument it believed was padding,
so it selects the record it was already reading, and not one of them changes. That is a claim about
a wire rather than a hope, so it is pinned by a test on the constant itself.

An unknown record is `BadMethod`, refused before the walk begins. The selector is part of the
method's name, so an unrecognised one gets the refusal an unrecognised method word gets, and no new
error code was needed. The check is before the walk rather than at the point the record is extracted
because of one case: against an empty domain, a check at extraction never runs, the walk falls
off the end, and the caller is handed `DONE` and prints "no threads" when what really happened is
that it asked a question this kernel does not understand. That is the same failure this method
already refuses for an unauthorized viewer, one axis over.

The rights check still runs first, so a holder without `ENUMERATE` gets `NotPermitted` whatever
record it names. A selector is never a way to probe which records exist.

### The records, and what milestone 282 adds

| record | value | x2 |
|---|---|---|
| `record::STATE` | 0 | one of `abi::survey`'s four state codes |
| `record::PLACEMENT` | 1 | the cpu the thread was **placed on** when it started, or `record::NO_CPU` |

Milestone 282 (a thread's CPU time, and the `top` it makes possible) is NOT-STARTED and is the next
consumer. Under §150 (how does a thread's CPU time reach userspace?)'s original ruling it was a
fourth return word; under the selector it is one
new constant, one arm in `sched::survey_supervised`, and one line in `record::is_known`. Nothing
that reads `STATE` or `PLACEMENT` is touched. §150's substance is untouched too: tick-sampled,
per-thread, scheduled on-CPU time is still what that figure means.

### Placement, and why it is not "where it is running now"

`record::PLACEMENT` reports the core `pick_spawn_target` chose when the thread started. The fact
already existed in the kernel: `sched::spawn_reporting_placement` has handed it to the in-kernel
job-mix supervisor since milestone 240 (the soak reports what happened and not where), as a
return value, which is only available to whoever
did the spawning. A userspace supervisor did not do the spawning, and that missing path is what kept
that supervisor in the kernel.

It is placement rather than location, and the reason is measured rather than tidy. Tracking the
core a thread is on *now* means a store in the context switch, which is the hottest line of the
hottest function. The kernel has exactly that field, `Thread::last_cpu`, and it is behind a
soak-build feature gate because shipping it unconditionally cost 5.7% of `ipc_fastpath`'s
footprint on aarch64 (5788 -> 6120 bytes), over milestone 132 (the fast path's footprint, and a gate so Mach's mistake cannot happen
quietly)'s 5% bound, with riscv64 and x86_64
growing 4.7% and 4.6% behind it. A placement is one store per thread creation, on a path that is
cold by definition, so the same information that was too expensive to keep continuously is free to
keep once. `Thread::placement` is therefore unconditional where `last_cpu` is not.

The cost of that choice is stated where a reader meets the record: a thread stolen onto another
core, or woken onto its waker's (DECISIONS §28 (SMP placement: local wakes), sub-point 2), still reports where it was placed. A reader
wanting "now" does not have it here.

The authority argument, and it is deliberately short. §150 already weighed that a viewer holding
`ENUMERATE` learns an aggregate about threads it cannot otherwise name, and accepted it for a CPU
time counter. A placement is strictly less than what was accepted: one bounded value out of at most
64, written once and never again, against a counter that moves continuously and can therefore be
differenced into a timing channel. A viewer that can already see a tid and a run state learns which
of a handful of cores that thread was put on, and learns nothing whatever about a thread outside the
domain it was handed.

### The cpu id is a name, not an index

The one way to misuse this record, and it is the bug `crates/cpu_set` exists to kill. The online
set is not `0..n`. On the VisionFive 2 it is `{1, 2, 3}`, because slot 0 is an M-mode monitor core
with no MMU, and treating the count as an index put `init` into a parked core's inbox and took three
boots to diagnose on first silicon (notes/visionfive2.md).

So: key a per-core tally by the id this record returns, and never iterate a range. A census built
that way needs no online mask at all, because the ids it observes are by construction a subset of
the online set, and that subset relation is asserted rather than asserted-in-prose
(`the_cpus_a_census_observes_are_a_subset_of_the_online_set`). That is the whole reason this record
can be handed to a userspace supervisor on its own.

What a census cannot do is tell an online core with nothing on it from a core that is not online,
because it never saw either. That is a question about the machine rather than about a domain, and
nothing in this tree answers it for userspace today; see this note's `BUGS`.

### Membership is the relationship, and it is proved

The kernel walks its thread table and reports an entry when `capability::survey_includes(fault_ep,
invoked_ep)` says so, which is `matches!(fault_ep, Some(ep) if ep == invoked_ep)` and nothing else.
Three Kani harnesses in `crates/capability`:

- inclusion holds if and only if the thread's recorded endpoint is the invoked one. Both
  directions matter here, unlike for the reap: one is confinement (a stranger is never shown) and the
  other is truthfulness (a member is never hidden, so a missing row means gone rather than
  concealed);
- the view and the reap have the same scope, for every liveness, so the set a monitor sees and
  the set a supervisor may collect from cannot diverge;
- plus §32's two existing reap harnesses, which this reuses rather than restates.

### Why one entry per call

`SCHED` is given back between entries. A survey that held it for a whole domain would let a
userspace program decide how long the scheduler is locked, which is a latency hole a program could
open on purpose.

The cost is that a survey is a sequence of snapshots rather than one. The cursor is a *slot
index*, not a position in a filtered sequence (`generational_table::Table::iter_from`), which is what makes the
resume safe: the entry that was at slot `k` is the only thing that can be at slot `k`, and if it
died, `k` is empty rather than somebody else's thread. So a resumed walk never reports a member twice
and never resolves a cursor to the wrong thread. It can miss a member born into an already-passed
slot, and it can list a member that dies before the table prints. That is `readdir`'s bargain, taken
knowingly.

## Refused, empty, and populated are three answers

This is the deliverable, not a detail. A monitor that reports nothing because it *could not look*
reads exactly like a quiet machine, which is the worst failure this tool has available. `filesystem_protocol`
chose `EPERM` over an empty listing for the same reason (milestone 108's shape).

| what the viewer holds | answer |
|---|---|
| the endpoint with `ENUMERATE`, domain has members | the rows |
| the endpoint with `ENUMERATE`, domain is empty | `DONE` on the first call: an answer |
| the endpoint **send-only** (`WRITE`, no `ENUMERATE`) | `NotPermitted` |
| the endpoint with `READ` but not `ENUMERATE` (a supervisor that was never widened) | `NotPermitted` |
| nothing in the slot | `NoSuchSlot` |

`pgrep` adds a fifth that is not about authority at all: a selector that matched nothing in a
domain that really has members. See its section below.

The send-only case is the interesting one and it is a real relationship in this tree: a peer that
reports *to* a supervisor holds exactly that. It may send here and it may not look, and the kernel
says so rather than answering with a plausible nothing.

The fourth row is the one the 2026-08-17 rights split added, and it is the direction a reader is
least likely to expect: `READ` is the *stronger* right on this object (it unlocks `RECV` and `REAP`)
and it still does not unlock the view. That is deliberate. The two are not ordered, because
receiving deaths and naming members differ in kind rather than in degree, so a holder that wants
both is granted both, which is what the kernel tests' `hold_supervisor` does.

Nothing this system ships holds both, and that fell out of the split rather than being designed:
`system_initializer` endows `job_undertaker` with `READ` on `deaths` and nothing else, and a `ps`
with `ENUMERATE` on `deaths` and nothing else. So the program that can free a job's memory cannot
enumerate the jobs, and the program that lists them cannot touch one. Before the split there was one
bit and both would have held it.

All four are asserted in one kernel test on both ISAs
(`kernel/src/user/survey_tests::a_viewer_without_the_domain_is_refused_rather_than_shown_an_empty_list`),
and the empty case is in the same test on purpose: neither claim means anything without the other.

## What `ps` is, concretely

Two halves at the IO boundary, which is the crate-and-program pair convention.

- `crates/ps` is the listing: the cursor walk, the buffer, the columns, the refusal catalogue.
  Host-tested in milliseconds, nine tests, and total for *every* reader including one that never
  advances its cursor.
- `components/src/ps.rs` is the syscall and two sinks, about sixty lines.

The kernel's survey tests drive `ps::collect` against the real `endpoint::SURVEY`, so the cursor
protocol is proved end to end rather than by a second copy of the walk written in a test.

Collect first, complain second, print third. The rule of DECISIONS §67 (a program's second stream is a declaration) is that a
program says everything it has to complain about and closes its second stream before it writes a byte of output,
because the reader drains diagnostics to end-of-stream first. A survey cannot know its complaints up
front (an endpoint can die mid-walk), so the whole domain goes into a buffer, then diagnostics, then
the table.

The buffer is the caller's, and that was a gate's doing. It began as a `[Row; MAX_ROWS]` local,
which made `collect`'s frame 4,336 bytes: larger than the 4,096-byte guard page under every kernel
thread stack, so one call could move `sp` past the guard in a single step and land in a neighbouring
thread's stack without ever faulting. `script/stack-frame-check` failed the build and named the
shape, which is the second time that gate has caught a `[T; MAX]` local wearing the clothes of a
bound. A caller-provided slice is the fix it recommends and is better anyway: a program that sizes
its own listing knows where the memory came from, and `ps` sizes it at `MAX_ROWS` while the kernel's
tests size it at eight.

A compile-time assertion in the kernel's survey tests keeps `ps::MAX_ROWS` at least as large as
`sched::MAX_THREADS`, so the shipped program has no truncation case. A caller with a shorter buffer
does, and it is not silent: `Survey::is_complete` is false, nothing is printed, and diagnostics
say the domain has more in it. Same rule as the refusal: a monitor never reports less than it saw
without saying so.

## Where it comes from at the prompt

`Manifest::domain` is a declaration, `Manifest::clock`'s twin: a process domain is not a name a
person types, so there is no token to place and no refusal to write. What the field does is tell
The progenitor which children to endow, and tell a person reading `caps ps` that the authority exists.

Init places the endpoint in `grant_plan::DOMAIN_SLOT` (seven) with `ENUMERATE`, using the same named-slot
mechanism §67 gave the diagnostics stream, for the same reason: how many low slots a child gets
depends on what else the line granted it, and a program that probes a fixed number needs that number
not to move.

The endpoint it places is `deaths`, which is what supervises every job the progenitor spawns for the shell.
So `ps` at the prompt lists this shell's jobs, including itself, and nothing else. Init, the shell,
the terminal, the filesystem server, the compositor, the net stack and every driver are outside it,
which is why `ps | wc` at the boot gate counts a handful of lines where a `/proc`-shaped listing
would count dozens.

## EXAMPLES

At the prompt:

```text
$ ps
         TID  STATE
           5  blocked
           9  running

$ ps > running.txt        the table lands in the file
$ ps | wc                 the table is counted, and no /proc was read to make it
$ caps ps                 the scope, printed before anything is spawned
  ps would grant the new process, and nothing else:
    cap 0  endpoint  result   report its answer back
    cap 7  endpoint  domain   ENUMERATE. the processes this shell's jobs are
                              supervised by, and no others. it can name them and
                              do nothing to them: not receive their deaths, not
                              collect them, and not learn anything about a process
                              outside this domain but that it exists
    ...
```

The refusal, which has no Unix equivalent because on Unix there is no domain to be outside of:

```text
$ ps        ps: this process holds no process-domain capability
```

From a host test, with no kernel at all:

```rust
let mut reader = |cursor: u64| match cursor {
    0 => (1, 7, abi::survey::RUNNING),
    _ => (abi::survey::DONE as i64, 0, 0),
};
// The row buffer is the caller's, because a `[Row; MAX_ROWS]` local outgrew the guard page; see
// below. `ps` sizes it at `MAX_ROWS`, the kernel's tests at eight.
let mut rows = [ps::Row::default(); ps::MAX_ROWS];
let survey = ps::collect(&mut rows, &mut reader);
assert_eq!(survey.rows()[0].tid, 7);
```

## `pgrep`, and the `pkill` that cannot exist

On Unix these are one lookup with two endings. `pgrep` prints the pids that match and `pkill`
signals them, and the only difference is what each does with the answer: `kill(pid)` turns a name
into an action, so on Unix anything that can find a process can end it. That symmetry was going
to be milestone 126's headline demonstration, `caps pgrep` beside `caps pkill`, one narrow and one
wide.

A domain names its members and does not act on them (calef, 2026-08-17). That ruling arrived
before the signalling stratum was built and mostly abolished it, and the reason is one the ABI had
already made without anybody reading it back:

- a survey returns a tid, which is a name and not a capability;
- `abi::tcb` has no `DESTROY`, so there is no method that takes a tid and ends the thread;
- killing a live child is `abi::untyped::DESTROY` on the region it was built from (DECISIONS §24's
  forcible `^C`), held by whoever spawned it, which is the shell and not a monitor.

So `pkill` cannot be assembled out of a view here, and making a domain confer control would be the
one place this system copied the thing it exists to refuse.

What replaces the demonstration, said as a trade rather than a win. `caps pgrep` prints a scope
and there is no `caps pkill` to print beside it, because that program cannot exist. That is a
weaker side-by-side than was promised and a stronger claim than was promised, and the write-up owes
both halves. The cost, plainly: `procps` gets ported without its signalling stratum, and a reader who
expects `kill` to be a program will not find one. Killing stays with the shell that spawned the
thing, which already holds the region.

The claim is asserted rather than argued. `kernel::user::survey_tests` builds a domain, filters it
down to the corpse with `pgrep dead`, and then asserts that the same capability that printed that
tid is refused the reap. Before `Rights::ENUMERATE` existed that assertion would have failed, and
the only thing standing between a `ps` and a reap was the program's own source code.

### Two halves, again

- `crates/pgrep` is the filter: a selector, the match, the four answers, the output format. It
  does not walk, because `ps::collect` already does and a second implementation of `SURVEY`'s
  resume protocol is a second thing that can be wrong. Twelve host tests, no emulator.
- `components/src/pgrep.rs` is the syscall and two sinks, and its capability contract is `ps`'s three
  exactly: the output sink, the domain with `ENUMERATE`, the diagnostics sink. The two manifests
  in `grant_plan` are identical field for field, and that sameness is the readable form of the ruling
  rather than a coincidence worth deduplicating.

### The fourth answer

`ps` distinguishes three outcomes; a filter adds one, and it is the one upstream loses:

| what happened | output | diagnostics |
|---|---|---|
| members matched | the tids, one per line | silent |
| the domain has members, none matched | nothing | `pgrep: none of the 3 processes in this domain are ready` |
| the domain is empty | nothing | `pgrep: this domain holds no processes` |
| the walk was refused | nothing | `pgrep: this endpoint may be sent to, but not looked at ...` |

Upstream's `pgrep` prints nothing and exits 1 for all three of the bottom rows, so a monitor cannot
tell an idle machine from a closed door. The refusal wording is `ps::Survey::complaint`'s, reused
rather than restated: two programs describing one refusal in two ways is a drift waiting to happen,
and splitting `write_diagnostics` at the sentence was the whole change needed to share it.

All four are asserted twice: in `crates/pgrep`'s host tests, and on a real kernel domain on both
ISAs in `kernel::user::survey_tests::a_filter_names_members_and_tells_its_four_answers_apart`.

### What the pattern matches, and why it is not a name

Upstream `pgrep`'s pattern matches a process name, and this system has none (see this note's
`BUGS`: there is `arg0` in `Spawn` and no display name). So the pattern matches the one other thing a
domain honestly knows about a member, which is its run state, spelled with the same four words
`ps` prints in its `STATE` column: `pgrep dead` names the corpses, `pgrep 'r*'` the ready and the
running, `pgrep '*'` everything.

`crates/glob` does the matching, unchanged and unextended. It was the right tool rather than the
nearest one: it is on the verification path already, it has a `cost_bound` nothing can blow past, and
four short words is exactly the shape it is good at. Two deliberate departures from its defaults, both
recorded at the call site: `Dot::Ordinary`, because a state name never begins with a dot and the
leading-dot rule is a filename-listing convention; and glob rather than upstream's extended regular
expression, because no regex engine exists in this tree and taking a dependency to match one of four
words is what DECISIONS §46 declines.

The pattern never crosses a process boundary. It resolves to a small bitmask where the person
typed it, and the mask is what the program is handed. That is the same move `rm -r` makes, where the
recursion flag becomes *rights* on the granted capability rather than text in an argument, and it is
the shape this system keeps arriving at: the boundary carries authority, not strings.

### The limitation that shaped it, which is a property of the boundary

Nothing in this system delivers bytes from a command line to a program. `Endowment::arg` is one
`u64`, `spawnproto` carries three words, and every string-shaped designation a person types (a file, a
directory) arrives at the child as a capability: that is what `rm logs/old` and `doc
notes/glob.md` both are, and it is why `rm` learns nothing about the name it removes.

So a *pattern* is not something the prompt can hand over today, and the shipped `pgrep` is `pgrep
'*'`: it names every member, one tid per line, which is `ps` without the columns and is what pipes.
This is `Prog::Date`'s deliberate under-declaration verbatim, an `ArgSpec::Forbidden` over a program
that reads registers the shell cannot set, and it lifts when `ArgSpec` grows the positional arity
milestone 47 deferred. The filter itself is not waiting on that: the kernel test hands it real
selectors over a real domain, which is the only place in the tree that can.

Worth noticing rather than fixing: this is a capability system's shape showing through a Unix
program's interface, not a missing feature. A command line here is a list of designations, and a
regular expression is not a designation of anything.

## BUGS

- Userspace cannot learn the machine's online cpu set, so a placement census cannot show an idle
  core. `record::PLACEMENT` reports which cpu a thread was placed on, and a tally of those ids is
  a correct census of *where the threads are*. It cannot report a core that is online and carrying
  nothing, because that core appears in no thread's record, and it cannot tell that case apart from
  a core that is parked. Nothing in this tree gives userspace `smp::online_harts_mask`, and this
  record deliberately does not smuggle it out: the mask is a fact about the machine and a survey
  answers questions about a domain.

  Do not work around it with a range. `for cpu in 0..count` is the VisionFive 2 bug
  (`crates/cpu_set`), and it is wrong on a real board in both directions at once. A supervisor that
  needs the full set needs a machine-description path that does not exist yet; a proposed milestone
  for one is in this lane's report.

- A placement can be stale, and the record says so where a reader meets it. It is where the
  thread was *put*, not where it is. A thread stolen by an idle core, or woken onto its waker's core
  (DECISIONS §28.2), goes on reporting its original placement. Keeping the live answer means a store
  in `schedule()`'s switch, which measured 5.7% of `ipc_fastpath`'s footprint on aarch64 and is why
  `Thread::last_cpu` is behind a soak-build feature. The honest reading of this record on a busy
  machine is "the arrangement the placement lottery produced", which is exactly what milestone 240
  built it to report, and not "the current layout".

- Two mechanisms for two questions, and the tree owes a reader the pointer. calef's 2026-09-21 ruling (the
  decisions section provisionally numbered 204, not yet on `main`, so named rather than cited) also
  said that a thread observing itself gets a per-thread page on `rseq`'s shape rather than a selector,
  because the consumer there is an allocator reading its own cpu on every allocation. That page is
  not built. Until it is, a thread that wants its own cpu has no cheap path, and nothing in this
  note is it.

- Holding a domain with `READ` was more authority than looking needs. Fixed 2026-08-17, and the
  entry is kept because the shape recurs and the fix is small enough to reuse.

  The finding: `READ` on a supervision endpoint is also what `RECV` and `endpoint::REAP` take, so a
  viewer endowed a view could take a death message out from under the real supervisor
  (`job_undertaker`, at the interactive boot) or collect a corpse. `ps` did neither, and its source
  was the whole argument that it did not, which is exactly the kind of argument this system exists
  to replace with a mechanism.

  The lane that found it deliberately left it, on the reasoning that splitting view from control
  changes the rights model and is the *same* decision the signalling stratum needs. That
  reasoning turned out to be wrong in a way worth recording, because the signalling stratum
  mostly evaporated when calef ruled that a domain names its members and does not act on them: there
  was no second decision to wait for, and the deferral was buying nothing.

  The fix is `capability::Rights::ENUMERATE`, the kernel-level twin of `filesystem_protocol`'s directory
  `ENUMERATE`, and it is the same argument one layer down. `SURVEY` takes it; `RECV` and `REAP`
  still take `READ`; `system_initializer` grants a viewer `ENUMERATE` alone. So a `ps` does not
  get refused a reap, it cannot name one, which is the ladder's top rung in place of an argument
  about a program's source.

  The tell that one bit was doing three jobs is worth carrying off. `READ` on an endpoint
  unlocked receive, reap and survey, and no grant could express any one of them. When a right
  unlocks operations that differ in kind rather than in degree, it is not a right, it is a
  category.

- The cursor and the tid are machine-wide slot indices, so a viewer can *count* threads outside
  its domain even though it can never name one. Found by the 2026-08-17 security audit
  (design/audit-reports/), recorded-accepted, and the fix proposed as a milestone in that report.

  The mechanism, in two lines of kernel. `sched::survey_supervised` returns `slot as u64 + 1` as
  the `next_cursor`, where `slot` is the index into `Scheduler::threads`, which is the whole
  machine's thread table. And a tid is a `generational_table` generational name, `(generation << 32) | slot`,
  so the low half of every tid a survey reports *is* that same index and the high half is the
  number of times that slot has been recycled since boot, machine-wide.

  What a viewer holding `ENUMERATE` alone can therefore work out about the rest of the system:

  - that other threads exist, from a single member, because its member's slot index is a lower
    bound on how many slots were occupied when that member was created;
  - how many threads were created between two of its own members, by subtracting their two
    cursors. That is the `c2 - c1 >= 2` assertion in
    `kernel/src/user/survey_tests::the_survey_cursor_counts_threads_the_viewer_cannot_name`, which
    builds a stranger between two members and measures the gap;
  - machine-wide churn in a slot, from the generation half, which counts other domains' thread
    lifetimes in that slot and only ever increases.

  Two domains that can each spawn can turn this into a covert channel without sharing any
  capability: one modulates global slot allocation by spawning and exiting, the other polls its own
  members' cursors. The bandwidth is low and nobody has measured it.

  Why accepted rather than fixed here. The honest fix is a per-domain cursor and a domain-local
  thread name, and both change what a tid *is*: `endpoint::REAP` takes a tid, `abi::fault`'s death
  message carries one, and `ps` prints one. So it is a change to something two programs agree on,
  which is the category that cannot be un-shipped by reverting a commit, and it reaches the syscall
  surface: the `REAP` of §16 (object revocation) and the death message of §26 (the fault endpoint). That makes it a
  milestone rather than an audit lane's patch. A cheaper partial exists and is worth weighing against it: return an opaque
  cursor (the slot index XOR a per-endpoint value) and leave tids alone, which closes the
  subtraction channel and leaves the generation half open.

  What is not affected, and it is the part that matters most. A viewer still cannot *name* a
  thread outside its domain, cannot learn its tid or its state, and cannot reap it. The
  confinement claim in `a_domain_is_exactly_the_children_of_the_endpoint_that_was_granted` is
  intact; this is a counting channel beside it, not a hole in it. The `caps ps` line and this
  note's example were corrected in the same lane, because "not learn that a process outside this
  domain exists" was a stronger sentence than the mechanism delivers.

- A process has no name, so there is no `CMD` column. This system has `arg0` in `Spawn` and no
  display name at all. Where a name would live is DECISIONS §164 (whether the kernel resolves a tid
  it already sent), still proposed. Who may read one is answered: a viewer holding `ENUMERATE`
  already learns a member's tid and state, and a name is more about a member already named.

- A survey is a sequence of snapshots. See "why one entry per call" above. A member born into an
  already-passed slot is missed until the next survey, and a row read early may be stale by the time
  the table prints. Fuchsia handles a process dying mid-enumeration and this does not; their answer
  is worth reading when somebody needs one.

- A child that is built but not yet started is not in its domain. Supervision is recorded at
  `START`, so an embryo has no endpoint to match. That is invisible at the prompt (the progenitor starts a job
  in the same breath as building it) and would matter to a builder watching its own construction.

- The survey cursor leaks the thread table's density, and a proposed milestone covers it.
  `survey_supervised` returns a machine-wide thread-table slot index plus one, and a tid is
  `(generation << 32) | slot` over the same table, so a viewer can subtract two members' cursors and
  count threads it cannot name. A test asserts the gap exists rather than pretending it does not.
  Nothing in `ps` or `pgrep` exposes a cursor to a person: `ps::Row` carries a tid and a state, and
  `crates/pgrep` reads the finished `Survey`, which never held one. Do not widen that.

- The comparison against Linux is not apples to apples, and a write-up must say so. Ours lists a
  domain; theirs lists a machine. That is the entire point, and a table putting the two side by side
  without stating it would be dishonest in the way §14's map "tie" caveat exists to prevent.

- `pgrep`'s selector cannot be typed at the prompt, so the shipped program always names every
  member. The reason is a property of the process boundary rather than of the program, and it is
  written up in the `pgrep` section above. The filter's own tests hand it real selectors.

- `pgrep` matches glob, not upstream's extended regular expression, and it matches a run state
  rather than a process name, because there is no process name. `pgrep 'r.*'` therefore does not mean
  what a `procps` user expects and `pgrep 'r*'` does. Recorded where a reader meets the feature
  (`crates/pgrep`'s `BUGS`) as well as here.

- `pgrep` has no exit status to report with, so "nothing matched" is a sentence on the second
  stream rather than upstream's exit 1. A caller wanting to branch on the answer has to count lines.
  `user_mode_runtime::exit` takes no code, and giving it one is a syscall-surface change.

- `ps` lists itself, and in a pipeline it may or may not list its own reader. It is a member of
  the domain it was spawned into, which is truthful and is what Unix's `ps` does too. The pipeline
  half is sharper: both stages of `ps | wc` go into the same domain, so whether `ps` walks before
  or after `wc` exists is a race, and the boot gate saw three lines on one run and two on the next.
  That is a snapshot behaving like a snapshot rather than a bug, and it is why the boot gate asserts
  the header and the scope while the confinement claim is asserted in the kernel test, which builds
  the domain it measures instead of inheriting one.

- A doomed thread does not say so. DECISIONS §16's `killed` flag marks a thread whose region
  owner has torn it down and which has not yet reached a preemption. It surveys as `RUNNING` or
  `READY`. Adding a bit to the state word is additive and cheap; it was left out to keep the first
  method minimal, and the moment something wants to watch a `^C` land is the moment to add it.

- `pmap` cannot be reached from the interactive prompt against any real process. See "What
  building it found" in the `pmap` section below: every `Object::AddressSpace` capability is minted and
  consumed within its own builder's thread, `Tcb::CONFIGURE` removes the space from the registry
  the instant it binds to a thread, and nothing shipped here delegates one to a second program.
  `ps`'s `Manifest::domain` has no analogue because there is nothing alive to wire it to. Fixing
  this is a spawn-protocol question, not a `pmap` change, and it is not decided here.

- `pmap` shows one row per mapped page, with no VA-range coalescing and no size column.
  `abi::aspace::LIST` reads the space's revocation log, which records one entry per page and
  nothing about adjacency between them; upstream `pmap` coalesces contiguous same-permission pages
  into ranges, which would need an ordering guarantee this log does not make.

- `pmap` cannot tell a device mapping from ordinary read/write memory. `kind` is derived from
  `paging::Flags`, which carries no bit for "this is device memory" as far as the syscall handler
  can see, so a `DeviceFrame` mapping (always read/write, never executable) reads as `rw-`,
  indistinguishable from a heap page.

- A resumed `LIST` cursor can land on a slot the space's own log recycled for an unrelated
  mapping, if a tombstoned entry was reused by `record_mapping` between two calls. Unlike a
  survey's slot table, a log entry carries no generation to detect this; see
  `kernel::revoke::list_mapping`'s doc for the mechanism and why it is accepted rather than fixed.

## `pmap`, and `ENUMERATE` extended to the address-space object

`pmap` works on both ISAs, over `abi::aspace::LIST`: `Endpoint::SURVEY`'s split one object type
over. `abi::aspace::MAP_INTO` needs `WRITE`, the authority to shape a space; `LIST` needs
`ENUMERATE`, the authority to look. A capability holding `ENUMERATE` alone lists every mapping and
cannot make one, proved in both directions by `kernel::user::pmap_tests`, the same discipline
`survey_tests` established: a viewer refused `MAP_INTO`, a builder refused `LIST`, an empty space
answering rather than refusing, an empty slot answering `NoSuchSlot`.

The listing reads the space's own revocation log rather than walking page tables. `kernel/src/
revoke.rs` already keeps one entry per mapped page, per space, for reclamation (§13); `LIST` costs
nothing that log did not already pay for, and the answer cannot drift out of agreement with what a
real revoke would find. `revoke::list_mapping` hands back a `va`; the syscall handler turns it into
a permission with `arch::mmu::translate_at(root, va)`, present on both architectures already and
previously reachable only from revocation's own tests. `kind` reuses `abi::aspace::MAP_RO`/
`MAP_RW`/`MAP_CODE`, the same three words `MAP_INTO`'s mode argument takes, rather than a second
vocabulary invented for what is, read back, the same fact about the same page.

### The delegation audit DECISIONS §114 required

Every site that mints an `Object::AddressSpace` capability was found and checked: `components/src/builder.rs`,
`crates/supervision_protocol::build_child_space`, `fixtures/src/hello.rs`'s `aspace_builder`,
`fixtures/src/os_primitives_benchmarker.rs`'s `spawn_one`, and `kernel/src/bench.rs`'s `map_el0`
harness. Every one retypes, maps, and (except `hello.rs`'s deliberately-unconfigured demo)
consumes the capability at `Tcb::CONFIGURE`, all inside the one thread that started it. None
delegates an `Object::AddressSpace` capability to a different program. So the caveat's feared case --
a holder nobody assessed for `ENUMERATE` gaining a real power the day the method starts consulting
the bit -- has no instance in the shipped tree today, and this is the audit's actual finding rather
than an assumption: there was nothing to narrow.

### What building it found, which the fork's wording did not anticipate

`Tcb::CONFIGURE` does not only consume the caller's capability, it removes the space's entry from
the registry `user_aspace_root` (and so `LIST`) resolves through (`take_user_aspace`). So the
instant a space is bound to a thread, every capability that ever named it, including one already
sitting in some other program's capability table, reads as an empty listing rather than a live one --
`kernel::user::pmap_tests::a_capability_outliving_its_space_reads_as_empty` proves this directly by
calling `take_user_aspace` the way `configure_tcb` does and showing `LIST` answers `DONE`.

Combined with the delegation-audit finding above (nothing hands an `Object::AddressSpace` capability to a
second program at all), the practical consequence is that there is no address space anywhere in
this system today that a program other than its own builder can be handed a live view of. `ps`
reaches the interactive shell because `Manifest::domain` tells `system_initializer` which live
supervision endpoint to place in a child's capability table (`deaths`, which persists for a thread's whole
life). Nothing plays that role for an address space, because nothing survives long enough, held by
anyone but its builder, to be worth wiring a manifest field to. `pmap`'s kernel mechanism and its
program are real and proven end to end against a genuine `Object::AddressSpace`
(`kernel::user::pmap_tests`), the same way `ps`'s `survey_tests` prove `SURVEY` without going
through the shell -- but unlike `ps`, that is the only place `pmap` runs today.

This is not this lane's decision to make, and it was not decided here. A fix would need a
builder to hand a narrowed, still-registered view of a space it is constructing to a third party
*before* `CONFIGURE` consumes its own copy, which changes how spawning works rather than how
`pmap` works, and is named as an open finding in `design/roadmap/126-who-else-is-running.md`'s
`BUGS` and `crates/pmap`'s own for whoever picks it up.

## `watch` was built, and then cut (milestone 281)

It is gone as of 2026-09-13, along with `crates/watch`, and the reason is the useful part of this
section rather than the obituary.

`watch` re-walked the domain a bounded number of times and prefixed each table with `CSI 2J`/`CSI H`
(erase, then home the cursor) so the terminal showed the latest snapshot in place. It was granted
exactly what `ps` and `pgrep` are, plus a required argument, and that turned out to be the whole
problem: it held the same three slots from the same `grant_plan` constants (`REPORT`,
`DOMAIN_SLOT`, `DIAGNOSTICS_SLOT`) while being, literally, `ps`'s own loop, down to a `diag_slot()`
fallback whose comment read "`ps`'s own fallback, verbatim".

That is a reusable test and it is why this paragraph stays after the program is gone. In a
capability system "should these be one program or two" is not a matter of taste: two programs are
two programs when they hold different authority. Identical cspaces mean there is no boundary
being drawn, and the split is arbitrary. The refresh needed no capability of its own either, because
the interval was a yield-spin over `monotonic_nanos` (granted to every process unconditionally)
rather than a clock grant, so even the least-authority argument for a separate program was absent.
Had the interval needed a clock, the answer would have gone the other way.

Milestone 281 was minted to fold it into `ps --watch N` on exactly that argument. calef took it one
step further: the flag's entire content was a busy-wait over a table of two columns that barely
changes, so deleting it buys the same simplification and costs nothing anyone was using.

It was never in `watch`'s family to begin with. Upstream `watch` re-runs an arbitrary command,
which this one could not: re-running a named command needs a program to hold spawn authority, which
here belongs to the shell alone (`grant_plan::spawnproto`) and is granted to nothing the shell spawns
(an interruptible child is built with no capabilities in its capability table at all), so there was
no route from "a program is running" to "that program can start a second one". That is the same
category of gap `top`, `pwdx` and `w` were blocked on. So it redrew the one thing it could already
reach: the domain it was spawned into, which is `ps`'s own listing. A very thin member of `top`'s
family wearing `watch`'s name, which is why the name was wrong and why neither it nor `crates/watch`
was ever ratified. calef declined to rule on both while this milestone might retire them; it did.

What replaced it was not a flag but milestone 282, which gave the domain something to rank by:
per-thread scheduled CPU time (DECISIONS §150), read as `abi::survey::record::CPU_TIME` through the
selector §204 (how userspace asks where a thread runs) put on `SURVEY`. The live view came back as `top`, ranked, rather than as a refresh
flag on a table of two columns.

## What this does not build

`w`, the machine-wide statistics and `pidwait`. `w` waits on a fork. The other two are ruled and
wait on effort: §225 (`free` sees the machine and your share) and §226 (`pidwait` takes tids).
`sysctl` is declined (§115 (no `sysctl`)), so is `pwdx` (§224 (no `pwdx`)), and the signalling
stratum is refused (milestone 455 (the signalling stratum of `procps`)). `pmap` is built but
unreachable from the prompt. Every fork is written up, with its premise checked, in
[what is left](process-view/what-is-left.md). One of them corrects this note's own earlier reading:
`pwdx` prints another process's working directory, not its name, and here only the shell has one,
which is why it was declined.

See `design/roadmap/126-who-else-is-running.md`, notes/glob.md (the matcher `pgrep` reuses),
notes/supervision.md (the mechanism this reads),
notes/pipes.md (the second stream), and notes/program-manifest.md (how the grant is declared).
