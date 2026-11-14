#!/usr/bin/env python3
"""Scores an ergonomics round: score.py <round-dir> -> markdown table + JSON summary.
<round-dir>/agent*/G??/attemptN.jsonl (+ .full.jsonl from the run, answer.txt)."""
import glob, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ergo

def main(d):
    rows = []
    for task_dir in sorted(glob.glob(os.path.join(d, "agent*", "G??"))):
        task = os.path.basename(task_dir)
        agent = os.path.basename(os.path.dirname(task_dir))
        atts = sorted(glob.glob(os.path.join(task_dir, "attempt?.jsonl")))
        ans = ergo.answer_text(os.path.join(task_dir, "answer.txt"))
        res = []
        for a in atts:
            if not os.path.exists(a + ".full.jsonl"):
                res.append({"file": os.path.basename(a), "pass": False, "why": ["never run"]})
                continue
            why = ergo.chk(task, ergo.ctx(a), ans)
            lines = ergo.read_attempt(a)
            nmsg = len(lines)
            nbytes = sum(len(l.encode()) for l in lines)
            errs = sum(1 for o in ergo.parse_full(a + ".full.jsonl")
                       if o.get("conn") in set(ergo.agent_conns(lines)) and o.get("msg", {}).get("type") == "error")
            res.append({"file": os.path.basename(a), "pass": not why, "why": why, "msgs": nmsg,
                        "bytes": nbytes, "tokens_est": round(nbytes / 4), "errors": errs})
        first = res[0]["pass"] if res else False
        anyp = any(r["pass"] for r in res)
        rows.append({"task": task, "agent": agent, "attempts": len(res), "first": first, "eventually": anyp,
                     "detail": res})
    print("| Görev | Ajan | Deneme | İlk denemede | Sonunda | Mesaj (ilk) | ~Belirteç (ilk) | Hata yanıtı (ilk) | Başarısızlık nedeni (ilk) |")
    print("|---|---|---|---|---|---|---|---|---|")
    for r in rows:
        f = r["detail"][0] if r["detail"] else {}
        print(f"| {r['task']} | {r['agent'][-1]} | {r['attempts']} | {'✓' if r['first'] else '✗'} | "
              f"{'✓' if r['eventually'] else '✗'} | {f.get('msgs','')} | {f.get('tokens_est','')} | "
              f"{f.get('errors','')} | {'; '.join(f.get('why', [])) or '—'} |")
    n = len(rows)
    fp = sum(r["first"] for r in rows)
    ev = sum(r["eventually"] for r in rows)
    failed_first = [r for r in rows if not r["first"]]
    corr = sum(r["eventually"] for r in failed_first)
    print(f"\nİlk denemede başarı: {fp}/{n}; sonunda: {ev}/{n}; ilk denemesi başarısız olup düzelen: {corr}/{len(failed_first)}")
    json.dump(rows, open(os.path.join(d, "score.json"), "w"), ensure_ascii=False, indent=1)

if __name__ == "__main__":
    main(sys.argv[1])
