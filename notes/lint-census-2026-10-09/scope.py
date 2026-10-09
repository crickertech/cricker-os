"""Which Rust the 2026-10-09 lint census counts, and the test-code strip it applies.

Run from the repository root. Imported by `census.py` and `clippy_counts.py`, so both scripts
answer "which file is in scope" the same way.

The scope is the Rust that runs on nife and is not test code:

- `git ls-files '*.rs'`, minus `vendor/`;
- minus `helpers/rust_source.HOST_ONLY` (bench/host, xtask, tools, fuzz, helpers, patches) and the
  three host-tool crates that module's `HOST_TOOL_CRATES` names, which is the unsafe census's own
  answer to "which Rust is ours";
- minus test code: `system_tests/`, `fixtures/`, `bench/`, the three `*_exerciser/` workloads,
  `notes/` probes, `build.rs`, and any `tests/` or `benches/` directory;
- minus every item whose `#[cfg(...)]` is false when `test`, `kani`, `loom` and the test-only
  features are off. The item is blanked through its closing brace or semicolon, and an out-of-line
  `mod x;` under such a cfg drops its file.
"""

import os
import re
import subprocess
import sys

sys.path.insert(0, 'helpers')
import rust_source as rs  # noqa: E402

TEST_FEATURES = re.compile(r'system_tests|soak_test|hosttest|confinement_attackers|ipc_stack_depth')
HOST_TOOL = tuple(f'crates/{c}/' for c in sorted(rs.HOST_TOOL_CRATES))
TEST_DIRS = ('system_tests/', 'fixtures/', 'bench/', 'notes/', 'std_exerciser/',
             'pinned_tls_exerciser/', 'cryptography_exerciser/')


def in_scope(path):
    if path.startswith(('vendor/',) + rs.HOST_ONLY + HOST_TOOL + TEST_DIRS):
        return False
    return not ('/tests/' in path or '/benches/' in path or path.endswith('build.rs'))


def _split_args(s):
    out, depth, cur = [], 0, ''
    for c in s:
        depth += (c == '(') - (c == ')')
        if c == ',' and depth == 0:
            out.append(cur)
            cur = ''
        else:
            cur += c
    out.append(cur)
    return out


def cfg_can_ship(pred):
    """False when the predicate holds only in a test configuration. Unknown atoms count as on."""
    pred = pred.strip()
    m = re.match(r'(all|any|not)\s*\((.*)\)$', pred, re.S)
    if m:
        vals = [cfg_can_ship(a) for a in _split_args(m.group(2)) if a.strip()]
        if m.group(1) == 'all':
            return all(vals)
        if m.group(1) == 'any':
            return any(vals)
        return not vals[0]
    if pred in ('test', 'kani', 'loom'):
        return False
    m = re.match(r'feature\s*=\s*"([^"]*)"', pred)
    return not (m and TEST_FEATURES.search(m.group(1)))


def _item_end(code, start):
    depth = 0
    k = start
    while k < len(code):
        c = code[k]
        if c in '([':
            depth += 1
        elif c in ')]':
            depth -= 1
        elif c == '{' and depth == 0:
            d, k = 1, k + 1
            while k < len(code) and d:
                d += (code[k] == '{') - (code[k] == '}')
                k += 1
            return k
        elif c in ';,' and depth == 0:
            return k + 1
        elif c == '}' and depth == 0:
            return k
        k += 1
    return len(code)


def strip_test_items(code):
    """Blank test-only items in comment-stripped source. Returns (code, out-of-line mods dropped)."""
    inner = re.search(r'#!\[cfg\((.*?)\)\]', code, re.S)
    if inner and not cfg_can_ship(inner.group(1)):
        return re.sub(r'[^\n]', ' ', code), []
    out, dropped = list(code), []
    for m in re.finditer(r'#\[cfg\(', code):
        j, depth = m.end(), 1
        while j < len(code) and depth:
            depth += (code[j] == '(') - (code[j] == ')')
            j += 1
        if cfg_can_ship(code[m.end():j - 1]):
            continue
        body = code.index(']', j) + 1
        end = _item_end(code, body)
        mod = re.search(r'\bmod\s+(\w+)\s*;', code[body:end])
        if mod:
            dropped.append(mod.group(1))
        for x in range(m.start(), end):
            if out[x] != '\n':
                out[x] = ' '
    return ''.join(out), dropped


def load():
    """{path: comment-, literal- and test-stripped source} for every in-scope file."""
    names = subprocess.run(['git', 'ls-files', '*.rs'], capture_output=True, text=True,
                           check=True).stdout.split()
    code, drop = {}, set()
    for n in filter(in_scope, names):
        with open(n, errors='replace') as f:
            c, mods = strip_test_items(rs.strip_non_code(f.read()))
        code[n] = c
        stem = os.path.splitext(os.path.basename(n))[0]
        base = os.path.dirname(n)
        sub = base if stem in ('lib', 'main', 'mod') else os.path.join(base, stem)
        for md in mods:
            drop.update({os.path.join(sub, md + '.rs'), os.path.join(sub, md, 'mod.rs')})
    for d in drop:
        code.pop(d, None)
    return code
