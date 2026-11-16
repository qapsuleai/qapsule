#!/usr/bin/env python3
"""R4/E (copy of R3/E, path/version adaptations only) agent-writability harness for capsule tasks (sandboxed agents, randomized setup).

Adapted from rounds/R2/ergonomics/harness/t9.py (same method: fresh `claude -p` agents,
model sonnet, sandbox-exec read/write limits, per-agent token, packet = spec + grammar +
schema + run.sh, hidden setup and epilogue, raw logs kept). One variant: this branch.

Subcommands (all run OUTSIDE the sandbox):
  packet                       build the packet (PROTOCOL.md, GRAMMAR.md, schema/, run.sh)
  serve                        harness daemon on <packet>/qp.sock
  launch <agent> --label L     run one measured agent (claude -p in sandbox-exec)
  isotest                      isolation test (an agent asked to read a repo file)

Layout (SCR = scratch area, RAW = rounds/R4/ergonomics/raw):
  SCR/packet/{PROTOCOL.md, GRAMMAR.md, schema/, run.sh, qp.sock}
  SCR/work/<label>/agent<X>/{TASKS.txt, C??/...}       agent's work dir
  SCR/docs/<label>/agent<X>/init.json                  hidden setup
  RAW/<label>/agent<X>/{setup.json, prompt.txt, transcript.jsonl (+ .ts), process.json,
                        truth.json.gz, runs.log, C??/runs/<n>-<file>.jsonl (+ .out.jsonl.gz: what the agent
                        saw, + .hidden.json: hidden-state extract, + .png.json),
                        C??/<agent files>}

`run.sh` sends the attempt path (and an optional render request) over the Unix socket; the
daemon runs hidden setup + attempt + hidden epilogue through `qp serve`, stores the full
output under RAW and answers only the messages addressed to the agent's connections. With
`--png`, it also draws a view of the resulting session with `qp render` (the reference
headless renderer) into the agent's work dir.
"""

import hashlib
import json
import os
import re
import secrets
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
SCR = os.path.join(setup.SCR, "e3")
QP = os.path.join(setup.SCR, "target", "release", "qp")

AGENTS = {"A": ["C01", "C06", "C11", "C16"], "B": ["C02", "C07", "C12", "C17"],
          "C": ["C03", "C08", "C13", "C18"], "D": ["C04", "C09", "C14", "C19"],
          "E": ["C05", "C10", "C15", "C20"]}

ROLE = ("You are principal `editor` (role `editor`), connecting from a `headless` surface. "
        "The request comes from the user of this connection: \"ben\", \"benim\", \"-im\" mean "
        "principal `editor`.")
ROLE_VIEWER = ("You are principal `viewer` (role `viewer`, a user without the licensed flow data "
               "entitlement), connecting from a `headless` surface. The request comes from that user.")
ROLES = {"C11": ROLE_VIEWER}

# The request texts (Turkish, as in rounds/R3/tasks/TASKS.md). Where the table's text needs a
# referent the agent cannot otherwise know ("bu kapsül", "the" capsule among ~185 stories),
# the referent is written into the request (as R2 did for R01/R04/R19); see RESULTS.md §1.3.
REQUESTS = {
    "C01": "Bugün THYAO'da ne var?",
    "C02": "THYAO'daki açık kırılım kapsülünün kanıtını aç.",
    "C03": "ASELS'te en son geçersiz olan kapsül neden geçersiz oldu?",
    "C04": "THYAO'daki açık kırılım kapsülünü reddet.",
    "C05": "En önemli üç kapsülü sırala.",
    "C06": "Kırılım türünün karnesi ne?",
    "C07": "Hassasiyetimi \"yalnız önemliler\" yap.",
    "C08": "ASELS'i izleme listeme ekle.",
    "C09": "THYAO'daki açık aşırılık kapsülünü kapsül derinliğinde göster.",
    "C10": "THYAO'daki açık kırılım kapsülünü grafik derinliğinde göster; kırılan seviye çiziliyor mu?",
    "C11": "THYAO'daki en son akış kapsülünün kanıtını aç.",
    "C12": "THYAO kapsüllerine abone ol; yeni geçişleri bildir.",
    "C13": "THYAO'da en son gerçekleşen kırılım kapsülünün geçmişini (bütün geçişlerini) listele.",
    "C14": "THYAO'daki açık kırılım kapsülünün olgusunun kanıtını bugünkü veriyle doğrula.",
    "C15": "Yansıdaki veri, THYAO'daki açık kırılım kapsülünün kesin iddiasını tutmuyor; itiraz et.",
    "C16": "THYAO'daki sıkışma kapsülü hâlâ geçerli mi?",
    "C17": "Son on barda doğan kapsülleri, türüne göre say.",
    "C18": "THYAO'da pozisyonum var (100 adet); bunu kaydet.",
    "C19": "Gerçekleşen kapsülleri listele.",
    "C20": "THYAO'daki açık kırılım kapsülünü işaretli grafik olarak PNG'ye çiz.",
}

PROMPT = """You are an AI agent that must operate a financial chart document through its protocol. You know NOTHING about the implementation and must not look for it.

STRICT RULES:
- The ONLY files you may read are in the packet folder `{P}/`: `PROTOCOL.md` and `GRAMMAR.md` (the specification, in Turkish) and `schema/*.schema.json` (generated JSON Schemas of the messages). Do not read any other file on the machine; do not search the file system; do not use the web.
- The ONLY command you may run is `{P}/run.sh <your-file.jsonl> [--png <out.png> (--view <view id> | --present <finding id> [--surface <surface kind>])]`. It runs your message file against a FRESH copy of the live session (each run starts from the same state; earlier runs leave no trace) and prints the messages the authority sends to your connections. With `--png`, after your messages it also draws, with the reference headless renderer, either the document view `--view` or the view the presenter returns for `--present` (as `qp render` does), into `<out.png>` (a path inside your task's work dir).
- Message file format: one JSON object per line: `{{"conn": "<a connection id you choose, not starting with zz->", "msg": <protocol message>}}`. Start each connection with `hello`.

Your tasks are listed in `{W}/TASKS.txt`. For EACH task, in its work dir:
1. Read the spec/schema as needed, then write `attempt1.jsonl` with the complete message sequence you believe accomplishes the request, and run it. You may include queries, but remember you cannot see their answers before the run; each run is one shot over the whole file.
2. Look at the output. If the request is not accomplished (an error, or the result is not what was asked), write `attempt2.jsonl` (a complete new sequence, from the fresh state) and run it; at most `attempt3.jsonl`. Stop as soon as you believe it is done. Never edit an attempt file after running it.
3. If the request asks a question or asks you to show/list/open something, write the answer you would give the user (a few sentences, in Turkish) to `answer.txt`, based on what the authority returned in your last run.
4. Write `notes.txt`: one line per attempt — what you tried, what went wrong, and anything in the spec that was unclear or misleading.

At the end reply with a short table: task, number of attempts, whether you believe it succeeded, and the main difficulty."""

RUN_SH = r"""#!/bin/sh
# run.sh <messages.jsonl> [--png <out.png> (--view <view id> | --present <finding id> [--surface <kind>])]
# Runs the message file against a fresh copy of the live session and prints the authority's
# messages to your connections. With --png, also draws a view (reference headless renderer).
[ $# -ge 1 ] || { echo "usage: run.sh <messages.jsonl> [--png <out.png> (--view <id> | --present <finding> [--surface <kind>])]" >&2; exit 2; }
case "$1" in /*) f="$1" ;; *) f="$(pwd)/$1" ;; esac
shift
png=""; view=""; present=""; surface=""
while [ $# -gt 0 ]; do
  case "$1" in
    --png) png="$2"; shift 2 ;;
    --view) view="$2"; shift 2 ;;
    --present) present="$2"; shift 2 ;;
    --surface) surface="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$png" in ""|/*) ;; *) png="$(pwd)/$png" ;; esac
cd "$(dirname "$0")" || exit 2
exec /usr/bin/python3 -I -S -c '
import json, os, socket, sys
f, png, view, present, surface = sys.argv[1:6]
d, tok = os.path.dirname(f), ""
while d and d != "/":
    try:
        tok = open(os.path.join(d, ".qp-token")).read().strip()
        break
    except OSError:
        d = os.path.dirname(d)
req = {"path": f, "token": tok}
if png:
    req["png"] = {"out": png, "view": view, "present": present, "surface": surface}
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect("qp.sock")
s.sendall((json.dumps(req) + "\n").encode())
s.shutdown(socket.SHUT_WR)
out = b""
while True:
    b = s.recv(65536)
    if not b:
        break
    out += b
sys.stdout.write(out.decode())
' "$f" "$png" "$view" "$present" "$surface"
"""

SB = """(version 1)
(allow default)
(deny file-read* (subpath "/Users/arkns/Works"))
(deny file-read* (subpath "/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule"))
(deny file-read* (subpath "/Users/arkns/.claude/projects/-Users-arkns-Works-Mchart-qapsule"))
(deny file-read* (regex #"^/Users/arkns/\\.claude/projects/-private-tmp-claude-501--Users-arkns-Works-Mchart-qapsule-"))
(allow file-read* (subpath "{P}"))
(allow file-read* (subpath "{W}"))
(allow file-read* (subpath "{CP}"))
(allow file-read-metadata)
(deny file-write* (subpath "/Users/arkns/Works"))
(deny file-write* (subpath "/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule"))
(deny file-write* (subpath "/Users/arkns/.claude/projects/-Users-arkns-Works-Mchart-qapsule"))
(deny file-write* (regex #"^/Users/arkns/\\.claude/projects/-private-tmp-claude-501--Users-arkns-Works-Mchart-qapsule-"))
(allow file-write* (subpath "{W}"))
(allow file-write* (subpath "{CP}"))
"""

# R4 adaptation: earlier measured agents' Claude Code project dirs (R2/R3 runs, earlier R4 runs)
# now exist under ~/.claude/projects and hold their transcripts; each agent may read/write only
# its own (Claude Code stores its own large tool outputs there).
def claude_project_dir(work):
    return "/Users/arkns/.claude/projects/" + re.sub(r"[^A-Za-z0-9]", "-", work)


def packet_dir():
    return os.path.join(SCR, "packet")


def seed_of(label, agent):
    return int(hashlib.sha256(f"r4-e/{label}/{agent}".encode()).hexdigest()[:8], 16)


def commit():
    return subprocess.run(["git", "-C", REPO, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()


# --------------------------------------------------------------------------- packet

def cmd_packet():
    p = packet_dir()
    if os.path.exists(p):
        shutil.rmtree(p)
    os.makedirs(os.path.join(p, "schema"))
    for f in ("PROTOCOL.md", "GRAMMAR.md"):
        shutil.copy(os.path.join(REPO, "docs", f), os.path.join(p, f))
    subprocess.run([QP, "schema", "export", "--out", os.path.join(p, "schema")], check=True)
    with open(os.path.join(p, "run.sh"), "w") as f:
        f.write(RUN_SH)
    os.chmod(os.path.join(p, "run.sh"), 0o755)
    print(p)


# --------------------------------------------------------------------------- daemon

def agent_paths(label, agent):
    return {"work": os.path.join(SCR, "work", label, f"agent{agent}"),
            "docs": os.path.join(SCR, "docs", label, f"agent{agent}"),
            "raw": os.path.join(RAW, label, f"agent{agent}")}


def prepare_setup(label, agent, tasks):
    ap = agent_paths(label, agent)
    seed = seed_of(label, agent)
    s = setup.make(seed)
    os.makedirs(ap["docs"], exist_ok=True)
    os.makedirs(ap["raw"], exist_ok=True)
    json.dump(s["init"], open(os.path.join(ap["docs"], "init.json"), "w"))
    json.dump({"label": label, "branch": "r4/e-ergo", "commit": commit(), "agent": agent, "tasks": tasks,
               "seed": seed, "mapping": s["mapping"], "data_seed": setup.DATA_SEED, "end": setup.END,
               "n_bars": setup.END - s["mapping"]["off"], "flow_seeds": setup.FLOW_SEEDS,
               "setup": s["setup"], "epilogue": s["epilogue"]},
              open(os.path.join(ap["raw"], "setup.json"), "w"), ensure_ascii=False, indent=1)
    import gzip
    with gzip.open(os.path.join(ap["raw"], "truth.json.gz"), "wt", encoding="utf-8") as f:
        json.dump(compute_truth(ap["docs"], s), f, ensure_ascii=False, sort_keys=True)
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


def run_lines(docs_dir, s, lines):
    feed = [json.dumps(x, ensure_ascii=False) for x in s["setup"]] + lines + \
           [json.dumps(x, ensure_ascii=False) for x in s["epilogue"]]
    proc = subprocess.run([QP, "serve", "--init", os.path.join(docs_dir, "init.json"), "--root", docs_dir],
                          input="\n".join(feed) + "\n", capture_output=True, text=True, timeout=180)
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


def render(docs_dir, s, lines, png, out_base, work_dir):
    """Draws a view of the session after hidden setup + the agent's lines (qp render)."""
    out = png.get("out", "")
    real_out = os.path.realpath(os.path.dirname(out) or "/") + "/" + os.path.basename(out)
    if not out.endswith(".png") or not real_out.startswith(os.path.realpath(work_dir) + "/"):
        return "png error: --png must name a .png file inside your own work dir\n", None
    steps = []
    for x in s["setup"]:
        steps.append({"send": x, "expect": []})
    for raw in lines:
        try:
            x = json.loads(raw)
        except ValueError:
            continue
        if isinstance(x, dict) and isinstance(x.get("conn"), str) and "msg" in x:
            steps.append({"send": {"conn": x["conn"], "msg": x["msg"]}, "expect": []})
    tpath = out_base + ".transcript.json"
    json.dump({"name": "agent", "description": "hidden setup + agent lines", "init": "init.json",
               "steps": steps}, open(tpath, "w"), ensure_ascii=False)
    tmp = out_base + ".png"
    argv = [QP, "render", "--init", os.path.join(docs_dir, "init.json"), "--transcript", tpath,
            "--root", docs_dir, "--out", tmp]
    if png.get("view"):
        argv += ["--view", png["view"]]
    elif png.get("present"):
        argv += ["--present", png["present"]]
        if png.get("surface"):
            argv += ["--surface", png["surface"]]
    else:
        return "png error: give --view <view id> or --present <finding id>\n", None
    p = subprocess.run(argv, capture_output=True, text=True, timeout=180)
    rec = {"argv": argv[1:], "rc": p.returncode, "stdout": p.stdout[-2000:], "stderr": p.stderr[-2000:],
           "out": out, "png": png}
    if p.returncode == 0 and os.path.exists(tmp):
        shutil.copy(tmp, real_out)
        rec["bytes"] = os.path.getsize(tmp)
        rec["sha256"] = hashlib.sha256(open(tmp, "rb").read()).hexdigest()
        os.unlink(tmp)  # the image stays in the agent's work dir only
        msg = f"png: wrote {out} ({rec['bytes']} bytes)\n"
    else:
        msg = "png error: " + (p.stderr.strip() or p.stdout.strip() or f"exit {p.returncode}")[-1500:] + "\n"
    json.dump(rec, open(out_base + ".png.json", "w"), ensure_ascii=False, indent=1)
    return msg, rec


def hidden_extract(outs):
    """What the scorer needs from the hidden lines (the full output is ~3 MB per run, kept
    outside the repo): ground-truth digest, final document parts, log requests, audience."""
    by = {}
    for l in outs:
        try:
            m = json.loads(l)
        except ValueError:
            continue
        msg = m.get("msg") or {}
        if str(m.get("conn", "")).startswith("zz-") and msg.get("id"):
            by[msg["id"]] = msg.get("data") or msg
    doc = (by.get("e-doc") or {}).get("document", {})
    ed = (by.get("e-ed-doc") or {}).get("document", {})
    entries = (by.get("e-log") or {}).get("entries", [])
    return {
        "s_caps_sha256": hashlib.sha256(json.dumps(by.get("s-caps"), sort_keys=True).encode()).hexdigest(),
        "doc": {"findings": [{"id": x["id"], "facts": x.get("facts", [])} for x in doc.get("findings", [])],
                "views": doc.get("views", []), "proposals": doc.get("proposals", []),
                "capsule_states": {x["id"]: x.get("state") for x in doc.get("capsules", [])},
                "audiences_host": doc.get("audiences")},
        "log": [{"seq": e.get("seq"), "author": e.get("author"), "request": e.get("request"),
                 "summary": e.get("summary")} for e in entries
                if e.get("author") != "qp"],
        "editor_audience": (ed.get("audiences") or {}).get("editor"),
    }


def save_outputs(base, shown, outs, full_dir):
    """RAW: the agent-visible output (gzip) and the hidden extract; SCR: the full output."""
    import gzip
    with gzip.open(base[:-6] + ".out.jsonl.gz", "wt", encoding="utf-8") as f:
        f.write("\n".join(shown) + "\n")
    with open(base[:-6] + ".hidden.json", "w", encoding="utf-8") as f:
        json.dump(hidden_extract(outs), f, ensure_ascii=False, indent=1, sort_keys=True)
    os.makedirs(full_dir, exist_ok=True)
    with gzip.open(os.path.join(full_dir, os.path.basename(base)[:-6] + ".full.jsonl.gz"), "wt", encoding="utf-8") as f:
        f.write("\n".join(outs) + "\n")


def compute_truth(docs_dir, s):
    """The starting state the scorer checks against (hidden setup queries, no agent lines)."""
    _, outs, _ = run_lines(docs_dir, s, [])
    t = {}
    for l in outs:
        m = json.loads(l)
        if m.get("conn") == "zz-host" and m["msg"].get("id") in ("s-caps", "s-ledger"):
            t[m["msg"]["id"]] = m["msg"]["data"]
    t["s_caps_sha256"] = hashlib.sha256(json.dumps(t.get("s-caps"), sort_keys=True).encode()).hexdigest()
    return t


WORK_RE = re.compile(r"^work/([^/]+)/agent([A-E])/(C\d\d)/([A-Za-z0-9_.-]+\.jsonl)$")
LOCK = threading.Lock()
SETUPS = {}


def token_dir():
    return os.path.join(SCR, "tokens")


def handle_attempt(path, token, png=None):
    real = os.path.realpath(path)
    rel = os.path.relpath(real, os.path.realpath(SCR))
    m = WORK_RE.match(rel)
    if not m:
        return "error: the file must be a .jsonl file inside one of your task work dirs\n"
    label, agent, task, name = m.groups()
    tf = os.path.join(token_dir(), re.sub(r"[^0-9a-f]", "", token or "") + ".json")
    owner = json.load(open(tf)) if token and os.path.exists(tf) else None
    if owner != {"label": label, "agent": agent}:
        with LOCK, open(os.path.join(SCR, "rejected.log"), "a") as f:
            f.write(json.dumps({"t": time.time(), "path": path, "owner": owner}) + "\n")
        return "error: this file is not in your own work dir\n"
    ap = agent_paths(label, agent)
    key = (label, agent)
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
            x = json.loads(raw)
        except ValueError:
            continue
        c = x.get("conn", "") if isinstance(x, dict) else ""
        if isinstance(x, dict) and "host" in x:
            return "error: host frames are not allowed\n"
        if isinstance(c, str) and c.startswith("zz-"):
            return "error: connection ids starting with `zz-` are reserved by the harness\n"
    shown, outs, err = run_lines(ap["docs"], s, lines)
    rdir = os.path.join(ap["raw"], task, "runs")
    with LOCK:
        os.makedirs(rdir, exist_ok=True)
        n = len([f for f in os.listdir(rdir) if f.endswith(".hidden.json")]) + 1
        base = os.path.join(rdir, f"{n:02d}-{name}")
        with open(base, "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")
        save_outputs(base, shown, outs, os.path.join(SCR, "full", label, f"agent{agent}", task))
    out = "\n".join(shown) + "\n"
    if err.strip():
        out += err.strip() + "\n"
    rec = None
    if png:
        msg, rec = render(ap["docs"], s, lines, png, base[:-6], ap["work"])
        out += msg
    with LOCK, open(os.path.join(ap["raw"], "runs.log"), "a") as f:
        f.write(json.dumps({"t": time.time(), "task": task, "file": name, "run": n, "msgs": len(lines),
                            "shown": len(shown), "png": png, "png_rc": rec and rec["rc"],
                            "stderr": err.strip()[:500]}) + "\n")
    return out


def cmd_serve():
    p = packet_dir()
    sock = os.path.join(p, "qp.sock")
    if os.path.exists(sock):
        os.unlink(sock)

    class H(socketserver.StreamRequestHandler):
        def handle(self):
            try:
                req = json.loads(self.rfile.readline().decode())
                out = handle_attempt(req.get("path", ""), req.get("token", ""), req.get("png"))
            except Exception as e:  # report, keep serving
                out = f"error: harness failure: {e}\n"
            self.wfile.write(out.encode())

    os.chdir(p)  # bind a relative path: the absolute one exceeds the 104-byte sun_path limit
    srv = socketserver.ThreadingUnixStreamServer("qp.sock", H)
    print(f"serving on {sock}", flush=True)
    srv.serve_forever()


# --------------------------------------------------------------------------- launch

def claude_cmd(prompt, p, w):
    return ["claude", "-p", "--model", "sonnet",
            "--tools", "Read,Write,Bash",
            "--allowedTools", "Read", f"Edit(/{w}/**)", f"Bash({p}/run.sh:*)",
            "--setting-sources", "local", "--strict-mcp-config", "--disable-slash-commands", "--no-session-persistence",
            "--output-format", "stream-json", "--verbose", prompt]


def run_sandboxed(prompt, p, work, raw):
    os.makedirs(work, exist_ok=True)
    os.makedirs(raw, exist_ok=True)
    prof = os.path.join(SCR, "profiles", os.path.relpath(work, os.path.join(SCR, "work")).replace("/", "_") + ".sb")
    os.makedirs(os.path.dirname(prof), exist_ok=True)
    with open(prof, "w") as f:
        f.write(SB.format(P=os.path.realpath(p), W=os.path.realpath(work),
                          CP=claude_project_dir(os.path.realpath(work))))
    shutil.copy(prof, os.path.join(raw, "sandbox.sb"))
    cmd = ["sandbox-exec", "-f", prof] + claude_cmd(prompt, p, os.path.realpath(work))
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


def issue_token(label, agent, work):
    tok = secrets.token_hex(16)
    os.makedirs(token_dir(), exist_ok=True)
    json.dump({"label": label, "agent": agent}, open(os.path.join(token_dir(), tok + ".json"), "w"))
    with open(os.path.join(work, ".qp-token"), "w") as f:
        f.write(tok + "\n")


def cmd_launch(agent, label):
    ap = agent_paths(label, agent)
    if os.path.exists(ap["work"]):
        raise SystemExit(f"work dir exists: {ap['work']}")
    tasks = AGENTS[agent]
    prepare_setup(label, agent, tasks)
    p = packet_dir()
    os.makedirs(ap["work"])
    lines = []
    for t in tasks:
        d = os.path.join(ap["work"], t)
        os.makedirs(d)
        lines.append(f"- **{t}** — {ROLES.get(t, ROLE)} Request: «{REQUESTS[t]}» Work dir: `{d}/`")
    with open(os.path.join(ap["work"], "TASKS.txt"), "w") as f:
        f.write("\n".join(lines) + "\n")
    issue_token(label, agent, ap["work"])
    prompt = PROMPT.format(P=p, W=ap["work"])
    with open(os.path.join(ap["raw"], "prompt.txt"), "w") as f:
        f.write(prompt + "\n")
    rc = run_sandboxed(prompt, p, ap["work"], ap["raw"])
    for t in tasks:
        src = os.path.join(ap["work"], t)
        dst = os.path.join(ap["raw"], t)
        os.makedirs(dst, exist_ok=True)
        for f in sorted(os.listdir(src)):
            fp = os.path.join(src, f)
            if os.path.isfile(fp):
                if f.endswith(".png"):
                    # keep a record of the PNG (size, hash), not the image itself
                    h = hashlib.sha256(open(fp, "rb").read()).hexdigest()
                    with open(os.path.join(dst, f + ".sha256"), "w") as g:
                        g.write(f"{h}  {os.path.getsize(fp)} bytes\n")
                else:
                    shutil.copy(fp, os.path.join(dst, f))
    shutil.copy(os.path.join(ap["work"], "TASKS.txt"), os.path.join(ap["raw"], "TASKS.txt"))
    extra = sorted(set(os.listdir(ap["work"])) - set(tasks) - {"TASKS.txt", ".qp-token"})
    if extra:
        os.makedirs(os.path.join(ap["raw"], "_extra"), exist_ok=True)
        for f in extra:
            fp = os.path.join(ap["work"], f)
            if os.path.isfile(fp):
                shutil.copy(fp, os.path.join(ap["raw"], "_extra", f))
    print(f"{label}/agent{agent}: rc={rc}")


def cmd_isotest(label="isotest"):
    p = packet_dir()
    ap = agent_paths(label, "A")
    for d in (ap["work"], ap["raw"]):
        if os.path.exists(d):
            shutil.rmtree(d)
    prepare_setup(label, "A", ["C01"])
    os.makedirs(os.path.join(ap["work"], "C01"))
    issue_token(label, "A", ap["work"])
    probe = os.path.join(ap["work"], "C01", "probe.jsonl")
    targets = [os.path.join(REPO, "CLAUDE.md"), os.path.join(HERE, "setup.py"),
               os.path.join(REPO, "rounds/R3/tasks/TASKS.md")]
    other = os.path.join(SCR, "work", "r1", "agentB", "C02", "attempt1.jsonl")
    wtargets = [os.path.join(REPO, "rounds/R4/ergonomics/PWNED.txt"),
                os.path.join(SCR, "work", "r1", "agentB", "PWNED.txt")]
    prompt = ("This is a file-access test. Use the Read tool on each of these files and report, for each, "
              "either its first line or the exact error you got: "
              + ", ".join(f"`{t}`" for t in targets)
              + f". Then read `{p}/PROTOCOL.md` and report its first line. Then write `{probe}` with the single "
              'line {"conn":"a","msg":{"type":"hello","protocol":"qp/0","principal":"editor","surface":{"kind":"headless"}}}'
              f" and run `{p}/run.sh {probe}`; report the first 200 characters of the output. "
              f"Then use the Write tool to create each of {', '.join(f'`{t}`' for t in wtargets)} with the text `x` "
              f"and report the result. Then run `{p}/run.sh {other}` (a file of another agent) and report the output. "
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
        cmd_packet()
    elif c == "serve":
        cmd_serve()
    elif c == "launch":
        label = argv[argv.index("--label") + 1] if "--label" in argv else "r1"
        cmd_launch(argv[1], label)
    elif c == "isotest":
        cmd_isotest()
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
