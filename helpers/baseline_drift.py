"""Cumulative icount drift per benchmark row since a fixed anchor, from git alone.

Milestone 415 (sub-tripwire drift accumulates across baseline saves), item 3, as calef ruled it on
2026-09-26 (UTC): "3b", a report and not a gate, in §190 (must an icount baseline save record why it
moved). `cargo xtask bench --check` compares
against the last saved floor and `--save` rewrites that floor, so steps under the 10% tripwire add
up and nothing fires. This module says how far each row has moved since a fixed anchor, beside the
`# why:` lines each save wrote, so slow drift is visible to someone who did not go looking.

    python3 helpers/baseline_drift.py              # the report for HEAD, as markdown, on stdout
    python3 helpers/baseline_drift.py --selftest   # the arithmetic against fixtures; script/lint
    python3 helpers/baseline_drift.py --audit      # reproduce the 2026-09-15 audit's five figures

`script/metrics` imports it for the weekly series (`baseline-drift.csv`), the status line on
notes/project-metrics.md, and the generated appendix notes/project-metrics/baseline-drift.md.

Name: provisional, minted by milestone 415's lane on 2026-09-26. A shared python module under
`helpers/`, outside `script/names`' scope, so its provenance is this paragraph. Refused
`drift_report`, which names the output and not the thing measured.

The anchor is the 2026-09-15 audit's (notes/benchmarks/baseline-save-audit.md), one commit per
architecture, pinned in ANCHORS below. aarch64 starts at `74431429`, the first save after the
one-hart fix and the QEMU pin, because every earlier number means something else. riscv64 and
x86_64 start at their birth. Refused: anchoring at the first save that carries `# why:` lines
(2026-09-23). That would read every row as zero on the day the report started, and riscv64's
`ctx_switch`, the +10.78% that motivated this milestone, would disappear from the one place meant
to show it. The saves before the ledger are listed with their commit subject instead, marked so.

What a row's number means. Every figure is per iteration (`ticks / iters`), so a save that changes
an iteration count reads as zero, which is the audit's first correction. `floor` is the committed
number against the anchor. `restamp` is the compiler term `cargo xtask bench --restamp` has carried
since the last `--save` (xtask/src/restamp.rs), read back from the last `# why:` line that holds
its ledger: a restamp moves `# toolchain:` without moving a number, so without this term the floor
would understate what the pinned compiler produces. `combined` compounds the two. A row born after
the anchor is measured from the first save that carries it, and says so.

BUGS

- The restamp ledger is read by string shape, mirroring `prior_cumulative` in
  xtask/src/restamp.rs. If that marker or its `name +0.123%` listing changes there, this reads
  nothing and reports a zero restamp term, silently. The selftest pins the shape both sides use
  today, and nothing compares the two.
- A row that is renamed reads as one row removed and one born. None has been renamed since the
  anchors.
- Like the audit, this reads what each save recorded, not what the tree measured between saves.
"""

import functools
import subprocess
import sys
from datetime import datetime, timezone

# (anchor commit, the path the file had at that commit). The audit's anchors, full SHAs so a
# short one never becomes ambiguous. aarch64's file was `bench/baseline.txt` until 2026-08-03.
ANCHORS = {
    'aarch64': ('744314290a65a6e5fce285f4d06b9d6f31e1869c', 'bench/baseline.txt'),
    'riscv64': ('8c279536fd1b2165d4516ba437a82de8f134e2c2', 'bench/baseline-riscv64.txt'),
    'x86_64': ('d31aa77c98e67b5dd2c7ee72c2631baf24d6e7e0', 'bench/baseline-x86_64.txt'),
}
# Every name the file has had, newest first. A save is read from the first that exists.
PATHS = {
    'aarch64': ('bench/baseline-aarch64.txt', 'bench/baseline.txt'),
    'riscv64': ('bench/baseline-riscv64.txt',),
    'x86_64': ('bench/baseline-x86_64.txt',),
}
ARCHES = tuple(ANCHORS)
TRIPWIRE_PCT = 10.0      # `cargo xtask bench --check`'s bound against the last floor
WATCH_PCT = 5.0          # half the tripwire: the weekly series counts rows past it
# Must match CUMULATIVE_MARKER in xtask/src/restamp.rs.
RESTAMP_MARKER = 'cumulative since the last --save:'


# --- reading one file ---------------------------------------------------------------------------

def parse(text):
    """{'rows': {name: (ticks, iters)}, 'why': [reason, ...]} from one baseline file's text."""
    rows, why = {}, []
    for line in text.split('\n'):
        stripped = line.strip()
        if stripped.startswith('# why:'):
            why.append(stripped[len('# why:'):].strip())
            continue
        if not stripped or stripped.startswith('#'):
            continue
        parts = stripped.split()
        if len(parts) == 3 and parts[1].isdigit() and parts[2].isdigit() and int(parts[2]) > 0:
            rows[parts[0]] = (int(parts[1]), int(parts[2]))
    return {'rows': rows, 'why': why}


def restamp_terms(why):
    """{row: percent} from the last `# why:` line carrying the restamp ledger, else {}."""
    ledger = None
    for reason in why:
        if RESTAMP_MARKER in reason:
            ledger = reason.rsplit(RESTAMP_MARKER, 1)[1].strip()
    out = {}
    if not ledger or ledger == 'none':
        return out
    for item in ledger.split(', '):
        name, _, pct = item.strip().partition(' ')
        try:
            out[name] = float(pct.strip().rstrip('%'))
        except ValueError:
            continue
    return out


# --- the arithmetic -----------------------------------------------------------------------------

def per_iter(cell):
    ticks, iters = cell
    return ticks / iters


def pct(before, after):
    """Per-iteration move from `before` to `after`, in percent."""
    return (per_iter(after) / per_iter(before) - 1.0) * 100.0


def compound(a, b):
    """Two successive percent moves as one."""
    return ((1.0 + a / 100.0) * (1.0 + b / 100.0) - 1.0) * 100.0


def cumulative(anchor, saves):
    """Per row, how far the last save sits from the anchor, and how it got there.

    `anchor` and each of `saves` are `parse()` results with a 'date' key, oldest save first. A save
    whose rows cost the same per iteration as the previous one's still counts as a save (a restamp, a header change) but
    moves nothing. Returns {row: {...}} for every row the last state carries.
    """
    states = [anchor] + list(saves)
    last = states[-1]
    restamp = restamp_terms(last['why'])
    out = {}
    for name, now in last['rows'].items():
        base, since = None, None
        steps, prev = [], None
        for state in states:
            cell = state['rows'].get(name)
            if cell is None:
                prev = None
                continue
            if base is None:
                base, since = cell, state
            elif prev is not None and per_iter(cell) != per_iter(prev):
                steps.append((pct(prev, cell), state))
            prev = cell
        floor = pct(base, now)
        term = restamp.get(name, 0.0)
        largest = max(steps, key=lambda s: abs(s[0]), default=None)
        out[name] = {
            'since': since['date'],
            'at_anchor': since is anchor,
            'floor': floor,
            'restamp': term,
            'combined': compound(floor, term),
            'moves': len(steps),
            'largest': largest[0] if largest else 0.0,
            'largest_date': largest[1]['date'] if largest else '',
            'tripped': sum(1 for s, _ in steps if abs(s) > TRIPWIRE_PCT),
        }
    return out


def save_summary(prev, save):
    """(rows moved, largest move as (row, percent) or None) between two consecutive states."""
    moved = []
    for name, cell in save['rows'].items():
        before = prev['rows'].get(name)
        if before is not None and per_iter(before) != per_iter(cell):
            moved.append((name, pct(before, cell)))
    largest = max(moved, key=lambda m: abs(m[1]), default=None)
    return len(moved), largest


def weekly(result):
    """The weekly series' cells for one architecture: largest upward combined drift, rows past 5%."""
    ups = [r['combined'] for r in result.values()]
    return (round(max([0.0] + ups), 2),
            sum(1 for c in ups if c > WATCH_PCT))


# --- reading history ----------------------------------------------------------------------------

def _git(*args):
    return subprocess.run(['git', *args], capture_output=True, check=True).stdout.decode()


@functools.lru_cache(maxsize=None)
def _show(rev, paths):
    for path in paths:
        out = subprocess.run(['git', 'show', '%s:%s' % (rev, path)], capture_output=True)
        if out.returncode == 0:
            return out.stdout.decode()
    return None


def _utc_date(iso):
    return datetime.fromisoformat(iso).astimezone(timezone.utc).strftime('%Y-%m-%d')


@functools.lru_cache(maxsize=None)
def _subject(sha):
    """A save's commit subject, or the pull request title a merge carries in its body."""
    subject, _, body = _git('show', '-s', '--format=%s%n%b', sha).partition('\n')
    if subject.startswith('Merge'):
        for line in body.split('\n'):
            if line.strip():
                return line.strip()
    return subject


def history(arch, rev='HEAD'):
    """(anchor, [save, ...]) for one architecture, or None where `rev` predates the anchor.

    Read from git with nothing checked out: first-parent commits after the anchor that touched the
    file under any of its names, skipping any whose text is unchanged."""
    sha, path = ANCHORS[arch]
    if subprocess.run(['git', 'merge-base', '--is-ancestor', sha, rev]).returncode != 0:
        return None
    anchor = parse(_show(sha, (path,)))
    anchor.update(commit=sha[:9], date=_utc_date(_git('show', '-s', '--format=%cI', sha).strip()),
                  subject='the anchor')
    log = _git('log', '--first-parent', '--format=%H %cI', '%s..%s' % (sha, rev), '--',
               *PATHS[arch])
    saves, prev_text = [], _show(sha, (path,))
    for line in reversed([entry for entry in log.split('\n') if entry.strip()]):
        commit, when = line.split()
        text = _show(commit, PATHS[arch])
        if text is None or text == prev_text:
            continue
        state = parse(text)
        state.update(commit=commit[:9], date=_utc_date(when), subject=_subject(commit))
        saves.append(state)
        prev_text = text
    return anchor, saves


def series_cells(rev):
    """{column: cell} for `script/metrics`' baseline-drift measure at `rev`."""
    out = {}
    for arch in ARCHES:
        found = history(arch, rev)
        if found is None:
            continue   # the week predates this architecture's anchor: absent, not zero
        top, over = weekly(cumulative(*found))
        out['baseline_drift_max_pct_' + arch] = '%.2f' % top
        out['baseline_drift_rows_over_5pct_' + arch] = over
    return out


# --- the report ---------------------------------------------------------------------------------

def _p(value):
    return '%+.2f%%' % value


def _cell_text(text):
    return text.replace('|', '\\|').replace('\n', ' ')


# The report's table headings. helpers/prose_ratchet.py skips a table under any heading in its
# GENERATED_TABLES, because nobody can cut words a script writes, and the selftest fails if one of
# these is missing there. Fixed strings for that reason: an anchor date in a heading would break it.
def table_headings(arch):
    return ('## %s: drift per row since the anchor' % arch,
            '## %s: every save since the anchor, and its reason' % arch)


TABLE_HEADINGS = tuple(h for arch in ARCHES for h in table_headings(arch))


def render(reports, stamp):
    """The appendix page. `reports` is [(arch, anchor, saves)]; `stamp` names the tree it read."""
    anchors = ', '.join('%s `%s` (%s)' % (arch, anchor['commit'], anchor['date'])
                        for arch, anchor, _saves in reports)
    out = [
        '# Cumulative icount drift since the anchor',
        '',
        '*Generated by `script/metrics` from git history (helpers/baseline_drift.py), read %s. '
        'Do not edit it by hand. An appendix to [notes/project-metrics.md](../project-metrics.md), '
        'for milestone 415 item 3. calef ruled on 2026-09-26 that this is a report and not a gate '
        '(§190).*' % stamp,
        '',
        'Every figure is per iteration, against a fixed anchor rather than the last floor. The '
        'anchors are the 2026-09-15 audit\'s: %s. `floor` is the committed number. `restamp` is '
        'the compiler term `--restamp` has carried since the last `--save`, and `combined` is both. '
        'The tripwire is 10%% per save. The weekly chart counts rows past 5%% combined.' % anchors,
    ]
    for arch, anchor, saves in reports:
        result = cumulative(anchor, saves)
        rows_heading, saves_heading = table_headings(arch)
        out += ['', rows_heading, '',
                '| row | since | floor | restamp | combined | saves that moved it | '
                'largest step | steps over 10% |',
                '|---|---|---:|---:|---:|---:|---|---:|']
        for name, r in sorted(result.items(), key=lambda kv: -kv[1]['combined']):
            since = 'anchor' if r['at_anchor'] else r['since']
            step = '%s (%s)' % (_p(r['largest']), r['largest_date']) if r['moves'] else '-'
            out.append('| `%s` | %s | %s | %s | %s | %d | %s | %d |'
                       % (name, since, _p(r['floor']), _p(r['restamp']), _p(r['combined']),
                          r['moves'], step, r['tripped']))
        out += ['', saves_heading, '',
                '| date | commit | rows moved | largest | reason |', '|---|---|---:|---|---|']
        prev = anchor
        for save in saves:
            moved, largest = save_summary(prev, save)
            big = '`%s` %s' % (largest[0], _p(largest[1])) if largest else '-'
            if save['why']:
                reason = '<br>'.join('`# why:` ' + _cell_text(w) for w in save['why'])
            else:
                reason = 'no `# why:` ledger; commit: ' + _cell_text(save['subject'])
            out.append('| %s | `%s` | %d | %s | %s |'
                       % (save['date'], save['commit'], moved, big, reason))
            prev = save
    return '\n'.join(out) + '\n'


def status(reports, week):
    """One line for the page: each architecture's largest combined drift, and rows past 5%."""
    parts, over = [], 0
    for arch, anchor, saves in reports:
        result = cumulative(anchor, saves)
        name, top = max(((n, r['combined']) for n, r in result.items()), key=lambda x: x[1])
        over += sum(1 for r in result.values() if r['combined'] > WATCH_PCT)
        parts.append('%s `%s` %s' % (arch, name, _p(top)))
    return '%s: %s; %d rows past 5%%.' % (week, ', '.join(parts), over)


def report(rev='HEAD'):
    reports = [(arch, *history(arch, rev)) for arch in ARCHES]
    sha = _git('rev-parse', '--short=9', rev).strip()
    return reports, 'at `%s`' % sha


# --- selftest -----------------------------------------------------------------------------------

def selftest():
    """The arithmetic against fixtures: each case is a way a wrong reader would mislead."""
    import prose_ratchet
    def state(date, body, why=()):
        text = ''.join('# why: %s\n' % w for w in why) + '# header\n' + body
        s = parse(text)
        s.update(date=date, commit=date, subject='s')
        return s

    anchor = state('d0', 'a 1000 10\nb 500 5\ngone 1 1\n')
    # An iteration-count change with the same per-iteration cost reads as zero.
    s1 = state('d1', 'a 2000 20\nb 525 5\n')
    # Two ~4% steps on `a`: neither trips, together they are +8.15%.
    s2 = state('d2', 'a 2080 20\nb 525 5\nnew 100 1\n')
    s3 = state('d3', 'a 2163 20\nb 525 5\nnew 150 1\n')
    same = state('d4', 'a 2163 20\nb 525 5\nnew 150 1\n',
                 why=['restamped; %s a +1.000%%, new -0.500%%' % RESTAMP_MARKER])
    r = cumulative(anchor, [s1, s2, s3, same])
    cases = [
        ('parse skips comments and the indented header',
         parse('# why: x\n   # Updating\nrow 10 2\nbad line\n'),
         {'rows': {'row': (10, 2)}, 'why': ['x']}),
        ('iteration count change is zero', round(pct((1000, 10), (2000, 20)), 9), 0.0),
        ('per-iteration, not per-total', round(pct((1000, 10), (1100, 10)), 6), 10.0),
        ('compounding', round(compound(10.0, 10.0), 6), 21.0),
        ('steps accumulate past the tripwire', round(r['a']['floor'], 2), 8.15),
        ('no step tripped', r['a']['tripped'], 0),
        ('moves counted, an unchanged save is not a move', r['a']['moves'], 2),
        ('a row absent at the anchor starts at its birth', (r['new']['at_anchor'], r['new']['since']),
         (False, 'd2')),
        ('a born row measures from its birth', round(r['new']['floor'], 6), 50.0),
        ('a removed row is not reported', 'gone' in r, False),
        ('restamp term read from the ledger', r['a']['restamp'], 1.0),
        ('restamp compounds onto the floor', round(r['a']['combined'], 4),
         round(compound(r['a']['floor'], 1.0), 4)),
        ('a row the ledger omits has no term', r['b']['restamp'], 0.0),
        ('the last ledger line wins',
         restamp_terms(['%s a +1.000%%' % RESTAMP_MARKER, '%s a +2.500%%' % RESTAMP_MARKER]),
         {'a': 2.5}),
        ('an empty ledger is none', restamp_terms(['%s none' % RESTAMP_MARKER]), {}),
        ('largest step is signed', round(cumulative(state('x', 'a 100 1\n'),
                                                    [state('y', 'a 80 1\n'),
                                                     state('z', 'a 84 1\n')])['a']['largest'], 6),
         -20.0),
        ('a step over 10% is counted', cumulative(state('x', 'a 100 1\n'),
                                                  [state('y', 'a 111 1\n')])['a']['tripped'], 1),
        ('weekly takes upward combined only', weekly({'a': {'combined': -9.0},
                                                      'b': {'combined': 6.0},
                                                      'c': {'combined': 2.0}}), (6.0, 1)),
        ('weekly never reports below zero', weekly({'a': {'combined': -3.0}}), (0.0, 0)),
        ('save summary', save_summary(anchor, s1), (1, ('b', 5.000000000000004))),
        ('the gate skips every generated table',
         sorted(set(TABLE_HEADINGS) - prose_ratchet.GENERATED_TABLES), []),
    ]
    failed = [(name, got, want) for name, got, want in cases if got != want]
    for name, got, want in failed:
        print('baseline drift selftest: %s: got %r, want %r' % (name, got, want), file=sys.stderr)
    if failed:
        return 1
    print('baseline drift selftest: %d cases pass' % len(cases))
    return 0


# The 2026-09-15 audit's five cumulative figures (notes/benchmarks/baseline-save-audit.md), at the
# merge that carried the last save it walked. Reproducing them from the same anchors is the check
# that this module and the audit mean the same arithmetic. It needs full history, so it is a mode
# of its own rather than part of `--selftest`, which script/lint runs on whatever clone CI has.
AUDIT_REV = '770f44983'
AUDIT = [('riscv64', 'ctx_switch', 10.78), ('aarch64', 'yield_switch', 9.16),
         ('riscv64', 'ipc_rtt', 9.25), ('aarch64', 'ipc_rtt', 8.73),
         ('x86_64', 'yield_switch', 9.94)]


def audit():
    bad = 0
    for arch, row, want in AUDIT:
        got = round(cumulative(*history(arch, AUDIT_REV))[row]['floor'], 2)
        print('%s %s: %+.2f%% (audit: %+.2f%%)' % (arch, row, got, want))
        bad += got != want
    return 1 if bad else 0


def main(argv):
    if argv and argv[0] == '--selftest':
        return selftest()
    if argv and argv[0] == '--audit':
        return audit()
    if argv:
        print(__doc__.split('\n\n')[1], file=sys.stderr)
        return 2
    sys.stdout.write(render(*report()))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
