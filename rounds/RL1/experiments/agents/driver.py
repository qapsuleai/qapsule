#!/usr/bin/env python3
"""RL1 ajan yazabilirlik teyidi — sürücü (GOAL §7; yalnız standart kütüphane).

`agents/run.py` (R5B) örneğinde: koşu başına taze `qp mcp serve --http`, istem dosyası,
durdurma ve motorla denetim. Model çağrısını sürücü yapmaz; çağıran (orkestratör) istem
dosyasını bir Claude Code alt ajanına verir. Ajan sunucuya yalnız istemdeki `qmcpx` komutuyla
(R5B `qmcp` istemcisinin üstünde ince katman) erişir. Tek görev = tek alt ajan oturumu (RX4).

  driver.py tasks                         seçilen 20 görev (SELECTION.md)
  driver.py start <model> <görev> [--field text|text_r] [--tag T]
        -> JSON {run, workdir, prompt_file, url, pid}
        koşu klasörü: $QP_RL1_AGENTS/<model>-<görev>[-T]/ (depo dışı; zorunlu ortam değişkeni)
        içinde: session/init.json + document.json (RX4 `seen` serisi, kimlik `rx4`, 1h, UTC,
        kapalı pencere), server.log.jsonl, state/document.json (--dump), prompt.md, meta.json
  driver.py stop <koşu> [--tokens N] [--tool-uses N] [--ms N] [--transcript F] [--no-engine]
        sunucuyu kapatır; kullanımı meta.json'a yazar: belirteç alt ajan dökümünden RX4
        tanımıyla (`--transcript`, yoksa ~/.claude/projects/*/*/subagents/ altında istem
        yolunu anan en yeni döküm), bildirimdeki sayı ayrıca; yalıtım taraması; sonra `engine`
  driver.py engine <koşu>
        her teslimi motorla yeniden koşar (komut satırı, PS §3.3): `check` → engine/N.check.json,
        program ise veri kümesi başına `run --full` → engine/N.<küme>.out.json. Puanlama
        (`score.py`) yalnız bu dosyaları okur.

Ortam: QP_RL1_AGENTS (koşu kökü, zorunlu), CARGO_TARGET_DIR (varsayılan <depo>/target),
QP_BIN (varsayılan $CARGO_TARGET_DIR/release/qp), QP_RUN_TMPL / QP_CHECK_TMPL (komut
şablonları; yer tutucular {qp} {program} {data} {id} {set}), QP_SETS (varsayılan
seen,heldout).
"""
import hashlib
import json
import os
import re
import signal
import socket
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "rx4"))
sys.path.insert(0, str(HERE))
import bars as barsmod  # noqa: E402
import conform  # noqa: E402
from rx4common import DATA_DIR, DEFAULT_RX4, REPO, load_json  # noqa: E402
from score import SELECTION  # noqa: E402

TARGET = Path(os.environ.get("CARGO_TARGET_DIR") or REPO / "target")
QP = os.environ.get("QP_BIN") or str(TARGET / "release" / "qp")
RUN_TMPL = os.environ.get("QP_RUN_TMPL") or "{qp} program run --json --full --data {data} {program}"
CHECK_TMPL = os.environ.get("QP_CHECK_TMPL") or "{qp} program check --json {program}"
SETS = [s for s in os.environ.get("QP_SETS", "seen,heldout").split(",") if s]
QMCPX = HERE / "qmcpx"
PREAMBLE = HERE / "PREAMBLE.md"
CARD = HERE / "LANG-CARD.md"
SERIES_ID = "rx4"
PRINCIPAL = "claude"
HOUR_MS = 3_600_000


def out_root():
    r = os.environ.get("QP_RL1_AGENTS")
    if not r:
        raise SystemExit("QP_RL1_AGENTS is not set (raw runs live outside the repo, e.g. the "
                         "session scratchpad)")
    return Path(r)


def dump(p, v):
    p = Path(p)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(v, ensure_ascii=False, indent=1, sort_keys=True) + "\n",
                 encoding="utf-8")


def sha(p):
    return "sha256:" + hashlib.sha256(Path(p).read_bytes()).hexdigest()


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


# ------------------------------------------------------------------ oturum

def write_session(wd):
    """SessionInit + Document: tek bar serisi `rx4` (RX4 seen, 3000 × 1h, takvimsiz → UTC
    hizası). Saat son barın kapanışından sonra: bütün barlar kapanmış (kapalı pencere, PR §1).
    Ajan ilkesi `claude` (ajan, öneren, Arkın adına; `--init` ile sunucu ajan eklemez)."""
    series = load_json(DATA_DIR / "seen.bars.json")
    series = dict(series, id=SERIES_ID)
    last_t = series["data"]["bars"][-1]["t"]
    doc = {
        "schema_version": "0", "id": "doc-rl1-agents", "title": "RL1 ajan teyidi (RX4 seen)",
        "display_tz": "UTC", "series": [series],
        "views": [{"id": "main", "title": "RX4 1h", "x": {"kind": "time"},
                   "y": [{"kind": "linear"}],
                   "layers": [{"mark": "candle", "series": SERIES_ID, "panel": 0, "encoding": {}}]}],
        "facts": [], "findings": [], "capsules": [], "intents": [], "ledger": {"entries": []},
    }
    init = {
        "document": "document.json", "clock": last_t + 2 * HOUR_MS, "epoch": 0,
        "proposal_ttl_ms": None,
        "principals": [
            {"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"},
            {"id": PRINCIPAL, "author": {"kind": "agent", "name": PRINCIPAL}, "role": "proposer",
             "on_behalf": "arkin"},
            {"id": "host", "author": {"kind": "engine", "name": "host", "version": "0"},
             "role": "host"},
        ],
    }
    sd = Path(wd) / "session"
    dump(sd / "document.json", doc)
    dump(sd / "init.json", init)
    return sd


def serve(wd):
    sd = Path(wd) / "session"
    port = free_port()
    cmd = [QP, "mcp", "serve", "--http", "--listen", "127.0.0.1", "--port", str(port),
           "--principal", PRINCIPAL, "--surface", "thin",
           "--init", str(sd / "init.json"), "--root", str(sd),
           "--log", str(Path(wd) / "server.log.jsonl"),
           "--dump", str(Path(wd) / "state" / "document.json")]
    (Path(wd) / "state").mkdir(parents=True, exist_ok=True)
    errp = Path(wd) / "server.stderr"
    err = open(errp, "w")
    p = subprocess.Popen(cmd, cwd=REPO, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                         stderr=err, start_new_session=True)
    for _ in range(300):
        time.sleep(0.1)
        if "listening" in errp.read_text():
            break
        if p.poll() is not None:
            raise SystemExit(f"server exited: {errp.read_text()}")
    return p, f"http://127.0.0.1:{port}/mcp", port, cmd


def kill(pid):
    """Yalnız bu sürücünün başlattığı süreç, kimliğiyle (K92)."""
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    for _ in range(100):
        time.sleep(0.1)
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return


# ------------------------------------------------------------------ istem

def task_outputs_for_qapsule(task):
    """Görevin çıktı listesi Qapsule adlarıyla: RX4 `fingerprint` → `state` (PR §5.4)."""
    outs = []
    for o in task["outputs"]:
        o = dict(o)
        if o["kind"] == "fingerprint":
            o["kind"] = "state"
        outs.append(o)
    return outs


def qmcpx_line(url, run, wd):
    return (f"env QMCP_URL={url} QMCP_PRINCIPAL={PRINCIPAL} QMCP_RUN={run} "
            f"QMCP_STATE={wd}/mcp.sid QMCPX_DIR={wd} QMCPX_INPUTS='[\"{SERIES_ID}\"]' "
            f"python3 {QMCPX}")


def write_prompt(wd, run, url, task, field="text"):
    pre = PREAMBLE.read_text(encoding="utf-8")
    body = pre.split("<!-- PREAMBLE-START -->", 1)[-1].split("<!-- PREAMBLE-END -->", 1)[0]
    body = (body.replace("{CARD}", str(CARD)).replace("{WORKDIR}", str(wd))
            .replace("{SERIES}", SERIES_ID).replace("{QMCPX}", qmcpx_line(url, run, wd)))
    outs = json.dumps(task_outputs_for_qapsule(task), ensure_ascii=False, indent=1)
    text = (body.strip() + f"\n\n---\n\n# Görev {task['id']}\n\n{task[field]}\n\n"
            "İstenen çıktılar (ad, tür ve görevde yazılı ayrıntılar):\n\n"
            f"```json\n{outs}\n```\n")
    pf = Path(wd) / "prompt.md"
    pf.write_text(text, encoding="utf-8")
    return pf


# ------------------------------------------------------------------ komutlar

def opt(a, name, default=None):
    return a[a.index(name) + 1] if name in a else default


def cmd_tasks(_a):
    tasks = {t["id"]: t for t in conform.load_tasks(DEFAULT_RX4)}
    for tid in SELECTION:
        t = tasks[tid]
        print(tid, t["cluster"], "ret" if conform.is_reject_task(t) else "program",
              ",".join(o["kind"] for o in t["outputs"]))
    return 0


def cmd_start(a):
    model, tid = a[1], a[2]
    if tid not in SELECTION:
        raise SystemExit(f"{tid} is not in SELECTION.md")
    field = opt(a, "--field", "text")
    tag = opt(a, "--tag")
    run = "-".join(x for x in [model, tid, tag] if x)
    wd = out_root() / run
    if (wd / "meta.json").exists():
        raise SystemExit(f"{wd} exists: choose another --tag or remove it")
    wd.mkdir(parents=True, exist_ok=True)
    task = {t["id"]: t for t in conform.load_tasks(DEFAULT_RX4)}[tid]
    write_session(wd)
    p, url, port, cmd = serve(wd)
    pf = write_prompt(wd, run, url, task, field)
    meta = {"run": run, "model": model, "task": tid, "field": field, "pid": p.pid,
            "port": port, "url": url, "server_cmd": cmd, "sets": SETS,
            "started": time.strftime("%Y-%m-%dT%H:%M:%S%z"), "started_epoch": time.time(),
            "preamble_sha": sha(PREAMBLE), "card_sha": sha(CARD), "qp_bin": QP}
    dump(wd / "meta.json", meta)
    print(json.dumps({"run": run, "workdir": str(wd), "prompt_file": str(pf), "url": url,
                      "pid": p.pid}))
    return 0


def run_dir(run):
    wd = out_root() / run
    if not (wd / "meta.json").exists():
        raise SystemExit(f"no run {run} under {out_root()}")
    return wd


def data_files(wd):
    out = {}
    for s in SETS:
        if s == "seen":
            out[s] = DATA_DIR / "seen.bars.json"
        else:
            p = Path(wd) / "engine" / f"{s}.bars.json"
            if not p.exists():
                barsmod.write_series(barsmod.series_from_rx4(DEFAULT_RX4, s), p)
            out[s] = p
    return out


def is_reject_file(path):
    text = Path(path).read_text(encoding="utf-8")
    if Path(path).suffix == ".json":
        try:
            obj = json.loads(text)
        except ValueError:
            return False
        return isinstance(obj, dict) and "reject" in obj
    first = next((ln.strip() for ln in text.splitlines()
                  if ln.strip() and not ln.strip().startswith("#")), "")
    return first == "reject" or first.startswith("reject ")


def engine_check(wd, run_tmpl=None, check_tmpl=None, qp=None):
    """Teslimleri motorla yeniden koşar; sonuçlar engine/ altına. Döner: işlenen teslim."""
    wd = Path(wd)
    run_tmpl, check_tmpl, qp = run_tmpl or RUN_TMPL, check_tmpl or CHECK_TMPL, qp or QP
    meta = load_json(wd / "meta.json")
    ed = wd / "engine"
    ed.mkdir(parents=True, exist_ok=True)
    data = data_files(wd)
    n = 0
    from score import submissions_of  # noqa: E402 (aynı klasör)
    for k, path in submissions_of(wd):
        rt = run_tmpl.replace("{qp}", qp)
        ct = check_tmpl.replace("{qp}", qp)
        obj, err, rc, _ = conform.run_cmd(ct, program=str(path), data="", id=meta["task"], set="")
        dump(ed / f"{k}.check.json", {"rc": rc, "err": err, "obj": obj})
        if not is_reject_file(path):
            for s, dp in data.items():
                obj, err, rc, _ = conform.run_cmd(rt, program=str(path), data=str(dp),
                                                  id=meta["task"], set=s)
                dump(ed / f"{k}.{s}.out.json", {"rc": rc, "err": err, "obj": obj})
        n += 1
    return n


def usage_of(transcript):
    """Alt ajan dökümünden (Claude Code `subagents/agent-*.jsonl`) RX4 tanımıyla belirteç:
    istek başına girdi + önbellek okuma + önbellek yazma + çıktı, kümülatif; aynı ileti
    kimliğinin satırları bir kez (RX4 COMPUTE-LANGUAGE §7). Ayrıca araç çağrıları ve model."""
    seen, tok, tools, models, reads, bash = {}, {"in": 0, "cache_r": 0, "cache_w": 0, "out": 0}, 0, set(), [], []
    for line in Path(transcript).read_text(encoding="utf-8").splitlines():
        try:
            r = json.loads(line)
        except ValueError:
            continue
        m = r.get("message") if isinstance(r.get("message"), dict) else {}
        if r.get("type") != "assistant":
            continue
        mid = m.get("id") or r.get("requestId") or r.get("uuid")
        if mid not in seen and isinstance(m.get("usage"), dict):
            u = m["usage"]
            seen[mid] = True
            tok["in"] += u.get("input_tokens", 0) or 0
            tok["cache_r"] += u.get("cache_read_input_tokens", 0) or 0
            tok["cache_w"] += u.get("cache_creation_input_tokens", 0) or 0
            tok["out"] += u.get("output_tokens", 0) or 0
            if m.get("model"):
                models.add(m["model"])
        for c in m.get("content") or []:
            if isinstance(c, dict) and c.get("type") == "tool_use":
                tools += 1
                inp = c.get("input") or {}
                if c.get("name") in ("Read", "Grep", "Glob"):
                    reads.append(str(inp.get("file_path") or inp.get("path") or inp.get("pattern")))
                elif c.get("name") == "Bash":
                    bash.append(str(inp.get("command", "")))
    return {"tokens": sum(tok.values()), "tokens_detail": tok, "requests": len(seen),
            "tool_uses": tools, "model_ids": sorted(models), "reads": reads, "bash": bash}


def violations(wd, u):
    """Yalıtım taraması (RX4 `runs/collect.py` fikri; kod kopyalanmadı): okunmasına izin
    verilenler kart, istem ve koşu klasörü; Bash'te `qmcpx` ve koşu klasörü dışı depo ya da
    `refs` yolu, `docs/` ve `cargo`/`git` bayrak olur (elle okunur)."""
    ok = (str(CARD), str(Path(wd)), str(Path(wd) / "prompt.md"))
    v = [f"read {p}" for p in u["reads"] if not any(p.startswith(x) for x in ok)]
    for cmd in u["bash"]:
        rest = cmd.replace(str(QMCPX), "").replace(str(Path(wd)), "").replace(str(CARD), "")
        if (str(REPO) in rest or "/refs/" in rest or "docs/" in rest
                or re.search(r"\b(cargo|git)\b", rest)):
            v.append(f"bash {cmd[:160]}")
    return v


def find_transcript(wd, started_after=0.0):
    """Koşu klasörünü (istem yolu) anan en yeni alt ajan dökümü."""
    base = Path.home() / ".claude" / "projects"
    key = str(Path(wd) / "prompt.md")
    hits = []
    for p in base.glob("*/*/subagents/agent-*.jsonl"):
        try:
            if p.stat().st_mtime < started_after:
                continue
            with open(p, encoding="utf-8") as f:
                head = f.read(200_000)
        except OSError:
            continue
        if key in head or str(Path(wd)) in head:
            hits.append((p.stat().st_mtime, p))
    return str(max(hits)[1]) if hits else None


def cmd_stop(a):
    run = a[1]
    wd = run_dir(run)
    meta = load_json(wd / "meta.json")
    kill(meta["pid"])
    kv = {a[i]: a[i + 1] for i in range(2, len(a) - 1) if a[i].startswith("--")}
    meta.update({"tokens_notification": int(kv.get("--tokens", 0)) or None,
                 "tool_uses": int(kv.get("--tool-uses", 0)) or None,
                 "duration_ms": int(kv.get("--ms", 0)) or None, "stopped": True})
    tp = kv.get("--transcript") or find_transcript(wd, meta.get("started_epoch", 0.0))
    if tp and Path(tp).exists():
        u = usage_of(tp)
        meta.update({"transcript": tp, "tokens": u["tokens"], "tokens_detail": u["tokens_detail"],
                     "requests": u["requests"], "tool_uses_transcript": u["tool_uses"],
                     "model_ids": u["model_ids"], "violations": violations(wd, u)})
        meta["tool_uses"] = meta["tool_uses"] or u["tool_uses"]
    else:
        meta.update({"transcript": None, "tokens": meta["tokens_notification"]})
    dump(wd / "meta.json", meta)
    n = 0 if "--no-engine" in a else engine_check(wd)
    print(json.dumps({"run": run, "engine_checked": n, "tokens": meta["tokens"],
                      "transcript": meta["transcript"], "violations": len(meta.get("violations") or []),
                      "dump": (wd / "state" / "document.json").exists()}))
    return 0


def cmd_engine(a):
    print(json.dumps({"run": a[1], "engine_checked": engine_check(run_dir(a[1]))}))
    return 0


def main(a):
    cmds = {"tasks": (cmd_tasks, 1), "start": (cmd_start, 3), "stop": (cmd_stop, 2),
            "engine": (cmd_engine, 2)}
    if not a or a[0] not in cmds or len(a) < cmds[a[0]][1]:
        print(__doc__)
        return 2
    return cmds[a[0]][0](a)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
