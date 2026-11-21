#!/usr/bin/env python3
"""R5B GOAL criterion 2: "every item is traceable to a fact" — audit of 50 random read
answers of the agent surface (thin, principal `claude`) on the synthetic R5B session.

Deterministic: a fixed pool of read calls (every instrument x the read tools that take
one, every capsule, the ledger, the person's attention list, ...), a fixed sample
(random.Random(50)), a fresh server. For every sampled answer, every capsule (`cap.`),
finding (`fnd.`) and fact (`fx.`) id it carries is resolved with `query_get` (capsules
via `query_capsule`): a capsule must lead to its finding and >= 1 fact; a finding to >= 1
fact; every fact must exist and carry an `evidence_ref` (the bars it rests on). Aggregate
answers (ledger rows) carry no item; they are counted apart and checked for their key.

  python3 rounds/R5B/experiments/trace50/trace.py > rounds/R5B/experiments/trace50/RESULT.json
"""
import json, os, random, re, subprocess, sys, time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
H = os.path.join(ROOT, "rounds", "R5B", "experiments", "harness")
sys.path.insert(0, os.path.join(H, "driver"))
QP = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "qp")
ID = re.compile(r"\b(?:cap|fnd|fx)\.[A-Za-z0-9_.:-]*[A-Za-z0-9]")


def main():
    import tempfile, urllib.request
    tmp = tempfile.mkdtemp(prefix="trace50-")
    err = os.path.join(tmp, "err")
    p = subprocess.Popen([QP, "mcp", "serve", "--http", "--port", "0", "--init",
                          os.path.join(H, "session", "init.r5b.json"), "--root",
                          os.path.join(H, "session"), "--source", "synthetic",
                          "--principal", "claude"], cwd=ROOT, stdout=subprocess.DEVNULL,
                         stderr=open(err, "w"))
    for _ in range(100):
        time.sleep(0.1)
        m = re.search(r"127\.0\.0\.1:\d+", open(err).read())
        if m:
            break
    url = f"http://{m.group(0)}/mcp"
    sid = [None]
    n = [0]

    def rpc(method, params):
        n[0] += 1
        params = dict(params)
        params["_meta"] = {"io.modelcontextprotocol/protocolVersion": "2026-07-28"}
        h = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream",
             "Origin": "http://127.0.0.1"}
        if sid[0]:
            h["Mcp-Session-Id"] = sid[0]
        body = json.dumps({"jsonrpc": "2.0", "id": n[0], "method": method, "params": params}).encode()
        with urllib.request.urlopen(urllib.request.Request(url, body, h), timeout=30) as r:
            sid[0] = r.headers.get("Mcp-Session-Id") or sid[0]
            raw = r.read().decode()
        if raw.lstrip().startswith("event") or raw.lstrip().startswith("data"):
            raw = [l[5:] for l in raw.splitlines() if l.startswith("data:")][-1]
        return json.loads(raw)

    rpc("initialize", {"protocolVersion": "2026-07-28", "capabilities": {},
                       "clientInfo": {"name": "trace50", "version": "0"}})

    def call(tool, args):
        r = rpc("tools/call", {"name": tool, "arguments": args})["result"]
        return r.get("structuredContent") or {}

    try:
        caps = call("query_capsules", {})["data"]["capsules"]
        cap_ids = sorted(c["id"] for c in caps)
        syms = sorted({c["instrument"] for c in caps})
        pool = [("query_capsules", {}), ("query_capsules", {"open": True}),
                ("query_ledger", {}), ("query_attention", {"principal": "arkin"})]
        for s in syms:
            pool += [("query_capsules", {"instrument": s}), ("query_stories", {"instrument": s}),
                     ("query_capsules", {"instrument": s, "open": True})]
        pool += [("query_capsule", {"capsule": c}) for c in cap_ids]
        sample = random.Random(50).sample(pool, 50)
        cache = {}

        def facts_of(i):
            if i in cache:
                return cache[i]
            ok, why = False, ""
            if i.startswith("fx."):
                g = call("query_get", {"id": i}).get("data", {}).get("item")
                ok = bool(g) and "evidence_ref" in g
                why = "" if ok else "fact missing or without evidence_ref"
            elif i.startswith("fnd."):
                g = call("query_get", {"id": i}).get("data", {}).get("item") or {}
                fx = [x for x in json.dumps(g).split('"') if x.startswith("fx.")]
                ok = bool(fx) and all(facts_of(x)[0] for x in set(fx))
                why = "" if ok else "finding without a resolvable fact"
            else:
                d = call("query_capsule", {"capsule": i}).get("data", {}).get("detail") or {}
                fnd = (d.get("item") or {}).get("finding")
                fx = [f.get("id") for f in d.get("facts") or []]
                ok = bool(fnd) and facts_of(fnd)[0] and bool(fx) and all(facts_of(x)[0] for x in fx)
                why = "" if ok else "capsule without finding/facts"
            cache[i] = (ok, why)
            return cache[i]

        rows = []
        for tool, args in sample:
            ans = call(tool, args)
            text = json.dumps(ans, ensure_ascii=False)
            ids = sorted(set(ID.findall(text)))
            if tool == "query_ledger":
                keys = [r.get("key") for r in ans.get("data", {}).get("rows", [])]
                rows.append({"tool": tool, "args": args, "kind": "aggregate",
                             "rows": len(keys), "all_keyed": all(keys)})
                continue
            bad = [i for i in ids if not facts_of(i)[0]]
            rows.append({"tool": tool, "args": args, "kind": "items", "ids": len(ids),
                         "untraceable": bad, "error": ans.get("type") == "error"})
        items = [r for r in rows if r["kind"] == "items"]
        out = {"sampled": len(rows), "pool": len(pool),
               "answers_with_items": sum(1 for r in items if r["ids"]),
               "answers_empty": sum(1 for r in items if not r["ids"]),
               "aggregate_answers": len(rows) - len(items),
               "ids_checked": len(cache),
               "untraceable_answers": sum(1 for r in items if r["untraceable"]),
               "untraceable_ids": sorted({i for r in items for i in r["untraceable"]}),
               "errors": sum(1 for r in items if r["error"]), "rows": rows}
        print(json.dumps(out, ensure_ascii=False, indent=1))
    finally:
        p.terminate()
        p.wait()
        for f in os.listdir(tmp):
            os.remove(os.path.join(tmp, f))
        os.rmdir(tmp)


if __name__ == "__main__":
    main()
