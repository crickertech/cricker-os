import json,subprocess,sys,os
S=os.environ.get('OUT', os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0,os.path.join(os.getcwd(),'helpers'))
import prose_only
d=json.load(open(f'{S}/q1.json'))
res={}
for r in d:
    if not (r['rs_all_comment'] and r['nonrs_md_prose_dirs']): continue
    m=r['merge']
    paths=subprocess.run(['git','ls-tree','-r','--name-only',m],capture_output=True,text=True).stdout.splitlines()
    want=[p for p in paths if p.endswith('Cargo.toml') or prose_only.scanned(p)]
    inp='\n'.join(f'{m}:{p}' for p in want)+'\n'
    out=subprocess.run(['git','cat-file','--batch'],input=inp.encode(),capture_output=True).stdout
    tree={};i=0
    for p in want:
        nl=out.index(b'\n',i);hdr=out[i:nl].split();size=int(hdr[2]);body=out[nl+1:nl+1+size];i=nl+1+size+1
        try: tree[p]=body.decode()
        except UnicodeDecodeError: tree[p]=None
    if not r['nonrs']:
        res[r['n']]=(True,[],['(no non-Rust files)']);continue
    p,t,rep=prose_only.classify(r['nonrs'],tree)
    res[r['n']]=(p,t,rep)
    print(r['n'],p,t,[x for x in rep if x.startswith('RUN')][:3])
json.dump({k:v[0] for k,v in res.items()},open(f'{S}/q1b.json','w'))
