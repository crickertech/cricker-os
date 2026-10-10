---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: a-per-cpu-allocator-is-what-the-current-cpu-page-was-for
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 561. A per-CPU allocator is what the current-CPU page was for

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `a-per-cpu-allocator-is-what-the-current-cpu-page-was-for` on 2026-09-22, filed 2026-09-21. Raised by the lane that built the current-CPU page, as the work its
own change exists to serve and which nobody is doing.

The page ships; a reader can call `user_mode_runtime::current_cpu` today.

## Why this belongs on the list rather than in a report

The page was justified by a consumer that does not exist yet. That is not a flaw in the reasoning:
the consumer is what decided the shape (a load rather than a crossing, because an allocator asks on
every allocation), and the shape had to be decided before anything could be built on it. But it does
mean the mechanism currently has **no measured benefit at all**, and the numbers quoted for it are
Linux's, about Linux. Until something allocates against it, nife has a fast answer to a question
nothing asks.

The honest version of that in a `BUGS` section is already written. This is the version that gets it
fixed.

## What it would be

A small-object allocator in userspace that keeps a free list per CPU id, reads its own CPU on each
allocation, and falls back correctly when the answer turns out to be the previous core's. The
fallback is the interesting half and the reason this is a milestone rather than an afternoon: the
value can be stale the instruction after it is read, Linux solves that with `rseq`'s restartable
sequences, and this tree deliberately does not have those. So the design question is what
correctness argument replaces them, and the candidates (a per-list lock taken only on the slow
path, a compare-and-swap that tolerates the wrong list, an owner check on free) differ in exactly
the cost this page was chosen to avoid.

## What it would prove, which is the point

- A number. Allocations per second against the same allocator with the CPU read removed, on all
  three architectures, which turns the page from an argument into a measurement.
- Whether the staleness matters in practice, which nothing in this tree currently knows.
- Whether `CPU_ID_BOUND` is the right thing to size by. Eight lists per process is the current
  answer and it is a guess; an allocator is what makes the cost of that guess visible.

## What would make it not worth doing

If the answer to the correctness question turns out to want restartable sequences, this stops being
an allocator milestone and becomes an `rseq` milestone, which is a much larger thing and a syscall
surface question. Finding that out early is a good outcome of starting it.

## The consumer, measured

2026-10-10 (UTC), by milestone 812 (`std::thread::spawn` runs real threads in one address space):
`std` programs now have threads, and the `std` heap is one spinlock. Allocation is 2.1 to 3.2 times
slower with four threads than with one. That is 200,000 small allocations and frees, split four
ways, under QEMU's TCG with four emulated cores, on all three architectures. The program is
`std_exerciser/src/bin/std_heap_contention.rs`, run by
`std_threads_tests::the_std_heap_is_measured_under_four_threads`, from commit `26ee75bae` (pull
request #1892). The figures per architecture are in `notes/std/threads.md`. This is the first
consumer this block was waiting for, and the same program is the measure for whatever replaces the
lock.

## Index row

Milestone 557 (a thread reads its own CPU from a page)'s current-CPU page was justified by a consumer that does not exist, so the mechanism has no measured benefit and the numbers quoted for it are Linux's, about Linux. This builds the consumer: a userspace small-object allocator with a free list per CPU id that reads its own CPU on every allocation. The interesting half is the fallback, because the value can be stale the instruction after it is read and this tree deliberately has no `rseq`, so what correctness argument replaces restartable sequences is the question the milestone answers.
