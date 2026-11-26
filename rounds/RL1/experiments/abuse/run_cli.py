#!/usr/bin/env python3
"""RL1/U3 — komut satırı saldırıları (`qp program check|run|explain`): statik maliyet, çalışma
bütçesi, geleceğe bakma, süreç güvenliği. Sonuç: results-cli.json (bu klasörde).

  python3 run_cli.py            (QP_BIN, ABUSE_WORK isteğe bağlı; bkz. qpabuse.py)
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from qpabuse import HERE, WORK, cli, ir, reasons, series_file, write  # noqa: E402

INPUTS = HERE / "inputs"
R = []


def rec(cls, aid, inp, expected, observed, ok, r=None):
    R.append({"class": cls, "id": aid, "input": inp, "expected": expected, "observed": observed,
              "verdict": "OK" if ok else "FAIL",
              "ms": r and r.get("ms"), "rss_mb": r and r.get("rss_mb")})
    print(f"[{'OK' if ok else 'FAIL'}] {aid}: {observed}"[:300], flush=True)


def crashed(r):
    """A panic, a signal or a hang (exit codes 0/1/2 are answers, PS §3.3)."""
    return r["exit"] not in (0, 1, 2) or "panicked" in r["err"] or "overflow" in r["err"]


def check(aid, cls, inp, path, want, expect_ok=False, extra=None):
    r = cli(["program", "check", "--json", path])
    rs = reasons(r["json"])
    if expect_ok:
        ok = r["exit"] == 0 and r["json"] and r["json"].get("ok") and not crashed(r)
    else:
        ok = r["exit"] == 1 and want in rs and not crashed(r)
    obs = f"exit {r['exit']}, reasons {rs[:4]}"
    if r["json"] and r["json"].get("cost"):
        obs += f", cost {r['json']['cost'].get('per_bar')}"
    if r["json"] and r["json"].get("truncated"):
        obs += ", truncated"
    if crashed(r):
        obs += f", CRASH {r['err'][:200]}"
    if extra:
        ok, obs = extra(r, ok, obs)
    rec(cls, aid, inp, "ok" if expect_ok else want, obs, ok, r)
    return r


C = {"id": "c", "op": "input", "field": "close"}


def chain(n, idlen=2):
    """`n` nodes: input then abs chain."""
    nodes = [dict(C, id="a0")]
    for i in range(1, n):
        nodes.append({"id": f"a{i}".ljust(idlen, "x") if idlen > 2 else f"a{i}", "op": "abs",
                      "x": nodes[-1]["id"]})
    return nodes


def static():
    S = "1-statik"
    n = chain(200)
    check("S01", S, "200 düğüm (sınır), abs zinciri", write("s01.json", ir(n, [{"name": "y", "kind": "series", "node": n[-1]["id"]}])), "", True)
    n = chain(201)
    check("S02", S, "201 düğüm", write("s02.json", ir(n, [{"name": "y", "kind": "series", "node": n[-1]["id"]}])), "E_TOO_MANY_NODES")
    check("S03", S, "stdev(close,665): per_bar tam 666 = sınır", INPUTS / "s03-stdev665.qp", "", True)
    check("S04", S, "stdev(close,666): per_bar 667 = sınır+1", INPUTS / "s04-stdev666.qp", "E_COST_LIMIT")
    check("S05", S, "percentrank(close,666)", write("s05.qp", "series p = percentrank(close, 666)\n"), "E_COST_LIMIT")
    check("S06", S, "stdev(close,2000) (azami pencere)", write("s06.qp", "series s = stdev(close, 2000)\n"), "E_COST_LIMIT")
    check("S07", S, "sma(close,2001) (pencere+1)", write("s07.qp", "series s = sma(close, 2001)\n"), "E_WINDOW_LIMIT")
    check("S08", S, "üst periyot indirimi yok: 2×stdev(close@1d,333)",
          write("s08.qp", "a = align(stdev(close@1d, 333))\nb = align(stdev(close@4h, 333))\nseries s = a + b\n"), "E_COST_LIMIT")
    outs = "".join(f"series o{i} = close\n" for i in range(5))
    check("S09", S, "5 çıktı (sınır 4)", write("s09.qp", outs), "E_TOO_LARGE")
    # IR over 8192 canonical bytes within 200 nodes: long ids.
    n = chain(150, idlen=60)
    check("S10", S, "kanonik ara gösterim > 8192 bayt (150 düğüm, 60 karakterlik kimlik)",
          write("s10.json", ir(n, [{"name": "y", "kind": "series", "node": n[-1]["id"]}])), "E_TOO_LARGE")
    big = WORK / "s11-50mb.json"
    with open(big, "w") as f:
        f.write('{"version":"qp-ir/0","nodes":[' + ",".join(['{"id":"c","op":"input","field":"close"}'] * 1_300_000) + "]}")
    r = check("S11", S, "50 MB ara gösterim dosyası", big, "E_TOO_LARGE",
              extra=lambda r, ok, obs: (ok and (r["rss_mb"] or 0) < 200, obs + f", rss {r['rss_mb']} MB"))
    big.unlink()
    check("S12", S, "200 000 derinlikte JSON `[`", write("s12.json", "[" * 200_000), "E_PARSE")
    check("S13", S, "60 000 iç içe parantez (metin)", write("s13.qp", "series s = " + "(" * 60000 + "close" + ")" * 60000 + "\n"), "E_TOO_LARGE")
    check("S14", S, "7000 terimli `close + close + …` (sol derin ağaç)",
          write("s14.qp", "x = " + " + ".join(["close"] * 7000) + "\nseries s = x\n"), "E_TOO_LARGE")
    check("S15", S, "60 000 tekli eksi `----…close`", write("s15.qp", "x = " + "-" * 60000 + "close\nseries s = x\n"), "E_TOO_LARGE")
    check("S16", S, "15 000 `not` zinciri", write("s16.qp", "x = " + "not " * 15000 + "true\nevent s = x\n"), "E_TOO_LARGE")
    check("S17", S, "20 000 `[1]` gecikme zinciri", write("s17.qp", "x = close" + "[1]" * 20000 + "\nseries s = x\n"), "E_TOO_LARGE")
    check("S18", S, "5000 karakterlik ad (metin)", write("s18.qp", "a" * 5000 + " = close\nseries s = " + "a" * 5000 + "\n"), "E_TOO_LARGE")
    n = [dict(C, id="a" * 65)]
    check("S19", S, "65 karakterlik düğüm kimliği (ara gösterim; desen ≤ 64)",
          write("s19.json", ir(n, [{"name": "y", "kind": "series", "node": "a" * 65}])), "E_BAD_PARAM",
          extra=lambda r, ok, obs: (r["exit"] == 1 and bool(reasons(r["json"])) and not crashed(r), obs))
    check("S20", S, "Unicode/sıfır genişlik/sağdan-sola ad: `çizgi`, `a\\u200bb`, `\\u202eevil`",
          INPUTS / "s20-unicode.qp", "E_PARSE")
    n = [dict(C, id="c"), {"id": "s", "op": "sma", "x": "c", "period": 3}]
    check("S21", S, "çıktı adı emoji `📈`", write("s21.json", ir(n, [{"name": "📈", "kind": "series", "node": "s"}])), "",
          extra=lambda r, ok, obs: (r["exit"] == 1 and bool(reasons(r["json"])) and not crashed(r), obs))
    params = [{"key": f"p{i}", "kind": "int", "min": 1, "max": 10, "default": 2} for i in range(17)]
    check("S22", S, "17 parametre (sınır 16)", write("s22.json", ir([dict(C)], [{"name": "y", "kind": "series", "node": "c"}], params)), "E_TOO_LARGE")
    check("S23", S, "`param p int = 10 in 1..1000000000000000000`, sma(close,p)",
          write("s23.qp", "param p int = 10 in 1..1000000000000000000\nseries s = sma(close, p)\n"), "E_PARAM_RANGE")
    check("S24", S, "parametreyle gizli maliyet: `param p int = 10 in 1..2000`, stdev(close,p)",
          write("s24.qp", "param p int = 10 in 1..2000\nseries s = stdev(close, p)\n"), "", True,
          extra=lambda r, ok, obs: (ok, obs + " (statik maliyet bağlanan değerle; S2x ve oturum B-sınıfı yeniden denetler)"))
    check("S25", S, "S24 + bağlanan değer p=2000 (ara gösterim + --params yok → CLI varsayılanı)",
          write("s25.qp", "param p int = 2000 in 1..2000\nseries s = stdev(close, p)\n"), "E_COST_LIMIT")
    n = [{"id": "a", "op": "abs", "x": "b"}, dict(C, id="b")]
    check("S26", S, "sonraki düğüme başvuru (döngü denemesi)", write("s26.json", ir(n, [{"name": "y", "kind": "series", "node": "a"}])), "E_UNKNOWN_REF")
    n = [{"id": "a", "op": "abs", "x": "a"}]
    check("S27", S, "kendine başvuru", write("s27.json", ir(n, [{"name": "y", "kind": "series", "node": "a"}])), "E_UNKNOWN_REF")
    n = [dict(C)] + [{"id": f"z{i}", "op": "rsii", "x": "c"} for i in range(199)]
    check("S28", S, "199 bilinmeyen düğüm (hata seli)", write("s28.json", ir(n, [{"name": "y", "kind": "series", "node": "z0"}])), "E_UNKNOWN_OP",
          extra=lambda r, ok, obs: (ok and len(r["json"].get("errors", [])) <= 50, obs + f", {len(r['json'].get('errors', []))} hata"))
    check("S29", S, "sayı taşması `1e400` sabit", write("s29.json", '{"version":"qp-ir/0","nodes":[{"id":"k","op":"const","value":1e400}],"outputs":[{"name":"y","kind":"series","node":"k"}]}'), "",
          extra=lambda r, ok, obs: (r["exit"] in (1, 2) and not crashed(r), obs))
    check("S30", S, "metin 70 000 karakter (sınır 65 536) yorum satırları", write("s30.qp", "#" + "x" * 70000 + "\nseries s = close\n"), "E_TOO_LARGE")
    # Fibonacci-shaped DAG: each node reads the two before it (a naive cone walk is 2^n).
    import string
    ids = list(string.ascii_letters) + [a + b for a in string.ascii_letters for b in string.ascii_letters]
    n = [{"id": ids[0], "op": "input", "field": "close"}, {"id": ids[1], "op": "input", "field": "open"}]
    n += [{"id": ids[i], "op": "add", "a": ids[i - 1], "b": ids[i - 2]} for i in range(2, 185)]
    dag = write("s32-dag.json", ir(n, [{"name": f"o{k}", "kind": "series", "node": ids[184 - k]} for k in range(4)]))
    r = cli(["program", "run", "--json", "--data", series_file(25000), dag], timeout=120)
    j = r["json"] or {}
    rec(S, "S32", "185 düğümlü Fibonacci DAG (her düğüm önceki ikisini okur; saf koni yürüyüşü 2^n), 4 çıktı, 20 000 bar",
        "doğrusal maliyet; hızlı biter", f"exit {r['exit']}, cost {j.get('cost', {}).get('per_bar')}/{j.get('cost', {}).get('live_per_bar')}, {r['ms']} ms",
        r["exit"] == 0 and r["ms"] < 30000 and not crashed(r), r)
    for kind in ("explain", "run"):
        args = ["program", kind, "--json"] + (["--data", series_file(500)] if kind == "run" else []) + [INPUTS / "s04-stdev666.qp"]
        r = cli(args)
        rec(S, f"S31-{kind}", f"`qp program {kind}` sınır+1 programı (S04)", "çalışmadan ret (çıkış 1)",
            f"exit {r['exit']}, reasons {reasons(r['json'])[:3]}", r["exit"] == 1 and not crashed(r), r)


def runtime():
    S = "2-çalışma"
    big = series_file(25000)
    r = cli(["program", "run", "--json", "--data", big, INPUTS / "r01-signal-flood.qp"], timeout=600)
    j = r["json"] or {}
    st = j.get("stopped") or {}
    rec(S, "R01", "4 sinyal çıktısı her barda, max_bars 500, + stdev 600 maliyeti, 25 000 bar",
        "çalışma bütçesiyle durur: stopped{reason: budget, bar, t, work, limit}",
        f"exit {r['exit']}, stopped {st}, work {j.get('work')}, from_t {j.get('from_t')}",
        st.get("reason") == "budget" and st.get("work", 0) > 0 and not crashed(r), r)
    # From the last program.try.max_bars bars only.
    from qpabuse import bars as mk
    b = mk(25000)
    rec(S, "R02", "25 000 barlık dosya, okuma penceresi", f"son 20 000 bar: from_t = {b[5000]['t']}",
        f"from_t {j.get('from_t')}", j.get("from_t") == b[5000]["t"], r)
    r = cli(["program", "run", "--json", "--full", "--data", big, INPUTS / "r01-signal-flood.qp"], timeout=600)
    j = r["json"] or {}
    st = j.get("stopped") or {}
    recs = [x for o in (j.get("outputs") or {}).values() for x in (o.get("records") or [])]
    late = [x for x in recs if st and x.get("born_t", 0) > st.get("t", 1 << 62)]
    rec(S, "R03", "R01 `--full`: durduktan sonra kayıt doğuyor mu", "durma barından sonra kayıt yok",
        f"stopped.t {st.get('t')}, kayıt {len(recs)}, sonra doğan {len(late)}", st.get("reason") == "budget" and not late, r)
    r = cli(["program", "run", "--json", "--data", big, INPUTS / "r04-state-keys.qp"], timeout=600)
    j = r["json"] or {}
    rec(S, "R04", "4 boyutlu durum tanımı, 8'er kenar (≤ 9^4 anahtar)", "biter ya da durur; çökme yok",
        f"exit {r['exit']}, state {str(j.get('state'))[:120]}, stopped {j.get('stopped')}",
        r["exit"] == 0 and not crashed(r), r)
    r = cli(["program", "run", "--json", "--data", big, INPUTS / "r05-event-every-bar.qp"], timeout=600)
    j = r["json"] or {}
    ev = (j.get("event") or [{}])[0]
    rec(S, "R05", "her barda olay (const true), 20 000 bar", "W_CONSECUTIVE_EVENTS; events 20000, runs 1",
        f"events {ev.get('events')}, runs {ev.get('runs')}, work {j.get('work')}",
        ev.get("events") == 20000 and ev.get("runs") == 1, r)
    bad = series_file(10, name="bad-nan.json")
    s = json.loads(bad.read_text())
    s["data"]["bars"][3]["c"] = 1e308 * 10 if False else 1.7976931348623157e308
    s["data"]["bars"][5]["t"] = s["data"]["bars"][4]["t"]
    bad.write_text(json.dumps(s))
    r = cli(["program", "run", "--json", "--data", bad, INPUTS / "s03-stdev665.qp"])
    rec("6-süreç", "R06", "veri dosyası: f64::MAX kapanış + yinelenen t", "kodlu ret ya da sonlu cevap; çökme yok",
        f"exit {r['exit']}, {r['err'][:120] or r['out'][:120]}", not crashed(r), r)


def full(prog, data):
    r = cli(["program", "run", "--json", "--full", "--data", data, prog], timeout=600)
    return r, (r["json"] or {}).get("outputs") or {}


def prefix(aid, desc, prog, n=1000, ks=(1, 7, 24, 61)):
    """Closed-window prefix stability (PR §10): run on n bars vs n+k bars, first n agree."""
    base_r, base = full(prog, series_file(n + max(ks), name=f"pre-{n + max(ks)}.json"))
    diffs = []
    for k in ks:
        m = n + max(ks) - k  # shorter prefixes of the long file
        _, short = full(prog, series_file(m, name=f"pre-{m}.json"))
        for name, o in base.items():
            so = short.get(name) or {}
            if "values" in o:
                a, b = o["values"][:m], so.get("values", [])[:m]
                if a != b:
                    i = next(i for i, (x, y) in enumerate(zip(a, b)) if x != y) if len(a) == len(b) else -1
                    diffs.append(f"{name}@{m}:bar{i}")
            if "bars" in o:
                a = [i for i in o["bars"] if i < m]
                if a != so.get("bars"):
                    diffs.append(f"{name}@{m}:events")
            if "keys" in o and o["keys"][:m] != so.get("keys", [])[:m]:
                diffs.append(f"{name}@{m}:keys")
            if "records" in o:
                closed = {(x["bar"], x["outcome"]["kind"], x.get("exit_bar")) for x in so.get("records", [])
                          if x["outcome"]["kind"] != "open"}
                full_ = {(x["bar"], x["outcome"]["kind"], x.get("exit_bar")) for x in o["records"]
                         if x["bar"] < m}
                if not closed <= full_:
                    diffs.append(f"{name}@{m}:records")
    rec("3-gelecek", aid, desc, "ilk n bar n+k koşusuyla bayt eşit (sinyal: kapanmış sonuçlar alt küme)",
        f"çıktılar {sorted(base)}; fark {diffs[:5] or 'yok'}", bool(base) and not diffs, base_r)


def lookahead():
    S = "3-gelecek"
    check("L01", S, "lag -1 (ara gösterim)", INPUTS / "l01-lag-neg.json", "E_LOOKAHEAD")
    check("L02", S, "`partial: true` üst periyot girdisi", INPUTS / "l02-partial.json", "E_LOOKAHEAD")
    check("L03", S, "metinde `close[-1]`", write("l03.qp", "series s = close[-1]\n"), "E_LOOKAHEAD",
          extra=lambda r, ok, obs: (r["exit"] == 1 and bool(reasons(r["json"])) and not crashed(r), obs))
    for aid, tf, what in (("L04", "1h", "taban = üst periyot"), ("L05", "30m", "tabandan küçük")):
        r = cli(["program", "run", "--json", "--data", series_file(500), write(f"{aid}.qp", f"series s = align(close@{tf})\n")])
        rec(S, aid, f"`close@{tf}` 1h tabanlı veriyle `run` ({what}; `check` bağsız saati denetlemez, PS §3.2)", "E_UNKNOWN_TF, çalışmaz",
            f"exit {r['exit']}, reasons {reasons(r['json'])}", r["exit"] == 1 and "E_UNKNOWN_TF" in reasons(r["json"]), r)
    check("L06", S, "`align(close)` (tabanın hizası)", write("l06.qp", "series s = align(close)\n"), "",
          extra=lambda r, ok, obs: (r["exit"] in (0, 1) and not crashed(r), obs))
    check("L07", S, "`reject E_LOOKAHEAD` metni", write("l07.qp", 'reject E_LOOKAHEAD "next close unknown"\n'), "", True,
          extra=lambda r, ok, obs: (ok and (r["json"].get("reject") or {}).get("code") == "E_LOOKAHEAD", obs + f", reject {r['json'].get('reject')}"))
    r = cli(["program", "run", "--json", "--data", series_file(500), write("l08.qp", 'reject E_LOOKAHEAD "x"\n')])
    rec(S, "L08", "`qp program run` ret dosyası", "çalışacak program yok (çıkış 1)", f"exit {r['exit']}", r["exit"] == 1, r)
    prefix("L09", "üst periyot: align(sma(close@1d,3)), align(close@4h), olay", INPUTS / "l09-htf.qp")
    prefix("L10", "ema/rsi/percentrank/stdev/bollinger/macd", INPUTS / "l10-kernels.qp")
    prefix("L11", "line: çapalar verinin ötesinde (gelecek t)", INPUTS / "l11-line-future.qp")
    prefix("L12", "time/bar_index hileleri + bars_since/cooldown/rearm/streak", INPUTS / "l12-time-index.qp")
    prefix("L13", "sinyal (max_bars 48) + durum tanımı", INPUTS / "l13-signal-state.qp")
    # Higher-tf value equals the last *closed* day's close (independent check, UTC days).
    r, out = full(INPUTS / "l14-align-day.qp", series_file(500))
    from qpabuse import bars as mk
    b = mk(500)
    day = 86_400_000
    days = {}
    for x in b:
        days.setdefault(x["t"] // day, []).append(x)
    want = []
    for x in b:
        close_i = x["t"] + 3_600_000
        done = [d for d in days if (d + 1) * day <= close_i and d * day >= b[0]["t"]]
        want.append(days[max(done)][-1]["c"] if done else None)
    got = (out.get("d") or {}).get("values")
    bad = [i for i, (g, w) in enumerate(zip(got or [], want)) if g != w]
    rec(S, "L14", "align(close@1d): bağımsız hesapla, gün kapanmadan değer yok",
        "her bar = kapanışı ≤ bar kapanışı olan son günün kapanışı", f"farklı bar {len(bad)} (ilk {bad[:3]})",
        got is not None and not bad, r)


def main():
    static()
    runtime()
    lookahead()
    out = HERE / "results-cli.json"
    out.write_text(json.dumps(R, ensure_ascii=False, indent=1) + "\n")
    print(f"{sum(r['verdict'] == 'OK' for r in R)}/{len(R)} OK -> {out}")


if __name__ == "__main__":
    main()
