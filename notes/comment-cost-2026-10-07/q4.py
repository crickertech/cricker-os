import subprocess,sys,os,random
S=os.environ.get('OUT', os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rustlex import lex
files=[f for f in subprocess.run(['git','ls-files','*.rs'],capture_output=True,text=True).stdout.splitlines() if not f.startswith(('vendor/','patches/'))]
blocks=[]
for f in files:
    src=open(f,encoding='utf-8',errors='replace').read(); _,cm=lex(src)
    cur=None
    for k,t,a,b in cm:
        if cur and a==cur[3]+1 and k==cur[1]: cur[3]=b; cur[4]+=1
        else:
            if cur: blocks.append(tuple(cur))
            cur=[f,k,a,b,1]
    if cur: blocks.append(tuple(cur))
print('blocks',len(blocks),file=sys.stderr)
import statistics
sizes=[b[3]-b[2]+1 for b in blocks]; print('median block lines',statistics.median(sizes),'mean',sum(sizes)/len(sizes),'p90',sorted(sizes)[int(.9*len(sizes))],file=sys.stderr)
random.seed(20261007)
for i,(f,k,a,b,_) in enumerate(random.sample(blocks,30),1):
    lines=open(f,encoding='utf-8',errors='replace').read().split('\n')
    body=lines[a-1:b]
    show=body if len(body)<=14 else body[:10]+[f'   ... ({len(body)-12} lines) ...']+body[-2:]
    print(f'##### {i} {f}:{a}-{b} {k} ({b-a+1} lines)')
    for l in show: print('  ',l[:160])
    for l in lines[b:b+3]: print(' >',l[:120])
