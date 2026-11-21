#!/usr/bin/env python3
"""R5B/H: qmcp against a real MCP server today.

`qp mcp serve --http` does not exist yet (subtask M); this test puts a tiny Streamable
HTTP front (stdlib http.server, 127.0.0.1, random port) before today's stdio server
`qp mcp --init … --principal …` and checks that qmcp does initialize / Mcp-Session-Id /
tools/list / tools/call, sends X-Qp-Principal and X-Qp-Run, keeps the session in
QMCP_STATE, writes the client log, and parses an SSE-framed answer as well as JSON.

usage: QP_BIN=<path to qp> python3 driver/test_qmcp.py
"""
import http.server
import json
import os
import subprocess
import sys
import tempfile
import threading
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
QMCP = os.path.join(HERE, "qmcp")
QP = os.environ.get("QP_BIN") or os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(REPO, "target")),
                                              "release", "qp")
INIT = os.path.join(REPO, "fixtures", "protocol", "init", "thyao.json")


class Bridge:
    """HTTP front: one stdio `qp mcp` process, one MCP session id."""

    def __init__(self):
        self.p = subprocess.Popen([QP, "mcp", "--init", INIT, "--root", REPO, "--principal", "arkin"],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.seen = []
        self.lock = threading.Lock()
        self.sse = False
        bridge = self

        class H(http.server.BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass

            def do_POST(self):
                body = self.rfile.read(int(self.headers["Content-Length"])).decode()
                msg = json.loads(body)
                bridge.seen.append({"method": msg.get("method"), "principal": self.headers.get("X-Qp-Principal"),
                                    "run": self.headers.get("X-Qp-Run"),
                                    "session": self.headers.get("Mcp-Session-Id")})
                if msg.get("method") != "initialize" and self.headers.get("Mcp-Session-Id") != "sess-1":
                    self.send_response(404)
                    self.end_headers()
                    return
                with bridge.lock:
                    bridge.p.stdin.write(json.dumps(msg) + "\n")
                    bridge.p.stdin.flush()
                    if "id" not in msg:
                        self.send_response(202)
                        self.end_headers()
                        return
                    while True:
                        out = json.loads(bridge.p.stdout.readline())
                        if out.get("id") == msg["id"]:
                            break
                raw = json.dumps(out).encode()
                self.send_response(200)
                if bridge.sse:
                    raw = b"event: message\ndata: " + raw + b"\n\n"
                    self.send_header("Content-Type", "text/event-stream")
                else:
                    self.send_header("Content-Type", "application/json")
                if msg.get("method") == "initialize":
                    self.send_header("Mcp-Session-Id", "sess-1")
                self.send_header("Content-Length", str(len(raw)))
                self.end_headers()
                self.wfile.write(raw)

        self.httpd = http.server.HTTPServer(("127.0.0.1", 0), H)
        self.url = f"http://127.0.0.1:{self.httpd.server_address[1]}/mcp"
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()

    def close(self):
        self.httpd.shutdown()
        self.httpd.server_close()
        self.p.stdin.close()
        self.p.wait(timeout=30)
        self.p.stdout.close()


def qmcp(env, *args):
    r = subprocess.run([sys.executable, QMCP, *args], env=env, capture_output=True, text=True, timeout=60)
    return r.returncode, r.stdout, r.stderr


class QmcpTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.b = Bridge()
        cls.tmp = tempfile.TemporaryDirectory(prefix="qmcp-")
        cls.env = dict(os.environ, QMCP_URL=cls.b.url, QMCP_PRINCIPAL="claude", QMCP_RUN="t1/F01",
                       QMCP_STATE=os.path.join(cls.tmp.name, "sid"),
                       QMCP_LOG=os.path.join(cls.tmp.name, "client.log.jsonl"))

    @classmethod
    def tearDownClass(cls):
        cls.b.close()
        cls.tmp.cleanup()

    def test_1_list(self):
        code, out, err = qmcp(self.env, "list", "--json")
        self.assertEqual(code, 0, err)
        tools = [json.loads(line) for line in out.splitlines()]
        names = {t["name"] for t in tools}
        self.assertIn("query_capsules", names)
        self.assertTrue(all("inputSchema" in t for t in tools))
        code, text, err = qmcp(self.env, "list")
        self.assertEqual(code, 0, err)
        self.assertEqual(len(text.splitlines()), len(tools))
        self.assertTrue(any(l.startswith("query_capsules: ") and "| params: " in l for l in text.splitlines()))
        self.assertLess(len(text), len(out) / 10, "the summary is much smaller than the definitions")
        code, sch, err = qmcp(self.env, "schema", "query_capsules")
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(sch), next(t["inputSchema"] for t in tools if t["name"] == "query_capsules"))
        self.assertEqual(self.b.seen[0]["method"], "initialize")
        self.assertEqual(self.b.seen[-1]["session"], "sess-1")
        self.assertTrue(all(s["principal"] == "claude" and s["run"] == "t1/F01" for s in self.b.seen))

    def test_2_call_json_and_sse(self):
        n0 = sum(1 for s in self.b.seen if s["method"] == "initialize")
        code, out, err = qmcp(self.env, "call", "query_list", '{"collection":"views"}')
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out)["type"], "result")
        self.b.sse = True
        try:
            code, out, err = qmcp(self.env, "call", "query_get", '{"id":"no-such"}')
        finally:
            self.b.sse = False
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out.splitlines()[0])["type"], "error")
        n1 = sum(1 for s in self.b.seen if s["method"] == "initialize")
        self.assertEqual(n0, n1, "the stored session id is reused")
        with open(self.env["QMCP_LOG"]) as f:
            rows = [json.loads(line) for line in f]
        self.assertEqual([r["tool"] for r in rows][-2:], ["query_list", "query_get"])
        self.assertEqual([r["ok"] for r in rows][-2:], [True, False])
        self.assertEqual(rows[-1]["error"], "not_found")
        self.assertEqual(rows[-1]["args"], {"id": "no-such"})

    def test_3_bad_args(self):
        code, _, err = qmcp(self.env, "call", "query_list", "{not json")
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main(verbosity=1)
