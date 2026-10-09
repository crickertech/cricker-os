"""The comment-block ratchet: §267 (a comment states the constraint as it is now), held to a
baseline that only falls.

calef ruled the ruling this gate holds on 2026-10-07 (UTC), deciding the forks of
`notes/comment-cost-2026-10-07.md` (PR #1800, fork (c)), and ratified its sharper form the same
day: "ratify #1800 Fork (c) in the sharper form". Milestone 860 (comments state the constraint as
it is now) files the sweep and builds this gate. It copies `helpers/file_length_ratchet.py`'s
pattern, §266 (a Rust source file stays under 2,000 lines)'s ratchet, which itself copies
`helpers/prose_ratchet.py`'s, §212 (a prose budget)'s: a committed list that only shrinks, a
comparison against the merge base, and a selftest that runs before the check.

    python3 helpers/comment_block_ratchet.py --check          # what script/lint runs
    python3 helpers/comment_block_ratchet.py --bank           # lower the baseline to the tree; never raises
    python3 helpers/comment_block_ratchet.py --selftest       # the rules, as fixtures
    python3 helpers/comment_block_ratchet.py --init           # write the baseline from scratch (milestone 860 only)

Name: provisional, minted by milestone 860's lane on 2026-10-09 (UTC). A shared python module
under `helpers/`, which `script/names` puts out of its own scope, so its provenance is this
paragraph rather than a `Name:` block. calef names things; expect this to change. Refused
`comment_ratchet.py`: §267 caps a *block*, not comment density, which the ruling keeps on purpose
(the tree stays commented far more heavily than production code, deliberately), and a name that
read as capping comments would say the opposite of the decision.

**The measure.** A block is a run of adjacent comment lines of one kind, doc (`///`, `//!`,
`/** */`, `/*! */`) or plain (`//`, `/* */`), exactly the definition
`notes/comment-cost-2026-10-07.md` section 4 measured with; its size is its line count. A trailing
comment on a code line is a span of one and can begin a block, which is the note's definition as
its own scripts took it. Every tracked `.rs` file outside `vendor/`, plus untracked ones, so a
lane's new file is judged before its first commit. A block of 40 lines is allowed; 41 is over.

**The cap: 40 lines**, calef's "about 40" read off the measured distribution rather than chosen
before it. Measured 2026-10-09 (UTC) at this branch's base `2d59ddde1` with this file's own lexer,
which reproduces the note's method: 28,950 blocks, median 2 lines, mean 4.77. Blocks over 40 hold
19.8% of comment lines (the note, one day earlier, measured 20%); over 30 holds 24.5% and over 50
holds 17.3%, so 40 is where the long tail's bulk sits, and it is the number calef named.

**The rules** (§266 section 3 option (b)'s shape, ruled there by calef over exact match, applied
one level down):

- A block not on the list fails at 41 lines or more. A new file's blocks meet the cap outright.
- A listed block fails if it grew against the merge base, and if it is larger than its entry. The
  growth check is unconditional: an entry can sit above its block's real size because a shrink was
  never banked, and that slack is not room to grow back into.
- A list entry is a ceiling that `--bank` lowers and nothing raises. An entry fails when its block
  is gone (its file is gone, or the file holds fewer over-cap blocks than the list holds rows), so
  dead rows cannot accumulate.
- Against the merge base, a row may not be added and an entry may not rise. Rows are matched file
  by file in line order, so a shrink that shifts later blocks down moves their rows in the same
  change, the way a rename moves a row. A rename moves its rows in the same change, at the same
  numbers.
- Exceptions are not provided for. A block that must stay over the cap needs §267 amended, by an
  architect.

**Why a ceiling and not exact match** (§266 section 3's measurement, which holds here too): of the
664 merges in the fortnight to 2026-10-08, 164 touched a file the file-length ratchet lists; two
shrinking changes to one file in one merge group each write a different number on the same row
under exact match and the group fails even when the Rust merged cleanly. Under a ceiling both pass;
the cost is that an entry can sit above its block's real size until someone banks it.

**The failure message** names the file, the block's first line, its size and the ceiling, and says
the remedy: history goes in the commit message, findings go in a note, and the block keeps the
constraint as it is now (§267). The remedy is never to delete a fact nobody has recorded.
"""

import os
import re
import subprocess
import sys

# --- the ceiling and the baseline -----------------------------------------------------------------

CEILING = 40
BASELINE = 'design/comment-block-baseline.tsv'

HEADER = """\
# The comment-block ratchet's baseline: milestone 860 (comments state the constraint as it is now),
# for §267 (a comment states the constraint as it is now).
# One row per comment block over the 40-line cap when it was written: the file and the block's
# first line (with `#2`, `#3`... only where two over-cap blocks would start at one line), then the
# block's lines. A number is a ceiling that may only go DOWN: lower it with
# `python3 helpers/comment_block_ratchet.py --bank`, which never raises and never adds. A row goes,
# in the same change that takes its block to 40 lines or below, or deletes it; a row may not be
# added and a number may not rise (script/lint compares against the merge base, matching rows in
# line order per file, so a shrink that shifts later blocks moves their rows in the same change).
# A rename moves its rows in the same change, at the same numbers. A block that must exceed its row
# needs §267 amended by an architect, not an edit here.
"""

REMEDY = ('The remedy is the sweep\'s move: history goes in the commit message, findings go in a '
          'note, and the block keeps the constraint as it is now (§267). It is never to delete a '
          'fact nobody has recorded.')


def fmt(n):
    return f'{n:,}'


def read_baseline(text):
    """{(path, first_line, occurrence): ceiling}, the occurrence starting at 1."""
    rows = {}
    for line in text.splitlines():
        if not line.strip() or line.startswith('#'):
            continue
        cells = line.split('\t')
        if cells[0] == 'block':
            continue
        key = parse_key(cells[0])
        if key:
            rows[key] = int(cells[1])
    return rows


def parse_key(cell):
    """`path:line` or `path:line#n` -> (path, line, n), or None when the cell is not one."""
    m = re.fullmatch(r'(.+):(\d+)(?:#(\d+))?', cell)
    if not m:
        return None
    return (m.group(1), int(m.group(2)), int(m.group(3) or 1))


def write_baseline(rows):
    out = [HEADER, 'block\tlines\n']
    for key in sorted(rows, key=lambda k: (k[0], k[1], k[2])):
        path, line, occ = key
        suffix = f'#{occ}' if occ > 1 else ''
        out.append(f'{path}:{line}{suffix}\t{rows[key]}\n')
    with open(BASELINE, 'w') as f:
        f.write(''.join(out))


# --- the lexer -------------------------------------------------------------------------------------
#
# Adapted from notes/comment-cost-2026-10-07/rustlex.py (lane/comment-cost-measurement,
# 2026-10-07 UTC), which stays untouched as that note's evidence and is the same lexer the note's
# distribution was measured with. It handles line comments, nested block comments, doc versus
# plain, strings, raw strings, byte strings, and char literals versus lifetimes; "good enough to
# decide" is its own bar and the note's, and it is quoted here rather than improved on purpose, so
# the gate and the measurement read the same tree.

def lex_comment_spans(src):
    """[(kind, start_line, end_line)], 1-based, in source order. kind is 'doc' or 'plain'."""
    i, n = 0, len(src)
    spans = []
    line = 1
    while i < n:
        c = src[i]
        if c == '\n':
            line += 1
            i += 1
            continue
        if c in ' \t\r':
            i += 1
            continue
        if src.startswith('//', i):
            j = src.find('\n', i)
            if j < 0:
                j = n
            text = src[i:j]
            doc = (text.startswith('///') and not text.startswith('////')) or text.startswith('//!')
            spans.append(('doc' if doc else 'plain', line, line))
            i = j
            continue
        if src.startswith('/*', i):
            depth = 0
            j = i
            sl = line
            while j < n:
                if src.startswith('/*', j):
                    depth += 1
                    j += 2
                    continue
                if src.startswith('*/', j):
                    depth -= 1
                    j += 2
                    if depth == 0:
                        break
                    continue
                if src[j] == '\n':
                    line += 1
                j += 1
            text = src[i:j]
            doc = ((text.startswith('/**') and not text.startswith('/***') and text != '/**/')
                   or text.startswith('/*!'))
            spans.append(('doc' if doc else 'plain', sl, line))
            i = j
            continue
        # Raw strings r"..", r#".."#, br#".."#, c#".."#: skip whole, newlines counted.
        m = re.match(r'(b|c)?r(#*)"', src[i:i + 300])
        if m and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == '_')):
            end = '"' + m.group(2)
            j = src.find(end, i + m.end())
            if j < 0:
                j = n
            else:
                j = j + len(end)
            line += src[i:j].count('\n')
            i = j
            continue
        if c == '"' or (c in 'bc' and i + 1 < n and src[i + 1] == '"'
                        and not (i > 0 and (src[i - 1].isalnum() or src[i - 1] == '_'))):
            j = i + (1 if c == '"' else 2)
            while j < n and src[j] != '"':
                if src[j] == '\\':
                    j += 1
                j += 1
            j += 1
            line += src[i:j].count('\n')
            i = j
            continue
        if c == "'" or (c == 'b' and i + 1 < n and src[i + 1] == "'"):
            k = i + (1 if c == "'" else 2)
            m = re.match(r"(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'\n])'", src[k:k + 16])
            if m:
                i = k + m.end()
                continue
            m = re.match(r"'[A-Za-z_][A-Za-z0-9_]*", src[i:])
            if m:
                i = i + m.end()
                continue
            i += 1
            continue
        m = re.match(r'[A-Za-z_][A-Za-z0-9_]*|[0-9][0-9A-Za-z_.]*', src[i:i + 200])
        if m:
            i = i + m.end()
            continue
        i += 1
    return spans


def blocks_over(src, ceiling=CEILING):
    """[(first_line, lines)] for every block over `ceiling`, in line order.

    A block is a run of adjacent comment spans of one kind whose line ranges are consecutive:
    the note's section 4 definition, as its own q4.py took it.
    """
    out = []
    cur = None
    for kind, a, b in lex_comment_spans(src):
        if cur and a == cur[1] + 1 and kind == cur[2]:
            cur = [cur[0], b, kind]
        else:
            if cur and cur[1] - cur[0] + 1 > ceiling:
                out.append((cur[0], cur[1] - cur[0] + 1))
            cur = [a, b, kind]
    if cur and cur[1] - cur[0] + 1 > ceiling:
        out.append((cur[0], cur[1] - cur[0] + 1))
    return out


def block_counts(paths, read):
    """{path: [(first_line, lines)]} for the over-cap blocks of each path."""
    return {p: blocks_over(read(p)) for p in paths}


def rows_from_counts(counts):
    """{(path, first_line, occurrence): lines}, the occurrence separating same-line collisions."""
    rows = {}
    seen = {}
    for path, blocks in counts.items():
        for line, size in blocks:
            occ = seen.get((path, line), 0) + 1
            seen[(path, line)] = occ
            rows[(path, line, occ)] = size
    return rows


# --- git ------------------------------------------------------------------------------------------

def git(*args):
    r = subprocess.run(('git',) + args, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def tracked_rust():
    """Every `.rs` path the rules reach: tracked outside `vendor/`, untracked ones too."""
    files = git('ls-files', '-z', '--cached', '--others', '--exclude-standard') or ''
    return {f for f in files.split('\0')
            if f and f.endswith('.rs') and not f.startswith('vendor/') and os.path.exists(f)}


BASE_OVERRIDE = None  # `--check --base REV`, for falsifying the merge-base half against a chosen commit


def merge_base():
    if BASE_OVERRIDE:
        return (git('rev-parse', BASE_OVERRIDE) or '').strip() or None
    base = (git('merge-base', 'HEAD', 'origin/main') or '').strip()
    head = (git('rev-parse', 'HEAD') or '').strip()
    return base if base and base != head else None


def renamed_rust(base):
    """[(old, new)] for every `.rs` file git itself calls renamed between `base` and the tree."""
    pairs = []
    for line in (git('diff', '--name-status', '-M', base, '--', '*.rs') or '').splitlines():
        cells = line.split('\t')
        if cells[0].startswith('R') and len(cells) == 3:
            pairs.append((cells[1], cells[2]))
    return pairs


# --- the rules, pure so the selftest can drive them ------------------------------------------------

def problems(counts, rows, base_counts=None, base_rows=None, renames=()):
    """Every failure in one tree. See the header for the rules.

    `counts` is the tree's {path: [(first_line, lines)]}; `rows` the baseline's
    {(path, first_line, occurrence): ceiling}; `base_counts` and `base_rows` the same at the merge
    base, where there was one; `renames` git's (old, new) `.rs` pairs since the base. The baseline
    comparison is skipped when `base_rows` is None, which is the base holding no baseline: the
    change that first lands one.
    """
    if not counts:
        return ['the check judged no Rust file at all. A selector that matches nothing passes by '
                'checking nothing (design/roadmap/0401-a-gate-that-selects-the-set-it-judges.md)']
    was_called = {new: old for old, new in renames}
    bad = []
    total_blocks = sum(len(blocks) for blocks in counts.values())

    # The tree: an unlisted block over the cap, and every way a listed one may not stand. Rows and
    # blocks are matched file by file in line order, so a shrink that shifts later blocks down has
    # moved its rows rather than deleting and adding them.
    by_file = {}
    for (path, line, occ), ceiling in rows.items():
        by_file.setdefault(path, []).append((line, occ, ceiling))
    for path in sorted(counts):
        blocks = counts[path]
        listed = sorted(by_file.get(path, []))
        if len(listed) < len(blocks):
            for line, size in blocks[len(listed):]:
                bad.append(f'{path}:{line}: a {fmt(size)}-line comment block against the '
                           f'{fmt(CEILING)}-line cap, and it is not on the list. A new block meets '
                           f'the cap outright. {REMEDY}')
        for (line, size), (_row_line, _occ, ceiling) in zip(blocks, listed):
            if size > ceiling:
                bad.append(f'{path}:{line}: a {fmt(size)}-line comment block, over its baseline '
                           f'ceiling of {fmt(ceiling)}. A listed block may not exceed its entry. '
                           f'{REMEDY}')
    for path in sorted(by_file):
        listed = sorted(by_file[path])
        blocks = counts.get(path)
        if blocks is None:
            old = was_called.get(path, path)
            if counts.get(old) is not None:
                continue  # the rename check below carries it
            bad.append(f'{BASELINE}: {path} is not in the tree. Move its rows with the rename, at '
                       f'the same numbers, or remove them; the list cannot go stale')
        elif len(listed) > len(blocks):
            for line, occ, _ceiling in listed[len(blocks):]:
                suffix = f'#{occ}' if occ > 1 else ''
                bad.append(f'{BASELINE}: {path}:{line}{suffix} has no over-cap block behind it. '
                           f'Remove its row; the list cannot go stale')

    # The tree against its merge base: a listed block may not grow, unconditionally.
    if base_counts is not None:
        for path in sorted(counts):
            old = was_called.get(path, path)
            was = base_counts.get(old)
            if not was:
                continue
            now = counts[path]
            for (line, size), (_was_line, was_size) in zip(now, was):
                if size > was_size:
                    bad.append(f'{path}:{line}: grew from {fmt(was_size)} to {fmt(size)} lines '
                               f'against the merge base. A listed block may only shrink. {REMEDY}')

    # The list against its merge-base copy: no additions, no raises, renames at the same numbers.
    if base_rows is not None:
        base_by_file = {}
        for (path, line, occ), ceiling in base_rows.items():
            base_by_file.setdefault(path, []).append((line, occ, ceiling))
        for path in sorted(by_file):
            listed = sorted(by_file[path])
            old = was_called.get(path, path)
            was = sorted(base_by_file.get(old) or base_by_file.get(path) or [])
            if not was:
                bad.append(f'{BASELINE}: {path} gained rows. The list only shrinks; a new block '
                           f'meets the cap outright')
                continue
            if len(listed) > len(was):
                bad.append(f'{BASELINE}: {path} gained a row against the merge base '
                           f'({len(listed)} rows over {len(was)}). The list only shrinks; a new '
                           f'block meets the cap outright')
            for (line, occ, ceiling), (_was_line, _was_occ, was_ceiling) in zip(listed, was):
                if ceiling > was_ceiling:
                    suffix = f'#{occ}' if occ > 1 else ''
                    bad.append(f'{BASELINE}: {path}:{line}{suffix} raised its ceiling from '
                               f'{fmt(was_ceiling)} to {fmt(ceiling)}. An entry only goes down '
                               f'(§266 section 3, option (b), applied one level down)')
    return bad


def banked(rows, counts):
    """The baseline after banking: entries lowered to the tree, never raised, never added.

    A row whose block is gone or at or under the cap drops out, which is the rule `check` already
    enforces on the tree: banking is how a sweep leaves the list in the same change.
    """
    out = {}
    by_file = {}
    for (path, line, occ), ceiling in rows.items():
        by_file.setdefault(path, []).append((line, occ, ceiling))
    for path, listed in by_file.items():
        for (line, occ, ceiling), (block_line, block_size) in zip(sorted(listed), counts.get(path, [])):
            out[(path, block_line, occ)] = min(block_size, ceiling)
    return out


# --- the verbs ------------------------------------------------------------------------------------

def gather():
    """(counts, rows, base_counts, base_rows, renames) for the tree and its merge base."""
    read = lambda p: open(p, encoding='utf-8', errors='replace').read()
    counts = block_counts(tracked_rust(), read)
    rows = read_baseline(open(BASELINE).read())
    base = merge_base()
    base_counts, base_rows, renames = None, None, ()
    if base:
        renames = renamed_rust(base)
        wanted = {path for (path, _line, _occ) in rows} | {new for _old, new in renames}
        base_counts = {}
        for path in sorted(wanted):
            text = git('show', f'{base}:{path}')
            if text is not None:
                over = blocks_over(text)
                if over:
                    base_counts[path] = over
        old_text = git('show', f'{base}:{BASELINE}')
        if old_text is not None:
            base_rows = read_baseline(old_text)
    return counts, rows, base_counts, base_rows, renames


def run_check():
    if not os.path.exists(BASELINE):
        print(f'comment-block ratchet: {BASELINE} is missing. The gate and its list are one change; '
              f'deleting one deletes the gate', file=sys.stderr)
        return 1
    counts, rows, base_counts, base_rows, renames = gather()
    bad = problems(counts, rows, base_counts, base_rows, renames)
    if bad:
        print('comment-block ratchet: the tree breaks §267 against its baseline:', file=sys.stderr)
        for b in bad:
            print(f'  {b}', file=sys.stderr)
        print('\nMove the history to a commit message and the findings to a note, bank the shrink '
              'with --bank, or amend §267 with an architect. See milestone 860\'s block, '
              'design/roadmap/0860-comments-state-the-constraint-as-it-is-now.md.', file=sys.stderr)
        return 1
    over = sum(len(blocks) for blocks in counts.values())
    print(f'comment-block ratchet: {len(counts)} Rust files, {fmt(over)} comment blocks over the '
          f'{fmt(CEILING)}-line cap, all held to {len(rows)} baseline rows')
    return 0


def bank(init=False):
    if init and os.path.exists(BASELINE):
        print(f'comment-block ratchet: {BASELINE} already exists. --init wrote it once, in '
              f'milestone 860; it is refused now, and --bank is the verb that never raises',
              file=sys.stderr)
        return 2
    if not init and not os.path.exists(BASELINE):
        print(f'comment-block ratchet: {BASELINE} does not exist; there is nothing to bank',
              file=sys.stderr)
        return 2
    counts = block_counts(tracked_rust(),
                          lambda p: open(p, encoding='utf-8', errors='replace').read())
    if init:
        rows = rows_from_counts(counts)
    else:
        rows = banked(read_baseline(open(BASELINE).read()), counts)
    write_baseline(rows)
    print(f'comment-block ratchet: {len(rows)} rows written to {BASELINE}')
    return 0


def selftest():
    """The rules as fixtures: the cap, the ratchet's every refusal, banking, and the traps."""
    RENAME = [('x/old.rs', 'x/renamed.rs')]

    def tree(**files):
        return {f'x/{name}.rs': blocks for name, blocks in files.items()}

    def rows(path, *pairs):
        return {(path, line, occ): size for line, occ, size in pairs}

    cases = []

    def fails(name, bad):
        cases.append((name, bool(bad)))

    def passes(name, bad):
        cases.append((name, not bad))

    BIG = [(100, 50)]
    BIGGER = [(100, 60)]

    # The cap: a new 41-line block fails; the measure never hands `problems` one at 40 or below,
    # so a block that left the cap simply stops being counted.
    fails('a new 41-line block fails', problems(tree(a=[(10, 41)]), {}))
    passes('a block at the cap is not counted at all', problems(tree(a=[]), {}))
    # A listed block: over its entry fails, growth against the merge base fails even within it.
    row = rows('x/a.rs', (100, 1, 55))
    base = tree(a=BIG)
    fails('a listed block over its entry fails', problems(tree(a=BIGGER), row, base, row))
    fails('a listed block grown against the merge base fails',
          problems(tree(a=[(100, 52)]), rows('x/a.rs', (100, 1, 60)), base, row))
    passes('a listed block unchanged passes', problems(base, row, base, row))
    passes('a listed block shrunk passes', problems(tree(a=[(100, 45)]), row, base, row))
    # A vanished block's row must go; a file's rows cannot outnumber its over-cap blocks.
    fails('a row whose block left the cap fails', problems(tree(a=[]), row, base, row))
    fails('a row for a deleted file fails', problems(tree(b=BIG), row, {}, row))
    passes('a shrunk block passes once its row is gone', problems(tree(a=[]), {}, base, row))
    # Two blocks in one file pair in line order; the second's row moves with a shift above it.
    two = tree(a=[(100, 50), (300, 60)])
    two_rows = rows('x/a.rs', (100, 1, 50), (300, 1, 60))
    passes('two listed blocks pass', problems(two, two_rows, two, two_rows))
    shifted = tree(a=[(100, 45), (290, 60)])
    shifted_rows = rows('x/a.rs', (100, 1, 45), (290, 1, 60))
    passes('a shrink that shifts the second block moves its row',
           problems(shifted, shifted_rows, two, two_rows))
    fails('a shifted row raised against the merge base fails',
          problems(shifted, rows('x/a.rs', (100, 1, 45), (290, 1, 61)), two, two_rows))
    # The list against its merge-base copy: rows are never added, entries never rise.
    fails('a row added against the merge base fails',
          problems(two, rows('x/a.rs', (100, 1, 50), (300, 1, 60), (500, 1, 45)),
                   tree(a=BIG), row))
    fails('an entry raised against the merge base fails',
          problems(base, rows('x/a.rs', (100, 1, 56)), base, row))
    fails('dropping a row whose block is still over the cap fails',
          problems(base, {}, base, row))
    # Renames move rows in the same change, at the same numbers.
    passes('a rename that moves its rows at the same numbers passes',
           problems(tree(renamed=BIG), rows('x/renamed.rs', (100, 1, 50)),
                    tree(old=BIG), rows('x/old.rs', (100, 1, 50)), RENAME))
    fails('a rename that leaves its rows behind fails',
          problems(tree(renamed=BIG), rows('x/old.rs', (100, 1, 50)),
                   tree(old=BIG), rows('x/old.rs', (100, 1, 50)), RENAME))
    # A selector that matches nothing.
    fails('a selector that matches nothing fails loudly', problems({}, {}))
    # Banking lowers to the tree, never raises, never adds, and drops dead rows.
    if banked(rows('x/a.rs', (100, 1, 55), (300, 1, 60)),
              tree(a=[(100, 50)])) != rows('x/a.rs', (100, 1, 50)):
        cases.append(('bank lowers an entry to the tree and drops a dead row', False))
    if banked(rows('x/a.rs', (100, 1, 50)), tree(a=[(100, 55)])) != rows('x/a.rs', (100, 1, 50)):
        cases.append(('bank never raises an entry', False))
    if banked(rows('x/a.rs', (100, 1, 50)), tree(a=[(100, 50), (400, 45)])) != \
            rows('x/a.rs', (100, 1, 50)):
        cases.append(('bank never adds a row', False))
    # The lexer: the block definition, doc versus plain, strings that hold comment markers.
    src = '//! a\n//! b\nlet x = 1;\n// c\n/* d\ne */\nlet s = "// not a comment";\n'
    if blocks_over('//! a\n' * 41) != [(1, 41)] or blocks_over('//! a\n' * 40) != []:
        cases.append(('a run of 41 doc lines is one block over the cap, 40 is not', False))
    if lex_comment_spans(src) != \
            [('doc', 1, 1), ('doc', 2, 2), ('plain', 4, 4), ('plain', 5, 6)]:
        cases.append(('adjacent spans of one kind merge, kinds differ, strings are skipped', False))
    if blocks_over('let s = r#"// not\na comment"#; // trailing\n// next\n'):
        cases.append(('raw strings do not start blocks', False))
    # The message names the file, the line, the sizes and the remedy.
    msg = problems(tree(a=[(10, 41)]), {})[0]
    if not all(s in msg for s in ('a.rs:10', '41', '40', 'commit message', 'note', 'never to delete')):
        cases.append(('the failure message names the file, line, sizes and remedy', False))

    failed = [name for name, ok in cases if not ok]
    for name in failed:
        print(f'comment-block ratchet selftest: {name}', file=sys.stderr)
    if not failed:
        print(f'comment-block ratchet selftest: {len(cases)} cases pass')
    return 1 if failed else 0


def main(argv):
    root = git('rev-parse', '--show-toplevel')
    if root:
        os.chdir(root.strip())
    global BASE_OVERRIDE
    if len(argv) >= 3 and argv[0] == '--check' and argv[1] == '--base':
        BASE_OVERRIDE = argv[2]
        argv = argv[:1]
    if argv and argv[0] == '--selftest':
        return selftest()
    if not argv or argv[0] == '--check':
        return run_check()
    if argv[0] in ('--bank', '--init'):
        return bank(init=argv[0] == '--init')
    print(__doc__.split('\n\n')[1], file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
