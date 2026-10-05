#!/usr/bin/env python3
"""Is this diff prose that no build and no test reads? If so CI skips the heavy jobs.

    helpers/prose_only.py classify <base>   # the CI gate: <base>..HEAD, one line per changed file
    helpers/prose_only.py --selftest        # script/lint: fixtures, no git

Name: provisional, minted by lane/docs-only-ci on 2026-10-05 UTC. A helper under `helpers/`,
outside `script/names`' scope, so its provenance is this paragraph. Refused `docs_only`, because
"docs" also covers a crate's README and rustdoc, which are code here, and this decides prose only.

# WHY

Every pull request ran the whole suite, Markdown or not. On 2026-10-05 twelve pull requests that
only removed `**` markers from design/ and notes/ took the hosted runner pool, and required jobs on
#1708 and #1712 were cancelled with "The job was not acquired by Runner of type hosted even after
multiple attempts". Before this file each heavy job decided "documentation only" for itself, nine
copies of one regex inside steps, so every one of them still took a runner to find out it had
nothing to do. The gate now decides once, and a prose-only change skips those jobs outright.

# THE RULE

A changed file is **prose** when it ends in `.md` and lies under `notes/`, `design/` or `briefs/`,
or at the repository root, AND nothing in the tree at the head being tested reads it. A diff is
prose-only when it is non-empty and every file in it is prose. Anything else runs everything.

"Nothing reads it" is computed from the tree on every run, never from a list. A list of consumers
somebody has to remember to extend is rung three at best and would rot the first time a lane added
an `include_str!` (calef, 2026-10-05). Every tracked file that could read something is scanned:
everything except Markdown, the prose directories themselves (their `.csv`, `.svg` and `.patch`
files are records), captured data (`.log`, `.csv`, `.tsv`) and `vendor/`. Comment lines are dropped
(`//`, `/*` or `*` anywhere, `#` outside Rust, where it opens an attribute; a `.patch` contributes
the lines it leaves in the tree). Two shapes count:

- **A file reference**: a `*.md` token used as a path. It is resolved repository-relative, relative
  to the referring file, with leading `../` stripped, and from any `notes/`, `design/` or `briefs/`
  inside it (`$root/notes/x.md`), and a changed file matching any of these is an input. This is
  what catches `include_str!`, `#[doc = include_str!(...)]`, a build script's read, xtask's
  `DOC_BUNDLES` (the notes the disk image installs and swish-check searches with `apropos`), a
  Cargo.toml `readme`, and a workflow that cats a note. A token inside a quoted string that holds
  whitespace is a sentence (`"see notes/stack.md"`) and is not counted: a message reads nothing.
  Anything the check cannot place, such as the middle line of a long Rust string, is counted, so
  it errs toward a full run.
- **A directory reference**: a quoted string that is exactly a prose directory (`"notes"`,
  `"design/roadmap"`) or a glob under one (`"design/roadmap/*.md"`). Code that names a directory
  walks it, so everything beneath it is read by the package that holds the code. In a `.rs` file
  the package is the nearest Cargo.toml's, and a prose-only run tests it (`prose_tests`): today that
  is `crates/documentation`'s corpus test, which went red on main on 2026-09-23 while the old
  predicate skipped it, and xtask. A package that cannot test on the host fails that step loudly.
  Rust in no package runs everything.

Why not the compiler's own record (cargo's dep-info, `target/**/*.d`)? It would be exact for
`include_str!`, but it exists only after a build, and the gate's purpose is to decide before any
build takes a runner. CI uploads no dep-info a gate could read. And dep-info cannot see a runtime
read, which is the larger half (DOC_BUNDLES, the corpus walk). The grep of the head tree is the
honest fallback: it errs toward running and it sees a new reference in the same diff that adds it.

The decision for every changed file is printed with the rule that made it, so a wrong skip can be
diagnosed from the gate's log.

# BUGS

- **A directory read outside Rust is logged, not acted on.** A script that walks `design/` (lint's
  roadmap, citations and decisions checks, `helpers/prose_ratchet.py`) is printed as a reader and
  does not stop the skip, because lint runs on every pull request and skipping nothing would be the
  alternative. A script that walks a prose directory and is run *only* by a job the prose path
  skips would slip through. None did on 2026-10-05; nothing checks that none does later.
- **A path built at run time from parts that are not a literal** (an environment variable, a
  `format!` whose pieces are neither a `.md` token nor a quoted prose directory) is invisible. So
  is a reader of the repository root that names it as `""` or `"."`: xtask's `apropos` does, and
  it runs in no gate.
- **A path inside a sentence-shaped string is not counted**, by the rule above, so a shell loop over
  `"notes/a.md notes/b.md"` would read both unseen. Nothing in the tree does that today.
- A script under `notes/` (two `.py` files in `notes/capability-peak-trace/`) is a record and is not
  scanned; if a gate ever ran one, what it reads would be invisible.
- `vendor/` is not scanned. Upstream code names no file of ours.
"""

import os
import re
import subprocess
import sys

PROSE_DIRS = ("notes/", "design/", "briefs/")

# This file names prose paths in its fixtures and its rule; it is not a reader of them.
SELF = "helpers/prose_only.py"

MD_TOKEN = re.compile(r"[A-Za-z0-9_./-]*[A-Za-z0-9_-]\.md\b")
DIR_LITERAL = re.compile(
    r"""(["'])(?:\.\./)*(?:\./)?/?((?:notes|design|briefs)(?:/[A-Za-z0-9_-]+)*)(?:/[^"'\s]*\*[^"'\s]*)?/?\1""")
QUOTED = re.compile(r""""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'""")


def path_tokens(line):
    """The .md tokens on a line that are used as paths rather than mentioned in a sentence.

    A token inside a quoted string that contains whitespace is a message ("see notes/stack.md"),
    and a message reads nothing. Every other token counts: a string that is only a path is data
    (`"notes/pipes.md"`, `include_str!("../x.md")`, `format!("{}/notes/x.md", root)`), and an
    unquoted one is a shell word or a line inside a string this check cannot see the start of,
    both of which are kept, because a wrong keep costs minutes and a wrong skip a red main."""
    sentences = [m.span() for m in QUOTED.finditer(line) if re.search(r"\s", m.group(0))]
    for m in MD_TOKEN.finditer(line):
        if not any(a < m.start() and m.end() < b for a, b in sentences):
            yield m.group(0)


def is_prose_path(path):
    return path.endswith(".md") and (path.startswith(PROSE_DIRS) or "/" not in path)


# Captured output and tables: read by code, never a reader of anything.
DATA = (".log", ".csv", ".tsv")


def scanned(path):
    """Is this file one whose text could read a prose file? Not prose, not data, not upstream."""
    return not path.startswith(PROSE_DIRS) and not path.endswith(".md") \
        and not path.endswith(DATA) and not path.startswith("vendor/") and path != SELF


def code_lines(path, text):
    """(line number, line) for every line that is not a whole-line comment.

    A comment is a line opening with `//`, `/*` or `*` anywhere, or `#` outside Rust, where `#`
    opens an attribute such as `#![doc = include_str!(...)]`. A patch (the falsification records,
    applied to Rust) contributes its context and added lines, judged as Rust; a removed line is
    gone from the tree the patch produces."""
    patch = path.endswith((".patch", ".diff"))
    rust = path.endswith(".rs") or patch
    for n, line in enumerate(text.splitlines(), 1):
        if patch:
            if line.startswith(("+++", "---", "@@", "diff ", "index ")) or not line[:1] in ("+", " "):
                continue
            line = line[1:]
        s = line.lstrip()
        if s.startswith(("//", "/*", "*")):
            continue
        if s.startswith("#") and not rust:
            continue
        yield n, line


def resolutions(referrer, token):
    out = set()
    t = token
    while t.startswith("./"):
        t = t[2:]
    out.add(os.path.normpath(t.lstrip("/")))
    out.add(os.path.normpath(os.path.join(os.path.dirname(referrer), token)))
    bare = t
    while bare.startswith("../"):
        bare = bare[3:]
    out.add(os.path.normpath(bare.lstrip("/")))
    # A path under a prefix the text cannot evaluate (`$root/notes/x.md`, `{}/design/y.md`).
    for m in re.finditer(r"(?:^|/)((?:notes|design|briefs)/.*)$", t):
        out.add(os.path.normpath(m.group(1)))
    return out


def package_of(path, tree):
    """The package name of the nearest Cargo.toml above `path`, or None."""
    d = os.path.dirname(path)
    while True:
        manifest = os.path.join(d, "Cargo.toml") if d else "Cargo.toml"
        text = tree.get(manifest)
        if text is not None:
            m = re.search(r'^\[package\][^\[]*?^name\s*=\s*"([^"]+)"', text, re.M | re.S)
            # A name is spliced into a `cargo test -p` line in CI, so anything but a plain
            # package name is refused here and its reader treated as in no package (run all).
            if m and re.fullmatch(r"[A-Za-z0-9_-]+", m.group(1)):
                return m.group(1)
            if m:
                return None
        if not d:
            return None
        d = os.path.dirname(d)


def scan(tree):
    """Every reference to a prose path in code: (files, dirs).

    files maps a resolved .md path to the places naming it; dirs maps a prose directory to
    (place, package-or-None) for each place that names it as a quoted literal."""
    files, dirs = {}, {}
    for path, text in tree.items():
        if not scanned(path) or text is None:
            continue
        for n, line in code_lines(path, text):
            for tok in path_tokens(line):
                for r in resolutions(path, tok):
                    files.setdefault(r, []).append(f"{path}:{n}")
            for m in DIR_LITERAL.finditer(line):
                pkg = package_of(path, tree) if path.endswith(".rs") else None
                dirs.setdefault(m.group(2), []).append((f"{path}:{n}", pkg))
    return files, dirs


def classify(changed, tree):
    """(prose_only, packages to test, report lines). Pure: no git, no disk."""
    report = []
    if not changed:
        report.append("no changed files: a scope that failed to resolve runs everything")
        return False, [], report
    files, dirs = scan(tree)
    prose_only = True
    tests, unacted = set(), set()
    for f in changed:
        if not is_prose_path(f):
            report.append(f"RUN   {f}: not Markdown under notes/, design/, briefs/ or the root")
            prose_only = False
            continue
        if f in files:
            where = ", ".join(sorted(set(files[f]))[:3])
            report.append(f"RUN   {f}: named by code ({where})")
            prose_only = False
            continue
        readers = [(d, place, pkg) for d, hits in dirs.items() if f.startswith(d + "/")
                   for place, pkg in hits]
        rust = sorted({pkg for _, _, pkg in readers if pkg})
        unowned = sorted({place for _, place, pkg in readers if pkg is None and place.split(":")[0].endswith(".rs")})
        if unowned:
            report.append(f"RUN   {f}: its directory is read by Rust in no package ({', '.join(unowned)})")
            prose_only = False
            continue
        tests.update(rust)
        unacted.update(place.split(":")[0] for _, place, pkg in readers if pkg is None)
        note = f"; tests of the packages that walk its directory: {' '.join(rust)}" if rust else ""
        report.append(f"SKIP  {f}: prose that no code names{note}")
    if unacted:
        report.append("note: these read a changed file's directory outside Rust and are not acted on,")
        report.append("      because lint runs them on every pull request or no gate does (see BUGS):")
        report.append("      " + " ".join(sorted(unacted)))
    return prose_only, (sorted(tests) if prose_only else []), report


def git_tree():
    """The checked-out tree, which in CI is the head under test."""
    paths = subprocess.run(["git", "ls-files"],
                           capture_output=True, text=True, check=True).stdout.splitlines()
    tree = {}
    for p in paths:
        if p.endswith("Cargo.toml") or scanned(p):
            try:
                with open(p, encoding="utf-8") as fh:
                    tree[p] = fh.read()
            except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
                tree[p] = None
    return tree


def main_classify(base):
    changed = subprocess.run(["git", "diff", "--name-only", base, "HEAD"],
                             capture_output=True, text=True, check=True).stdout.splitlines()
    prose, tests, report = classify(changed, git_tree())
    for line in report:
        print(line)
    print(f"prose-only: {'yes' if prose else 'no'} ({len(changed)} changed files)")
    print(f"prose-tests: {' '.join(tests)}")
    out = os.environ.get("GITHUB_OUTPUT")
    if out:
        with open(out, "a") as fh:
            fh.write(f"prose={'true' if prose else 'false'}\n")
            fh.write(f"prose_tests={' '.join(tests)}\n")
    return 0


def selftest():
    base_tree = {
        "Cargo.toml": "[workspace]\nmembers = []\n",
        "crates/documentation/Cargo.toml": '[package]\nname = "documentation"\nversion = "0.1.0"\n',
        "crates/documentation/tests/render.rs":
            '// walks the corpus\nfor dir in ["notes", "design/decisions"] {}\n',
        "crates/abi/Cargo.toml": '[package]\nname = "abi"\n',
        "crates/abi/src/lib.rs": '//! see notes/commented.md\npub fn f() {}\n',
        "xtask/Cargo.toml": '[package]\nname = "xtask"\n',
        "xtask/src/manual.rs": 'const B: &[&str] = &["notes/pipes.md"];\n',
        "script/lint": '# reads notes/lint-comment.md\nls "notes"\n',
        "notes/plain.md": None, "notes/pipes.md": None, "design/roadmap/1-x.md": None,
    }

    def run(changed, extra=None):
        tree = dict(base_tree)
        tree.update(extra or {})
        return classify(changed, tree)

    cases = []
    # Code: a .rs file is never prose.
    p, t, r = run(["kernel/src/main.rs"])
    cases.append(("code", p is False and r[0].startswith("RUN")))
    # Docs only, under a directory nothing walks: skips, no tests owed.
    p, t, r = run(["design/roadmap/1-x.md"])
    cases.append(("docs only", p is True and t == [] and r[0].startswith("SKIP")))
    # Docs under a walked directory: skips, and owes the walker's package tests.
    p, t, r = run(["notes/plain.md"])
    cases.append(("walked directory", p is True and t == ["documentation"]))
    # A script reading a directory is logged and does not stop the skip.
    cases.append(("script reader logged", any("script/lint" in x for x in r)))
    # A note named by code (DOC_BUNDLES): an input, runs everything.
    p, t, r = run(["notes/pipes.md"])
    cases.append(("named .md", p is False and "xtask/src/manual.rs:1" in r[0]))
    # include_str! relative to the source file, added in the same diff.
    p, t, r = run(["notes/plain.md", "crates/abi/src/lib.rs"],
                  {"crates/abi/src/lib.rs": 'pub const S: &str = include_str!("../../../notes/plain.md");\n'})
    cases.append(("included .md", p is False and any("named by code" in x for x in r)))
    p, t, r = run(["notes/plain.md"],
                  {"crates/abi/src/lib.rs": '#![doc = include_str!("../../../notes/plain.md")]\n'})
    cases.append(("doc attribute include", p is False))
    # A comment naming a note does not make it an input, in Rust or in a script.
    p, t, r = run(["notes/commented.md", "notes/lint-comment.md"])
    cases.append(("comments are not readers", p is True))
    # Mixed: one prose file and one code file runs everything.
    p, t, r = run(["design/roadmap/1-x.md", "Cargo.lock"])
    cases.append(("mixed", p is False and t == []))
    # Markdown outside the prose directories (a crate README) is not prose.
    p, t, r = run(["crates/abi/README.md"])
    cases.append(("crate README", p is False))
    # A top-level .md is prose; a non-.md under notes/ is not.
    cases.append(("root md", run(["ARCHITECTS.md"])[0] is True))
    cases.append(("notes non-md", run(["notes/project-metrics/data.csv"])[0] is False))
    # Nothing changed is a scope that failed, and runs everything.
    cases.append(("empty", run([])[0] is False))
    # A directory read from Rust in no package cannot be tested, so it runs everything.
    p, t, r = run(["notes/plain.md"], {"loose/walk.rs": 'let d = "notes";\n'})
    cases.append(("unowned rust reader", p is False))

    # A path mentioned in a sentence is a message, not a read; the same path alone is data.
    p, t, r = run(["notes/plain.md"],
                  {"crates/abi/src/lib.rs": 'panic!("the guard was hit; see notes/plain.md");\n'})
    cases.append(("message is not a read", p is True))
    p, t, r = run(["notes/plain.md"], {"script/x": 'cat "$root/notes/plain.md"\n'})
    cases.append(("bare path in a script", p is False))
    # A glob over a prose directory is a directory read.
    p, t, r = run(["design/roadmap/1-x.md"],
                  {"crates/abi/src/lib.rs": 'let g = glob("design/roadmap/*.md");\n'})
    cases.append(("glob reader", p is True and "abi" in t))

    # A package name that is not a plain name is never handed to the CI shell.
    p, t, r = run(["notes/plain.md"], {"evil/Cargo.toml": '[package]\nname = "x; curl y"\n',
                                        "evil/src/lib.rs": 'let d = "notes";\n'})
    cases.append(("hostile package name", p is False and t == []))

    bad = [name for name, ok in cases if not ok]
    for name, ok in cases:
        print(f"{'ok  ' if ok else 'FAIL'} {name}")
    if bad:
        print(f"prose_only selftest: {len(bad)} of {len(cases)} failed: {', '.join(bad)}", file=sys.stderr)
        return 1
    print(f"prose_only selftest: {len(cases)} cases")
    return 0


if __name__ == "__main__":
    args = sys.argv[1:]
    if args == ["--selftest"]:
        sys.exit(selftest())
    if len(args) == 2 and args[0] == "classify":
        sys.exit(main_classify(*args[1:]))
    print(__doc__.split("\n\n")[1], file=sys.stderr)
    sys.exit(2)
