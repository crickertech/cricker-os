# The `job_mix` name, and the marker rule it settled

*An appendix to [the naming record](../naming.md), moved from the crate's Name block by milestone
862 (comments state the constraint as it is now) on 2026-10-09 (UTC). Wording kept except where a
sentence had to split.*

Name: ratified 2026-09-13 (calef, working the unratified worklist). Coined by milestone 168 (a
multi-tasking workload benchmark)'s lane on 2026-09-04. A noun pair naming the thing the crate
crates use, and it is the phrase the source itself uses: AIM7's workfile is a *mix* of *jobs*.

The stem was settled a week before this ruling, while calef ratified something else.
`job_mix_task` was chosen over the maintainer's `mix_task` on 2026-09-05 for a reason the
maintainer had not made: *the family stays greppable as one string*, so `job_mix` finds the crate,
`fixtures/src/job_mix_task.rs` and `script/job-mix`. Three members in three naming domains, each
correct for its own, which is the domain table working rather than a coincidence.

That sentence counted three members and there were four, so the property it rests on was already
false when it was written (found and repaired by milestone 296 (the `initboot` feature was
`shell`)). The fourth is the kernel-side
supervisor of this same workload, and it was spelled `kernel/src/jobmix.rs`, with a `jobmix` Cargo
feature and `jobmix:` console markers. A squish is exactly what a separator-insensitive grep
cannot reach. At `9b68f17e`, `git grep -lie 'job[_-]mix'` returns 31 files and
`git grep -lie jobmix` returns 25, and `kernel/src/user.rs` is in the second set and not the
first. The greppability the ratification was *made for* did not hold, and it did not hold because
of the one member nobody had counted.

The repair is therefore not the hyphen rule being applied to a stray file. It is this ruling being
carried out. `job_mix` is the ratified name of this thing; `jobmix` was a misspelling of it. The
module is `kernel/src/job_mix.rs`, the feature is `job_mix`, and `script/board-image`'s flag is
`--job-mix`.

## Console markers take the command's spelling

The console markers went to `job-mix:`, the command's spelling rather than the crate's, and calef
ratified that on 2026-09-14 (`job-mix:` and `job-mix-census:`). **The rule it settles**: a console
marker takes the spelling of the command a reader typed to produce it, not of the crate that
implements it, because the reader's path to the string runs through the command. That makes
`soak`'s markers a precedent rather than a coincidence, and gives the next workload's markers an
answer before anyone has to ask.

The case as it stood when he ruled: a marker is neither a Rust identifier nor a shell command. It
is a string a person reads on a serial console after typing `script/job-mix`, and the one
recognizer that matches it sits beside `script/board-console`. `kernel/src/soak.rs` set the
precedent by accident rather than by argument, since `soak` is one word and cannot show a seam:
its markers matched `script/soak` exactly without anyone having to decide that they should.

That precedent has since been tested, and it held. calef ruled `script/soak` to `soak-test` the
same day, and the markers moved with the command to `soak-test:` and `soak-test-census:` rather
than staying with the crate or the module, which are still spelled `soak`. The rule stated here
predicted what a rename would do to a marker, and the rename did it. The deciding evidence was
that `xtask`'s own line already read "job-mix: QEMU ended before printing `jobmix: done`", one
sentence in two spellings.

## The refusals

The refusal of `aim7` was righter than this record knew, and the reason inverts the usual
direction. It was refused for claiming somebody else's benchmark. A premise check on 2026-09-13
(`notes/job-mix.md`, and the correction in §96 (process kernel or event kernel)) found the
benchmark is not merely unclaimed but **unreachable in principle**. Warton ran AIM7 on Wombat, the
paravirtualised ARM Linux, so the number this crate was built toward is a delta between two kernel
models measured through a hosted Linux. This tree has one kernel model and no hosted Linux. A name
that had claimed AIM7 would
now be claiming something that cannot be done here at all.

Refused `workload`, too general for a tree that already has a soak workload and a compute
workload. Refused `benchmark`, because the crate is the workload's *definition* and produces no
measurement: the same distinction `os_primitives_benchmarker`'s own header draws between the agent
and the output.
