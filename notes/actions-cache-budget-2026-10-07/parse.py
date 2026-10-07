import json,re,os,collections,statistics as st
from datetime import datetime
S=os.environ['S']; jobs=json.load(open(f"{S}/jobs.json"))
ts=lambda l: datetime.fromisoformat(l[:26]+'+00:00') if re.match(r'\d{4}-',l) else None
recs=[]
for j in jobs:
    p=f"{S}/logs/{j['job']}.log"
    L=[re.sub(r'\x1b\[[0-9;]*m','',x) for x in open(p,errors='replace').read().splitlines()]
    i=next((k for k,l in enumerate(L) if 'Run Swatinem/rust-cache' in l),None)
    if i is None: continue
    r=dict(j); r['prefix']=None
    t0=ts(L[i]); cfg=next(k for k in range(i,len(L)) if 'Cache Configuration' in L[k])
    r['pre']=(ts(L[cfg])-t0).total_seconds()
    r['toolchain_in_pre']=any(('downloading component' in L[k] or 'syncing channel' in L[k] or 'installing component' in L[k]) for k in range(i,cfg))
    rs=next(k for k in range(cfg,len(L)) if 'Restoring cache' in L[k])
    end=next(k for k in range(rs+1,len(L)) if '##[group]Run ' in L[k] or 'Post job' in L[k])
    r['restore_s']=(ts(L[end])-ts(L[rs])).total_seconds()
    res=L[rs+1][29:]
    r['result']='miss' if 'No cache found' in res else ('exact' if res.startswith('Cache hit for:') else ('partial' if 'restore-key' in res else res[:60]))
    m=[x for x in L[rs:end] if 'Received' in x]
    r['restored_mb']=int(re.search(r'Received (\d+) of (\d+)',m[-1]).group(2))/1e6 if m else 0
    sv=[k for k,l in enumerate(L) if '... Saving cache ...' in l and k>end]
    up=any('Cache up-to-date' in l for l in L[end:])
    r['saved']=bool(sv); r['uptodate']=up
    if sv:
        k=sv[0]; e=next((q for q in range(k+1,len(L)) if 'Post job cleanup' in L[q] or '##[group]' in L[q]),k+1)
        # include the cleaning steps before save
        c=max(q for q in range(end,k) if 'Cleaning' in L[q] or q==end)
        cl=next(q for q in range(end,k+1) if '... Cleaning' in L[q]) if any('... Cleaning' in L[q] for q in range(end,k)) else k
        r['save_s']=(ts(L[e])-ts(L[cl])).total_seconds()
        s=[x for x in L[k:e] if 'Sent' in x]
        r['saved_mb']=int(re.search(r'of (\d+)',s[-1]).group(1))/1e6 if s else 0
    else: r['save_s']=0; r['saved_mb']=0
    kl=next((L[q+1].strip()[29:].strip() if False else L[q+1][29:].strip() for q in range(cfg,rs) if L[q].endswith('Cache Key:')),'')
    r['key']=kl
    r['jobname']=re.sub(r'v0-rust-|-Linux.*','',kl)
    recs.append(r)
json.dump(recs,open(f"{S}/recs.json",'w'),default=str)
print('jobs with rust-cache',len(recs),'of',len(jobs))
def tab(keyf,title):
    print('\n'+title)
    g=collections.defaultdict(list)
    for r in recs: g[keyf(r)].append(r)
    for k,v in sorted(g.items(),key=lambda x:str(x[0])):
        c=collections.Counter(r['result'] for r in v)
        print(f"{str(k):38s} n={len(v):3d} exact={c['exact']:3d} partial={c['partial']:3d} miss={c['miss']:3d} pre_med={st.median(r['pre'] for r in v):5.1f} pre_tool={sum(r['toolchain_in_pre'] for r in v):3d} restore_med={st.median(r['restore_s'] for r in v):4.1f} saves={sum(r['saved'] for r in v):3d} save_med={st.median([r['save_s'] for r in v if r['saved']] or [0]):4.1f} savedGB={sum(r['saved_mb'] for r in v)/1e3:5.2f}")
tab(lambda r:r['event'],'by event')
tab(lambda r:(r['wf'],r['jobname']),'by job')
hits=[r for r in recs if r['result']!='miss']
print('\nhit restore secs',sorted(round(r['restore_s'],1) for r in hits))
print('hit restored MB median',st.median(r['restored_mb'] for r in hits))
