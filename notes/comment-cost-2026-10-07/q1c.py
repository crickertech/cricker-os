import json,subprocess,sys,os,re
S=os.environ.get('OUT', os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rustlex import lex
def show(rev,p):
    r=subprocess.run(['git','show',f'{rev}:{p}'],capture_output=True,text=True); return r.stdout if r.returncode==0 else ''
def fences(src):
    _,cm=lex(src); blocks=[];cur=None
    for k,t,a,b in cm:
        if k!='doc': continue
        for ln in t.split('\n'):
            s=re.sub(r'^\s*(///|//!|/\*\*|/\*!|\*/|\*)?','',ln).rstrip()
            if s.lstrip().startswith('```'):
                if cur is None: cur=[]
                else: blocks.append('\n'.join(cur)); cur=None
            elif cur is not None: cur.append(s)
    return blocks
d=json.load(open(f'{S}/q1.json'))
for r in d:
    if not r['rs_all_comment']: continue
    hit=[x['path'] for x in r['rs'] if fences(show(r['merge']+'^1',x['path']))!=fences(show(r['merge'],x['path']))]
    print(r['n'],'doctest code changed in:',hit)
