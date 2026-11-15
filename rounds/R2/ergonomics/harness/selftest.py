#!/usr/bin/env python3
"""Scorer self-test: id-based reference solutions for R01-R20 must pass, an empty attempt
must fail every task. Runs against the base binary (r2/int). usage: selftest.py [variant]"""

import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import score  # noqa: E402
import setup  # noqa: E402
import t9  # noqa: E402


def solutions(m):
    V = m["main"]
    hello = {"conn": "a", "msg": {"type": "hello", "protocol": "qp/0", "principal": "editor",
                                  "surface": {"kind": "headless"}}}

    def cmd(*ops):
        return [hello, {"conn": "a", "msg": {"type": "command", "id": "c1", "ops": list(ops)}}]

    def q(query):
        return [hello, {"conn": "a", "msg": {"type": "query", "id": "q1", "query": query}}]

    S = {
        "R01": cmd({"op": "add_study", "id": "myrsi7", "study": "rsi", "inputs": [m["px"]], "params": {"period": 7}},
                   {"op": "insert_panel", "view": V, "panel": {"id": 7, "y": {"kind": "linear"}}, "after": 0,
                    "layers": [{"mark": "line", "series": "myrsi7", "panel": 7}]}),
        "R02": cmd({"op": "set_encoding", "view": V, "layer": m["L-bb"], "encoding": {"y": "middle"}},
                   {"op": "insert_layer", "view": V, "layer": {"mark": "line", "series": m["bb20"], "panel": 0,
                                                               "encoding": {"y": "upper", "color": "down"}}},
                   {"op": "insert_layer", "view": V, "layer": {"mark": "line", "series": m["bb20"], "panel": 0,
                                                               "encoding": {"y": "lower", "color": "#90a4ae"}}}),
        "R03": cmd({"op": "set_emphasis", "view": V,
                    "emphasis": [{"target": {"item": m["fnd-brk-new"]}, "kind": "highlight"}]}),
        "R04": q({"kind": "present", "finding": m["fnd-brk-new"], "surface": "watch"}),
        "R05": cmd({"op": "set_scale", "view": V, "scale": "y0", "kind": "log"}),
        "R06": cmd({"op": "insert_layer", "view": V, "layer": {"mark": "rule", "panel": 1, "encoding": {"y": 30}}}),
        "R07": cmd({"op": "delete_panel", "view": V, "panel": 2}),
        "R08": cmd({"op": "insert_panel", "view": V, "panel": {"id": 9, "y": {"kind": "linear"}}, "after": 0,
                    "layers": [{"mark": "bar", "series": m["px"], "panel": 9, "encoding": {"y": "v"}}]}),
        "R09": cmd({"op": "insert_layer", "view": V, "layer": {"mark": "band", "series": m["bb20"], "panel": 0,
                                                               "encoding": {"y": "upper", "y2": "lower", "opacity": 0.2}}}),
        "R10": cmd({"op": "insert_layer", "view": V, "layer": {"mark": "bar", "series": m["macd"], "panel": 2,
                    "encoding": {"y": "histogram", "color": {"when": {"field": "histogram", "op": "ge", "than": 0},
                                                             "then": "up", "else": "down"}}}}),
        "R11": cmd({"op": "insert_layer", "view": V, "layer": {"mark": "line", "series": m["px"], "panel": 0,
                    "encoding": {"y": "c", "color": {"when": {"field": "c", "op": "ge",
                                                              "than": {"series": m["sma50"], "field": "sma"}},
                                                     "then": "up", "else": "down"}}}}),
        "R12": cmd({"op": "set_view_range", "view": V, "from_bar": 140, "to_bar": 199}),
        "R13": q({"kind": "screen", "principal": "arkin"}),
        "R14": q({"kind": "hit", "view": V, "x": 900, "y": 200, "width": 1200, "height": 600}),
        "R15": cmd({"op": "set_encoding", "view": V, "layer": m["L-rsi"], "encoding": {"width": 2}}),
        "R16": cmd({"op": "insert_layer", "view": V, "layer": {"mark": "point", "fact": m["fact-brk-old"], "panel": 0}}),
        "R17": cmd({"op": "add_fact", "id": "lvl12345", "kind": "level", "series": m["px"], "bar": 199,
                    "text": "Kullanıcı seviyesi 123,45", "evidence": {"level": 123.45}},
                   {"op": "insert_layer", "view": V, "layer": {"mark": "rule", "panel": 0, "encoding": {"y": 123.45},
                    "origin": {"/encoding/y": {"kind": "fact", "fact": "lvl12345", "field": "level"}}}}),
        "R18": cmd({"op": "move_panel", "view": V, "panel": 2, "before": 1}),
        "R20": cmd({"op": "insert_panel", "view": V, "panel": {"id": 8, "y": {"kind": "linear"}},
                    "layers": [{"mark": "line", "series": m["bb20"], "panel": 8, "encoding": {"y": "bandwidth"}}]}),
        "R19": q({"kind": "present", "finding": m["fnd-brk-new"], "surface": "phone"}),
    }
    A = {"R04": "Saatte şu söylenir: «THYAO 122,40 direncini kırdı: yeni zirve denemesi.»",
         "R13": "Ekranda 3 panel var: fiyat (mumlar + Bollinger), RSI(14) ve 70 çizgisi, MACD.",
         "R19": "Telefonda kapsül derinliğinde görünür; son 60 bar gösterilir."}
    return S, A


def main(variant="base"):
    s = setup.make(12345)
    m = s["mapping"]
    tmp = tempfile.mkdtemp()
    json.dump(s["doc"], open(os.path.join(tmp, "doc.json"), "w"), ensure_ascii=False)
    json.dump(s["init"], open(os.path.join(tmp, "init.json"), "w"))
    S, A = solutions(m)
    bad = 0
    for t in sorted(S):
        lines = [json.dumps(x, ensure_ascii=False) for x in S[t]]
        shown, outs, _ = t9.run_lines(t9.qp_bin(variant), tmp, s, lines)
        rp, fp = os.path.join(tmp, f"{t}.jsonl"), os.path.join(tmp, f"{t}.full.jsonl")
        open(rp, "w").write("\n".join(lines) + "\n")
        open(fp, "w").write("\n".join(outs) + "\n")
        c = score.ctx(rp, fp)
        ans = A.get(t, "")
        if t == "R14":
            h = c["hit"]
            ans = f"Bar {h['bar']}, değer {h['value']:.2f}."
        why = score.check(t, c, m, ans)
        # empty attempt (hello only) must fail
        e_lines = lines[:1]
        _, e_outs, _ = t9.run_lines(t9.qp_bin(variant), tmp, s, e_lines)
        open(rp, "w").write("\n".join(e_lines) + "\n")
        open(fp, "w").write("\n".join(e_outs) + "\n")
        ewhy = score.check(t, score.ctx(rp, fp), m, "")
        ok = (not why) and bool(ewhy)
        bad += not ok
        print(f"{t}: reference {'pass' if not why else 'FAIL ' + str(why)}; empty {'fails' if ewhy else 'PASSES'}"
              + ("" if c["errors"] == [] else f"  errors={[e['error'].get('message') for e in c['errors']]}"))
    print("selftest:", "OK" if not bad else f"{bad} problems")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(*(sys.argv[1:2] or ["base"])))
