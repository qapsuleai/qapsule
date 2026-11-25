#!/usr/bin/env python3
"""rx4-ir/0 → qp-ir/0 mekanik çevirici (PROGRAMS §15, §15.1). Yalnız standart kütüphane.

    python3 -I translate.py [--rx4 DIR] [--out DIR]

Girdi: `<rx4>/solutions/*.ir.json` (26 program görevi) ve bu dizindeki `probes/*.rx4.json`
(ret görevleri için elle yazılmış yoklamalar: `.reject` = kodlu ret nesnesi, `.bad` = motorun
reddetmesi gereken program). Çıktı: `programs/<ad>.qp.json` (tek satır) ve
`programs/translation.json` (her dosya için durum, yapılan değişiklikler, statik maliyet;
çevrilemeyenler `{id, reason}` kaydıyla, tahmin yok).

Çeviri kuralları (§15.1 "Mekanik çeviri"):
  1. `version`: `rx4-ir/0` → `qp-ir/0` (başka sürüm: çevrilemez).
  2. `tf: "1h"` (`input`, `atr`) silinir: RX4 tabanı sabit `1h`, Qapsule'de taban = bağlı
     serinin aralığı (§15/1). `4h`, `1d` kalır.
  3. Çıktı türü `fingerprint` → `state` (§5.4, §15/10); `dims` aynı biçim.
  4. Ret nesnesi `{"reject": {"code", "reason"}}` aynı biçim; kod §11.2 alt kodu olmalı
     (RX4 kodları alt kod `reason` olur, §15/6).
  5. `rx4-ir/0`'da olup `qp-ir/0`'da olmayan alan yok; çıkarsa çevrilemez kaydı.
Ardından qp-ir/0 biçim denetimi (§2, §4, §5; tip/saat denetimi motorun işi).
"""
import argparse
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from rx4common import DEFAULT_RX4, PROBES_DIR, PROGRAMS_DIR, dump_line, load_json  # noqa: E402

RX4_VERSION = "rx4-ir/0"
QP_VERSION = "qp-ir/0"
RX4_BASE_TF = "1h"

ID_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]{0,63}$")
NAME_RE = re.compile(r"^[a-z][a-z0-9_]{0,31}$")
TF_RE = re.compile(r"^[1-9][0-9]*[mhdwM]$")

# PROGRAMS §11.2 alt kodları (ret nesnesinin `code`'u bunlardan olmalı).
REASONS = {
    "E_PARSE", "E_VERSION", "E_UNKNOWN_OP", "E_UNKNOWN_FIELD", "E_TYPE", "E_MISSING_ARG",
    "E_UNKNOWN_TF", "E_CLOCK_MISMATCH", "E_OUTPUT_CLOCK", "E_NO_OUTPUT", "E_UNKNOWN_REF",
    "E_UNKNOWN_PARAM", "E_DUPLICATE_NAME", "E_BAD_PARAM", "E_PARAM_RANGE", "E_NO_OUTCOME_RULE",
    "E_BAD_RULE", "E_NOT_BAR_SERIES", "E_PROGRAM_SERIES", "E_TRY_ONLY", "E_EVENT_OPERAND",
    "E_WINDOW_LIMIT", "E_TOO_MANY_NODES", "E_TOO_LARGE", "E_COST_LIMIT", "E_BUDGET",
    "E_LOOKAHEAD", "E_UNSUPPORTED",
}

# qp-ir/0 düğümleri (§4): op → (başvuru alanları, tamsayı alanları, öbür zorunlu, isteğe bağlı)
_W = ("x",), ("period",), (), ()
OPS = {
    "input": ((), (), ("field",), ("tf", "partial")),
    "const": ((), (), ("value",), ()),
    "bar_index": ((), (), (), ()),
    "time": ((), (), (), ()),
    "sma": _W, "ema": _W, "rsi": _W, "highest": _W, "lowest": _W, "stdev": _W,
    "percentrank": _W,
    "macd": (("x",), ("fast", "slow", "signal"), (), ()),
    "bollinger": (("x",), ("period",), ("k",), ()),
    "atr": ((), ("period",), (), ("tf",)),
    "lag": (("x",), ("n",), (), ()),
    "streak": (("c",), (), (), ()), "bars_since": (("c",), (), (), ()),
    "once": (("c",), (), (), ()),
    "rearm": (("trigger", "reset"), (), (), ()),
    "cooldown": (("trigger",), ("bars",), (), ()),
    "defined": (("x",), (), (), ()),
    "align": (("x",), (), (), ()),
    "line": ((), (), ("a", "b"), ()),
    "bucket": (("x",), (), ("edges",), ()),
}
for _op in ("add", "sub", "mul", "div", "min", "max", "gt", "ge", "lt", "le", "eq", "ne",
            "and", "or", "cross_above", "cross_below"):
    OPS[_op] = (("a", "b"), (), (), ())
for _op in ("neg", "abs", "not"):
    OPS[_op] = (("x",), (), (), ())

MULTI_OUT = {"macd": ("macd", "signal", "histogram"),
             "bollinger": ("upper", "middle", "lower", "bandwidth")}
BAR_FIELDS = {"open", "high", "low", "close", "volume"}

# §4 bar başı ağırlıkları (`stdev`, `percentrank`: `period`).
W0 = {"input", "const", "bar_index", "time", "align", "line"}
W2 = {"sma", "ema", "rsi", "highest", "lowest", "atr"}
W3 = {"macd", "bollinger"}
DEFAULT_MAX_COST_PER_BAR = 666  # docs/PARAMETERS.md "RL1" program.max_cost_per_bar


class Untranslatable(Exception):
    pass


def _is_int(v):
    return isinstance(v, int) and not isinstance(v, bool)


def _is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


# ------------------------------------------------------------------ çeviri

def translate(doc, passthrough=False):
    """rx4-ir/0 belgesi → (qp-ir/0 belgesi, değişiklik listesi).

    `passthrough`: `.bad` yoklamaları için; bilinmeyen op/alan olduğu gibi geçer (motor
    reddetmeli), yalnız sürüm ve `tf`/`fingerprint` kuralları uygulanır.
    """
    if not isinstance(doc, dict):
        raise Untranslatable("kök bir JSON nesnesi değil")
    if "reject" in doc:
        rej = doc["reject"]
        if set(doc) != {"reject"} or not isinstance(rej, dict) or set(rej) != {"code", "reason"}:
            raise Untranslatable("ret nesnesi `{reject: {code, reason}}` biçiminde değil")
        if rej["code"] not in REASONS:
            raise Untranslatable(f"ret kodu {rej['code']!r} §11.2 alt kodu değil")
        return {"reject": {"code": rej["code"], "reason": rej["reason"]}}, ["ret: biçim aynı"]
    changes = []
    extra = set(doc) - {"version", "nodes", "outputs"}
    if extra:
        raise Untranslatable(f"rx4-ir/0 kökünde bilinmeyen alan: {sorted(extra)}")
    if doc.get("version") != RX4_VERSION:
        raise Untranslatable(f"sürüm {doc.get('version')!r} != {RX4_VERSION!r}")
    nodes = []
    for i, n in enumerate(doc.get("nodes") or []):
        if not isinstance(n, dict):
            raise Untranslatable(f"/nodes/{i} nesne değil")
        m = dict(n)
        op = m.get("op")
        if op not in OPS and not passthrough:
            raise Untranslatable(f"/nodes/{i}: op {op!r} qp-ir/0'da yok")
        if op in ("input", "atr") and m.get("tf") == RX4_BASE_TF:
            del m["tf"]
            changes.append(f"/nodes/{i}: tf \"1h\" silindi (taban)")
        if op in OPS and not passthrough:
            refs, ints, req, opt = OPS[op]
            unknown = set(m) - {"id", "op"} - set(refs) - set(ints) - set(req) - set(opt)
            if unknown:
                raise Untranslatable(f"/nodes/{i} ({op}): qp-ir/0'da olmayan alan {sorted(unknown)}")
        nodes.append(m)
    outputs = []
    for i, o in enumerate(doc.get("outputs") or []):
        if not isinstance(o, dict):
            raise Untranslatable(f"/outputs/{i} nesne değil")
        m = dict(o)
        if m.get("kind") == "fingerprint":
            m["kind"] = "state"
            changes.append(f"/outputs/{i}: kind fingerprint → state")
        if m.get("kind") == "signal" and isinstance(m.get("rule"), dict) and "kind" in m["rule"]:
            raise Untranslatable(f"/outputs/{i}/rule/kind: qp-ir/0'da yok (§6)")
        outputs.append(m)
    changes.insert(0, f"version {RX4_VERSION} → {QP_VERSION}")
    return {"version": QP_VERSION, "nodes": nodes, "outputs": outputs}, changes


# ------------------------------------------------------------------ qp-ir/0 biçim denetimi

def validate(doc):
    """qp-ir/0 biçim denetimi (§2, §4, §5). Döner: sorun listesi (boşsa geçerli)."""
    errs = []
    if "reject" in doc:
        return [] if doc["reject"].get("code") in REASONS else ["ret kodu bilinmiyor"]
    if doc.get("version") != QP_VERSION:
        errs.append("/version: qp-ir/0 değil")
    extra = set(doc) - {"version", "params", "nodes", "outputs"}
    if extra:
        errs.append(f"/: bilinmeyen alan {sorted(extra)}")
    seen = {}
    for i, n in enumerate(doc.get("nodes") or []):
        at = f"/nodes/{i}"
        nid, op = n.get("id"), n.get("op")
        if not isinstance(nid, str) or not ID_RE.match(nid):
            errs.append(f"{at}/id: desen dışı {nid!r}")
        elif nid in seen:
            errs.append(f"{at}/id: tekrar {nid!r}")
        if op not in OPS:
            errs.append(f"{at}/op: bilinmeyen {op!r}")
            seen[nid] = op
            continue
        refs, ints, req, opt = OPS[op]
        for f in set(n) - {"id", "op"} - set(refs) - set(ints) - set(req) - set(opt):
            errs.append(f"{at}/{f}: bilinmeyen alan")
        for f in refs + ints + req:
            if f not in n:
                errs.append(f"{at}/{f}: eksik")
        for f in refs:
            r = n.get(f)
            if not isinstance(r, str):
                errs.append(f"{at}/{f}: başvuru dizgi değil")
                continue
            base, _, sub = r.partition(".")
            if base not in seen:
                errs.append(f"{at}/{f}: tanımsız ya da sonraki düğüm {r!r}")
            elif sub and sub not in MULTI_OUT.get(seen[base], ()):
                errs.append(f"{at}/{f}: olmayan çıktı {r!r}")
        for f in ints:
            if f in n and not (_is_int(n[f]) or isinstance(n[f], dict)):
                errs.append(f"{at}/{f}: tamsayı değil {n[f]!r}")
        if op == "input" and n.get("field") not in BAR_FIELDS:
            errs.append(f"{at}/field: bilinmeyen {n.get('field')!r}")
        if "tf" in n and not (isinstance(n["tf"], str) and TF_RE.match(n["tf"])):
            errs.append(f"{at}/tf: aralık yazımı değil {n['tf']!r}")
        if op == "const" and not (_is_num(n.get("value")) or isinstance(n.get("value"), (bool, dict))):
            errs.append(f"{at}/value: sayı ya da bool değil")
        if op == "line":
            for p in ("a", "b"):
                pt = n.get(p)
                if not (isinstance(pt, dict) and set(pt) == {"t", "price"}):
                    errs.append(f"{at}/{p}: {{t, price}} değil")
        if op == "bucket":
            e = n.get("edges")
            if not (isinstance(e, list) and 1 <= len(e) <= 8 and all(_is_num(v) for v in e)
                    and all(a < b for a, b in zip(e, e[1:]))):
                errs.append(f"{at}/edges: 1…8 elemanlı kesin artan sayı listesi değil")
        seen[nid] = op
    names = set()
    outs = doc.get("outputs") or []
    if not outs:
        errs.append("/outputs: boş")
    for i, o in enumerate(outs):
        at = f"/outputs/{i}"
        name, kind = o.get("name"), o.get("kind")
        if not isinstance(name, str) or not NAME_RE.match(name):
            errs.append(f"{at}/name: desen dışı {name!r}")
        elif name in names:
            errs.append(f"{at}/name: tekrar {name!r}")
        names.add(name)
        allowed = {"series": {"node"}, "event": {"node"},
                   "signal": {"node", "side", "rule", "source"}, "state": {"dims"}}.get(kind)
        if allowed is None:
            errs.append(f"{at}/kind: bilinmeyen {kind!r}")
            continue
        for f in set(o) - {"name", "kind"} - allowed:
            errs.append(f"{at}/{f}: bilinmeyen alan")
        for f in allowed - set(o):
            errs.append(f"{at}/{f}: eksik")
        if "node" in o and o["node"].partition(".")[0] not in seen:
            errs.append(f"{at}/node: tanımsız {o['node']!r}")
        if kind == "signal":
            if o.get("side") not in ("al", "sat"):
                errs.append(f"{at}/side: al|sat değil")
            rule = o.get("rule") or {}
            if set(rule) - {"target_pct", "stop_pct", "max_bars"} or "max_bars" not in rule:
                errs.append(f"{at}/rule: {{target_pct?, stop_pct?, max_bars}} değil")
            src = o.get("source") or {}
            if set(src) != {"strategy", "version"} or not all(isinstance(v, str) for v in src.values()):
                errs.append(f"{at}/source: {{strategy, version}} dizgi değil")
        if kind == "state":
            dims = o.get("dims")
            if not (isinstance(dims, list) and 1 <= len(dims) <= 4):
                errs.append(f"{at}/dims: 1…4 boyut değil")
            else:
                for j, d in enumerate(dims):
                    if set(d) != {"name", "node"} or d["node"].partition(".")[0] not in seen:
                        errs.append(f"{at}/dims/{j}: {{name, node}} değil ya da tanımsız düğüm")
    return errs


def static_cost(doc):
    """§8.1 `per_bar = Σ w + çıktı sayısı` (bilgi; parametresiz programlar için)."""
    total = 0
    for n in doc.get("nodes") or []:
        op = n.get("op")
        if op in W0:
            w = 0
        elif op in W2:
            w = 2
        elif op in W3:
            w = 3
        elif op in ("stdev", "percentrank"):
            w = n.get("period") if _is_int(n.get("period")) else 0
        else:
            w = 1
        total += w
    return total + len(doc.get("outputs") or [])


# ------------------------------------------------------------------ toplu çeviri

def sources(rx4):
    """(ad, görev kimliği, tür, yol): tür ∈ solution | reject | bad."""
    out = []
    for p in sorted((Path(rx4) / "solutions").glob("*.ir.json")):
        tid = p.name[: -len(".ir.json")]
        out.append((tid, tid, "solution", p))
    for p in sorted(PROBES_DIR.glob("*.rx4.json")):
        stem = p.name[: -len(".rx4.json")]  # G5-01.reject / G5-01.bad
        tid, _, kind = stem.partition(".")
        out.append((stem, tid, kind, p))
    return out


def translate_all(rx4, out_dir, write=True):
    records = []
    for name, tid, kind, path in sources(rx4):
        rec = {"name": name, "task": tid, "source_kind": kind, "source": str(path.name)}
        try:
            doc, changes = translate(load_json(path), passthrough=(kind == "bad"))
        except (Untranslatable, ValueError) as e:
            rec.update(status="untranslatable", reason=str(e))
            records.append(rec)
            continue
        problems = [] if kind == "bad" else validate(doc)
        rec["changes"] = changes
        if problems:
            rec.update(status="invalid", reason="; ".join(problems))
        else:
            rec["status"] = "ok"
        if "nodes" in doc and kind != "bad":
            rec["per_bar"] = static_cost(doc)
            rec["over_cost_limit"] = rec["per_bar"] > DEFAULT_MAX_COST_PER_BAR
        rec["program"] = f"{name}.qp.json"
        if write:
            out_dir.mkdir(parents=True, exist_ok=True)
            (out_dir / rec["program"]).write_text(dump_line(doc) + "\n", encoding="utf-8")
        records.append(rec)
    if write:
        (out_dir / "translation.json").write_text(
            "[\n" + ",\n".join(dump_line(r) for r in records) + "\n]\n", encoding="utf-8")
    return records


def summary(records):
    sol = [r for r in records if r["source_kind"] == "solution"]
    ok = [r for r in sol if r["status"] == "ok"]
    probes = [r for r in records if r["source_kind"] != "solution"]
    lines = [f"çözüm: {len(ok)}/{len(sol)} çevrildi ve biçim denetiminden geçti; "
             f"yoklama: {sum(r['status'] == 'ok' for r in probes)}/{len(probes)}"]
    for r in records:
        if r["status"] != "ok":
            lines.append(f"  {r['name']}: {r['status']}: {r['reason']}")
        elif r.get("over_cost_limit"):
            lines.append(f"  {r['name']}: per_bar {r['per_bar']} > {DEFAULT_MAX_COST_PER_BAR} (§15/7)")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description="rx4-ir/0 → qp-ir/0 çevirici")
    ap.add_argument("--rx4", type=Path, default=DEFAULT_RX4, help="RX4 harness dizini")
    ap.add_argument("--out", type=Path, default=PROGRAMS_DIR)
    a = ap.parse_args()
    print(summary(translate_all(a.rx4, a.out)))


if __name__ == "__main__":
    main()
