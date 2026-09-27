//! **The interactive boot's undertaker** (milestone 22 phase B.2, the interactive increment).
//!
//! The prompt's the progenitor builds a fresh process for every command a person runs. Before this program
//! existed, each of those processes stayed dead-but-uncollected forever and its region stayed spent,
//! so the progenitor's budget only ever went one way: a long session ran out of memory and the shell started
//! answering "could not spawn (the progenitor is out of memory)". The progenitor could not collect them itself, because
//! there is no non-blocking receive and the progenitor is parked in `RECV` on the shell's spawn channel for its
//! whole life. So the collecting is a second process, and this is it.
//!
//! **Its authority is two endpoint capabilities and nothing else.** `READ` on the progenitor's supervision
//! endpoint, and `WRITE` on the progenitor's result endpoint so it can say one word to whoever is waiting
//! there. No untyped, no frame, no TCB, no address space. It cannot build a process, allocate a
//! page, or reach any child's memory; `Rendezvous::REAP` (DECISIONS §32) is authorized by the
//! supervision relationship the kernel already records, not by holding the region. The pages
//! therefore go back to **The progenitor's** job budget, because the progenitor split the region and §13 says a region
//! belongs to whoever owns it. This process can free a job's memory and can never spend it.
//!
//! **The second capability is milestone 235's whole fix**
//! (design/roadmap/235-a-faulted-job-should-reach-the-prompt.md). A job the kernel killed sends
//! nothing, so the shell's read of the result endpoint had nothing to complete it and the prompt
//! never came back. This process is the one that learns the job died, so it is the one that says
//! so: [`grant_plan::spawnproto::JOB_FAULTED`], once, after the corpse is collected. `WRITE` and no
//! `GRANT`: the right to say that word to the shell, and nothing it could hand on.
//!
//! It is deliberately not a supervisor in the `sub_server_supervisor` sense: it holds no restart
//! policy, because a command a person typed has no business being restarted when it ends. Collecting
//! is the whole job.
//!
//! # BUGS
//!
//! **It can say one word and no more.** A refused reap is still taken as the kernel contradicting
//! itself (it named a thread dead and then refused to collect it) and trapped on, which at least
//! dies loudly at the pc of the mistake. The visible symptom of this process dying is that progenitor's
//! job budget stops coming back and the prompt eventually answers "could not spawn". Milestone 235
//! gave this program a channel for the one thing a person at a prompt has to be told; it did not
//! make it a reporter.
//!
//! **A fault report can park, and then it wedges this loop** (milestone 235, recorded rather than
//! solved). [`report`]'s `SEND` is a rendezvous and the ABI has no non-blocking send, so it
//! completes only when someone reads the progenitor's result endpoint. Every ordinary job leaves the shell
//! with a read outstanding there, so the common case rendezvous immediately. The exception is a
//! **screen-narrowed line** (DECISIONS §106, the shell watches its tail's own fault endpoint
//! instead): if a *non-tail* stage of such a line faults, nobody is reading the result endpoint,
//! this loop parks in the `SEND`, and it collects nothing further until the next command reads
//! there, which then takes the stale word and attributes the fault to the wrong line. The collect
//! happens **before** the report for exactly this reason, so a parked report never costs the
//! reclamation the prompt's memory depends on.
//!
//! Closing it needs either a non-blocking send or a receive that can wait on two endpoints, and
//! both are the syscall surface (DECISIONS §10 (process model), §16 (object revocation)), which is
//! an architect's and not a lane's. Proposed as its own milestone in this lane's report.
//!
//! **A refused reclaim is retried rather than trapped on** (milestone 31 phase 3, 2026-08-17), and
//! that is the one thing this program does that is not a single call. A job behind a directory grant
//! shares its region with the caretaker carrying the grant, so the first reclaim is refused by
//! construction and the retry is what collects it; see [`MAX_ATTEMPTS`], which explains the whole
//! mechanism and why running out is still a trap.
//!
//! It also cannot reap a **hung** job: a live thread is refused on purpose (§32), and nothing here
//! escalates. That is §32's recorded watchdog case and it belongs to milestone 23, not here. At the
//! prompt the forcible tier already exists for the case a person can see: `^C` twice tears the job's
//! region down through the shell (DECISIONS §24), and those jobs are built from the shell's own
//! untyped rather than the progenitor's, so they never reach this program at all.
//!
//! Name: ratified 2026-08-03 (calef), replacing `job_reaper`. Refused `job_reaper` (named for what
//! it does to jobs, while its siblings are named for what they serve, and "reaper" carries a faint
//! sense of taking a life) and, on 2026-08-04, `job_killer` (it claims an authority this program is
//! specifically denied). An undertaker arrives strictly after death and never causes it, which is
//! exactly the constraint DECISIONS §32 enforces by refusing to reap a live thread, and it joins
//! the register its siblings already speak: supervisor, caretaker, undertaker.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{reap, recv_fault, send, yield_now};

/// The supervision endpoint the progenitor endows every job with, held `READ`: the right to receive deaths
/// here, which is the same right §32 makes the right to collect them.
const DEATHS: u64 = 0;

/// **The progenitor's result endpoint, held `WRITE`** (milestone 235): the one the shell reads a job's answer
/// off, and therefore the one a job's *absence* of an answer has to arrive on. Nothing else in this
/// process may be said here, and nothing else is: [`report`] sends one constant.
const REPORT: u64 = 1;

/// **How many times one corpse may be refused before this program treats it as a broken kernel**
/// (milestone 31 phase 3).
///
/// A refusal is no longer always a bug, and that is what this constant is for. A job behind a
/// directory grant shares its region with the `fs_subtree_caretaker` that carries the grant
/// (DECISIONS §92), and the caretaker's serve loop only ends when that region is torn down. So the
/// **first** reclaim of such a region is refused by construction: `reap_region_objects` sweeps the
/// region's endpoints, which wakes the caretaker out of `RECV`, and a thread that can be scheduled
/// is `RefuseAndArm`. The same pass arms §16's kill, the scheduler turns the caretaker into a corpse
/// at its next preemption, and the retry finds nothing left alive and reclaims. That is exactly the
/// contract `reclaim_region` documents for the shell's `^C` escalation, met here for the first time
/// by an ordinary command.
///
/// Two properties make the number a safety net rather than a policy. It is far past what the
/// mechanism needs, because one preemption is enough and this yields between attempts. And running
/// out is still [`fail`], so the failure this program was written to make loud is still loud: a
/// corpse that never becomes collectable is a leak, and a leak here ends the prompt's memory a few
/// commands later with nothing to point at.
const MAX_ATTEMPTS: usize = 1024;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    loop {
        // The kernel is the only sender on this endpoint (§26 clears the child's fault slot at
        // `START`), so the tid is trustworthy without a badge.
        let (event, tid, _pc, _addr, _rsvd) = recv_fault(DEATHS);
        collect(tid);
        // **Only a fault is news** (milestone 235). §26.3 flows clean exits down this endpoint too,
        // and every command a person runs ends in one; a word for those would arrive on the result
        // endpoint behind the child's own answer and the next command's read would take it. So the
        // ordinary path stays exactly as silent as it was.
        if event == abi::fault::EVENT_FAULT {
            report();
        }
    }
}

/// Tell whoever is waiting on the progenitor's result endpoint that the job they are waiting for is dead.
///
/// **After [`collect`], deliberately.** This `SEND` is a rendezvous with no non-blocking form, so it
/// can park (this module's `BUGS` says exactly when); a parked report must not be able to cost the
/// reclamation that keeps the prompt's memory coming back, and putting the collect first is what
/// makes that true rather than hoped.
///
/// The return value is dropped on purpose. The endpoint is the progenitor's and outlives this process, so the
/// only failure the kernel could return here is one this program has no second channel to report on
/// and no authority to do anything about.
fn report() {
    let _ = send(REPORT, grant_plan::spawnproto::JOB_FAULTED, 0, 0);
}

/// Collect one corpse, retrying while the region still holds something that can run.
///
/// The tid arrived on a death message from this very endpoint, so two of §32's refusals are
/// unreachable: it cannot be `StillAlive` and it cannot be another supervisor's child. The third,
/// `NotPermitted`, means the *region* is not yet reclaimable, which is a fact about the region's
/// other residents rather than about this corpse; see [`MAX_ATTEMPTS`]. Yielding between attempts
/// is what lets the armed kill be spent: the thread standing in the way has to reach a preemption,
/// and spinning without giving up the core would be racing it on a single-core machine.
fn collect(tid: u64) {
    for _ in 0..MAX_ATTEMPTS {
        if reap(DEATHS, tid) == 0 {
            return;
        }
        yield_now();
    }
    fail()
}

fn fail() -> ! {
    user_mode_runtime::trap()
}

user_mode_runtime::panic_handler!();
