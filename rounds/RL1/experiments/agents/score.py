#!/usr/bin/env python3
"""RL1 ajan yazabilirlik teyidi — puanlayıcı (GOAL §7; VERIFY 18; yalnız standart kütüphane).

    python3 -I score.py <koşu kökü> [--models haiku,opus] [--out RESULT.md] [--sets seen,heldout]

`<koşu kökü>/<model>-<görev>/` klasörlerini (driver.py yazar) okur; ağ, model ve motor
çağırmaz: motor çıktıları `driver.py stop` sırasında `engine/` altına yazılmıştır. Böylece
puan iki kez aynı çıkar ve çevrimdışı sınanır (`selftest.py`).

Teslim durumu RX4 `harness/SELECTION.md` §2 ile aynı tanım (kod kopyalanmadı):
`correct`, `silent_wrong`, `error`, `false_reject`, `reject_wrong_code`. Program görevinde doğru
⇔ seçilen veri kümelerinin hepsinde görevin bütün çıktıları RX4 `expected/<id>/<küme>.json`
ile eşit; karşılaştırma `../rx4/conform.py` (`compare_task`, `adapt_outputs`) ile.

Dört ölçüt (GOAL §7) ve ek kayıtlar:
  first_try      ilk teslimi `correct` olan görev (RX4 M1)
  final          son teslimi `correct` olan görev (RX4 M2); M3 = ilki yanlış, sonu doğru
  after_error    son teslimden önce motordan hata iletisi (check/try/explain/add/submit
                 cevabında `E_` kodu ya da qp/0 `error`) almış görevlerden sonu doğru olanlar
  reject_needed  ret görevlerinde (4) son teslim kabul kodlu ret; ilk teslim; program görevinde
                 yanlış ret (0 olmalı)
  invented       `E_UNKNOWN_OP` cevapları: uydurma işlev adları (ad, görev, model)
  ek             sessiz yanlış (M4), seen doğru / heldout yanlış (M5), teslimsiz, metin/IR,
                 check'siz teslim, öneri (`add_program`) sonucu, program aracı dışı çağrı,
                 belirteç ve araç çağrısı (meta.json; driver `stop` bayraklarından)
"""
import argparse
import json
import re
import statistics
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "rx4"))
import bars as barsmod  # noqa: E402
import conform  # noqa: E402
from rx4common import DEFAULT_RX4, load_json  # noqa: E402

SELECTION = ["G1-01", "G1-03", "G2-01", "G2-03", "G3-01", "G3-02", "G3-03", "G4-01", "G4-02",
             "G4-04", "G5-01", "G5-02", "G5-03", "G7-01", "G7-03", "G7-05", "G8-01", "G8-03",
             "G9-02", "G9-03"]
PROGRAM_TOOLS = ("check_program", "try_program", "explain_program", "add_program")
LOCAL_TOOLS = ("_list", "_schema", "submit")
DECISION_RE = re.compile(r"approve|decide|accept|reject_proposal|decision", re.I)

# RX4 kol A (metin dil; Qapsule'ün seçtiği yüzey) — ../refs/docs/rounds/RX4/REPORT.md §1,
# ../refs/reports/COMPUTE-LANGUAGE.md §3.1 ve §7; alt küme sütunları bu 20 görev için
# ../refs/experiments/rx4/runs/sessions.json'dan (PLAN.md §5'te nasıl hesaplandığı).
BASELINE = {
    "haiku": {"first_try": 20, "final": 20, "m3": "—", "silent_wrong": 0, "reject_final": 4,
              "reject_first": 4, "false_reject": 0, "after_error": "0/0", "invented": 0,
              "failed_check_sessions": 0, "tokens_median": 270696, "tools_mean": 7.45,
              "full30_tokens_median": 270704, "full30_tools_mean": 7.53},
    "opus": {"first_try": 20, "final": 20, "m3": "—", "silent_wrong": 0, "reject_final": 4,
             "reject_first": 4, "false_reject": 0, "after_error": "0/0", "invented": 0,
             "failed_check_sessions": 0, "tokens_median": 207990, "tools_mean": 5.15,
             "full30_tokens_median": 208070, "full30_tools_mean": 5.17},
}


# ------------------------------------------------------------------ cevap okuma

def reply_errors(reply):
    """Motor cevabındaki hata nesneleri: `reason` (ya da RX4 biçimi `code`) `E_` ile başlayan
    sözlükler; ret nesnesi (`reject`) hata değildir. qp/0 `error` cevabı alt kodsuz da olsa
    bir hata sayılır."""
    found = []

    def walk(x):
        if isinstance(x, dict):
            r = x.get("reason") if isinstance(x.get("reason"), str) else x.get("code")
            # PS §3.1: the top-level error repeats the first of `errors[]`; count the list only
            has_list = isinstance(x.get("errors"), list) and bool(x["errors"])
            if isinstance(r, str) and re.match(r"^E_[A-Z_]+$", r) and not has_list:
                found.append(x)
            for k, v in x.items():
                if k != "reject":
                    walk(v)
        elif isinstance(x, list):
            for v in x:
                walk(v)
    walk(reply)
    if not found and isinstance(reply, dict) and reply.get("type") == "error":
        found.append(reply.get("error") or reply)
    if not found and isinstance(reply, dict):
        data = reply.get("data") if isinstance(reply.get("data"), dict) else reply
        if data.get("ok") is False:
            found.append({"reason": "E_?", "message": "ok:false without an error object"})
    return found


def err_reason(e):
    r = e.get("reason")
    return r if isinstance(r, str) else e.get("code")


def invented_name(err, args):
    """`E_UNKNOWN_OP` hatasından uydurulan ad: iletideki ilk `…` alıntısı, yoksa `span`ın
    kaynak metindeki karşılığı, yoksa IR yolundaki `op`."""
    m = re.search(r"`([^`]+)`", str(err.get("message", "")))
    if m:
        return m.group(1)
    span, src = err.get("span"), (args or {}).get("source")
    if isinstance(span, dict) and isinstance(src, str):
        lines = src.splitlines()
        ln, col, n = span.get("line", 0), span.get("col", 0), span.get("len", 0)
        if 1 <= ln <= len(lines):
            return lines[ln - 1][col - 1:col - 1 + n] or "?"
    at = err.get("at")
    prog = (args or {}).get("program")
    if isinstance(at, str) and isinstance(prog, dict):
        node = prog
        for part in at.strip("/").split("/"):
            node = node[int(part)] if isinstance(node, list) and part.isdigit() else \
                (node.get(part) if isinstance(node, dict) else None)
            if node is None:
                break
        if isinstance(node, str):
            return node
    return "?"


def reject_code(source_text, ir_obj, reply):
    c = conform.reject_code_in(reply) if reply else None
    if c:
        return c
    if isinstance(ir_obj, dict) and isinstance(ir_obj.get("reject"), dict):
        return ir_obj["reject"].get("code")
    if source_text:
        m = re.search(r"^\s*reject\s+(E_[A-Z_]+)", source_text, re.M)
        if m:
            return m.group(1)
    return None


def find_ir(reply):
    if isinstance(reply, dict):
        if isinstance(reply.get("ir"), dict):
            return reply["ir"]
        for k in ("data", "result", "reply"):
            r = find_ir(reply.get(k))
            if r is not None:
                return r
    return None


# ------------------------------------------------------------------ veri

class Truth:
    """Görevler, beklenen çıktılar ve bar zaman indeksleri (RX4 düzeneği, salt okunur)."""

    def __init__(self, rx4, sets):
        self.rx4 = Path(rx4)
        self.sets = sets
        self.tasks = {t["id"]: t for t in conform.load_tasks(self.rx4)}
        self.tindex = {}
        for s in sets:
            times = barsmod.bar_times(barsmod.series_from_rx4(self.rx4, s))
            self.tindex[s] = {t: i for i, t in enumerate(times)}
        self._exp = {}

    def expected(self, tid, s):
        k = (tid, s)
        if k not in self._exp:
            self._exp[k] = load_json(self.rx4 / "expected" / tid / f"{s}.json")
        return self._exp[k]


# ------------------------------------------------------------------ bir koşu

def submissions_of(rd):
    sd = rd / "submissions"
    if not sd.is_dir():
        return []
    out = []
    for p in sd.iterdir():
        head = p.name.split(".")[0]
        if head.isdigit() and ".reply." not in p.name and p.suffix in (".qp", ".json"):
            out.append((int(head), p))
    return sorted(out)


def read_jsonl(p):
    rows = []
    if p.exists():
        for line in p.read_text(encoding="utf-8").splitlines():
            if line.strip():
                try:
                    rows.append(json.loads(line))
                except ValueError:
                    rows.append({"tool": "_unparsable"})
    return rows


def classify(task, n, path, rd, truth):
    """Bir teslimin durumu ve ayrıntısı."""
    text = path.read_text(encoding="utf-8")
    ir_obj = None
    if path.suffix == ".json":
        try:
            ir_obj = json.loads(text)
        except ValueError:
            ir_obj = None
    reply_p = rd / "submissions" / f"{n}.reply.json"
    reply = load_json(reply_p) if reply_p.exists() else None
    eng_check_p = rd / "engine" / f"{n}.check.json"
    eng_check = load_json(eng_check_p) if eng_check_p.exists() else None
    is_rej = (isinstance(ir_obj, dict) and "reject" in ir_obj) or (
        path.suffix == ".qp" and re.match(r"^\s*(#.*\n\s*)*reject\b", text) is not None)
    row = {"n": n, "surface": "text" if path.suffix == ".qp" else "ir",
           "kind": "reject" if is_rej else "program"}
    reject_task = conform.is_reject_task(task)
    if is_rej:
        code = reject_code(text, ir_obj, (eng_check or {}).get("obj") or reply)
        row["code"] = code
        if not reject_task:
            row["status"] = "false_reject"
        elif code in task["accept"]["reject"]:
            row["status"] = "correct"
        else:
            row["status"] = "reject_wrong_code"
        return row
    # program teslimi: motor çıktıları (driver stop)
    ir = ir_obj if isinstance(ir_obj, dict) and "nodes" in ir_obj else (
        find_ir((eng_check or {}).get("obj")) or find_ir(reply) or {})
    per_set, all_ok, any_run = {}, True, True
    for s in truth.sets:
        op = rd / "engine" / f"{n}.{s}.out.json"
        if not op.exists():
            per_set[s] = "motor çıktısı yok"
            all_ok, any_run = False, False
            continue
        out = load_json(op)
        obj = out.get("obj")
        got = conform.adapt_outputs(obj, ir, truth.tindex[s]) if isinstance(obj, dict) else None
        if out.get("rc", 0) != 0 or got is None:
            codes = conform.reasons_in(obj) if obj is not None else []
            per_set[s] = f"motor koşmadı (çıkış {out.get('rc')}): {', '.join(codes) or out.get('err') or '?'}"
            all_ok, any_run = False, False
            continue
        if reject_task:
            per_set[s] = "hatasız koştu (ret gerekiyordu)"
            all_ok = False
            continue
        ok, why, _bits = conform.compare_task(task, got, truth.expected(task["id"], s))
        per_set[s] = "eşit" if ok else why
        all_ok = all_ok and ok
    row["sets"] = per_set
    if not any_run:
        row["status"] = "error"
    elif all_ok:
        row["status"] = "correct"
    else:
        row["status"] = "silent_wrong"
        if ("seen" in per_set and per_set["seen"] == "eşit"
                and any(v != "eşit" for k, v in per_set.items() if k != "seen")):
            row["m5"] = True
    return row


def score_run(rd, truth):
    meta = load_json(rd / "meta.json")
    tid = meta["task"]
    task = truth.tasks[tid]
    subs = [classify(task, n, p, rd, truth) for n, p in submissions_of(rd)]
    tr = read_jsonl(rd / "transcript.jsonl")
    calls, errors_before_final, invented, offsurface, decisions = {}, 0, [], [], 0
    final_seq = max((r["seq"] for r in tr if r.get("tool") == "submit" and "n" in r), default=None)
    first_submit_seq = min((r["seq"] for r in tr if r.get("tool") == "submit" and "n" in r),
                           default=None)
    checked_before_submit = any(r.get("tool") == "check_program" and
                                (first_submit_seq is None or r["seq"] < first_submit_seq)
                                for r in tr)
    proposals = {"calls": 0, "acked": 0, "try_only": 0, "errors": 0}
    for r in tr:
        tool = r.get("tool", "?")
        calls[tool] = calls.get(tool, 0) + 1
        if tool not in PROGRAM_TOOLS and tool not in LOCAL_TOOLS:
            offsurface.append(tool)
            if DECISION_RE.search(tool):
                decisions += 1
        if tool not in PROGRAM_TOOLS and tool != "submit":
            continue
        errs = reply_errors(r.get("reply")) if r.get("reply") is not None else []
        if errs and (final_seq is None or r["seq"] < final_seq):
            errors_before_final += 1
        for e in errs:
            if err_reason(e) == "E_UNKNOWN_OP":
                invented.append({"name": invented_name(e, r.get("args")), "tool": tool,
                                 "seq": r.get("seq")})
        if tool == "add_program":
            proposals["calls"] += 1
            rep = r.get("reply") or {}
            if rep.get("type") == "ack":
                proposals["acked"] += 1
            elif any(err_reason(e) == "E_TRY_ONLY" for e in errs):
                proposals["try_only"] += 1
            elif errs:
                proposals["errors"] += 1
    first = subs[0]["status"] if subs else None
    last = subs[-1]["status"] if subs else None
    return {
        "run": rd.name, "model": meta["model"], "task": tid, "cluster": task["cluster"],
        "reject_task": conform.is_reject_task(task),
        "documentable": all(o["kind"] in ("series", "event") for o in task["outputs"]),
        "submissions": subs, "first": first, "final": last,
        "error_replies_before_final": errors_before_final,
        "invented": invented, "calls": calls, "offsurface": offsurface,
        "decision_attempts": decisions, "checked_before_submit": checked_before_submit,
        "proposals": proposals,
        "tokens": meta.get("tokens"), "tool_uses": meta.get("tool_uses"),
        "duration_ms": meta.get("duration_ms"),
    }


# ------------------------------------------------------------------ toplam

def totals(rows):
    t = {"tasks": len(rows)}
    t["first_try"] = sum(1 for r in rows if r["first"] == "correct")
    t["final"] = sum(1 for r in rows if r["final"] == "correct")
    fw = [r for r in rows if r["first"] != "correct"]
    t["m3"] = {"first_wrong": len(fw), "recovered": sum(1 for r in fw if r["final"] == "correct")}
    ae = [r for r in rows if r["error_replies_before_final"] > 0]
    t["after_error"] = {"tasks_with_error_reply": len(ae),
                        "final_correct": sum(1 for r in ae if r["final"] == "correct")}
    rj = [r for r in rows if r["reject_task"]]
    t["reject_needed"] = {"tasks": len(rj),
                          "final_ok": sum(1 for r in rj if r["final"] == "correct"),
                          "first_ok": sum(1 for r in rj if r["first"] == "correct"),
                          "program_attempts": sum(1 for r in rj for s in r["submissions"]
                                                  if s["kind"] == "program")}
    subs = [s for r in rows for s in r["submissions"]]
    t["false_rejects"] = sum(1 for s in subs if s["status"] == "false_reject")
    t["silent_wrong"] = sum(1 for s in subs if s["status"] == "silent_wrong")
    t["m5_seen_ok_heldout_wrong"] = sum(1 for s in subs if s.get("m5"))
    t["errors"] = sum(1 for s in subs if s["status"] == "error")
    t["no_submission"] = sum(1 for r in rows if not r["submissions"])
    t["submissions"] = len(subs)
    t["surface"] = {"text": sum(1 for s in subs if s["surface"] == "text"),
                    "ir": sum(1 for s in subs if s["surface"] == "ir")}
    names = {}
    for r in rows:
        for i in r["invented"]:
            names.setdefault(i["name"], set()).add(r["task"])
    t["invented"] = {"replies": sum(len(r["invented"]) for r in rows),
                     "tasks": sum(1 for r in rows if r["invented"]),
                     "names": {k: sorted(v) for k, v in sorted(names.items())}}
    t["failed_check_sessions"] = sum(1 for r in rows if r["error_replies_before_final"] > 0)
    t["submitted_without_check"] = sum(1 for r in rows if r["submissions"]
                                       and not r["checked_before_submit"])
    doc = [r for r in rows if r["documentable"] and not r["reject_task"]]
    t["proposals"] = {"documentable_tasks": len(doc),
                      "proposed": sum(1 for r in doc if r["proposals"]["acked"] > 0),
                      "try_only_replies": sum(r["proposals"]["try_only"] for r in rows),
                      "add_errors": sum(r["proposals"]["errors"] for r in rows)}
    t["offsurface_calls"] = sum(len(r["offsurface"]) for r in rows)
    t["decision_attempts"] = sum(r["decision_attempts"] for r in rows)
    tok = [r["tokens"] for r in rows if r["tokens"]]
    tu = [r["tool_uses"] for r in rows if r["tool_uses"]]
    t["tokens"] = {"sessions": len(tok), "sum": sum(tok) if tok else None,
                   "median": statistics.median(tok) if tok else None}
    t["tool_uses_mean"] = round(statistics.mean(tu), 2) if tu else None
    t["by_cluster"] = {}
    for r in rows:
        c = t["by_cluster"].setdefault(r["cluster"], {"tasks": 0, "first_try": 0, "final": 0})
        c["tasks"] += 1
        c["first_try"] += r["first"] == "correct"
        c["final"] += r["final"] == "correct"
    return t


def verdict(t, base):
    """Önceden yazılı "belirgin altında" eşikleri (PLAN.md §6). Döner: tetiklenen satırlar."""
    hits = []
    n = t["tasks"]
    if n and t["final"] <= base["final"] * n / 20 - 2:
        hits.append(f"sonunda doğru {t['final']}/{n} (RX4 {base['final']}/20): ≥ 2 görev eksik")
    if n and t["first_try"] <= base["first_try"] * n / 20 - 3:
        hits.append(f"ilk denemede doğru {t['first_try']}/{n}: ≥ 3 görev eksik")
    if t["silent_wrong"] > 0:
        hits.append(f"sessiz yanlış {t['silent_wrong']} (RX4 0)")
    rn = t["reject_needed"]
    if rn["tasks"] and rn["final_ok"] < rn["tasks"]:
        hits.append(f"ret gereken yerde ret {rn['final_ok']}/{rn['tasks']} (RX4 4/4)")
    if t["false_rejects"] > 0:
        hits.append(f"yanlış ret {t['false_rejects']} (RX4 0)")
    if t["invented"]["tasks"] >= 2:
        hits.append(f"uydurma işlev adı {t['invented']['tasks']} görevde (RX4 dolaylı 0)")
    return hits


def cost_note(t, base):
    m = t["tokens"]["median"]
    if m and m > 1.5 * base["tokens_median"]:
        return f"belirteç medyanı {m:.0f} > 1,5 × RX4 {base['tokens_median']} (doğruluk değil, maliyet)"
    return None


# ------------------------------------------------------------------ rapor

def fmt_md(by_model, truth, root):
    L = ["# RL1 — Ajan yazabilirlik teyidi: sonuç", "",
         f"Üreten `score.py` (koşu kökü `{root}`; veri kümeleri {', '.join(truth.sets)}). "
         "Elle düzenlenmez; yorum REPORT.md'ye.", ""]
    models = list(by_model)
    hdr = "| Ölçüt | " + " | ".join(f"Qapsule {m} | RX4 A-{m}" for m in models) + " |"
    L += ["## Yan yana (RX4 kol A, aynı 20 görev)", "", hdr,
          "|---|" + "---:|---:|" * len(models)]

    def cell(m, f_q, f_b):
        t, b = by_model[m]["totals"], BASELINE.get(m, {})
        return f"{f_q(t)} | {f_b(b) if b else '—'}"
    lines = [
        ("İlk denemede doğru", lambda t: f"{t['first_try']}/{t['tasks']}", lambda b: f"{b['first_try']}/20"),
        ("Sonunda doğru (≤ 3 teslim)", lambda t: f"{t['final']}/{t['tasks']}", lambda b: f"{b['final']}/20"),
        ("Düzelme (ilki yanlış → sonu doğru)", lambda t: f"{t['m3']['recovered']}/{t['m3']['first_wrong']}", lambda b: b["m3"]),
        ("Hata iletisinden sonra doğru", lambda t: f"{t['after_error']['final_correct']}/{t['after_error']['tasks_with_error_reply']}", lambda b: b["after_error"]),
        ("Ret gereken yerde ret (son)", lambda t: f"{t['reject_needed']['final_ok']}/{t['reject_needed']['tasks']}", lambda b: f"{b['reject_final']}/4"),
        ("Ret gereken yerde ret (ilk)", lambda t: f"{t['reject_needed']['first_ok']}/{t['reject_needed']['tasks']}", lambda b: f"{b['reject_first']}/4"),
        ("Yanlış ret", lambda t: str(t["false_rejects"]), lambda b: str(b["false_reject"])),
        ("Uydurma işlev adı (görev)", lambda t: f"{t['invented']['tasks']} ({t['invented']['replies']} cevap)", lambda b: f"{b['invented']} (dolaylı)"),
        ("Sessiz yanlış (teslim)", lambda t: str(t["silent_wrong"]), lambda b: str(b["silent_wrong"])),
        ("Teslim öncesi hata alan oturum", lambda t: str(t["failed_check_sessions"]), lambda b: str(b["failed_check_sessions"])),
        ("Belirteç, görev başı medyan", lambda t: f"{t['tokens']['median']:.0f}" if t["tokens"]["median"] else "—", lambda b: str(b["tokens_median"])),
        ("Araç çağrısı, ortalama", lambda t: str(t["tool_uses_mean"] or "—"), lambda b: str(b["tools_mean"])),
    ]
    for name, fq, fb in lines:
        L.append(f"| {name} | " + " | ".join(cell(m, fq, fb) for m in models) + " |")
    L += ["", "## Hüküm (önceden yazılı eşikler, PLAN.md §6)", ""]
    for m in models:
        t = by_model[m]["totals"]
        hits = verdict(t, BASELINE.get(m, BASELINE["haiku"]))
        cn = cost_note(t, BASELINE.get(m, BASELINE["haiku"]))
        L.append(f"- **{m}:** " + ("belirgin altında — " + "; ".join(hits) + " → neden aranır (yüzey yeniden seçilmez)"
                                    if hits else "RX4 ile aynı bantta") + (f" · not: {cn}" if cn else ""))
    L += ["", "## Ek kayıtlar", ""]
    for m in models:
        t = by_model[m]["totals"]
        L.append(f"- **{m}:** teslim {t['submissions']} (metin {t['surface']['text']}, IR "
                 f"{t['surface']['ir']}); teslimsiz {t['no_submission']}; hata {t['errors']}; "
                 f"M5 {t['m5_seen_ok_heldout_wrong']}; check'siz teslim {t['submitted_without_check']}; "
                 f"öneri {t['proposals']['proposed']}/{t['proposals']['documentable_tasks']} "
                 f"(E_TRY_ONLY {t['proposals']['try_only_replies']}, add hatası {t['proposals']['add_errors']}); "
                 f"program aracı dışı çağrı {t['offsurface_calls']}; karar denemesi {t['decision_attempts']}; "
                 f"uydurma adlar {json.dumps(t['invented']['names'], ensure_ascii=False)}")
    L += ["", "## Görev tablosu", "",
          "| Görev | Küme | " + " | ".join(f"{m}: teslimler | {m}: hata ileti | {m}: belirteç" for m in models) + " |",
          "|---|---|" + "---|---:|---:|" * len(models)]
    tids = sorted({r["task"] for m in models for r in by_model[m]["rows"]})
    for tid in tids:
        cells = []
        for m in models:
            r = next((x for x in by_model[m]["rows"] if x["task"] == tid), None)
            if r is None:
                cells.append("koşulmadı | — | —")
                continue
            seq = " → ".join(f"{s['status']}{'(' + s['code'] + ')' if s.get('code') else ''}"
                             f"[{s['surface']}]" for s in r["submissions"]) or "teslimsiz"
            cells.append(f"{seq} | {r['error_replies_before_final']} | {r['tokens'] or '—'}")
        L.append(f"| {tid} | {truth.tasks[tid]['cluster']} | " + " | ".join(cells) + " |")
    L += ["", "## Yanlış teslimlerin ayrıntısı", ""]
    for m in models:
        for r in by_model[m]["rows"]:
            for s in r["submissions"]:
                if s["status"] != "correct":
                    det = s.get("sets") or {"kod": s.get("code")}
                    L.append(f"- {m} {r['task']} #{s['n']} `{s['status']}`: "
                             + "; ".join(f"{k}: {v}" for k, v in det.items()))
    return "\n".join(L) + "\n"


def score_root(root, models, truth):
    by_model = {}
    for m in models:
        rows = []
        for rd in sorted(Path(root).glob(f"{m}-G*-*")):
            if (rd / "meta.json").exists():
                rows.append(score_run(rd, truth))
        by_model[m] = {"rows": rows, "totals": totals(rows)}
    return by_model


def main():
    ap = argparse.ArgumentParser(description="RL1 ajan teyidi puanlayıcısı")
    ap.add_argument("root", type=Path)
    ap.add_argument("--models", default="haiku,opus")
    ap.add_argument("--sets", default="seen,heldout")
    ap.add_argument("--rx4", type=Path, default=DEFAULT_RX4)
    ap.add_argument("--out", type=Path, default=HERE / "RESULT.md")
    a = ap.parse_args()
    truth = Truth(a.rx4, [s for s in a.sets.split(",") if s])
    by_model = score_root(a.root, [m for m in a.models.split(",") if m], truth)
    a.out.write_text(fmt_md(by_model, truth, a.root), encoding="utf-8")
    a.out.with_suffix(".json").write_text(json.dumps(by_model, ensure_ascii=False, indent=1,
                                                     default=sorted) + "\n", encoding="utf-8")
    for m, v in by_model.items():
        t = v["totals"]
        print(f"{m}: görev {t['tasks']}, ilk {t['first_try']}, son {t['final']}, "
              f"ret {t['reject_needed']['final_ok']}/{t['reject_needed']['tasks']}, "
              f"uydurma {t['invented']['tasks']}, sessiz yanlış {t['silent_wrong']}")
    print("->", a.out)


if __name__ == "__main__":
    main()
