"""R4 denetimi, VERIFY 15: çekirdeğin boyadığı yüz görüntülerinde karşıtlık (WCAG oranı).
Arka plan = en sık renk. Her bölge için: bölgedeki arka plana en karşıt piksel (dolu çizginin
rengi; kenar yumuşatması bunu düşürmez, yükseltmez). Bölgeler: üst satır (ek bilgi), işaret
(sol sütun), başlık (sağ). Gri kopya: ITU-R BT.601 luma, ayrıca kaydedilir."""
import sys, collections
from PIL import Image

def lum(c):
    def ch(v):
        v /= 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = c[:3]
    return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)

def ratio(a, b):
    la, lb = sorted([lum(a), lum(b)], reverse=True)
    return (la + 0.05) / (lb + 0.05)

def region_max(img, box, bg):
    best, col = 1.0, None
    for y in range(box[1], box[3]):
        for x in range(box[0], box[2]):
            p = img.getpixel((x, y))
            r = ratio(p, bg)
            if r > best:
                best, col = r, p
    return best, col

for path in sys.argv[1:]:
    im = Image.open(path).convert("RGB")
    w, h = im.size
    bg = collections.Counter(im.getdata()).most_common(1)[0][0]
    s = w / 390
    top = region_max(im, (0, 0, w, int(26 * s)), bg)
    glyph = region_max(im, (0, int(28 * s), int(44 * s), int(80 * s)), bg)
    head = region_max(im, (int(48 * s), int(28 * s), w, h), bg)
    print(f"{path.split('/')[-1]}: bg {bg} | ek bilgi {top[0]:.2f} | işaret {glyph[0]:.2f} {glyph[1]} | başlık {head[0]:.2f} {head[1]}")
