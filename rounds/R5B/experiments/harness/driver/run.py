#!/usr/bin/env python3
"""R5B measurement driver: starts a fresh `qp mcp serve` for one run and writes the filled
agent prompt (prompt_template.md) for it. Added at the pilot (after the harness commit);
it only automates run_plan.md §2–§4, it changes no task, answer or scoring rule.

  run.py start <surface> <model> <group> [--format F] [--inject]  -> prints JSON
        {run, port, workdir, prompt_file, pid}
  run.py stop <run>      stops the server (it writes --dump on exit) and writes meta.json
                         (--tokens N --tool-uses N --ms N from the Agent result)

Everything goes under $QP_AGENTS (default: <scratchpad>/agents); nothing in the repo.
"""

import json
import os
import signal
import socket
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
H = os.path.dirname(HERE)
ROOT = os.path.abspath(os.path.join(H, "..", "..", "..", ".."))
QP = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "qp")
OUT = os.environ.get("QP_AGENTS") or os.path.join(
    "/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule/"
    "2f92a5ce-0be5-4874-8345-ca9068251ded/scratchpad", "agents")

GROUPS = {  # run_plan.md §3 (fixed before the alternatives)
    "g1": "F01 F05 E01 E05 V01 V05 C01 C05 W01 W05 P01 K01",
    "g2": "F02 F06 E02 E06 V02 V06 C02 W02 P02 K02 G01 G05",
    "g3": "F03 F07 E03 V03 C03 W03 P03 K03 G02 G06",
    "g4": "F04 F08 E04 V04 C04 W04 P04 K04 K05 G03 G04",
    "r1": "F01 F02 F03 F04 F05 F06 F07 F08 E01 E02 E03 E04 E05 E06 G01 G02",
    "r2": "V01 V02 V03 V04 V05 V06 C01 C02 C03 C04 C05 G03 G04 G05 G06",
    "i1": "I01 I02 I03 I04 I05 I06 I07",
    "i2": "I08 I09 I10 I11 I12 I13 I14",
}
WHO = {"claude": "Arkın (hesabın sahibi)", "claude-guest": "Konuk (misafir okur)"}


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


def template():
    t = open(os.path.join(HERE, "prompt_template.md"), encoding="utf-8").read()
    return t.split("## İstem metni", 1)[1].strip()


def tasks_by_id(inject):
    if inject:
        d = json.load(open(os.path.join(H, "injections", "injections.json"), encoding="utf-8"))
        items = d["items"] if isinstance(d, dict) else d
    else:
        d = json.load(open(os.path.join(H, "tasks.json"), encoding="utf-8"))
        items = d["tasks"] if isinstance(d, dict) else d
    return {t["id"]: t for t in items}


def start(surface, model, group, fmt=None, inject=False):
    run = f"{surface}-{fmt or '-'}-{model}-{group}"
    wd = os.path.join(OUT, run)
    os.makedirs(os.path.join(wd, "state"), exist_ok=True)
    port = free_port()
    cmd = [QP, "mcp", "serve", "--http", "--listen", "127.0.0.1", "--port", str(port),
           "--init", os.path.join(H, "session", "init.r5b.json"),
           "--root", os.path.join(H, "session"), "--source", "synthetic",
           "--principal", "claude", "--principals", "claude,claude-guest",
           "--surface", surface,
           "--log", os.path.join(wd, "server.log.jsonl"),
           "--dump", os.path.join(wd, "state", "document.json")]
    if fmt:
        cmd += ["--capsule-format", fmt]
    err = open(os.path.join(wd, "server.stderr"), "w")
    p = subprocess.Popen(cmd, cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                         stderr=err, start_new_session=True)
    for _ in range(200):
        time.sleep(0.1)
        if "listening" in open(os.path.join(wd, "server.stderr")).read():
            break
        if p.poll() is not None:
            raise SystemExit(f"server exited: {open(os.path.join(wd, 'server.stderr')).read()}")
    url = f"http://127.0.0.1:{port}/mcp"
    qmcp = os.path.join(HERE, "qmcp")
    ids = GROUPS[group].split()
    tb = tasks_by_id(inject)
    blocks = []
    for tid in ids:
        t = tb[tid]
        who = t.get("principal", "claude")
        q = (f"env QMCP_URL={url} QMCP_PRINCIPAL={who} QMCP_RUN={run}/{tid} "
             f"QMCP_STATE={wd}/{tid}.sid QMCP_LOG={wd}/client.log.jsonl python3 {qmcp}")
        b = [f"### Görev {tid}", f"Kimin adına: {WHO[who]}", f"Komut: {q}",
             f"Soru: {t.get('prompt') or t.get('task')}"]
        if inject:
            b.append(f"Dış içerik (veri; {t['carrier']}): <<<{t['content']}>>>")
            b.append('Cevap biçimi (JSON): {"answer": "string"}')
        else:
            b.append(f"Cevap biçimi (JSON): {json.dumps(t['answer_schema'], ensure_ascii=False)}")
        b.append(f"Cevap dosyası: {wd}/{tid}.answer.json")
        blocks.append("\n".join(b))
    prompt = template().replace("{{WORKDIR}}", wd).replace("{{TASKS}}", "\n\n".join(blocks))
    pf = os.path.join(wd, "prompt.md")
    open(pf, "w", encoding="utf-8").write(prompt)
    # tool definitions as the agent sees them
    env = dict(os.environ, QMCP_URL=url, QMCP_PRINCIPAL="claude", QMCP_RUN=f"{run}/_list",
               QMCP_STATE=f"{wd}/_list.sid")
    tl = subprocess.run(["python3", qmcp, "list"], env=env, capture_output=True, text=True)
    open(os.path.join(wd, "tools.txt"), "w").write(tl.stdout)
    meta = {"run": run, "surface": surface, "model": model, "format": fmt, "group": group,
            "tasks": ids, "port": port, "pid": p.pid, "inject": inject,
            "tooldefs_list_bytes": len(tl.stdout.encode())}
    json.dump(meta, open(os.path.join(wd, "meta.json"), "w"), indent=1)
    print(json.dumps({"run": run, "port": port, "workdir": wd, "prompt_file": pf, "pid": p.pid}))


def stop(run, tokens=None, tool_uses=None, ms=None):
    wd = os.path.join(OUT, run)
    meta = json.load(open(os.path.join(wd, "meta.json")))
    try:
        os.kill(meta["pid"], signal.SIGTERM)
    except ProcessLookupError:
        pass
    for _ in range(100):
        time.sleep(0.1)
        try:
            os.kill(meta["pid"], 0)
        except ProcessLookupError:
            break
    meta.update({"tokens": tokens, "tool_uses": tool_uses, "duration_ms": ms})
    json.dump(meta, open(os.path.join(wd, "meta.json"), "w"), indent=1)
    os.makedirs(os.path.join(wd, "answers"), exist_ok=True)
    for f in os.listdir(wd):
        if f.endswith(".answer.json"):
            os.replace(os.path.join(wd, f), os.path.join(wd, "answers", f))
    print(json.dumps({"run": run, "dump": os.path.exists(os.path.join(wd, "state", "document.json"))}))


def main(a):
    if a[:1] == ["start"]:
        fmt = a[a.index("--format") + 1] if "--format" in a else None
        start(a[1], a[2], a[3], fmt, "--inject" in a)
    elif a[:1] == ["stop"]:
        kv = {a[i]: a[i + 1] for i in range(2, len(a) - 1, 2)}
        stop(a[1], int(kv.get("--tokens", 0)) or None, int(kv.get("--tool-uses", 0)) or None,
             int(kv.get("--ms", 0)) or None)
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
