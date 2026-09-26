---
status: PROPOSED
raised: 2026-09-26
---

# 223. The process view is the supervision subtree

Raised 2026-09-26 by the maintainer on `maintainer/126-followups`, from section 6 of
[`notes/process-view/what-is-left.md`](../../notes/process-view/what-is-left.md), which
milestone 126 (the `procps` package: who else is running) wrote in pull request #1349. A developer lane
may not write this directory, so the lane left the text for the integrator. *(Section number
provisional until the merge queue lands it. §221 and §222 are claimed by #1340 and #1350.)*

The tree took this decision by construction when `ps` shipped over `rendezvous::SURVEY` on
2026-08-16. Nothing under `design/decisions/` recorded it, so the alternative is neither built nor
refused. This section writes down what was built and asks calef to rule on the refusal.

## What is being decided

Where a program that lists processes gets its list from. Two shapes were named in milestone 126's
block, under "The other fork: where the process view comes from":

- A. The view is a supervision domain. A viewer holding `ENUMERATE` on a supervision endpoint sees
  the threads that endpoint supervises and nothing else.
- B. Processes live in a separate namespace object with a capability of its own. A holder sees
  whichever set the namespace was built to contain.

The proposed ruling is A, with B refused. The draft text from the note, unchanged in substance:

> The process view is the supervision subtree. A viewer holding `ENUMERATE` on a supervision
> endpoint sees that endpoint's domain and nothing else. A set of processes that is not a subtree
> is expressed by a supervisor whose purpose is to be their common parent. Refused: a separate
> process namespace with its own capability, which can express any set but can also disagree with
> the tree.

## Recommendation: A, and DECIDED by construction if calef agrees

A cannot disagree with reality. Membership is `capability::survey_includes`, a one-line predicate:
a thread is in the view exactly when its `Thread::fault_ep` is the invoked endpoint. That is the
relationship `rendezvous::REAP` already authorizes with, and the Kani proof
`the_view_and_the_reap_have_the_same_scope` in `crates/capability` holds the two to one scope. A
namespace object is a second record of who belongs where, and a second record can drift from the
first. Keeping them in step would be new bookkeeping at every spawn and every death.

The one thing B can say that A cannot is "these unrelated processes, together". Question 4 below
finds that A already says it without a common-parent supervisor, which makes the refusal cheaper
than the note assumed.

## The seven questions

1. What else was considered, and why did each lose?
   - B, a separate namespace object. Loses because it is a second source of truth for membership,
     and because it is a new kernel object type on the syscall surface (§10 (process model: capability-based) and §16 (object revocation)) to buy
     expressiveness nobody has asked for.
   - An ambient listing, Linux's `/proc`. Refused in milestone 126's block before any code: any
     process reads every command line on the machine with no grant.
   - A userspace registry that supervisors report into. Loses for B's reason with a worse failure,
     since a supervisor that forgets to report makes the registry wrong silently.
2. What does the tree already do in the analogous case? The same move, three times. `REAP` scopes
   collection by the supervision relationship (§32 (a supervisor may collect a corpse without being
   able to build one)). `pmap` reads an address space over `ENUMERATE` on the object itself (§114 (`ENUMERATE` extends to the address-space object)).
   `rm -r` acts on the directory capability it was handed, a subtree held rather than named. None
   keeps a second membership record.
3. What is the prior art outside the tree? Read on 2026-09-26, not recalled. Fuchsia's
   `zx_object_get_info` documents `ZX_INFO_JOB_PROCESSES` as requiring a job handle with
   `ZX_RIGHT_ENUMERATE` and returning "one for each direct child Process". That is A, down to the
   right's name and the one-level scope. Linux's `pid_namespaces(7)` says a process "can see ...
   only processes contained in its own PID namespace and in descendants of that namespace", and
   that a process's parent is in the same namespace or an ancestor. So even Linux's namespaces are
   tied to the process tree; neither system ships B as posed.
4. Is the premise true? Mostly, with two corrections.
   - "Subtree" overstates the depth. `SURVEY` reports threads whose fault endpoint is the invoked
     endpoint, so it is one supervision domain, not every descendant. A grandchild appears only if
     its spawner named the same endpoint at `START`. This matches Fuchsia's "direct child", and the
     ruling should say "domain" where it means one level.
   - The common-parent supervisor is not the only way to watch unrelated processes. A monitor
     handed `ENUMERATE` on two supervision endpoints walks each and sees their union. The kernel
     needs nothing new. Its granularity is whole domains: it cannot pick one member out of a
     domain it does not supervise, and that is the confinement working.
5. What does each option cost? A is built and paid for: `survey_supervised` in
   `kernel/src/sched.rs` is 39 lines, the syscall arm about 35, the predicate one, plus one Kani
   proof. B was never built, so its cost is not measured. Its parts are nameable: a new object
   type, create and membership methods on the syscall surface, and membership upkeep on the death
   path that §40 (no reaper of last resort) already keeps careful.
6. How reversible is it, and who has acted on it? `SURVEY` is method 6 on the wire (`crates/abi`)
   with a record selector ruled 2026-09-21. `ps`, `pgrep`, `top` and `swapper` read it, through
   `crates/user_mode_runtime` and `crates/grant_plan`. A ruling for A changes none of them. B could
   still be added later beside A without breaking a reader, so the refusal is reversible. What
   would not be reversible is shipping B and then retiring it once programs hold its capabilities.
7. Would we still choose A if both cost the same? Yes. The argument is fewer moving parts and a
   view that cannot drift, not effort.

## What is blocked until this is answered

Nothing is blocked in code. What waits is the record. A lane building a monitor over processes it
does not supervise, or a `w` that lists sessions, has no section to cite for why B is not on
offer. Section 6 of the note calls that "neither built nor refused". Milestone 282 (a thread's CPU time, and the `top` it makes possible) and any
`pgrep` wait mode inherit whatever scope this rules.

If calef says no to A, the next step is a milestone that measures B's cost. `SURVEY` stays as it
is, since B does not replace it.

## BUGS

- `design/roadmap/126-who-else-is-running.md`, `notes/process-view.md` and `crates/abi`'s `SURVEY`
  rustdoc all say "subtree". The kernel implements one level. Changing the word in those files is
  a naming edit and waits on this ruling.
