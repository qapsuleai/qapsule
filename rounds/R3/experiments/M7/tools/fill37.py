"""Fills transcript 37's expectations from a `qp serve --host-frames --batch-end` run.

usage: python3 -I fill37.py skeleton.json serve-out.jsonl > 37-dispute-retract.json
`welcome` messages are reduced to `{type, seq}` (as in the other transcripts); every
other message is expected in full. The outputs were read and checked by hand.
"""
import json
import sys

sk = json.load(open(sys.argv[1]))
batches, cur = [], []
for line in open(sys.argv[2]):
    v = json.loads(line)
    if v.get("batch_end"):
        batches.append(cur)
        cur = []
    elif "host" not in v:
        cur.append(v)
assert len(batches) == len(sk["steps"]), (len(batches), len(sk["steps"]))
for step, out in zip(sk["steps"], batches):
    exp = []
    for m in out:
        if m["msg"].get("type") == "welcome":
            m = {"conn": m["conn"], "msg": {"type": "welcome", "seq": m["msg"]["seq"]}}
        exp.append(m)
    step["expect"] = exp
sk["final"] = {"document": {
    "capsules": [{"id": "cap-floor", "state": "invalidated",
                  "story": {"stage": "retracted"}}],
    "ledger": {"entries": []},
}}
print(json.dumps(sk, ensure_ascii=False, indent=2))
