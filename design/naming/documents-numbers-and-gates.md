# Documents, numbers, branches and the naming gates

*An appendix to [`design/naming.md`](../naming.md), which is the rule. This file holds where a
document goes, why `§N` is not milestone N, why a numbering gap passes, the branch convention, and
what `script/lint` checks. It exists to verify or challenge the main page, and a reader who only
needs to name, ratify or rename something should not have to open it. The directory `design/naming/`
and this file's stem are provisional names, minted 2026-09-24 by the lane that split the file;
naming is an architect's.*

## Where a document goes

Four places, and the distinction is what the document is *for*, not what it is about.

| | holds | shape |
|---|---|---|
| `design/` | the option space, before a decision | "here are four answers and three are bad" |
| `design/decisions/` | the decision, and the argument that settled it | numbered `§N`, append-only |
| `design/tenets/` | why a rule in `AGENTS.md` holds | one file per theme, linked from the rule |
| `notes/` | what exists, and what building it taught us | a running glossary, indexed in `notes/README.md` |

`design/tenets/` was ratified by calef on 2026-09-24 (UTC), directory and filenames, when
`AGENTS.md` was split into rules plus appendices. The reason that survived scrutiny is that a tenet
gets cited on its own rather than only as `AGENTS.md`'s footnote. That makes it an argument about
how to work, and puts it beside the option space it reasons over.

`notes/` was refused: a note records what exists and what building it taught us, and a tenet records
neither. The first argument offered for `design/`, that thirteen rows would crowd `notes/README.md`,
was refused with it: `script/lint` reads `notes/*.md` without recursing, so a `notes/tenets/`
subdirectory would have cost that index nothing. The directory's own README carries the block, per
§75 (carry provenance in their own README).

`design/roadmap/` is the exception that proves the split. It lives in `design/` because a milestone
block is an argument for doing something, not a record of having done it. That stays true after the
milestone ships and the block gains a "Built" line.

A note is not optional. Every concept and every finding gets one, indexed in
[notes/README.md](../../notes/README.md). For a demonstration OS the documentation is part of the
deliverable rather than a courtesy to the author.

## `§N` is not milestone N, and they collide

DECISIONS section numbers and roadmap milestone numbers are separate schemes over the same small
integers. There are 41 sections and 39 milestone blocks, so almost every number means two things:

| N | DECISIONS `§N` | roadmap milestone N |
|---|---|---|
| 24 | the two-tier Ctrl-C | a Virtualization.framework board |
| 28 | SMP placement | the line discipline |
| 31 | the foreign-language C seam | the capability shell |
| 39 | this naming rule | components, services, and the directory layout |

*Corrected 2026-09-24: the "41 sections and 39 milestone blocks" count is from when this was
written. The tree now holds about 213 decision files and 575 roadmap blocks, so the overlap covers
every number up to 213.*

This has already produced a wrong citation in the tree. The block of milestone 50 (pipes and
redirection) cited "§31's FileSpec", which points at §31 (the foreign-language seam), the C seam,
and has nothing to do with file grants. The thing it meant was milestone 31 (a capability shell)
phase 2, granting against the §27 (the filesystem service) filesystem contract. Fixed in c0643bc.

The rules for writing either number are on the [main page](../naming.md).

`script/decisions --check` verifies that every cited `§N` resolves to *some* section. It cannot
verify that it resolves to the *right* one: a well-formed wrong citation is indistinguishable from a
correct one from the outside. `script/citations`, from milestone 444 (a citation says what it
cites), now asks a citation to carry a name, which closes that gap where it is used.

### Both schemes are sparse: a duplicate is the defect, a gap is not (milestone 443)

The rule came from milestone 443 (lanes wait on each other).

Two files claiming one number is fatal in both schemes. Every citation to that number is then
ambiguous, and no gate can tell which one a sentence meant. `script/decisions --check` and
`script/roadmap --check` each fail on it twice over, once for two index rows and once for two files.

A hole in the numbers is reported and passes. `§157` with nothing behind it misleads nobody.
(Corrected 2026-09-24: that number has since been minted, as §157 (a trivial install), so it is no
longer a hole.) The number appears in no citation, every citation that does exist still resolves,
and the sequence was never a promise that it is dense. The roadmap has always worked this way, and
441 and 442 are unused today. `script/decisions` failed on a gap until 2026-09-19 and no longer
does.

The gate was buying a cosmetic property with the tree's most dangerous edit. A gap is closed by
renumbering, and
[§194 (sessions interleave rather than serialize)](../decisions/194-sessions-interleave-rather-than-serialize.md)
records what renumbering costs. A citation rewritten by number can be silently wrong and still pass
every gate, because the section it now names exists. On 2026-09-19 one branch was renumbered four
times in two and a half hours, from §156-§189 to §160-§194. Another session was minting from the
same range, and a contiguous scheme made every collision displace the whole run. The range began at
§156 (what the package manager waits on); §160 (what a subshell copies) and §189 (which of two
definitions `caretaker` carries) sit inside it.

So when two sessions collide on a number, the later lander takes the next free ones and leaves the
hole. Only the colliding sections move, not the run behind them: four files rather than thirty-four.
The rule above it is unchanged and is `AGENTS.md`'s. A number is provisional until the merge queue
lands it, and anything global to the tree is the integrator's at merge.

What no longer has a gate: a decision file deleted outright, with its index row deleted in the same
commit and no citation to it anywhere in the tree. That now leaves nothing behind to notice. Every
partial shape still fails: a file with no row, a row with no file, a row pointing at a missing file,
and a `§N` in the tree that resolves to nothing.

## Branches

Eight prefixes were in use when this was written, including both `feature/` and `feat/` for the same
idea. One spelling, and `feature/` is the older one:

`milestone/` (a roadmap milestone), `fix/` (a bug with a name), `bench/` (measurement work),
`audit/` (reading rather than writing), `integration/` (joining lanes), `finalize/` (landing them).
Plus `main`, and the tooling's own `worktree-agent-*`, which no person types.

*Corrected 2026-09-24: `maintainer/` is the commonest prefix on the remote today (20 of 31 branches
on `git ls-remote --heads`) and is not in the list above. `roadmap/` is also in use. The list
describes the convention when it was written, and the prefix is unbounded by design.*

This is a convention and not a gate, since 2026-08-18. It was an enforced allowlist, and calef asked
what the taxonomy was for. The answer, checked rather than argued: nothing consumes it except the
check itself. A grep across `script/`, `helpers/`, `.github/workflows/` and `xtask/` for any other
reader of a branch prefix returns only false positives. Only `milestone/N-` is read by anything, and
`script/lint`'s milestone-branch-touches-its-block check is what reads it.

Every observed failure of the allowlist was a false rejection of legitimate work, four times:

- `roadmap/`, the repository's *second* commonest prefix, refused while about thirty-five merges
  using it were already on `main`.
- `gh-readonly-queue/*`, which failed every group build's clippy job, so the queue evicted and
  rebuilt forever.
- `dependabot/*`.
- `claude/*`: a harness names its own branches and a lane cannot rename them from inside.

Each was fixed by widening the list after something broke. That is the signature §61 (a lint is
adopted on evidence) used to drop three lints and milestone 78 (the load-sensitive assertions) used
on three assertions: a check that only ever rejects valid work is measuring the wrong thing.

What survives as a gate is the one prefix that carries a mechanism. It survives as a branch name
rather than a label because the check that reads it runs locally and offline, from `symbolic-ref`,
with no network and no GitHub context. A label cannot be read without an authenticated round trip.
It would not exist yet anyway, because a lane runs `script/lint` before it runs `gh pr create`. A
branch name is also fixed when the lane is cut, where a label is editable at any time. For a check
whose job is "you claimed 126, so move 126's block", the claim has to be the immutable half.

## What is checked, and what cannot be

`script/lint`'s `naming conventions` block enforces seven things, and the first five are cheap greps
because lint runs constantly. (An earlier version of this sentence said four and then listed five,
which is the ordinary way a hand-kept count drifts; take it from the script.)

1. No name ending in `-d`, over `components/src/*.rs` and `fixtures/src/*.rs`, both `Cargo.toml`s'
   `[[bin]]` names, and `crates/*`. Four characters or more, so a three-letter name ending in `d` is
   read as an abbreviation rather than a daemon. (`kbd` was this rule's worked example until its
   2026-08-28 rename.) Words that genuinely end in `d` go in `naming_allow` with a reason, the same
   shape as a per-item `#[allow]`. `uuid` (RFC 9562's own term) is the one there today. `asid` was
   the other until 2026-09-18, and §154 (the acronym test)'s rename removed the entry rather than
   re-argued it. An exemption that exists only because a name is an unreadable acronym is one the
   expansion deletes.
2. The word "daemon" appears nowhere, outside `design/decisions/` and `design/`. Those are where the
   argument about the word lives, and therefore they have to be able to name it.
3. Contract crates spelled `*_proto`.

   *Corrected 2026-09-24: `*_protocol` since milestone 265 (calef, 2026-09-05). The check is the
   same; only the string moved.*
4. The current branch carries a recognised prefix.

   *Corrected 2026-09-24: no longer. Since the allowlist was retired (see Branches), check 4 only
   refuses a near-miss of a milestone claim, such as `milestone-126-pgrep`, which would skip check
   4b in silence. The shape rule lives in `helpers/branch-name-check.sh` (2026-09-22), shared with
   `script/claim`.*
5. No `#[path]` module is shared by two or more binaries (CLAUDE.md rule 7). This is the newest and
   the one with teeth: it counts consumers per include target and fires at two. A module with a
   single consumer is an ordinary submodule and is fine. The rule is about *agreement between
   binaries*, not file layout. The allow-list is empty, which is the intended steady state.

   `virtio` was its one entry for about an hour. It could not be a crate while it reached back into
   whichever binary included it for `check`, and the resolution was to delete `check` rather than
   pass it in. Rust already has the per-binary "how this program dies" hook, `#[panic_handler]`.
   Both binaries already had one executing the same instruction by two different routes. An entry
   here needs a reason of that calibre.
6. `notes/` and `design/` filenames are lowercase and hyphenated, because they are URL slugs the
   moment any of this is published. `README.md` is the only exception, and it is GitHub behaviour
   rather than style.
7. Every crate, program and `script/` entry point carries a `Name:` block (milestone 115 (the names
   that were ratified), and the provenance appendix). Presence only: it cannot check that the reason
   is still true. It checks that the block names one of the three states, that `ratified` carries a
   date and that `recorded` carries a citation. It never checks that the state is `ratified`, so a
   name waiting on an architect does not fail anybody's build. `script/names --unratified` is how
   that queue gets worked.

   *Corrected 2026-09-24: four states, not three. §89 (`provisional` becomes the fourth provenance
   state) added `provisional` on 2026-08-16, and `helpers/name_provenance.py`'s `STATUSES` lists
   `ratified`, `recorded`, `provisional`, `unrecorded`.*

   And exactly one such block per file, in the spelling the parse reads (milestone 283 (one
   provenance block per file)). That was a convention 205 files happened to follow until two did
   not. The two ways of breaking it compound into silence. The parse stops at the first `Name:`
   line, so a stale proposal block above a ratified one is what gets read. The parse matches
   `^<prefix> ?Name:`, so a header wearing markdown (`//! **Name: ratified ...**`) is invisible even
   when it is the only one. Both at once is how two of calef's ratifications sat on the worklist for
   a week. The gate reported `provisional`, which is a legitimate answer nothing disputes.

   So the check asks the question a person asks by looking at the file: does anything here read as
   a provenance header without being the one that was read? A comment line counts if its content,
   after the marker and any leading markdown, begins `Name:`. That holds at `///` and `//` as well
   as the surface's own prefix. The failure names the file, the line, and which of four things is
   wrong with it: markup, indentation, a different comment marker, or a second block nothing reads
   past.

   Two things are deliberately not headers. `` `Name:` `` in backticks is a *mention* of the
   convention, which the scripts implementing it write constantly. A line carrying an angle-bracket
   placeholder (`Name: ratified <YYYY-MM-DD>`) is showing the *form*, which is how `script/names`'
   own header documents the three spellings.

   The parse was not widened to admit bold, deliberately. 205 files use the plain form, so admitting
   a second spelling would make both legal, which is the opposite of the fix.

Everything else here is prose because it needs judgement and no checker can supply it. In
particular a checker cannot catch the jargon half of §39: `linedisc` would have passed all four
rules above. It ends in `c`, contains no daemon, is not a proto crate, and had a perfectly good
branch. What caught it was a person reading the name and not knowing what it meant, and that
remains the test.

*Corrected 2026-09-24: "all four rules above" dates from when there were four; the point holds for
all seven.*

Two limits. The checks read the filesystem for names and use `git grep` for the word, so an
untracked file with "daemon" in it is invisible until it is added. (The conflict-marker check has
the same blind spot.) And check 1 sees the *names* of things rather than the things, so a component
whose name is fine and whose behaviour is a daemon is not its problem.

## How this file became the rule

This record was written at milestone 46 (rename the components for what they are), alongside the
rename that made four of the names honest. It began as a note in `notes/` and was the working
reference for the parts §39 (a component is named for what it is) does not cover: crates, scripts,
where a document goes, and the two numbering schemes.

§155 (the naming conventions move out of the constitution, and the note becomes the rule) made it
the authority on 2026-09-18. It moved to `design/` on the same ruling, because a file that is
normative is not a note. `AGENTS.md` kept only the authority itself, the part a lane must act on
without opening anything. The rules a lane applies moved here verbatim from `AGENTS.md`. Where the
two disagree, this file is the rule for conventions and `AGENTS.md` is the bug. That is the reverse
of what was true before §155.

This overturns milestone 262 (the naming rule is written twice) by finishing what it started. On
2026-09-05 the rule was written out twice, in full, in both files. Milestone 262 shrank that to one
statement per rule in `AGENTS.md`, with the case for it here. The residue was still duplication.
When the acronym test changed on 2026-09-18, it again had to be edited in both places in one commit,
and nothing compares the two. §155 removes the duplication rather than shrinking it. It buys back 58
lines of a constitution that is on a budget (milestone 118 (CLAUDE.md has a budget)).

What makes that safe is the provisional name. A lane that has never read this file invents a name
that is expected to change. So the conventions are needed at ratification and at rename, which are
the unhurried moments, and not at invention.

On 2026-09-24 the page was split into a main page and these appendices under §212 (a prose budget).
The split found two defects in the old main page. The lane rules opened mid-sentence ("new crate,
program or module ships"), having lost their first word when they moved from `AGENTS.md`. And the
`NAME_LEN` constraint in the domain section stopped mid-sentence ("which bounds a"). Both are whole
again on the main page.
