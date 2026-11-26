#!/usr/bin/env python3
"""RL1/U3 — MCP yüzeyi (`qp mcp serve --stdio`, ajan `claude`, bağlantı ajan işaretli):
program araçları ve üretilmiş `op_*` araçlarıyla politika denemeleri. Sonuç: results-mcp.json.

  python3 run_mcp.py            (önce run_session.py init-b.json'u üretmiş olmalı; ya da kendisi üretir)
"""
import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from qpabuse import HERE, QP, WORK  # noqa: E402
from run_session import EVT, SMA, init  # noqa: E402

R = []


def rec(aid, inp, expected, observed, ok):
    R.append({"class": "5-politika", "id": aid, "input": inp, "expected": expected, "observed": observed,
              "verdict": "OK" if ok else "FAIL", "ms": None, "rss_mb": None})
    print(f"[{'OK' if ok else 'FAIL'}] {aid}: {observed}"[:300], flush=True)


class Mcp:
    def __init__(self, init_path, surface):
        self.p = subprocess.Popen([QP, "mcp", "serve", "--stdio", "--principal", "claude", "--surface", surface,
                                   "--init", str(init_path), "--root", str(WORK)],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=open(WORK / "mcp.stderr", "a"), text=True, bufsize=1)
        self.k = 0
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": {"name": "abuse", "version": "0"}})
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        self.p.stdin.flush()

    def rpc(self, method, params):
        self.k += 1
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.k, "method": method, "params": params}) + "\n")
        self.p.stdin.flush()
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise RuntimeError("mcp exited")
            v = json.loads(line)
            if v.get("id") == self.k:
                return v

    def tools(self):
        return [t["name"] for t in self.rpc("tools/list", {})["result"]["tools"]]

    def call(self, name, args):
        v = self.rpc("tools/call", {"name": name, "arguments": args})
        res = v.get("result") or {}
        return res.get("structuredContent") or res or v.get("error")

    def close(self):
        try:
            self.p.stdin.close()
            self.p.wait(timeout=10)
        except Exception:
            self.p.kill()
            self.p.wait()
        return self.p.returncode


def short(r):
    s = json.dumps(r, ensure_ascii=False)
    e = (r or {}).get("error") if isinstance(r, dict) else None
    if e:
        return f"error {e.get('code')}/{e.get('reason')}"
    if isinstance(r, dict) and r.get("type") == "ack":
        return f"ack {r.get('status')} {r.get('proposal') or ''}"
    return s[:140]


def main():
    ip = init(None, "init-mcp.json")  # no agent_policy: the MCP server marks its connection
    m = Mcp(ip, "thin")
    names = m.tools()
    prog_tools = [n for n in names if "program" in n]
    r1 = m.call("op_add_program", {"id": "m1", "program": SMA, "inputs": ["px"], "mode": "direct"}) \
        if "op_add_program" in names else {"skip": True}
    r2 = m.call("op_add_program", {"id": "m2", "program": EVT, "inputs": ["px"]}) if "op_add_program" in names else {"skip": True}
    r3 = m.call("op_remove_program", {"program": "m2"}) if "op_remove_program" in names else {"skip": "no tool"}
    r4 = m.call("program_add", {"id": "m3", "program": "series y = sma(close, 5)", "inputs": ["px"]}) \
        if "program_add" in names else {"skip": "no tool"}
    r5 = m.call("command", {"ops": [{"op": "accept_proposal", "proposal": (r2 or {}).get("proposal", "prop-1")}]}) \
        if "command" in names else {"skip": "no tool"}
    r6 = m.call("command", {"ops": [{"op": "put_series", "series": {"id": "fake", "interval": "1h", "source": {
        "kind": "study", "study": "program", "inputs": ["px"], "params": {}, "output": "y",
        "program": {"instance": "fake", "program_id": "sha256:0", "instance_id": "sha256:0", "scope": "document",
                    "owner": "nobody", "since_t": 0, "params": {},
                    "ir": {"version": "qp-ir/0", "nodes": [{"id": "c", "op": "input", "field": "close"}],
                           "outputs": [{"name": "y", "kind": "series", "node": "c"}]}}}}}]}) \
        if "command" in names else {"skip": "no tool"}
    rec("M01", "MCP (thin yüzey) araçları", "program araçları listede", f"{prog_tools}", bool(prog_tools))
    rec("M02", "MCP op_add_program mode:direct", "forbidden/proposal_only", short(r1),
        isinstance(r1, dict) and ((r1.get("error") or {}).get("code") == "forbidden"))
    rec("M03", "MCP op_add_program (öneri) sonra op_remove_program", "öneri; remove forbidden/agent_cannot_write",
        f"{short(r2)}; {short(r3)}", (r2 or {}).get("status") == "proposed" and (r3.get("skip") or (r3.get("error") or {}).get("code") == "forbidden"))
    rec("M04", "MCP program_add (metin)", "öneri", short(r4), r4.get("skip") is not None or (r4 or {}).get("status") == "proposed"
        or "proposed" in json.dumps(r4))
    rec("M05", "MCP command accept_proposal (ajan karar veremez)", "forbidden", short(r5),
        r5.get("skip") is not None or (r5.get("error") or {}).get("code") == "forbidden")
    rec("M06", "MCP command put_series program alanıyla (sahip `nobody`)", "forbidden ya da E_PROGRAM_SERIES", short(r6),
        r6.get("skip") is not None or (r6.get("error") or {}).get("code") in ("forbidden", "invalid"))
    big = m.call("program_check", {"program": "x = " + "(" * 100000 + "close" + ")" * 100000 + "\nseries s = x"}) \
        if "program_check" in names else {"skip": True}
    rec("M07", "MCP program_check 100 000 parantez", "kodlu ret, süreç ayakta", short(big)[:120],
        m.p.poll() is None)
    code = m.close()
    rec("M08", "MCP sunucusu kapanışı", "temiz çıkış (0)", f"çıkış {code}", code == 0)
    out = HERE / "results-mcp.json"
    out.write_text(json.dumps(R, ensure_ascii=False, indent=1) + "\n")
    print(f"{sum(r['verdict'] == 'OK' for r in R)}/{len(R)} OK -> {out}")


if __name__ == "__main__":
    main()
