"""Derived stimuli (SPEC §G3, §G5): throttle copy (blur + downscale), grey copy, side-by-side
pair for the critic, motion pair with region letters. Deterministic (Pillow, fixed
parameters); every output is a metadata-free PNG (RESEARCH U12).
"""

from __future__ import annotations

from PIL import Image, ImageDraw, ImageFilter, ImageFont

# SPEC §G3: Gaussian σ = 4.5 CSS px (calibrated, calibration/sigma.json) at the reader's size, then 0.5× (Lanczos). Pillow's
# GaussianBlur `radius` is the standard deviation (Pillow docs, ImageFilter.GaussianBlur).
THROTTLE_SIGMA = 4.5
THROTTLE_SCALE = 0.5
GAP = 24          # px between the two halves of a pair
LABEL_H = 28      # px band above each half for the letter


def _clean(img: Image.Image) -> Image.Image:
    out = Image.new("RGB", img.size)
    out.paste(img.convert("RGB"))
    return out


def save(img: Image.Image, path) -> None:
    _clean(img).save(path, format="PNG", optimize=False)


def throttle(src, dst, sigma: float = THROTTLE_SIGMA, scale: float = THROTTLE_SCALE) -> None:
    im = Image.open(src).convert("RGB").filter(ImageFilter.GaussianBlur(radius=sigma))
    w, h = im.size
    save(im.resize((max(1, round(w * scale)), max(1, round(h * scale))), Image.LANCZOS), dst)


def half(src, dst, scale: float = THROTTLE_SCALE) -> None:
    """The glance copy: 0.5x (Lanczos), no blur (SPEC §G3a)."""
    im = Image.open(src).convert("RGB")
    w, h = im.size
    save(im.resize((max(1, round(w * scale)), max(1, round(h * scale))), Image.LANCZOS), dst)


def grey(src, dst) -> None:
    """Luminance copy (ITU-R BT.709 weights on linear light, re-encoded to sRGB)."""
    im = Image.open(src).convert("RGB")
    lut = [((i / 255) / 12.92 if i / 255 <= 0.04045 else ((i / 255 + 0.055) / 1.055) ** 2.4) for i in range(256)]
    px = im.load()
    out = Image.new("RGB", im.size)
    po = out.load()
    for y in range(im.size[1]):
        for x in range(im.size[0]):
            r, g, b = px[x, y]
            v = 0.2126 * lut[r] + 0.7152 * lut[g] + 0.0722 * lut[b]
            s = v * 12.92 if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055
            c = round(min(1.0, max(0.0, s)) * 255)
            po[x, y] = (c, c, c)
    save(out, dst)


def _font(size: int):
    for f in ("/System/Library/Fonts/Supplemental/Arial.ttf", "/Library/Fonts/Arial.ttf",
              "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(f, size)
        except OSError:
            continue
    return ImageFont.load_default()


def pair(left, right, dst, labels=("A", "B")) -> None:
    """Two screens side by side at 1× (no scaling; 2 × 402 + gap < 1568 px, so the model
    does not downscale either), a neutral letter above each."""
    a, b = Image.open(left).convert("RGB"), Image.open(right).convert("RGB")
    h = max(a.size[1], b.size[1])
    out = Image.new("RGB", (a.size[0] + GAP + b.size[0], h + LABEL_H), (128, 128, 128))
    out.paste(a, (0, LABEL_H))
    out.paste(b, (a.size[0] + GAP, LABEL_H))
    d = ImageDraw.Draw(out)
    f = _font(20)
    for x0, w, t in ((0, a.size[0], labels[0]), (a.size[0] + GAP, b.size[0], labels[1])):
        d.text((x0 + w // 2 - 6, 3), t, fill=(255, 255, 255), font=f)
    save(out, dst)


def regions(src, dst) -> None:
    """Quadrant letters A (top left), B (top right), C (bottom left), D (bottom right) drawn
    over a frame (M21 `m_where`); thin grey lines and small letters, the same for every
    design."""
    im = Image.open(src).convert("RGB")
    w, h = im.size
    d = ImageDraw.Draw(im)
    d.line([(w // 2, 0), (w // 2, h)], fill=(128, 128, 128), width=1)
    d.line([(0, h // 2), (w, h // 2)], fill=(128, 128, 128), width=1)
    f = _font(16)
    for t, (x, y) in zip("ABCD", ((4, 4), (w // 2 + 4, 4), (4, h // 2 + 4), (w // 2 + 4, h // 2 + 4))):
        d.rectangle([x - 2, y - 2, x + 14, y + 18], fill=(128, 128, 128))
        d.text((x, y), t, fill=(255, 255, 255), font=f)
    save(im, dst)


def columns(src, dst, plot) -> None:
    """(M18 `c_live`) Four numbered columns 1-4 over the price plot's x extent (the visible
    bar range in four equal parts): thin grey lines at the three inner borders and at both
    ends, a small letter at the top of each column; the same for every design. `plot` =
    [x0, x1, y0, y1] in image px (gl.compute.chart_plot)."""
    im = Image.open(src).convert("RGB")
    x0, x1, y0, y1 = plot
    d = ImageDraw.Draw(im)
    f = _font(14)
    for i in range(5):
        x = round(x0 + (x1 - x0) * i / 4)
        d.line([(x, y0), (x, y1)], fill=(128, 128, 128), width=1)
    for i, t in enumerate("1234"):  # numbered: the reader's answer options are lettered
        x = round(x0 + (x1 - x0) * (i + 0.5) / 4) - 6
        y = round(y1) + 2
        d.rectangle([x - 2, y - 1, x + 13, y + 16], fill=(128, 128, 128))
        d.text((x, y), t, fill=(255, 255, 255), font=f)
    save(im, dst)


def quadrant(x: float, y: float, w: float, h: float) -> str:
    return ("A" if x < w / 2 else "B") if y < h / 2 else ("C" if x < w / 2 else "D")
