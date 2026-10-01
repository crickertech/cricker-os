#!/usr/bin/env python3
"""The week digest: the five files that moved a flagged series most, and the commits that moved
them, between two weekly snapshot refs. Milestone 623 (bullet under the chart explains a cliff);
provisional.

    python3 helpers/week_digest.py OLD NEW [SERIES ...]

OLD and NEW are the two weekly snapshot commits, the `commit` cells of adjacent rows in
`notes/project-metrics/weeks.csv`. SERIES is a metrics column name, `kernel_code_lines` say, the
field a flag line names second; with no SERIES the whole tree is digested. Output is plain lines a
pull request body can carry as-is:

    2026W39..2026W40 kernel_code_lines (kernel/src/), five files by lines moved:
    kernel/src/user/tests.rs: 3360 lines; 30a234b78 system_tests: move the 66 test-only files...
    ...

**Why a helper and not part of `script/metrics`.** The digest is `git log` archaeology over a
diff; `script/metrics` is a reader of blobs and series. Keeping the walk here keeps the metrics
script's inputs what its header says they are, and lets the digest run alone against any two refs
an annotator wants, flagged week or not.

**Churn is added plus removed, not net**, for the reason `script/metrics`' model columns give: a
careful sweep can net out near zero while being the largest thing that happened. A file moved
unchanged to another directory appears here as its whole line count, because from the series'
side (a path prefix) those lines left, which is exactly the cliff the digest exists to pre-chew.

The series-to-path table maps `kernel_*` to `kernel/src/` and falls back to the whole tree for
everything else, which for most series is the honest scope rather than a good filter. Its BUGS
section below carries that limitation where a reader meets it.

Name: provisional (milestone 623's lane, 2026-09-29). Naming is an architect's call.

BUGS

- It names movers, not causes. The commits are named with their subjects and the annotator still
  reads them; a digest that guessed causes would be worse than none.
- The series-to-path mapping is one entry deep (`kernel/src/`) plus a whole-tree fallback. A
  series whose files live across the tree digests the tree, and a series narrower than its prefix
  (kernel comments, say) inherits kernel/src wholesale.
- Merge commits contribute nothing, because `git log --numstat` shows no diff for them by default;
  their branches' own commits are walked and carry the changes, so nothing is missed, but a
  squash-merge convention would make this blind and this repository has refused one.
"""

import subprocess
import sys

# A series name from a flag line to the path prefix its lines live under. Whole tree when absent.
SERIES_PATHS = {'kernel': 'kernel/src/'}


def _git(*args):
    return subprocess.run(['git', *args], capture_output=True, check=True,
                          text=True).stdout


def series_scope(series):
    """The path prefix a series lives under, or '' for the whole tree."""
    prefix, _, _field = series.partition('_')
    return SERIES_PATHS.get(prefix, '')


def digest(old, new, series):
    """[(path, lines, [(sha, subject), ...])] for the five files that moved most, largest first."""
    scope = series_scope(series)
    log = _git('log', '--numstat', '--format=%h %s', '%s..%s' % (old, new))
    files, commits = {}, {}
    header = None
    for line in log.split('\n'):
        line = line.rstrip()
        if not line:
            continue
        parts = line.split('\t')
        if len(parts) != 3 or any(p.startswith(('Merge pull request', 'C ')) for p in parts):
            # A commit header is `sha subject` on one line; numstat rows are two numbers and a
            # path. A subject containing a tab would misparse, and no subject in this tree does.
            if '\t' not in line:
                header = line
            continue
        added, removed, path = parts
        if scope and not path.startswith(scope):
            continue
        added = int(added) if added.isdigit() else 0
        removed = int(removed) if removed.isdigit() else 0
        moved = added + removed
        if not moved:
            continue
        entry = files.setdefault(path, 0)
        files[path] = entry + moved
        if header:
            sha, _, subject = header.partition(' ')
            commits.setdefault(path, [])
            if (sha, subject) not in commits[path]:
                commits[path].append((sha, subject))
    ranked = sorted(files.items(), key=lambda kv: -kv[1])[:5]
    return [(path, moved, commits.get(path, [])) for path, moved in ranked]


def main(argv):
    if len(argv) < 2 or argv[0] in ('-h', '--help'):
        print(__doc__.strip().split('\n\n')[1], file=sys.stderr)
        return 2 if argv and argv[0] not in ('-h', '--help') else 0
    old, new = argv[0], argv[1]
    series = argv[2] if len(argv) > 2 else 'the whole tree'
    scope = series_scope(series) if len(argv) > 2 else ''
    where = ' (%s)' % scope if scope else ''
    print('%s..%s %s%s, five files by lines moved:' % (old, new, series, where))
    for path, moved, commits in digest(old, new, series if len(argv) > 2 else ''):
        who = '; '.join('%s %s' % (sha, subject) for sha, subject in commits)
        print('%s: %d lines; %s' % (path, moved, who or 'no commit found'))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
