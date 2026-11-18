"""Denetçi (R5/A madde 4): kapsül dilinin bağımsız taraması.

Düzeneğin `scan.rs`'ini kullanmaz ve listeleri ondan kopyalamaz. Girdi yalnız metin: `langh dump`
çıktısındaki `dump.json` (ürün işlevlerinin ürettiği başlık / ayrıntı / karne / sözlük; 531 durum ×
TR/EN). Listeler denetçinin gerekçesiyle (aşağıda), düzenekten bilerek daha geniş:

VAGUE_HARD   olasılık/kaçamak sözcükleri ve boş önem sözleri; sayı olsa da yasak (K49, CAPSULE §8:
             gözlem; "muhtemelen"/"olabilir" bir tahmin ya da tavsiye tonu taşır).
VAGUE_SOFT   yoğunluk sıfatları; cümlede sayı yoksa yasak (somutluk K49), sayı varsa bilgi.
ADVICE       al/sat/fırsat/öneri/emir kipleri ve "gerekir/should/consider" gibi yönlendirmeler.
INTERNAL     ürünün iç kavram adları (CONCEPTS: olgu, bulgu, kapsül, kademe, aile, teyit, dedektör,
             sinyal, skor, güven puanı, replika, otorite, mühür, bar, karne-dışı "ledger"...).
REPEAT       aynı ekranda (başlık + ayrıntı + karne + sözlük) normalleştirilmiş aynı cümle iki kez
             (≥ 2 sözcük); cümle sınırı . ! ? ; ve " · ".
Türkçe küçük harf: I→ı, İ→i. Sözcük sınırı: harf olmayan karakter. TR kökleri ek alır; ekli biçim
listesi yerine "kök + Türkçe harfler" eşlenir ve bilinen başka sözcükler (barış, bariyer değil...)
ayrıca dışlanır.
"""
import json
import re
import sys
from collections import Counter, defaultdict

L = "a-zçğıöşüâîû"


def lower(s, lang):
    if lang == "tr":
        s = s.replace("I", "ı").replace("İ", "i")
    return s.lower()


def tr_root(root, excl=()):
    """root + any Turkish suffix letters, word-bounded; excluded whole words removed later."""
    return (re.compile(rf"(?<![{L}]){root}[{L}]*(?![{L}])"), set(excl))


def word(w):
    return (re.compile(rf"(?<![{L}]){w}(?![{L}])"), set())


VAGUE_HARD = {
    "tr": [tr_root("muhtemel"), tr_root("olasılıkla"), word("büyük ihtimalle"), word("belki"),
           word("galiba"), word("sanki"), tr_root("görünüyor"), word("olabilir"), word("olabilirdi"),
           word("önemli bir"), tr_root("kritik"), word("kilit"), word("dikkat çekici"),
           word("kayda değer"), word("hatırı sayılır"), word("güçlü sinyal"), word("güçlü bir sinyal"),
           word("teknik görünüm"), word("anlamlı bir"), word("belirgin bir")],
    "en": [word("likely"), word("unlikely"), word("probably"), word("possibly"), word("perhaps"),
           word("maybe"), word("may"), word("might"), word("could"), word("seems?"), word("appears? to"),
           word("important"), word("critical"), word("crucial"), word("key level"), word("notable"),
           word("noteworthy"), word("considerable"), word("meaningful"), word("strong signal"),
           word("technical picture")],
}
VAGUE_SOFT = {
    "tr": [tr_root("olağandışı"), word("büyük"), tr_root("önemli"), tr_root("güçlü"), word("sert"),
           word("keskin"), word("ciddi"), word("oldukça"), word("hayli"), word("epey"), word("yüksek"),
           word("düşük")],
    "en": [word("unusual(ly)?"), word("heavy"), word("big"), word("large"), word("significant(ly)?"),
           word("strong(ly)?"), word("sharp(ly)?"), word("major"), word("substantial(ly)?"),
           word("quite"), word("rather"), word("high"), word("low")],
}
ADVICE = {
    "tr": [word("al"), word("alın"), word("alınız"), tr_root("almalı"), tr_root("alabilir"),
           word("sat"), word("satın"), word("satınız"), tr_root("satmalı"), tr_root("satabilir"),
           tr_root("fırsat"), tr_root("tavsiye"), tr_root("öneri"), tr_root("öner"),
           word("hedef fiyat"), tr_root("kaçırma"), word("pozisyon aç[{L}]*".format(L=L)),
           tr_root("zarar kes"), word("stop"), word("kâr al[{L}]*".format(L=L)), tr_root("yapmalı"),
           tr_root("gerekir"), tr_root("gerekiyor"), word("dikkat edin"), word("dikkat et"),
           tr_root("değerlendir", ("değerlendirildi",)), tr_root("beklemeli"), tr_root("izleyin"),
           tr_root("takip edin")],
    "en": [word("buy(ing)?"), word("sell(ing)?"), tr_root("opportunit"), word("should"),
           tr_root("recommend"), word("advice"), tr_root("advis"), word("target price"),
           word("don'?t miss"), word("do not miss"), word("stop.?loss"), word("take profit"),
           word("go (long|short)"), word("consider"), word("must"), word("ought"), word("watch out"),
           word("you need")],
}
INTERNAL = {
    "tr": [tr_root("bar", ("barış", "barışı", "bariyer", "barem")), tr_root("teyit"),
           tr_root("aile", ("aileler",)), tr_root("dedektör"), tr_root("olgu", ("olgun", "olgunlaştı")),
           tr_root("bulgu", ("bulgur",)), tr_root("kademe"), tr_root("sinyal"), tr_root("skor"),
           tr_root("kapsül"), tr_root("replika"), tr_root("otorite"), tr_root("mühür"),
           tr_root("güven puan"), tr_root("önem puan"), tr_root("aday", ("adaylık",)),
           tr_root("kesinleş"), tr_root("tier"), tr_root("barrier"), tr_root("bariyer")],
    "en": [word("bars?"), tr_root("confirm"), tr_root("famil"), tr_root("detector"), word("facts?"),
           tr_root("finding"), word("tiers?"), word("signals?"), tr_root("score"), tr_root("capsule"),
           tr_root("replica"), tr_root("authority"), word("seal(ed)?"), word("confidence"),
           word("importance"), tr_root("candidate"), word("ledger"), word("barrier")],
}


def hits(patterns, text):
    out = []
    for rx, excl in patterns:
        for m in rx.finditer(text):
            if m.group(0) not in excl:
                out.append(m.group(0))
    return out


def sentences(t):
    for part in re.split(r"[.!?;]\s+|\s·\s|[.!?]$", t):
        p = part.strip()
        if p:
            yield p


def norm(s):
    return " ".join(re.findall(rf"[{L}0-9]+", s))


def scan(path):
    d = json.load(open(path))
    sym = re.compile(r"SYN\d+")
    count = Counter()
    ex = defaultdict(list)
    for row in d["rows"]:
        for lang in ("tr", "en"):
            s = row[lang]
            surfaces = [("headline", s["headline"]), ("detail", s["detail"]), ("ledger", s["ledger"])]
            surfaces += [("glossary", g["term"] + ": " + g["text"]) for g in s.get("glossary", [])]
            seen = Counter()
            for surf, raw in surfaces:
                t = lower(sym.sub("", raw), lang)
                for sent in sentences(t):
                    has_num = bool(re.search(r"\d", sent))
                    for cat, pats in (("vague_hard", VAGUE_HARD), ("advice", ADVICE), ("internal", INTERNAL)):
                        for h in hits(pats[lang], sent):
                            count[cat] += 1
                            ex[(cat, lang, h)].append(f"{row['id']} {surf}: {sent[:120]}")
                    for h in hits(VAGUE_SOFT[lang], sent):
                        cat = "soft_info" if has_num else "vague_soft"
                        count[cat] += 1
                        ex[(cat, lang, h)].append(f"{row['id']} {surf}: {sent[:120]}")
                    n = norm(sent)
                    if len(n.split()) >= 2:
                        seen[n] += 1
            for n, c in seen.items():
                if c > 1:
                    count["repeat"] += 1
                    ex[("repeat", lang, n[:40])].append(row["id"])
    print(f"== {d['language']}: {len(d['rows'])} rows x 2 langs")
    for cat in ("vague_hard", "vague_soft", "advice", "internal", "repeat", "soft_info"):
        print(f"   {cat:11s} {count[cat]}")
    for (cat, lang, h), v in sorted(ex.items(), key=lambda kv: -len(kv[1])):
        if cat == "soft_info":
            continue
        print(f"   [{cat} {lang} '{h}'] x{len(v)} e.g. {v[0]}")
    soft = Counter((lang, h) for (cat, lang, h), v in ex.items() if cat == "soft_info" for _ in v)
    print("   soft_info (word with a number in the sentence; allowed):", dict(soft.most_common(8)))
    return count


for p in sys.argv[1:]:
    scan(p)
