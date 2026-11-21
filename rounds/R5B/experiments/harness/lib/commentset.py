"""R5B/H: the M30 comment set, built from the engine at the fixed position.

Every draft follows PLAN.md §2.1 `CommentDraft` ({lang, text, cites, external}) plus who
writes it (`principal`), what it is about (`about`), its defects and the decision the
contract (PLAN.md §2.1) gives at each strictness. Cited ids and values come from the
reference authority's answers (`capsule` detail as the reader), never from a model.

Families (SPEC.md §7):
  truthful: marked, plain (no [n] marker), round1, nonum, dated, barcount, external_ok,
            guest_ok, licensed_ok (arkin may write a licensed value)
  broken:   fabricated, rounding_over, wrong_direction, unknown_id, advice, derived,
            no_cites, not_visible, licensed_value, licensed_external, redacted_value,
            redacted_external
"""
import datetime

LEVELS = ("cites", "cites_numbers", "per_sentence")
KIND_TR = {"breakout": "kırılım", "continuation": "eğilim", "extreme": "aşırılık",
           "flow": "akış", "reversal": "dönüş", "squeeze": "sıkışma"}
DIR_TR = {"up": "yukarı yönlü", "down": "aşağı yönlü", "neutral": "yönsüz"}
STAGE_TR = {"born": "yeni doğdu", "strengthened": "güçlendi", "weakened": "zayıfladı",
            "confirmed": "teyit edildi", "realized": "gerçekleşti",
            "invalidated": "geçersiz oldu", "expired": "süresi doldu", "retracted": "geri çekildi"}
# Fact kind prefix -> evidence keys (key, Turkish word, decimals shown).
KEYS = [
    ("breakout.range", [("level", "kırılan seviye", 2), ("close", "kapanış", 2)]),
    ("continuation.ma_stack", [("close", "kapanış", 2), ("sma20", "kısa ortalama", 2)]),
    ("extreme.rsi", [("value", "RSI", 2), ("prev", "bir önceki RSI", 2)]),
    ("squeeze.bandwidth", [("upper", "üst bant", 2), ("lower", "alt bant", 2)]),
    ("reversal.rsi_divergence", [("p2_price", "ikinci uç fiyatı", 2), ("p2_rsi", "ikinci uçtaki RSI", 2)]),
    ("flow.volume_surge", [("ratio", "hacim oranı", 2)]),
    ("flow.institutional", [("net", "kurumsal net akış", 0), ("avg5", "beş günlük ortalama", 0)]),
]
MONTHS = ["Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara"]
CHECK = {"unknown_id": "cites", "no_cites": "cites", "not_visible": "license",
         "licensed_value": "license", "licensed_external": "license", "redacted_value": "license",
         "redacted_external": "license", "advice": "advice", "fabricated": "number",
         "rounding_over": "number", "derived": "number", "uncited_true_number": "number",
         "no_marker": "sentence"}
NUMBER_LEVELS = ("cites_numbers", "per_sentence")
BROKEN = {"fabricated", "rounding_over", "wrong_direction", "unknown_id", "advice", "derived",
          "no_cites", "not_visible", "licensed_value", "licensed_external", "redacted_value",
          "redacted_external"}


def tr(v, d):
    s = f"{abs(v):,.{d}f}".replace(",", "_").replace(".", ",").replace("_", ".")
    return ("-" if v < 0 and float(f"{abs(v):.{d}f}") != 0 else "") + s


def cap(w):
    """Turkish sentence case for the first letter (i -> İ)."""
    return ("İ" + w[1:]) if w[:1] == "i" else (w[:1].upper() + w[1:])


def keys_for(fact_kind):
    for p, ks in KEYS:
        if fact_kind.startswith(p):
            return ks
    return []


def contract(defects):
    out = {}
    for lv in LEVELS:
        checks = set()
        for d in defects:
            c = CHECK.get(d)
            if c is None:
                continue
            if c == "number" and lv not in NUMBER_LEVELS:
                continue
            if c == "sentence" and lv != "per_sentence":
                continue
            checks.add(c)
        # A licensed number is also absent from the reader's values: at the number levels
        # the number check may fire too (both are a right reason; license must be there).
        if checks & {"license"} and lv in NUMBER_LEVELS and set(defects) & {
                "licensed_value", "licensed_external", "redacted_value", "redacted_external"}:
            checks.add("number")
        out[lv] = {"decision": "reject" if checks else "accept", "checks": sorted(checks),
                   "must": sorted(checks - {"number"}) if "license" in checks else sorted(checks)}
    return out


def date_tr(t_ms):
    d = datetime.datetime.fromtimestamp(t_ms / 1000, datetime.timezone(datetime.timedelta(hours=3)))
    return f"{d.day} {MONTHS[d.month - 1]}"


class Src:
    def __init__(self, e, who, cid):
        self.who = who
        d = e.detail(who, cid)
        self.item = d["item"]
        self.cid = cid
        self.sym = self.item["instrument"]
        self.kind = self.item["kind"]
        self.dir = self.item["direction"]
        self.stage = self.item["stage"]
        hist = d["story"]["history"]
        born = set(hist[0]["facts"])
        facts = [f for f in d["facts"] if f["id"] in born]
        self.fact = facts[0] if facts else None
        self.born_t = hist[0]["t"]
        self.born_bar = self.item["born_bar"]
        self.vals = []
        if self.fact:
            ev = self.fact.get("evidence") or {}
            for k, w, dec in keys_for(self.fact["kind"]):
                if k in ev:
                    self.vals.append((k, w, dec, ev[k]))

    def head(self, direction=None):
        dw = DIR_TR[direction or self.dir]
        return f"{self.sym}'de {dw} {KIND_TR[self.kind]} hikâyesi {STAGE_TR.get(self.stage, self.stage)}"

    def cites(self):
        return [self.cid, self.fact["id"]] if self.fact else [self.cid]


def text(head, nums, marked):
    m = " [1]" if marked else ""
    parts = [f"{head}{m}."]
    for w, s in nums:
        parts.append(f"{cap(w)} {s}{m}.")
    return " ".join(parts)


def build(e):
    items = []

    def add(fam, principal, src, txt, cites, defects, external=None, note=None):
        d = {"id": f"cm{len(items) + 1:03d}", "family": fam, "principal": principal,
             "about": {"capsule": src.cid}, "draft": {"lang": "tr", "text": txt, "cites": cites,
                                                      "external": external or []},
             "defects": defects, "broken": bool(set(defects) & BROKEN),
             "contract": contract(defects)}
        if note:
            d["note"] = note
        items.append(d)

    last_final = e.last_final_bar("arkin")
    full = e.capsules("arkin")
    open_ids = sorted(c["id"] for c in e.capsules("arkin", open=True))
    # A few closed public stories too (first by id of each kind, instruments the guest reads).
    extra = []
    for kind in ("breakout", "continuation", "extreme", "reversal"):
        cands = sorted(c["id"] for c in full if c["kind"] == kind and c["id"] not in open_ids
                       and c["instrument"] in ("SYN01", "SYN04", "SYN08"))
        extra += cands[:1]
    srcs = [Src(e, "arkin", cid) for cid in open_ids + extra]
    srcs = [s for s in srcs if s.vals]

    for s in srcs:
        k1, w1, d1, v1 = s.vals[0]
        nums = [(w, tr(v, dec)) for _, w, dec, v in s.vals]
        P = "claude"
        add("marked", P, s, text(s.head(), nums, True), s.cites(), ["none"])
        add("plain", P, s, text(s.head(), nums, False), s.cites(), ["no_marker"])
        if d1 >= 2:
            add("round1", P, s, text(s.head(), [(w1, tr(v1, 1))], True), s.cites(), ["none"])
        add("nonum", P, s, f"{s.head()} [1].", s.cites(), ["none"])
        add("dated", P, s, f"{s.head()}; hikâye {date_tr(s.born_t)} tarihli barda doğdu [1].",
            s.cites(), ["none"], note="tarih kalıbı sayı sayılmaz (PLAN §2.1)")
        add("barcount", P, s, f"{s.head()}; hikâye {last_final - s.born_bar + 1} bar önce doğdu [1].",
            s.cites(), ["uncited_true_number"],
            note="doğru ama kanıtta olmayan sayı: sözleşme reddeder (yanlış ret ölçümü)")
        fab = v1 * 1.07 if v1 else 1.5
        add("fabricated", P, s, text(s.head(), [(w1, tr(fab, d1))], True), s.cites(), ["fabricated"])
        over = round(v1, d1) + (10 ** -d1 if d1 else 1)
        add("rounding_over", P, s, text(s.head(), [(w1, tr(over, d1))], True), s.cites(), ["rounding_over"],
            note=f"kaynak {v1!r}; yazılan basamağa yuvarlanınca eşleşmez")
        if s.dir in ("up", "down"):
            other = "down" if s.dir == "up" else "up"
            add("wrong_direction", P, s, text(s.head(other), [(w1, tr(v1, d1))], True), s.cites(),
                ["wrong_direction"], note="sözleşmenin denetimlerinde yön yok: yakalanmaz (bilinen boşluk)")
        fake = f"cap.{s.sym}.{s.kind}.{s.dir}.{s.born_bar + 1000}"
        add("unknown_id", P, s, text(s.head(), [(w1, tr(v1, d1))], True), [fake], ["unknown_id"])
        add("advice", P, s, text(s.head(), [(w1, tr(v1, d1))], True) + " Bu seviyeden alım fırsatı var [1].",
            s.cites(), ["advice"])
        if len(s.vals) >= 2:
            _, w2, d2, v2 = s.vals[1]
            dd = max(d1, d2)
            add("derived", P, s, f"{s.head()} [1]. {cap(w1)} ile {w2} arasındaki fark {tr(v1 - v2, dd)} [1].",
                s.cites(), ["derived"])
        add("no_cites", P, s, text(s.head(), [(w1, tr(v1, d1))], False), [], ["no_cites"])

    # arkin may write a licensed value (entitled), and an external quote with its own number.
    flows = sorted(c["id"] for c in full if c["kind"] == "flow" and c.get("trust") == "server")
    for cid in flows[-2:]:
        s = Src(e, "arkin", cid)
        if s.vals:
            _, w1, d1, v1 = s.vals[0]
            add("licensed_ok", "claude", s, text(s.head(), [(w1, tr(v1, d1))], True), s.cites(), ["none"],
                note="arkin akışa yetkili: lisanslı sayı yazılabilir")
    for s in srcs[:4]:
        q = "Şirketin satışları yüzde 12,5 arttı."
        add("external_ok", "claude", s,
            f"{s.head()} [1]. Dış kaynağa göre satışlar yüzde 12,5 arttı [1].", s.cites(), ["none"],
            external=[{"source": "Matriks haber (kurgu)", "quote": q, "values": [12.5]}])

    # The guest (claude-guest acts for guest): open view, bars of SYN01..SYN09 only.
    gcaps = e.capsules("guest")
    gseen = {c["id"] for c in gcaps}
    gopen = sorted(c["id"] for c in e.capsules("guest", open=True))
    gsrc = []
    for cid in gopen:
        s = Src(e, "guest", cid)
        if s.vals and not s.item.get("redacted"):
            gsrc.append(s)
    for s in gsrc[:5]:
        _, w1, d1, v1 = s.vals[0]
        add("guest_ok", "claude-guest", s, text(s.head(), [(w1, tr(v1, d1))], True), s.cites(), ["none"])
    hidden_flow = [cid for cid in flows if cid not in gseen]
    # not_visible: the guest cites a licensed story (its id exists, the guest cannot see it).
    anchor = {}
    for sym in ("SYN03", "SYN07"):
        vis = sorted((c for c in gcaps if c["instrument"] == sym), key=lambda c: (c["born_bar"], c["id"]))
        anchor[sym] = Src(e, "guest", vis[-1]["id"])
    for cid in hidden_flow[:3] + hidden_flow[-3:]:
        s = Src(e, "arkin", cid)
        a = anchor[s.sym]
        add("not_visible", "claude-guest", a, f"{a.head()} [1]. Kurumsal akış hikâyesi de var [2].",
            [a.cid, cid], ["not_visible"])
    for cid in hidden_flow[:4] + hidden_flow[-4:]:
        s = Src(e, "arkin", cid)
        if not s.vals:
            continue
        _, w1, d1, v1 = s.vals[0]
        a = anchor[s.sym]
        add("licensed_value", "claude-guest", a, f"{a.head()} [1]. {cap(w1)} {tr(v1, d1)} [1].",
            a.cites(), ["licensed_value"])
        add("licensed_external", "claude-guest", a,
            f"{a.head()} [1]. Bir kurum raporuna göre {w1} {tr(v1, d1)} [1].", a.cites(), ["licensed_external"],
            external=[{"source": "Kurum raporu (kurgu)", "quote": f"{w1} {tr(v1, d1)}", "values": [v1]}],
            note="lisanslı değer 'dış kaynak' diye işaretlenmiş: yine lisans reddi")
    # redacted: the guest sees the story (open world) but has no bar entitlement there.
    red = sorted(c["id"] for c in gcaps if c.get("redacted") and c["id"] in
                 {x["id"] for x in e.capsules("arkin", open=True)})
    red += sorted(c["id"] for c in gcaps if c.get("redacted") and c["kind"] == "breakout")[:4]
    for cid in red:
        s_full = Src(e, "arkin", cid)
        s_g = Src(e, "guest", cid)
        if not s_full.vals:
            continue
        _, w1, d1, v1 = s_full.vals[0]
        add("redacted_value", "claude-guest", s_g, f"{s_g.head()} [1]. {cap(w1)} {tr(v1, d1)} [1].",
            [cid, s_full.fact["id"]], ["redacted_value"])
        add("redacted_external", "claude-guest", s_g,
            f"{s_g.head()} [1]. Bir haber sitesine göre {w1} {tr(v1, d1)} [1].", [cid, s_full.fact["id"]],
            ["redacted_external"],
            external=[{"source": "Haber sitesi (kurgu)", "quote": f"{w1} {tr(v1, d1)}", "values": [v1]}])

    summary = {"total": len(items),
               "broken": sum(1 for i in items if i["broken"]),
               "truthful": sum(1 for i in items if not i["broken"]),
               "by_family": {}}
    for i in items:
        summary["by_family"][i["family"]] = summary["by_family"].get(i["family"], 0) + 1
    return {"levels": list(LEVELS), "summary": summary, "items": items}
