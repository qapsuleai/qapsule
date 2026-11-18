"""Denetçi (R5/A madde 4): renk hesapları — düzeneğin kodundan bağımsız uygulama.

- WCAG 2.2 göreli parlaklık ve karşıtlık oranı (sRGB, 0,04045 eşiği).
- Alfa bindirme düz (önçarpımsız) RGBA ile, sRGB uzayında (tarayıcı ve tiny-skia gibi).
- CIELAB (D65) ve CIEDE2000 (Sharma 2005 formülleri).
- Gri: göreli parlaklık Y'den sRGB gri (Y → L*, a = b = 0).
- Renk körlüğü: Viénot 1999 protan/deutan (doğrusal RGB matrisleri, DaltonLens'in yayımladığı
  değerler) ve Machado 2009 protan/deutan/tritan (şiddet 1,0). Tritan için Brettel yerine Machado
  (düzenekten bilerek farklı yöntem); düzenek protan/deutan'da Viénot+Machado'nun küçüğünü alır.
"""
from __future__ import annotations

import math


def hex2rgba(h: str):
    h = h.lstrip("#")
    r, g, b = (int(h[i:i + 2], 16) for i in (0, 2, 4))
    a = int(h[6:8], 16) if len(h) >= 8 else 255
    return r, g, b, a


def over(fg, bg, alpha=None):
    r, g, b, a = fg
    a = (a if alpha is None else alpha) / 255.0
    return tuple(round(fg[i] * a + bg[i] * (1 - a)) for i in range(3))


def lin(c):
    c = c / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def unlin(v):
    v = min(1.0, max(0.0, v))
    return 255 * (12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055)


def lum(rgb):
    r, g, b = (lin(x) for x in rgb[:3])
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a, b):
    la, lb = lum(a), lum(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def lab(rgb):
    r, g, b = (lin(x) for x in rgb[:3])
    x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b
    y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b
    z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b
    xn, yn, zn = 0.95047, 1.0, 1.08883

    def f(t):
        return t ** (1 / 3) if t > (6 / 29) ** 3 else t / (3 * (6 / 29) ** 2) + 4 / 29

    fx, fy, fz = f(x / xn), f(y / yn), f(z / zn)
    return 116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)


def de2000(c1, c2):
    return de2000_lab(lab(c1), lab(c2))


def de2000_lab(p1, p2):
    L1, a1, b1 = p1
    L2, a2, b2 = p2
    C1, C2 = math.hypot(a1, b1), math.hypot(a2, b2)
    Cb = (C1 + C2) / 2
    G = 0.5 * (1 - math.sqrt(Cb ** 7 / (Cb ** 7 + 25 ** 7)))
    a1p, a2p = (1 + G) * a1, (1 + G) * a2
    C1p, C2p = math.hypot(a1p, b1), math.hypot(a2p, b2)
    h1p = math.degrees(math.atan2(b1, a1p)) % 360 if C1p else 0.0
    h2p = math.degrees(math.atan2(b2, a2p)) % 360 if C2p else 0.0
    dLp, dCp = L2 - L1, C2p - C1p
    if C1p * C2p == 0:
        dhp = 0.0
    elif abs(h2p - h1p) <= 180:
        dhp = h2p - h1p
    elif h2p - h1p > 180:
        dhp = h2p - h1p - 360
    else:
        dhp = h2p - h1p + 360
    dHp = 2 * math.sqrt(C1p * C2p) * math.sin(math.radians(dhp / 2))
    Lbp, Cbp = (L1 + L2) / 2, (C1p + C2p) / 2
    if C1p * C2p == 0:
        hbp = h1p + h2p
    elif abs(h1p - h2p) <= 180:
        hbp = (h1p + h2p) / 2
    elif h1p + h2p < 360:
        hbp = (h1p + h2p + 360) / 2
    else:
        hbp = (h1p + h2p - 360) / 2
    T = (1 - 0.17 * math.cos(math.radians(hbp - 30)) + 0.24 * math.cos(math.radians(2 * hbp))
         + 0.32 * math.cos(math.radians(3 * hbp + 6)) - 0.20 * math.cos(math.radians(4 * hbp - 63)))
    dtheta = 30 * math.exp(-(((hbp - 275) / 25) ** 2))
    RC = 2 * math.sqrt(Cbp ** 7 / (Cbp ** 7 + 25 ** 7))
    SL = 1 + 0.015 * (Lbp - 50) ** 2 / math.sqrt(20 + (Lbp - 50) ** 2)
    SC = 1 + 0.045 * Cbp
    SH = 1 + 0.015 * Cbp * T
    RT = -math.sin(math.radians(2 * dtheta)) * RC
    return math.sqrt((dLp / SL) ** 2 + (dCp / SC) ** 2 + (dHp / SH) ** 2 + RT * (dCp / SC) * (dHp / SH))


def matrix(m, rgb):
    v = [lin(x) for x in rgb[:3]]
    return tuple(unlin(sum(m[i][j] * v[j] for j in range(3))) for i in range(3))


VIENOT = {
    "protan": [[0.11238, 0.88762, 0.0], [0.11238, 0.88762, 0.0], [0.00401, -0.00401, 1.0]],
    "deutan": [[0.29275, 0.70725, 0.0], [0.29275, 0.70725, 0.0], [-0.02234, 0.02234, 1.0]],
}
MACHADO = {
    "protan": [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
    "deutan": [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
    "tritan": [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
}


def gray(rgb):
    y = lum(rgb)
    g = unlin(y)
    return (g, g, g)


def conditions(rgb):
    """The five viewing conditions: normal, gray, protan, deutan, tritan (each simulated
    colour; for protan/deutan both Viénot and Machado, the caller takes the smaller ΔE)."""
    return {
        "normal": [tuple(rgb)],
        "gray": [gray(rgb)],
        "protan": [matrix(VIENOT["protan"], rgb), matrix(MACHADO["protan"], rgb)],
        "deutan": [matrix(VIENOT["deutan"], rgb), matrix(MACHADO["deutan"], rgb)],
        "tritan": [matrix(MACHADO["tritan"], rgb)],
    }


def de_conditions(a, b):
    ca, cb = conditions(a), conditions(b)
    return {k: min(de2000(x, y) for x, y in zip(ca[k], cb[k])) for k in ca}


if __name__ == "__main__":
    # self-check against published CIEDE2000 test pairs (Sharma 2005, pairs 1 and 17) via Lab is
    # not possible without Lab input; check WCAG on known values instead.
    assert abs(contrast((0, 0, 0), (255, 255, 255)) - 21.0) < 1e-9
    assert abs(contrast((118, 118, 118), (255, 255, 255)) - 4.54) < 0.01
    assert de2000((255, 0, 0), (255, 0, 0)) == 0
    # Sharma, Wu, Dalal (2005) test data, pairs 1, 7, 17, 25
    for p1, p2, want in [((50, 2.6772, -79.7751), (50, 0, -82.7485), 2.0425),
                         ((50, 0, 0), (50, -1, 2), 2.3669),
                         ((50, 2.5, 0), (73, 25, -18), 27.1492),
                         ((60.2574, -34.0099, 36.2677), (60.4626, -34.1751, 39.4387), 1.2644)]:
        assert abs(de2000_lab(p1, p2) - want) < 1e-4, (p1, p2, de2000_lab(p1, p2))
    print("colour self-check ok")
