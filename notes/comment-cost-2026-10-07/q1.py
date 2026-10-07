"""Q1: which merged PRs had comment-only Rust edits. Runs in the lane worktree."""
import json, subprocess, sys, os, re
S = os.environ.get('OUT', os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rustlex import lex
sys.path.insert(0, os.path.join(os.getcwd(),'helpers'))
import prose_only

def git(*a, check=True):
    return subprocess.run(['git', *a], capture_output=True, text=True, check=check).stdout

def show(rev, path):
    r = subprocess.run(['git', 'show', f'{rev}:{path}'], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None

FENCE = re.compile(r'```')

prs = json.load(open(f'{S}/prs.json'))
out = []
for p in prs:
    m = p['mergeCommit']['oid']
    names = git('diff', '--name-status', '-M', f'{m}^1', m).splitlines()
    files = []
    for ln in names:
        parts = ln.split('\t')
        st = parts[0][0]
        files.append((st, parts[1], parts[-1]))
    rs = [(st, a, b) for st, a, b in files if b.endswith('.rs') or a.endswith('.rs')]
    rec = {'n': p['number'], 'title': p['title'], 'branch': p['headRefName'], 'merge': m,
           'mergedAt': p['mergedAt'], 'nfiles': len(files), 'nrs': len(rs)}
    rs_detail = []
    for st, a, b in rs:
        if st in 'AD':
            rs_detail.append({'path': b, 'kind': 'added' if st == 'A' else 'deleted'}); continue
        old, new = show(f'{m}^1', a), show(m, b)
        if old is None or new is None:
            rs_detail.append({'path': b, 'kind': 'code'}); continue
        oc, ocm = lex(old); nc, ncm = lex(new)
        if oc != nc:
            rs_detail.append({'path': b, 'kind': 'code'}); continue
        # comment-only: find changed comments
        oset = [t for _, t, _, _ in ocm]; nset = [t for _, t, _, _ in ncm]
        import difflib
        changed_old, changed_new = [], []
        sm = difflib.SequenceMatcher(a=oset, b=nset, autojunk=False)
        for tag, i1, i2, j1, j2 in sm.get_opcodes():
            if tag != 'equal':
                changed_old += ocm[i1:i2]; changed_new += ncm[j1:j2]
        doc = any(k == 'doc' for k, *_ in changed_old + changed_new)
        fence = any(k == 'doc' and FENCE.search(t) for k, t, *_ in changed_old + changed_new)
        # a fenced doc block can span many /// lines; check whether the changed lines sit in a doc
        # block that has a fence anywhere in it
        def block_has_fence(cm, changed):
            lines = {a for k, t, a, b in changed if k == 'doc'}
            if not lines: return False
            docs = sorted((a, t) for k, t, a, b in cm if k == 'doc')
            # group contiguous doc lines
            groups, cur, last = [], [], None
            for a, t in docs:
                if last is not None and a != last + 1:
                    groups.append(cur); cur = []
                cur.append((a, t)); last = a
            if cur: groups.append(cur)
            for g in groups:
                ls = {a for a, _ in g}
                if ls & lines and any('```' in t for _, t in g): return True
            return False
        fence = fence or block_has_fence(ocm, changed_old) or block_has_fence(ncm, changed_new)
        rs_detail.append({'path': b, 'kind': 'comment', 'doc': doc, 'fence': fence,
                          'old': [t for _, t, _, _ in changed_old][:60], 'new': [t for _, t, _, _ in changed_new][:60]})
    rec['rs'] = rs_detail
    rec['rs_all_comment'] = bool(rs) and all(d['kind'] == 'comment' for d in rs_detail)
    rec['rs_any_comment'] = any(d['kind'] == 'comment' for d in rs_detail)
    nonrs = [b for st, a, b in files if not (b.endswith('.rs') or a.endswith('.rs'))]
    rec['nonrs'] = nonrs
    rec['nonrs_md_prose_dirs'] = all(prose_only.is_prose_path(f) for f in nonrs)
    out.append(rec)
    print(p['number'], len(files), len(rs), rec['rs_all_comment'], file=sys.stderr)

json.dump(out, open(f'{S}/q1.json', 'w'), indent=1)
