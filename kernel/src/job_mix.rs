//! **The multi-tasking workload benchmark, and the thing that times it** (milestone 168).
//!
//! `design/decisions/96-process-kernel-or-event-kernel.md` has four inputs and three of them are
//! settled. The live one is performance, and the retrospective it rests on says exactly where the
//! difference lives: *"generally within 1% on micro-benchmarks but a 20% performance advantage of
//! the event kernel on a multitasking workload (AIM7)"* (Elphinstone and Heiser, *L4 Microkernels:
//! The Lessons from 20 Years of Research and Deployment*, ACM TOCS 34(1), April 2016, section 4.1,
//! read on 2026-09-04). Every instrument this tree owns is on the left of that sentence. This
//! module is the right of it.
//!
//! `crates/job_mix` is the workload's definition, and its header carries what AIM7 actually is,
//! which of its properties are kept and which of its 53 jobs are deliberately not.
//! `fixtures/src/job_mix_task.rs` is one task. This file is the supervisor: it builds the pool, releases a
//! subrun's worth of tasks, owns the wall clock, and prints.
//!
//! # The division of labour, and why it is the soak's and not the bench's
//!
//! `kernel/src/bench.rs` would have been the obvious home and is the wrong one, for a reason the
//! gate makes concrete: **this milestone's number has to be taken on radon**, and the bench boot
//! has never run on a board. The path a board takes is `script/board-image` plus
//! `script/board-console`, which is a kernel feature that prints to the serial console and a watcher
//! that reads it. So this is shaped like `kernel/src/soak.rs`: `--features job_mix`, the tour ends
//! here instead of halting, output is machine-readable lines on the console, and `script/job-mix` is
//! the QEMU rehearsal that proves the mechanism before anybody carries a card to a desk.
//!
//! It differs from the soak in the one way that matters: **a soak never ends and this one does.**
//! The sweep is finite, and when it is finished this prints [`job_mix::DONE`] and halts.
//!
//! # The pool is built once and released in slices, which is not how AIM7 does it
//!
//! AIM7 forks a fresh set of tasks per subrun. This spawns [`job_mix::MAX_TASKS`] processes once and
//! releases the first N of them per subrun, leaving the rest blocked on their own go endpoints.
//!
//! **Two reasons, and the second is a limitation rather than a design.** The honest one: a process
//! here costs an address space and a loaded image, and `spawn_el0` in `kernel/src/bench.rs` exists
//! to reclaim exactly one of those per iteration; spawning 63 across a sweep would measure the
//! memory-region allocator's fragmentation as much as the scheduler. The one to know: it means the
//! parked tasks' kernel stacks exist during every subrun even though nothing touches them. For the
//! quantity §96 asks about that is the right choice (the cost is in the stacks the machine *cycles
//! through*, and E1 measured the same way), but it is not a faithful reproduction of AIM7's own
//! subrun, and a reader comparing the two should know which is which.
//!
//! # Release skew, priced rather than hidden
//!
//! The supervisor hands out the go-ahead one rendezvous at a time, and a rendezvous is synchronous,
//! so task N starts N round trips after task 0. At the top of the sweep that is 32 sends of roughly
//! a microsecond against a subrun of tens of milliseconds, so it is under a part in a thousand. It
//! is not zero, and the alternative (a shared flag every parked task spins on) was refused: it would
//! have the tasks *not* in this subrun burning cores while the subrun ran, which would make the
//! sweep measure the pool size rather than N.
//!
//! # BUGS
//!
//! - **A number from one boot is a draw from a distribution, and on radon that distribution spans
//!   fifteenfold.** `notes/soak.md` records four runs on that board whose rates differ by that much,
//!   and milestone 240's census explains them: the rate tracks how the boot-time placement lottery
//!   landed. Nothing in this module fixes placement, because nothing in this kernel can. What it
//!   does instead is **print the census** ([`print_census`]) so a run's number is never quotable
//!   without the arrangement that produced it, and the bench procedure in `notes/job-mix.md` asks
//!   for repeated boots rather than one. **A single boot's jobs-per-minute figure is not a result.**
//! - **Each point is the median of [`job_mix::REPEATS`], and that is a decision about this
//!   workload rather than a house rule.** Until 2026-09-19 this kept the best of three, which is
//!   `kernel/src/bench.rs`'s `tp_best` methodology and is right on a shared dev Mac. On radon the
//!   spread between repeats is the tasks' own contention, not somebody else's load, and a best-of
//!   drawn from it moved 29% between boots at `tasks=4`. `job_mix::REPEATS` carries the evidence
//!   for 21 and the median. The fastest and slowest repeats are printed on the same line so the
//!   spread is never hidden, and every repeat still gets its own `job-mix-repeat:` line.
//! - **The breakdown exchange after each subrun is not free for the machine**, only for the clock.
//!   It is `JOB_KINDS` rendezvous per released task, after the clock stops and before the next
//!   subrun starts, so it warms the report endpoint and the supervisor's stack between repeats in a
//!   way the pre-2026-09-19 sweep did not. It is the same for every repeat and every point.
//! - **This does not compare a process kernel against an event kernel**, and it cannot: there is no
//!   event kernel to compare against. It measures what this kernel does under multi-tasking load,
//!   which is the input §96 says it is missing. §96 stays open either way.
//! - **The supervisor is a thread on the machine under test.** It is blocked in `RECEIVE` for the whole
//!   of every timed window rather than spinning, so it is much less load than the soak's yielding
//!   watcher; it is not zero, and it is one more thread the scheduler holds.
//! - **The subrun's wall clock includes the tasks' own report sends.** The last of N tasks finishing
//!   is followed by N rendezvous the supervisor drains, and the clock stops after the last of them.
//!   That is deliberate (AIM7's subrun ends when every task has finished, and reporting completion
//!   is part of finishing) and it is a fixed additive cost that grows with N, so it flatters the
//!   small subruns by a few microseconds.
//! - **Any message on the report endpoint counts as a report.** `subrun` takes the first three
//!   words of whatever arrives, so a task's `CALL` there (two words, answered `Gone` under §246 (a
//!   plain `RECEIVE` never takes a capability)) is counted as that task's report of zero jobs. A
//!   miscompiled task did exactly that on 2026-10-05, and the miscount wedged sweeps instead of
//!   naming the fault (notes/job-mix/spawn-destroy-gone.md). Checking the sender's badge and the
//!   message's shape would turn it into a `FAILED` line.

// **The seven markers in this import moved out of this file** (milestone 324 part 2): `CENSUS`,
// `DONE`, `FAILED`, `KIND`, `POINT`, `STARTED` and `SUBRUN`. They were private `const`s here and
// string literals in `xtask/src/main.rs`, agreeing with `crates/board_console`'s recogniser by a
// reader having checked; that is milestone 268's finding 3 wearing different clothes, and the fix
// is the one that milestone found. They now sit beside the rest of the workload's definition, which
// both halves of the instrument already read, and `crates/boot_ladder`'s header carries the general
// argument for why a printed line is a crate rather than a literal.
#[cfg(feature = "lock_wait")]
use job_mix::LOCK;
use job_mix::{
    CENSUS, DONE, ECHO_SERVERS, FAILED, JOB_KINDS, KIND, MAX_TASKS, POINT, REPEATS, STARTED,
    SUBRUN, TASK_SWEEP,
};

use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::user::{self, Spawn};
use crate::{arch, println, sched, smp};

/// Run the sweep and never come back. The caller is the boot thread at the end of the tour, and
/// this replaces its `arch::halt()`.
pub fn run() -> ! {
    let cpus = smp::online_count();

    // Measured through the progenitor's chain, which this boot replaces; see `soak::run` and
    // `trust::require_program` (milestone 563 (a seal check that reads bytes cannot see a check
    // that was dropped)).
    let Some(image) = crate::trust::require_program("job_mix_task") else {
        println!("{FAILED}no 'job_mix_task' program in the initrd archive; nothing to run");
        arch::halt(arch::HaltReason::measurement_boot());
    };

    let report = sched::create_rendezvous();
    let mut go = [0u64; MAX_TASKS];
    for slot in &mut go {
        *slot = sched::create_rendezvous();
    }
    // One untyped budget and one child-done endpoint per task, for the map and spawn jobs. Built
    // with the pool rather than per subrun, for the reason the pool is: a subrun should time the
    // jobs, not the supervisor's set-up. See `job_mix::TASK_BUDGET_PAGES` for why a task never
    // shares its region.
    let mut budget = [0u64; MAX_TASKS];
    for (i, slot) in budget.iter_mut().enumerate() {
        let Some(region) = crate::memory_region::create(job_mix::TASK_BUDGET_PAGES) else {
            println!(
                "{FAILED}could not create task {i}'s {}-page budget",
                job_mix::TASK_BUDGET_PAGES
            );
            arch::halt(arch::HaltReason::measurement_boot());
        };
        *slot = region;
    }
    let mut child_done = [0u64; MAX_TASKS];
    for slot in &mut child_done {
        *slot = sched::create_rendezvous();
    }
    let mut echo = [0u64; ECHO_SERVERS];
    for slot in &mut echo {
        *slot = sched::create_rendezvous();
    }

    // Where the kernel put each task at spawn, kept rather than forgotten: milestone 240's finding
    // is that this decides throughput on real silicon by up to fifteenfold, so a result printed
    // without it is a number nobody can interpret. Servers first, then tasks, which is the order
    // `census_letter` reads them back in.
    let mut placed = [u8::MAX; ECHO_SERVERS + MAX_TASKS];

    // **The servers before the tasks.** A mixer whose server is not yet receiving simply parks on
    // the endpoint, so the order is not load-bearing for correctness; starting the answering half
    // first keeps a fresh boot's first subrun from timing a server's spawn.
    for (i, &ep) in echo.iter().enumerate() {
        let grant = rendezvous_cap(ep, Rights::READ);
        let started = sched::spawn_reporting_placement(move || {
            user::run(
                image,
                Spawn {
                    arg0: job_mix::ROLE_ECHO,
                    arg1: i as u64,
                    arg2: 0,
                    grants: &[grant],
                    maps: &[],
                },
            )
        });
        let Some((_tid, cpu)) = started else {
            println!("{FAILED}could not spawn echo server {i} of {ECHO_SERVERS}");
            arch::halt(arch::HaltReason::measurement_boot());
        };
        placed[i] = u8::try_from(cpu).unwrap_or(u8::MAX);
    }

    for i in 0..MAX_TASKS {
        // Slot order is `job_mix`'s, and both sides read it from there rather than from a second
        // copy of the numbering. The rights are the least each use needs: a task may write its
        // report and may not read other tasks' reports; it may read its own go endpoint and may not
        // release anybody, including itself; it may call a server and may not answer as one.
        // The budget and the child-done endpoint are the two a task needs to create things: it may
        // spend its own region and nobody else's, and it hears only from its own children.
        let grants = [
            rendezvous_cap(report, Rights::WRITE),
            rendezvous_cap(go[i], Rights::READ),
            rendezvous_cap(echo[i % ECHO_SERVERS], Rights::WRITE),
            memory_region_cap(budget[i]),
            rendezvous_cap(
                child_done[i],
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
        ];
        let started = sched::spawn_reporting_placement(move || {
            user::run(
                image,
                Spawn {
                    arg0: job_mix::ROLE_MIXER,
                    arg1: i as u64,
                    // A distinct, odd, well-spread seed per task, so no two walk the mix in the
                    // same order. Two tasks handed the same seed would be in lockstep, which is
                    // the one property `job_mix::order` exists to prevent.
                    arg2: (i as u64)
                        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                        .wrapping_add(1),
                    grants: &grants,
                    maps: &[],
                },
            )
        });
        let Some((_tid, cpu)) = started else {
            println!("{FAILED}could not spawn task {i} of {MAX_TASKS}");
            arch::halt(arch::HaltReason::measurement_boot());
        };
        placed[ECHO_SERVERS + i] = u8::try_from(cpu).unwrap_or(u8::MAX);
    }

    let hz = arch::timer::frequency();
    println!();
    println!(
        "{STARTED} {MAX_TASKS} tasks and {ECHO_SERVERS} servers on {cpus} online core(s), \
         cntfrq={hz} Hz"
    );
    println!(
        "job-mix: one job is one of {} kinds ({} per round, {} rounds per task per subrun); see \
         crates/job_mix for the mix and what it deliberately leaves out",
        job_mix::JOB_KINDS,
        job_mix::MIX_LEN,
        job_mix::ROUNDS_PER_TASK
    );
    println!(
        "job-mix: this measures THIS kernel under multi-tasking load; it compares nothing against an \
         event kernel and does not decide design/decisions/96-process-kernel-or-event-kernel.md"
    );
    println!(
        "job-mix: each point is the median of {REPEATS} repeats, with the fastest and slowest \
         beside it (since 2026-09-19); a transcript before that date printed the best of 3 as \
         ticks= and jpm=, which is a different statistic from a different mix and is not \
         comparable with jpm_median="
    );
    print_census(&placed);
    println!(
        "{CENSUS} placement decides throughput on real silicon by up to fifteenfold \
         (notes/soak.md, milestone 240), so a jobs-per-minute figure from ONE boot is a draw and \
         not a result; notes/job-mix.md's procedure asks for repeated boots"
    );

    for &tasks in &TASK_SWEEP {
        let mut samples = [0u64; REPEATS];
        let mut kind_ticks = [0u64; JOB_KINDS];
        let mut region_ticks = [0u64; JOB_KINDS];
        #[cfg(feature = "lock_wait")]
        crate::lock_wait::reset();
        for (repeat, sample) in samples.iter_mut().enumerate() {
            let ticks = subrun(report, &go[..tasks]);
            println!("{SUBRUN}tasks={tasks} repeat={repeat} ticks={ticks}");
            *sample = ticks;
            breakdown(report, &go[..tasks], &mut kind_ticks, &mut region_ticks);
        }
        let Some(spread) = job_mix::spread(&mut samples) else {
            unreachable!("REPEATS is a nonzero constant");
        };
        let jobs = tasks as u64 * job_mix::JOBS_PER_TASK;
        println!(
            "{POINT}{tasks} jobs={jobs} repeats={REPEATS} ticks_min={} ticks_median={} \
             ticks_max={} jpm_median={}",
            spread.min,
            spread.median,
            spread.max,
            job_mix::jobs_per_minute(jobs, spread.median, hz)
        );
        // Per kind, summed over every task and every repeat at this point: what each kind of job
        // cost on average, and how much of that was the region calls. Totals rather than a
        // per-job figure alone, so a reader can re-derive the division.
        for kind in 0..JOB_KINDS {
            let kind_jobs = job_mix::jobs_of_kind(kind as u8) * tasks as u64 * REPEATS as u64;
            println!(
                "{KIND} tasks={tasks} kind={} jobs={kind_jobs} ticks={} per_job={} \
                 region_ticks={}",
                job_mix::KIND_NAMES[kind],
                kind_ticks[kind],
                kind_ticks[kind] / kind_jobs.max(1),
                region_ticks[kind]
            );
        }
        #[cfg(feature = "lock_wait")]
        print_lock_wait(tasks);
    }

    println!("{DONE}");
    // Parked, not exited: the watcher saw the marker and tears the run down, and a forgotten QEMU
    // costs nothing in `wfi` (AGENTS.md's rule).
    arch::halt(arch::HaltReason::measurement_boot());
}

/// One subrun: release `go.len()` tasks, wait for all of them, and return the wall-clock ticks.
///
/// The clock starts before the first release and stops after the last report, which is AIM7's own
/// definition of a subrun (it ends when every one of its tasks has completed its jobs) and which
/// this module's `BUGS` prices.
///
/// A task that could not finish reports [`job_mix::REPORT_FAILED`], and this halts on it with the
/// task, the job kind and the refusal, rather than printing a tick count for work that was not
/// done. The clock has already been read by then, and no number is printed for the point.
fn subrun(report: sched::RendezvousId, go: &[sched::RendezvousId]) -> u64 {
    #[cfg(feature = "lock_wait")]
    crate::lock_wait::arm();
    let t0 = arch::timer::now();
    for &ep in go {
        sched::ipc_send(ep, [job_mix::GO_RUN, 0, 0]);
    }
    let mut failure = None;
    for _ in go {
        let [jobs, err, who, ..] = sched::ipc_receive(report);
        if jobs == job_mix::REPORT_FAILED && failure.is_none() {
            failure = Some((err, who));
        }
    }
    let ticks = arch::timer::now() - t0;
    #[cfg(feature = "lock_wait")]
    crate::lock_wait::disarm();
    if let Some((err, who)) = failure {
        let kind = (who >> 32) as usize;
        println!(
            "{FAILED}task {} could not finish a {} job: the kernel refused with {}",
            who & 0xffff_ffff,
            job_mix::KIND_NAMES.get(kind).copied().unwrap_or("unknown"),
            err as i64
        );
        arch::halt(arch::HaltReason::measurement_boot());
    }
    ticks
}

/// Ask each task released in the last subrun for its per-kind breakdown and add it to the running
/// totals. **After the clock stopped**, so it costs the timed window nothing; it is `JOB_KINDS`
/// rendezvous per task, which on the largest point is a few hundred, untimed.
fn breakdown(
    report: sched::RendezvousId,
    go: &[sched::RendezvousId],
    kind_ticks: &mut [u64; JOB_KINDS],
    region_ticks: &mut [u64; JOB_KINDS],
) {
    for &ep in go {
        sched::ipc_send(ep, [job_mix::GO_BREAKDOWN, 0, 0]);
        for _ in 0..JOB_KINDS {
            let [kind, ticks, region, ..] = sched::ipc_receive(report);
            if let Some(k) = kind_ticks.get_mut(kind as usize) {
                *k += ticks;
                region_ticks[kind as usize] += region;
            }
        }
    }
}

/// The `job-mix-lock:` lines for one point: every lock rank some thread found held during a timed
/// window, then the `current_cap` site. See `crate::lock_wait` for what each number can and cannot
/// see, and `job_mix::LOCK` for the line's shape.
#[cfg(feature = "lock_wait")]
fn print_lock_wait(tasks: usize) {
    for rank in 0..crate::lock_wait::ranks() {
        let (contended, wait_ticks) = crate::lock_wait::rank_totals(rank);
        if contended != 0 {
            println!(
                "{LOCK} tasks={tasks} rank={rank} name={} contended={contended} \
                 wait_ticks={wait_ticks}",
                crate::lock_wait::rank_name(rank).unwrap_or("-")
            );
        }
    }
    let (reaps, stack_free_ticks, remove_ticks) = crate::lock_wait::reap_totals();
    println!(
        "{LOCK} tasks={tasks} site=reap reaps={reaps} stack_free_ticks={stack_free_ticks} \
         remove_ticks={remove_ticks}"
    );
    let (calls, contended, wait_ticks) = crate::lock_wait::current_cap_totals();
    println!(
        "{LOCK} tasks={tasks} site=current_cap calls={calls} contended={contended} \
         wait_ticks={wait_ticks}"
    );
}

/// **Print which of the pool is on which core, one line per core** (the shape milestone 240 gave
/// `kernel/src/soak.rs`, for the same reason).
///
/// `placed[i]` is where thread `i` was put at spawn, servers first and then tasks, and `u8::MAX`
/// means nothing knows yet. Tokens are `S<n>` for a server and `T<n>` for a task.
///
/// A core with no threads gets a line too, because that is the informative case: four cores online
/// and one of them empty is an explanation for a slow run, and a census that printed only the
/// occupied cores would hide it. This states where the threads are and draws no conclusion, which
/// is the constraint milestone 240 wrote for itself: if a series of boots shows the rate does not
/// follow the placement, that is the result, and this must not have prejudged it.
fn print_census(placed: &[u8]) {
    println!("{CENSUS} where the kernel placed each thread at spawn: S=echo server, T=task");
    let mut accounted = 0usize;
    for cpu in smp::online_cpus() {
        let here = u8::try_from(cpu).unwrap_or(u8::MAX);
        let n = placed.iter().filter(|&&c| c == here).count();
        accounted += n;
        crate::print!("{CENSUS} core={cpu} threads={n}");
        for (i, &c) in placed.iter().enumerate() {
            if c == here {
                if i < ECHO_SERVERS {
                    crate::print!(" S{i}");
                } else {
                    crate::print!(" T{}", i - ECHO_SERVERS);
                }
            }
        }
        println!();
    }
    // Everything the per-core loop did not name, which has two causes and both deserve a line
    // rather than a silent shortfall: a thread that has not been switched to yet (its `last_cpu`
    // is unset), and a core id outside the online set, which would be a placement bug rather than
    // a census one.
    let missing = placed.len() - accounted;
    if missing != 0 {
        let unknown = placed.iter().filter(|&&c| c == u8::MAX).count();
        println!(
            "{CENSUS} unplaced={missing} of which not-yet-run={unknown} (online={})",
            smp::online_count()
        );
    }
}
