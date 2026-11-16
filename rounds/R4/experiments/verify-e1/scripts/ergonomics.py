"""GOAL 13 ergonomics through `qp serve` (verify-e1): own messages, answers checked against
the full `capsules` list. Usage: python3 -I ergonomics.py <qp> <init.json>"""
import json, subprocess, sys
qp, init = sys.argv[1], sys.argv[2]
msgs = [
 {"conn":"a","msg":{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}}},
 {"conn":"a","msg":{"type":"query","id":"all","query":{"kind":"capsules"}}},
 # capsule with selectors (own choices)
 {"conn":"a","msg":{"type":"query","id":"q1","query":{"kind":"capsule","capsule":{"instrument":"THYAO","meaning":"continuation","latest":True}}}},
 {"conn":"a","msg":{"type":"query","id":"q2","query":{"kind":"capsule","capsule":{"meaning":"breakout","direction":"down","open":False,"first":True}}}},
 {"conn":"a","msg":{"type":"query","id":"q3","query":{"kind":"capsule","capsule":{"stage":"realized","latest":True}}}},
 {"conn":"a","msg":{"type":"query","id":"q4","query":{"kind":"capsule","capsule":{"meaning":"flow"}}}},
 {"conn":"a","msg":{"type":"query","id":"q5","query":{"kind":"capsule","capsule":{"meaning":"squeeze","direction":"up"}}}},
 {"conn":"a","msg":{"type":"query","id":"q6","query":{"kind":"capsule","capsule":{"meening":"flow"}}}},
 # capsules latest / born filters
 {"conn":"a","msg":{"type":"query","id":"l1","query":{"kind":"capsules","latest":True,"meaning":"extreme"}}},
 {"conn":"a","msg":{"type":"query","id":"l2","query":{"kind":"capsules","born_since_bar":100}}},
 {"conn":"a","msg":{"type":"query","id":"l3","query":{"kind":"capsules","born_since_bar":100,"open":True,"latest":True}}},
 # finding selector with stage/open
 {"conn":"a","msg":{"type":"query","id":"f1","query":{"kind":"present","finding":{"meaning":"breakout","open":False,"latest":True}}}},
 {"conn":"a","msg":{"type":"query","id":"f2","query":{"kind":"present","finding":{"instrument":"THYAO","stage":"weakened","latest":True}}}},
 # §20.3 audience format
 {"conn":"b","msg":{"type":"hello","protocol":"qp/0","principal":"ana","surface":{"kind":"phone"}}},
 {"conn":"b","msg":{"type":"command","id":"au1","ops":[{"op":"set_audience","audience":{"group":"future","sensitivity":"balanced","watchlist":["THYAO"],"positions":{"THYAO":-25}}}]}},
 {"conn":"b","msg":{"type":"command","id":"au2","ops":[{"op":"set_sensitivity","sensitivity":None}]}},
 {"conn":"b","msg":{"type":"command","id":"au3","ops":[{"op":"set_audience","audience":{"group":"future","risk":"high"}}]}},
 {"conn":"b","msg":{"type":"query","id":"au4","query":{"kind":"capsules","audience":True}}},
 {"conn":"b","msg":{"type":"query","id":"au5","query":{"kind":"document"}}},
 {"conn":"a","msg":{"type":"query","id":"au6","query":{"kind":"document"}}},
]
p = subprocess.run([qp, "serve", "--init", init, "--batch-end"], input="\n".join(json.dumps(m) for m in msgs) + "\n",
                   capture_output=True, text=True)
out = [json.loads(l) for l in p.stdout.splitlines() if l.strip()]
by = {}
for o in out:
    m = o.get("msg", {})
    if "id" in m: by[m["id"]] = o
allc = by["all"]["msg"]["data"]["capsules"]
print("capsules:", len(allc))
def ids(lst): return [c["id"] for c in lst]
def show(i):
    m = by.get(i, {}).get("msg", {})
    if m.get("type") == "error":
        e = m["error"]; return f"ERROR {e['code']} {e.get('message','')[:120]} did_you_mean={e.get('did_you_mean')} cand={str(e.get('candidates'))[:120]}"
    d = m.get("data", {})
    if d.get("kind") == "capsule_detail":
        it = d["detail"]["item"]; return f"{it['id']} stage={it['stage']} born={it['born_bar']} last={it['last_bar']} resolved={m.get('resolved')}"
    if d.get("kind") == "capsules": return f"{len(d['capsules'])}: {ids(d['capsules'])[:6]}"
    return json.dumps(m)[:300]
for i in ["au5","au6"]:
    m = by.get(i, {}).get("msg", {}); print(i, "audiences:", json.dumps(m.get("data",{}).get("document",{}).get("audiences")))
for i in ["q1","q2","q3","q4","q5","q6","l1","l2","l3","f1","f2","au1","au2","au3","au4"]:
    print(i, show(i))
# independent expectations
CLOSED = {"realized","invalidated","expired","retracted"}
def order_key(c): return (c["last_bar"], c["born_bar"])
th_cont = [c for c in allc if c["instrument"]=="THYAO" and c["kind"]=="continuation"]
print("expect q1 (latest = max last transition bar, then born):", sorted(th_cont, key=order_key)[-1]["id"] if th_cont else None, "| candidates", [(c['id'],c['last_bar']) for c in th_cont])
bd = [c for c in allc if c["kind"]=="breakout" and c["direction"]=="down" and c["stage"] in CLOSED]
print("expect q2 (first closed down breakout):", sorted(bd, key=order_key)[0]["id"] if bd else None)
rz = [c for c in allc if c["stage"]=="realized"]
print("expect q3:", sorted(rz, key=order_key)[-1]["id"] if rz else None, [(c['id'],c['last_bar'],c['born_bar']) for c in sorted(rz,key=order_key)[-3:]])
print("expect q4: ambiguous if >1 flow:", len([c for c in allc if c["kind"]=="flow"]))
ex = [c for c in allc if c["kind"]=="extreme"]
print("expect l1 latest born extreme:", sorted(ex, key=lambda c:(c["born_bar"],c["id"]))[-1]["id"] if ex else None)
print("expect l2 born>=100:", len([c for c in allc if c["born_bar"]>=100]))
o3 = [c for c in allc if c["born_bar"]>=100 and c["stage"] not in CLOSED]
print("expect l3:", sorted(o3, key=lambda c:(c["born_bar"],c["id"]))[-1]["id"] if o3 else None)
print("stderr:", p.stderr[:300])
