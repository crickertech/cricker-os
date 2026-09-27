---
status: NOT-STARTED
raised: 2026-09-14
milestone_dependencies: none
decision_dependencies: 184
machine_requirements: none
specific_machine: none
needs_person: no
---
# 403. An x86_64 host in the host pass

Filed 2026-09-14 as an unnumbered proposal, left open by milestone 288,
which fixed four instances of "a test that states a fact about the author's laptop" in `crates/elf`
and could not stop a fifth; numbered 2026-09-19 by milestone 433's drain of the proposal pile.
**Premise re-read against the tree on 2026-09-19 and still true, with one clarification worth
having**: every `runs-on:` in `.github/workflows/` is `ubuntu-24.04-arm` except one, and that one is
`verify.yml`'s `prove-kernel-x86_64`, which runs `script/verify --only kernel` because CBMC needs a
goto-binary for the host it runs on. So **no host test pass has ever run on x86_64**, which is this
block's claim, and the x86_64 runner image is already in use for a different job, which prices
option 1 and option 2 a little lower than the proposal assumed.
*(Number provisional until the merge queue lands it.)*

The decision is
§184 (host test pass), written up
2026-09-19 by milestone 435's slice-c lane because this gate named no section. That section refuses
option 1 on a measurement rather than restating the three: the x86_64 runner image is already in use
by `verify.yml`, so option 1 and option 2 cost the same slot and option 2 buys a symmetric claim for
it, which leaves the real question as 2 against 3.
Whether to spend runner minutes on a second host is an architect's, because it is a
standing cost on every pull request rather than a one-off. Everything below is the measurement he
would otherwise have to ask for; the shapes are priced, so the answer is one sentence and the lane
that follows it needs no further ruling.

## What is not covered

Every machine that has ever run this suite is aarch64: the development machine is Apple Silicon and
CI is `ubuntu-24.04-arm`. Milestone 288 fixed the class where it could be made unrepresentable
(`FOREIGN_MACHINES` is derived, a never-a-nife-machine number is checked in a `const`), but the
residue is plain: **`b.e_machine = 183` still compiles on every host**, and so does any other literal
standing in for a host-relative fact. Nothing in this repository would notice.

The tree has now paid for this three times, in three different crates, each found by a stranger
rather than by a gate:

- `xtask`'s host pass stopped compiling on x86_64 when three crates took `user_rt` dependencies, and
  *"nobody noticed, because CI moved to `ubuntu-24.04-arm` the same day"*. Found 2026-08-14 by
  milestone 117's first run on a clean x86_64 checkout.
- `crates/elf` failed 20 of 25 host tests on x86_64 from at least milestone 161 until milestone 288,
  two milestones having filed it as a proposal before one closed it.
- `fuzz/seeds/elf_parse/` seeded an empty corpus on x86_64, silently, because the note recording the
  host dependence was written before x86_64 was a target.

`script/stranger-test` is the mechanism already aimed at this class and it is the right place to
start reading; whether it can reach a second host cheaply is the open question rather than an
assumed no.

## Three shapes, and none of them is obviously right

1. **A second CI job on `ubuntu-24.04` (x86_64) running only the host pass.** Smallest thing that
   closes the class. Costs one runner slot per pull request, competing with group builds for the
   same concurrency, which `AGENTS.md` already names as a real ceiling.
2. **A matrix leg**, host pass on both architectures. Same cost, tidier shape, and it makes the
   claim symmetric rather than making x86_64 a special case bolted on.
3. **A periodic run rather than a per-pull-request one**, which is `script/stranger-test`'s existing
   posture. Cheapest, and it finds the fault a day late rather than at the pull request. Given that
   the three instances above went undetected for weeks, a day late would have been an enormous
   improvement over what actually happened.

**What is deliberately not proposed**: a lint that greps for machine literals. `git grep -w TODO`'s
82% false-positive rate is the tree's own worked example of why, and a number is exactly the kind of
token a grep cannot tell apart from its legitimate uses (`crates/elf` must name 183, 243 and 62
somewhere, and does).

## What it costs to answer

Option 3 is probably minutes of work and the other two are a CI file edit each. **The expensive part
is none of those**: it is whether the project wants to pay runner concurrency on every pull request
for a class of fault it has met three times in two months. That is a judgement about a standing
cost, which is why this is a proposal with the numbers attached rather than a lane that picked one.

## Index row

Every machine that has ever run this suite is aarch64: the development machine is Apple Silicon and
CI is `ubuntu-24.04-arm`. Milestone 288 made the class unrepresentable where it could
(`FOREIGN_MACHINES` is derived, a never-a-nife-machine number is checked in a `const`) and the
residue is plain, because a machine literal standing in for a host-relative fact still compiles on
every host and nothing here would notice. The tree has paid for this three times in three crates,
each found by a stranger rather than by a gate: `xtask`'s host pass stopped compiling on x86_64 and
nobody noticed because CI moved to an arm runner the same day; `crates/elf` failed 20 of 25 host
tests on x86_64 from milestone 161 to milestone 288, with two milestones filing it as a proposal
before one closed it; and `fuzz/seeds/elf_parse/` seeded an empty corpus on x86_64 silently. Three
shapes are priced and none is obviously right: a second CI job on `ubuntu-24.04` running only the
host pass, which is the smallest thing that closes the class and costs a runner slot per pull
request against a concurrency ceiling AGENTS.md already names; a matrix leg, same cost and a
symmetric claim rather than a special case; or a periodic run, which is `script/stranger-test`'s
existing posture and finds the fault a day late, which given three faults that went undetected for
weeks would still have been an enormous improvement. A lint that greps for machine literals is
deliberately not proposed. The expensive part is none of the edits: it is whether the project wants
to pay runner concurrency on every pull request for this class, which is a standing cost and
therefore calef's.
