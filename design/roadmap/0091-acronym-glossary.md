---
status: NOT-STARTED
raised: 2026-08-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 91. A glossary, and every acronym linked to it

Raised 2026-08-03 by calef, from the reader's chair: navigating the
acronyms is the hardest part of understanding these docs. That is the naming tenet's own concern
one level down; CLAUDE.md says names are what make this OS legible to humans and to LLMs, and an
acronym is a name whose claim is hidden until the reader already knows it.

Answered by calef 2026-09-05, and the answer changed the shape of the milestone
rather than picking a filename. The scheduling constraint stands on its own: this touches nearly
every documentation file, so it should start only when no lane holds unmerged `notes/` edits.

There is no glossary file. Every term gets a note, flat in `notes/`, the way an encyclopedia gets
an article. calef: *"It really seems like we should be writing files aka Wikipedia."*

The tree had already half-built it without a rule. Of the eighteen most frequent all-caps tokens
in the markdown, six already own a note: `notes/qemu.md`, `notes/dma.md` (386 lines),
`notes/iommu.md`, `notes/uart.md`, `notes/tcb.md`, `notes/abi.md`. Nobody planned that. It happened
because those are concepts and concepts get notes here, which is the convention this decision
extends rather than invents.

The maintainer proposed a hybrid and it was refused for a good reason. The proposal was a
glossary holding one row per borrowed term (`TOLUD`, `ECAM`, `DRHD`), with notes reserved for
concepts this tree owns. **That requires a per-term judgment about what counts as "ours"**, which
is the case-by-case reasoning calef had refused the same evening over `dynamic_ram`: an exception
argued from relevance is exactly what a test is supposed to replace. A rule needing taste at every
application is not a rule.

Two things the file-per-term answer does better, and the first is measured. A stub grows: this
tree learned something about `TOLUD` on 2026-09-04, that QEMU models the register nowhere on either
boot path, which is why milestone 256 had to distinguish an absent answer from a disagreeing one.
That finding is already more than a glossary row holds and would have had nowhere to live. And
the gate gets simpler: a link target that is a file can be checked for existence, where an
anchor inside one long glossary has to be parsed and breaks whenever somebody reorganizes the page.

And flat has a second reason, which is calef's and is the one that should shape the work
(2026-09-05): *"If nife proves worthwhile I can foresee building a website and the notes are a key
element of that website. Likewise I see the notes being useful within help on a nife system."*

Both destinations are already in the tree rather than being speculation. `AGENTS.md`'s naming
table gives the reason ordinary markdown is hyphenated: *"filenames become URL slugs in every static
site generator, and hyphens are word separators in a URL where underscores are joiners."* So the
website is why notes are named the way they already are, and flat-and-hyphenated is that rule
applied one level down. The on-system half is
[milestone 40](0040-documentation-service.md), documentation as a system service,
searchable and rendered and installed by packages.

What that changes for a lane, and it is worth stating because it is not obvious from "write a
note per acronym":

- A note is a page, so it should stand on its own for a reader who arrived from a search engine
  or from `doc <term>` on a running system, not only for someone reading the tree top to bottom.
- Nesting would put an arbitrary word in a URL. `/terms/tolud` says something about our
  filing rather than about the subject, and it is the kind of thing that cannot be changed later
  without breaking links somebody else has made.
- Links between notes are the navigation, because there is no glossary page to be the table of
  contents. `notes/README.md` is the index and every term note needs its line, which is the check
  in part 3.

**Flat rather than `notes/terms/`**, also calef's, and the reason is that the six which already
exist are flat: nesting would break every link pointing at them today, to buy a separation between
short notes and long ones that `notes/tcb.md` (54 lines) and `notes/mmu.md` (93) show this tree does
not make anyway.

Measured, so the size is a number: the markdown tree (notes/, design/, the root files)
carries ~835 distinct all-caps tokens. The top of the list is the real problem: IPC appears 251
times, DMA 231, EL0 181, IOMMU 180, TCB 123, none of them expanded anywhere a reader can reach
from the use. The naive count also includes things that are *not* acronyms (rights constants like
WRITE, the status vocabulary like BUILT, plain emphasis like NOT), which is the finding that
shapes the enforcement below: the true glossary is likely a low hundred entries, and the gate
needs a recorded line between prose and code.

The deliverable, in three parts:

1. **A note per acronym**, flat in `notes/`, named for the term in lower case (`notes/tolud.md`,
   `notes/ecam.md`). Each carries the expansion, what the term means *in this tree*, and where the
   concept is used. **TCB is the case that justifies the whole milestone**: here it is the trusted
   computing base *and* a thread control block, both senses live in this tree, and
   `notes/tcb.md` already exists and must carry both rather than one. A note that starts as three
   sentences is a stub and that is fine; six of these are already substantial pages.

   Every term note gets a `notes/README.md` line, like every other note. That index is the
   list of articles, and milestone 259's sweep found index entries go stale independently of what
   they point at, so the gate below should check the line exists rather than trusting anyone to
   add it.
2. Every prose use links to its entry. Every use, not first-use-per-file, and the rationale
   is calef's stated pain: readers land mid-file, from a search, a cross-reference, or a code
   comment's pointer, and a first-use convention only serves the reader who started at the top.
   The line that keeps this sane: **backticked tokens are code identifiers and are exempt**
   (`WRITE` the right, `BUILT` the status, register names); bare all-caps tokens in prose are
   acronyms and link. Tokens that are neither (emphasis, non-acronym capitals) go in a recorded
   exemption list next to the gate, each with a reason.
3. A gate, or it drifts: `script/lint` learns to fail on a bare all-caps prose token that is
   neither linked to its note nor in the exemption list, so a new acronym cannot arrive undefined.
   The file-per-term answer makes this check cheaper than the glossary would have: the target
   is a path, so the gate asks whether `notes/<term>.md` exists and whether `notes/README.md`
   carries its line. No anchor parsing, and nothing to break when a page is reorganized.
   The gate's own blind spot, recorded now: it cannot check that a link points at the *right*
   note, the same limit the citation checks already record.

## Scope note

The markdown tree first; rustdoc comments and kernel code comments are out of scope for this
milestone (comments cite notes by design, so the glossary serves them transitively; linking
inside rustdoc is a different mechanism and a follow-on decision). Sequencing: this touches
nearly every documentation file, so it lands as mechanical passes (glossary first, then linking
in reviewable batches) and should start only when no lane holds unmerged notes/ edits, for the
same conflict reason the roadmap split cited. Milestone 40 inherits these notes the way it inherits every other one.

## Index row

a note per acronym, flat in notes/, the way an encyclopedia gets an article (calef, 2026-09-05).
IPC appears 601 times and DMA 385 with no expansion a reader can reach; six of the top eighteen
already own a note, which is the convention this extends rather than invents
