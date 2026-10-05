---
status: NOT-STARTED
raised: 2026-09-27
promoted_from: a-job-is-finished-when-its-memory-is-back
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 685. A job is finished when its memory is back

Promoted from `design/roadmap/proposals/a-job-is-finished-when-its-memory-is-back.md` on 2026-10-03 (UTC). The number 685 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by milestone 205 (how a foreign program is told what to do)'s lane,
`milestone/205-designation` (#1402). The slug and every name below are a lane's coinage and
provisional. Every real fix is a change two programs agree on, so it is calef's.

## The problem, measured

A job's region goes back to the progenitor's pool when `job_undertaker` reaps it. The shell shows
the next prompt when it has read the job's output, and nothing orders the two. So a job typed at
once can find the pool still holding the last one. The pool holds one `std`-sized share
(`JOBS_BUDGET_PAGES` in `crates/system_initializer`), so two `std` jobs in a row are where it shows.

CI hit it on 2026-09-27 on #1402, aarch64: `/installed/std-grep needle`, typed right after
`/installed/std-grep needle docs`, answered "could not spawn (the progenitor is out of memory)".
A riscv64 failure earlier that day, on a pipeline typed after a plain `std_exerciser`, fits the
same race; that one was inferred, not measured.

What #1402 ships is a stopgap: the progenitor yields up to 1,024 times and retries before it
answers "out of memory" (`split_job`). That is a timing guess. Whether 1,024 yields cover one reap
depends on what else is runnable, and a guess that holds on a quiet runner may not on a loaded one.
It is kept only until one of the options below is ruled.

## A second consumer

Lane 606's #1418 (milestone 599 (a frame per filesystem client channel)'s window pool) hit the same
missing signal from the other side. The progenitor is never told a job has died, so it hands out
file-server windows round-robin, and an eighth concurrent job shares a window. It needs to know
which job was reaped, after its memory is back, to free that job's window. So the fix should carry
a job identity the progenitor already holds: the child's thread id, which `build_child` returns and
the undertaker's death message carries.

## What already carries it, checked

Nothing does. The undertaker holds two capabilities: `DEATHS`, the supervision endpoint it receives
on, and `REPORT`, the result endpoint the shell reads (`components/src/job_undertaker.rs`). It holds
nothing that reaches the progenitor. A notification (§101 (notification objects)) ORs bits and
cannot carry a thread id, or count. A badge (milestone 599) names a sender, not a job. So every
option below gives the undertaker one new capability and gives some message a new meaning, which is
a change two programs agree on.

## The options

| | the carrier | failure mode |
|---|---|---|
| **A. A badged send on the spawn endpoint** | The undertaker gets a `WRITE` copy of the spawn endpoint, badged. After each collect it sends "reaped, tid". The progenitor's loop tells it from a request by the badge, notes the job gone (and, for #1418, frees its window). A pool split that fails while the progenitor has a job not yet reaped receives on the spawn endpoint until that reap arrives, then retries once | While the progenitor waits, a request from another holder of the spawn endpoint could arrive first. Today the boot shell is the only holder and it is parked in its own request, so nothing can; a second holder would need the wait to set requests aside |
| **B. A notification plus a shared page** | The undertaker writes reaped tids into a small ring on a page and signals a notification the progenitor binds to its spawn receive (`notification::BOUND`) | Two contracts, the ring's layout and the signal, where A has one message |
| **C. The shell waits for the reap** | The undertaker sends a word on the result endpoint after every collect, and the shell reads one per job before it prompts | Does nothing for #1418, which needs A or B anyway. Every shell spawn path must read exactly one word per job; one too few leaves a stale word, one too many hangs the prompt |
| **D. Keep the retry** (#1402 ships this) | Nothing | A pool the reaper would have refilled answers "out of memory" when the runner is busy. A timing guess |

**Recommendation: A.** It is one message on an endpoint the progenitor already serves, it carries
the thread id #1418 needs, and it makes the pool wait for the event it is waiting for rather than
for a number of yields. C would be the more honest prompt on its own, "the prompt came back" meaning
"the job's resources came back", but #1418 needs A or B regardless, so C plus A is more machinery
than A alone. The prompt can adopt C's promise later on top of A, since A is what tells the system
a job's memory is back. D is chosen on effort, and 205's `BUGS` says so.

## The seven questions

1. Considered and refused. B, C and D above, each with its reason. A caller-side sleep, because
   there is no timed wait at the prompt (§147 (a timer a userspace service cannot hold) is not wired
   to the shell). Folding the undertaker into the progenitor, because the undertaker exists so that
   a parked report cannot stall spawns (`components/src/job_undertaker.rs`, `report`).
2. What the tree does already. The undertaker already sends `JOB_FAULTED`, after collecting, on
   the result endpoint; A sends its sibling to the progenitor instead. Milestone 599 built badged
   endpoints for telling senders apart on one endpoint, which is A's discriminator. Jobs the shell
   supervises itself (`spawnproto::Wiring::screen`, §106 (the `terminal_sink_caretaker`
   narrowing)) and interruptible jobs are not reaped by the undertaker, so A's count must leave them
   out.
3. **Prior art, from memory and not re-read.** Unix delivers `SIGCHLD` to the parent and a
   `waitpid` names the child, so the parent learns which process ended. seL4 leaves it to the
   user-level manager, which is this tree's shape: the undertaker is that manager's reaping half.
4. Is the premise true? Yes, measured once in CI on 2026-09-27 (#1402, aarch64), with an
   earlier riscv64 failure that fits it, and #1418 found the missing signal independently. A larger pool moves the line and does not
   order the reap.
5. **Cost.** A: one badged capability in the undertaker's table, one message kind in `spawnproto`
   (constants there, which both programs already link), a per-job table or count in the progenitor,
   and a receive-until-reaped on a failed split.
6. **Reversibility.** Inside the tree; no outside program has acted on it. It changes
   `spawnproto`, which the shell, the progenitor and now the undertaker read.
7. At equal cost? A, still, because it serves both consumers with one mechanism.

## What is blocked on the answer

Nothing is blocked; #1402's retry holds until then. #1418's window reuse waits on it.

## Index row

The shell shows the next prompt when it has read a job's output, but the job's memory returns when `job_undertaker` reaps it, and nothing orders the two. Proposed: a job is finished only when its memory is back; every real fix changes something two programs agree on.
