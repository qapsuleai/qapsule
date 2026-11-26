#!/usr/bin/env python3
"""RL1/U3 — canlı oturum saldırıları (`qp serve`, stdin/stdout): günlük şişirme, kişi başı
bütçe, politika, veri yetkisi, süreç güvenliği. Sonuç: results-session.json (bu klasörde).

  python3 run_session.py        (QP_BIN, ABUSE_WORK isteğe bağlı; bkz. qpabuse.py)

İki oturum: A (init'te `agent_policy` yok: ajan kuralı yalnız barındırıcının işaretlediği
bağlantıda, `qp serve` hiçbirini işaretlemez) ve B (`agent_policy: "r5b"`: ajan kuralı her
bağlantıda — `qp mcp serve`'ün bağlantı işaretinin eşdeğeri).
"""
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from qpabuse import HERE, HOUR, WORK, Serve, bars, ir, write  # noqa: E402

R = []
N_BARS = 3000


def rec(cls, aid, inp, expected, observed, ok, ms=None, rss=None, verdict=None):
    v = verdict if (verdict and not ok) else ("OK" if ok else "FAIL")
    R.append({"class": cls, "id": aid, "input": inp, "expected": expected, "observed": observed,
              "verdict": v, "ms": ms, "rss_mb": rss})
    print(f"[{v}] {aid}: {observed}"[:300], flush=True)


def init(policy=None, name="init.json"):
    b = bars(N_BARS)
    doc = {"schema_version": "0", "id": "doc-abuse", "title": "abuse", "display_tz": "UTC",
           "series": [
               {"id": "px", "instrument": {"symbol": "ABUSE"}, "interval": "1h",
                "source": {"kind": "inline"}, "data": {"kind": "bars", "bars": b}},
               {"id": "bx", "instrument": {"symbol": "THYAO"}, "interval": "1h",
                "source": {"kind": "bind", "feed": "bist", "symbol": "THYAO", "interval": "1h",
                           "window": {"last": 10000}}}],
           "views": [{"id": "main", "title": "m", "x": {"kind": "time"}, "y": [{"kind": "linear"}],
                      "layers": [{"mark": "candle", "series": "px", "panel": 0, "encoding": {}}]}],
           "facts": [], "findings": [], "capsules": [], "intents": [], "ledger": {"entries": []}}
    i = {"document": doc, "clock": b[-1]["t"] + 2 * HOUR, "epoch": 0, "proposal_ttl_ms": None,
         "data": {"entitlements": {"arkin": ["*"], "deniz": ["*"], "claude": ["*"], "scout": ["*"],
                                   "can": [], "host": ["*"]},
                  "feeds": [{"feed": {"feed": "bist", "symbol": "THYAO", "interval": "1h"},
                             "synthetic": {"n": 300, "seed": 7}}]},
         "principals": [
             {"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"},
             {"id": "deniz", "author": {"kind": "human", "name": "deniz"}, "role": "editor"},
             {"id": "claude", "author": {"kind": "agent", "name": "claude"}, "role": "proposer",
              "on_behalf": "arkin"},
             {"id": "scout", "author": {"kind": "agent", "name": "scout"}, "role": "editor",
              "on_behalf": "arkin"},
             {"id": "can", "author": {"kind": "human", "name": "can"}, "role": "viewer"},
             {"id": "host", "author": {"kind": "engine", "name": "host", "version": "0"}, "role": "host"}]}
    if policy:
        i["agent_policy"] = policy
    return write(name, i)


SMA = ir([{"id": "c", "op": "input", "field": "close"}, {"id": "m", "op": "sma", "x": "c", "period": 10}],
         [{"name": "y", "kind": "series", "node": "m"}])
EVT = ir([{"id": "c", "op": "input", "field": "close"}, {"id": "m", "op": "sma", "x": "c", "period": 10},
          {"id": "x", "op": "cross_above", "a": "c", "b": "m"}],
         [{"name": "e", "kind": "event", "node": "x"}, {"name": "m", "kind": "series", "node": "m"}])
# live_per_bar 666 (one output, stdev 664 + input 0 + output 1 → per_bar 665; cone 664+1).
HEAVY = ir([{"id": "c", "op": "input", "field": "close"}, {"id": "s", "op": "stdev", "x": "c", "period": 665}],
           [{"name": "y", "kind": "series", "node": "s"}])
# Four series outputs sharing stdev 600: per_bar 604 within 666, live_per_bar 4 × 601 = 2404.
WIDE = ir([{"id": "c", "op": "input", "field": "close"}, {"id": "s", "op": "stdev", "x": "c", "period": 600}],
          [{"name": f"o{i}", "kind": "series", "node": "s"} for i in range(4)])
SIGNAL = ir([{"id": "c", "op": "input", "field": "close"}, {"id": "k", "op": "const", "value": True}],
            [{"name": "s", "kind": "signal", "node": "k", "side": "al", "rule": {"max_bars": 10},
              "source": {"strategy": "x", "version": "1"}}])


def fat():
    """~8 KB canonical IR, 4 series outputs, one int param: the log-bloat vehicle."""
    nodes = [{"id": "c", "op": "input", "field": "close"},
             {"id": "m", "op": "sma", "x": "c", "period": {"param": "len"}}]
    prev = "m"
    k = 0
    while True:
        nid = f"n{k:03d}_" + "x" * 58
        cand = nodes + [{"id": nid, "op": "abs", "x": prev}]
        p = ir(cand, [{"name": f"o{i}", "kind": "series", "node": nid} for i in range(4)],
               [{"key": "len", "kind": "int", "min": 2, "max": 50, "default": 10}])
        if len(json.dumps(p, separators=(",", ":"))) > 8000:
            break
        nodes = cand
        prev = nid
        k += 1
    return ir(nodes, [{"name": f"o{i}", "kind": "series", "node": prev} for i in range(4)],
              [{"key": "len", "kind": "int", "min": 2, "max": 50, "default": 10}])


class Sess:
    def __init__(self, path):
        self.s = Serve(path, WORK)
        self.k = 0
        for c, p in (("a", "arkin"), ("d", "deniz"), ("c", "claude"), ("s", "scout"), ("v", "can")):
            self.s.hello(c, p)

    def cmd(self, conn, ops, **kw):
        self.k += 1
        m = {"type": "command", "id": f"k{self.k}", "ops": ops}
        m.update(kw)
        return self.s.reply(conn, m)

    def q(self, conn, query):
        self.k += 1
        return self.s.reply(conn, {"type": "query", "id": f"q{self.k}", "query": query})

    def doc(self):
        r, _ = self.q("a", {"kind": "document"})
        return (r.get("data") or {}).get("document") or r.get("data") or {}


def err(r):
    e = r.get("error") or {}
    return f"{e.get('code')}/{e.get('reason')}"


def is_err(r, code=None, reason=None):
    e = r.get("error") or {}
    return r.get("type") == "error" and (code is None or e.get("code") == code) and \
        (reason is None or e.get("reason") == reason or
         any(x.get("reason") == reason for x in e.get("errors") or []))


def short(r):
    if r.get("type") == "error":
        e = r["error"]
        return f"error {e.get('code')}/{e.get('reason')} at {e.get('path')}"
    if r.get("type") == "ack":
        return f"ack {r.get('status')} {r.get('proposal') or ''} created {len(r.get('created') or [])}"
    return json.dumps(r)[:160]


# ------------------------------------------------------------------ log bloat & budget

def bloat(S):
    C = "4-günlük"
    # B01: ops per command.
    ops = [{"op": "add_program", "id": f"b1_{i}", "program": SMA, "inputs": ["px"]} for i in range(9)]
    r, ms = S.cmd("a", ops)
    rec(C, "B01", "tek komutta 9 add_program (sınır 8)", "limit/E_TOO_LARGE, hiçbiri uygulanmaz",
        short(r), is_err(r, "limit", "E_TOO_LARGE"), ms, S.s.peak)
    ops = [{"op": "set_program_params", "program": "nope", "params": {}}] * 64
    r, ms = S.cmd("a", ops)
    rec(C, "B02", "tek komutta 64 set_program_params", "limit/E_TOO_LARGE (/ops/8)",
        short(r), is_err(r, "limit", "E_TOO_LARGE"), ms)
    # B03: instances per owner (32).
    t = time.monotonic()
    got = 0
    last = None
    for i in range(40):
        r, ms = S.cmd("a", [{"op": "add_program", "id": f"i{i:02d}", "program": SMA, "inputs": ["px"]}])
        if r.get("type") == "ack":
            got += 1
        else:
            last = r
            break
    rec(C, "B03", "arkin 40 kez add_program (sınır 32 örnek)", "32 kabul, 33. limit/E_BUDGET",
        f"{got} kabul, sonra {short(last) if last else 'hiç ret yok'}",
        got == 32 and last is not None and is_err(last, "limit", "E_BUDGET"),
        round((time.monotonic() - t) * 1000), S.s.peak)
    # B04: undo of a remove at the cap.
    r1, _ = S.cmd("a", [{"op": "remove_program", "program": "i00"}])
    r2, _ = S.cmd("a", [{"op": "add_program", "id": "i_re", "program": SMA, "inputs": ["px"]}])
    seq = r1.get("seq")
    r3, ms = S.cmd("a", [{"op": "undo", "seq": seq}])
    rec(C, "B04", "sınırdayken: remove_program, yeni add_program (32), sonra remove'un undo'su (33. örnek)",
        "undo limit/E_BUDGET", f"remove {short(r1)}; add {short(r2)}; undo {short(r3)}",
        r1.get("type") == "ack" and r2.get("type") == "ack" and is_err(r3, "limit", "E_BUDGET"), ms)
    # B05: deniz has her own budget; per document there is no total beyond per-owner (PR §8.2).
    okd = 0
    for i in range(32):
        r, _ = S.cmd("d", [{"op": "add_program", "id": f"d{i:02d}", "program": SMA, "inputs": ["px"]}])
        okd += r.get("type") == "ack"
    n_inst = len({s["source"]["program"]["instance"] for s in S.doc().get("series", [])
                  if (s.get("source") or {}).get("program")})
    rec(C, "B05", "deniz (ikinci kişi) 32 örnek; doküman toplamı", "kişi başı ayrı bütçe (32+32); doküman geneli tavan şartnamede yok",
        f"deniz {okd} kabul; dokümanda {n_inst} örnek", okd == 32 and n_inst == 64)
    for i in range(32):
        S.cmd("d", [{"op": "remove_program", "program": f"d{i:02d}"}])
    for i in range(1, 32):
        S.cmd("a", [{"op": "remove_program", "program": f"i{i:02d}"}])
    S.cmd("a", [{"op": "remove_program", "program": "i_re"}])
    # B06: live cost per owner (2000).
    res = []
    for i in range(4):
        r, ms = S.cmd("d", [{"op": "add_program", "id": f"h{i}", "program": HEAVY, "inputs": ["px"]}])
        res.append(short(r))
    rec(C, "B06", "deniz 4 × stdev(close,665) örneği (live_per_bar 666; kişi sınırı 2000)",
        "3 kabul (1998), 4. limit/E_BUDGET", "; ".join(res),
        res[:3] == [res[0]] * 3 and res[0].startswith("ack") and "E_BUDGET" in res[3])
    r, ms = S.cmd("a", [{"op": "add_program", "id": "wide", "program": WIDE, "inputs": ["px"]}])
    rec(C, "B07", "tek örnek: per_bar 604 (≤666) ama 4 çıktı × koni 601 = live_per_bar 2404",
        "limit/E_BUDGET (canlı maliyet kişi sınırını tek başına aşar)", short(r), is_err(r, "limit", "E_BUDGET"), ms)
    r, ms = S.cmd("d", [{"op": "set_program_params", "program": "h0", "params": {}}])
    rec(C, "B08", "deniz bütçe doluyken h0'a boş set_program_params", "kabul (maliyet değişmez) ya da kodlu ret; çökme yok",
        short(r), r.get("type") in ("ack", "error"), ms)
    for i in range(3):
        S.cmd("d", [{"op": "remove_program", "program": f"h{i}"}])
    # B09: log-bytes churn with an ~8 KB IR × 4 outputs.
    F = fat()
    size = len(json.dumps(F, separators=(",", ":")))
    r, _ = S.cmd("a", [{"op": "add_program", "id": "fat", "program": F, "inputs": ["px"]}])
    assert r.get("type") == "ack", r
    props = []
    for i in range(3):
        r, _ = S.cmd("c", [{"op": "set_program_params", "program": "fat", "params": {"len": 20 + i}}])
        props.append(r.get("proposal") or short(r))
    t = time.monotonic()
    n = 0
    last = None
    mss = []
    while n < 400:
        r, ms = S.cmd("a", [{"op": "set_program_params", "program": "fat", "params": {"len": 10 + (n % 2)}}])
        mss.append(ms)
        if r.get("type") != "ack":
            last = r
            break
        n += 1
    total = round((time.monotonic() - t) * 1000)
    rec(C, "B09", f"set_program_params döngüsü, ara gösterim {size} B × 4 çıktı (günlük sınırı 4 MiB)",
        "≈ 120 değişimden sonra limit/E_BUDGET", f"{n} değişim kabul, sonra {short(last) if last else 'ret yok'}; "
        f"komut başına ort. {round(sum(mss) / len(mss), 1)} ms, en çok {max(mss)} ms",
        last is not None and is_err(last, "limit", "E_BUDGET") and 80 < n < 200, total, S.s.peak)
    # B10: undo/redo churn after the cap (undo writes the series again: counted).
    r, ms = S.cmd("a", [{"op": "undo"}])
    rec(C, "B10", "bütçe dolunca undo (eski değerleri geri yazar)", "limit/E_BUDGET (undo'nun baytı da sayılır)",
        short(r), is_err(r, "limit", "E_BUDGET"), ms)
    # B11: the agent proposes, the human accepts past the cap.
    r0, _ = S.cmd("c", [{"op": "set_program_params", "program": "fat", "params": {"len": 30}}])
    rec(C, "B11a", "bütçe doluyken ajan yeni set_program_params önerir", "öneri oluşurken limit/E_BUDGET (PS §2.3)",
        short(r0), is_err(r0, "limit", "E_BUDGET"))
    r, ms = S.cmd("a", [{"op": "accept_proposal", "proposal": props[0]}])
    rec(C, "B11", "dolmadan önce verilmiş ajan önerisi (set_program_params); bütçe dolunca insan tek başına kabul eder",
        "kabul limit/E_BUDGET, öneri bekler", f"öneriler {props}; kabul {short(r)}",
        all(props) and is_err(r, "limit", "E_BUDGET"), ms)
    def pend():
        q, _ = S.q("a", {"kind": "list", "collection": "proposals"})
        return [p["id"] for p in (q.get("data") or {}).get("items", []) if p.get("status", "pending") == "pending"]

    r, ms = S.cmd("a", [{"op": "accept_proposal", "proposal": props[1]},
                        {"op": "accept_proposal", "proposal": props[2]}])
    rec(C, "B12", "aynı, ama iki accept_proposal tek komutta (Check fazı yalnız tek başına kabulü denetler)",
        "limit/E_BUDGET ya da kabul hiç olmaz", short(r) + " — " + str((r.get("error") or {}).get("message")),
        is_err(r, "limit", "E_BUDGET") or is_err(r, "invalid"), ms)
    r, ms = S.cmd("a", [{"op": "accept_proposal", "proposal": props[1]},
                        {"op": "add_comment", "about": {"kind": "document"}, "text": "x", "lang": "tr"}])
    rec(C, "B13", "aynı, accept_proposal + zararsız bir işlem (add_comment) tek komutta", "limit/E_BUDGET ya da kabul hiç olmaz",
        short(r) + " — " + str((r.get("error") or {}).get("message")) + f"; bekleyen öneriler {pend()}",
        r.get("type") == "error" and props[1] in pend() and props[2] in pend(), ms)
    # Over-cap result: the bytes the owner has now.
    lg, _ = S.q("a", {"kind": "log"})
    entries = ((lg.get("data") or {}).get("entries")) or []
    owner_bytes = 0
    for e in entries:
        for c in e.get("changes", []):
            if c.get("op") == "put_series" and ((c.get("series") or {}).get("source") or {}).get("program", {}).get("owner") == "arkin":
                owner_bytes += len(json.dumps(c["series"], separators=(",", ":"), ensure_ascii=False))
    R[-1]["observed"] += f"; arkin'in günlükteki program baytı ≈ {owner_bytes} (sınır 4194304)"
    R[-2]["observed"] += f"; (B12 sonrası)"
    S.cmd("a", [{"op": "remove_program", "program": "fat"}])
    # B14: message size.
    huge = {"op": "add_program", "id": "huge", "program": {"version": "qp-ir/0", "nodes": [], "outputs": [],
                                                           "pad": "x" * 1_100_000}, "inputs": ["px"]}
    r, ms = S.cmd("a", [huge])
    if r.get("type") == "none":
        S.k += 1
        mine, allv, ms = S.s.send("a", {"type": "command", "id": "big", "ops": [huge]})
        r = next((o["msg"] for o in allv if (o.get("msg") or {}).get("type") == "error"), {"type": "none"})
    rec(C, "B14", "1,1 MB'lık komut satırı", "limit (ileti boyutu)", short(r) if r.get("type") != "none" else json.dumps(r)[:200], r.get("type") == "error" and S.s.alive(), ms)


# ------------------------------------------------------------------ policy & data

def policy(S, tag):
    C = "5-politika"
    r, ms = S.cmd("c", [{"op": "add_program", "id": f"pc{tag}", "program": SMA, "inputs": ["px"]}], mode="direct")
    rec(C, f"P01{tag}", "ajan (proposer) add_program mode:direct", "forbidden/proposal_only",
        short(r), is_err(r, "forbidden"), ms)
    r, ms = S.cmd("c", [{"op": "add_program", "id": f"pa{tag}", "program": EVT, "inputs": ["px"]}])
    prop = r.get("proposal")
    ok1 = r.get("status") == "proposed"
    r2, _ = S.cmd("c", [{"op": "accept_proposal", "proposal": prop}])
    r3, _ = S.cmd("d", [{"op": "accept_proposal", "proposal": prop}])
    r4, _ = S.cmd("a", [{"op": "accept_proposal", "proposal": prop}])
    owner = None
    for s in S.doc().get("series", []):
        p = (s.get("source") or {}).get("program")
        if p and p["instance"] == f"pa{tag}":
            owner = p["owner"]
    rec(C, f"P02{tag}", "ajan add_program → öneri; ajan kendi önerisini kabul; editör (deniz, decide yok) kabul; sahip kabul",
        "öneri; ajan forbidden; deniz forbidden; arkin kabul, owner = arkin",
        f"{short(r)}; ajan {short(r2)}; deniz {short(r3)}; arkin {short(r4)}; owner {owner}",
        ok1 and is_err(r2, "forbidden") and is_err(r3) and r4.get("type") == "ack" and owner == "arkin")
    inst = f"pa{tag}"
    r, ms = S.cmd("c", [{"op": "remove_program", "program": inst}])
    rec(C, f"P03{tag}", f"ajan (proposer) remove_program; oturum {tag}", "forbidden/agent_cannot_write (öneri bile değil)", short(r),
        is_err(r, "forbidden"), ms, verdict="KOŞULLU" if tag == "A" else None)
    r, ms = S.cmd("s", [{"op": "add_program", "id": f"ps{tag}", "program": SMA, "inputs": ["px"]}])
    expect_direct = tag == "A"
    rec(C, f"P04{tag}", f"yazma yetkili ajan (scout, editor) add_program; oturum {tag}",
        "öneri (PS §2.2: ajan yalnız önerir)", short(r) + ("  [ajan kuralı bağlantı işaretine bağlı: `qp serve` işaretlemez, init'te agent_policy yok]" if expect_direct else ""),
        r.get("status") == "proposed", verdict="KOŞULLU" if tag == "A" else None)
    r, ms = S.cmd("s", [{"op": "remove_program", "program": inst}])
    rec(C, f"P05{tag}", f"yazma yetkili ajan remove_program; oturum {tag}", "forbidden/agent_cannot_write",
        short(r), is_err(r, "forbidden"), ms, verdict="KOŞULLU" if tag == "A" else None)
    # Poking program series directly (a fresh instance of the owner: P05 may have removed `inst`).
    inst = f"tg{tag}"
    S.cmd("a", [{"op": "add_program", "id": inst, "program": EVT, "inputs": ["px"]}])
    ser = next((s for s in S.doc().get("series", []) if s["id"] == f"{inst}.m"), None)
    forged = json.loads(json.dumps(ser))
    forged["source"]["program"]["owner"] = "nobody"
    r, _ = S.cmd("a", [{"op": "put_series", "series": forged}])
    rec(C, f"P06{tag}", "insan put_series: program serisini sahibi `nobody` yapılmış kopyayla yazar (bütçe atlatma)",
        "invalid/E_PROGRAM_SERIES", short(r), is_err(r, "invalid", "E_PROGRAM_SERIES"))
    forged["id"] = f"fresh{tag}"
    r, _ = S.cmd("a", [{"op": "put_series", "series": forged}])
    rec(C, f"P07{tag}", "insan put_series: yeni kimlikle, program alanı taşıyan seri (denetimsiz ara gösterim)",
        "invalid/E_PROGRAM_SERIES", short(r), is_err(r, "invalid", "E_PROGRAM_SERIES"))
    r, _ = S.cmd("c", [{"op": "put_series", "series": forged}])
    rec(C, f"P08{tag}", "ajan put_series program alanıyla", "E_PROGRAM_SERIES ya da forbidden",
        short(r), r.get("type") == "error")
    r, _ = S.cmd("a", [{"op": "set_params", "series": f"{inst}.m", "params": {"period": 2}}])
    rec(C, f"P09{tag}", "set_params program serisine", "invalid/E_PROGRAM_SERIES", short(r), is_err(r, "invalid", "E_PROGRAM_SERIES"))
    r, _ = S.cmd("a", [{"op": "remove", "id": f"{inst}.e"}])
    rec(C, f"P10{tag}", "remove tek çıktı", "invalid/E_PROGRAM_SERIES", short(r), is_err(r, "invalid", "E_PROGRAM_SERIES"))
    b = bars(N_BARS + 1)[-1]
    r, _ = S.cmd("a", [{"op": "append_bars", "series": f"{inst}.m", "bars": [b]}])
    r2, _ = S.cmd("a", [{"op": "update_last_bar", "series": f"{inst}.e", "bar": b}])
    rec(C, f"P11{tag}", "append_bars / update_last_bar program serisine", "ret (bar serisi değil)",
        f"{short(r)}; {short(r2)}", r.get("type") == "error" and r2.get("type") == "error")
    cmp_ = {"kind": "compare", "left": f"{inst}.e", "op": "gt", "right": "px"}
    r, _ = S.cmd("a", [{"op": "add_intent", "text": "x", "trigger": cmp_}])
    r2, _ = S.cmd("a", [{"op": "add_finding", "claim": "x", "facts": [], "confidence": 0.5,
                         "invalidation": dict(cmp_, left="px", right=f"{inst}.e")}])
    rec(C, f"P12{tag}", "olay serisi compare işleneni (add_intent, add_finding)", "unsupported/E_EVENT_OPERAND",
        f"{short(r)}; {short(r2)}", is_err(r, "unsupported", "E_EVENT_OPERAND") and is_err(r2, None, "E_EVENT_OPERAND"))
    r, _ = S.cmd("a", [{"op": "add_program", "id": f"sg{tag}", "program": SIGNAL, "inputs": ["px"]}])
    rec(C, f"P13{tag}", "add_program sinyal çıktılı", "unsupported/E_TRY_ONLY", short(r), is_err(r, "unsupported", "E_TRY_ONLY"))
    r, _ = S.cmd("a", [{"op": "add_program", "id": f"pp{tag}", "program": SMA, "inputs": [f"{inst}.m"]}])
    rec(C, f"P14{tag}", "add_program girdisi program serisi", "invalid/E_NOT_BAR_SERIES", short(r), is_err(r, "invalid", "E_NOT_BAR_SERIES"))
    r, _ = S.cmd("a", [{"op": "add_program", "id": f"po{tag}", "program": SMA, "inputs": ["px"], "owner": "deniz"}])
    r2, _ = S.cmd("a", [{"op": "add_program", "id": f"pt{tag}", "source": "series y = close", "inputs": ["px"]}])
    rec(C, f"P15{tag}", "add_program `owner` alanı (sahip sahteciliği) / metin `source`", "schema (bilinmeyen alan)",
        f"{short(r)}; {short(r2)}", is_err(r, "schema") and is_err(r2, "schema"))
    # Entitlement: can (no data entitlement) on the bound series bx.
    r, _ = S.cmd("a", [{"op": "add_program", "id": f"pb{tag}", "program": SMA, "inputs": ["bx"]}])
    q1, _ = S.q("v", {"kind": "try_program", "program": SMA, "inputs": ["bx"]})
    q2, _ = S.q("v", {"kind": "try_program", "instance": f"pb{tag}"})
    q3, _ = S.q("v", {"kind": "explain_program", "instance": f"pb{tag}", "at": {"output": "y", "t": 1771891200000}})
    q4, _ = S.q("v", {"kind": "check_program", "program": SMA, "inputs": ["bx"]})
    leak = json.dumps(q4.get("data") or {})
    rec(C, f"P16{tag}", "yetkisiz okuyucu (can): try_program bx; try_program instance (bx'e bağlı); explain_program at; check_program inputs bx",
        "try/explain no_entitlement; check bar verisi sızdırmaz",
        f"add {short(r)}; try {err(q1)}; try-inst {err(q2)}; explain-at {err(q3) if q3.get('type') == 'error' else 'CEVAP: ' + json.dumps(q3.get('data'))[:150]}; check bars={((q4.get('data') or {}).get('cost') or {}).get('bars')}",
        is_err(q1, "no_entitlement") and is_err(q2, "no_entitlement") and is_err(q3))
    q5, _ = S.q("a", {"kind": "try_program", "program": SMA, "inputs": ["px"],
                      "range": {"from_t": -9223372036854775808, "to_t": 18446744073709551615}})
    q6, _ = S.q("a", {"kind": "try_program", "program": SMA, "inputs": ["px", "bx"]})
    q7, _ = S.q("a", {"kind": "try_program", "program": SMA, "source": "series y = close", "inputs": ["px"]})
    rec(C, f"P17{tag}", "try_program uç aralık (i64 min .. u64 max); program+source birlikte",
        "kodlu ret, çökme yok; ikisi birden invalid", f"{short(q5)[:80]}; {short(q7)}",
        S.s.alive() and q5.get("type") == "error" and is_err(q7, "invalid"))
    q8, _ = S.q("v", {"kind": "try_program", "program": SMA, "inputs": ["px", "bx"]})
    q9, _ = S.q("a", {"kind": "check_program", "program": SMA, "inputs": ["px", "bx"]})
    rec(C, f"P18{tag}", "try_program / check_program iki girdiyle (px, bx); yetkisiz can için de",
        "E_NOT_BAR_SERIES (PR §11.2 \"yok, iki tane\"; add_program öyle yapıyor)",
        f"try(arkin) {short(q6)[:60]}; try(can) {short(q8)[:60]}; check {('ok=' + str((q9.get('data') or {}).get('ok'))) if q9.get('type') == 'result' else short(q9)} — fazla girdi sessizce yok sayılıyor (ilki kullanılıyor; sıra bx,px ise can no_entitlement alıyor: yetki atlatma yok)",
        is_err(q6, None, "E_NOT_BAR_SERIES") and is_err(q9, None, "E_NOT_BAR_SERIES"))
    S.cmd("a", [{"op": "remove_program", "program": f"pb{tag}"}])
    r, _ = S.cmd("a", [{"op": "remove_program", "program": inst}])
    return r


def lookahead_session(S):
    """No retroactive trigger (PS §1.2 `since_t`): a value change that turns every past bar
    into an event fires nothing; a bar closed after `since_t` does (positive control)."""
    C = "3-gelecek"
    lv = ir([{"id": "c", "op": "input", "field": "close"}, {"id": "k", "op": "const", "value": {"param": "level"}},
             {"id": "g", "op": "gt", "a": "c", "b": "k"}], [{"name": "e", "kind": "event", "node": "g"}],
            [{"key": "level", "kind": "float", "min": 0, "max": 1000000, "default": 1000000}])
    S.cmd("a", [{"op": "add_program", "id": "lv", "program": lv, "inputs": ["px"]}])
    S.s.send("a", {"type": "subscribe", "id": "sub-i", "topic": {"kind": "intents"}})
    S.cmd("a", [{"op": "add_intent", "text": "lv", "trigger": {"kind": "threshold", "series": "lv.e", "op": "ge", "value": 1}}])
    _, after_set, _ = S.s.send("a", {"type": "command", "id": "spp", "ops": [
        {"op": "set_program_params", "program": "lv", "params": {"level": 1.0}}]})
    tr, _ = S.q("a", {"kind": "try_program", "instance": "lv"})
    n_ev = ((tr.get("data") or {}).get("event") or [{}])[0].get("events")
    fired = lambda lines: [o["msg"].get("type") for o in lines if o.get("msg", {}).get("type") in ("notify",)
                           and "intent" in json.dumps(o["msg"])]
    nb = bars(N_BARS + 3)[N_BARS:]
    _, after1, _ = S.s.send("a", {"type": "command", "id": "ap1", "ops": [{"op": "append_bars", "series": "px", "bars": nb[:1]}]})
    _, after3, _ = S.s.send("a", {"type": "command", "id": "ap3", "ops": [{"op": "append_bars", "series": "px", "bars": nb[1:3]}]})
    rec(C, "L20", "geriye dönük tetik: niyet `lv.e ge 1`; set_program_params her geçmiş barı olay yapar (dene: olay sayısı); sonra since_t'den önce ve sonra kapanan barlar",
        "değişimde ve since_t öncesi kapanan barda bildirim yok; since_t sonrası kapanan barda var",
        f"dene olayları {n_ev}; değişimde {len(fired(after_set))}; 1 bar sonra {len(fired(after1))}; 3 bar sonra {len(fired(after3))}",
        (n_ev or 0) > 1000 and not fired(after_set) and not fired(after1) and len(fired(after3)) >= 1,
        verdict="SINANAMADI" if (n_ev or 0) > 1000 and not fired(after_set) and not fired(after3) else None)
    if R[-1]["verdict"] == "SINANAMADI":
        R[-1]["observed"] += (" — olumlu denetim de tetiklemedi: bu dalda (rl1/program) qp-plane program serisi hesabı "
                              "(RL1/PA, main'de 356bb3dc) yok; `study`/`at` sorguları program serisini görmüyor. "
                              "Geriye dönük tetik yokluğu bu yüzden kanıt değil; birleştirme sonrası yeniden koşulmalı")
    S.cmd("a", [{"op": "remove", "id": "intent-x"}])


def safety(S):
    C = "6-süreç"
    lines = [
        "{not json",
        json.dumps({"conn": "a", "msg": {"type": "command", "id": "z", "ops": [
            {"op": "add_program", "id": "deep", "program": json.loads("[" * 120 + "]" * 120), "inputs": ["px"]}]}}),
        '{"conn":"a","msg":' + "[" * 100000 + "]" * 100000 + "}",
        json.dumps({"conn": "a", "msg": {"type": "query", "id": "z2", "query": {"kind": "check_program",
                                                                                "source": "x = " + "(" * 50000 + "close" + ")" * 50000 + "\nseries s = x"}}}),
        json.dumps({"conn": "a", "msg": {"type": "query", "id": "z3", "query": {"kind": "check_program",
                                                                                "source": "x = " + " + ".join(["close"] * 15000) + "\nseries s = x"}}}),
        json.dumps({"conn": "a", "msg": {"type": "query", "id": "z4", "query": {"kind": "explain_program",
                                                                                "program": {"version": "qp-ir/0", "nodes": [{"id": "c", "op": "input", "field": "close"}] * 5000, "outputs": []}}}}),
    ]
    obs = []
    t = time.monotonic()
    for ln in lines:
        S.s.p.stdin.write(ln + "\n")
        S.s.p.stdin.flush()
        while True:
            l = S.s.p.stdout.readline()
            if not l:
                obs.append("EXIT")
                break
            v = json.loads(l)
            if v.get("batch_end"):
                break
            m = v.get("msg") or {}
            if m.get("type") == "error":
                e = m["error"]
                obs.append(f"{e.get('code')}/{e.get('reason')}")
            elif m.get("type") == "result":
                d = m.get("data") or {}
                obs.append(f"result ok={d.get('ok')} {[x.get('reason') for x in d.get('errors', [])][:2]}")
    ms = round((time.monotonic() - t) * 1000)
    alive = S.s.alive()
    r, _ = S.q("a", {"kind": "check_program", "program": SMA})
    rec(C, "X01", "oturuma bozuk JSON, 120 derin ara gösterim, 100 000 derin JSON satırı, 50 000 parantez ve 15 000 terimli metin, 5000 düğümlü explain",
        "her biri kodlu hata; süreç ayakta; sonraki sorgu cevaplanır", f"{obs}; ayakta {alive}; sonraki {short(r)[:40]}",
        alive and r.get("type") == "result" and "EXIT" not in obs, ms, S.s.peak)


def main():
    t = time.monotonic()
    A = Sess(init(None, "init-a.json"))
    bloat(A)
    policy(A, "A")
    lookahead_session(A)
    safety(A)
    peak_a = A.s.peak
    code_a = A.s.close()
    B = Sess(init("r5b", "init-b.json"))
    policy(B, "B")
    peak_b = B.s.peak
    code_b = B.s.close()
    rec("6-süreç", "X02", "iki oturumun sonu", "süreç temiz çıkar (0)", f"A çıkış {code_a} (en çok {peak_a} MB), B çıkış {code_b} (en çok {peak_b} MB)",
        code_a == 0 and code_b == 0, round((time.monotonic() - t) * 1000), max(peak_a, peak_b))
    out = HERE / "results-session.json"
    out.write_text(json.dumps(R, ensure_ascii=False, indent=1) + "\n")
    print(f"{sum(r['verdict'] == 'OK' for r in R)}/{len(R)} OK -> {out}")


if __name__ == "__main__":
    main()
