#!/usr/bin/env python3
"""RL1/U3 — kötü niyet denetimi (GOAL §7, VERIFY 14) için ortak parçalar (yalnız standart kütüphane).

Sentetik bar üretimi, `qp program` komut satırı çağrısı (süre ve en yüksek bellek, `/usr/bin/time -l`),
`qp serve` oturumu (stdin/stdout JSON satırları, `--batch-end`). Gerçek piyasa verisi yok.

Ortam: QP_BIN (varsayılan $CARGO_TARGET_DIR/debug/qp), ABUSE_WORK (üretilen büyük girdiler ve
oturum dosyaları; depo dışı, varsayılan geçici klasör).
"""
import json
import os
import re
import subprocess
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", "/Users/arkns/Works/Mchart/qapsule/target/wt/abuse"))
QP = os.environ.get("QP_BIN") or str(TARGET / "debug" / "qp")
WORK = Path(os.environ.get("ABUSE_WORK") or tempfile.mkdtemp(prefix="qp-abuse-"))
WORK.mkdir(parents=True, exist_ok=True)
HOUR = 3_600_000
T0 = 1_704_067_200_000  # 2024-01-01T00:00Z


def bars(n, seed=7, t0=T0, step=HOUR):
    """Deterministic synthetic random walk (LCG), 1h bars."""
    x = seed & 0xFFFFFFFF
    p = 100.0
    out = []
    for i in range(n):
        x = (1103515245 * x + 12345) & 0x7FFFFFFF
        r = (x / 0x7FFFFFFF - 0.5) * 0.02
        o = p
        c = round(p * (1 + r), 4)
        h = round(max(o, c) * 1.002, 4)
        lo = round(min(o, c) * 0.998, 4)
        out.append({"t": t0 + i * step, "o": o, "h": h, "l": lo, "c": c, "v": float(100 + (x % 900))})
        p = c
    return out


def series_file(n, name=None, **kw):
    """A `fixtures/bars` style series file with `n` bars; returns its path."""
    p = WORK / (name or f"bars-{n}.json")
    if not p.exists():
        s = {"id": "px", "instrument": {"symbol": "ABUSE"}, "interval": "1h",
             "source": {"kind": "inline"}, "data": {"kind": "bars", "bars": bars(n, **kw)}}
        p.write_text(json.dumps(s))
    return p


def write(name, text):
    p = WORK / name
    p.write_text(text if isinstance(text, str) else json.dumps(text))
    return p


def ir(nodes, outputs, params=None):
    d = {"version": "qp-ir/0"}
    if params:
        d["params"] = params
    d["nodes"] = nodes
    d["outputs"] = outputs
    return d


def cli(args, timeout=120):
    """Runs `qp <args>` under /usr/bin/time -l; returns dict(exit, out, json, ms, rss_mb, err)."""
    t = time.monotonic()
    try:
        p = subprocess.run(["/usr/bin/time", "-l", QP] + [str(a) for a in args],
                           capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return {"exit": "timeout", "out": "", "json": None, "ms": timeout * 1000, "rss_mb": None,
                "err": "timeout"}
    ms = round((time.monotonic() - t) * 1000)
    m = re.search(r"(\d+)\s+maximum resident set size", p.stderr)
    rss = round(int(m.group(1)) / 2**20, 1) if m else None
    js = None
    try:
        js = json.loads(p.stdout)
    except ValueError:
        pass
    err = "\n".join(l for l in p.stderr.splitlines()
                    if not re.match(r"\s+\d+\s+\S", l) and "real" not in l)[:400]
    return {"exit": p.returncode, "out": p.stdout[:2000], "json": js, "ms": ms, "rss_mb": rss,
            "err": err}


def reasons(js):
    """Every `reason` of a program_check reply (or a protocol error with `errors`)."""
    if not isinstance(js, dict):
        return []
    errs = js.get("errors") or []
    r = [e.get("reason") for e in errs]
    if not r and js.get("reason"):
        r = [js["reason"]]
    return r


def rss_of(pid):
    try:
        o = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)], capture_output=True, text=True).stdout
        return round(int(o.strip()) / 1024, 1)
    except ValueError:
        return None


class Serve:
    """`qp serve --batch-end` over stdin/stdout; frames `{"conn", "msg"}`."""

    def __init__(self, init, root):
        self.p = subprocess.Popen([QP, "serve", "--batch-end", "--init", str(init), "--root", str(root)],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=open(WORK / "serve.stderr", "a"), text=True, bufsize=1)
        self.n = 0
        self.peak = 0.0

    def send(self, conn, msg, timeout=120):
        """Sends one frame; returns (replies for conn, all lines, ms)."""
        line = json.dumps({"conn": conn, "msg": msg}, ensure_ascii=False)
        t = time.monotonic()
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        out = []
        while True:
            l = self.p.stdout.readline()
            if not l:
                raise RuntimeError(f"serve exited (code {self.p.poll()})")
            v = json.loads(l)
            if v.get("batch_end"):
                break
            out.append(v)
        ms = round((time.monotonic() - t) * 1000, 1)
        r = rss_of(self.p.pid)
        if r:
            self.peak = max(self.peak, r)
        mine = [o["msg"] for o in out if o.get("conn") == conn]
        return mine, out, ms

    def hello(self, conn, principal):
        return self.send(conn, {"type": "hello", "protocol": "qp/0", "principal": principal,
                                "surface": {"kind": "headless"}})[0]

    def reply(self, conn, msg, timeout=120):
        """The reply with the message's id (ack/result/error), plus ms."""
        mine, _, ms = self.send(conn, msg, timeout)
        mid = msg.get("id")
        for m in mine:
            if m.get("id") == mid and m.get("type") in ("ack", "result", "error"):
                return m, ms
        return {"type": "none", "lines": mine}, ms

    def alive(self):
        return self.p.poll() is None

    def close(self):
        """Closes stdin and waits; kills only its own process by PID if it hangs (K92)."""
        try:
            self.p.stdin.close()
            self.p.wait(timeout=10)
        except Exception:
            os.kill(self.p.pid, 9)
            self.p.wait()
        return self.p.returncode
