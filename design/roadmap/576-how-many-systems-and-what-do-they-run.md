---
status: NOT-STARTED
raised: 2026-09-22
milestone_dependencies: 198
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 576. How many systems are out there, and what do they run

The number is provisional: the integrator mints it at merge. Minted
2026-09-22 at calef's request: *"We want to know system count and packages subscribed to... the
metrics will help us make and maintain a better product if we know what people are using."*

There is nothing to count until something installs, so this
waits on rung 3c (over the internet: a public repository) of milestone 198 (a package manager, and the trivial install that makes a second customer
possible). What leaves a stranger's machine is the irreversible category this tree is most
careful about, so the shape of the report is an architect's.

calef ruled on 2026-10-06 (UTC) that this depends on rung 3c of milestone 198 alone, not on all of it. The
`milestone_dependencies` field names whole milestones and cannot name a rung, so it still reads `198`. That is
stricter than the ruling, and `script/roadmap --ready` will hold this block until 198 is built. The prose is
the record of the real dependency.

## What this is

`basalt` will know which packages a system subscribes to, and the installer will know a system
exists. Neither fact reaches us today. Without them we choose what to build by taste, and the
three principles say to choose by what a customer runs.

## Why this is harder here than it looks, and it is one sentence

A package list is a fingerprint. Counting installs is easy to do anonymously; counting *package
popularity* is the part that de-anonymises, because a set of a few hundred packages identifies a
machine better than an IP address does. Any design that ships "the list this system subscribes to"
has shipped an identifier, whatever it calls the field. So the two things calef asked for pull in
opposite directions and have to be answered separately rather than by one report format.

## DECIDED 2026-09-23: Fedora's model, which is opt-out and carries no identifier

calef, asked to choose between opt-in and opt-out, ruled opt-out, and then on seeing the prior
art: *"Fedora's model seems like the right one."* That second ruling is the larger one, because it
decides the mechanism rather than the posture, and the mechanism answers the questions below.

`countme` works by piggybacking on a request the system was already making. The package client
adds a small bucket to one repository request a week, saying roughly how long since this system first
checked in, and the server counts distinct addresses per bucket per week. No unique value is sent
and none is stored. The bucket is what separates a new install from a long-lived one without
following either. *(The bucket boundaries are described from memory and must be read from Fedora's
own implementation before anything is built.)*

Applied here, the design collapses in a way worth stating plainly: there is no reporter.
`basalt` already fetches packages from a repository, so:

- Package popularity is the fetch traffic, counted per package, independently. No linkage between
  one system's requests, so no list is ever assembled anywhere. This is the unlinkable shape question
  2 asked for, and it arrives for free rather than by engineering.
- System count is the age bucket on those same requests.
- Nothing extra leaves the machine that was not already leaving it to fetch a package.

What this costs, recorded because this block argued the other way an hour earlier. The original
draft made much of a capability-confined reporter being a demonstration of the thesis, since a person
could see and revoke exactly what it may reach. A design with no reporter has nothing to confine. The
smaller version survives (whatever talks to the repository holds one endpoint and a person can see
it), and it is ordinary rather than a showcase. The better design won and the argument for the other
one is left here rather than quietly deleted.

## The prior art, which has already solved half of this

- Fedora's `countme` is the strong one and it counts systems with no identifier at all. Each
  system adds a bucket to an existing repository request saying roughly how many weeks it has been
  since it first checked in, and the server counts distinct addresses per week per bucket. Nothing
  unique is ever sent or stored, and the age bucket is what separates a new install from a long-lived
  one without following either.
- Debian's `popularity-contest` is the honest cautionary half. Opt-in, and it submits a machine's
  package list under a persistent identifier, which is the fingerprint problem accepted openly rather
  than solved.
- Homebrew's analytics is the failure worth not repeating: opt-out, to a third party, and the
  breach of expectation cost more goodwill than the data was worth.

## The idea that makes this a nife milestone rather than a generic feature

The reporter should be a capability-confined program, and that is a demonstration rather than a
precaution. On this system a program holds exactly the authority it was handed. A reporter can be
given one endpoint capability and one read-only view of the subscription list, and a person can
see what it holds and revoke it, which on Linux requires trusting a promise in a manual page.

That inverts the usual telemetry argument. Everywhere else, "it only sends what we say it sends" is
a claim; here it is a property of the object graph, checkable from outside the program. If this OS
is a demonstrator, a confined reporter is one of the better things it could demonstrate, because
telemetry is exactly where users have learned not to believe anyone.

## What calef has to decide, and none of it should be guessed

1. ~~Opt-in or opt-out~~ DECIDED 2026-09-23: opt-out. The reporter runs by default and can
   be turned off. An architect's call, and it is the one this block said every other decision is
   downstream of, so the consequences are written into question 2 rather than left to be
   rediscovered.
2. Whether package subscriptions are reported at all, and **opt-out has already narrowed this to
   two answers.** A system that reports by default may report each package **independently, with no
   linkage between reports from one system**, so popularity is countable and no list is ever
   assembled. Or it may not report subscriptions at all. **What it may not do is ship an assembled
   list**, because a few hundred package names identify a machine better than an address does, and
   sending that without asking is Debian's popularity-contest data model without its consent model.
   The unlinkable shape costs more requests, and under opt-out it is the only shape available that
   still answers the question calef asked.

   This is what makes opt-out defensible rather than a gamble. Fedora's `countme` is on by
   default and nobody objects, because it carries no identifier at all. Homebrew's analytics were
   opt-out *and* carried a persistent identifier *and* went to a third party, and the breach of
   expectation cost more than the data was worth. Opt-out is a position about consent; it is only
   survivable when paired with a position about identity, and this is that position.
3. What else is worth knowing. Architecture and release are cheap and low-risk. A hardware
   profile is neither, and upgrade success or failure is arguably the most useful signal available
   and the most sensitive to get wrong.
4. **Where it lands, and who can read it.** A fact that has left a machine cannot be recalled, so
   retention and access are part of the design rather than operations detail.

## What must be true before any of it ships

- The wire format is `anything two programs agree on`, so it is decided once and carefully.
- The thing that leaves the machine is the irreversible category. §79 (password-equivalent material) is the precedent: an hour of argument was worth it because deleting the code
  afterwards does not unsend anything.
- Opt-out raises the bar on disclosure rather than lowering it. A thing that runs by default has
  to say so where a person installing will actually read it, and the way to turn it off has to be one
  obvious step rather than a documented incantation. That is the whole of what Homebrew got wrong,
  and it was a presentation failure rather than a data one.
- Somebody will have to check what opt-out means legally wherever this is installed, which is not
  a question this tree can answer by reasoning about capabilities. Naming it here so it is a known
  open item rather than a surprise.
- **A `BUGS` section states plainly what the reporter can and cannot see**, where a person meets it.
  A telemetry feature whose limits are documented anywhere other than beside the feature has
  learned nothing from why people distrust telemetry.

## Index row

How many systems run this, and which packages they subscribe to, learned without learning who they
are. Counting installs is solved prior art (Fedora's `countme` uses no identifier at all); counting
package popularity is the hard half, because a subscription list is a fingerprint. The reporter is
capability-confined, so what it may send is a checkable property rather than a promise.
