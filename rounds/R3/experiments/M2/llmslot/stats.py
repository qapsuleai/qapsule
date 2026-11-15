"""Summary of the two generation runs: rejections, fallbacks, cost, latency, run-to-run equality.

    python3 -I stats.py            # reads log.run1.json, log.run2.json, sentences*.json
"""

import json
import statistics as st
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import slots as S  # noqa: E402
import verify as V  # noqa: E402

out = {}
for run, logf in ((1, "log.run1.json"), (2, "log.run2.json")):
    p = HERE / logf
    if not p.exists():
        continue
    res = json.loads(p.read_text())["results"]
    atts = [a for r in res for a in r["attempts"]]
    secs = sorted(a["seconds"] for a in atts)
    first = sorted(r["attempts"][0]["seconds"] for r in res)
    rej = {"tr": 0, "en": 0}
    reasons = {}
    for a in atts:
        for l, e in a["verdict"].items():
            if e:
                rej[l] += 1
                for x in e:
                    k = x.split(":")[0] + ":" + x.split(":")[1][:40] if ":" in x else x
                    reasons[k] = reasons.get(k, 0) + 1
    cost = sum(a["cost_usd"] for a in atts)
    total_secs = {r["id"]: sum(a["seconds"] for a in r["attempts"]) for r in res}
    out[run] = {
        "situations": len(res),
        "calls": len(atts),
        "retry_calls": len(atts) - len(res),
        "situations_needing_retry": sum(len(r["attempts"]) > 1 for r in res),
        "rejected_texts": rej,
        "reasons": dict(sorted(reasons.items(), key=lambda kv: -kv[1])),
        "fallbacks": sum(len(r["fallback"]) for r in res),
        "cost_usd": round(cost, 4),
        "cost_per_sentence_usd": round(cost / (2 * len(res)), 5),
        "cost_per_situation_usd": round(cost / len(res), 5),
        "call_seconds_median": round(st.median(secs), 2),
        "call_seconds_p90": round(secs[int(0.9 * (len(secs) - 1))], 2),
        "call_seconds_max": round(secs[-1], 2),
        "first_call_seconds_median": round(st.median(first), 2),
        "situation_seconds_incl_retries_p90": round(
            sorted(total_secs.values())[int(0.9 * (len(total_secs) - 1))], 2),
        "situation_seconds_incl_retries_max": round(max(total_secs.values()), 2),
        "tokens_in_mean": round(st.mean(a["input_tokens"] for a in atts)),
        "tokens_out_mean": round(st.mean(a["output_tokens"] for a in atts)),
        "models": sorted({a["model"] for a in atts if a["model"]}),
    }

s1 = json.loads((HERE / "sentences.json").read_text())["sentences"]
p2 = HERE / "sentences.run2.json"
if p2.exists():
    s2 = json.loads(p2.read_text())["sentences"]
    eq = {l: sum(s1[k][l] == s2[k][l] for k in s1) for l in ("tr", "en")}
    out["run_to_run_byte_equal"] = {l: f"{eq[l]}/{len(s1)}" for l in eq}
    out["run_to_run_byte_equal_rate"] = {l: round(eq[l] / len(s1), 4) for l in eq}
    # re-verify run 2 independently (the verifier is deterministic)
    inputs = {x["id"]: x for x in json.loads((HERE / "inputs.json").read_text())}
    bad = 0
    for k, v in s2.items():
        sl = S.build(inputs[k])
        for l in ("tr", "en"):
            bad += bool(V.check(sl, l, v[l], S.allowed_values(sl)))
    out["run2_verifier_failures_after_fallback"] = bad
print(json.dumps(out, ensure_ascii=False, indent=1))
