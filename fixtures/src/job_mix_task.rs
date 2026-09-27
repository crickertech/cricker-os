//! **One task of the multi-tasking workload** (milestone 168), at EL0, where a real workload runs.
//!
//! `crates/job_mix` is the workload's definition and its header carries the argument for *what* the
//! mix is and why AIM7's own categories were kept while its 53 job names were not. This file is one
//! task: it runs the mix when the supervisor releases it, self-times, and reports.
//! `kernel/src/job_mix.rs` is the supervisor that releases it and owns the wall clock.
//!
//! # Why the tasks are processes and not kernel threads
//!
//! Milestone 134's E1 (`ipc_thread_scaling`) already sweeps IPC round-trip latency against thread
//! count, and its threads are **kernel** threads calling `sched::` directly. That is the right shape
//! for a micro-benchmark and the wrong one here. DECISIONS §96 asks what a process kernel costs a
//! *workload*, and a workload is EL0 processes: every job below crosses the trap boundary the way a
//! real program does, and the two jobs that do not ([`job_mix::COMPUTE`] and [`job_mix::TOUCH`]) are
//! there precisely to be the application-mode time between two kernel entries, which is what
//! displaces the cache a returning thread wanted.
//!
//! # The two roles
//!
//! - [`job_mix::ROLE_MIXER`]: block on the go endpoint, run [`job_mix::ROUNDS_PER_TASK`] rounds of
//!   the mix in this task's own order, report `[jobs, ticks, index]`, block again. A go word of
//!   [`job_mix::GO_BREAKDOWN`] instead asks for the last subrun's per-kind ticks, untimed. It never exits;
//!   the supervisor halts the machine when the sweep is done.
//! - [`job_mix::ROLE_ECHO`]: `RECV_CAP` and `REPLY`, forever. The [`job_mix::ROUND_TRIP`] job's other
//!   half.
//!
//! # BUGS
//!
//! - **A mixer's self-timed ticks are not the benchmark's number.** They include whatever the
//!   scheduler did to this task while it was runnable, which is the point of a multi-tasking
//!   benchmark, so they are per-task latency rather than throughput. The number is the supervisor's
//!   wall clock over the whole subrun; these are printed beside it so a subrun with one very slow
//!   task can be told from one with many evenly slow ones.
//! - **The compute grind can be recognised by a future compiler.** It is an LCG folded into a value
//!   that leaves through a `#[inline(never)]` return and is accumulated across rounds, which is the
//!   same defence `kernel/src/bench.rs`'s `busy` uses; nothing checks that it still holds. A
//!   compute job optimised to nothing would show as an implausible jump in the whole sweep rather
//!   than as a subtle skew, which is the failure mode to hope for and not a guarantee.
//! - **The touch job walks this task's own `.bss`.** It is not a fresh mapping per iteration, so it
//!   measures cache displacement and not the page-table work AIM7's virtual-memory jobs also do.
//!   That work is [`job_mix::MAP`]'s, added 2026-09-19.
//! - **The map and spawn jobs report a refusal; the others cannot.** A refused `SPLIT`, retype,
//!   map, configure or start ends the subrun and reports [`job_mix::REPORT_FAILED`] with the kernel's
//!   error, and the supervisor prints `job-mix: FAILED` and halts. That is how the rehearsal proves
//!   `job_mix::MAP_REGION_PAGES` and `job_mix::TASK_BUDGET_PAGES` are large enough on every
//!   architecture, rather than a task silently doing less work than it claims.
//! - **A mixer whose `CALL` is refused keeps counting the job as done.** `user_mode_runtime::call` returns two
//!   words and no status this program can distinguish from a legitimate reply, so a wedged echo
//!   server shows up as a subrun that never completes rather than as an error. The supervisor's
//!   own stall is what says so.
//!
//! Name: ratified 2026-09-05 (calef, milestone 168). Shipped provisionally as `job_mixer`, which
//! was wrong rather than merely inconsistent: **this program does not mix anything.** It is one task
//! *inside* the mix, and the `-er` suffix claimed an agent role it does not have, which is the
//! failure `dwarden` is cited for in AGENTS.md's own evidence, a name for the wrong relationship so
//! a reader who correctly infers the scheme gets it wrong.
//!
//! `job_mix_task` was calef's, over the maintainer's `mix_task`, and it is better for a reason the
//! maintainer had not made: **the family stays greppable as one string**, so `job_mix` finds the
//! crate, the script and this program. `job` alone was refused because this tree already uses the
//! word in the shell sense (milestone 48 is job control, and `job_undertaker` collects them).
//!
//! **`task` is this crate's own word**, not a new one: 26 of the tree's 70 uses of it are inside
//! `crates/job_mix`, and `abi` never uses it, so it competes with no kernel concept.
//!
//! Also refused, carried here from the provisional block this ratification replaced: `aim` and
//! `aim7`, for naming somebody else's benchmark. This keeps AIM7's four methodological properties
//! and none of its 53 jobs, so a name claiming the original would be a claim about comparability
//! that `crates/job_mix`'s `BUGS` explicitly denies. And `worker`, which was taken at the time by
//! `user/src/worker.rs` (`least_authority_demo` since 2026-09-13) and is a generic word besides.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{call, now, recv, recv_cap, reply, send, yield_now};

/// The working set the [`job_mix::TOUCH`] job walks: this task's own memory, sized in
/// `crates/job_mix` against the smallest L1d this project targets.
///
/// A `static mut` rather than a stack array because the user stack is one page and this is 32 KiB,
/// and because every task having its own copy is the point: two tasks sharing one buffer would be
/// measuring a cache line's worth of sharing instead of displacement.
static mut WORKING_SET: [u64; job_mix::TOUCH_WORDS] = [0; job_mix::TOUCH_WORDS];

/// A capability slot this task was given nothing in, for the [`job_mix::NULL_SYSCALL`] job to be
/// refused on. Past every slot `kernel/src/job_mix.rs` grants, and it must stay that way: a slot
/// that ever held an object would make the job invoke it rather than bounce off the kernel's
/// slot check.
const EMPTY_SLOT: u64 = 63;

/// A non-elidable integer grind, the same shape as `kernel/src/bench.rs`'s `busy`: an LCG mixed
/// with an xorshift, folded into a returned value the caller accumulates so the optimizer cannot
/// delete the loop.
#[inline(never)]
fn grind(iters: u64, seed: u64) -> u64 {
    let mut x = seed | 1;
    let mut i = 0;
    while i < iters {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407 ^ i);
        x ^= x >> 29;
        i += 1;
    }
    x
}

/// Read and write every word of [`WORKING_SET`] once, returning a value the caller accumulates so
/// the walk cannot be deleted.
#[inline(never)]
fn touch(seed: u64) -> u64 {
    let mut acc = seed;
    let mut i = 0;
    while i < job_mix::TOUCH_WORDS {
        // SAFETY: `WORKING_SET` is this process's own `.bss` and this program is single-threaded
        // within its address space, so there is no other reference to it anywhere. The index is
        // bounded by the array's own length.
        unsafe {
            let p = &raw mut WORKING_SET[i];
            acc = acc.wrapping_add(p.read()).rotate_left(7);
            p.write(acc);
        }
        i += 1;
    }
    acc
}

/// Where the [`job_mix::MAP`] job maps its frame in the space it builds for itself. Any page-aligned
/// user address works, since nothing else is ever mapped in that space; this one is the address
/// `os_primitives_benchmarker`'s timed maps use.
const MAP_TARGET_VA: u64 = address_space_map::pair_page(0x40_0000);
/// Every architecture this tree targets maps 4 KiB pages at the leaf.
const PAGE: u64 = 4096;
/// Where a child's code and stack go in the address space the [`job_mix::SPAWN`] job builds for it:
/// the address-space map's image base and top stack page, where a loaded program's would be.
const CHILD_CODE_VA: u64 = address_space_map::IMAGE_BASE;
const CHILD_STACK_VA: u64 = address_space_map::STACK_TOP_PAGE;
/// Where this task maps, writable, the one frame it fills with the child's code. Far from anything
/// the loader maps, which is the choice `os_primitives_benchmarker` made for the same frame.
const STUB_SCRATCH_VA: u64 = address_space_map::pair_page(0x0100_0000);

/// A job's syscall was refused: the refusal, as the negative `abi::Error` the kernel returned.
type Refused = i64;

/// A syscall's result as a slot or a refusal: the retype and split verbs return the new slot, or a
/// negative error.
fn slot(r: i64) -> Result<u64, Refused> {
    if r < 0 { Err(r) } else { Ok(r as u64) }
}

/// A syscall that answers zero on success.
fn zero(r: i64) -> Result<(), Refused> {
    if r == 0 { Ok(()) } else { Err(r) }
}

/// **User page mapping**: split a region, build an address space and a frame in it, map the frame
/// at [`job_mix::MAP_CALLS`] fresh addresses, and give the whole region back.
///
/// Returns the ticks spent in `SPLIT` and `DESTROY`, the two calls that take the kernel's one
/// memory-region lock, so the supervisor can say how much of the job is that lock rather than the
/// map path (milestone 168's hazard: at 32 tasks this could be measuring the allocator).
#[inline(never)]
fn map_job() -> Result<u64, Refused> {
    let t0 = now();
    let region = slot(user_mode_runtime::split_region(
        job_mix::SLOT_BUDGET,
        job_mix::MAP_REGION_PAGES,
    ))?;
    let mut region_ticks = now() - t0;
    let aspace = slot(user_mode_runtime::retype_object(
        region,
        abi::objtype::ADDRESS_SPACE,
    ))?;
    let frame = slot(user_mode_runtime::retype_page_frame(region))?;
    let mut i = 0;
    while i < job_mix::MAP_CALLS {
        zero(user_mode_runtime::map_into(
            aspace,
            MAP_TARGET_VA + i * PAGE,
            frame,
            abi::address_space::MAP_RO,
        ))?;
        i += 1;
    }
    let t1 = now();
    zero(user_mode_runtime::destroy_region(region))?;
    region_ticks += now() - t1;
    // The objects are gone with their region; the slots that named them are not, and a fixed
    // table would fill over a subrun if they stayed.
    user_mode_runtime::cap_delete(frame);
    user_mode_runtime::cap_delete(aspace);
    user_mode_runtime::cap_delete(region);
    Ok(region_ticks)
}

/// **Process creation**: [`job_mix::SPAWN_CALLS`] children, each built from EL0, run to its exit,
/// reaped and reclaimed, one at a time. `os_primitives_benchmarker`'s `spawn_one`, per task.
///
/// Returns the ticks spent inside `SPLIT` and `DESTROY` calls, for [`map_job`]'s reason. The yields
/// between a refused `DESTROY` and the next attempt are not counted there: that wait is the child
/// finishing its exit, which is the scheduler's time and not the allocator's.
#[inline(never)]
fn spawn_job(code_frame: u64) -> Result<u64, Refused> {
    let mut region_ticks = 0;
    let mut n = 0;
    while n < job_mix::SPAWN_CALLS {
        let t0 = now();
        let child = slot(user_mode_runtime::split_region(
            job_mix::SLOT_BUDGET,
            job_mix::CHILD_PAGES,
        ))?;
        region_ticks += now() - t0;
        let aspace = slot(user_mode_runtime::retype_object(
            child,
            abi::objtype::ADDRESS_SPACE,
        ))?;
        zero(user_mode_runtime::map_into(
            aspace,
            CHILD_CODE_VA,
            code_frame,
            abi::address_space::MAP_CODE,
        ))?;
        let stack = slot(user_mode_runtime::retype_page_frame(child))?;
        zero(user_mode_runtime::map_into(
            aspace,
            CHILD_STACK_VA,
            stack,
            abi::address_space::MAP_RW,
        ))?;
        user_mode_runtime::cap_delete(stack);
        let tcb = slot(user_mode_runtime::retype_object(
            child,
            abi::objtype::THREAD_CONTROL_BLOCK,
        ))?;
        slot(user_mode_runtime::tcb_cap_insert(
            tcb,
            job_mix::SLOT_CHILD_DONE,
            abi::rights::WRITE,
            0,
        ))?;
        // CONFIGURE consumes the address-space capability, so there is no slot to delete for it.
        zero(user_mode_runtime::tcb_configure(
            tcb,
            CHILD_CODE_VA,
            CHILD_STACK_VA + PAGE,
            aspace,
        ))?;
        zero(user_mode_runtime::tcb_start(tcb, 0, 0, 0))?;
        // The child's done word. Then reclaim: `DESTROY` refuses a region a live thread occupies,
        // and the child is between its `SEND` and its exit for a moment after this returns.
        let _ = recv(job_mix::SLOT_CHILD_DONE);
        loop {
            let t1 = now();
            let r = user_mode_runtime::destroy_region(child);
            region_ticks += now() - t1;
            if r == 0 {
                break;
            }
            if r != abi::Error::NotPermitted as i64 {
                return Err(r);
            }
            yield_now();
        }
        user_mode_runtime::cap_delete(tcb);
        user_mode_runtime::cap_delete(child);
        n += 1;
    }
    Ok(region_ticks)
}

/// The frame every child's code is aliased from, retyped from this task's budget **before** any job
/// splits it, so it outlives every child and every `DESTROY` (a region reclaims in reverse order of
/// what was taken from it). Filled once with [`user_mode_runtime::child_stub::SEND_THEN_EXIT`].
fn prepare_child_code() -> Result<u64, Refused> {
    let frame = slot(user_mode_runtime::retype_page_frame(job_mix::SLOT_BUDGET))?;
    if !user_mode_runtime::map_page_frame(frame, STUB_SCRATCH_VA, true, job_mix::SLOT_BUDGET) {
        return Err(abi::Error::OutOfMemory as i64);
    }
    // SAFETY: `STUB_SCRATCH_VA` is the page just mapped read/write into this address space, and
    // the stub is far smaller than it. The source is this program's `.rodata` and the destination a
    // frame retyped a moment ago, so they cannot overlap. Copied as bytes because the stub's element
    // type differs by architecture (`os_primitives_benchmarker` has the longer argument).
    unsafe {
        core::ptr::copy_nonoverlapping(
            user_mode_runtime::child_stub::SEND_THEN_EXIT
                .as_ptr()
                .cast::<u8>(),
            STUB_SCRATCH_VA as *mut u8,
            core::mem::size_of_val(&user_mode_runtime::child_stub::SEND_THEN_EXIT),
        );
    }
    Ok(frame)
}

/// Run one job. `Ok((value, region_ticks))`: a value the caller accumulates so no job can be
/// optimised away, and the ticks the job spent in region calls (zero for the five that make none).
fn run_job(job: u8, seed: u64, code_frame: u64) -> Result<(u64, u64), Refused> {
    let value = match job {
        job_mix::COMPUTE => grind(job_mix::COMPUTE_ITERS, seed),
        job_mix::TOUCH => touch(seed),
        job_mix::NULL_SYSCALL => {
            let mut i = 0;
            while i < job_mix::NULL_SYSCALL_CALLS {
                // **A real trap, and the cheapest one this ABI has.** `is_granted` invokes a method
                // number no object type defines on a slot this task was given nothing in, so the
                // kernel validates the slot, refuses, and returns: an entry and an exit with no
                // object work between them. `now()` was refused for this job because it is *not* a
                // syscall on any of the three architectures (an unprivileged counter read), so a
                // job named for the trap would have measured a loop.
                core::hint::black_box(user_mode_runtime::is_granted(EMPTY_SLOT));
                i += 1;
            }
            seed
        }
        job_mix::YIELD => {
            let mut i = 0;
            while i < job_mix::YIELD_CALLS {
                yield_now();
                i += 1;
            }
            seed
        }
        job_mix::ROUND_TRIP => {
            let mut acc = seed;
            let mut i = 0;
            while i < job_mix::ROUND_TRIP_CALLS {
                let (r0, _r1) = call(job_mix::SLOT_ECHO, acc, i);
                acc = acc.wrapping_add(r0).rotate_left(11);
                i += 1;
            }
            acc
        }
        job_mix::MAP => return map_job().map(|r| (seed.rotate_left(3), r)),
        job_mix::SPAWN => return spawn_job(code_frame).map(|r| (seed.rotate_left(5), r)),
        // Unreachable while `crates/job_mix`'s own test holds (the mix contains no undefined kind),
        // and a no-op rather than a panic if it ever does not: a task that cannot say anything is
        // better reported by the supervisor's stall than by a fault report interleaved with the
        // console of a board nobody is watching.
        _ => seed,
    };
    Ok((value, 0))
}

/// The task's entry.
///
/// `x0` is the role, `x1` this task's index, and `x2` its seed. The supervisor gives every task a
/// distinct index and a distinct seed, so no two tasks walk the mix in the same order.
#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, index: u64, seed: u64) -> ! {
    if role == job_mix::ROLE_ECHO {
        loop {
            let (_op, reply_slot, arg) = recv_cap(job_mix::SLOT_SERVE);
            reply(reply_slot, arg, 0);
        }
    }

    let order = job_mix::order(seed);
    let mut acc = seed | 1;
    // Built before the first go-ahead, so no subrun times it. A refusal here is kept and reported
    // as the first subrun's failure, because nothing can be said before the supervisor asks.
    let code_frame = prepare_child_code();
    // Per kind: ticks spent in the kind's jobs, and within them in region calls, for the last subrun.
    let mut kind_ticks = [0u64; job_mix::JOB_KINDS];
    let mut region_ticks = [0u64; job_mix::JOB_KINDS];

    loop {
        // The go-ahead. The supervisor hands these out one rendezvous at a time, so a task's own
        // clock starts when it is released rather than when the subrun does; the difference between
        // the two is the release skew the supervisor's own doc comment prices.
        let (go, _, _) = recv(job_mix::SLOT_GO);
        if go == job_mix::GO_BREAKDOWN {
            // Untimed: the supervisor stopped its clock before asking.
            for kind in 0..job_mix::JOB_KINDS {
                send(
                    job_mix::SLOT_REPORT,
                    kind as u64,
                    kind_ticks[kind],
                    region_ticks[kind],
                );
            }
            continue;
        }

        let code = match code_frame {
            Ok(frame) => frame,
            Err(e) => {
                send(
                    job_mix::SLOT_REPORT,
                    job_mix::REPORT_FAILED,
                    e as u64,
                    index | (u64::from(job_mix::SPAWN) << 32),
                );
                continue;
            }
        };
        kind_ticks = [0; job_mix::JOB_KINDS];
        region_ticks = [0; job_mix::JOB_KINDS];

        let t0 = now();
        let mut failed = None;
        let mut round = 0;
        'subrun: while round < job_mix::ROUNDS_PER_TASK {
            for &job in &order {
                // Two counter reads per job, and neither is a syscall on any architecture this
                // tree targets, so the breakdown costs the subrun a few instructions a job.
                let j0 = now();
                match run_job(job, acc, code) {
                    Ok((value, region)) => {
                        acc = value;
                        region_ticks[job as usize] += region;
                    }
                    Err(e) => {
                        failed = Some((job, e));
                        break 'subrun;
                    }
                }
                kind_ticks[job as usize] += now() - j0;
            }
            round += 1;
        }
        let ticks = now() - t0;

        // `acc` is kept alive across the report rather than folded into it: the supervisor reads
        // `index` and would print a corrupted one, and a benchmark that mangles its own labels to
        // defeat the optimizer has traded a readable result for a defence `black_box` already
        // gives. It carries into the next subrun, so nothing above it is dead.
        core::hint::black_box(acc);
        match failed {
            None => {
                send(job_mix::SLOT_REPORT, job_mix::JOBS_PER_TASK, ticks, index);
            }
            Some((job, e)) => {
                send(
                    job_mix::SLOT_REPORT,
                    job_mix::REPORT_FAILED,
                    e as u64,
                    index | (u64::from(job) << 32),
                );
            }
        }
    }
}

user_mode_runtime::panic_handler!();
