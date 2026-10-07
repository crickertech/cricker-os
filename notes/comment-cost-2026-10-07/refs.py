import subprocess,sys,os,re,collections
S=os.environ.get('OUT', os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rustlex import lex
files=[f for f in subprocess.run(['git','ls-files','*.rs'],capture_output=True,text=True).stdout.splitlines() if not f.startswith(('vendor/','patches/'))]
tracked=set(subprocess.run(['git','ls-files'],capture_output=True,text=True).stdout.splitlines())
dirs=set()
for t in tracked:
    p=t
    while '/' in p:
        p=p.rsplit('/',1)[0]; dirs.add(p)
PAT={
 'path .md (notes/design/briefs)': re.compile(r'(?<![\w/])((?:\.\./)*(?:notes|design|briefs)/[\w./-]+\.md)'),
 'path proposals/': re.compile(r'design/roadmap/proposals/[\w.-]+'),
 'path other (src file)': re.compile(r'(?<![\w/])((?:kernel|crates|components|xtask|system_tests|script|helpers|uefi_loader)/[\w./-]+)'),
 'PR/issue #NNN': re.compile(r'(?<![\w&])#\d{3,4}\b'),
 'milestone N': re.compile(r'\b[Mm]ilestone \d+'),
 '§N': re.compile(r'§\s?\d+'),
 'DECISIONS §': re.compile(r'DECISIONS'),
 'lane/ branch': re.compile(r'\blane/[\w-]+'),
 'date 2026-': re.compile(r'\b20\d\d-\d\d-\d\d\b'),
 'line number (file:N / line N)': re.compile(r'\.(?:rs|md|py|sh):\d+|\bline \d+\b'),
}
cnt=collections.Counter(); lines_with=collections.Counter(); dead=collections.Counter(); deadex=[]
total_cmt_lines=0; any_ref_lines=0
for f in files:
    src=open(f,encoding='utf-8',errors='replace').read()
    _,cm=lex(src)
    for k,t,a,b in cm:
        for ln in t.split('\n'):
            total_cmt_lines+=1; hit=False
            for name,p in PAT.items():
                ms=p.findall(ln)
                if ms:
                    cnt[name]+=len(ms); lines_with[name]+=1
                    if name not in ('date 2026-',): hit=True
                    if name.startswith('path'):
                        for m in ms:
                            m=m if isinstance(m,str) else m[0]
                            m=m.rstrip('.,;:)`\'"')
                            cands={os.path.normpath(m), os.path.normpath(os.path.join(os.path.dirname(f),m)), os.path.normpath(re.sub(r'^(\.\./)+','',m))}
                            if not any(c in tracked or c in dirs for c in cands) and '*' not in m and '{' not in m and '<' not in m:
                                dead[name]+=1
                                if len(deadex)<400: deadex.append((f,a,m))
            if hit: any_ref_lines+=1
print('comment lines',total_cmt_lines,'lines with a non-date reference',any_ref_lines)
for n in PAT: print(f'{n:34} refs {cnt[n]:6} lines {lines_with[n]:6} dead {dead[n]}')
import random; random.seed(7)
for x in deadex: print("DEAD",x)
