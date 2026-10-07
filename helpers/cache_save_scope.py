#!/usr/bin/env python3
"""A workflow that runs on a merge group saves no rust-cache entry from it.

    helpers/cache_save_scope.py              # script/lint: every rust-cache step is scoped
    helpers/cache_save_scope.py --selftest

Name provisional (lane/actions-cache-budget, 2026-10-07 UTC).

# WHY

A run restores caches from its own ref and from the default branch's. Every merge group runs on a
fresh `gh-readonly-queue/main/pr-N-<sha>` ref, so an entry saved there can never be restored by a
later run. On 2026-10-07 the repository held 10.7 GB of caches against GitHub's 10 GB limit; 163
entries (3.8 GB) were rust-cache saves from merge groups, and 111 merge-group jobs sampled that day
restored none. Those entries pushed out the pull request entries that do hit. The measurement is in
notes/actions-cache-budget-2026-10-07.md.

# THE RULE

In every workflow under `.github/workflows/` whose `on:` lists `merge_group`, each step that uses
`Swatinem/rust-cache` sets `save-if:` to an expression that names `merge_group`. The check reads
text, not YAML, because a runner's Python and this Mac's both lack a YAML module; it finds the step
by its `uses:` line and looks for `save-if:` before the next step begins.

# BUGS

- Only rust-cache is covered. `actions/cache` steps (QEMU, the patched Kani, the vendor pins) use
  stable keys that hit `main`'s entries, so they save only on a miss. After a key change, though, a
  merge group's save is just as unreachable; the note records that as an open question.
- The check accepts any `save-if:` that mentions `merge_group`, so `== 'merge_group'` passes too. It
  catches a step that forgot the scope, not one that inverted it.
"""

import os
import re
import sys

WORKFLOW_DIR = ".github/workflows"
USES = re.compile(r"^(\s*)-?\s*uses:\s*Swatinem/rust-cache@")
STEP = re.compile(r"^(\s*)-\s")


def runs_on_merge_group(text):
    return re.search(r"^  merge_group:", text, re.M) is not None


def unscoped_steps(text):
    """Line numbers (1-based) of rust-cache steps with no merge_group save-if."""
    lines = text.split("\n")
    bad = []
    for i, line in enumerate(lines):
        m = USES.match(line)
        if not m:
            continue
        indent = len(m.group(1))
        ok = False
        for j in range(i + 1, len(lines)):
            nxt = lines[j]
            s = STEP.match(nxt)
            if s and len(s.group(1)) <= indent:
                break
            if nxt.strip() and not nxt.lstrip().startswith("#") and \
                    len(nxt) - len(nxt.lstrip()) < indent:
                break
            if re.match(r"\s*save-if:.*merge_group", nxt):
                ok = True
                break
        if not ok:
            bad.append(i + 1)
    return bad


def lint(root="."):
    failures = []
    d = os.path.join(root, WORKFLOW_DIR)
    for name in sorted(os.listdir(d)):
        if not name.endswith((".yml", ".yaml")):
            continue
        text = open(os.path.join(d, name)).read()
        if not runs_on_merge_group(text):
            continue
        for n in unscoped_steps(text):
            failures.append("%s/%s:%d" % (WORKFLOW_DIR, name, n))
    if failures:
        sys.stderr.write(
            "lint: a rust-cache step in a merge-group workflow saves an entry no run can restore.\n"
            "  Add `with: save-if: ${{ github.event_name != 'merge_group' }}` (see\n"
            "  helpers/cache_save_scope.py):\n")
        for f in failures:
            sys.stderr.write("    %s\n" % f)
        return 1
    return 0


def selftest():
    scoped = (
        "on:\n  merge_group:\njobs:\n  a:\n    steps:\n"
        "      - uses: Swatinem/rust-cache@v2\n"
        "        if: x\n"
        "        with:\n"
        "          save-if: ${{ github.event_name != 'merge_group' }}\n"
        "      - run: true\n")
    bare = (
        "on:\n  merge_group:\njobs:\n  a:\n    steps:\n"
        "      - uses: Swatinem/rust-cache@v2\n"
        "      - run: true\n"
        "        with:\n"
        "          save-if: merge_group\n")
    other_key = (
        "jobs:\n  a:\n    steps:\n"
        "      - uses: Swatinem/rust-cache@v2\n"
        "        with:\n"
        "          shared-key: x\n"
        "      - run: true\n")
    cases = [
        (unscoped_steps(scoped), []),
        # The next step's `save-if` must not count for the bare one above it.
        (unscoped_steps(bare), [6]),
        (unscoped_steps(other_key), [4]),
        (runs_on_merge_group(scoped), True),
        (runs_on_merge_group(other_key), False),
    ]
    ok = all(got == want for got, want in cases)
    for got, want in cases:
        if got != want:
            print("cache_save_scope selftest: got %r, want %r" % (got, want))
    print("cache_save_scope selftest: %s" % ("ok" if ok else "FAILED"))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ["--selftest"]:
        return selftest()
    if argv[1:]:
        sys.stderr.write(__doc__)
        return 2
    return lint()


if __name__ == "__main__":
    sys.exit(main(sys.argv))
