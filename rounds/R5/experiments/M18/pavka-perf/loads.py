import os
import re

S = os.path.dirname(os.path.abspath(__file__))


def secs(t):
    h, m, s = map(int, t.split(':'))
    return h * 3600 + m * 60 + s


for mlog, llog in (('measure.log', 'load.log'), ('measure2.log', 'load2.log')):
    starts = []
    for line in open(f'{S}/{mlog}'):
        m = re.match(r'== (\S+) .*? (\S+) (phone\S*) reps', line)
        if m:
            starts.append((secs(m.group(1)), m.group(2) + ' ' + m.group(3)))
        elif line.startswith('== ') and 'done' in line:
            starts.append((secs(line.split()[1]), None))
    samples = []
    for line in open(f'{S}/{llog}'):
        p = line.split()
        if len(p) >= 2:
            samples.append((secs(p[0]), float(p[1])))
    for (a, name), (b, _) in zip(starts, starts[1:]):
        xs = sorted(v for t, v in samples if a <= t < b)
        if xs:
            print(f'{mlog} {name}: load1 median {xs[len(xs) // 2]:.1f} (min {xs[0]:.1f}, max {xs[-1]:.1f}, n {len(xs)})')
