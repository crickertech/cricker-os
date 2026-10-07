"""Minimal Rust lexer: split source into code tokens and comments.

Handles line comments, nested block comments, doc comments, strings, raw strings (r#".."#),
byte strings, char literals versus lifetimes. Good enough to decide "did the code change".
"""
import re

def lex(src):
    """Return (code_tokens, comments) where comments is a list of (kind, text, start_line, end_line).
    kind: 'doc' for ///, //!, /** */, /*! */; 'plain' otherwise."""
    i, n = 0, len(src)
    code = []
    comments = []
    line = 1
    buf_start = None
    def flush_ident():
        pass
    tok = []
    def push(t):
        code.append(t)
    while i < n:
        c = src[i]
        if c == '\n':
            line += 1; i += 1; continue
        if c in ' \t\r':
            i += 1; continue
        if src.startswith('//', i):
            j = src.find('\n', i)
            if j < 0: j = n
            text = src[i:j]
            doc = (text.startswith('///') and not text.startswith('////')) or text.startswith('//!')
            comments.append(('doc' if doc else 'plain', text, line, line))
            i = j; continue
        if src.startswith('/*', i):
            depth = 0; j = i; sl = line
            while j < n:
                if src.startswith('/*', j): depth += 1; j += 2; continue
                if src.startswith('*/', j):
                    depth -= 1; j += 2
                    if depth == 0: break
                    continue
                if src[j] == '\n': line += 1
                j += 1
            text = src[i:j]
            doc = (text.startswith('/**') and not text.startswith('/***') and text != '/**/') or text.startswith('/*!')
            comments.append(('doc' if doc else 'plain', text, sl, line))
            i = j; continue
        # raw strings r"..", r#".."#, br#".."#
        m = re.match(r'(b|c)?r(#*)"', src[i:i+300])
        if m and (i == 0 or not (src[i-1].isalnum() or src[i-1] == '_')):
            hashes = m.group(2)
            end = '"' + hashes
            j = src.find(end, i + m.end())
            if j < 0: j = n
            s = src[i:j+len(end)]
            line += s.count('\n'); push(s); i = j + len(end); continue
        if c == '"' or (c in 'bc' and i+1 < n and src[i+1] == '"' and not (i > 0 and (src[i-1].isalnum() or src[i-1]=='_'))):
            j = i + (1 if c == '"' else 2)
            while j < n and src[j] != '"':
                if src[j] == '\\': j += 1
                j += 1
            s = src[i:j+1]; line += s.count('\n'); push(s); i = j + 1; continue
        if c == "'" or (c == 'b' and i+1 < n and src[i+1] == "'"):
            k = i + (1 if c == "'" else 2)
            m = re.match(r"(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'\n])'", src[k:k+16])
            if m:
                push(src[i:k+m.end()]); i = k + m.end(); continue
            # lifetime or label
            m = re.match(r"'[A-Za-z_][A-Za-z0-9_]*", src[i:])
            if m:
                push(m.group(0)); i += m.end(); continue
            push(c); i += 1; continue
        m = re.match(r'[A-Za-z_][A-Za-z0-9_]*|[0-9][0-9A-Za-z_.]*', src[i:i+200])
        if m:
            push(m.group(0)); i += m.end(); continue
        push(c); i += 1
    return code, comments


def comment_lines(src):
    """Count physical lines: (code_lines, comment_only_lines, blank_lines, doc_lines)."""
    code, comments = lex(src)
    lines = src.split('\n')
    cmt = set(); doc = set()
    for kind, text, a, b in comments:
        for l in range(a, b + 1):
            cmt.add(l)
            if kind == 'doc': doc.add(l)
    # a line is code if it has a code token; approximate by stripping comments from the line
    # simpler: a line is comment-only if it is in cmt and its stripped text starts with // or /* or * or is inside a block
    code_l = cmt_l = blank = doc_l = 0
    inblock = set()
    for kind, text, a, b in comments:
        if b > a:
            for l in range(a + 1, b): inblock.add(l)
    for idx, raw in enumerate(lines, 1):
        s = raw.strip()
        if not s:
            blank += 1; continue
        if idx in inblock or s.startswith('//') or s.startswith('/*') or (idx in cmt and (s.startswith('*'))):
            cmt_l += 1
            if idx in doc: doc_l += 1
            continue
        if idx in cmt and s.endswith('*/') and '/*' not in s:
            cmt_l += 1; continue
        code_l += 1
    return code_l, cmt_l, blank, doc_l
