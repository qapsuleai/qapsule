"""R5B/H: a tiny driver for `qp serve` (line transport, docs/PROTOCOL.md §3).

Standard library only. Deterministic: the replay host is paused (`running: false`), the
position is reached with `replay seek`, and every read is a `query` answered before the
next line is sent (`--batch-end`).
"""
import json
import os
import subprocess

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", ".."))
DEFAULT_TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.join(REPO, "target"))


def find_qp():
    """`QP_BIN`, else the release binary under CARGO_TARGET_DIR, else `qp` on PATH."""
    if os.environ.get("QP_BIN"):
        return os.environ["QP_BIN"]
    cand = os.path.join(DEFAULT_TARGET, "release", "qp")
    return cand if os.path.exists(cand) else "qp"


class QpError(Exception):
    pass


class Serve:
    def __init__(self, init_path, qp=None, root=None):
        self.qp = qp or find_qp()
        root = root or os.path.dirname(os.path.abspath(init_path))
        self.p = subprocess.Popen(
            [self.qp, "serve", "--init", init_path, "--root", root, "--batch-end",
             "--no-local-data"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8")
        self.n = 0

    def close(self):
        if self.p.poll() is None:
            self.p.stdin.close()
            self.p.wait(timeout=60)

    def __enter__(self):
        return self

    def __exit__(self, *a):
        self.close()

    def line(self, frame):
        """Sends one input line; returns every output line up to the batch end."""
        self.p.stdin.write(json.dumps(frame, ensure_ascii=False, separators=(",", ":")) + "\n")
        self.p.stdin.flush()
        out = []
        while True:
            raw = self.p.stdout.readline()
            if not raw:
                raise QpError("qp serve ended: " + self.p.stderr.read()[-2000:])
            v = json.loads(raw)
            if v.get("batch_end"):
                return out
            out.append(v)

    def send(self, conn, msg):
        return self.line({"conn": conn, "msg": msg})

    def _id(self, prefix):
        self.n += 1
        return f"{prefix}{self.n}"

    def reply(self, conn, msg):
        """Sends `msg` (with a fresh id) and returns the reply to it on `conn`."""
        msg = dict(msg)
        msg["id"] = self._id(msg.get("type", "m")[0])
        for o in self.send(conn, msg):
            m = o.get("msg", {})
            if o.get("conn") == conn and m.get("id") == msg["id"]:
                return m
        raise QpError(f"no reply to {msg}")

    def hello(self, conn, principal, kind="headless", lang=None):
        surface = {"kind": kind}
        if lang:
            surface["lang"] = lang
        out = self.send(conn, {"type": "hello", "protocol": "qp/0", "principal": principal,
                               "surface": surface})
        for o in out:
            if o.get("conn") == conn and o["msg"].get("type") in ("welcome", "error"):
                if o["msg"]["type"] == "error":
                    raise QpError(json.dumps(o["msg"]))
                return o["msg"]
        raise QpError("no welcome")

    def query(self, conn, query):
        """`result` message (whole) or raises with the error."""
        m = self.reply(conn, {"type": "query", "query": query})
        if m.get("type") != "result":
            raise QpError(json.dumps(m, ensure_ascii=False))
        return m

    def query_or_error(self, conn, query):
        return self.reply(conn, {"type": "query", "query": query})

    def replay(self, conn, control):
        m = self.reply(conn, {"type": "replay", "control": control})
        if m.get("type") != "result":
            raise QpError(json.dumps(m, ensure_ascii=False))
        return m["data"]["replay"]
