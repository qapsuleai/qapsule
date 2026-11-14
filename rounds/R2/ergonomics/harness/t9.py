#!/usr/bin/env python3
"""R2/T9 agent-writability harness (sandboxed agents, randomized ids).

Subcommands (all run OUTSIDE the sandbox):
  packet  <variant>                    build the packet (spec + schema + run.sh)
  serve   <variant>                    harness daemon on <packet>/qp.sock
  launch  <variant> <agent> [--label L] run one measured agent (claude -p in sandbox-exec)
  isotest <variant>                    isolation test (an agent asked to read a repo file)

Layout (SCR = scratch area, RAW = rounds/R2/ergonomics/raw):
  SCR/packet/<variant>/{PROTOCOL.md, GRAMMAR.md, schema/, run.sh, qp.sock}
  SCR/work/<label>/<variant>/agent<X>/{TASKS.txt, R??/...}      agent's work dir
  SCR/docs/<label>/<variant>/agent<X>/{doc.json, init.json}     hidden setup document
  RAW/<label>/<variant>/agent<X>/{setup.json, transcript.jsonl, transcript.ts, result.json,
                                   R??/runs/<n>-<file>.jsonl (+ .full.jsonl), R??/<agent files>}

`run.sh` is a tiny client: it sends the absolute path of the attempt file over the Unix
socket; the daemon (here) reads it, runs hidden setup + attempt + hidden epilogue through
the variant's `qp serve`, stores the full output under RAW and answers only the messages
addressed to the agent's connections.
"""

import hashlib
import json
import os
import re
import shutil
import socketserver
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import setup  # noqa: E402

REPO = setup.ROOT
ERGO = os.path.dirname(HERE)
RAW = os.path.join(ERGO, "raw")
SCR = os.environ.get("T9_SCR", "/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule/"
                     "e6910705-d032-4e80-bf66-0a30bb080f3b/scratchpad/t9")
VARIANTS = {"base": "r2/int", "A": "r2/t8a-selectors", "B": "r2/t8b-names"}

AGENTS = {"A": ["R01", "R06", "R11", "R16"], "B": ["R02", "R07", "R12", "R17"],
          "C": ["R03", "R08", "R13", "R18"], "D": ["R04", "R09", "R14", "R19"],
          "E": ["R05", "R10", "R15", "R20"]}

ROLE = ("you are principal `editor`, connecting from a `headless` surface, working for the user "
        "`arkin` (who has this chart open on a desktop).")
REQUESTS = {
    "R01": "RSI(7)'yi fiyatın hemen altına ayrı panel olarak ekle.",
    "R02": "Üst bandı kırmızı yap.",
    "R03": "Son kırılımı vurgula.",
    "R04": "Son kırılım bulgusunu saat için göster: kullanıcıya ne söylenir?",
    "R05": "Fiyat panelini logaritmik ölçeğe çevir.",
    "R06": "RSI paneline 30 çizgisini de ekle.",
    "R07": "MACD panelini kaldır.",
    "R08": "Hacmi fiyatın altına çubuk olarak ekle.",
    "R09": "Bollinger bandını dolgulu alan olarak göster.",
    "R10": "MACD histogramını çubuk yap; pozitifse yeşil, negatifse kırmızı.",
    "R11": "Kapanışı SMA50'nin üstündeyken yeşil, altındayken kırmızı çiz.",
    "R12": "Grafiği son 60 bara ayarla, böyle kalsın.",
    "R13": "Ekranda şu an ne var? Kaç panel, hangi göstergeler?",
    "R14": "Fiyat panelinde, 1200×600'lük ekranda (900, 200) pikselinde ne var?",
    "R15": "RSI çizgisini kalınlaştır (2 px).",
    "R16": "Eski kırılımın olgusunu fiyat panelinde nokta olarak işaretle.",
    "R17": "Fiyat paneline 123,45'te yatay çizgi çiz.",
    "R18": "MACD panelini RSI'ın üstüne taşı.",
    "R19": "Son kırılım bulgusu telefonda nasıl görünür? Kaç bar gösterilir?",
    "R20": "Bant genişliğini ayrı panelde çizgi olarak göster.",
}

PROMPT = """You are an AI agent that must operate a financial chart document through its protocol. You know NOTHING about the implementation and must not look for it.

STRICT RULES:
- The ONLY files you may read are in the packet folder `{P}/`: `PROTOCOL.md` and `GRAMMAR.md` (the specification, in Turkish) and `schema/*.schema.json` (generated JSON Schemas of the messages). Do not read any other file on the machine; do not search the file system; do not use the web.
- The ONLY command you may run is `{P}/run.sh <your-file.jsonl>`. It runs your message file against a FRESH copy of the live document (each run starts from the same state; earlier runs leave no trace) and prints the messages the authority sends to your connections.
- Message file format: one JSON object per line: `{{"conn": "<a connection id you choose, not starting with zz->", "msg": <protocol message>}}`. Start each connection with `hello`.

Your tasks are listed in `{W}/TASKS.txt`. For EACH task, in its work dir:
1. Read the spec/schema as needed, then write `attempt1.jsonl` with the complete message sequence you believe accomplishes the request, and run it. You may include queries, but remember you cannot see their answers before the run; each run is one shot over the whole file.
2. Look at the output. If the request is not accomplished (an error, or the result is not what was asked), write `attempt2.jsonl` (a complete new sequence, from the fresh state) and run it; at most `attempt3.jsonl`. Stop as soon as you believe it is done. Never edit an attempt file after running it.
3. If the request asks a question, write the answer you would give the user (one or two sentences, in Turkish) to `answer.txt`, based on what the authority returned in your last run.
4. Write `notes.txt`: one line per attempt — what you tried, what went wrong, and anything in the spec that was unclear or misleading.

At the end reply with a short table: task, number of attempts, whether you believe it succeeded, and the main difficulty."""

RUN_SH = r"""#!/bin/sh
# run.sh <messages.jsonl>: runs the message file against a fresh copy of the live document
# and prints the authority's messages to your connections.
[ $# -eq 1 ] || { echo "usage: run.sh <messages.jsonl>" >&2; exit 2; }
case "$1" in /*) f="$1" ;; *) f="$(pwd)/$1" ;; esac
cd "$(dirname "$0")" || exit 2
exec /usr/bin/python3 -I -S -c '
import socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect("qp.sock")
s.sendall((sys.argv[1] + "\n").encode())
s.shutdown(socket.SHUT_WR)
out = b""
while True:
    b = s.recv(65536)
    if not b:
        break
    out += b
sys.stdout.write(out.decode())
' "$f"
"""

SB = """(version 1)
(allow default)
(deny file-read* (subpath "/Users/arkns/Works"))
(deny file-read* (subpath "/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule"))
(deny file-read* (subpath "/Users/arkns/.claude/projects/-Users-arkns-Works-Mchart-qapsule"))
(allow file-read* (subpath "{P}"))
(allow file-read* (subpath "{W}"))
(allow file-read-metadata)
"""


def qp_bin(variant):
    return os.path.join(SCR, f"target-{variant}", "release", "qp")


def packet_dir(variant):
    return os.path.join(SCR, "packet", variant)


def seed_of(label, variant, agent):
    return int(hashlib.sha256(f"r2-t9/{label}/{variant}/{agent}".encode()).hexdigest()[:8], 16)


# --------------------------------------------------------------------------- packet

def cmd_packet(variant):
    p = packet_dir(variant)
    wt = os.path.join(SCR, "wt", variant)
    if os.path.exists(p):
        shutil.rmtree(p)
    os.makedirs(os.path.join(p, "schema"))
    for f in ("PROTOCOL.md", "GRAMMAR.md"):
        shutil.copy(os.path.join(wt, "docs", f), os.path.join(p, f))
    subprocess.run([qp_bin(variant), "schema", "export", "--out", os.path.join(p, "schema")], check=True)
    with open(os.path.join(p, "run.sh"), "w") as f:
        f.write(RUN_SH)
    os.chmod(os.path.join(p, "run.sh"), 0o755)
    print(p)


# --------------------------------------------------------------------------- daemon

def agent_paths(label, variant, agent):
    return {"work": os.path.join(SCR, "work", label, variant, f"agent{agent}"),
            "docs": os.path.join(SCR, "docs", label, variant, f"agent{agent}"),
            "raw": os.path.join(RAW, label, variant, f"agent{agent}")}


def prepare_setup(label, variant, agent):
    ap = agent_paths(label, variant, agent)
    seed = seed_of(label, variant, agent)
    s = setup.make(seed)
    os.makedirs(ap["docs"], exist_ok=True)
    os.makedirs(ap["raw"], exist_ok=True)
    json.dump(s["doc"], open(os.path.join(ap["docs"], "doc.json"), "w"), ensure_ascii=False)
    json.dump(s["init"], open(os.path.join(ap["docs"], "init.json"), "w"))
    commit = subprocess.run(["git", "-C", os.path.join(SCR, "wt", variant), "rev-parse", "HEAD"],
                            capture_output=True, text=True).stdout.strip()
    json.dump({"label": label, "variant": variant, "branch": VARIANTS[variant], "commit": commit,
               "agent": agent, "tasks": AGENTS[agent], "seed": seed, "mapping": s["mapping"],
               "setup": s["setup"], "epilogue": s["epilogue"]},
              open(os.path.join(ap["raw"], "setup.json"), "w"), ensure_ascii=False, indent=1)
    return s


def read_lines(text):
    return [l.strip() for l in text.splitlines() if l.strip()]


def agent_conns(lines):
    conns = []
    for raw in lines:
        try:
            c = json.loads(raw).get("conn")
        except (ValueError, AttributeError):
            continue
        if isinstance(c, str) and c not in conns:
            conns.append(c)
    return conns


def run_lines(qp, docs_dir, s, lines):
    feed = [json.dumps(x, ensure_ascii=False) for x in s["setup"]] + lines + \
           [json.dumps(x, ensure_ascii=False) for x in s["epilogue"]]
    proc = subprocess.run([qp, "serve", "--init", os.path.join(docs_dir, "init.json"), "--root", docs_dir],
                          input="\n".join(feed) + "\n", capture_output=True, text=True, timeout=120)
    outs = [l for l in proc.stdout.splitlines() if l.strip()]
    mine = set(agent_conns(lines)) | {""}
    shown = []
    for l in outs:
        try:
            m = json.loads(l)
        except ValueError:
            continue
        if m.get("conn") in mine and not m.get("batch_end"):
            shown.append(l)
    return shown, outs, proc.stderr


WORK_RE = re.compile(r"^work/([^/]+)/([^/]+)/agent([A-E])/(R\d\d)/([A-Za-z0-9_.-]+\.jsonl)$")
LOCK = threading.Lock()
SETUPS = {}


def handle_attempt(variant, path):
    real = os.path.realpath(path)
    rel = os.path.relpath(real, os.path.realpath(SCR))
    m = WORK_RE.match(rel)
    if not m or m.group(2) != variant:
        return "error: the file must be a .jsonl file inside one of your task work dirs\n"
    label, _, agent, task, name = m.groups()
    ap = agent_paths(label, variant, agent)
    key = (label, variant, agent)
    with LOCK:
        if key not in SETUPS:
            SETUPS[key] = setup.make(json.load(open(os.path.join(ap["raw"], "setup.json")))["seed"])
    s = SETUPS[key]
    try:
        text = open(real, encoding="utf-8").read(2_000_000)
    except OSError as e:
        return f"error: cannot read {path}: {e}\n"
    lines = read_lines(text)
    for raw in lines:
        try:
            c = json.loads(raw).get("conn", "")
        except (ValueError, AttributeError):
            c = ""
        if isinstance(c, str) and c.startswith("zz-"):
            return "error: connection ids starting with `zz-` are reserved by the harness\n"
    shown, outs, err = run_lines(qp_bin(variant), ap["docs"], s, lines)
    rdir = os.path.join(ap["raw"], task, "runs")
    with LOCK:
        os.makedirs(rdir, exist_ok=True)
        n = len([f for f in os.listdir(rdir) if f.endswith(".full.jsonl")]) + 1
        base = os.path.join(rdir, f"{n:02d}-{name}")
        with open(base, "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")
        with open(base[:-6] + ".full.jsonl", "w", encoding="utf-8") as f:
            f.write("\n".join(outs) + "\n")
        with open(os.path.join(ap["raw"], "runs.log"), "a") as f:
            f.write(json.dumps({"t": time.time(), "task": task, "file": name, "run": n,
                                "msgs": len(lines), "shown": len(shown), "stderr": err.strip()[:500]}) + "\n")
    out = "\n".join(shown) + "\n"
    if err.strip():
        out += err.strip() + "\n"
    return out


def cmd_serve(variant):
    p = packet_dir(variant)
    sock = os.path.join(p, "qp.sock")
    if os.path.exists(sock):
        os.unlink(sock)

    class H(socketserver.StreamRequestHandler):
        def handle(self):
            path = self.rfile.readline().decode().strip()
            try:
                out = handle_attempt(variant, path)
            except Exception as e:  # report, keep serving
                out = f"error: harness failure: {e}\n"
            self.wfile.write(out.encode())

    os.chdir(p)  # bind a relative path: the absolute one exceeds the 104-byte sun_path limit
    srv = socketserver.ThreadingUnixStreamServer("qp.sock", H)
    print(f"serving {variant} on {sock}", flush=True)
    srv.serve_forever()


# --------------------------------------------------------------------------- launch

def claude_cmd(prompt, p):
    return ["claude", "-p", "--model", "sonnet",
            "--tools", "Read,Write,Bash",
            "--allowedTools", "Read", "Write", f"Bash({p}/run.sh:*)",
            "--setting-sources", "local", "--strict-mcp-config", "--disable-slash-commands", "--no-session-persistence",
            "--output-format", "stream-json", "--verbose", prompt]


def run_sandboxed(prompt, p, work, raw, profile_extra=""):
    os.makedirs(work, exist_ok=True)
    os.makedirs(raw, exist_ok=True)
    prof = os.path.join(SCR, "profiles", os.path.relpath(work, os.path.join(SCR, "work")).replace("/", "_") + ".sb")
    os.makedirs(os.path.dirname(prof), exist_ok=True)
    with open(prof, "w") as f:
        f.write(SB.format(P=os.path.realpath(p), W=os.path.realpath(work)) + profile_extra)
    shutil.copy(prof, os.path.join(raw, "sandbox.sb"))
    cmd = ["sandbox-exec", "-f", prof] + claude_cmd(prompt, p)
    with open(os.path.join(raw, "command.json"), "w") as f:
        json.dump({"cwd": work, "argv": cmd}, f, ensure_ascii=False, indent=1)
    t0 = time.time()
    tr = open(os.path.join(raw, "transcript.jsonl"), "w", encoding="utf-8")
    ts = open(os.path.join(raw, "transcript.ts"), "w")
    proc = subprocess.Popen(cmd, cwd=work, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=open(os.path.join(raw, "stderr.txt"), "w"), text=True)
    for line in proc.stdout:
        tr.write(line)
        ts.write(f"{time.time():.3f}\n")
        tr.flush()
        ts.flush()
    rc = proc.wait()
    tr.close()
    ts.close()
    json.dump({"rc": rc, "start": t0, "end": time.time(), "wall_s": round(time.time() - t0, 1)},
              open(os.path.join(raw, "process.json"), "w"))
    return rc


def cmd_launch(variant, agent, label):
    ap = agent_paths(label, variant, agent)
    if os.path.exists(ap["work"]):
        raise SystemExit(f"work dir exists: {ap['work']}")
    prepare_setup(label, variant, agent)
    p = packet_dir(variant)
    os.makedirs(ap["work"])
    lines = []
    for t in AGENTS[agent]:
        d = os.path.join(ap["work"], t)
        os.makedirs(d)
        lines.append(f"- **{t}** — {ROLE} Request: «{REQUESTS[t]}» Work dir: `{d}/`")
    with open(os.path.join(ap["work"], "TASKS.txt"), "w") as f:
        f.write("\n".join(lines) + "\n")
    prompt = PROMPT.format(P=p, W=ap["work"])
    with open(os.path.join(ap["raw"], "prompt.txt"), "w") as f:
        f.write(prompt + "\n")
    rc = run_sandboxed(prompt, p, ap["work"], ap["raw"])
    # copy the agent's files (attempts as left, notes, answers) into the raw logs
    for t in AGENTS[agent]:
        src = os.path.join(ap["work"], t)
        dst = os.path.join(ap["raw"], t)
        os.makedirs(dst, exist_ok=True)
        for f in sorted(os.listdir(src)):
            if os.path.isfile(os.path.join(src, f)):
                shutil.copy(os.path.join(src, f), os.path.join(dst, f))
    shutil.copy(os.path.join(ap["work"], "TASKS.txt"), os.path.join(ap["raw"], "TASKS.txt"))
    extra = sorted(set(os.listdir(ap["work"])) - set(AGENTS[agent]) - {"TASKS.txt"})
    if extra:
        os.makedirs(os.path.join(ap["raw"], "_extra"), exist_ok=True)
        for f in extra:
            fp = os.path.join(ap["work"], f)
            if os.path.isfile(fp):
                shutil.copy(fp, os.path.join(ap["raw"], "_extra", f))
    print(f"{label}/{variant}/agent{agent}: rc={rc}")


def cmd_isotest(variant, label="isotest"):
    p = packet_dir(variant)
    ap = agent_paths(label, variant, "A")
    for d in (ap["work"], ap["raw"]):
        if os.path.exists(d):
            shutil.rmtree(d)
    prepare_setup(label, variant, "A")
    os.makedirs(os.path.join(ap["work"], "R01"))
    probe = os.path.join(ap["work"], "R01", "probe.jsonl")
    targets = [os.path.join(REPO, "CLAUDE.md"), os.path.join(HERE, "setup.py"),
               os.path.join(REPO, "rounds/R2/tasks/TASKS.md")]
    prompt = ("This is a file-access test. Use the Read tool on each of these files and report, for each, "
              "either its first line or the exact error you got: "
              + ", ".join(f"`{t}`" for t in targets)
              + f". Then read `{p}/PROTOCOL.md` and report its first line. Then write `{probe}` with the single "
              'line {"conn":"a","msg":{"type":"hello","protocol":"qp/0","principal":"editor","surface":{"kind":"headless"}}}'
              f" and run `{p}/run.sh {probe}`; report the first 200 characters of the output. "
              f"Finally try the Bash command `cat {targets[0]}` and report what happened. Reply with a short table.")
    with open(os.path.join(ap["raw"], "prompt.txt"), "w") as f:
        f.write(prompt + "\n")
    rc = run_sandboxed(prompt, p, ap["work"], ap["raw"])
    print(f"isotest rc={rc}")


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    c = argv[0]
    if c == "packet":
        cmd_packet(argv[1])
    elif c == "serve":
        cmd_serve(argv[1])
    elif c == "launch":
        label = argv[argv.index("--label") + 1] if "--label" in argv else "v1"
        cmd_launch(argv[1], argv[2], label)
    elif c == "isotest":
        cmd_isotest(argv[1])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
