import gzip
import json
import os

S = os.path.dirname(os.path.abspath(__file__))
T = '/Users/arkns/Works/Mchart/qapsule/target/wt'
M = S + '/m18/rounds/R5/experiments/M18/run'
for r in ('run-phone', 'run-jit'):
    os.makedirs(S + '/' + r, exist_ok=True)
    d = json.load(open(M + '/states.json'))
    d['chart_stories']['stories'] = json.load(gzip.open(M + '/chart_stories.json.gz'))
    json.dump(d, open(S + '/' + r + '/states.json', 'w'))
    print(r, len(d['chart_stories']['stories']))
d = [
    {'name': 'pavka-old', 'story': 'pavka', 'qp': T + '/m18-pp-old/release/qp', 'web': S + '/trees/old/shells/web'},
    {'name': 'pavka-new', 'story': 'pavka', 'qp': T + '/m18-pp-new/release/qp', 'web': S + '/trees/new/shells/web'},
    {'name': 'korda', 'story': 'korda', 'qp': T + '/m18-pp-korda/release/qp', 'web': S + '/trees/korda/shells/web'},
    {'name': 'r4', 'story': None, 'qp': T + '/m18-pp-r4/release/qp', 'web': S + '/trees/r4/shells/web'},
]
json.dump(d, open(S + '/designs.json', 'w'), indent=1)
