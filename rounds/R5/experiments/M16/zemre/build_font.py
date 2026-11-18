"""Builds the `zemre` look's two static font files from Atkinson Hyperlegible Next (OFL-1.1,
no Reserved Font Name): a reproducible record of what was done to the published file.

    python3 build_font.py <AtkinsonHyperlegibleNext[wght].ttf> <out-dir>

Steps (rounds/R5/RESEARCH.md §4.2): a static instance per weight (fontdue draws a variable
font at its default instance only); the digits 0-9 remapped in the cmap to their `tnum`
glyphs (fontdue applies no OpenType feature, so tabular figures must be the default ones);
a Latin subset (Basic Latin, Latin-1, Latin Extended-A: every Turkish letter) with no layout
features at all (no GSUB ligatures, no GPOS kerning: fontdue applies neither and the canvas
draws with `fontKerning = 'none'`, so both paint paths lay text out alike). Needs fontTools.
Source: https://github.com/google/fonts/tree/main/ofl/atkinsonhyperlegiblenext (2026-11-18).
"""

import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from fontTools import subset

WEIGHTS = {400: "Regular", 700: "Bold"}
UNICODES = (
    list(range(0x20, 0x7F))
    + list(range(0xA0, 0x180))
    + [0x2010, 0x2011, 0x2012, 0x2013, 0x2014, 0x2018, 0x2019, 0x201A, 0x201C, 0x201D,
       0x201E, 0x2022, 0x2026, 0x2030, 0x2039, 0x203A, 0x20AC, 0x2122, 0x2190, 0x2191,
       0x2192, 0x2193, 0x2212]
)


def tnum_map(font):
    """cmap digit glyph -> its `tnum` substitute (single substitution lookups)."""
    gsub = font["GSUB"].table
    out = {}
    for fr in gsub.FeatureList.FeatureRecord:
        if fr.FeatureTag != "tnum":
            continue
        for li in fr.Feature.LookupListIndex:
            lk = gsub.LookupList.Lookup[li]
            for st in lk.SubTable:
                if lk.LookupType == 7:
                    st = st.ExtSubTable
                if hasattr(st, "mapping"):
                    out.update(st.mapping)
    return out


def build(src, out_dir, wght, style):
    font = TTFont(src)
    font = instancer.instantiateVariableFont(font, {"wght": wght}, updateFontNames=False)
    tn = tnum_map(font)
    for table in font["cmap"].tables:
        for cp in range(0x30, 0x3A):
            g = table.cmap.get(cp)
            if g in tn:
                table.cmap[cp] = tn[g]
    opts = subset.Options()
    opts.layout_features = []
    opts.name_IDs = ["*"]
    opts.notdef_outline = True
    opts.hinting = False
    opts.drop_tables += ["GSUB", "GPOS", "GDEF", "STAT", "DSIG"]
    sub = subset.Subsetter(opts)
    sub.populate(unicodes=UNICODES)
    sub.subset(font)
    font["OS/2"].usWeightClass = wght
    fam = "Atkinson Hyperlegible Next"
    for rec in list(font["name"].names):
        if rec.nameID in (1, 16):
            rec.string = fam
        elif rec.nameID in (2, 17):
            rec.string = style
        elif rec.nameID == 4:
            rec.string = f"{fam} {style}"
        elif rec.nameID == 6:
            rec.string = f"AtkinsonHyperlegibleNext-{style}"
    path = Path(out_dir) / f"AtkinsonHyperlegibleNext-{style}.ttf"
    font.save(path)
    cmap = font.getBestCmap()
    adv = sorted({font["hmtx"][cmap[cp]][0] for cp in range(0x30, 0x3A)})
    tr = "ğĞşŞıİçÇöÖüÜâÂîÎûÛ"
    missing = [c for c in tr if ord(c) not in cmap]
    print(path, path.stat().st_size, "digit advances:", adv, "turkish missing:", missing)


if __name__ == "__main__":
    for w, s in WEIGHTS.items():
        build(sys.argv[1], sys.argv[2], w, s)
