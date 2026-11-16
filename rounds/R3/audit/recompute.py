"""R3 son denetim: yedi dedektörün bağımsız yeniden hesabı (VERIFY 6).

Yalnız `rounds/R3/experiments/T3.md` tablosundan ve `qp-compute` formül tablosundan yazıldı;
qp-facts'i çağırmaz, verify-e1 betiğine bakılmadı. Aritmetik denetçinin kendisi: her
pencere her barda sıfırdan (sum / p), std iki geçişli popülasyon std'si.

Kullanım: python3 -I recompute.py <bars.json> <engine.json>
Çıktı: dedektör başına karar (bar, kod) eşleşmesi ve kanıt değerlerinin en büyük göreli farkı.
"""
import json
import math
import sys


def sma(x, p, i):
    if i < p - 1:
        return None
    return sum(x[i - p + 1:i + 1]) / p


def rsi_series(c, p=14):
    out = [None] * len(c)
    if len(c) <= p:
        return out
    ch = [c[k] - c[k - 1] for k in range(1, len(c))]
    g = sum(max(d, 0.0) for d in ch[:p]) / p
    l = sum(max(-d, 0.0) for d in ch[:p]) / p

    def val(g, l):
        if l == 0.0:
            return 50.0 if g == 0.0 else 100.0
        return 100.0 - 100.0 / (1.0 + g / l)

    out[p] = val(g, l)
    for k in range(p + 1, len(c)):
        d = c[k] - c[k - 1]
        g = (g * (p - 1) + max(d, 0.0)) / p
        l = (l * (p - 1) + max(-d, 0.0)) / p
        out[k] = val(g, l)
    return out


def atr_series(h, l, c, p=14):
    n = len(c)
    tr = [h[0] - l[0]] + [max(h[k] - l[k], abs(h[k] - c[k - 1]), abs(l[k] - c[k - 1]))
                          for k in range(1, n)]
    out = [None] * n
    if n < p:
        return out
    a = sum(tr[:p]) / p
    out[p - 1] = a
    for k in range(p, n):
        a = (a * (p - 1) + tr[k]) / p
        out[k] = a
    return out


def boll(c, i, p=20, k=2.0):
    if i < p - 1:
        return None
    w = c[i - p + 1:i + 1]
    m = sum(w) / p
    var = sum((x - m) ** 2 for x in w) / p
    sd = math.sqrt(var) if var > 0 else 0.0
    bw = 0.0 if m == 0 else 2 * k * sd / m
    return m, m + k * sd, m - k * sd, bw


def clip(s):
    if s != s:
        return 0.0
    return min(1.0, max(0.0, s))


def detect(bars, flow):
    t = [b[0] for b in bars]
    o = [b[1] for b in bars]
    h = [b[2] for b in bars]
    l = [b[3] for b in bars]
    c = [b[4] for b in bars]
    v = [b[5] for b in bars]
    n = len(bars)
    rsi = rsi_series(c)
    atr = atr_series(h, l, c)
    s20 = [sma(c, 20, i) for i in range(n)]
    s50 = [sma(c, 50, i) for i in range(n)]
    out = {k: [] for k in ["continuation.ma_stack", "reversal.rsi_divergence", "breakout.range",
                           "squeeze.bandwidth", "extreme.rsi", "flow.volume_surge",
                           "flow.institutional"]}

    # ma_stack
    st = None
    for i in range(n):
        if i < 49 or s20[i - 5] is None or atr[i] is None:
            continue
        f, s, f5 = s20[i], s50[i], s20[i - 5]
        new = "bull" if (c[i] > f and f > s and f > f5) else (
            "bear" if (c[i] < f and f < s and f < f5) else None)
        stren = clip(abs(f - s) / atr[i]) if atr[i] > 0 else 0.0
        ev = {"close": c[i], "sma20": f, "sma50": s, "sma20_prev5": f5, "atr": atr[i]}
        if new != st:
            if st is not None:
                out["continuation.ma_stack"].append((i, st + "_end", stren, ev))
            if new is not None:
                out["continuation.ma_stack"].append((i, new, stren, ev))
            st = new

    # rsi_divergence
    last_peak = last_trough = None
    for i in range(4, n):
        j = i - 2
        if all(h[j] > h[q] for q in (j - 2, j - 1, j + 1, j + 2)):
            if last_peak is not None:
                p1 = last_peak
                if 5 <= j - p1 <= 40 and h[j] > h[p1] and rsi[p1] is not None and rsi[j] is not None \
                        and rsi[j] < rsi[p1]:
                    out["reversal.rsi_divergence"].append(
                        (i, "bearish", clip(abs(rsi[p1] - rsi[j]) / 20),
                         {"p1_price": h[p1], "p2_price": h[j], "p1_rsi": rsi[p1], "p2_rsi": rsi[j],
                          "p1_bar": p1, "p2_bar": j}))
            last_peak = j
        if all(l[j] < l[q] for q in (j - 2, j - 1, j + 1, j + 2)):
            if last_trough is not None:
                p1 = last_trough
                if 5 <= j - p1 <= 40 and l[j] < l[p1] and rsi[p1] is not None and rsi[j] is not None \
                        and rsi[j] > rsi[p1]:
                    out["reversal.rsi_divergence"].append(
                        (i, "bullish", clip(abs(rsi[p1] - rsi[j]) / 20),
                         {"p1_price": l[p1], "p2_price": l[j], "p1_rsi": rsi[p1], "p2_rsi": rsi[j],
                          "p1_bar": p1, "p2_bar": j}))
            last_trough = j

    # breakout.range
    for i in range(20, n):
        hh = max(h[i - 20:i])
        ll = min(l[i - 20:i])
        if c[i] > hh:
            out["breakout.range"].append((i, "up", clip(abs(c[i] - hh) / atr[i]),
                                          {"level": hh, "close": c[i], "atr": atr[i]}))
        elif c[i] < ll:
            out["breakout.range"].append((i, "down", clip(abs(c[i] - ll) / atr[i]),
                                          {"level": ll, "close": c[i], "atr": atr[i]}))

    # squeeze.bandwidth
    bws = [None] * n
    bb = [None] * n
    for i in range(19, n):
        bb[i] = boll(c, i)
        bws[i] = bb[i][3]
    insq, entry = False, None
    for i in range(138, n):
        win = bws[i - 119:i + 1]
        bmin, bmax = min(win), max(win)
        m, up, lo, bw = bb[i]
        ev = {"upper": up, "lower": lo, "middle": m, "bw": bw, "bw_min": bmin, "bw_max": bmax,
              "half_width": up - m}
        stren = clip((bmax - bw) / bmax) if bmax != 0 else 0.0
        if not insq and bw <= bmin:
            insq, entry = True, bw
            out["squeeze.bandwidth"].append((i, "squeeze", stren, ev))
        elif insq and bw > 1.5 * entry:
            ev = dict(ev, bw_entry=entry)
            insq = False
            out["squeeze.bandwidth"].append((i, "squeeze_end", stren, ev))

    # extreme.rsi
    zone = None
    for i in range(15, n):
        vv, pv = rsi[i], rsi[i - 1]
        if vv is None or pv is None:
            continue
        if zone == "ob" and vv < 70:
            out["extreme.rsi"].append((i, "overbought_end", clip(abs(vv - 70) / 30),
                                       {"level": 70.0, "value": vv, "prev": pv}))
            zone = None
        elif zone == "os" and vv > 30:
            out["extreme.rsi"].append((i, "oversold_end", clip(abs(vv - 30) / 30),
                                       {"level": 30.0, "value": vv, "prev": pv}))
            zone = None
        if zone is None:
            if pv <= 70 and vv > 70:
                zone = "ob"
                out["extreme.rsi"].append((i, "overbought", clip(abs(vv - 70) / 30),
                                           {"level": 70.0, "value": vv, "prev": pv}))
            elif pv >= 30 and vv < 30:
                zone = "os"
                out["extreme.rsi"].append((i, "oversold", clip(abs(vv - 30) / 30),
                                           {"level": 30.0, "value": vv, "prev": pv}))

    # flow.volume_surge
    for i in range(20, n):
        avg = sum(v[i - 20:i]) / 20
        if avg > 0 and v[i] > 2.5 * avg:
            r = v[i] / avg
            out["flow.volume_surge"].append((i, "surge", clip((r - 2.5) / 2.5),
                                             {"volume": v[i], "vol_avg": avg, "ratio": r}))

    # flow.institutional (pencere = önceki 5 gözlem)
    win = []
    for i in range(n):
        x = flow[i]
        if not isinstance(x, (int, float)) or isinstance(x, bool) or not math.isfinite(x):
            continue
        net = float(x)
        if len(win) == 5:
            avg5 = sum(win) / 5
            if net != 0 and abs(net) > 2 * avg5:
                s = 1.0 if avg5 == 0 else clip((abs(net) / avg5 - 2) / 2)
                out["flow.institutional"].append((i, "net_buy" if net > 0 else "net_sell", s,
                                                  {"net": net, "avg5": avg5}))
        win.append(abs(net))
        if len(win) > 5:
            win.pop(0)
    return out


def main():
    bars_in = json.load(open(sys.argv[1]))
    eng = json.load(open(sys.argv[2]))
    tot = {}
    for inst, e in zip(bars_in, eng):
        mine = detect(inst["bars"], inst["flow"])
        for det, rows in mine.items():
            er = e["detections"][det]
            a = sorted((r[0], r[1]) for r in rows)
            b = sorted((r["bar"], r["code"]) for r in er)
            d = tot.setdefault(det, {"mine": 0, "engine": 0, "only_mine": 0, "only_engine": 0,
                                     "max_rel": 0.0, "first": None})
            d["mine"] += len(a)
            d["engine"] += len(b)
            sa, sb = set(a), set(b)
            d["only_mine"] += len(sa - sb)
            d["only_engine"] += len(sb - sa)
            if (sa ^ sb) and d["first"] is None:
                d["first"] = (inst["symbol"], sorted(sa ^ sb)[:3])
            emap = {(r["bar"], r["code"]): r for r in er}
            for (i, code, s, ev) in rows:
                r = emap.get((i, code))
                if r is None:
                    continue
                pairs = [(s, r["strength"])] + [(ev[k], r["evidence"].get(k)) for k in ev]
                for x, y in pairs:
                    if y is None:
                        d["max_rel"] = float("inf")
                        continue
                    rel = abs(x - y) / max(abs(x), abs(y), 1e-300)
                    d["max_rel"] = max(d["max_rel"], rel)
    for det, d in tot.items():
        print(f"{det:28s} denetçi {d['mine']:5d}  motor {d['engine']:5d}  "
              f"yalnız-denetçi {d['only_mine']}  yalnız-motor {d['only_engine']}  "
              f"en büyük göreli fark {d['max_rel']:.2e}  {d['first'] or ''}")


main()
