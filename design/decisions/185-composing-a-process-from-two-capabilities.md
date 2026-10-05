---
status: DECIDED
raised: 2026-09-19
decided: 2026-10-05
ratified_by: calef
---

# 185. What carries the claim that userspace composes a process from an authority you can count on one hand

Raised 2026-09-19 by milestone 435 (forty-five milestones are gated on a decision nobody wrote down)'s slice-c lane, which found milestone 404 (composing a process from two capabilities is proved for two verbs and no more)'s
`DECISION` gate naming no section. The gap was opened deliberately by milestone 295, performing
calef's ruling to retire `components/src/builder.rs`, and that lane recorded it rather than closing
it. *(Section number provisional until the merge queue lands it.)*

## Ruling

Decided 2026-10-05 (UTC) by calef: **option (a), and the fixture is renamed
`process_composition_witness`.** His words:

> I like the UNIX convention of a program doing one thing well. So adding on to an existing program
> rubs me the wrong way. But process_composition_witness seems reasonable. The one thing it does
> well is test.

The maintainer reads that as (a) plus a rename, and the reading is recorded here so it can be
corrected. The extended fixture's one job is witnessing that a process can be composed from two
capabilities. Building an address space is a step of that job, not a second one, so the program
still does one thing. The old name described a step; the new one describes the job.

- Chosen, (a). Extend `fixtures/src/address_space_witness.rs` to start and run a thread in the
  space it builds, still from the same two capabilities, with its verdict asserted on every pull
  request.
- Name. The fixture becomes `process_composition_witness`, ratified by calef 2026-10-05. The
  rename is ratified and not yet performed: it happens in the build lane, which also rewrites
  the fixture's `Name:` block.
- Not chosen, (b). A second fixture would be a second program in every archive, and the one
  thing it would do is already the extended fixture's.
- Not chosen, (c). A host-side Kani proof is not a replacement, because it does not witness the
  kernel permitting the sequence. It stays a possible later addition beside (a).
- Not chosen, (d). The claim keeps a carrier; a demonstrator should keep a boot step a stranger
  can read.
- Consequence. Milestone 19b (run a real workload)'s "nothing runs in the space it built" reading stops holding once
  this is built. The fixture's header and milestone 19b's account say it today; the build lane
  updates the first and leaves the second as the account it was.

The text below is the proposal as raised, kept as the argument that was ruled on.

## What is being decided

`builder` carried two claims wearing one sentence, and retiring it kept one.

Userspace, not the kernel, composes a process. Carried by the progenitor, on every architecture
that runs one, on the boot a card performs. Better off than it was.

...from an authority you can count on one hand. `builder` held exactly two capabilities, a
memory region in slot 0 and a report line in slot 1. The progenitor does not: it is also granted the
NS16550 and the UART's interrupt line, because it is building a system rather than demonstrating a
floor. This is the half that lost its only carrier, and the decision is what carries it now, or
whether it should be carried at all.

## Is the premise true

Checked 2026-09-19 in this worktree, and it is, on all three legs. *(The two `tests.rs` line
citations were re-pointed 2026-09-26 against `main` at `508333ebe` by the decisions-hygiene lane,
from 3022 and 2706, and again 2026-10-05 to `system_tests/src/user/tests.rs`, where the tests now
live, at 269c1d48c; both still assert what is described.)*

- `fixtures/src/address_space_witness.rs` exists and holds the same two capabilities, retypes an
  address space, retypes a page frame, maps the frame into the space it built, and proves the kernel
  enforces break-before-make inside it. `system_tests/src/user/tests.rs:3264`
  (`a_process_can_build_an_address_space_from_el0`) asserts the verdict on both architectures whose
  test kernel can load a user ELF, under `script/test`, on every pull request.
- `system_tests/src/user/tests.rs:2937` (`a_process_can_build_start_and_run_a_child_thread`) drives the
  whole sequence and the child runs and reports, on both architectures. It is a kernel-side
  test: it calls `memory_region::create`, `user_address_space_map`, `configure` and `start`
  directly, not through `ecall`/`svc` out of a granted budget.
- Nothing joins the two.

So the gap is a join rather than a hole, and saying it that precisely is what makes option (a)
look as small as it is. `address_space_witness` stops where milestone 19b stopped, because threads
were 19c's object; the verbs past that point are all proved, from the other side of the boundary.

And the coverage is better than it was, which is why this is a proposal rather than an alarm: no
pull-request check has ever executed `builder`, because `script/test`'s riscv64 leg,
`script/cpu-matrix`, `script/swish-check`, `script/bench --riscv --check` and `script/icount` all
park before the tour (milestone 406).

## What this tree already does in the analogous case

A claim gets a carrier that runs on every pull request, or it is prose. That is §97's posture
and milestone 406's whole complaint one subsystem over. `builder`'s failure was not that it was
wrong; it was that the only thing exercising it was a boot no check performs, so a step nothing
asserts and a step nothing needs produced identical evidence, which is what cost milestone 289 a
lane.

And a floor is proved by holding it, not by describing it. `crates/grant_plan` reasons about
what a *shell* grants; nothing in this tree reasons about what a *composer* needs to hold.

## The options, priced

| | what | cost |
|---|---|---|
| (a) | extend `address_space_witness` to run something in the space it builds | an afternoon. Same two capabilities, same fixture, more verdict bits, asserted on every pull request, which is where `builder` never was |
| (b) | a new fixture that composes a child from two capabilities and nothing else | an afternoon plus a name. Honest about being a second thing; costs one more program in every archive, which milestone 206 (a program image has under 896 KiB) is the standing reason to be careful about |
| (c) | prove it host-side: a crate that takes the verb sequence and the capability set and decides whether the set suffices | a week. Kani-reachable, runs in milliseconds, and the only option producing a *proof* rather than a demonstration. It does not witness the kernel actually permitting the sequence, which is the thing `builder` witnessed |
| (d) | decide the claim is not worth a carrier | a sentence. The kernel enforces the floor on every call, 19b proves it does so inside a space a process built, and a demonstration is not evidence a model checker would accept |

## Recommendation

(a), and the one thing in it that is not a lane's is why this section exists rather than a
commit: the fixture's name would stop describing it. `address_space_witness` witnesses an
address space; a fixture that also starts a thread in it witnesses something larger, and a rename is
a naming decision with extra steps, which AGENTS.md puts squarely with calef.

The second cost of (a), stated so it is chosen rather than discovered: milestone 19b's clean
"nothing can run in the space it built" reading goes away. That reading is currently a fact a
reader can check in one file.

(d) is legitimate and belongs on the list rather than being allowed to lose by default. Its real
cost is that a demonstrator loses the one boot step a stranger could read and immediately
understand, which is what `builder` was for.

## Would we still choose (a) if all four cost the same

Mostly yes, and the honest exception is (c). If a week were free, (c) produces a proof where (a)
produces a demonstration, and this project's direction (§14) values the first more. (a) wins on
cost *and* on witnessing the kernel rather than a model of it, so the recommendation does not rest
on effort alone, but (c) is the option that a bigger budget would change.

## How reversible, and who has acted on it

High. A fixture and its verdict bits; nothing two programs agree on. The name is the one part
that lands in a reader's head, which is the standard reason it is not a lane's.

## What is blocked until this is answered

Nothing. The tree is no worse off than it was the hour before `builder` was deleted, because
nothing on a pull request ever ran `builder` either. This is recorded so the risk milestone 295
accepted stays visible instead of becoming the kind of fact that lives only in a merged pull request
body, which is what `notes/untracked-work-sweep.md` exists to name.
