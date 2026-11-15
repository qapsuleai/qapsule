#!/usr/bin/env python3
"""Denetçinin bağımsız dedektör hesabı (R3 Evre 1, VERIFY adım 6 ön kontrolü).

qp-facts'i çağırmaz; kurallar rounds/R3/experiments/T3.md tablosundan yeniden yazıldı.
Girdi: export ikilisinin JSON'u. Çalıştırma: python3 -I recompute.py <json> [naive|engine]

Kip:
  naive  — denetçinin kendi aritmetiği: pencere toplamları her bar sıfırdan soldan sağa,
           std iki geçişli (ortalama, sonra kare sapmalar), RSI = 100 - 100/(1+RS).
  engine — T3.md "Denetçiye not"undaki işlem sırası: SMA kayan toplam, Bollinger Welford.
Karar karşılaştırması (aynı bar/kod/yön/kenar) her iki kipte de yapılır; değer farkı raporlanır.
"""
import json
import math
import sys

MODE = sys.argv[2] if len(sys.argv) > 2 else "naive"


def sma(xs, p):
    out = [None] * len(xs)
    if MODE == "naive":
        for i in range(p - 1, len(xs)):
            s = 0.0
            for x in xs[i - p + 1:i + 1]:
                s += x
            out[i] = s / p
    else:
        s = 0.0
        for i, x in enumerate(xs):
            if i >= p - 1:
                out[i] = (s + x) / p
                s = (s + x) - xs[i - p + 1]
            else:
                s = s + x
    return out


def rsi(c, p=14):
    out = [None] * len(c)
    ag = al = 0.0
    for i in range(1, len(c)):
        d = c[i] - c[i - 1]
        g = d if d > 0 else 0.0
        l_ = -d if d < 0 else 0.0
        if i < p:
            ag += g
            al += l_
            continue
        if i == p:
            ag = (ag + g) / p
            al = (al + l_) / p
        else:
            ag = (ag * (p - 1) + g) / p
            al = (al * (p - 1) + l_) / p
        if al == 0.0:
            out[i] = 100.0 if ag > 0 else 50.0
        else:
            out[i] = 100.0 - 100.0 / (1.0 + ag / al)
    return out


def atr(h, l, c, p=14):
    n = len(c)
    tr = []
    for i in range(n):
        if i == 0:
            tr.append(h[i] - l[i])
        else:
            tr.append(max(h[i] - l[i], abs(h[i] - c[i - 1]), abs(l[i] - c[i - 1])))
    out = [None] * n
    a = None
    for i in range(n):
        if i == p - 1:
            s = 0.0
            for x in tr[:p]:
                s += x
            a = s / p
            out[i] = a
        elif i >= p:
            a = (a * (p - 1) + tr[i]) / p
            out[i] = a
    return out


def bollinger(c, p=20, k=2.0):
    n = len(c)
    up = [None] * n
    lo = [None] * n
    mid = [None] * n
    bw = [None] * n
    for i in range(p - 1, n):
        w = c[i - p + 1:i + 1]
        if MODE == "naive":
            s = 0.0
            for x in w:
                s += x
            m = s / p
            q = 0.0
            for x in w:
                q += (x - m) * (x - m)
        else:
            # Welford over the window (fresh each bar; order of a windowed Welford may differ)
            m = 0.0
            q = 0.0
            for j, x in enumerate(w):
                d = x - m
                m += d / (j + 1)
                q += d * (x - m)
        sd = math.sqrt(max(q, 0.0) / p)
        mid[i] = m
        up[i] = m + k * sd
        lo[i] = m - k * sd
        bw[i] = 2 * k * sd / m
    return up, lo, mid, bw


def clip(x):
    if x != x:
        return 0.0
    return min(1.0, max(0.0, x))


def ma_stack(b):
    c = [x[4] for x in b]
    f20 = sma(c, 20)
    s50 = sma(c, 50)
    a = atr([x[2] for x in b], [x[3] for x in b], c)
    out = []
    st = None
    for i in range(len(b)):
        if i < 49:
            continue
        f, s, f5, at = f20[i], s50[i], f20[i - 5], a[i]
        new = None
        if c[i] > f and f > s and f > f5:
            new = "bull"
        elif c[i] < f and f < s and f < f5:
            new = "bear"
        if new != st:
            stren = clip(abs(f - s) / at) if at > 0 else 0.0
            ev = dict(close=c[i], sma20=f, sma50=s, sma20_prev5=f5, atr=at)
            if st is not None:
                out.append((i, st + "_end", "Up" if st == "bull" else "Down", "Exit", stren, ev))
            if new is not None:
                out.append((i, new, "Up" if new == "bull" else "Down", "Enter", stren, ev))
            st = new
    return out


def rsi_div(b):
    h = [x[2] for x in b]
    l = [x[3] for x in b]
    c = [x[4] for x in b]
    r = rsi(c)
    out = []
    lastp = None
    lastt = None
    for i in range(4, len(b)):
        j = i - 2
        if all(h[j] > h[k] for k in (j - 2, j - 1, j + 1, j + 2)):
            if lastp is not None and 5 <= j - lastp <= 40 and h[j] > h[lastp] \
                    and r[j] is not None and r[lastp] is not None and r[j] < r[lastp]:
                ev = dict(p1_price=h[lastp], p2_price=h[j], p1_rsi=r[lastp], p2_rsi=r[j],
                          p1_bar=float(lastp), p2_bar=float(j))
                out.append((i, "bearish", "Down", "Point", clip(abs(r[lastp] - r[j]) / 20), ev))
            lastp = j
        if all(l[j] < l[k] for k in (j - 2, j - 1, j + 1, j + 2)):
            if lastt is not None and 5 <= j - lastt <= 40 and l[j] < l[lastt] \
                    and r[j] is not None and r[lastt] is not None and r[j] > r[lastt]:
                ev = dict(p1_price=l[lastt], p2_price=l[j], p1_rsi=r[lastt], p2_rsi=r[j],
                          p1_bar=float(lastt), p2_bar=float(j))
                out.append((i, "bullish", "Up", "Point", clip(abs(r[lastt] - r[j]) / 20), ev))
            lastt = j
    return out


def breakout(b):
    h = [x[2] for x in b]
    l = [x[3] for x in b]
    c = [x[4] for x in b]
    a = atr(h, l, c)
    out = []
    for i in range(20, len(b)):
        hh = max(h[i - 20:i])
        ll = min(l[i - 20:i])
        if c[i] > hh:
            out.append((i, "up", "Up", "Point", clip(abs(c[i] - hh) / a[i]),
                        dict(level=hh, close=c[i], atr=a[i])))
        elif c[i] < ll:
            out.append((i, "down", "Down", "Point", clip(abs(c[i] - ll) / a[i]),
                        dict(level=ll, close=c[i], atr=a[i])))
    return out


def squeeze(b):
    c = [x[4] for x in b]
    up, lo, mid, bw = bollinger(c)
    out = []
    inside = False
    entry = None
    for i in range(138, len(b)):
        w = bw[i - 119:i + 1]
        mn, mx = min(w), max(w)
        ev = dict(upper=up[i], lower=lo[i], middle=mid[i], bw=bw[i], bw_min=mn, bw_max=mx,
                  half_width=up[i] - mid[i])
        stren = clip((mx - bw[i]) / mx)
        if not inside:
            if bw[i] <= mn:
                inside = True
                entry = bw[i]
                out.append((i, "squeeze", "Neutral", "Enter", stren, ev))
        else:
            if bw[i] > 1.5 * entry:
                ev2 = dict(ev, bw_entry=entry)
                out.append((i, "squeeze_end", "Neutral", "Exit", stren, ev2))
                inside = False
    return out


def extreme(b):
    c = [x[4] for x in b]
    r = rsi(c)
    out = []
    z = None
    for i in range(15, len(b)):
        v, prev = r[i], r[i - 1]
        if z == "ob" and v < 70:
            out.append((i, "overbought_end", "Up", "Exit", clip(abs(v - 70) / 30),
                        dict(level=70.0, value=v, prev=prev)))
            z = None
        elif z == "os" and v > 30:
            out.append((i, "oversold_end", "Down", "Exit", clip(abs(v - 30) / 30),
                        dict(level=30.0, value=v, prev=prev)))
            z = None
        if z is None:
            if prev <= 70 and v > 70:
                z = "ob"
                out.append((i, "overbought", "Up", "Enter", clip(abs(v - 70) / 30),
                            dict(level=70.0, value=v, prev=prev)))
            elif prev >= 30 and v < 30:
                z = "os"
                out.append((i, "oversold", "Down", "Enter", clip(abs(v - 30) / 30),
                            dict(level=30.0, value=v, prev=prev)))
    return out


def vol_surge(b):
    v = [x[5] for x in b]
    sv = sma(v, 20)
    out = []
    for i in range(20, len(b)):
        avg = sv[i - 1]
        if avg > 0 and v[i] > 2.5 * avg:
            ratio = v[i] / avg
            d = b[i][4] - b[i][1]
            dr = "Up" if d > 0 else ("Down" if d < 0 else "Neutral")
            out.append((i, "surge", dr, "Point", clip((ratio - 2.5) / 2.5),
                        dict(volume=v[i], vol_avg=avg, ratio=ratio)))
    return out


def institutional(b, flow):
    out = []
    win = []
    for i in range(len(b)):
        net = flow[i]
        if net is None or not math.isfinite(net):
            continue
        if len(win) == 5:
            a0, a1, a2, a3, a4 = win
            avg = ((((a0 + a1) + a2) + a3) + a4) / 5
            if net != 0 and abs(net) > 2 * avg:
                stren = 1.0 if avg == 0 else clip((abs(net) / avg - 2) / 2)
                out.append((i, "net_buy" if net > 0 else "net_sell",
                            "Up" if net > 0 else "Down", "Point", stren,
                            dict(net=net, avg5=avg)))
        win.append(abs(net))
        if len(win) > 5:
            win.pop(0)
    return out


DETS = {
    "continuation.ma_stack": ma_stack,
    "reversal.rsi_divergence": rsi_div,
    "breakout.range": breakout,
    "squeeze.bandwidth": squeeze,
    "extreme.rsi": extreme,
    "flow.volume_surge": vol_surge,
    "flow.institutional": None,
}


def main():
    data = json.load(open(sys.argv[1]))
    print(f"mode={MODE}")
    total_bad = 0
    for det in DETS:
        n_mine = n_eng = 0
        mism = []
        maxdiff = 0.0
        maxrel = 0.0
        sdiff = 0.0
        bitexact = True
        for inst in data:
            b = inst["bars"]
            mine = institutional(b, inst["flow"]) if det == "flow.institutional" else DETS[det](b)
            eng = inst["detections"][det]
            n_mine += len(mine)
            n_eng += len(eng)
            km = {(x[0], x[1]): x for x in mine}
            ke = {(e["bar"], e["code"]): e for e in eng}
            for k in sorted(set(km) | set(ke)):
                if k not in km or k not in ke:
                    mism.append((inst["symbol"], k, "only-engine" if k in ke else "only-auditor"))
                    continue
                m, e = km[k], ke[k]
                if m[2] != e["dir"] or m[3] != e["edge"]:
                    mism.append((inst["symbol"], k, f"dir/edge {m[2]}/{m[3]} vs {e['dir']}/{e['edge']}"))
                sdiff = max(sdiff, abs(m[4] - e["strength"]))
                if m[4] != e["strength"]:
                    bitexact = False
                for key, val in m[5].items():
                    if key not in e["evidence"]:
                        mism.append((inst["symbol"], k, f"missing evidence {key}"))
                        continue
                    ev = e["evidence"][key]
                    d = abs(val - ev)
                    if d != 0.0:
                        bitexact = False
                    maxdiff = max(maxdiff, d)
                    if ev != 0:
                        maxrel = max(maxrel, d / abs(ev))
                extra = set(e["evidence"]) - set(m[5])
                if extra:
                    mism.append((inst["symbol"], k, f"extra evidence {sorted(extra)}"))
        total_bad += len(mism)
        print(f"{det:26s} engine={n_eng:5d} auditor={n_mine:5d} mismatches={len(mism):3d} "
              f"max_abs={maxdiff:.3e} max_rel={maxrel:.3e} strength_max_abs={sdiff:.3e} "
              f"bit_exact={bitexact}")
        for x in mism[:12]:
            print("   ", x)
    print("TOTAL_MISMATCHES", total_bad)


main()
