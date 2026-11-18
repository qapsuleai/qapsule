"""Denetçi (R5/A madde 6): gömülü yazı tiplerinin ad tablosu, Türkçe karakter kapsamı ve rakam
genişlikleri. Kendi küçük TTF ayrıştırıcım (fontTools yok): name, cmap (biçim 4/12), hmtx."""
import struct
import sys
from pathlib import Path


def tables(b):
    n = struct.unpack(">H", b[4:6])[0]
    t = {}
    for i in range(n):
        tag, _, off, ln = struct.unpack(">4sIII", b[12 + 16 * i: 28 + 16 * i])
        t[tag.decode()] = (off, ln)
    return t


def names(b, t):
    off, _ = t["name"]
    _, count, soff = struct.unpack(">HHH", b[off:off + 6])
    out = {}
    for i in range(count):
        pid, eid, lid, nid, ln, o = struct.unpack(">HHHHHH", b[off + 6 + 12 * i: off + 18 + 12 * i])
        raw = b[off + soff + o: off + soff + o + ln]
        if pid == 3 and lid == 0x409:
            out[nid] = raw.decode("utf-16-be", "replace")
        elif pid == 1 and nid not in out:
            out[nid] = raw.decode("mac_roman", "replace")
    return out


def cmap(b, t):
    off, _ = t["cmap"]
    n = struct.unpack(">H", b[off + 2:off + 4])[0]
    best = None
    for i in range(n):
        pid, eid, so = struct.unpack(">HHI", b[off + 4 + 8 * i: off + 12 + 8 * i])
        fmt = struct.unpack(">H", b[off + so: off + so + 2])[0]
        if fmt == 12:
            best = (12, off + so)
        elif fmt == 4 and best is None:
            best = (4, off + so)
    fmt, s = best
    m = {}
    if fmt == 4:
        segx2 = struct.unpack(">H", b[s + 6:s + 8])[0]
        seg = segx2 // 2
        ends = struct.unpack(f">{seg}H", b[s + 14: s + 14 + segx2])
        starts = struct.unpack(f">{seg}H", b[s + 16 + segx2: s + 16 + 2 * segx2])
        deltas = struct.unpack(f">{seg}h", b[s + 16 + 2 * segx2: s + 16 + 3 * segx2])
        ro_off = s + 16 + 3 * segx2
        ros = struct.unpack(f">{seg}H", b[ro_off: ro_off + segx2])
        for k in range(seg):
            for c in range(starts[k], ends[k] + 1):
                if c == 0xFFFF:
                    continue
                if ros[k] == 0:
                    g = (c + deltas[k]) & 0xFFFF
                else:
                    a = ro_off + 2 * k + ros[k] + 2 * (c - starts[k])
                    g = struct.unpack(">H", b[a:a + 2])[0]
                    if g:
                        g = (g + deltas[k]) & 0xFFFF
                if g:
                    m[c] = g
    else:
        ng = struct.unpack(">I", b[s + 12:s + 16])[0]
        for k in range(ng):
            a, z, g = struct.unpack(">III", b[s + 16 + 12 * k: s + 28 + 12 * k])
            for c in range(a, z + 1):
                m[c] = g + c - a
    return m


def advances(b, t):
    off, _ = t["hhea"]
    nh = struct.unpack(">H", b[off + 34: off + 36])[0]
    ho, _ = t["hmtx"]
    adv = [struct.unpack(">H", b[ho + 4 * i: ho + 4 * i + 2])[0] for i in range(nh)]
    return lambda g: adv[min(g, nh - 1)]


TR = "ğĞşŞıİçÇöÖüÜâÂîÎûÛ"
EXTRA = "₺%‰−–—·…“”‘’✓✗"
for p in sys.argv[1:]:
    b = Path(p).read_bytes()
    t = tables(b)
    nm = names(b, t)
    cm = cmap(b, t)
    adv = advances(b, t)
    miss_tr = [c for c in TR if ord(c) not in cm]
    miss_x = [c for c in EXTRA if ord(c) not in cm]
    digits = sorted({adv(cm[ord(d)]) for d in "0123456789" if ord(d) in cm})
    print(f"== {Path(p).name} ({len(b)} B, {len(cm)} cmap codepoints, tables: {' '.join(sorted(t))})")
    for k in (0, 1, 2, 3, 4, 5, 6, 13, 14):
        if k in nm:
            print(f"   name {k}: {nm[k][:110]}")
    print(f"   Turkish missing: {miss_tr or 'none'}; extra missing: {miss_x or 'none'}; digit advances: {digits}")
