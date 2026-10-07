import json,subprocess,os,sys,concurrent.futures as cf
S=os.environ['S']; T=subprocess.check_output(['gh','auth','token'],text=True).strip()
def api(p): return json.loads(subprocess.check_output(['gh','api',p]))
jobs=[]
for wf in ['ci.yml','verify.yml']:
    for line in open(f"{S}/{wf.split('.')[0]}_runs.tsv"):
        rid,ev,br,concl,_=line.rstrip('\n').split('\t')
        if concl not in ('success','failure'): continue
        for j in api(f"repos/nifeos/nife/actions/runs/{rid}/jobs?per_page=100")['jobs']:
            if j['conclusion'] in ('success','failure'):
                jobs.append(dict(wf=wf,run=rid,event=ev,branch=br,job=j['id'],name=j['name'],started=j['started_at'],completed=j['completed_at']))
json.dump(jobs,open(f"{S}/jobs.json",'w'))
def get(j):
    out=f"{S}/logs/{j['job']}.log"
    if os.path.exists(out): return
    data=subprocess.run(['curl','-sL','-H',f'Authorization: Bearer {T}',f"https://api.github.com/repos/nifeos/nife/actions/jobs/{j['job']}/logs"],capture_output=True).stdout
    open(out,'wb').write(data)
with cf.ThreadPoolExecutor(12) as ex: list(ex.map(get,jobs))
print(len(jobs))
