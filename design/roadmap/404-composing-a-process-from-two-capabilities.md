---
status: NOT-STARTED
raised: 2026-09-14
milestone_dependencies: none
decision_dependencies: 185
machine_requirements: none
specific_machine: none
needs_person: no
---
# 404. Composing a process from two capabilities is proved for two verbs and no more

Filed 2026-09-14 as an unnumbered proposal by milestone 295's lane, while
performing calef's ruling to retire `components/src/builder.rs`; numbered 2026-09-19 by milestone
433's drain of the proposal pile. Premise re-read against the tree on 2026-09-19 and still true:
`fixtures/src/address_space_witness.rs` still holds exactly two capabilities and still stops where
milestone 19b stopped, `kernel::user::tests::a_process_can_build_start_and_run_a_child_thread` still
drives the whole sequence from the kernel side, and nothing joins the two. Two corrections that do
not touch the argument: the fixture was `address_space_builder.rs` when this was written and was
renamed on 2026-09-18, and `crates/supervision_proto` has been `crates/supervision_protocol` since
milestone 265 (proto). The gate token was `DESIGN`, which is not in the roadmap's gate vocabulary; it is
`DECISION`, which is what it meant, and the prose below is unchanged.
*(Number provisional until the merge queue lands it.)*

The decision is
§185 (what carries the claim that userspace composes a process from an authority you can count on one hand),
written up 2026-09-19 by milestone 435's slice-c lane because this gate named no section. (This
block's status paragraph already records one token correction, from `DESIGN`, which is not in the
roadmap's gate vocabulary; the section is the other half of that repair, since a token in the
vocabulary that points at nothing is the defect milestone 435 exists to sweep.)
What the replacement should *be* is the open question, and the options differ in
cost by an order of magnitude. Nothing is blocked on it: the tree is no worse off than it was the
hour before `builder` was deleted, because nothing on a pull request ever ran `builder` either.

## The claim, split where it actually breaks

`builder` carried two claims wearing one sentence.

Userspace, not the kernel, composes a process. Carried by the progenitor, on every architecture
that runs one, on the boot a card performs. Better off than it was.

...from an authority you can count on one hand. `builder` held exactly two capabilities, a
memory region in slot 0 and a report line in slot 1. The progenitor does not: it is granted the
NS16550 and the UART's interrupt line too, because it is building a system rather than demonstrating
a floor. This is the half that lost its only carrier.

## What is proved, so the gap is the real size and not a bigger one

`fixtures/src/address_space_witness.rs` holds exactly the same two capabilities and, from them,
retypes an address space, retypes a page frame, maps the frame into the space it built, and proves
the kernel enforces break-before-make inside that space. `kernel::user::tests::
a_process_can_build_an_address_space_from_el0` asserts the verdict `0b111` on both architectures
whose test kernel can load a user ELF, under `script/test`, on every pull request.

That is more coverage than `builder` ever had, and it is worth saying out loud because it is the
reason this is a proposal rather than an alarm: `script/test`'s riscv64 leg, `script/cpu-matrix`,
`script/swish-check`, `script/bench --riscv --check` and `script/icount` all park before the tour, so
no pull-request check has ever executed `builder`
(milestone 406, `design/roadmap/406-nothing-in-ci-boots-the-riscv-tour.md`).

## What is proved nowhere

`address_space_witness` stops where milestone 19b stopped. Nothing runs in the space it builds,
because threads were 19c's object. The rest of `builder`'s body is unasserted from a two-capability
floor by anything in this tree:

- read an ELF out of an archive by name, in userspace,
- parse it and lay its segments down into the space,
- retype a thread control block from the same budget,
- endow it (a narrowed view of the one report endpoint the composer holds),
- configure it at the ELF's entry and start it,
- and receive the child's word.

The verbs themselves are proved, from the other side of the boundary.
`kernel::user::tests::a_process_can_build_start_and_run_a_child_thread` drives exactly that sequence
and the child runs and reports, on both architectures. It is a kernel-side test: it calls
`memory_region::create`, `user_address_space_map`, `configure` and `start` directly, not through
`ecall`/`svc` out of a granted budget. So the sequence works; nothing witnesses a *program* driving
it from a fixed endowment.

`fixtures/src/os_primitives_benchmarker.rs` starts a child from userspace and holds more than two
capabilities; it is a benchmark. `crates/supervision_protocol`'s `build_child` is the one loader they
all share, and every caller of it is endowed for its job rather than trimmed to a floor.

So the gap is a join, not a hole. Two verbs from userspace at a two-capability floor
(`address_space_witness`), and the whole sequence from the kernel
(`a_process_can_build_start_and_run_a_child_thread`). `builder` was the only thing that was both, and
saying it that precisely is what makes option (a) below look as small as it is.

## The options

**(a) Extend `address_space_witness` to run something in the space it builds.** The smallest change
that reaches the whole sequence: same two capabilities, same fixture, more verdict bits. It becomes a
host-unrunnable QEMU test like the one it already is, on both architectures, asserted on every pull
request, which is where `builder` never was. The cost is that the fixture's name stops describing it
and names are an architect's, and that the clean "nothing can run in the built space" reading of
milestone 19b (run a real workload) is gone.

**(b) A new fixture that composes a child from two capabilities and nothing else**, leaving 19b's
alone. Honest about being a second thing; costs one more program in every archive, and this tree has
291 milestones' worth of reasons to be careful about that (`design/roadmap/206-user-image-ceiling.md`).

**(c) Prove it host-side instead.** `crates/grant_plan` reasons about what a *shell* grants; nothing
here reasons about what a *composer* needs to hold. A crate that took the verb sequence and the
capability set and decided whether the set suffices would be Kani-reachable and would run in
milliseconds. It is the most work and the only option that produces a *proof* rather than a
demonstration, and it does not witness the kernel actually permitting the sequence, which is the
thing `builder` witnessed.

**(d) Decide the claim is not worth a carrier.** Legitimate and should be on the list. The argument:
the capability model's floor is enforced by the kernel on every call, the 19b test proves the kernel
enforces it inside a space a process built, and a demonstration is not evidence a model checker would
accept anyway. The cost: a demonstrator loses the one boot step a stranger could read and immediately
understand, which is what `builder` was for.

## What it costs to decide

Little. (a) is an afternoon, (b) is an afternoon plus a name, (c) is a week, (d) is a sentence. The
recommendation is **(a)**, and the reason it is not simply done is the one thing in it that is
calef's: the fixture would need a name that still describes it, and renaming is a naming decision
with extra steps.

## What is blocked until it is answered

Nothing. Recorded so that the risk milestone 295 accepted stays visible instead of becoming the kind
of fact that lives only in a merged pull request body, which is the failure
`notes/untracked-work-sweep.md` exists to name.

## Index row

`builder` carried two claims wearing one sentence, and retiring it kept one of them. That userspace
rather than the kernel composes a process is carried by the progenitor, on every architecture that
runs one, on the boot a card performs, and is better off than it was. That it does so *from an
authority you can count on one hand* lost its only carrier, because `builder` held exactly two
capabilities where the progenitor is also granted the NS16550 and the UART's interrupt line.
`fixtures/src/address_space_witness.rs` holds the same two capabilities and proves more than
`builder` ever did, on every pull request rather than on a boot no check performs, but it stops
where milestone 19b stopped: nothing runs in the space it builds. The remaining verbs are all
proved, from the kernel side, by `a_process_can_build_start_and_run_a_child_thread`, which calls the
kernel entry points directly rather than through a granted budget. So the gap is a join rather than
a hole: two verbs from userspace at a two-capability floor, and the whole sequence from the kernel,
with `builder` the only thing that was ever both. Four options, priced: extend the witness to run
something in the space it builds (an afternoon, the recommendation, and the one thing in it that is
calef's is that the fixture's name would stop describing it); a second fixture that leaves 19b's
reading alone, at the cost of another program in every archive; a host-side crate that decides
whether a capability set suffices for a verb sequence, which is the only option producing a proof
and does not witness the kernel permitting anything; or deciding the claim is not worth a carrier,
which is legitimate and costs a demonstrator the one boot step a stranger could read.
