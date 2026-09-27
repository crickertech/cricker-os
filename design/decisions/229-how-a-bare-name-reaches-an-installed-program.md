---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 229. How a bare name at the prompt reaches an installed program: the activation set, and never an owner's vouch

Raised 2026-09-26 by the maintainer, from milestone 47 (navigation and naming)'s block, under
"The manifest question was answered elsewhere" in
[design/roadmap/47-navigation-and-naming.md](../roadmap/47-navigation-and-naming.md). The options
table is there. *(Section number provisional until the merge queue lands it.)*

## The ruling

calef, 2026-09-26 (UTC): *"I don't want bare word uptime to run the local one. I want the system
one to override and the local one to need to be called by path. That allows a user to run both."*

That is option B with a sub-rule, recorded here as B2:

- A bare name resolves through the live activation set, one entry per name, with no search order.
- Any word containing `/` is a path, and a path always means that file.
- An owner's `vouch ./x` grants permissions by digest, but never claims or replaces a bare name.
  The bare-name lookup ignores owner-vouch entries, so the system program keeps the name and a
  local build runs by path.
- Both refusals in the recommendation below stand. At install, another package cannot take a name.
  At the prompt, a name that is both an image program and a live entry is refused, naming both
  paths.

### Refused, with reasons

- A (paths only) and C (a bound directory), for the reasons in the options table.
- B1, the owner's vouch takes over the name. calef wants both programs runnable, with the system
  one on the bare word.
- B3, a priority between vouched and packaged entries. A priority is a search order, which is
  what B exists to avoid.

### What the building lane must check

The owner's `vouch` comes from PR #1340, for milestone 198 (a package manager), not yet merged. It writes an ordinary entry, `<name> owner <digest>`, with
`activation_set::OWNER` in the package column. Two things follow, and the lane chooses how to meet
each:

- The name lookup must not match those entries. Either it skips `OWNER` rows, or `vouch` records
  them so that no name lookup can match them.
- On that branch an owner entry is "replaced by a later vouch or install of the same program
  name" (the doc on `activation_set::OWNER`), because `with_entry` keeps one entry per name. So
  `vouch ./uptime` would today displace the packaged `uptime`, not merely sit beside it. B2 needs
  a vouch never to replace a package's entry, which the same choice can settle.

The options as they were raised follow, unchanged apart from this section.

## What was decided

What `uptime` at the prompt means when `uptime` is an installed package's program rather than one
compiled into the image. This is what is left of `PATH` here. §208 (installing is granting) made the
activation set the record of what is installed, and §219 (how the shell names an installed program
to the spawner) ruled that the shell sends a program's bytes, not its name. So no name crosses a
wire, and the question is only how the shell turns a word into a file.

## The premise, checked 2026-09-26

An installed program runs by path today (`packages/greeting/0.1.0/greeting` in
`xtask/src/swish_check.rs`), and its bare name is refused as "no such program"
(`notes/packages/fetching.md`). The activation set already maps names to packages:
`activation_set::Entry::program` is documented as "the name a person types", and
`activation_set::lookup(table, program)` exists, with no caller at the prompt. The shell already reads
the live generation to vouch for bytes by digest (`live_generation_listing` in
`components/src/swish.rs`).

One fact the block did not state. Within a generation, a name has one entry by construction:
`activation_set::with_entry` replaces an entry of the same program name. That is right for an
upgrade, but it also means a second package that ships a program of the same name silently
replaces the first package's program today.

## The options

| | A bare name not in the image | Cost | Reversibility |
|---|---|---|---|
| A. Paths only (today) | Refused | Zero; `bind` already shortens a path | Nothing to undo |
| B. The live activation set, ambiguity refused | Resolves through `activation_set::lookup` in the live generation to `packages/<name>/<version>/<program>`, then runs as a path does | One lookup the shell has the code to make. No search order, because each name has one entry | Cheap now. Once scripts name installed programs bare, removing it breaks them |
| C. A bound directory (Plan 9's `/bin`) | Tried as `<bound name>/<name>` after the image | A convention for which bind is searched, and with two sources, an order | An order is what milestone 47's `PATH` section says `PATH` gets wrong |

## Recommendation: B, with two refusals made explicit

B, because it is the program namespace as the endowment with no search at all: installing is what
puts a name in it (§208), and a name has one meaning. Two refusals make "no search order" true
rather than a hope:

- At install, a program whose name equals an image program's is refused, and so is one whose name
  another package already provides. The second is new: `with_entry` replaces on a matching name
  today, so the refusal has to distinguish "the same package, a newer version" (an upgrade, kept)
  from "a different package" (refused).
- At the prompt, a name that is both an image program and a live entry is refused, naming both
  paths. The install check cannot cover this alone, because a later image can add a program whose
  name a package already holds.

The recommendation is allowed because the fork is reversible now. No wire changes (§219 already
carries bytes), the change is the shell's own lookup, and nobody has acted on it: no script and no
gate names an installed program bare. It becomes expensive only once scripts do, which is the
reason to answer it before `package install` has users.

## The seven questions

1. What else was considered. A and C, above, each with its reason.
2. What this tree does in the analogous case. An image program is found by its one entry in
   `grant_plan::Prog`, with no search. B gives installed programs the same property from the table
   §208 already made the record.
3. Prior art, read 2026-09-26. Nix's `nix-env --install` documents that two packages clashing on a
   file name make building the new profile generation fail, with `meta.priority` as the override.
   B takes the failure and refuses the priority, since a priority is a search order under another
   name. From memory, not re-read: Plan 9 builds `/bin` as a union of bound directories, searched in
   bind order, which is C.
4. Is the premise true. Yes, with the correction about `with_entry` above.
5. Cost. Not measured, since nothing is built. By inspection: a name lookup beside the digest lookup
   the shell already makes, and a package-name comparison in `with_entry`'s caller.
6. Reversibility. Reversible today, as above.
7. Equal cost. Yes. C costs more and is refused for its order; A costs less, so B is not chosen for
   effort.

## What this unblocks

Bare names for installed programs, buildable in milestone 47 (navigation and naming). With this
ruling none of the milestone's three open items waits on an architect.
