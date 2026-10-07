import json,subprocess,sys,os,datetime
S=os.environ.get('OUT', os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
def api(path):
    out=subprocess.run(['gh','api','--paginate',path,'--jq','.[] | arrays | .[]'],capture_output=True,text=True).stdout
    return [json.loads(l) for l in out.splitlines() if l.strip()]
def jobs(run):
    js=api(f'repos/nifeos/nife/actions/runs/{run}/jobs?per_page=100&filter=all')
    return js
def ts(s): return datetime.datetime.fromisoformat(s.replace('Z','+00:00'))
ALWAYS={'draft gate','clippy'}
targets=[int(x) for x in sys.argv[1:]]
d={r['n']:r for r in json.load(open(f'{S}/q1.json'))}
res={}
for n in targets:
    if n in d:
        br=d[n]['branch']; day=d[n]['mergedAt'][:10]
    else:
        info=json.loads(subprocess.run(['gh','pr','view',str(n),'--json','headRefName,createdAt'],capture_output=True,text=True).stdout)
        br=info['headRefName']; day=None
    runs=[]
    for w in ('ci.yml','verify.yml'):
        runs+= [(w,'pr',x) for x in api(f'repos/nifeos/nife/actions/workflows/{w}/runs?branch={br}&event=pull_request&per_page=100')]
        if day:
            mg=api(f'repos/nifeos/nife/actions/workflows/{w}/runs?event=merge_group&created={day}&per_page=100')
            runs+= [(w,'mg',x) for x in mg if f'/pr-{n}-' in x['head_branch']]
    tot={'pr':[0,0],'mg':[0,0]}; nruns={'pr':0,'mg':0}
    for w,kind,r in runs:
        nruns[kind]+=1
        for j in jobs(r['id']):
            if not j.get('started_at') or not j.get('completed_at') or j['conclusion']=='skipped': continue
            sec=(ts(j['completed_at'])-ts(j['started_at'])).total_seconds()
            mins=max(1,-(-int(sec)//60)) if sec>0 else 0
            base=j['name'].split(' (')[0]
            tot[kind][0 if base in ALWAYS else 1]+=mins
    res[n]={'runs':nruns,'min_always':tot['pr'][0]+tot['mg'][0],'min_skippable':tot['pr'][1]+tot['mg'][1],'pr':tot['pr'],'mg':tot['mg']}
    print(n,br,res[n],flush=True)
json.dump(res,open(f'{S}/minutes-{"-".join(sys.argv[1:])}.json','w'))
