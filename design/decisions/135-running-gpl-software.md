---
status: AMENDED
raised: 2026-08-30
decided: 2026-08-30
ratified_by: calef
---

# 135. Running GPL software is aggregation, the capability boundary is what makes it so, and packages are how it arrives

*Amended 2026-09-27T15:11Z: the project itself will serve GPL packages. See the "Amendment" at the
bottom.*

calef, 2026-08-30, ratifying the principle he raised himself and the two
amendments below. Raised the same day, from the question of whether to delete the SMB
implementation: *"Isn't delivering GPL software on nife as stand alone programs a worth
demonstration in itself? There is a lot of GPL software we would want to run."* And then the
sharpening that changed the answer: *"does GPL and LGPL need to be shipped in an image as
aggregation? Can't it be installed through our to be delivered package manager?"* *(Section number
provisional until the merge queue lands it.)*

<!-- writing-standards: exception. Marked 2026-09-27 (UTC) by the maintainer recording calef's
     GPL-packages amendment, below. Reason: this edit only appends an amendment section; the
     document's pre-existing bold density (38 spans, from the 2026-08-30 ruling) is unrelated
     content, and bringing it under the density limit of §213 (writing standards: three countable
     rules, one review rule, and a ratchet) is a separate rewrite that would obscure the amendment
     inside a cosmetic change. Remove this marker when that rewrite lands. -->

## What is being decided

Whether nife may run GPL and LGPL programs, what that does to nife's own licence, **through which
channel such programs reach a machine**, and whether the ability to do it is a claim worth making
rather than a compliance detail.

## Why now, and it is not the SMB question

The SMB question is answered on other grounds (Samba is unrunnable here for POSIX reasons, not
licence ones; see the refusal at the bottom). What forces this section is that **the roadmap already
plans to run GPL programs and nobody noticed.** Milestone 99 / milestone 171 is `git`, GPLv2.
Milestone 170 is `nano`, GPLv3. Those lanes will meet this question with no decision behind them.

A licence posture is in the *move fast on what can be undone* tenet's irreversible category, a fact
that leaves the machine: it is quoted, relied on, and cannot be unpublished. It should be decided
before a lane needs it rather than by a lane that needed it.

## The insight, which is architectural rather than legal

**The same boundary that confines a compromise confines a licence.**

In a monolith, GPL code in the kernel makes the kernel GPL. That is why Linux's ext4, ksmbd, and its
drivers are GPL, and why a permissively licensed kernel cannot take any of them. In a capability
microkernel every service is a separate program communicating over IPC, so a GPL filesystem server, a
GPL network service or a GPL editor is **aggregation rather than derivation**, and the licence stops
at the process boundary the same way a fault does.

That belongs beside the confinement claim rather than in a compliance appendix, and it is the
demonstration calef is pointing at: not *"we can run GPL software"*, which is unremarkable, but
*"somebody else's large, memory-unsafe, ambient-authority-assuming program runs here holding only
what it was granted, and its licence reaches nothing."* `caps <program>` prints the first half. This
section is what makes the second half sayable.

## The pattern this corrects, and every instance of it was right

This tree has routed around the GPL three times and been correct every time:

- Milestone 149 wrote STREAM clean rather than take the one Rust implementation, which is GPL.
- §100 refused Linux's `lib/fonts/font_8x16.c`, GPL-2.0 on line one.
- Milestone 190 refused `lwext4`, whose extents and xattr files are GPLv2.

**Every one of those was a linked dependency. Not one was a separate program.** The tree learned
"avoid GPL" from cases where avoidance was right and has no recorded rule saying where it stops being
right, which is exactly the condition §46 was written to fix: a practice unanimous in effect and
written down nowhere.

**§87 (MIT OR Apache-2.0, and why the GPL's lesson does not transfer) does not cover this.** It
decides nife's own licence and why copyleft's strategy is not ours. It says nothing about running
GPL programs, and nothing in it should be read as refusing them.

## The delivery channel, which is calef's correction and the better answer

The first draft of this section said GPL programs ship in the image as aggregation. **That is legal
and it is not the right default**, because aggregation still makes nife the distributor, with every
obligation that follows: offering source for what we ship, and under GPLv3 supplying installation
information as well.

**A package manager moves the obligation to whoever runs the repository.** If nife does not
distribute the binary, nife has no GPL obligation for it, and the user obtains it from upstream by
their own act.

**The prior art is FreeBSD's, and it is both decisive and current.** FreeBSD spent years moving GPL
software out of the base system, replacing GCC with Clang and finally retiring `dialog` for
`bsddialog`, and **FreeBSD 16 completed the removal of all GPL code from its base system in July
2026**. Users did not lose anything: thousands of GPL applications remain available through ports and
binary packages. That is precisely the split proposed here, from the project whose documentation
standard this tree already follows.

So the rule is a **base and packages split**:

- **The image carries no GPL or LGPL.** Everything nife ships is permissive, which keeps `deny.toml`
  meaningful and keeps a downstream free to vendor the whole image.
- **GPL and LGPL programs arrive as packages**, installed by the user.
- **Milestone 47 already makes that shape natural**: a program namespace is an endowment, so
  installing a program is granting it into a namespace. The licence boundary and the capability
  boundary turn out to be the same boundary, which is worth noticing rather than arranging.

**The package manager does not exist**, which was this section's one open question and is no longer
one. Milestone 198 (a package manager, and the trivial install that makes a second customer
possible) now owns it, and calef dissolved the interim problem the same day rather than answering
it:

> I don't think we expose nife to third parties (aka other customers) until we have a package
> manager and a trivial install process.
>
> -- calef, 2026-08-30

**Nothing is conveyed, so no obligation exists.** The base-and-packages rule is therefore
**forward-binding**: it says what must be true on the day this project first hands somebody a
system, not what must be true today. `git` and `nano` need no interim channel because there is no
interim distribution.

## Two amendments, ratified with the principle

**1. The rule binds on conveyance, not on building.** An earlier draft required that *the image*
carry no copyleft, and that was too strict in the direction that matters: cross-compiling `nano`
into a locally built test image is exactly what milestone 123 (the demonstration: somebody else's
software, running narrow) needs, and it conveys nothing to anyone. The GPL's obligations attach to
conveying. So:

> **No artifact this project conveys carries copyleft until packages exist to carry it. What a build
> produces on a developer's own machine is unconstrained.**

**2. Fetch GPL source, do not vendor it.** `crickertech/nife` is public, so **source is already
conveyed**, which is the permissive case and imposes essentially nothing. But vendoring would put
copyleft in the tree itself, which costs a claim this project otherwise gets to make cleanly and
grows the repository for no gain. A recipe that fetches upstream at build time keeps both. Note that
this is a preference with a reason rather than a prohibition: vendoring GPL source would be legal
and is what some ports trees do.

## GPLv3's replaceability clause, and the mechanism this system already has

The clause that makes GPLv3 painful for embedded vendors is the requirement that users be able to
replace the software on the device. It is why Samba is avoided by appliance makers, and it is the
reason a base-and-packages split matters more here than on a desktop.

**Where nife does ship a GPLv3 program, §41 is the mechanism that satisfies it by construction.**
Live component replacement makes the stable name an endpoint, and a swap is a change in who is parked
on it, with no forwarding process and zero steady-state cost. What GPLv3 asks a device vendor to
permit is a thing this architecture does as a feature. That is worth stating in the same breath,
because it converts a compliance burden into a demonstration.

## What this permits, what it does not change, and what it requires

**Permits.** GPL and LGPL programs as confined nife programs, holding only granted capabilities,
delivered by package.

**Does not change.** Linked dependencies stay permissive-only. `deny.toml`'s allow-list is untouched
and stays an allow-list, §83's prefer-Rust rule still governs crates, and §46 still governs whether a
dependency is taken at all. **A GPL crate in the shipping graph is refused exactly as firmly as
before**, and this section must never be cited to relax that. Milestone 190's `lwext4` refusal stands
for the same reason.

**Requires.**

1. **No conveyed artifact carries copyleft**, per amendment 1, and something should check it rather
   than a person remembering.
2. **Each packaged program's licence is recorded where a reader meets the program**, not in a registry.
3. **A GPLv3 program is shipped only where the replaceability answer is written down**, per above.
4. **The claim is stated with its limits**, the way this tree states benchmark ties: the boundary is
   the process, not the machine, and aggregation is a legal conclusion this project is not qualified
   to give as advice.

## What was refused

**Shipping GPL in a conveyed image as the default.** Legal, and it makes nife the distributor for no
gain once a package manager exists. The first draft kept it as the interim answer; amendment 1
retired the need for one, so it is now simply refused.

**Refusing GPL software outright**, which is where the tree's instinct was drifting. It would forfeit
`git`, `nano`, and most of the corpus milestone 123 (the demonstration: somebody else's software,
running narrow) needs, in exchange for nothing, since the process boundary already provides the
isolation the refusal would be buying.

**Using this to rescue Samba.** Samba's blocker is POSIX, not licence: `fork`, threads and a full
libc, which is tier three plus §105's declined shared-address-space threads. ksmbd is in-kernel and
so is the LKL problem. Licence isolation makes GPL software permissible; it does not make Samba
runnable, and nife's own SMB server remains the permissive one a downstream who cannot take GPL would
use.

## BUGS

- **This is not legal advice and nobody here is qualified to give it.** "Aggregation" is a conclusion
  drawn from how the FSF and every major distribution have treated separate programs for decades, not
  from counsel. Anyone shipping a product on nife should get their own.
- **The package manager does not exist**, so the rule's channel is unavailable today. That is
  milestone 198's gap rather than this section's, and it is why the rule is written to bind forward.
- **Nothing enforces requirement 1**, and amendment 1 makes it harder rather than easier to enforce:
  a gate would have to know which artifacts are *conveyed*, and this project conveys none, so there
  is nothing for a check to look at yet. Rung three describing a rung two that cannot exist until
  198 does.
- **Amendment 1 draws a line that a careless day could cross without noticing.** The difference
  between a local build and a conveyed artifact is a decision somebody makes, not a property a tool
  can see, and the first time an image is handed to anyone is the moment this section starts
  applying in earnest.
- **LGPL is treated as GPL throughout and they differ**, materially, for linking. A future case that
  wants to *link* LGPL rather than run it is not covered here and should not be decided by analogy.
- **The demonstration claim is untested.** No GPL program runs on nife today. Milestone 121
  (`ripgrep` on nife: enumeration as a capability) is MIT/Unlicense, so it is not even a test of
  it, and the first real one is `git` or `nano`.

## Amendment, 2026-09-27: the project will serve GPL packages

calef, 2026-09-27T15:11Z, verbatim: *"There will be a package repo that serves GPL binaries. We
can make it a different org if need be. But we will want to expose GPL packages to our users
through our basalt distribution even if they have to opt in for it."*

**This supersedes this section's premise that the package manager moves the GPL obligation away
from nife.** The project (or a sibling organisation it runs) will distribute GPL binaries, so it
takes on the distributor's obligations for them: corresponding source published beside each
binary, and the licence text shipped in the package. A separate organisation isolates the
obligation; it does not remove it.

**Unchanged.** The image and the `base` kind carry no copyleft. GPL and LGPL arrive only in
`optional` packages, from a repository the user opts into through the basalt distribution
(milestone 120 (nife and the organization)). Opt-in is the rule, not a default-on source.

**Open, owed to milestone 198 (a package manager).** Whether the GPL repository lives under this
organisation or a sibling one, and the mechanism that publishes corresponding source beside each
binary. Both are recorded as open questions in milestone 198's own block.
