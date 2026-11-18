import json, glob, sys, collections
root = sys.argv[1]; run = sys.argv[2]
names = json.load(open(f'{root}/runs/m17/key.json'))

def arr(raw):
    v = json.loads(raw); t = v.get('result', ''); s = t.find('['); e = t.rfind(']')
    return json.loads(t[s:e+1])

units = collections.defaultdict(list)
for d in sorted(glob.glob(f'{root}/{run}/critics/*')):
    m = json.load(open(d + '/meta.json')); a = arr(open(d + '/answer.json').read())
    pos = {n: (s, ln) for (n, s), ln in zip(m['items'], m['lens'])}
    for x in a:
        if x.get('no') not in pos: continue
        s, (la, lb) = pos[x['no']]
        c = (x.get('secim') or '').strip()[:1].upper()
        win = m['a'] if c == 'A' else m['b']
        lens = {m['a']: la, m['b']: lb}
        units[(frozenset((m['a'], m['b'])), m['rep'], s)].append((win, lens))
cand = {c for c, n in names.items() if n in ('selvi', 'tunca', 'marla')}
for label, filt in (('all pairs', lambda p: True), ('candidate pairs only', lambda p: p <= cand)):
    lw = n = 0; diffs = []; wins = []
    for (p, rep, s), v in units.items():
        if not filt(p) or len(v) != 2: continue
        if v[0][0] != v[1][0]: continue
        w = v[0][0]; lens = v[0][1]; o = [c for c in p if c != w][0]
        if lens[w] == lens[o]: continue
        n += 1; lw += lens[w] > lens[o]
    print(f'{run} {label}: longer won {lw}/{n} = {lw/n:.3f}' if n else f'{label}: none')
