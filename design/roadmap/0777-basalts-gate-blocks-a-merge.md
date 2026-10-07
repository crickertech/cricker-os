---
status: NOT-STARTED
promoted_from: basalts-gate-blocks-a-merge
raised: 2026-10-04
milestone_dependencies: 755
decision_dependencies: 247
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 777. basalt's gate blocks a merge, and a green pin bump merges itself

Raised by the lane for milestone 755 (basalt v0 pins nife and runs its gate), from what v0 could not
do from a lane. All names here are provisional.

## What is owed

basalt v0 has a gate that reports and nothing that obeys it. `nifeos/basalt` has no ruleset, no
required check, auto-merge off and no merge queue. A red pin can be merged by hand, and a person
merges the daily pin bump's pull request.

The pin bump has its identity: calef installed `nife-smelter` on basalt and set its two secrets on
2026-10-04. The bump opens its pull request with an installation token, and the gate starts on
it by itself. What remains is making green mean merged and red mean blocked.

Two changes, the first a person's (repository settings need an admin):

1. A ruleset on `main` requires the gate's check, and auto-merge is allowed.
2. The bump arms `gh pr merge --auto` on its pull request, as nife's toolchain bump does, so a
   green bump merges. A red one waits for a person.

## Risk and cost

Measured on 755's runs: a warm gate run is about 11 minutes on one arm64 runner, free on a public
repository. A required check that never reports blocks a pull request. The gate must therefore run on
every pull request (it does today, with no path filter).

## Done when

A pin to a commit that fails nife's system test cannot be merged into basalt's `main`, and a green
pin bump merges itself without a person.

## Index row

basalt's gate reports and nothing obeys it. The repository has no ruleset, no required check and no auto-merge. A ruleset requiring the gate's check, plus an armed auto-merge on the daily pin bump, makes green mean merged and red mean blocked.
