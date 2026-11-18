import json
import os
import sys

S = os.path.dirname(os.path.abspath(__file__))
names = ['pavka-old', 'pavka-new', 'korda', 'r4']


def fmt(v):
    if v is None:
        return '—'
    if isinstance(v, float):
        s = f'{v:.3f}'.rstrip('0').rstrip('.')
        return s.replace('.', ',')
    return str(v)


def load(run, n):
    p = f'{S}/{run}/k52/{n}.json'
    return json.load(open(p)) if os.path.exists(p) else None


for run, cfg in [(a.split(':')[0], a.split(':')[1]) for a in sys.argv[1:]]:
    print(f'\n### {run} {cfg}\n')
    print('| yapı | görünen (ortanca) 5 / 50 / 200 | tik p95 | sahne boyama p95 (sürükleme) | sahne kurma p95 | boyama geçişi p95 | sürüklemede kare p99 | uzun kare payı (ortanca) | koşu koşu uzun kare payı 5 ; 50 ; 200 | sahne boyama sayısı | geçti |')
    print('|---|---|---|---|---|---|---|---|---|---|---|')
    for n in names:
        d = load(run, n)
        if not d:
            continue
        c = d['configs'][cfg]
        bd = c['by_depth']
        ds = ['5', '50', '200']
        med = lambda xs: sorted(xs)[len(xs) // 2]
        row = [n,
               ' / '.join(fmt(med(bd[k]['visible'])) for k in ds),
               ' / '.join(fmt(bd[k]['tick_p95']) for k in ds),
               ' / '.join(fmt(bd[k]['scene_drag_p95']) for k in ds),
               ' / '.join(fmt(bd[k].get('scene_build_drag_p95')) for k in ds),
               ' / '.join(fmt(bd[k].get('paint_pass_drag_p95')) for k in ds),
               ' / '.join(fmt(bd[k]['drag_frame_p99']) for k in ds),
               ' / '.join(fmt(bd[k]['drag_long']) for k in ds),
               ' ; '.join(', '.join(fmt(x) for x in bd[k]['drag_long_runs']) for k in ds),
               ' / '.join(fmt(bd[k]['scene_drag_n']) for k in ds),
               'evet' if all(bd[k]['pass'] for k in ds) else 'hayır (' + ', '.join(k for k in ds if not bd[k]['pass']) + ')']
        print('| ' + ' | '.join(row) + ' |')
