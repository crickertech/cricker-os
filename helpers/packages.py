"""Package boundaries: every tracked path has a home, and every crate and program has a package.

Milestone 611 (every program and crate belongs to a package, and every package has a home). calef
ruled on 2026-09-27, on pull request #1389 (its decision's fork 3), that the tree stays one repository for
now "with package boundaries drawn and enforced inside it by lint check", and that the end state is
everything moving out, with no default of staying. This file is that lint check, and the count the
weekly metrics page draws from it.

    python3 helpers/packages.py --check      # what script/lint runs
    python3 helpers/packages.py --selftest   # planted violations, no tree, no git
    python3 helpers/packages.py --table        # the table notes/package-boundaries.md carries
    python3 helpers/packages.py --write-table  # write it there, between the markers
    python3 helpers/packages.py --summary    # one line of counts

Name: provisional, minted by milestone 611's lane on 2026-09-27. So are the file formats below, the
four kinds, every package name and every home. calef names things and rules formats; the note
notes/package-boundaries.md lists the questions.

**Two declarations, because calef separated them (2026-09-27): "Everything in the tree may not be
in a package, but it may be in a repo."**

1. `packages/<name>.package`, one per package: what releases, updates and is trusted together.
   Directives, one per line, the grammar `packages/*.recipe` already uses:

       name <name>                 required; equals the file's stem
       kind base|optional|sdk|test required, no default
       home <repo> provisional     required, no default; or `home <repo> ratified <date>`,
                                   or `home undecided <reason>`
       crate <cargo package>       a member crate; claims its directory
       interface <cargo package>   a member crate other packages may link without declaring it
       program <binary>            a member program; claims its source file
       path <prefix>               a member path (the std overlay, a feature's note, a recipe)
       depends <package>           a declared dependency; its crates may be linked
       exception <date> <member> <crate> <reason>
                                   one recorded, dated link across a boundary the rules refuse

2. `packages/homes`, for tracked paths in no package: project records, CI, scripts, bench data.
   `path <prefix> <home spec>` per line, the home spec being what follows `home` above.

**The rules, each a rule over the tree and none a list of names:**

- Every tracked path has exactly one home. A path's claim is the longest prefix naming it, among
  every package member and every `packages/homes` line; two claims of the same prefix fail, and so
  does a path nothing claims. A package's paths take the package's home.
- Every Cargo package in the tree is a member of exactly one package, and so is every binary target.
  A crate's binaries go with it, unless any of them is placed in another package: then every one
  of its binaries needs a `program` line (`components` today), so a new program in a crate that
  hosts several packages is placed on purpose and never by default.
- A link across a package boundary is allowed to an `interface` crate, to any crate of a package
  the linker `depends` on, or under a dated `exception`. Anything else is reaching into another
  package's internals. Normal and build dependencies are checked; dev-dependencies are not, because
  they never ship (the note's BUGS says what that leaves open).
- A crate whose binaries sit in several packages is checked per program instead, by the crate names
  each program's source mentions, since its manifest's dependency list is the union of all of them.
- Nothing but a `test` package depends on a `test` package, and a `base` package does not depend on
  an `optional` one.
- An exception whose link no longer exists fails, so the list can only shrink by being fixed.

Dependencies come from each `Cargo.toml` read with `tomllib`, not from `cargo metadata`, so the
same code runs over a historical commit for `script/metrics` without a checkout or a network.
"""

import os
import re
import subprocess
import sys
import tomllib

KINDS = ('base', 'optional', 'sdk', 'test')
HOMES_FILE = 'packages/homes'
PACKAGE_DIR = 'packages/'
PACKAGE_SUFFIX = '.package'
DATE = re.compile(r'^\d{4}-\d{2}-\d{2}$')
REPO = re.compile(r'^[a-z0-9][a-z0-9._/-]*$')


# ---------------------------------------------------------------------------------------------
# Parsing. Pure functions over text, so the selftest needs no tree.

def parse_home(words, where):
    """A home spec: `<repo> provisional`, `<repo> ratified <date>`, or `undecided <reason>`."""
    if not words:
        return None, [f'{where}: a home is required, even when the answer is "undecided"']
    if words[0] == 'undecided':
        if len(words) < 2:
            return None, [f'{where}: an undecided home says why, in a few words']
        return {'repo': None, 'status': 'undecided', 'reason': ' '.join(words[1:])}, []
    repo = words[0]
    if not REPO.match(repo):
        return None, [f'{where}: {repo!r} is not a repository name']
    if words[1:] == ['provisional']:
        return {'repo': repo, 'status': 'provisional'}, []
    if len(words) == 3 and words[1] == 'ratified' and DATE.match(words[2]):
        return {'repo': repo, 'status': 'ratified', 'date': words[2]}, []
    return None, [f'{where}: a home is `<repo> provisional`, `<repo> ratified <date>` '
                  'or `undecided <reason>`']


def parse_package(stem, text):
    where = f'{PACKAGE_DIR}{stem}{PACKAGE_SUFFIX}'
    pkg = {'file': where, 'name': None, 'kind': None, 'home': None, 'crates': [],
           'interfaces': set(), 'programs': [], 'paths': [], 'depends': [], 'exceptions': []}
    errors = []
    for number, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith('#'):
            continue
        at = f'{where}:{number}'
        directive, *words = line.split()
        if directive == 'name':
            pkg['name'] = ' '.join(words)
        elif directive == 'kind':
            if words not in ([k] for k in KINDS):
                errors.append(f'{at}: kind is one of {", ".join(KINDS)}')
            else:
                pkg['kind'] = words[0]
        elif directive == 'home':
            pkg['home'], errs = parse_home(words, at)
            errors += errs
        elif directive in ('crate', 'interface', 'program', 'path', 'depends'):
            if len(words) != 1:
                errors.append(f'{at}: {directive} takes one word')
                continue
            if directive == 'interface':
                pkg['crates'].append(words[0])
                pkg['interfaces'].add(words[0])
            else:
                pkg[{'crate': 'crates', 'program': 'programs', 'path': 'paths',
                     'depends': 'depends'}[directive]].append(words[0])
        elif directive == 'exception':
            if len(words) < 4 or not DATE.match(words[0]):
                errors.append(f'{at}: exception is `<date> <member> <crate> <reason>`')
                continue
            pkg['exceptions'].append({'date': words[0], 'member': words[1], 'crate': words[2],
                                      'reason': ' '.join(words[3:]), 'at': at})
        else:
            errors.append(f'{at}: unknown directive {directive}')
    if pkg['name'] != stem:
        errors.append(f'{where}: name must be {stem!r}, the file\'s stem')
    for field in ('kind', 'home'):
        if pkg[field] is None and not any(field in e for e in errors):
            errors.append(f'{where}: no {field} (required, with no default)')
    if not (pkg['crates'] or pkg['programs'] or pkg['paths']):
        errors.append(f'{where}: a package with no members')
    return pkg, errors


def parse_homes(text):
    claims, errors = [], []
    for number, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith('#'):
            continue
        at = f'{HOMES_FILE}:{number}'
        words = line.split()
        if words[0] != 'path' or len(words) < 3:
            errors.append(f'{at}: a line is `path <prefix> <home spec>`')
            continue
        home, errs = parse_home(words[2:], at)
        errors += errs
        if home:
            claims.append((words[1], home, at))
    return claims, errors


# ---------------------------------------------------------------------------------------------
# The Cargo side: what crates exist, which binaries they build, what they link.

DEP_TABLES = ('dependencies', 'build-dependencies')


def cargo_crates(texts, tracked):
    """Every Cargo package in the tree, from its manifest: directory, binaries, path links."""
    tracked_set = set(tracked)
    by_dir, crates, errors = {}, {}, []
    for path in sorted(p for p in texts if p == 'Cargo.toml' or p.endswith('/Cargo.toml')):
        try:
            manifest = tomllib.loads(texts[path])
        except tomllib.TOMLDecodeError as e:
            errors.append(f'{path}: {e}')
            continue
        if 'package' not in manifest:
            continue
        directory = os.path.dirname(path)
        name = manifest['package']['name']
        bins = {}
        for b in manifest.get('bin', []):
            bins[b['name']] = os.path.normpath(os.path.join(directory, b.get('path', f'src/bin/{b["name"]}.rs')))
        main = os.path.join(directory, 'src/main.rs') if directory else 'src/main.rs'
        if not bins and manifest['package'].get('autobins', True) and main in tracked_set:
            bins[name] = main
        links = []
        tables = [manifest]
        tables += [t for t in manifest.get('target', {}).values() if isinstance(t, dict)]
        for table in tables:
            for kind in DEP_TABLES:
                for dep, spec in table.get(kind, {}).items():
                    if isinstance(spec, dict) and 'path' in spec:
                        target = os.path.normpath(os.path.join(directory, spec['path']))
                        links.append((dep, spec.get('package', dep), target, kind))
        crates[name] = {'dir': directory, 'bins': bins, 'links': links, 'manifest': path}
        by_dir[directory] = name
    for crate in crates.values():
        crate['links'] = [(alias, by_dir.get(target, real), kind)
                          for alias, real, target, kind in crate['links']]
    return crates, errors


PATH_MODULE = re.compile(r'#\[path\s*=\s*"([^"]+)"\]')


COMMENT = re.compile(r'//[^\n]*|/\*.*?\*/', re.S)


def program_mentions(source_path, texts, lib_names):
    """The crates one program's source names in code (`name::`, `use name`, `extern crate name`).

    Comments are stripped first, because this tree's comments name other crates constantly.
    `lib_names` is only what the program's own crate declares, so a local module that happens
    to share a crate's name is not mistaken for it unless the manifest also links that crate.
    """
    seen, queue, found = set(), [source_path], set()
    while queue:
        path = queue.pop()
        if path in seen or path not in texts:
            continue
        seen.add(path)
        raw = texts[path]
        text = COMMENT.sub('', raw)
        for word in re.findall(r'(?<![A-Za-z0-9_:])([a-z_][a-z0-9_]*)\s*::', text):
            if word in lib_names:
                found.add(lib_names[word])
        for word in re.findall(r'\b(?:use|extern crate)\s+([a-z_][a-z0-9_]*)', text):
            if word in lib_names:
                found.add(lib_names[word])
        for rel in PATH_MODULE.findall(raw):
            queue.append(os.path.normpath(os.path.join(os.path.dirname(path), rel)))
    return found


# ---------------------------------------------------------------------------------------------
# The evaluation.

def evaluate(tracked, texts):
    """Everything the gate and the metric need, from the tracked paths and the texts it reads.

    `texts` holds at least every tracked `Cargo.toml`, every `packages/*.package`, `packages/homes`
    and the source of every program whose crate's binaries are split across packages.
    """
    errors = []
    packages = {}
    for path in sorted(texts):
        if path.startswith(PACKAGE_DIR) and path.endswith(PACKAGE_SUFFIX) and path.count('/') == 1:
            stem = path[len(PACKAGE_DIR):-len(PACKAGE_SUFFIX)]
            pkg, errs = parse_package(stem, texts[path])
            errors += errs
            packages[stem] = pkg
    home_claims, errs = parse_homes(texts.get(HOMES_FILE, ''))
    errors += errs
    crates, errs = cargo_crates(texts, tracked)
    errors += errs

    # Membership: each crate and each program in exactly one package.
    crate_pkg, program_pkg = {}, {}
    for name, pkg in packages.items():
        for c in pkg['crates']:
            if c not in crates:
                errors.append(f'{pkg["file"]}: crate {c} is not a Cargo package in this tree')
            elif c in crate_pkg:
                errors.append(f'crate {c} is in two packages, {crate_pkg[c]} and {name}')
            else:
                crate_pkg[c] = name
    all_bins = {b: (c, p) for c, info in crates.items() for b, p in info['bins'].items()}
    for name, pkg in packages.items():
        for b in pkg['programs']:
            if b not in all_bins:
                errors.append(f'{pkg["file"]}: program {b} is not a binary target in this tree')
            elif b in program_pkg:
                errors.append(f'program {b} is in two packages, {program_pkg[b]} and {name}')
            else:
                program_pkg[b] = name
    for c in sorted(set(crates) - set(crate_pkg)):
        errors.append(f'crate {c} ({crates[c]["dir"] or "."}) is in no package')
    for c, info in crates.items():
        placed = {program_pkg[b] for b in info['bins'] if b in program_pkg}
        if placed - {crate_pkg.get(c)}:
            for b in sorted(set(info['bins']) - set(program_pkg)):
                errors.append(f'program {b} ({info["bins"][b]}) is in no package; a crate whose '
                              'programs sit in several packages names each in a `program` line')
        else:
            for b in info['bins']:
                program_pkg.setdefault(b, crate_pkg.get(c))

    # Homes: every tracked path, one claim, the longest prefix wins.
    claims = {}

    def claim(prefix, owner, home, at):
        if prefix in claims:
            errors.append(f'{at}: {prefix} is already claimed by {claims[prefix][2]}')
        else:
            claims[prefix] = (owner, home, at)

    for name, pkg in packages.items():
        for c in pkg['crates']:
            if c in crates:
                claim(crates[c]['dir'] + '/' if crates[c]['dir'] else crates[c]['manifest'],
                      name, pkg['home'], pkg['file'])
        for b in pkg['programs']:
            if b in all_bins:
                claim(all_bins[b][1], name, pkg['home'], pkg['file'])
        for p in pkg['paths']:
            claim(p, name, pkg['home'], pkg['file'])
        # A package's own definition travels with it, so it is a member without saying so.
        claim(pkg['file'], name, pkg['home'], pkg['file'])
    for prefix, home, at in home_claims:
        claim(prefix, None, home, at)

    def owner_of(path):
        best = None
        for prefix in _prefixes(path):
            if prefix in claims:
                best = prefix
        return best

    used, unclaimed, file_home = set(), [], {}
    for path in tracked:
        prefix = owner_of(path)
        if prefix is None:
            unclaimed.append(path)
            continue
        used.add(prefix)
        file_home[path] = claims[prefix]
    for prefix, (_o, _h, at) in claims.items():
        if prefix not in used:
            errors.append(f'{at}: {prefix} claims no tracked path, or only paths a longer claim holds')
    if unclaimed:
        errors.append(f'{len(unclaimed)} tracked path(s) have no home, for example '
                      + ', '.join(unclaimed[:5]) + ' (claim them in a package or packages/homes)')

    # Links across a boundary.
    edges = []   # (package, member, crate linked)
    for c, info in crates.items():
        own = crate_pkg.get(c)
        split = {program_pkg.get(b) for b in info['bins']} - {own}
        if split:
            for b, src in info['bins'].items():
                declared = {alias.replace('-', '_'): dep for alias, dep, _k in info['links']}
                for dep in program_mentions(src, texts, declared) - {c}:
                    edges.append((program_pkg.get(b), b, dep))
        else:
            for _alias, dep, _kind in info['links']:
                if dep != c:
                    edges.append((own, c, dep))
    exceptions_used = set()
    violations = []
    for pkg_name, member, dep in sorted(set(e for e in edges if e[0])):
        target = crate_pkg.get(dep)
        if target is None or target == pkg_name:
            continue
        pkg = packages[pkg_name]
        if dep in packages[target]['interfaces'] or target in pkg['depends']:
            continue
        hit = [x for x in pkg['exceptions'] if x['member'] == member and x['crate'] == dep]
        if hit:
            exceptions_used.add(hit[0]['at'])
            continue
        violations.append(f'{pkg_name}: {member} links {dep}, an internal crate of {target} '
                          f'(make it an interface, declare `depends {target}`, or record an exception)')
    errors += violations
    for pkg in packages.values():
        for x in pkg['exceptions']:
            if x['at'] not in exceptions_used:
                errors.append(f'{x["at"]}: exception for {x["member"]} -> {x["crate"]} names a '
                              'link that is gone or allowed; delete it')
        for d in pkg['depends']:
            if d not in packages:
                errors.append(f'{pkg["file"]}: depends on {d}, which is not a package')
                continue
            dk, k = packages[d]['kind'], pkg['kind']
            if dk == 'test' and k != 'test':
                errors.append(f'{pkg["file"]}: a {k} package depends on the test package {d}')
            if k == 'base' and dk == 'optional':
                errors.append(f'{pkg["file"]}: a base package depends on the optional package {d}')

    # Where things live now: a package is still here while any path it claims is tracked here.
    in_repo = {owner for (owner, _h, _a) in file_home.values() if owner}
    path_status = {}
    for (_o, home, _a) in file_home.values():
        if home:
            path_status[home['status']] = path_status.get(home['status'], 0) + 1
    return {'errors': errors, 'packages': packages, 'crates': crates, 'crate_pkg': crate_pkg,
            'program_pkg': program_pkg, 'in_repo': in_repo, 'unclaimed': unclaimed,
            'path_status': path_status, 'tracked': len(tracked),
            'violations': len(violations)}


def _prefixes(path):
    parts = path.split('/')
    for i in range(1, len(parts)):
        yield '/'.join(parts[:i]) + '/'
    yield path


def counts(result):
    """The metric's cells. A package still here is split by where its home stands; one that has
    left is counted once, since leaving is the end state and its home is by then decided."""
    pk = result['packages']
    here = [p for n, p in pk.items() if n in result['in_repo']]
    status = lambda s: sum(1 for p in here if p['home'] and p['home']['status'] == s)
    paths = result['path_status']
    return {'packages_total': len(pk),
            'packages_base': sum(1 for p in pk.values() if p['kind'] == 'base'),
            'packages_home_undecided': status('undecided'),
            'packages_home_provisional': status('provisional'),
            'packages_home_ratified': status('ratified'),
            'packages_in_this_repo': len(here),
            'packages_moved_out': len(pk) - len(here),
            'paths_home_undecided': paths.get('undecided', 0),
            'paths_home_provisional': paths.get('provisional', 0),
            'paths_home_ratified': paths.get('ratified', 0),
            'paths_unclaimed': len(result['unclaimed']),
            'package_exceptions': sum(len(p['exceptions']) for p in pk.values())}


# ---------------------------------------------------------------------------------------------
# Reading a tree: the working tree for the gate, a {path: text} dictionary for script/metrics.

def wanted(path):
    """The texts evaluate() reads. script/metrics widens its blob filter with this."""
    return (path == 'Cargo.toml' or path.endswith('/Cargo.toml') or path == HOMES_FILE
            or (path.startswith(PACKAGE_DIR) and path.endswith(PACKAGE_SUFFIX))
            or path.endswith('.rs'))


def working_tree(root='.'):
    tracked = subprocess.run(['git', 'ls-files', '-z'], cwd=root, capture_output=True,
                             check=True).stdout.decode().split('\0')
    tracked = [p for p in tracked if p]
    texts = {}
    for p in tracked:
        if wanted(p) and os.path.isfile(os.path.join(root, p)):
            with open(os.path.join(root, p), encoding='utf-8', errors='replace') as f:
                texts[p] = f.read()
    return tracked, texts


def home_text(home):
    if home is None:
        return '(none)'
    if home['status'] == 'undecided':
        return f'undecided: {home["reason"]}'
    if home['status'] == 'ratified':
        return f'`{home["repo"]}` (ratified {home["date"]})'
    return f'`{home["repo"]}` (provisional)'


def table(result):
    rows = ['| package | kind | members | allowed dependencies | home |', '|---|---|---|---|---|']
    for name, p in sorted(result['packages'].items(), key=lambda kv: (KINDS.index(kv[1]['kind'] or 'test'), kv[0])):
        crates = [f'{c}*' if c in p['interfaces'] else c for c in p['crates']]
        progs = sorted(b for b, owner in result['program_pkg'].items() if owner == name)
        members = []
        if crates:
            members.append('crates: ' + ', '.join(f'`{c}`' for c in sorted(crates)))
        if len(progs) > 12:
            members.append(f'programs: {len(progs)}, too many to list here')
        elif progs:
            members.append('programs: ' + ', '.join(f'`{b}`' for b in progs))
        if p['paths']:
            members.append('paths: ' + ', '.join(f'`{x}`' for x in p['paths']))
        deps = ', '.join(f'`{d}`' for d in p['depends']) or 'interfaces only'
        if p['exceptions']:
            deps += f'; {len(p["exceptions"])} dated exception(s)'
        rows.append(f'| `{name}` | {p["kind"]} | {"; ".join(members)} | {deps} | '
                    f'{home_text(p["home"])} |')
    return '\n'.join(rows)


TABLE_BEGIN = '<!-- package table: helpers/packages.py --table writes this -->'
TABLE_END = '<!-- end of package table -->'
NOTE = 'notes/package-boundaries.md'


def check(root='.'):
    tracked, texts = working_tree(root)
    result = evaluate(tracked, texts)
    errors = list(result['errors'])
    note = os.path.join(root, NOTE)
    if os.path.exists(note):
        with open(note, encoding='utf-8') as f:
            text = f.read()
        if TABLE_BEGIN in text and TABLE_END in text:
            current = text.split(TABLE_BEGIN, 1)[1].split(TABLE_END, 1)[0].strip()
            if current != table(result).strip():
                errors.append(f'{NOTE}: the package table is stale; '
                              'run python3 helpers/packages.py --write-table')
        else:
            errors.append(f'{NOTE}: the package table markers are missing')
    else:
        errors.append(f'{NOTE} is missing; it carries the table calef ratifies')
    for e in errors:
        print(f'packages: {e}', file=sys.stderr)
    c = counts(result)
    if errors:
        return 1
    print(f'packages: {c["packages_total"]} packages ({c["packages_base"]} base), every crate and '
          f'program in one; {result["tracked"]} tracked paths, every one with a home; '
          f'undecided homes: {c["packages_home_undecided"]} packages, {c["paths_home_undecided"]} '
          f'paths; {c["packages_home_provisional"]} provisional; '
          f'{c["package_exceptions"]} dated exceptions')
    return 0


# ---------------------------------------------------------------------------------------------
# The selftest: a small tree that passes, and one planted violation per rule.

def _fixture():
    toml = lambda name, deps='', extra='': (f'[package]\nname = "{name}"\nversion = "0.1.0"\n'
                                            f'{extra}\n[dependencies]\n{deps}')
    texts = {
        'Cargo.toml': '[workspace]\nmembers = []\n',
        'crates/proto/Cargo.toml': toml('proto'),
        'crates/lib/Cargo.toml': toml('lib'),
        'crates/tool/Cargo.toml': toml('tool', 'proto = { path = "../proto" }\n'
                                       'lib = { path = "../lib" }\n'),
        'host/Cargo.toml': toml('host', 'lib = { path = "../crates/lib" }\n',
                                '[[bin]]\nname = "a"\npath = "src/a.rs"\n'
                                '[[bin]]\nname = "b"\npath = "src/b.rs"\n'),
        'host/src/a.rs': 'use proto::X;\n',
        'host/src/b.rs': 'fn main() { lib::go(); }\n',
        'crates/tool/src/main.rs': '',
        'packages/homes': 'path Cargo.toml undecided the workspace root\npath notes/ records provisional\n'
                          'path packages/homes records provisional\n',
        'packages/contracts.package': 'name contracts\nkind sdk\nhome sdk provisional\ninterface proto\n',
        'packages/toolbox.package': 'name toolbox\nkind base\nhome toolbox provisional\n'
                                    'crate lib\ncrate tool\ncrate host\nprogram b\n',
        'packages/extra.package': 'name extra\nkind optional\nhome undecided nobody asked yet\n'
                                  'program a\npath notes/extra.md\n',
    }
    tracked = sorted(set(texts) | {'notes/a.md', 'notes/extra.md', 'crates/lib/src/lib.rs'})
    return tracked, texts


def selftest():
    tracked, texts = _fixture()
    good = evaluate(tracked, texts)
    assert good['errors'] == [], good['errors']
    c = counts(good)
    assert (c['packages_total'], c['packages_home_undecided'], c['paths_home_undecided']) == (3, 1, 4), c
    assert good['program_pkg'] == {'a': 'extra', 'b': 'toolbox', 'tool': 'toolbox'}, good['program_pkg']

    def planted(change, expect):
        t, x = _fixture()
        change(t, x)
        errs = evaluate(t, x)['errors']
        assert any(expect in e for e in errs), (expect, errs)

    # A path nobody claims.
    planted(lambda t, x: t.append('stray.txt'), 'have no home')
    # A crate in no package, and one in two.
    planted(lambda t, x: x.update({'crates/new/Cargo.toml': 'package = {name = "new"}\n'}) or
            t.append('crates/new/Cargo.toml'), 'crate new (crates/new) is in no package')
    planted(lambda t, x: x.update({'packages/extra.package': x['packages/extra.package'] + 'crate lib\n'}),
            'crate lib is in two packages')
    # A program of a split crate that nobody placed.
    planted(lambda t, x: x.update({'host/Cargo.toml': x['host/Cargo.toml'].replace(
                '[dependencies]', '[[bin]]\nname = "c"\npath = "src/c.rs"\n[dependencies]'),
                'host/src/c.rs': ''}) or t.append('host/src/c.rs'),
            'program c (host/src/c.rs) is in no package')
    # A program reaching into another package's internals, and a crate doing the same.
    planted(lambda t, x: x.update({'host/src/a.rs': 'use lib::Y;\n'}), 'extra: a links lib')
    planted(lambda t, x: x.update({'packages/contracts.package': x['packages/contracts.package']
                                   .replace('interface proto', 'crate proto')}),
            'toolbox: tool links proto, an internal crate of contracts')
    # A home missing, a kind missing, a malformed home.
    planted(lambda t, x: x.update({'packages/extra.package': 'name extra\nkind optional\nprogram a\npath notes/extra.md\n'}),
            'no home')
    planted(lambda t, x: x.update({'packages/extra.package': 'name extra\nhome undecided x\nprogram a\npath notes/extra.md\n'}),
            'no kind')
    planted(lambda t, x: x.update({'packages/homes': 'path Cargo.toml undecided\npath notes/ records provisional\n'}),
            'says why')
    # Two claims of one prefix, and a claim of nothing.
    planted(lambda t, x: x.update({'packages/homes': x['packages/homes'] + 'path notes/extra.md records provisional\n'}),
            'already claimed')
    planted(lambda t, x: x.update({'packages/homes': x['packages/homes'] + 'path gone/ records provisional\n'}),
            'claims no tracked path')
    # An exception that excuses a link which is not there any more.
    planted(lambda t, x: x.update({'packages/extra.package': x['packages/extra.package']
                                   + 'exception 2026-09-27 a lib a stale excuse\n'}),
            'names a link that is gone')
    # Kinds: a base package depending on an optional one; a non-test on a test package.
    planted(lambda t, x: x.update({'packages/toolbox.package': x['packages/toolbox.package'] + 'depends extra\n'}),
            'base package depends on the optional package extra')
    planted(lambda t, x: x.update({'packages/extra.package': x['packages/extra.package'].replace('optional', 'test'),
                                   'packages/toolbox.package': x['packages/toolbox.package'] + 'depends extra\n'}),
            'depends on the test package extra')

    # And the ways out that are not weakening: an exception that names the link, and a declaration.
    t, x = _fixture()
    x['host/src/a.rs'] = 'use lib::Y;\n'
    x['packages/extra.package'] += 'exception 2026-09-27 a lib the reason goes here\n'
    assert evaluate(t, x)['errors'] == []
    print('packages: selftest passed (14 planted violations, each caught)')


def main(argv):
    if argv == ['--selftest']:
        selftest()
        return 0
    if argv == ['--check']:
        return check()
    if argv == ['--write-table']:
        tracked, texts = working_tree()
        with open(NOTE, encoding='utf-8') as f:
            text = f.read()
        head, rest = text.split(TABLE_BEGIN, 1)
        tail = rest.split(TABLE_END, 1)[1]
        with open(NOTE, 'w', encoding='utf-8') as f:
            f.write(f'{head}{TABLE_BEGIN}\n{table(evaluate(tracked, texts))}\n{TABLE_END}{tail}')
        return 0
    if argv in (['--table'], ['--summary']):
        tracked, texts = working_tree()
        result = evaluate(tracked, texts)
        if argv == ['--table']:
            print(table(result))
        else:
            print(counts(result))
        return 0
    print(__doc__.split('\n\n')[1], file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
