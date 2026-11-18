import json, glob, collections, sys
root = sys.argv[1]
L = {'breakout_up':'A','breakout_down':'B','reversal_up':'C','reversal_down':'D','trend_up':'E','trend_down':'F','squeeze':'G','extreme_up':'H','extreme_down':'I','flow_in':'J','flow_out':'K'}
key = {r['id']: r['key'] for r in json.load(open(f'{root}/out/r4/dump.json'))['rows']}
lab = {x['situation']: x['label'] for x in json.load(open(f'{root}/runs/m17/set.json'))}
names = json.load(open(f'{root}/runs/m17/key.json'))

def arr(raw):
    v = json.loads(raw); t = v.get('result', ''); s = t.find('['); e = t.rfind(']')
    return json.loads(t[s:e+1])

def load(run):
    out = {}
    for d in sorted(glob.glob(f'{root}/{run}/readers/*')):
        m = json.load(open(d + '/meta.json')); a = arr(open(d + '/answer.json').read())
        it = dict((n, s) for n, s in m['items'])
        for x in a:
            s = it.get(x.get('no'))
            if s: out[(m['code'], m['persona'], m['cond'], m['page'], m['reader'], s)] = x
    return out

h = load('runs/m17')
err = collections.defaultdict(collections.Counter); tot = collections.Counter()
for (code, p, cond, pg, r, s), x in h.items():
    want = L[key[s]['event']]; got = (x.get('S1') or '?').strip()[:1].upper()
    tot[(names[code], cond, s)] += 1
    if got != want: err[(names[code], cond, s)][f'{want}->{got}'] += 1
print('## S1 errors (lang, cond, situation)')
for k in sorted(err): print(k[0], k[1], k[2], lab[k[2]], dict(err[k]), '/', tot[k])

# haiku <-> sonnet raw agreement on identical prompts (reader 1)
so = load('runs/m17-sonnet')
agree = collections.Counter(); n = collections.Counter()
for k, x in so.items():
    y = h.get(k)
    if y is None: continue
    for q in ('S1', 'S3', 'S4', 'S6'):
        a = (x.get(q) or '').strip()[:1].upper(); b = (y.get(q) or '').strip()[:1].upper()
        n[q] += 1; agree[q] += (a == b)
print('## reader letter agreement haiku r1 vs sonnet', {q: f'{agree[q]}/{n[q]}={agree[q]/n[q]:.3f}' for q in n})

def crit(run):
    out = {}
    for d in sorted(glob.glob(f'{root}/{run}/critics/*')):
        m = json.load(open(d + '/meta.json')); a = arr(open(d + '/answer.json').read())
        it = dict((nn, s) for nn, s in m['items'])
        for x in a:
            s = it.get(x.get('no')); c = (x.get('secim') or '').strip()[:1].upper()
            win = m['a'] if c == 'A' else m['b'] if c == 'B' else None
            out[(m['a'], m['b'], m['rep'], m['order'], s)] = win
    return out
hc = crit('runs/m17'); sc = crit('runs/m17-sonnet')
ag = sum(1 for k, v in sc.items() if hc.get(k) == v); nn = sum(1 for k in sc if k in hc)
print(f'## critic pick agreement (same prompt) {ag}/{nn}={ag/nn:.3f}')
# agreement excluding bozuk pairs
bz = [c for c, nm in names.items() if nm == 'bozuk'][0]; r4 = [c for c, nm in names.items() if nm == 'r4'][0]
ks = [k for k in sc if k in hc and bz not in k[:2] and r4 not in k[:2]]
ag2 = sum(1 for k in ks if hc[k] == sc[k])
print(f'## critic agreement among the three candidates only {ag2}/{len(ks)}={ag2/len(ks):.3f}')
