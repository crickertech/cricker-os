---
status: IN-PROGRESS
branch: milestone/561-a-per-cpu-allocator
raised: 2026-09-21
promoted_from: a-per-cpu-allocator-is-what-the-current-cpu-page-was-for
milestone_dependencies: 812
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

## What the lane found, 2026-10-10 (UTC)

A per-CPU list needs an atomic on every operation without `rseq`, stale id or not, because a thread
preempted mid-pop hands the same core, and the same list, to another thread. A cache per thread
needs none, and after milestone 812 reaching it costs a subset of what reaching a per-CPU list costs
(both start from the thread pointer). So the lane builds a cache per thread in front of the one
locked heap, and measures it against the per-CPU candidate rather than arguing. The current-CPU page
loses the consumer it was built for; it keeps `available_parallelism`'s allowance.
`notes/std/allocation.md` has the argument, the table of candidates and the numbers.

- Built: `crates/allocation_cache` (name provisional), the cache as host-tested pure logic, in
  front of a real `user_mode_heap` under a random walk of allocations, frees, refills and spills.
- Waiting on milestone 812 (`std::thread::spawn` runs real threads in one address space), pull
  request #1892: wiring it into `std`'s allocator through 812's per-thread block, and the measurement
  on three architectures (one lock, per thread, per CPU, per CPU with the CPU read removed).

Reuse: none to take, after surveying `talc`, `linked_list_allocator`, `buddy_system_allocator`,
`dlmalloc`, `rlsf`, `frusa`, `rsbmalloc`, `mimalloc` and `snmalloc` on 2026-10-10 (UTC)
(`notes/std/allocation.md`). The pure-Rust `no_std` ones are one heap behind one lock, which is what
`std` has; the thread-caching ones are C or C++ or need `mmap`, and would replace `user_mode_heap`,
which §31 (the foreign-language seam) rule 4 keeps. The cache adapts glibc's `tcache` design.

## Index row

Milestone 557 (a thread reads its own CPU from a page)'s current-CPU page was justified by a consumer that does not exist, so the mechanism has no measured benefit and the numbers quoted for it are Linux's, about Linux. This builds the consumer: a userspace small-object allocator with a free list per CPU id that reads its own CPU on every allocation. The interesting half is the fallback, because the value can be stale the instruction after it is read and this tree deliberately has no `rseq`, so what correctness argument replaces restartable sequences is the question the milestone answers.
