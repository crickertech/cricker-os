"""Text counts for the 2026-10-09 lint census: what a grep sees once comments, strings and tests
are gone. Run from the repository root: `python3 notes/lint-census-2026-10-09/census.py`.

These are pattern counts, not lint hits. `clippy_counts.py` gives the lint's own view, which is
narrower (one configuration's compiled code) and exact about what the lint would flag.
"""

import collections
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import scope  # noqa: E402

PATTERNS = {
    '.unwrap()': r'\.unwrap\(\)',
    '.expect(': r'\.expect\(',
    'panic!': r'\bpanic!\s*\(',
    'unreachable!': r'\bunreachable!\s*\(',
    'todo!': r'\btodo!\s*\(',
    'unimplemented!': r'\bunimplemented!\s*\(',
    'as <integer>': r'\bas\s+(?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize)\b',
    'static mut (declarations)': r'(?m)^[ \t]*(?:pub(?:\([^)]*\))?\s+)?static\s+mut\s+\w',
    'allow(...) attributes': r'#!?\[(?:cfg_attr\([^\]]*)?allow\(',
    '  of them dead_code': r'#!?\[(?:cfg_attr\([^\]]*)?allow\([^\]]*\bdead_code\b',
    '  of them bare #[allow(dead_code)]': r'#\[allow\([^\]]*\bdead_code\b',
    '  of them clippy::': r'#!?\[(?:cfg_attr\([^\]]*)?allow\([^\]]*\bclippy::',
    '  of them unused*': r'#!?\[(?:cfg_attr\([^\]]*)?allow\([^\]]*\bunused',
    'expect(...) attributes': r'#!?\[(?:cfg_attr\([^\]]*)?expect\(',
}


def main():
    code = scope.load()
    tree, kernel = collections.Counter(), collections.Counter()
    for path, text in code.items():
        for name, pat in PATTERNS.items():
            n = len(re.findall(pat, text))
            tree[name] += n
            if path.startswith('kernel/src/'):
                kernel[name] += n
    k_files = sum(1 for p in code if p.startswith('kernel/src/'))
    print(f'files in scope: {len(code)} ({k_files} under kernel/src/)')
    print(f'{"pattern":36} {"in scope":>9} {"kernel":>7}')
    for name in PATTERNS:
        print(f'{name:36} {tree[name]:9} {kernel[name]:7}')

    # Markers and ignores are read from raw text over every tracked file, tests included, because
    # a TODO lives in a comment and an #[ignore] lives in a test.
    names = subprocess.run(['git', 'ls-files', '*.rs', ':!vendor'], capture_output=True,
                           text=True, check=True).stdout.split()
    todo, ignores = 0, []
    for n in names:
        with open(n, errors='replace') as f:
            for i, line in enumerate(f, 1):
                if re.search(r'\b(TODO|FIXME|XXX)\b', line):
                    todo += 1
                m = re.match(r'\s*#\[ignore\b(.*)', line)
                if m:
                    ignores.append((n, i, '=' in m.group(1)))
    print(f'lines mentioning TODO/FIXME/XXX in any tracked .rs: {todo}')
    print(f'#[ignore] attributes: {len(ignores)}, with a reason: {sum(r for *_, r in ignores)}')


if __name__ == '__main__':
    main()
