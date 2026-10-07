import subprocess,sys,os,json,collections
S=os.environ.get('OUT', os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rustlex import comment_lines, lex
root=sys.argv[1] if len(sys.argv)>1 else '.'
label=sys.argv[2] if len(sys.argv)>2 else 'nife'
if label=='nife':
    files=[f for f in subprocess.run(['git','ls-files','*.rs'],capture_output=True,text=True,cwd=root).stdout.splitlines() if not f.startswith(('vendor/','patches/'))]
else:
    files=[os.path.relpath(os.path.join(dp,f),root) for dp,_,fs in os.walk(root) for f in fs if f.endswith(('.rs','.c','.h'))]
AREAS=('kernel/','crates/','components/','xtask/','system_tests/')
agg=collections.defaultdict(lambda:[0,0,0,0,0,0]); per=[]
for f in files:
    try: src=open(os.path.join(root,f),encoding='utf-8',errors='replace').read()
    except Exception: continue
    if f.endswith(('.c','.h')):
        # C: treat /* */ and // like Rust (no raw strings); good enough
        pass
    c,m,b,dl=comment_lines(src)
    _,cm=lex(src)
    cbytes=sum(len(t) for _,t,_,_ in cm)
    area=next((a for a in AREAS if f.startswith(a)),'other') if label=='nife' else label
    a=agg[area]; a[0]+=c;a[1]+=m;a[2]+=b;a[3]+=dl;a[4]+=len(src.encode());a[5]+=cbytes
    t=agg['ALL'];t[0]+=c;t[1]+=m;t[2]+=b;t[3]+=dl;t[4]+=len(src.encode());t[5]+=cbytes
    per.append((m,c,len(src.encode()),cbytes,f))
print(f'{"area":14} {"files":>6} {"code":>8} {"comment":>8} {"c/code":>6} {"cmt%lines":>9} {"doc":>7} {"cmtbytes%":>9}')
cnt=collections.Counter((next((a for a in AREAS if p[4].startswith(a)),'other') if label=='nife' else label) for p in per); cnt['ALL']=len(per)
for k,(c,m,b,dl,by,cb) in sorted(agg.items()):
    print(f'{k:14} {cnt[k]:6} {c:8} {m:8} {m/max(c,1):6.2f} {100*m/max(c+m,1):8.1f}% {dl:7} {100*cb/max(by,1):8.1f}%')
per.sort(reverse=True)
json.dump(per,open(f'{S}/q3-{label}.json','w'))
if label=='nife':
    print('top 20 by comment lines')
    for m,c,by,cb,f in per[:20]: print(f'{m:6} {c:6} {m/max(c,1):5.2f} {by//4:7} {f}')
    sizes=sorted(by for _,_,by,_,_ in per); import statistics
    print('median bytes',statistics.median(sizes),'p90',sizes[int(.9*len(sizes))],'mean',sum(sizes)/len(sizes))
    shares=sorted(cb/by for _,_,by,cb,_ in per if by>2000); print('median file comment-byte share',statistics.median(shares))
