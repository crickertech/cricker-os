---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: a-list-strangers-read-to-find-a-kernel
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 559. A list strangers read to find a kernel worth reading

The number is **provisional**: the integrator mints it at merge. Promoted from the proposal `a-list-strangers-read-to-find-a-kernel` on 2026-09-22, filed 2026-09-21. Raised by calef: *"Create a proposal to add nife or basalt to
https://github.com/jubalh/awesome-os."*

Not because the work is hard, which is one line in somebody else's README, but
because it is the category this project treats as irreversible: a fact that leaves the machine. A
listing can be deleted; a stranger's first impression of this tree cannot, and neither can a claim
they quote.

## What the list is, read rather than assumed

`github.com/jubalh/awesome-os`, 2,285 stars, last pushed 2026-09-21, one `README.md` and a
`.github/` holding only `FUNDING.yml`. Its own statement of purpose is the load-bearing fact here:

> The goal is to collect all kinds of different open source OSs so people can study their code and
> learn from them.

Entries sit under `## Open Source Operating Systems`, alphabetically, one line each: a bullet, the
project's name as a markdown link to its home, then a hyphen and one clause of description. The
neighbours are the right ones: Redox, Theseus, Hubris, Genode, Asterinas, Hermit, Maestro,
Charlotte, Fomos.

## The premise to check first, because it looks like a blocker and is not

`AGENTS.md` carries calef's own precondition, 2026-08-30: *"I don't think we expose nife to third
parties (aka other customers) until we have a package manager and a trivial install process."* A
listing looks like exactly that exposure, and a proposal that ignored it would be arguing around the
rule rather than with it.

**It is not the same act, and the list says so itself.** The precondition is about *customers*, people
who would run this system and depend on it, and it exists because we cannot yet install it for them.
This list recruits readers: its stated goal is code to study. Reading is the one thing nife is
ready for today, and it is what the tree has spent ten weeks becoming good at. The notes, the honest
`BUGS` sections, the recorded refusals and the corrections are worth more to a reader than to a user.

So the precondition stands and does not bind here. If the entry implies "run this", it binds
immediately, which is a constraint on the wording rather than on the decision.

## nife or basalt, which is nearly decided by the above

nife. `basalt` is the name milestone 120 (the rename: the OS becomes `nife`, and the project
gets an organization) reserved for the distribution, and it is *an empty repository*. A list
whose purpose is code to study has nothing to point at. When basalt exists and has an install story,
it is the entry that would satisfy the precondition above and could be added beside this one.

## What the entry would say

The list's entries are short and factual, and its voice is not a pitch. A draft, to be argued with:

> `* [nife](https://github.com/nifeos/nife) - Capability microkernel in Rust for aarch64,
>   riscv64 and x86_64, where every driver, filesystem and network stack is a confined userspace
>   process`

**Three things it deliberately does not say.** No line count, because a number in somebody else's
README goes stale and we cannot correct it there. No claim about verification, because 145 Kani
harnesses are a fact about effort and `design/fatal-risks/README.md` risk 2 is honest that they have caught
nothing after the day they were written. And no comparison to seL4, because a comparative claim in a
directory listing is the kind of thing that gets quoted without its caveats, which is precisely what
the *facts that leave the machine* rule exists to prevent.

## What must be true before it is submitted

The list sends a stranger to `README.md` and then to a clone. `AGENTS.md`'s third principle is that a
newcomer must be able to succeed without asking anyone, and *"where the answer is no, that is a bug
in the tree and not in the stranger."* A listing converts that principle from an aspiration into
somebody's actual Tuesday.

1. The stranger test is current. milestone 117 (the stranger test) built the measurement; run it
   again against the tree as it stands and treat a failure as the blocker rather than as a note.
   This is the one item here that could take real work.
2. **A clean clone builds on a machine that is not ours.** `script/setup` is the promise; the failure
   mode is a dependency the dev Macs happen to have.
3. The README's numbers are current. It leads with 694 `unsafe` blocks in 39,892 lines, and those
   move every week.
4. The front door works: `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` and the licence
   are all in place as of 2026-09-21.

## What it costs if it goes wrong

A stranger who clones, fails to build, and leaves, tells nobody. That is the failure this project
cannot measure and should most fear, and it is the reason item 1 is first rather than last.

## BUGS

- There are no stated inclusion criteria and no `CONTRIBUTING.md` in that repository, so
  acceptance is one maintainer's judgement and a refusal would be a public pull request that was
  closed. That is a small, real cost and it should be decided with open eyes rather than discovered.
- Nothing will measure what the listing brings. GitHub's referrer data is coarse and this project
  has no attribution mechanism, so any later claim that the listing produced readers or contributors
  would be unfalsifiable. If that matters, the honest move is to record the date it landed and treat
  anything after it as correlation.
- This proposal assumes the list stays what it is. It is one person's README and its purpose
  could change; the quotation above is from the copy read on 2026-09-21.

## Index row

The first deliberate act of telling strangers this tree exists: a one-line entry in a 2,285-star
directory whose stated purpose is code to study, which is the one thing nife is ready for, with the
wording constrained so it recruits readers rather than implying a system anyone can install.
