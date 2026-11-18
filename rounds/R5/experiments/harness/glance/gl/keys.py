"""Question bank and answer keys (SPEC §G4).

Every key is derived from the engine's own capsule item (`qp replay` messages, the
`CapsuleItem` of `crates/qp-schema/src/protocol.rs`) and the page's feed rows; never from a
design. Deterministic: the same state record always gives the same key.
"""

from __future__ import annotations

import datetime as _dt

# Product threshold for "few cases" (crates/qp-grammar/src/depth.rs GLYPH_MIN_LEDGER_N).
MIN_LEDGER_N = 20
# Exchange time zone of the synthetic market (Europe/Istanbul, fixed UTC+3 since 2016).
TZ = _dt.timezone(_dt.timedelta(hours=3))

UNKNOWN = "unk"  # "bilmiyorum / okuyamıyorum" — always the last option, never shuffled

# Plain-language kind options (no internal terms; same wording as R4 S1b, extended).
KINDS = {
    "breakout": {"tr": "fiyat önemli bir seviyeyi geçti", "en": "the price crossed an important level"},
    "reversal": {"tr": "yön değişiyor olabilir", "en": "the direction may be turning"},
    "continuation": {"tr": "hareket sürüyor", "en": "the move is continuing"},
    "squeeze": {"tr": "fiyat sıkıştı, hareketsiz", "en": "the price is squeezed, barely moving"},
    "extreme": {"tr": "fiyat aşırı gitti", "en": "the price went unusually far"},
    "flow": {"tr": "büyük alım ya da satım (para girişi/çıkışı) var", "en": "large buying or selling (money in/out)"},
}

QUESTIONS = {
    # --- closed face / glance (M16) -------------------------------------------------
    "dir": {
        "tr": "En üstteki kapsülde ne oldu: yön?",
        "en": "In the top capsule, what happened: direction?",
        "options": [("up", {"tr": "yukarı", "en": "up"}), ("down", {"tr": "aşağı", "en": "down"}),
                    ("flat", {"tr": "yönsüz / yatay", "en": "no direction / sideways"})],
    },
    "kind": {
        "tr": "En üstteki kapsülde ne oldu: ne tür bir şey?",
        "en": "In the top capsule, what kind of thing happened?",
        "options": [(k, v) for k, v in KINDS.items()],
    },
    "valid": {
        "tr": "En üstteki kapsülün söylediği şey hâlâ geçerli mi?",
        "en": "Does what the top capsule says still hold?",
        "options": [("valid", {"tr": "evet, sürüyor", "en": "yes, it still holds"}),
                    ("no_longer", {"tr": "hayır, artık geçerli değil (ama kapanmadı)", "en": "no, it no longer holds (but not closed)"}),
                    ("realized", {"tr": "kapandı: gerçekleşti", "en": "closed: it played out"}),
                    ("failed", {"tr": "kapandı: tutmadı", "en": "closed: it did not play out"}),
                    ("expired", {"tr": "kapandı: süresi doldu ya da geri çekildi", "en": "closed: ran out of time or withdrawn"})],
    },
    "when": {
        "tr": "Bu en son ne zaman oldu (ya da değişti)?",
        "en": "When did this last happen (or change)?",
        "options": [("today", {"tr": "bugün", "en": "today"}), ("yesterday", {"tr": "dün", "en": "yesterday"}),
                    ("days", {"tr": "birkaç gün önce", "en": "a few days ago"}),
                    ("week", {"tr": "bir haftadan daha önce", "en": "more than a week ago"})],
    },
    "trust": {
        "tr": "Bu tür bir işaret geçmişte ne kadar tuttu?",
        "en": "How often did this kind of signal play out in the past?",
        "options": [("mostly", {"tr": "çoğunlukla tuttu", "en": "mostly played out"}),
                    ("half", {"tr": "yarı yarıya", "en": "about half the time"}),
                    ("rarely", {"tr": "çoğunlukla tutmadı", "en": "mostly did not"}),
                    ("few", {"tr": "az örnek var ya da hiç kayıt yok, bilinmiyor", "en": "few or no past cases, unknown"}),
                    ("hidden", {"tr": "bu bilgi bana gösterilmiyor", "en": "this is not shown to me"})],
    },
    "advice": {
        "tr": "Ekran sana ne yapmanı söylüyor?",
        "en": "What does the screen tell you to do?",
        "options": [("buy", {"tr": "al", "en": "buy"}), ("sell", {"tr": "sat", "en": "sell"}),
                    ("wait", {"tr": "bekle", "en": "wait"}),
                    ("nothing", {"tr": "hiçbir şey, yalnız bilgi veriyor", "en": "nothing, it only informs"})],
    },
    "numbers": {  # unentitled reader (K33, A32; M20 "neden rakam göremiyorum")
        "tr": "Kapsülde rakam (fiyat, değer) görüyor musun?",
        "en": "Do you see numbers (prices, values) in the capsule?",
        "options": [("yes", {"tr": "evet", "en": "yes"}),
                    ("no_license", {"tr": "hayır; ekran bunun yetkim/aboneliğim olmadığı için olduğunu söylüyor", "en": "no; the screen says it is because I lack the entitlement/subscription"}),
                    ("no_reason", {"tr": "hayır; nedenini söylemiyor", "en": "no; it does not say why"})],
    },
    # --- throttle (blurred, downscaled) ----------------------------------------------
    "t_first": {
        "tr": "Bu bulanık ekranda gözüne ilk çarpan öğe hangisi?",
        "en": "On this blurred screen, which element catches your eye first?",
        "options": [("mark", {"tr": "bir işaret / simge", "en": "a mark / symbol"}),
                    ("sentence", {"tr": "bir cümle ya da başlık", "en": "a sentence or headline"}),
                    ("number", {"tr": "bir sayı", "en": "a number"}),
                    ("chart", {"tr": "bir grafik çizgisi", "en": "a chart line"}),
                    ("chrome", {"tr": "menü, düğme ya da sekme", "en": "a menu, button or tab"}),
                    ("none", {"tr": "hiçbiri öne çıkmıyor", "en": "nothing stands out"})],
    },
    "t_dir": {
        "tr": "Bulanık hâliyle bile en üstteki öğenin yönünü seçebiliyor musun?",
        "en": "Even blurred, can you tell the top item's direction?",
        "options": [("up", {"tr": "yukarı", "en": "up"}), ("down", {"tr": "aşağı", "en": "down"}),
                    ("flat", {"tr": "yönsüz / yatay", "en": "no direction / sideways"})],
    },
    "t_valid": {
        "tr": "Bulanık hâliyle en üstteki öğe sürüyor mu, bitmiş mi?",
        "en": "Blurred, is the top item ongoing or finished?",
        "options": [("live", {"tr": "sürüyor", "en": "ongoing"}), ("ended", {"tr": "bitmiş / kapanmış ya da artık geçerli değil", "en": "finished / closed or no longer holds"})],
    },
    # --- feed (M16 feed screen) ------------------------------------------------------
    "f_live": {
        "tr": "Ekranda tamamen görünen kapsüllerden kaçı hâlâ sürüyor?",
        "en": "Of the capsules fully visible on screen, how many still hold?",
        "options": [("0", {"tr": "hiçbiri", "en": "none"}), ("1", {"tr": "bir", "en": "one"}),
                    ("2", {"tr": "iki", "en": "two"}), ("3+", {"tr": "üç ya da daha fazla", "en": "three or more"})],
    },
    "f_down": {
        "tr": "Ekranda tamamen görünenler arasında aşağı yönlü bir olay var mı?",
        "en": "Among the fully visible ones, is there a downward event?",
        "options": [("yes", {"tr": "evet", "en": "yes"}), ("no", {"tr": "hayır", "en": "no"})],
    },
    # --- evidence (M20) --------------------------------------------------------------
    "e_rests": {
        "tr": "Bu iddia neye dayanıyor?",
        "en": "What does this claim rest on?",
        "options": [("price", {"tr": "fiyatın kendisine ve fiyattan hesaplanan değerlere", "en": "the price and values computed from it"}),
                    ("volume", {"tr": "işlem hacmine", "en": "traded volume"}),
                    ("flow", {"tr": "para akışına (takas, kurum)", "en": "money flow (settlement, institutions)"}),
                    ("news", {"tr": "haberlere ya da bilançoya", "en": "news or fundamentals"}),
                    ("opinion", {"tr": "bir uzmanın görüşüne", "en": "an expert's opinion"})],
    },
    "e_who": {
        "tr": "Bunu kim doğruladı?",
        "en": "Who verified this?",
        "options": [("server", {"tr": "sunucu doğruladı", "en": "the server verified it"}),
                    ("device", {"tr": "bu cihazda yeniden sınandı", "en": "it was re-checked on this device"}),
                    ("none", {"tr": "kimse; doğrulanmadı", "en": "nobody; not verified"})],
    },
    # --- transition mid-frame (M21) ---------------------------------------------------
    "m_same": {
        "tr": "Sağdaki ara karede, soldaki ekranın en üstündeki kapsül nerede? Aynı nesne mi taşınıyor?",
        "en": "In the right-hand in-between frame, where is the top capsule of the left screen? Is the same object moving?",
        "options": [("moving", {"tr": "aynı nesne yer değiştiriyor / büyüyor", "en": "the same object is moving / growing"}),
                    ("cut", {"tr": "kayboldu, yerine başka bir şey geldi", "en": "it vanished, something else replaced it"})],
    },
    "m_where": {
        "tr": "Sağdaki karede, soldaki en üstteki kapsül (ya da ondan taşınan parçalar) çoğunlukla hangi harfli bölgede?",
        "en": "In the right-hand frame, in which lettered region is the left screen's top capsule (or the parts carried over from it) mostly?",
        "options": [("A", {"tr": "A (sol üst)", "en": "A (top left)"}), ("B", {"tr": "B (sağ üst)", "en": "B (top right)"}),
                    ("C", {"tr": "C (sol alt)", "en": "C (bottom left)"}), ("D", {"tr": "D (sağ alt)", "en": "D (bottom right)"})],
    },
    # (M21, SPEC Ek G8) position-based continuity: two in-between frames (I p = 0.25, II p = 0.5),
    # no "before" frame; numbered bands 1-4 (top to bottom) drawn by the harness; options lettered.
    "m_from": {
        "tr": "Bu iki kare aynı geçişin art arda iki anı (I önce, II biraz sonra). Açılan kapsül ekranın hangi numaralı şeridinden geliyor; geçiş nereden başlıyor?",
        "en": "These two frames are two consecutive moments of the same transition (I first, II a moment later). From which numbered band of the screen does the opening capsule come; where does the transition start?",
        "options": [("1", {"tr": "1. şerit (en üst)", "en": "band 1 (top)"}), ("2", {"tr": "2. şerit", "en": "band 2"}),
                    ("3", {"tr": "3. şerit", "en": "band 3"}), ("4", {"tr": "4. şerit (en alt)", "en": "band 4 (bottom)"}),
                    ("none", {"tr": "bir yerden gelmiyor; olduğu yerde beliriyor", "en": "it does not come from anywhere; it appears in place"})],
    },
    # --- state change on the feed (M21, SPEC Ek G6; written before any M21 reading) ---------
    # Numbered horizontal bands 1-4 (top to bottom) drawn by the harness; options lettered.
    "n_where": {
        "tr": "Bu ekranda az önce değişen (yeni çıkan ya da durumu yeni değişen) bir kapsül var mı? Varsa çoğunlukla hangi numaralı şeritte?",
        "en": "Is there a capsule on this screen that just changed (newly appeared, or whose state just changed)? If so, in which numbered band is it mostly?",
        "options": [("1", {"tr": "1. şerit (en üst)", "en": "band 1 (top)"}), ("2", {"tr": "2. şerit", "en": "band 2"}),
                    ("3", {"tr": "3. şerit", "en": "band 3"}), ("4", {"tr": "4. şerit (en alt)", "en": "band 4 (bottom)"}),
                    ("none", {"tr": "hayır; az önce değişen bir şey görmüyorum", "en": "no; I see nothing that just changed"})],
    },
    "n_where2": {
        "tr": "Sağdaki karede az önce değişen (yeni çıkan ya da durumu yeni değişen) bir kapsül var mı? Varsa çoğunlukla hangi numaralı şeritte?",
        "en": "In the right-hand frame, is there a capsule that just changed (newly appeared, or whose state just changed)? If so, in which numbered band is it mostly?",
        "options": [("1", {"tr": "1. şerit (en üst)", "en": "band 1 (top)"}), ("2", {"tr": "2. şerit", "en": "band 2"}),
                    ("3", {"tr": "3. şerit", "en": "band 3"}), ("4", {"tr": "4. şerit (en alt)", "en": "band 4 (bottom)"}),
                    ("none", {"tr": "hayır; değişen bir şey görmüyorum", "en": "no; I see nothing that changed"})],
    },
    "n_what": {
        "tr": "Soldaki ekrandan sağdakine ne oldu?",
        "en": "What happened from the left screen to the right one?",
        "options": [("born", {"tr": "yeni bir kapsül çıktı", "en": "a new capsule appeared"}),
                    ("state", {"tr": "var olan bir kapsülün durumu değişti", "en": "an existing capsule's state changed"}),
                    ("none", {"tr": "hiçbir şey değişmedi", "en": "nothing changed"})],
    },
    # --- story chart (M18; SPEC §G9 "M18 görevleri"; written before any M18 reading) --------
    # Keys from the engine's story list (PROTOCOL §23: id, born bar/time, closing, outcome)
    # and the visible bar range of the open scene; never from a design's drawing.
    # "Bu hissede (görünen dönemde) ne oldu": the visible window is set by the 5 / 50 / 200
    # rule, so the count is of the stories whose life meets the visible range (§23.1 rule).
    "c_recent": {
        "tr": "Grafikte görünen dönemde bu hissede kaç olay (hikâye) var?",
        "en": "How many events (stories) does this stock have in the period shown on the chart?",
        "options": [("0", {"tr": "hiç", "en": "none"}), ("1-9", {"tr": "1 ile 9 arası", "en": "1 to 9"}),
                    ("10-29", {"tr": "10 ile 29 arası", "en": "10 to 29"}),
                    ("30-99", {"tr": "30 ile 99 arası", "en": "30 to 99"}),
                    ("100+", {"tr": "100 ya da daha fazla", "en": "100 or more"})],
    },
    "c_held": {
        "tr": "Grafikte görünen, bitmiş (kapanmış) olayların kaçı tuttu (beklenen gerçekleşti)?",
        "en": "Of the finished (closed) events visible on the chart, how many played out?",
        "options": [("most", {"tr": "çoğu tuttu", "en": "most played out"}),
                    ("half", {"tr": "yaklaşık yarısı", "en": "about half"}),
                    ("few", {"tr": "çoğu tutmadı", "en": "most did not"}),
                    ("none_closed", {"tr": "grafikte bitmiş olay yok", "en": "there is no finished event on the chart"})],
    },
    "c_live": {
        # Regions are numbered (1-4), not lettered: the answer options are lettered.
        "tr": "Şu an hâlâ süren (canlı) olay grafikte hangi numaralı bölgede başladı? (Birden çok varsa herhangi biri.)",
        "en": "In which numbered region of the chart did the event that is still ongoing (live) start? (If there are several, any one.)",
        "options": [("A", {"tr": "1. bölge (en sol)", "en": "region 1 (leftmost)"}),
                    ("B", {"tr": "2. bölge", "en": "region 2"}),
                    ("C", {"tr": "3. bölge", "en": "region 3"}),
                    ("D", {"tr": "4. bölge (en sağ)", "en": "region 4 (rightmost)"}),
                    ("none", {"tr": "şu an süren olay yok", "en": "no event is ongoing now"})],
    },
    "c_evid": {
        "tr": "Vurgulanan (seçili) işaret neye dayanıyor; ne tür bir şey gösteriyor?",
        "en": "What does the highlighted (selected) mark rest on; what kind of thing does it show?",
        "options": [(k, v) for k, v in KINDS.items()],
    },
}

# Question sets per stimulus kind (SPEC §G4.1).
SETS = {
    "face": ["dir", "kind", "valid", "when", "trust", "advice"],
    "face_visitor": ["dir", "kind", "valid", "trust", "numbers", "advice"],
    "throttle": ["t_first", "t_dir", "t_valid"],
    "gray": ["dir", "valid"],  # "colour alone carries no meaning": same face, grey copy
    "feed": ["f_live", "f_down"],
    "evidence": ["e_rests", "e_who", "valid"],
    "evidence_visitor": ["numbers", "e_who"],
    # (M20, Ek G7a) the same questions on the capsule-depth shot (the primary measure).
    "cap_evidence": ["e_rests", "e_who", "valid"],
    "cap_evidence_visitor": ["numbers", "e_who"],
    "motion": ["m_same", "m_where"],
    # (Ek G6) the validity control asks both motion questions with the final frame.
    "motion_final": ["m_same", "m_where"],
    # (M21, Ek G6) a single frame of the feed half way through / just after a state change,
    # and the pair (before | half way).
    # (Ek G8) the position question on two in-between frames, and its final-frame control.
    "motion2": ["m_from"],
    "motion2_final": ["m_from"],
    "notice": ["n_where"],
    "notice_pair": ["n_where2", "n_what"],
    # The glance copy (SPEC §G3a): the same face at 0.5x (arm's-length / one-second proxy,
    # RESEARCH F10), same questions.
    "face_half": ["dir", "kind", "valid", "when", "trust", "advice"],
    "face_visitor_half": ["dir", "kind", "valid", "trust", "numbers", "advice"],  # validity control (RESEARCH U10): final frame only
    # M18 story chart at 5 / 50 / 200 stories (SPEC §G9): last three months, which held, the
    # live one (lettered columns over the plot), the selected mark's evidence kind.
    "chart": ["c_recent", "c_held", "c_live", "c_evid"],
}

# M18 (SPEC §G9): held share buckets as the trust question (>= 0.6 / 0.4-0.6 / < 0.4).
CHART_COLUMNS = "ABCD"
COLUMN_MARGIN = 0.10   # a birth within 10 % of a column width from a column line is ambiguous


def count_key(visible: list) -> str:
    """Bucket of the number of stories whose life meets the visible range."""
    n = len(visible)
    return "0" if n == 0 else "1-9" if n <= 9 else "10-29" if n <= 29 else "30-99" if n <= 99 else "100+"


def held_key(stories: list) -> str:
    closed = [s for s in stories if s.get("outcome") in ("realized", "invalidated", "expired")]
    if not closed:
        return "none_closed"
    rate = sum(1 for s in closed if s["outcome"] == "realized") / len(closed)
    return "most" if rate >= 0.6 else "half" if rate >= 0.4 else "few"


def live_keys(stories: list, rng: tuple[int, int]) -> set[str]:
    """Columns (A-D over the visible bar range) where a live story was born; a birth outside
    the range or near a column line is not scorable (empty set if no scorable live story but
    some live story exists)."""
    lo, hi = rng
    live = [s for s in stories if s.get("validity") != "closed" and not s.get("closed_bar")]
    if not live:
        return {"none"}
    out = set()
    span = max(1, hi - lo)
    for s in live:
        f = (int(s["born_bar"]) - lo) / span
        if not 0.0 <= f <= 1.0:
            continue
        c = min(3, int(f * 4))
        r = f * 4 - c
        if (c > 0 and r < COLUMN_MARGIN) or (c < 3 and r > 1 - COLUMN_MARGIN):
            continue
        out.add(CHART_COLUMNS[c])
    return out

# Throttle: which first-seen element counts as a correct hierarchy (SPEC §G3: K7, the mark
# is the fastest-read element; the sentence is accepted as second best with weight 0.5).
T_FIRST_CREDIT = {"mark": 1.0, "sentence": 0.5}


def chance(qid: str) -> float:
    """Chance level of one question: 1 / number of options (the 'unknown' option is a
    non-answer and is not counted; SPEC §G6.3)."""
    if qid == "t_first":
        n = len(QUESTIONS[qid]["options"])
        return sum(T_FIRST_CREDIT.values()) / n
    return 1.0 / len(QUESTIONS[qid]["options"])


def _day(t_ms: int) -> _dt.date:
    return _dt.datetime.fromtimestamp(t_ms / 1000, TZ).date()


def _bucket(days: int) -> str:
    if days <= 0:
        return "today"
    if days == 1:
        return "yesterday"
    if days <= 5:
        return "days"
    return "week"


def trading_days(a: _dt.date, b: _dt.date) -> int:
    """Weekdays after `a` up to and including `b` (b >= a)."""
    n, d = 0, a
    while d < b:
        d += _dt.timedelta(days=1)
        if d.weekday() < 5:
            n += 1
    return n


def when_keys(item_t: int, now_t: int) -> set[str]:
    """Accepted 'when' buckets: by calendar days and by trading days (both accepted when
    they differ — a Friday event read on Monday is 'yesterday' in trading days and 'a few
    days ago' in calendar days; SPEC §G4.2)."""
    a, b = _day(item_t), _day(now_t)
    if b < a:
        b = a
    return {_bucket((b - a).days), _bucket(trading_days(a, b))}


def trust_key(item: dict, redacted: bool) -> str:
    if redacted:
        return "hidden"
    led = item.get("ledger_at_birth") or {}
    n, r = int(led.get("n", 0)), int(led.get("realized", 0))
    if n < MIN_LEDGER_N:
        return "few"
    rate = r / n
    if rate >= 0.6:
        return "mostly"
    if rate >= 0.4:
        return "half"
    return "rarely"


def valid_key(item: dict) -> str:
    v = item["validity"]
    if v == "valid":
        return "valid"
    if v == "no_longer":
        return "no_longer"
    return {"realized": "realized", "invalidated": "failed"}.get(item["stage"], "expired")


def dir_key(item: dict) -> str:
    return {"up": "up", "down": "down"}.get(item["direction"], "flat")


def keys_for(qset: str, rec: dict) -> dict[str, set[str]]:
    """The accepted answer codes per question for one stimulus record (`states.json`)."""
    item = rec.get("item") or {}
    red = bool(rec.get("redacted"))
    out: dict[str, set[str]] = {}
    for q in SETS[qset]:
        if q in ("dir", "t_dir"):
            out[q] = {dir_key(item)}
        elif q == "kind":
            out[q] = {item["kind"]}
        elif q == "valid":
            out[q] = {valid_key(item)}
        elif q == "t_valid":
            out[q] = {"live" if item["validity"] == "valid" else "ended"}
        elif q == "when":
            out[q] = when_keys(int(item["t"]), int(rec["now_t"]))
        elif q == "trust":
            out[q] = {trust_key(item, red)}
        elif q == "advice":
            out[q] = {"nothing"}
        elif q == "numbers":
            out[q] = {"no_license"} if red else {"yes"}
        elif q == "t_first":
            out[q] = set(T_FIRST_CREDIT)
        elif q == "f_live":
            n = sum(1 for it in rec.get("visible", []) if it["validity"] == "valid")
            out[q] = {str(n) if n < 3 else "3+"}
        elif q == "f_down":
            out[q] = {"yes" if any(it["direction"] == "down" for it in rec.get("visible", [])) else "no"}
        elif q == "e_rests":
            out[q] = set(rec.get("families") or [])
        elif q == "e_who":
            out[q] = {"server" if item.get("trust") == "server" else "device"}
        elif q == "m_same":
            out[q] = {"moving" if rec.get("shared") else "cut"}
        elif q == "m_where":
            out[q] = {rec["region"]} if rec.get("region") else set()
        elif q == "c_recent":
            out[q] = {count_key(rec.get("stories_visible") or [])} if "stories_visible" in rec else set()
        elif q == "c_held":
            out[q] = {held_key(rec.get("stories_visible") or [])}
        elif q == "c_live":
            out[q] = live_keys(rec.get("stories") or [], tuple(rec["range"])) if rec.get("range") else set()
        elif q == "c_evid":
            out[q] = {item["kind"]} if item.get("kind") else set()
    return out


def credit(qid: str, code: str | None, accepted: set[str]) -> float:
    """Score of one answer: 1 / 0 (t_first: weighted credit; SPEC §G6.1)."""
    if code is None or code == UNKNOWN:
        return 0.0
    if qid == "t_first":
        return T_FIRST_CREDIT.get(code, 0.0)
    return 1.0 if code in accepted else 0.0
