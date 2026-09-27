"""Reading an unnumbered roadmap proposal, for the `script/` entry points that count them.

**What a proposal is** (milestone 247, calef 2026-09-03). Anybody may add to the roadmap;
prioritising it is a different act. A lane that finds work it is not doing writes
`design/roadmap/proposals/<slug>.md` with no number in it, because the thing concurrent lanes
collide over is the NUMBER and not the authority. The integrator promotes one at merge: give it a
number, `git mv` it up a directory, add the index row.

**Why this file exists.** `script/roadmap` gates the pile and lists it; `script/metrics` counts it
per week (milestone 276). Milestone 236's rule is that a derivation two scripts each carry a copy of
will drift while both keep looking authoritative, and the pile's whole defence against becoming a
graveyard is that a script can see it, so a second definition that quietly disagreed about what
counts is the one failure this record cannot afford.

**What is deliberately not here**, the same line `helpers/rust_source.py` and
`helpers/name_provenance.py` draw: only the derivation over a filename and a file's text. The
directory LISTING stays at the caller, because `script/roadmap` reads the working tree and
`script/metrics` reads blobs at revisions nobody has checked out.

Name: provisional, minted by milestone 276's lane on 2026-09-11, and it is a shared python module
under `helpers/`, which `script/names` puts out of its own scope, so this paragraph is the record
rather than a `Name:` block. `proposals` alone would not say which proposals (this tree also has
`**Status: PROPOSED**` decisions in `design/decisions/`, a different record with a different form);
`roadmap_records` would promise the index rows too, which live in two different parses that this
does not touch. An architect names modules, and none has ratified it.
"""

import re

import roadmap_block

DIRECTORY = 'design/roadmap/proposals'

# A proposal filename is a lowercase hyphenated slug carrying NO number. The slug is the readable
# half of a numbered block's name, so nothing new has to be learned, and it collides only if two
# lanes pick the same words, which is visible rather than silent.
_SLUG = re.compile(r"[a-z][a-z0-9-]*\.md")

# The date is what makes the pile measurable. A proposal nobody promotes is the same burial in a new
# place, and age is the only tell a script has.
_STATUS = re.compile(r"\*\*Status: PROPOSED (\d{4}-\d{2}-\d{2})\.\*\*")

_TITLE = re.compile(r"# [^0-9]")
_GATE = re.compile(r"\*\*Gate: ([A-Z0-9§, ]+)\.\*\* (\S)")

# What a proposal can be wrong about, as tokens rather than sentences, because the callers say it
# differently: `script/roadmap` prints a paragraph telling a lane how to fix the file, and
# `script/metrics` only needs to know this file is not a proposal it can count.
NOT_MARKDOWN = 'not-markdown'
NOT_A_SLUG = 'not-a-slug'
# Not a problem at all: a file this directory is allowed to hold that is not a proposal.
EXEMPT_FILE = 'exempt'
NO_TITLE = 'title'
NO_STATUS = 'status'
NO_GATE = 'gate'


# The one filename in this directory that is not a proposal. `README.md` says what the directory is
# for, in prose, and it is committed for a second reason: git does not track an empty directory, and
# on 2026-09-20 promoting every proposal at once made the directory vanish from the index, after
# which git's rename detection moved two lanes' new proposals a level up on their own branches.
#
# A file rather than a pattern, so that a second exemption has to be argued for.
EXEMPT = ('README.md',)


def filename_problem(filename):
    """Why this directory entry is not a proposal file, or None."""
    if filename in EXEMPT:
        return EXEMPT_FILE
    if not filename.endswith('.md'):
        return NOT_MARKDOWN
    if not _SLUG.fullmatch(filename):
        return NOT_A_SLUG
    return None


def classify(text):
    """(date, title, problem) for one proposal's text; `problem` is None when it is well formed.

    The three checks are the ones a numbered block also has to pass, in the same order: a title with
    no number in it (an unpromoted proposal has none), a dated `PROPOSED` status, and a gate, because
    a proposal a lane cannot start is worth as little as a milestone a lane cannot start.
    """
    lines = text.split('\n')
    # Milestone 596 (the roadmap blocks get frontmatter too): a proposal takes the numbered block's frontmatter, with `status: PROPOSED`, the
    # date it was written as `raised`, and the five dependency fields of §207 (the roadmap is a graph) where the gate was. Both
    # forms are read, because `script/metrics` counts the pile at revisions from before the switch.
    fields, start, fm_problems = roadmap_block.frontmatter(lines)
    if fields is not None:
        body = lines[start:]
        if not body or not _TITLE.match(body[0]):
            return None, None, NO_TITLE
        raised = fields.get('raised', '')
        if (fm_problems or fields.get('status') != roadmap_block.PROPOSED
                or not roadmap_block.DATE.fullmatch(raised)):
            return None, None, NO_STATUS
        if any(fields.get(r) is None for r in roadmap_block.DEPENDENCIES):
            return None, None, NO_GATE
        return raised, body[0][2:], None
    if not lines or not _TITLE.match(lines[0]):
        return None, None, NO_TITLE
    first = next((line for line in lines[1:] if line.strip()), '')
    status = _STATUS.match(first)
    if not status:
        return None, None, NO_STATUS
    paragraphs, current = [], []
    for line in lines:
        if line.strip():
            current.append(line)
        elif current:
            paragraphs.append(current)
            current = []
    if current:
        paragraphs.append(current)
    gate = ' '.join(paragraphs[2]) if len(paragraphs) > 2 else ''
    if not _GATE.match(gate):
        return None, None, NO_GATE
    return status.group(1), lines[0][2:], None


# ---- promotion, which removes the proposal ------------------------------------
#
# **A promoted proposal is deleted, and the milestone is where its work lives.** calef ruled this
# twice in one day and the second ruling is the one that stands: on the morning of 2026-09-18 he
# allowed a promoted proposal to be retired in place, carrying a `**Promoted:**` line; that evening,
# looking at what the first cluster promotion actually did to the directory, he reversed it. *"I'd
# like to drain the proposal files as we promote them versus accumulate another place where we
# capture work."*
#
# **The reversal is right and the evidence that argued against it was weaker than it looked.** The
# case for keeping was milestone 304's proposal, said to carry three things it got wrong that its
# block did not. Its block carries all three, in more detail: the prerequisite that was already
# done, the four `E0133`s, and Kani's bundled rustc running ten months behind this tree's pin. The
# same was true of milestone 313's. So keeping the files preserved almost nothing and cost a
# directory that grows for ever, which is a second place work accumulates and the exact shape
# `notes/roadmap.md` calls a burial in a new location.
#
# So the count this module feeds is the count of files, and a promoted proposal stops being either.
# What a promotion owes instead is that **anything the proposal carried and the milestone does not
# gets folded in before the file goes**, which is draining rather than discarding.
#
# **What this cannot do**, and the second half is the larger one.
#
# Nothing can find a promotion nobody wrote down. `promoted_from` reads a numbered block's own
# claim, and a spelling it does not know is a silent miss.
#
# **And a proposal closed without ever being promoted is invisible here**, which is the half of
# calef's rule no gate reaches. The ordering IS gated for a proposal a numbered block names in a
# `**Proposed.**` follow-on bullet, because that disposition must resolve to a file that exists, so
# deleting one fails the build. The rest are standalone, and deleting one leaves NOTHING in the
# working tree to notice: this module reads the tree, not the history. A gate that read `git log`
# could see it and is deliberately not written here, because `script/metrics` reads blobs at
# revisions nobody has checked out and the two callers would then need different answers to the same
# question.
CITATION = 'promoted from the proposal `<slug>`'

_PROMOTED_FROM = re.compile(r"promoted from the proposal `([a-z][a-z0-9-]*)`", re.I)

# The two spellings already in the tree on 2026-09-18, recognised so that this reads the record as
# it stands rather than only the record as it should have been written. A path (milestones 304, 315)
# or a parenthesised slug (313).
#
# **Both require the promotion verb, and that is the whole difficulty.** The first draft matched any
# `proposals/<slug>.md` in a status paragraph and reported milestone 259, which says it was minted
# "as the other half of" a proposal that is still open and still worth a lane. Citing a proposal and
# being promoted from one are different claims, and only the second one disposes of anything.
_PROMOTED_FROM_LOOSE = (
    re.compile(r"promoted from\s+`(?:design/roadmap/)?proposals/([a-z][a-z0-9-]*)\.md`", re.I),
    re.compile(r"proposal\s*\(`([a-z][a-z0-9-]*)`\)", re.I),
)


def promoted_from(status_paragraph):
    """The proposal slug a numbered block says it was promoted from, or None.

    None is the common case: most blocks were never proposals. A block that cites a proposal
    WITHOUT claiming promotion (milestone 259, "the other half of") returns None too, because that
    proposal is still somebody's to take.
    """
    for pattern in (_PROMOTED_FROM,) + _PROMOTED_FROM_LOOSE:
        m = pattern.search(status_paragraph)
        if m:
            return m.group(1)
    return None
