#!/usr/bin/env python3
"""RX4 uygunluk koşucusu (PROGRAMS §15.1; GOAL §3 "RX4 uygunluğu"; VERIFY 17).

    python3 -I conform.py --dry-run
    python3 -I conform.py --run "<şablon>" [--check "<şablon>"] [--sets seen,heldout]
                          [--only G1-01 ...] [--out RESULT.md] [--keep-raw DIR]

Şablon yer tutucuları (her argümanda ayrı ayrı doldurulur; kabuk kullanılmaz, cwd = depo):
  {program} çevrilmiş qp-ir/0 dosyası, {data} bar serisi dosyası, {id} görev, {set} veri kümesi.
Varsayılan önerilen şablonlar (CLI çıkınca kesinleşir):
  --run   "target/debug/qp program run --json --full --data {data} {program}"
  --check "target/debug/qp program check --json {program}"

Program görevleri (26): motor çıktısı RX4 biçimine çevrilir (`adapt_outputs`) ve
`expected/<id>/<küme>.json` ile RX4 `score.compare_outputs` kuralıyla karşılaştırılır (kural
burada yeniden yazıldı, RX4 kodu içe aktarılmaz): yalnız görevin çıktıları; seri
`|a−b| ≤ 1e-9·max(1,|b|)`, olay barları birebir, sinyal `(bar, side, outcome, exit_bar)`
birebir, durum anahtarları birebir. Ek olarak seri değerlerinde bit eşitlik sayılır.
Ret görevleri (4: G4-04, G5-*): `probes/<id>.reject` (kodlu ret) `--check` ile; motor onu
geçerli ret diye tanımalı ve kodu kabul kümesinde olmalı. `probes/<id>.bad` (yasak programın
kendisi) motorca reddedilmeli; alt kodu ayrıca yazılır.
`--dry-run`: yalnız çevirir, qp-ir/0 biçimini, görev/çıktı eşleşmesini, veri dosyasını ve
çevirici + karşılaştırıcının kendini (beklenen çıktı → motor biçimi → geri) sınar; motor koşmaz.
"""
import argparse
import json
import re
import shlex
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bars as barsmod  # noqa: E402
import translate as tr  # noqa: E402
from rx4common import (DATA_DIR, DEFAULT_RX4, HERE, PROGRAMS_DIR, REPO, SETS,  # noqa: E402
                       is_reject_task, load_json, load_tasks)

DEFAULT_RUN = "target/debug/qp program run --json --full --data {data} {program}"
DEFAULT_CHECK = "target/debug/qp program check --json {program}"
TIMEOUT = 120
TODO = "_(doldurulacak)_"


# ------------------------------------------------------------------ karşılaştırma kuralı

def num_close(a, b):
    """Seri değeri: ikisi de boş ya da `|a−b| ≤ 1e-9·max(1,|b|)`; bool sayı sayılmaz."""
    if a is None or b is None:
        return a is None and b is None
    if isinstance(a, bool) or isinstance(b, bool):
        return False
    return abs(a - b) <= 1e-9 * max(1.0, abs(b))


def bits_equal(a, b):
    if a is None or b is None:
        return a is None and b is None
    if isinstance(a, bool) or isinstance(b, bool):
        return False
    return struct.pack("<d", float(a)) == struct.pack("<d", float(b))


def compare_output(kind, got, exp):
    """(eşit mi, neden). `kind` RX4 adıyla (fingerprint)."""
    if got.get("kind") != kind:
        return False, f"tür {got.get('kind')!r} != {kind!r}"
    if kind == "series":
        g, e = got.get("values"), exp["values"]
        if not isinstance(g, list) or len(g) != len(e):
            return False, f"seri uzunluğu {len(g) if isinstance(g, list) else None} != {len(e)}"
        bad = [i for i, (x, y) in enumerate(zip(g, e)) if not num_close(x, y)]
        if bad:
            i = bad[0]
            return False, f"seri {len(bad)} barda farklı; ilk bar {i}: {g[i]!r} != {e[i]!r}"
        return True, ""
    if kind == "event":
        g, e = got.get("bars") or [], exp["bars"]
        if g != e:
            gs, es = set(g), set(e)
            return False, (f"olay {len(g)} vs {len(e)}; fazla {sorted(gs - es)[:5]}, "
                           f"eksik {sorted(es - gs)[:5]}")
        return True, ""
    if kind == "signal":
        def key(lst):
            return [(s.get("bar"), s.get("side"), s.get("outcome"), s.get("exit_bar")) for s in lst]
        g, e = key(got.get("signals") or []), key(exp["signals"])
        if g != e:
            d = next((i for i, (x, y) in enumerate(zip(g, e)) if x != y), min(len(g), len(e)))
            return False, (f"sinyal {len(g)} vs {len(e)}; ilk fark #{d}: "
                           f"{g[d] if d < len(g) else None} vs {e[d] if d < len(e) else None}")
        return True, ""
    if kind == "fingerprint":
        g, e = got.get("keys"), exp["keys"]
        if not isinstance(g, list) or len(g) != len(e):
            return False, "durum anahtarı uzunluğu farklı"
        bad = [i for i, (x, y) in enumerate(zip(g, e)) if x != y]
        if bad:
            i = bad[0]
            return False, f"durum {len(bad)} barda farklı; ilk bar {i}: {g[i]!r} != {e[i]!r}"
        return True, ""
    return False, f"bilinmeyen tür {kind}"


def compare_task(task, got, exp):
    """Görevin çıktıları; fazladan çıktı yok sayılır. Döner (eşit, neden, (bit eşit, toplam))."""
    outs = (got or {}).get("outputs")
    if not isinstance(outs, dict):
        return False, "çıktı yok", (0, 0)
    reasons, beq, btot = [], 0, 0
    for spec in task["outputs"]:
        name = spec["name"]
        if name not in outs:
            reasons.append(f"{name}: eksik çıktı")
            continue
        ok, why = compare_output(spec["kind"], outs[name], exp["outputs"][name])
        if not ok:
            reasons.append(f"{name}: {why}")
        if spec["kind"] == "series" and isinstance(outs[name].get("values"), list):
            e = exp["outputs"][name]["values"]
            g = outs[name]["values"]
            btot += len(e)
            beq += sum(1 for x, y in zip(g, e) if bits_equal(x, y))
    return not reasons, "; ".join(reasons), (beq, btot)


# ------------------------------------------------------------------ motor çıktısı → RX4 biçimi

def _find_outputs(obj, depth=0):
    if depth > 4 or not isinstance(obj, dict):
        return None
    o = obj.get("outputs")
    if isinstance(o, dict) or (isinstance(o, list) and all(isinstance(x, dict) for x in o)):
        return o
    for k in ("data", "result", "reply", "run"):
        r = _find_outputs(obj.get(k), depth + 1)
        if r is not None:
            return r
    return None


def _t_of(item):
    if isinstance(item, dict):
        for k in ("bar_t", "t", "born_t"):
            if k in item:
                return item[k]
        return None
    return item


def adapt_one(o, dims, tindex):
    """Bir motor çıktısını RX4 biçimine çevirir. Kabul edilen biçimler:
    seri `values`; olay `bars` (indeks) | `events`/`t` (zaman ya da {t}) | `values` (1/0/null);
    sinyal `signals`/`records`: `bar` | `born_t`/`t`, `outcome` dizgi | {kind, exit_t},
    `exit_bar` | `outcome.exit_t`; durum `keys`: null | [k…] | {boyut: k}."""
    kind = o.get("kind")
    if kind == "state":
        kind = "fingerprint"
    if kind == "series":
        return {"kind": "series", "values": o.get("values")}
    if kind == "event":
        if isinstance(o.get("bars"), list):
            return {"kind": "event", "bars": o["bars"]}
        for k in ("events", "t", "times"):
            if isinstance(o.get(k), list):
                return {"kind": "event", "bars": [tindex.get(_t_of(x), -1) for x in o[k]]}
        if isinstance(o.get("values"), list):
            return {"kind": "event", "bars": [i for i, v in enumerate(o["values"]) if v in (1, True)
                                              and v is not None]}
        return {"kind": "event", "bars": None}
    if kind == "signal":
        recs = o.get("signals") if isinstance(o.get("signals"), list) else o.get("records") or []
        out = []
        for s in recs:
            bar = s["bar"] if "bar" in s else tindex.get(_t_of(s), -1)
            oc = s.get("outcome")
            exit_bar = s.get("exit_bar")
            if isinstance(oc, dict):
                if "exit_bar" not in s:
                    et = oc.get("exit_t")
                    exit_bar = None if et is None else tindex.get(et, -1)
                oc = oc.get("kind")
            out.append({"bar": bar, "side": s.get("side"), "outcome": oc, "exit_bar": exit_bar})
        return {"kind": "signal", "signals": out}
    if kind == "fingerprint":
        keys = o.get("keys")
        if not isinstance(keys, list):
            return {"kind": "fingerprint", "keys": None}
        res = []
        for k in keys:
            if isinstance(k, dict):
                res.append(None if any(d not in k or k[d] is None for d in dims)
                           else [k[d] for d in dims])
            else:
                res.append(k)
        return {"kind": "fingerprint", "keys": res}
    return {"kind": kind}


def adapt_outputs(engine, program, tindex):
    raw = _find_outputs(engine)
    if raw is None:
        return None
    if isinstance(raw, list):
        raw = {o.get("name"): o for o in raw}
    dims = {o["name"]: [d["name"] for d in o.get("dims", [])] for o in program.get("outputs", [])}
    return {"outputs": {n: adapt_one(o, dims.get(n, []), tindex) for n, o in raw.items()
                        if isinstance(o, dict)}}


def reasons_in(obj):
    """Hata nesnelerindeki `E_` alt kodları (sırayla, tekrarsız): `reason` (Qapsule) ya da
    `code` (RX4 biçimi)."""
    found = []

    def walk(x):
        if isinstance(x, dict):
            for k in ("reason", "code"):
                v = x.get(k)
                if isinstance(v, str) and re.match(r"^E_[A-Z_]+$", v) and v not in found:
                    found.append(v)
            for k, v in x.items():
                if k != "reject":
                    walk(v)
        elif isinstance(x, list):
            for v in x:
                walk(v)
    walk(obj)
    return found


def reject_code_in(obj):
    if isinstance(obj, dict):
        r = obj.get("reject")
        if isinstance(r, dict) and isinstance(r.get("code"), str):
            return r["code"]
        for k in ("data", "result", "reply"):
            c = reject_code_in(obj.get(k))
            if c:
                return c
    return None


# ------------------------------------------------------------------ alt süreç

def run_cmd(template, **fill):
    argv = [tok.format(**fill) for tok in shlex.split(template)]
    try:
        r = subprocess.run(argv, cwd=REPO, capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        return None, "zaman aşımı", -1, ""
    except OSError as e:
        return None, f"çalıştırılamadı: {e}", -1, ""
    obj = None
    for stream in (r.stdout, r.stderr):
        s = stream.strip()
        if not s:
            continue
        for cand in (s, s.splitlines()[-1]):
            try:
                obj = json.loads(cand)
                break
            except json.JSONDecodeError:
                continue
        if obj is not None:
            break
    return obj, (r.stderr or "").strip()[:300], r.returncode, r.stdout


# ------------------------------------------------------------------ koşu

def program_rows(task, rec, sets, data, tindex, exp, a, raw_dir):
    rows = []
    for s in sets:
        row = {"task": task["id"], "cluster": task["cluster"], "set": s, "bits": None}
        if rec is None or rec["status"] != "ok":
            row.update(verdict="çevrilemedi",
                       detail=(rec or {}).get("reason", "çözüm dosyası yok"))
            rows.append(row)
            continue
        prog = PROGRAMS_DIR / rec["program"]
        obj, err, rc, stdout = run_cmd(a.run, program=str(prog), data=str(data[s]),
                                       id=task["id"], set=s)
        if raw_dir:
            (raw_dir / f"{task['id']}.{s}.out.json").write_text(stdout, encoding="utf-8")
        got = adapt_outputs(obj, load_json(prog), tindex[s]) if isinstance(obj, dict) else None
        if rc != 0 or got is None:
            codes = reasons_in(obj) if obj is not None else []
            row.update(verdict="fark", detail=f"motor koşmadı (çıkış {rc}): "
                       f"{', '.join(codes) or err or 'çıktı bulunamadı'}")
            rows.append(row)
            continue
        ok, why, bits = compare_task(task, got, exp[s])
        row.update(verdict="eşit" if ok else "fark", detail=why,
                   bits=bits if bits[1] else None)
        rows.append(row)
    return rows


def reject_row(task, recs, a):
    accept = task["accept"]["reject"]
    row = {"task": task["id"], "cluster": task["cluster"], "set": "—", "bits": None}
    notes, ok = [], True
    rj, bad = recs.get("reject"), recs.get("bad")
    if rj is None or rj["status"] != "ok" or bad is None or bad["status"] != "ok":
        row.update(verdict="çevrilemedi", detail="yoklama eksik ya da çevrilemedi")
        return row
    obj, err, rc, _ = run_cmd(a.check, program=str(PROGRAMS_DIR / rj["program"]),
                              data="", id=task["id"], set="")
    code = reject_code_in(obj)
    if code in accept and rc == 0:
        notes.append(f"ret tanındı: {code}")
    else:
        ok = False
        notes.append(f"ret tanınmadı (çıkış {rc}, kod {code!r}, {err or reasons_in(obj)})")
    obj, err, rc, _ = run_cmd(a.check, program=str(PROGRAMS_DIR / bad["program"]),
                              data="", id=task["id"], set="")
    codes = reasons_in(obj) if obj is not None else []
    hit = [c for c in codes if c in accept]
    if hit:
        notes.append(f"yasak program reddedildi: {hit[0]}")
    else:
        ok = False
        notes.append(f"yasak program kabul kümesiyle reddedilmedi (çıkış {rc}, {codes or err})")
    row.update(verdict="ret görevi: eşit" if ok else "ret görevi: fark",
               detail=f"kabul {accept}; " + "; ".join(notes))
    return row


def old_explanations(path):
    keep = {}
    if not path.exists():
        return keep
    for line in path.read_text(encoding="utf-8").splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) == 7 and re.match(r"^G\d-\d\d$", cells[0]):
            keep[(cells[0], cells[2])] = (cells[3], cells[6])
    return keep


def write_result(rows, a, out):
    old = old_explanations(out)
    def esc(s):
        return str(s).replace("|", "\\|").replace("\n", " ")
    counts = {}
    for r in rows:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
    bq = sum(r["bits"][0] for r in rows if r["bits"])
    bt = sum(r["bits"][1] for r in rows if r["bits"])
    L = ["# RL1 — RX4 uygunluk tablosu", "",
         "PROGRAMS §15.1; üreten `conform.py` (elle düzenlenen yalnız **Açıklama** sütunu; yeniden",
         "koşuda karar değişmemişse açıklama korunur). Her fark ya hata ya da §15'te yazılı sapmadır.",
         "", f"- Koşu şablonu: `{a.run}`", f"- Denetim şablonu: `{a.check}`",
         f"- Veri kümeleri: {', '.join(a.sets)}; UTC hizası, kapalı pencere, aralık `1h`",
         "- Karar: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())),
         f"- Seri değerlerinde bit eşitlik: {bq}/{bt}", "",
         "| Görev | Küme | Veri | Karar | Bit eşit (seri) | Ayrıntı | Açıklama |",
         "|---|---|---|---|---|---|---|"]
    for r in rows:
        bits = f"{r['bits'][0]}/{r['bits'][1]}" if r["bits"] else "—"
        prev = old.get((r["task"], r["set"]))
        if r["verdict"] in ("eşit", "ret görevi: eşit"):
            expl = prev[1] if prev and prev[0] == r["verdict"] and prev[1] != TODO else "—"
        else:
            expl = prev[1] if prev and prev[0] == r["verdict"] else TODO
        L.append(f"| {r['task']} | {r['cluster']} | {r['set']} | {r['verdict']} | {bits} | "
                 f"{esc(r['detail']) or '—'} | {expl} |")
    out.write_text("\n".join(L) + "\n", encoding="utf-8")
    (out.with_suffix(".json")).write_text(
        json.dumps({"run": a.run, "check": a.check, "sets": a.sets, "rows": rows},
                   ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return counts


# ------------------------------------------------------------------ kuru koşu

def to_engine_shape(exp_out, prog_out, times):
    """Beklenen RX4 çıktısını §6 tarzı (zamanlı) bir motor çıktısına çevirir: adaptörün sınaması."""
    k = exp_out["kind"]
    if k == "series":
        return {"kind": "series", "values": exp_out["values"]}
    if k == "event":
        return {"kind": "event", "events": [{"t": times[b]} for b in exp_out["bars"]]}
    if k == "signal":
        return {"kind": "signal", "signals": [
            {"born_t": times[s["bar"]], "side": s["side"],
             "outcome": {"kind": s["outcome"], **({} if s["exit_bar"] is None else
                                                   {"exit_t": times[s["exit_bar"]]})}}
            for s in exp_out["signals"]]}
    names = [d["name"] for d in prog_out["dims"]]
    return {"kind": "state", "keys": [None if key is None else dict(zip(names, key))
                                      for key in exp_out["keys"]]}


def dry_run(a, tasks, records):
    problems = []
    by_task = {}
    for r in records:
        by_task.setdefault(r["task"], {})[r["source_kind"]] = r
    print(tr.summary(records))
    seen_file = DATA_DIR / "seen.bars.json"
    series = barsmod.series_from_rx4(a.rx4, "seen")
    if not seen_file.exists():
        problems.append(f"{seen_file} yok (python3 -I bars.py)")
    else:
        disk = load_json(seen_file)
        if disk != series:
            problems.append(f"{seen_file} RX4 seen verisiyle aynı değil")
    times = barsmod.bar_times(series)
    tindex = {t: i for i, t in enumerate(times)}
    selftest = 0
    for t in tasks:
        recs = by_task.get(t["id"], {})
        if is_reject_task(t):
            for k in ("reject", "bad"):
                if recs.get(k, {}).get("status") != "ok":
                    problems.append(f"{t['id']}: {k} yoklaması yok ya da çevrilemedi")
            rj = recs.get("reject")
            if rj and rj["status"] == "ok":
                code = load_json(PROGRAMS_DIR / rj["program"])["reject"]["code"]
                if code not in t["accept"]["reject"]:
                    problems.append(f"{t['id']}: ret kodu {code} kabul kümesinde değil")
            continue
        rec = recs.get("solution")
        if not rec or rec["status"] != "ok":
            continue  # özet zaten yazdı
        prog = load_json(PROGRAMS_DIR / rec["program"])
        pouts = {o["name"]: o for o in prog["outputs"]}
        for spec in t["outputs"]:
            want = "state" if spec["kind"] == "fingerprint" else spec["kind"]
            if spec["name"] not in pouts or pouts[spec["name"]]["kind"] != want:
                problems.append(f"{t['id']}: çıktı {spec['name']} ({want}) programda yok")
        for s in a.sets:
            ep = Path(a.rx4) / "expected" / t["id"] / f"{s}.json"
            if not ep.exists():
                problems.append(f"{ep} yok")
        exp = load_json(Path(a.rx4) / "expected" / t["id"] / "seen.json")
        fake = {"data": {"kind": "program_run", "outputs": [
            dict(name=n, **to_engine_shape(exp["outputs"][n], pouts[n], times))
            for n in exp["outputs"] if n in pouts]}}
        ok, why, _ = compare_task(t, adapt_outputs(fake, prog, tindex), exp)
        ok2, why2, bits = compare_task(t, exp, exp)
        if ok and ok2 and bits[0] == bits[1]:
            selftest += 1
        else:
            problems.append(f"{t['id']}: öz sınama başarısız: {why or why2}")
    nprog = sum(1 for t in tasks if not is_reject_task(t))
    print(f"görev: {len(tasks)} ({nprog} program, {len(tasks) - nprog} ret); "
          f"adaptör + karşılaştırıcı öz sınaması: {selftest}/{nprog}")
    print(f"veri: {seen_file.relative_to(REPO)} ({len(times)} bar)")
    for p in problems:
        print(f"  SORUN: {p}")
    return not problems


def main():
    ap = argparse.ArgumentParser(description="RX4 uygunluk koşucusu")
    ap.add_argument("--run", default=None, help=f"motor koşu şablonu (öneri: {DEFAULT_RUN!r})")
    ap.add_argument("--check", default=None, help=f"ret yoklaması şablonu (öneri: {DEFAULT_CHECK!r})")
    ap.add_argument("--sets", default=",".join(SETS))
    ap.add_argument("--rx4", type=Path, default=DEFAULT_RX4)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--out", type=Path, default=HERE / "RESULT.md")
    ap.add_argument("--keep-raw", type=Path, help="motor çıktılarını buraya yaz (depo dışı)")
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()
    a.sets = [s for s in a.sets.split(",") if s]
    if set(a.sets) - set(SETS):
        sys.exit(f"bilinmeyen veri kümesi: {a.sets}")
    tasks = load_tasks(a.rx4)
    if a.only:
        tasks = [t for t in tasks if t["id"] in a.only]
    records = tr.translate_all(a.rx4, PROGRAMS_DIR)
    if a.dry_run:
        sys.exit(0 if dry_run(a, tasks, records) else 1)
    if not a.run:
        sys.exit("--run gerekli (ya da --dry-run)")
    a.check = a.check or a.run
    by_task = {}
    for r in records:
        by_task.setdefault(r["task"], {})[r["source_kind"]] = r
    if a.keep_raw:
        a.keep_raw.mkdir(parents=True, exist_ok=True)
    rows = []
    with tempfile.TemporaryDirectory(prefix="qp-rx4-") as td:
        data, tindex, exp_cache = {}, {}, {}
        for s in a.sets:
            series = barsmod.series_from_rx4(a.rx4, s)
            data[s] = (DATA_DIR / "seen.bars.json" if s == "seen"
                       else barsmod.write_series(series, Path(td) / f"{s}.bars.json"))
            tindex[s] = {t: i for i, t in enumerate(barsmod.bar_times(series))}
        for t in tasks:
            recs = by_task.get(t["id"], {})
            if is_reject_task(t):
                rows.append(reject_row(t, recs, a))
                continue
            exp_cache = {s: load_json(Path(a.rx4) / "expected" / t["id"] / f"{s}.json")
                         for s in a.sets}
            rows += program_rows(t, recs.get("solution"), a.sets, data, tindex, exp_cache, a,
                                 a.keep_raw)
    counts = write_result(rows, a, a.out)
    print(" ".join(f"{k}: {v}" for k, v in sorted(counts.items())), "->", a.out)


if __name__ == "__main__":
    main()
