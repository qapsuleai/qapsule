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
    "motion": ["m_same", "m_where"],
    "motion_final": ["m_where"],
    # The glance copy (SPEC §G3a): the same face at 0.5x (arm's-length / one-second proxy,
    # RESEARCH F10), same questions.
    "face_half": ["dir", "kind", "valid", "when", "trust", "advice"],
    "face_visitor_half": ["dir", "kind", "valid", "trust", "numbers", "advice"],  # validity control (RESEARCH U10): final frame only
}

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
    return out


def credit(qid: str, code: str | None, accepted: set[str]) -> float:
    """Score of one answer: 1 / 0 (t_first: weighted credit; SPEC §G6.1)."""
    if code is None or code == UNKNOWN:
        return 0.0
    if qid == "t_first":
        return T_FIRST_CREDIT.get(code, 0.0)
    return 1.0 if code in accepted else 0.0
