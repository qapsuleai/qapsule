"""Colour arithmetic (SPEC §G7): WCAG 2.x contrast, CIEDE2000, colour-vision-deficiency
simulation, alpha compositing. Pure functions, no I/O; unit-tested in tests/test_color.py.

CVD methods (RESEARCH §8, §16, F6; DaltonLens 2021 review):
- protan, deutan: Viénot, Brettel & Mollon 1999 (one projection plane through black, white
  and sRGB blue in LMS; Smith–Pokorny cone space of the paper);
- tritan: Brettel, Viénot & Mollon 1997 (two half-planes through the neutral axis, anchors
  485 nm and 660 nm);
- Machado, Oliveira & Fernandes 2009 (severity 1.0) for all three, reported as a second
  column; the gate takes the smaller ΔE of the two methods (conservative).
"""

from __future__ import annotations

import math

# --- sRGB <-> linear ------------------------------------------------------------------


def srgb_to_lin(c: float) -> float:
    c = c / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def lin_to_srgb(v: float) -> float:
    v = min(1.0, max(0.0, v))
    s = v * 12.92 if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055
    return s * 255.0


def composite(fg, bg):
    """`fg` RGBA (0..255, straight alpha) over opaque `bg` RGB -> RGB (sRGB-space blend, as
    browsers and tiny-skia blend)."""
    a = (fg[3] if len(fg) > 3 else 255) / 255.0
    return tuple(fg[i] * a + bg[i] * (1 - a) for i in range(3))


def rel_luminance(rgb) -> float:
    r, g, b = (srgb_to_lin(x) for x in rgb[:3])
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a, b) -> float:
    """WCAG 2.x contrast ratio of two opaque sRGB colours."""
    la, lb = rel_luminance(a), rel_luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def is_large_text(px: float, bold: bool) -> bool:
    """WCAG 'large scale': >= 18 pt (24 CSS px) or >= 14 pt (18.66 CSS px) bold."""
    return px >= 24.0 or (bold and px >= 18.66)


def text_threshold(px: float, bold: bool) -> float:
    return 3.0 if is_large_text(px, bold) else 4.5


# --- CIELAB and CIEDE2000 ---------------------------------------------------------------

_M_XYZ = ((0.4124564, 0.3575761, 0.1804375), (0.2126729, 0.7151522, 0.0721750), (0.0193339, 0.1191920, 0.9503041))
_WHITE = (0.95047, 1.0, 1.08883)


def lab(rgb):
    lr = [srgb_to_lin(x) for x in rgb[:3]]
    xyz = [sum(_M_XYZ[i][j] * lr[j] for j in range(3)) for i in range(3)]

    def f(t):
        return t ** (1 / 3) if t > (6 / 29) ** 3 else t / (3 * (6 / 29) ** 2) + 4 / 29

    fx, fy, fz = (f(xyz[i] / _WHITE[i]) for i in range(3))
    return (116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz))


def de2000(lab1, lab2) -> float:
    """CIEDE2000 (Sharma, Wu & Dalal 2005 implementation notes), kL = kC = kH = 1."""
    L1, a1, b1 = lab1
    L2, a2, b2 = lab2
    C1, C2 = math.hypot(a1, b1), math.hypot(a2, b2)
    Cb = (C1 + C2) / 2
    G = 0.5 * (1 - math.sqrt(Cb ** 7 / (Cb ** 7 + 25 ** 7)))
    a1p, a2p = (1 + G) * a1, (1 + G) * a2
    C1p, C2p = math.hypot(a1p, b1), math.hypot(a2p, b2)

    def hp(a, b):
        if a == 0 and b == 0:
            return 0.0
        h = math.degrees(math.atan2(b, a))
        return h + 360 if h < 0 else h

    h1p, h2p = hp(a1p, b1), hp(a2p, b2)
    dLp = L2 - L1
    dCp = C2p - C1p
    if C1p * C2p == 0:
        dhp = 0.0
    else:
        dhp = h2p - h1p
        if dhp > 180:
            dhp -= 360
        elif dhp < -180:
            dhp += 360
    dHp = 2 * math.sqrt(C1p * C2p) * math.sin(math.radians(dhp / 2))
    Lbp = (L1 + L2) / 2
    Cbp = (C1p + C2p) / 2
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
    dth = 30 * math.exp(-(((hbp - 275) / 25) ** 2))
    Rc = 2 * math.sqrt(Cbp ** 7 / (Cbp ** 7 + 25 ** 7))
    Sl = 1 + 0.015 * (Lbp - 50) ** 2 / math.sqrt(20 + (Lbp - 50) ** 2)
    Sc = 1 + 0.045 * Cbp
    Sh = 1 + 0.015 * Cbp * T
    Rt = -math.sin(math.radians(2 * dth)) * Rc
    return math.sqrt((dLp / Sl) ** 2 + (dCp / Sc) ** 2 + (dHp / Sh) ** 2 + Rt * (dCp / Sc) * (dHp / Sh))


def de(rgb1, rgb2) -> float:
    return de2000(lab(rgb1), lab(rgb2))


# --- CVD simulation ---------------------------------------------------------------------


def _mat(m, v):
    return [sum(m[i][j] * v[j] for j in range(3)) for i in range(3)]


def _inv3(m):
    a, b, c = m[0]
    d, e, f = m[1]
    g, h, i = m[2]
    det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    return [[(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det],
            [(f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det],
            [(d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det]]


# Linear sRGB -> LMS (Smith & Pokorny 1975 fundamentals with sRGB primaries), the matrix
# of Viénot, Brettel & Mollon 1999, eq. (5).
LMS = [[17.8824, 43.5161, 4.11935], [3.45565, 27.1554, 3.86714], [0.0299566, 0.184309, 1.46709]]
LMS_INV = _inv3(LMS)

# Values as in DaltonLens (CIE 1931 2°): 485 nm (0.05699, 0.16987, 0.5864), 660 nm
# (0.16161, 0.061, 0.00001).
_CMF = {485: (0.05699, 0.16987, 0.58640), 660: (0.16161, 0.06100, 0.00001)}
# XYZ -> linear sRGB (IEC 61966-2-1).
_RGB_FROM_XYZ = _inv3([list(r) for r in _M_XYZ])

# Machado, Oliveira & Fernandes 2009, severity 1.0 (linear RGB).
MACHADO = {
    "protan": [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
    "deutan": [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
    "tritan": [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
}


def _plane(missing: int, a_lms, b_lms):
    """Coefficients (p, q) so that LMS[missing] = p * LMS[u] + q * LMS[v] holds for both
    anchor colours a and b (u, v the two remaining cones)."""
    u, v = [k for k in range(3) if k != missing]
    det = a_lms[u] * b_lms[v] - a_lms[v] * b_lms[u]
    p = (a_lms[missing] * b_lms[v] - b_lms[missing] * a_lms[v]) / det
    q = (a_lms[u] * b_lms[missing] - b_lms[u] * a_lms[missing]) / det
    return u, v, p, q


_W_LMS = _mat(LMS, [1.0, 1.0, 1.0])
_B_LMS = _mat(LMS, [0.0, 0.0, 1.0])
_VIENOT = {"protan": _plane(0, _W_LMS, _B_LMS), "deutan": _plane(1, _W_LMS, _B_LMS)}


def _anchor_lms(nm):
    return _mat(LMS, _mat(_RGB_FROM_XYZ, list(_CMF[nm])))


_T485, _T660 = _anchor_lms(485), _anchor_lms(660)
_TRITAN = (_plane(2, _W_LMS, _T485), _plane(2, _W_LMS, _T660))
# Half-plane selection as in DaltonLens (Simulator_Brettel1997): the separation plane's
# normal is neutral x confusion axis (tritan: the S axis), oriented so that the 485 nm wing
# is on its positive side; a stimulus with a negative dot product takes the 660 nm plane.
_N_SEP = [_W_LMS[1] * 1.0 - _W_LMS[2] * 0.0, _W_LMS[2] * 0.0 - _W_LMS[0] * 1.0, 0.0]
if sum(_N_SEP[k] * _T485[k] for k in range(3)) < 0:
    _N_SEP = [-x for x in _N_SEP]


def _project(lms, plane, missing):
    u, v, p, q = plane
    out = list(lms)
    out[missing] = p * lms[u] + q * lms[v]
    return out


def simulate(rgb, kind: str, method: str = "vienot_brettel"):
    """Simulated sRGB colour (0..255) seen by a dichromat (`protan`, `deutan`, `tritan`)."""
    lin = [srgb_to_lin(x) for x in rgb[:3]]
    if method == "machado":
        out = _mat(MACHADO[kind], lin)
    elif kind in _VIENOT:
        lms = _mat(LMS, lin)
        out = _mat(LMS_INV, _project(lms, _VIENOT[kind], 0 if kind == "protan" else 1))
    elif kind == "tritan":
        lms = _mat(LMS, lin)
        side = sum(_N_SEP[i] * lms[i] for i in range(3))
        plane = _TRITAN[1] if side < 0 else _TRITAN[0]
        out = _mat(LMS_INV, _project(lms, plane, 2))
    else:
        raise ValueError(kind)
    return tuple(lin_to_srgb(v) for v in out)


def gray(rgb):
    """Greyscale by relative luminance (what a grey filter shows)."""
    y = rel_luminance(rgb)
    g = lin_to_srgb(y)
    return (g, g, g)


CONDITIONS = ("normal", "gray", "protan", "deutan", "tritan")


def view(rgb, cond: str, method: str = "vienot_brettel"):
    if cond == "normal":
        return tuple(float(x) for x in rgb[:3])
    if cond == "gray":
        return gray(rgb)
    return simulate(rgb, cond, method)


def pair_de(c1, c2, cond: str) -> float:
    """ΔE00 of two colours under a condition; for CVD the smaller of the two methods."""
    if cond in ("normal", "gray"):
        return de(view(c1, cond), view(c2, cond))
    return min(de(view(c1, cond), view(c2, cond)), de(view(c1, cond, "machado"), view(c2, cond, "machado")))
