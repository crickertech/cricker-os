import json,sys,re,collections
from datetime import datetime,timezone
rows={}
for l in open(sys.argv[1]):
    r=json.loads(l); rows[r['id']]=r
rows=list(rows.values())
now=datetime.now(timezone.utc)
def t(s): return datetime.fromisoformat(s.replace('Z','+00:00'))
def refkind(ref):
    if ref=='refs/heads/main': return 'main'
    if ref.startswith('refs/heads/gh-readonly-queue'): return 'merge-group'
    if ref.startswith('refs/pull/'): return 'pr'
    return 'other:'+ref
def prefix(k):
    # rust-cache keys: v0-rust-<prefix>-<job>-<os>-<hash>-<hash>
    m=re.match(r'(v0-rust-[^-]+(?:-[a-z0-9_]+)*?)-(Linux|macOS|Windows)',k)
    if m: return m.group(1)+'-'+m.group(2)
    return re.sub(r'-[0-9a-f]{16,}.*','',k)
tot=sum(r['size_in_bytes'] for r in rows)
print('entries',len(rows),'GB',round(tot/1e9,3))
by=collections.defaultdict(lambda:[0,0,0])
for r in rows:
    g=by[refkind(r['ref'])]; g[0]+=1; g[1]+=r['size_in_bytes']
    if (t(r['last_accessed_at'])-t(r['created_at'])).total_seconds()<600: g[2]+=1
print('\nby ref kind: n, GB, never-reaccessed(<10min after create)')
for k,v in sorted(by.items(),key=lambda x:-x[1][1]): print(k,v[0],round(v[1]/1e9,2),v[2])
by=collections.defaultdict(lambda:collections.Counter())
for r in rows:
    p=prefix(r['key']); k=refkind(r['ref']); c=by[p]
    c['n']+=1; c['b']+=r['size_in_bytes']; c['n_'+k]+=1; c['b_'+k]+=r['size_in_bytes']
    if (t(r['last_accessed_at'])-t(r['created_at'])).total_seconds()<600: c['cold']+=1
print('\nby key prefix')
for p,c in sorted(by.items(),key=lambda x:-x[1]['b']):
    print(f"{p:60s} n={c['n']:3d} GB={c['b']/1e9:6.2f} main={c['n_main']}/{c['b_main']/1e9:.2f} pr={c['n_pr']}/{c['b_pr']/1e9:.2f} mq={c['n_merge-group']}/{c['b_merge-group']/1e9:.2f} cold={c['cold']} avgMB={c['b']/c['n']/1e6:.0f}")
ages=sorted((now-t(r['created_at'])).total_seconds()/3600 for r in rows)
print('\noldest created h',round(ages[-1]),'median',round(ages[len(ages)//2]),'youngest',round(ages[0],1))
la=sorted((now-t(r['last_accessed_at'])).total_seconds()/3600 for r in rows)
print('last access h: median',round(la[len(la)//2],1),'max',round(la[-1]))
