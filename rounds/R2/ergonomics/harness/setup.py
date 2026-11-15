#!/usr/bin/env python3
"""R2/T9 hidden setup: the RD1 `thyao` document with freshly randomized ids.

`make(seed)` returns the document, the session init, the id mapping, the hidden setup
lines and the hidden epilogue. Everything is a pure function of the seed (Python's
`random.Random(seed)`), so a run can be rebuilt from the seed recorded in its raw logs.

Hidden state added on top of RD1 (rounds/R2/tasks/TASKS.md): SMA(50) series not in a
view; two `breakout` facts on the price series (bars 120 and 185, `evidence.level`);
two findings on them (old and new breakout) with live capsules; the user `arkin`
connected on a desktop surface.
"""

import copy
import json
import os
import random
import string

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
BASE_DOC = os.path.join(ROOT, "fixtures/protocol/docs/thyao.json")
BASE_INIT = os.path.join(ROOT, "fixtures/protocol/init/thyao.json")

C = 1760000000000
MIN = 60000
NOW = C + 30 * MIN

# Logical names of every id in the setup document. The values are randomized per run.
KEYS = ["px", "rsi14", "macd", "bb20", "sma50", "main",
        "fact-rsi-cross70", "fact-brk-new", "fact-brk-old",
        "fnd-overbought", "fnd-brk-new", "fnd-brk-old",
        "cap-overbought", "cap-brk-new", "cap-brk-old",
        "L-candles", "L-bb", "L-rsi", "L-rule70", "L-macd"]

# Ids that appear in the specs (docs/PROTOCOL.md, docs/GRAMMAR.md, schema descriptions of
# any variant); a randomized id must never collide with these or look like them.
SPEC_IDS = {"px", "rsi14", "rsi7", "macd", "bb20", "bb", "sma20", "sma50", "ema50", "main", "three", "r0",
            "boll", "fact-rsi-cross70", "fnd-overbought", "cap-overbought", "candles", "rsi", "hist",
            "ob", "os", "band", "mid", "sma", "q05", "q95", "f-brk-2", "fnd-9e1"}

ALPH = string.ascii_lowercase + string.digits


def rand_id(rng, used):
    while True:
        s = rng.choice(string.ascii_lowercase) + "".join(rng.choice(ALPH) for _ in range(4))
        # avoid authority-style ids (l3, series-5-0 ...) and spec ids
        if s in used or s in SPEC_IDS or (s[0] == "l" and s[1:].isdigit()):
            continue
        used.add(s)
        return s


def make_mapping(seed):
    rng = random.Random(seed)
    used = set()
    return {k: rand_id(rng, used) for k in KEYS}


def prov(author, based_on, kind="agent", version=None):
    a = {"kind": kind, "name": author}
    if version:
        a["version"] = version
    return {"author": a, "at": C, "based_on": based_on}


def make_doc(m):
    d = json.load(open(BASE_DOC, encoding="utf-8"))
    N = m.__getitem__
    bars = d["series"][0]["data"]["bars"]
    ren = {"px": N("px"), "rsi14": N("rsi14"), "macd": N("macd"), "bb20": N("bb20")}
    for s in d["series"]:
        s["id"] = ren[s["id"]]
        src = s["source"]
        if src.get("kind") == "study":
            src["inputs"] = [ren[i] for i in src["inputs"]]
    inst = copy.deepcopy(d["series"][1]["instrument"])
    d["series"].append({"id": N("sma50"), "instrument": inst, "interval": "1d",
                        "source": {"kind": "study", "study": "sma", "inputs": [N("px")],
                                   "params": {"period": 50.0}}})
    v = d["views"][0]
    v["id"] = N("main")
    lids = [N("L-candles"), N("L-bb"), N("L-rsi"), N("L-rule70"), N("L-macd")]
    for lid, l in zip(lids, v["layers"]):
        l["id"] = lid
        if "series" in l:
            l["series"] = ren[l["series"]]
    # facts: RD1's RSI cross + two breakouts (document order deliberately not bar order)
    f0 = d["facts"][0]
    f0["id"] = N("fact-rsi-cross70")
    f0["series"] = N("rsi14")
    f0["provenance"]["based_on"] = [N("rsi14")]

    def brk(key, bar, level):
        b = bars[bar]
        return {"id": N(key), "kind": "breakout", "series": N("px"), "bar": bar, "t": b["t"],
                "text": f"Kapanış {level:.2f} direncinin üstüne çıktı, bar {bar}.",
                "evidence": {"level": level, "prev": bars[bar - 1]["c"], "value": b["c"]},
                "provenance": prov("qp-facts", [N("px")], "engine", "0.0.0")}

    d["facts"] = [f0, brk("fact-brk-new", 185, 122.40), brk("fact-brk-old", 120, 103.90)]
    fo = d["findings"][0]
    fo["id"] = N("fnd-overbought")
    fo["facts"] = [N("fact-rsi-cross70")]
    fo["invalidation"]["series"] = N("rsi14")
    fo["provenance"]["based_on"] = [N("fact-rsi-cross70")]
    fnew = {"id": N("fnd-brk-new"),
            "claim": "THYAO 122,40 direncini yukarı kırdı; yükseliş yeni zirveyi deniyor.",
            "facts": [N("fact-brk-new")], "confidence": 0.55,
            "invalidation": {"kind": "threshold", "series": N("px"), "op": "lt", "value": 115.0},
            "provenance": prov("scout", [N("fact-brk-new")])}
    fold = {"id": N("fnd-brk-old"),
            "claim": "THYAO 103,90 direncini yukarı kırdı; yatay bant sona erdi.",
            "facts": [N("fact-brk-old")], "confidence": 0.6,
            "invalidation": {"kind": "threshold", "series": N("px"), "op": "lt", "value": 98.0},
            "provenance": prov("scout", [N("fact-brk-old")])}
    d["findings"] = [fo, fnew, fold]
    co = d["capsules"][0]
    co["id"] = N("cap-overbought")
    co["finding"] = N("fnd-overbought")
    co["provenance"]["based_on"] = [N("fact-rsi-cross70")]

    def cap(key, fkey, fact, headline, direction):
        return {"id": N(key), "finding": N(fkey), "state": "live", "revision": 0, "headline": headline,
                "glyphs": [{"kind": "direction", "value": direction}, {"kind": "extremity", "value": 0.5},
                           {"kind": "uncertainty", "value": 0.4}],
                "depth": "capsule", "provenance": prov("scout", [N(fact)])}

    d["capsules"] = [co,
                     cap("cap-brk-new", "fnd-brk-new", "fact-brk-new", "THYAO 122,40 direncini kırdı: yeni zirve denemesi.", 0.7),
                     cap("cap-brk-old", "fnd-brk-old", "fact-brk-old", "THYAO 103,90 direncini kırdı: yatay bant bitti.", 0.5)]
    return d


def make_init():
    i = json.load(open(BASE_INIT, encoding="utf-8"))
    i["document"] = "doc.json"
    return i


def h(conn, principal, kind="headless"):
    return {"conn": conn, "msg": {"type": "hello", "protocol": "qp/0", "principal": principal,
                                  "surface": {"kind": kind}}}


def setup_lines(m):
    return [h("zz-host", "host"), h("zz-ark", "arkin", "desktop"),
            {"conn": "zz-host", "msg": {"type": "clock", "now": NOW}}]


def epilogue_lines(m):
    q = lambda i, query: {"conn": "zz-host", "msg": {"type": "query", "id": i, "query": query}}
    return [q("e-doc", {"kind": "document"}),
            q("e-screen", {"kind": "screen", "principal": "arkin"}),
            q("e-log", {"kind": "log"}),
            # R14 reference: the same point query from a fresh headless connection
            h("zz-ref", "editor"),
            {"conn": "zz-ref", "msg": {"type": "query", "id": "e-hit",
                                       "query": {"kind": "hit", "view": m["main"], "x": 900, "y": 200,
                                                 "width": 1200, "height": 600}}}]


def make(seed):
    m = make_mapping(seed)
    return {"seed": seed, "mapping": m, "doc": make_doc(m), "init": make_init(),
            "setup": setup_lines(m), "epilogue": epilogue_lines(m)}


if __name__ == "__main__":
    import sys
    r = make(int(sys.argv[1]) if len(sys.argv) > 1 else 1)
    print(json.dumps(r["mapping"], indent=1))
