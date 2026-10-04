---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: 755
decision_dependencies: 247
machine_requirements: none
specific_machine: none
needs_person: yes
---
# basalt's gate blocks a merge, and a green pin bump merges itself

Raised by the lane for milestone 755 (basalt v0 pins nife and runs its gate), from what v0 could not
do from a lane. All names here are provisional.

## What is owed

basalt v0 has a gate that reports and nothing that obeys it. `nifeos/basalt` has no ruleset, no
required check, auto-merge off and no merge queue, so a red pin can be merged by hand, and the
daily pin bump's pull request is merged by a person.

The pin bump has its identity: calef installed `nife-smelter` on basalt and set its two secrets on
2026-10-04, so the bump opens its pull request with an installation token and the gate starts on
it by itself. What remains is making green mean merged and red mean blocked.

Two changes, the first a person's (repository settings need an admin):

1. A ruleset on `main` requiring the gate's check, and auto-merge allowed.
2. The bump arms `gh pr merge --auto` on its pull request, as nife's toolchain bump does, so a
   green bump merges and a red one waits for a person.

## Risk and cost

Measured on 755's runs: a warm gate run is about 11 minutes on one arm64 runner, free on a public
repository. A required check that never reports blocks a pull request, so the gate must run on
every pull request (it does today, with no path filter).

## Done when

A pin to a commit that fails nife's system test cannot be merged into basalt's `main`, and a green
pin bump merges itself without a person.
