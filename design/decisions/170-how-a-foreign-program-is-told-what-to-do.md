---
status: DECIDED
raised: 2026-09-19
decided: 2026-09-26
ratified_by: calef
---

# 170. How a foreign program is told what to do

Raised 2026-09-19 by milestone 435 (forty-five milestones are gated on a decision nobody wrote
down)'s lane, which found milestone 205 (how a foreign program is told what to do) gated on
`DECISION` with no decision anywhere a reader can open. The block was minted 2026-08-31 out of
milestone 121's lane. *(Section number provisional until the merge queue lands it.)*

Ruled by calef on 2026-09-26 (UTC), built up over one conversation with the maintainer after the
shim was priced in [notes/foreign-program-arguments.md](../../notes/foreign-program-arguments.md)
(#1314).

## The ruling

A program written by somebody else hears its arguments as bytes. What those bytes may touch is
decided separately, by the program's manifest and by the directories the line grants.

1. Arguments arrive as plain bytes: a POSIX-shaped argv in one page, carrying no authority. The
   measurement priced this at about 50 lines, written once, with no new syscall.
2. A command-line word that resolves to an existing file is granted that file, in the way the
   program's own manifest declares: read-only, read-write, or the file's directory. The manifest
   travels in the ELF, per §197 (a package is one archive file), M2, ruled the same day.
3. Any program's manifest may also declare "may create the named path" for a word that does not
   yet resolve, as in `cc -o main` or `tar cf x.tar`. This holds for vouched and unvouched programs
   alike. In practice it is write authority for that one name in its directory.
4. For an unvouched program, the default is read-only. It has no vouch, so under §219 (how the
   shell names an installed program to the spawner) its own note grants nothing. A named file is
   granted read-only, and read-write or create each need an explicit mark on the word. This
   section calls it the read-only default.
5. The directories granted on the line bound everything else. `..` and absolute paths are already
   refused, by the `std` layer before they reach the wire.

calef's framing of the read-only default: unvouched programs are copied binaries or fresh builds,
and locking them down by default means they can do little damage.

The maintainer's correction to that framing, recorded as such. This is confinement, not
antivirus. It does not recognise bad code; it bounds all unvouched code alike. An unvouched program
can still:

- read what was named on its line;
- write where `>` points;
- use the processor time and memory it was granted;
- print misleading output;
- damage the one file whose word carried a mark.

## Still open

The mark's spelling. Clause 4 needs a way to say "this word is writable" and "this word may be
created" on the line. How that is spelled is a naming decision, and it is calef's. Milestone 205
ships a provisional spelling, says so, and does not choose.

Environment variables and exit codes for a foreign program. They are the same family, and this
ruling is silent on both. §111 (inert configuration is a validated page) covers the native case
only.

The block's layout. Two programs agree on it, so milestone 205 proposes it and a ruling fixes it.
The note lists what is in it:

- where the page sits, a slot or `std`'s ignored entry registers;
- whether `argv[0]` is present, which the note shows it must be;
- bytes rather than UTF-8;
- the one-page ceiling of 4,080 bytes.

## Refused, with reasons

| option | why it lost |
|---|---|
| B. Designation per token, from a table per program | Only the program knows its own grammar. The table is 104 flags for `ripgrep` and 260 fields for `gitoxide`, redone per release, and neither is derivable in general. |
| C. `grant_plan` as the only channel | It carries one `u64` and a 64-bit flag mask. It cannot carry a regex, so `rg pattern` cannot be said at all. |
| Asking y/n at the prompt before granting | It breaks scripts and pipelines, which have nobody to answer. |

## Recorded as a later refinement, not ruled

Honouring an unvouched binary's note up to a ceiling set per session. That ties to §220 (signed
builds: a vendor signs, a developer self-signs), which is PROPOSED. Until something rules it, an
unvouched note grants nothing, as clause 4 says.

## Known cost

A secret passed on a command line is plain bytes, handed to the program with no protection. §111 refused free-form strings on the configuration page for this reason.
No wire can help, because a regex is also arbitrary bytes. Programs that take a secret this way
need a policy answer, which is §41 (the endpoint is the broker)'s shape.

## The analysis the ruling was made against

The rest of this section is the record as it stood before 2026-09-26, shortened.

### The premise, measured 2026-09-19

The block said *"The nife ABI has no argument vector"*, and that was right. It was not true that
programs here take no arguments. `grant_plan::Endowment` carries `arg: u64` and `flags: u64`, where
bit `i` is set when the manifest's `flags[i]` was typed. A program declares what it takes in a
`Manifest`, and the shell binds tokens into those slots or prints a typed refusal. What unmodified
`ripgrep` reaches for is `std::env::args()`, which compiles std's `unsupported` backend and yields
nothing.

### What the tree already did in the analogous case

Milestone 47 (navigation and naming) split Unix's environment map in three. Inert configuration
became a validated read-only page (§111). Names were answered by the namespace, because designation
is authorization. Secrets went behind an endpoint (§41). "Arguments" is the same kind of word: a
path is a designation, `--jobs 8` is data, and `--password` is a secret. The ruling keeps that
split. Bytes carry data, the manifest and the line's directories carry designation, and secrets stay
a known cost. §15 (the native ABI) had chosen capability slots over a self-describing environment,
and the ruling keeps authority out of the bytes for the same reason.

### The options as first written

| | shape | cost as first written |
|---|---|---|
| A | An argv, as POSIX has it | What ported programs expect. It puts designation, data and secrets back in one string vector. |
| B | A nife-shaped equivalent: designations as capabilities, inert values as data | Coherent with §15 and milestone 47. Every foreign program needs a shim, then uncosted. |
| C | `grant_plan` is the only answer, and a shim is policy | The most honest about this system and the least welcoming to the corpus of milestone 123 (the demonstration: somebody else's software, running narrow). |

The measurement found a fourth reading of B, used by Fuchsia, Genode, seL4's `sel4utils` and Xous:
bytes travel as an argv and authority travels separately, as a namespace. That is what clauses 1
and 5 rule, with clauses 2 to 4 adding what a manifest may say about a single word.

## How reversible it is

Not at all, in the way that matters. Every future program and every port is written against it.
Nothing outside this tree had acted on it at the ruling.

## What the ruling unblocks

Milestone 205, which builds the transport and the designation rules above. Behind it, everything
milestone 121 (`ripgrep` on nife) still owes: the confined `rg`, the loud `ENUMERATE` refusal and
the walk benchmark. Milestone 595 (the shell runs a `std` program)'s `rg pattern` step. Scripts
that take a parameter (§219). Milestone 123's corpus still waits on §171 (where a program image
starts) as well.
