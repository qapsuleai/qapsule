import json, math, sys
root = sys.argv[1]; run = sys.argv[2]
names = json.load(open(f'{root}/runs/m17/key.json'))
sc = json.load(open(f'{root}/{run}/scores.json'))

def wilson(k, n, z=1.959964):
    if n == 0: return (float('nan'),) * 3
    p = k / n; d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d; h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return p, c - h, c + h

order = ['selvi', 'tunca', 'marla', 'r4', 'bozuk']
code = {v: k for k, v in names.items()}
print('| dil | persona | koşul | S1 | S2 | S3 | S4 | S5 | S6 |')
print('|---|---|---|---|---|---|---|---|---|')
for nm in order:
    r = sc['readers'][code[nm]]
    for p in ('F', 'H'):
        for c in ('yuz', 'acik'):
            cells = []
            for q in ('S1', 'S2', 'S3', 'S4', 'S5', 'S6'):
                t = r[p][c][q]; k, n = t['k'], t['n']
                pp, lo, hi = wilson(k, n)
                cells.append(f'{pp:.2f} [{lo:.2f}; {hi:.2f}]')
            print(f'| `{nm}` | {p} | {c} | ' + ' | '.join(cells) + ' |')
