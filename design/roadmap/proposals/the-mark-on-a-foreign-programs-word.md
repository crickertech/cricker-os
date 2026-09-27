---
status: PROPOSED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: 170
machine_requirements: none
specific_machine: none
needs_person: no
---
# The mark on a foreign program's word

Raised by milestone 205 (how a foreign program is told what to do)'s lane,
`milestone/205-foreign-program-arguments`. §170 (how a foreign program is told what to do) clause 4
gives an unvouched program every named file read-only. A person widens one word with a mark: "this
word is writable", or "this word may be created". §170 left the spelling to calef, because it is a
name a person types and every script will carry. The slug is a lane's coinage.

Nothing is built yet, because the mark qualifies a grant and the grant is the designation half
([`designating-a-foreign-programs-words.md`](designating-a-foreign-programs-words.md)). This page
exists so the spelling is decided before that half lands, not while it waits.

## Refused: there is no mark

calef ruled on 2026-09-27 at 06:35Z (UTC): "no mark", option 1. An unvouched program gets every
named word read-only, plus `>` for its output. To widen that, a person installs (vouches for) the
program, and its manifest then applies. An unvouched note grants nothing, so clause 3's "may
create" does not apply to it either.

The reasons:

- Choosing a spelling is irreversible, because every script carries it.
- Having no mark is reversible. A mark can be added later without breaking any script.
- Installing is already how trust widens here, by §208 (installing a package is granting it).

This file's frontmatter says PROPOSED only because a proposal can carry no other status. It is
refused, and waits for an integrator to promote it as a refused block. Everything below is the
analysis as it was put to calef.

## What the spelling has to survive

- **The program must never see the mark.** `sed -i x.txt` has to reach `sed` as `x.txt`. So the
  shell strips the mark from the argv, and a mark must be something no program's own grammar can
  already mean, or stripping it changes what the program was told.
- **It has to be greppable and previewable.** `caps` prints the widened word, as it prints the `-r`
  that widens `rm`.
- **Quoting passes it through.** `'+w:x'` is a word with a plus in it, as `"*.rs"` is a word with a
  star (milestone 67 (quoting and sequencing)'s rule: a quoted word designates itself).

## Options

What the shell treats as special today, checked in `crates/grant_plan/src/line.rs` and
`crates/glob`: `|`, `<`, `>`, `2>`, `&&`, `;`, both quotes, and the glob bytes `* ? [ ] \`. Anything
else is a plain byte in a word.

| | spelling | for | against |
|---|---|---|---|
| **M1** | a prefix sigil on the word: `+x.txt` writable, `++out.o` may be created | short; reads as "more than read-only"; unused by any shell | `+` starts real arguments (`sort +1`, `tail +5`, `date +%s`), so the shell would steal a word `date` needs. Stripping changes a program's meaning, which is the one thing a mark must not do |
| **M2** | a separate shell word before it: `rw x.txt`, `new out.o` | nothing to strip from inside a word; says what it means | a bare word is the program's own argument everywhere else (`rg rw x.txt` is ambiguous), so it needs a spelling no program could want, which puts us back at a sigil |
| **M3** | a shell option that names the word: `--grant-rw x.txt`, `--grant-new out.o` | the shell already owns one long option on every line (`--mem`), so this is that shape again; a program never sees it | long; and `--grant-rw` is a flag a program *could* define, though no program in the measured corpus does |
| **M4** | a suffix: `x.txt:rw`, `out.o:new` | reads naturally | `:` is legal in a filename and in a program's argument (`host:port`, `scp`), so a real word would be misread |

## Recommendation: M3, spelled `--rw` and `--new` if calef wants them short

Would we still choose it if every option cost the same? Yes. M3 is the only option that adds no new
lexical rule: `--mem` already establishes that a `--` word the shell recognises is the shell's,
taken off the line before the program sees it, and quoted to pass it through
(`design/roadmap/205-foreign-program-arguments.md`'s `BUGS` records that `--mem` behaves exactly
this way on a foreign line today). M1 and M4 lose because real arguments start with `+` or contain
`:`. M2 loses because it needs M1's sigil anyway.

The cost is a prefix collision on the long form, and it is small. `--rw` and `--new` are shorter and
collide more often. `--grant-rw` and `--grant-new` are longer and say what they are. The name is
calef's; the lane will build whichever is ruled.

## Prior art

No shell marks a word's rights, because none derives rights from words. The nearest is Plan 9's
`bind` flags (`-a`, `-b`, `-c` for create) on a namespace operation rather than on a command's
word, from memory and not re-read for this page. Capsicum's `capsh`-style wrappers and Fuchsia's
component manifests move the decision into configuration rather than onto the line.

## Reversibility

Irreversible once scripts use it: a mark is typed, and a changed spelling breaks every script.
Nothing uses it today. That is the reason to decide it before the designation half lands.

## What is blocked on the answer

Clause 4's widening only. The designation half can land read-only first, which is clause 4's
default, and gain the mark when this is ruled.
