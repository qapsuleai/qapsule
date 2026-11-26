#!/usr/bin/env python3
"""Çevrimdışı öz sınama: sunucu (gerçek), model ve ağ yok.

    python3 -I selftest.py

1. Sahte bir MCP sunucusu (Streamable HTTP, yalnız standart kütüphane; PS §3 arayüzünün
   cevap biçimleri) yerel bir portta açılır; `qmcpx` R5B `qmcp` istemcisiyle ona bağlanır ve
   hazır senaryoları (görev başına bir koşu klasörü) oynar: check, uydurma işlev, ret, öneri,
   E_TRY_ONLY, karar aracı denemesi, 4. teslimin reddi.
2. `driver.engine_check` sahte bir `qp` betiğiyle (komut satırı biçimi PS §3.3) hazır motor
   çıktılarını engine/ altına yazar; çıktılar RX4 `expected/` dosyalarından üretilir
   (`conform.to_engine_shape`), yanlış olanlar bir barı bozularak.
3. `score.py` bunları puanlar; toplamlar beklenenle karşılaştırılır; iki puanlama bayt eşit.
Geçici dosyalar sınama bitince silinir.
"""
import http.server
import importlib.machinery
import importlib.util
import io
import json
import os
import re
import shutil
import sys
import tempfile
import threading
from contextlib import redirect_stdout
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "rx4"))
import bars as barsmod  # noqa: E402
import conform  # noqa: E402
import driver  # noqa: E402
import score  # noqa: E402
from rx4common import DEFAULT_RX4, PROGRAMS_DIR, load_json  # noqa: E402

FAILS = []


def check(cond, what):
    if not cond:
        FAILS.append(what)
        print("  FAIL:", what)


# ------------------------------------------------------------------ sahte MCP sunucusu

def program_check_reply(args):
    # the thin tools: `program` is the text (a string) or the IR (an object)
    prog = args.get("program")
    src = prog if isinstance(prog, str) else None
    if not args.get("inputs"):
        return {"type": "error", "error": {"code": "invalid", "reason": "E_NOT_BAR_SERIES",
                                           "message": "inputs: one bar series"}}
    if isinstance(src, str):
        m = re.search(r"\b(zscore|parabolic_sar)\s*\(", src)
        if m:
            line = next(i for i, ln in enumerate(src.splitlines(), 1) if m.group(1) in ln)
            col = src.splitlines()[line - 1].index(m.group(1)) + 1
            return {"type": "result", "data": {"kind": "program_check", "ok": False, "errors": [
                {"code": "schema", "reason": "E_UNKNOWN_OP",
                 "message": f"unknown function `{m.group(1)}`",
                 "span": {"line": line, "col": col, "len": len(m.group(1))},
                 "did_you_mean": ["stdev"]}], "warnings": []}}
        rj = re.match(r"^\s*reject\s+(E_[A-Z_]+)\s+\"(.*)\"", src)
        if rj:
            return {"type": "result", "data": {"kind": "program_check", "ok": True, "errors": [],
                                               "reject": {"code": rj.group(1), "reason": rj.group(2)}}}
        return {"type": "result", "data": {"kind": "program_check", "ok": True, "errors": [],
                                           "warnings": [], "ir": {"version": "qp-ir/0"}}}
    if isinstance(prog, dict) and "reject" in prog:
        return {"type": "result", "data": {"kind": "program_check", "ok": True, "errors": [],
                                           "reject": prog["reject"]}}
    return {"type": "result", "data": {"kind": "program_check", "ok": True, "errors": []}}


def tool_reply(name, args):
    if name in ("program_check", "program_try", "program_explain"):
        return program_check_reply(args)
    if name == "program_add":
        src = args.get("program") if isinstance(args.get("program"), str) else ""
        if re.search(r"^\s*(signal|state)\b", src, re.M):
            return {"type": "error", "error": {
                "code": "unsupported", "reason": "E_TRY_ONLY", "message": "signal output: try only",
                "errors": [{"code": "unsupported", "reason": "E_TRY_ONLY",
                            "message": "signal output: try only"}]}}
        return {"type": "ack", "created": ["prog-1-0.out"], "proposal": "prop-1",
                "status": "proposed", "seq": 1}
    return {"type": "error", "error": {"code": "not_found", "message": f"no tool {name}"}}


class Mock(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        method, mid = body.get("method"), body.get("id")
        if mid is None:
            self.send_response(202)
            self.end_headers()
            return
        if method == "initialize":
            res = {"protocolVersion": "2025-11-25", "capabilities": {"tools": {}},
                   "serverInfo": {"name": "mock", "version": "0"}}
        elif method == "tools/list":
            res = {"tools": [{"name": t, "description": "mock", "inputSchema": {"type": "object"}}
                             for t in score.PROGRAM_TOOLS]}
        elif method == "tools/call":
            p = body["params"]
            if "source" in (p.get("arguments") or {}):  # as qp-mcp: deny_unknown_fields
                self.reply({"jsonrpc": "2.0", "id": mid, "error": {
                    "code": -32602, "message": f"tool `{p['name']}`: unknown field `source`"}})
                return
            rep = tool_reply(p["name"], p.get("arguments") or {})
            res = {"structuredContent": rep, "content": [{"type": "text", "text": json.dumps(rep)}]}
        else:
            res = {}
        self.reply({"jsonrpc": "2.0", "id": mid, "result": res})

    def reply(self, obj):
        out = json.dumps(obj).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Mcp-Session-Id", "mock-session")
        self.send_header("Content-Length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)


# ------------------------------------------------------------------ koşu oynatma

def load_qmcpx():
    loader = importlib.machinery.SourceFileLoader("qmcpx_mod", str(HERE / "qmcpx"))
    spec = importlib.util.spec_from_loader("qmcpx_mod", loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


class Agent:
    def __init__(self, qx, url, root, model, tid):
        self.qx, self.wd = qx, root / f"{model}-{tid}"
        self.wd.mkdir(parents=True)
        driver.dump(self.wd / "meta.json", {"run": self.wd.name, "model": model, "task": tid,
                                            "tokens": 100000, "tool_uses": 5})
        self.env = {"QMCP_URL": url, "QMCP_PRINCIPAL": "claude", "QMCP_RUN": self.wd.name,
                    "QMCP_STATE": str(self.wd / "mcp.sid"), "QMCPX_DIR": str(self.wd),
                    "QMCPX_INPUTS": '["rx4"]'}
        self.k = 0

    def run(self, *argv):
        os.environ.update(self.env)
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = self.qx.main(list(argv))
        return rc, buf.getvalue()

    def write(self, text, ext="qp"):
        self.k += 1
        p = self.wd / f"prog{self.k}.{ext}"
        p.write_text(text, encoding="utf-8")
        return str(p)

    def check(self, text):
        return self.run("call", "program_check", "--program-file", self.write(text),
                        '{"inputs":["rx4"]}')

    def submit(self, text, ext="qp"):
        return self.run("submit", self.write(text, ext))

    def canned(self, n, outputs_by_set=None, check_obj=None):
        cd = self.wd / "canned"
        cd.mkdir(exist_ok=True)
        for s, obj in (outputs_by_set or {}).items():
            driver.dump(cd / f"{n}.{s}.json", obj)
        if check_obj is not None:
            driver.dump(cd / f"{n}.check.json", check_obj)


FAKE_QP = r'''
import json, os, sys
a = sys.argv[1:]
prog = a[-1]
d = os.path.join(os.path.dirname(os.path.dirname(prog)), "canned")
n = os.path.basename(prog).split(".")[0]
if a[:2] == ["program", "check"]:
    p = os.path.join(d, n + ".check.json")
    print(open(p).read() if os.path.exists(p) else json.dumps({"data": {"kind": "program_check", "ok": True}}))
    sys.exit(0)
data = a[a.index("--data") + 1]
s = os.path.basename(data).split(".")[0]
p = os.path.join(d, f"{n}.{s}.json")
if not os.path.exists(p):
    print(json.dumps({"type": "error", "error": {"code": "schema", "reason": "E_PARSE"}}))
    sys.exit(1)
print(open(p).read())
'''


def engine_outputs(tid, s, times, break_bar=None):
    """RX4 beklenen çıktısından motor biçimi (PS §3.3 `--full`); break_bar ile bir seri
    değeri ya da olay barı bozulur."""
    exp = load_json(DEFAULT_RX4 / "expected" / tid / f"{s}.json")
    outs = []
    for name, o in exp["outputs"].items():
        o = json.loads(json.dumps(o))
        if break_bar is not None:
            if o["kind"] == "series":
                i = next(i for i, v in enumerate(o["values"]) if v is not None and i >= break_bar)
                o["values"][i] += 1.0
            elif o["kind"] == "event":
                o["bars"] = [b for b in o["bars"] if b != o["bars"][0]]
        outs.append(dict(name=name, **conform.to_engine_shape(o, None, times)))
    return {"type": "result", "data": {"kind": "program_run", "outputs": outs}}


def main():
    tmp = Path(tempfile.mkdtemp(prefix="rl1-agents-selftest-"))
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Mock)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{srv.server_address[1]}/mcp"
    try:
        root = tmp / "runs"
        qx = load_qmcpx()
        times = {s: barsmod.bar_times(barsmod.series_from_rx4(DEFAULT_RX4, s))
                 for s in ("seen", "heldout")}
        ok = lambda tid: {s: engine_outputs(tid, s, times[s]) for s in times}  # noqa: E731

        # 1. G1-01: check, submit (doğru, metin), öneri, bir karar aracı denemesi
        a = Agent(qx, url, root, "haiku", "G1-01")
        src = "r = rsi(close, 14)\nevent r4m_ust = cross_above(r, 70)\n"
        a.check(src)
        rc, out = a.submit(src)
        check(rc == 0 and json.loads(out)["submission"] == 1, "G1-01 submit")
        a.canned(1, ok("G1-01"))
        a.run("call", "program_add", '{"program": "r = rsi(close, 14)\\nevent r4m_ust = cross_above(r, 70)\\n", "inputs": ["rx4"]}')
        a.run("call", "approve_proposal", '{"id": "p1"}')

        # 2. G2-01: uydurma işlev (E_UNKNOWN_OP), sonra yanlış teslim, sonra doğru
        a = Agent(qx, url, root, "haiku", "G2-01")
        rc, _ = a.run("call", "program_check", '{"source": "x = 1", "inputs": ["rx4"]}')
        check(rc == 1, "qmcpx: unknown argument `source` is a transport error (exit 1)")
        rc, out = a.check("z = zscore(close, 20)\nseries zq3_skor = z\n")
        check('"E_UNKNOWN_OP"' in out, "G2-01 mock E_UNKNOWN_OP")
        good = ("m = sma(close, 20)\nz = (close - m) / stdev(close, 20)\nseries zq3_skor = z\n"
                "event zq3_asiri = cross_above(z, 2)\n")
        a.check(good)
        a.submit(good)
        a.canned(1, {s: engine_outputs("G2-01", s, times[s], break_bar=500) for s in times})
        a.submit(good)
        a.canned(2, ok("G2-01"))

        # 3. G4-04 tuzak: doğru ret
        a = Agent(qx, url, root, "haiku", "G4-04")
        rej = 'reject E_LOOKAHEAD "the open 4h bar close is a future value"\n'
        a.check(rej)
        a.submit(rej)
        a.canned(1, check_obj={"data": {"kind": "program_check", "ok": True,
                                        "reject": {"code": "E_LOOKAHEAD"}}})

        # 4. G5-03: ret, ama kabul dışı kod
        a = Agent(qx, url, root, "haiku", "G5-03")
        a.check(rej)
        a.submit(rej)

        # 5. G7-01: yanlış ret; sinyalli programı program_add'e verme (E_TRY_ONLY)
        a = Agent(qx, url, root, "haiku", "G7-01")
        sig = ("r = rsi(close, 14)\nsignal f6g_dip = cross_above(r, 30) side al target 4% "
               "stop 2% max_bars 24 source \"rsi-dip\" \"1\"\n")
        a.run("call", "program_add", json.dumps({"program": sig, "inputs": ["rx4"]}))
        a.submit('reject E_UNSUPPORTED "signals are not supported"\n')

        # 6. G3-02: teslimsiz
        a = Agent(qx, url, root, "haiku", "G3-02")
        a.check("e = cooldown(cross_above(close, sma(close, 20)), 10)\nevent x = e\n")

        # 7. G1-03: ara gösterimle teslim; seen doğru, heldout yanlış (M5); check'siz teslim
        a = Agent(qx, url, root, "haiku", "G1-03")
        ir_text = (PROGRAMS_DIR / "G1-03.qp.json").read_text(encoding="utf-8")
        a.submit(ir_text, ext="json")
        a.canned(1, {"seen": engine_outputs("G1-03", "seen", times["seen"]),
                     "heldout": engine_outputs("G1-03", "heldout", times["heldout"], break_bar=900)})

        # 8. G2-03: üç teslim, dördüncüsü reddedilir
        a = Agent(qx, url, root, "haiku", "G2-03")
        lvl = "lvl = highest(high, 20)[1]\nseries h9n_seviye = lvl\nevent h9n_kir = close > lvl\n"
        a.check(lvl)
        for i in (1, 2, 3):
            rc, _ = a.submit(lvl)
            a.canned(i, ok("G2-03"))
        rc, out = a.submit(lvl)
        check(rc == 2 and "refused" in out, "G2-03 4th submit refused")
        check(len(score.submissions_of(a.wd)) == 3, "G2-03 three submissions kept")

        # motor adımı: sahte qp
        fake = tmp / "fakeqp.py"
        fake.write_text(FAKE_QP, encoding="utf-8")
        qp = f"{sys.executable} -I {fake}"
        for rd in sorted(root.iterdir()):
            driver.engine_check(rd, qp=qp)

        # puanlama
        truth = score.Truth(DEFAULT_RX4, ["seen", "heldout"])
        res = score.score_root(root, ["haiku"], truth)
        t = res["haiku"]["totals"]
        exp = {"tasks": 8, "first_try": 3, "final": 4, "silent_wrong": 2, "false_rejects": 1,
               "m5_seen_ok_heldout_wrong": 1, "no_submission": 1, "submissions": 10,
               "decision_attempts": 1, "offsurface_calls": 1, "submitted_without_check": 2,
               "failed_check_sessions": 2}
        for k, v in exp.items():
            check(t[k] == v, f"totals.{k} = {t[k]} (beklenen {v})")
        check(t["m3"] == {"first_wrong": 5, "recovered": 1}, f"m3 {t['m3']}")
        check(t["after_error"] == {"tasks_with_error_reply": 2, "final_correct": 1},
              f"after_error {t['after_error']}")
        check(t["reject_needed"] == {"tasks": 2, "final_ok": 1, "first_ok": 1, "program_attempts": 0},
              f"reject_needed {t['reject_needed']}")
        check(t["invented"]["names"] == {"zscore": ["G2-01"]}, f"invented {t['invented']}")
        check(t["surface"] == {"text": 9, "ir": 1}, f"surface {t['surface']}")
        check(t["proposals"]["proposed"] == 1 and t["proposals"]["try_only_replies"] == 1,
              f"proposals {t['proposals']}")
        statuses = {r["task"]: [s["status"] for s in r["submissions"]] for r in res["haiku"]["rows"]}
        check(statuses == {"G1-01": ["correct"], "G1-03": ["silent_wrong"],
                           "G2-01": ["silent_wrong", "correct"],
                           "G2-03": ["correct", "correct", "correct"], "G3-02": [],
                           "G4-04": ["correct"], "G5-03": ["reject_wrong_code"],
                           "G7-01": ["false_reject"]}, f"statuses {statuses}")
        md1 = score.fmt_md(res, truth, root)
        md2 = score.fmt_md(score.score_root(root, ["haiku"], truth), truth, root)
        check(md1 == md2, "two scorings byte-equal")
        check("belirgin altında" in md1, "verdict line present")

        # hata sayımında tekrar yok (PS §3.1: üst düzey hata errors[]'in ilkini tekrarlar)
        e = score.reply_errors({"type": "error", "error": {"code": "schema", "reason": "E_UNKNOWN_OP",
                                "errors": [{"reason": "E_UNKNOWN_OP"}, {"reason": "E_TYPE"}]}})
        check([x["reason"] for x in e] == ["E_UNKNOWN_OP", "E_TYPE"], f"reply_errors {e}")

        # istem ve oturum dosyaları
        os.environ["QP_RL1_AGENTS"] = str(tmp / "drv")
        wd = tmp / "drv" / "opus-G8-03"
        wd.mkdir(parents=True)
        sd = driver.write_session(wd)
        init, doc = load_json(sd / "init.json"), load_json(sd / "document.json")
        sb = doc["series"][0]["data"]["bars"]
        seen = load_json(driver.DATA_DIR / "seen.bars.json")["data"]["bars"]
        check(len(sb) == 3001 and sb[:3000] == seen and doc["series"][0]["id"] == "rx4",
              "session document: RX4 seen + one sentinel bar")
        # the last bar is live (PR §1): the sentinel, so every RX4 bar is closed
        check(sb[-1]["t"] == seen[-1]["t"] + 3_600_000 and sb[-1]["v"] == 0.0
              and init["clock"] == sb[-1]["t"] + 3_600_000, "closed RX4 bars (sentinel live bar)")
        check(any(p["id"] == "claude" and p["author"]["kind"] == "agent" for p in init["principals"]),
              "agent principal")
        task = {t["id"]: t for t in conform.load_tasks(DEFAULT_RX4)}["G8-03"]
        pf = driver.write_prompt(wd, "opus-G8-03", "http://127.0.0.1:1/mcp", task)
        pt = pf.read_text(encoding="utf-8")
        check("{QMCPX}" not in pt and "{CARD}" not in pt and "{WORKDIR}" not in pt,
              "placeholders filled")
        check('"kind": "state"' in pt and "fingerprint" not in pt.split("İstenen çıktılar")[1],
              "fingerprint → state in prompt")
        check(task["text"] in pt and str(driver.CARD) in pt and "QMCPX_DIR=" in pt, "prompt body")
        # alt ajan dökümünden belirteç (aynı ileti kimliği bir kez) ve yalıtım taraması
        tp = tmp / "agent-x.jsonl"
        u1 = {"input_tokens": 2, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 0,
              "output_tokens": 10}
        rows = [{"type": "user", "message": {"content": f"Read {wd}/prompt.md"}},
                {"type": "assistant", "message": {"id": "m1", "model": "claude-haiku-5-5", "usage": u1,
                 "content": [{"type": "tool_use", "name": "Read", "input": {"file_path": str(driver.CARD)}}]}},
                {"type": "assistant", "message": {"id": "m1", "model": "claude-haiku-5-5", "usage": u1,
                 "content": [{"type": "tool_use", "name": "Read",
                              "input": {"file_path": str(driver.REPO / "docs" / "PROGRAMS.md")}}]}},
                {"type": "assistant", "message": {"id": "m2", "model": "claude-haiku-5-5",
                 "usage": dict(u1, cache_read_input_tokens=100, cache_creation_input_tokens=0),
                 "content": [{"type": "tool_use", "name": "Bash",
                              "input": {"command": f"python3 {driver.QMCPX} call program_check '{{}}'"}}]}}]
        tp.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")
        u = driver.usage_of(tp)
        check(u["tokens"] == 224 and u["requests"] == 2 and u["tool_uses"] == 3,
              f"usage_of {u['tokens']} {u['requests']} {u['tool_uses']}")
        v = driver.violations(wd, u)
        check(len(v) == 1 and "PROGRAMS.md" in v[0], f"violations {v}")
        check("PREAMBLE-START" not in pt and "İşaretlerin dışı" not in pt, "preamble markers cut")
    finally:
        srv.shutdown()
        shutil.rmtree(tmp, ignore_errors=True)
    print("selftest:", "OK" if not FAILS else f"{len(FAILS)} FAIL")
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
