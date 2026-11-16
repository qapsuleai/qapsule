"""Engine-side slot builder for the `llmslot` M2 candidate (deterministic, no LLM).

Turns one `SentenceInput` (as exported by `harness m2 inputs`) into the typed slots the
language model is allowed to use. Every number the model may write is pre-formatted here
(TR and EN); the verifier (`verify.py`) accepts only numbers equal to one of these slot
values. A redacted (licensed, K33) situation gets no numbers at all.

Pure function of the input: same input -> same slots (byte-equal JSON).
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# number formatting
# ---------------------------------------------------------------------------


def _decimals(v: float) -> int:
    """Smallest number of decimals (<= 4) that represents v exactly enough."""
    for k in range(0, 5):
        if abs(round(v, k) - v) < 1e-9:
            return k
    return 4


def fmt(v: float, lang: str, k: int | None = None) -> str:
    """TR: 1.345.000 / 196,96 ; EN: 1,345,000 / 196.96 (sign dropped: abs value)."""
    v = abs(v)
    if k is None:
        k = _decimals(v)
    s = f"{v:,.{k}f}"  # EN style
    if lang == "tr":
        s = s.replace(",", "\u0000").replace(".", ",").replace("\u0000", ".")
    return s


# ---------------------------------------------------------------------------
# vocabulary the engine hands to the model (labels, not free text)
# ---------------------------------------------------------------------------

KIND = {
    "continuation": ("trend devamı", "trend continuation"),
    "reversal": ("dönüş sinyali (RSI uyumsuzluğu)", "reversal signal (RSI divergence)"),
    "breakout": ("kırılım", "breakout"),
    "squeeze": ("sıkışma (Bollinger bant daralması)", "squeeze (Bollinger band narrowing)"),
    "extreme": ("aşırılık (RSI aşırı alım / aşırı satım)", "extreme (RSI overbought / oversold)"),
    "flow": ("kurumsal para akışı", "institutional money flow"),
}

# evidence key labels per kind: key -> (tr, en)
LABELS = {
    "continuation": {
        "close": ("kapanış", "close"),
        "fast": ("kısa vadeli ortalama", "short-term moving average"),
        "slow": ("uzun vadeli ortalama", "long-term moving average"),
    },
    "reversal": {
        "price": ("sinyal barındaki fiyat", "price at the signal bar"),
        "rsi": ("RSI (son uç noktada)", "RSI (at the latest swing)"),
        "prev_rsi": ("RSI (önceki uç noktada)", "RSI (at the previous swing)"),
        "close": ("kapanış", "close"),
    },
    "breakout": {
        "level": ("aralık sınırı (kırılan seviye)", "range boundary (the level broken)"),
        "close": ("kapanış", "close"),
        "atr": ("ATR (ortalama gerçek aralık)", "ATR (average true range)"),
    },
    "squeeze": {
        "bandwidth": ("Bollinger bant genişliği", "Bollinger bandwidth"),
        "threshold": ("sıkışma eşiği", "squeeze threshold"),
    },
    "extreme": {
        "level": ("RSI eşiği", "RSI threshold"),
        "prev": ("önceki bar RSI", "previous bar RSI"),
        "value": ("RSI", "RSI"),
        "rsi": ("RSI", "RSI"),
    },
    "flow": {
        "net": ("kurumsal net akış", "institutional net flow"),
        "avg5": ("akışın yakın dönem ortalaması", "recent average of the flow"),
    },
    "volume_surge": {
        "volume": ("hacim", "volume"),
        "avg20": ("hacim ortalaması", "average volume"),
    },
}

STAGE = {
    "born": ("yeni oluştu", "newly formed"),
    "confirmed": ("teyit edildi", "confirmed"),
    "weakened": ("zayıfladı", "weakened"),
    "realized": ("gerçekleşti (beklenen sonuç geldi)", "realized (the expected outcome came)"),
    "invalidated": ("geçersiz oldu (beklenen sonuç tutmadı)", "invalidated (the expected outcome failed)"),
    "expired": ("süresi doldu (ufuk içinde sonuç gelmedi)", "expired (no outcome within its horizon)"),
}

REASON = {
    "new_story": ("sinyal oluştu", "the signal appeared"),
    "new_family": ("hacim artışı sinyali teyit etti", "a volume surge confirmed the signal"),
    "opposite_fact": ("ters yönde bir sinyal geldi", "an opposite-direction signal appeared"),
    "state_ended": ("durum sona erdi (koşul artık sağlanmıyor)", "the state ended (its condition no longer holds)"),
    "outcome_hit": ("beklenen sonuç geldi", "the expected outcome came"),
    "outcome_miss": ("beklenen sonuç tutmadı", "the expected outcome failed"),
    "horizon": ("ufuk süresi doldu", "its horizon ran out"),
}

OP = {"gt": ("üstünde", "above"), "lt": ("altında", "below")}

TIME_CLASS_DOC = {
    "event": "event: something happened at a past bar -> past tense + when",
    "state_holding": "state that holds now -> present tense",
    "state_ended": "state that held but has ended -> past tense + when + 'no longer'",
    "expectation": "expectation -> conditional (if ...)",
}


def _label(kind: str, key: str, i: int) -> str:
    table = LABELS.get(kind, {})
    if key in table:
        return table[key][i]
    for t in LABELS.values():
        if key in t:
            return t[key][i]
    return key


def validity(inp: dict) -> str:
    """yes / no / unknown: closed stages -> no; else from still_holds."""
    if inp["story"]["stage"] in ("invalidated", "expired"):
        return "no"
    h = inp["still_holds"]
    return "unknown" if h is None else ("yes" if h else "no")


def time_class(inp: dict) -> str:
    t = inp["story"]["time"]
    if t == "event":
        return "event"
    if t == "expectation":
        return "expectation"
    return "state_holding" if validity(inp) == "yes" else "state_ended"


def time_phrases(b: int, redacted: bool) -> dict:
    """Allowed phrases for 'when the lead fact happened' (bars ago b, daily bars)."""
    if b == 0:
        return {"tr": ["bugün"], "en": ["today"]}
    if b == 1:
        if redacted:
            return {"tr": ["dün"], "en": ["yesterday"]}
        return {"tr": ["dün", "1 bar önce"], "en": ["yesterday", "1 bar ago"]}
    if redacted:
        # no digits and no number words allowed; daily bars, 5 bars = one trading week
        if b <= 5:
            return {"tr": ["bir hafta önce"], "en": ["a week ago"]}
        return {"tr": ["bir aydan uzun süre önce"], "en": ["over a month ago"]}
    return {"tr": [f"{b} bar önce"], "en": [f"{b} bars ago"]}


def build(inp: dict) -> dict:
    """Slots of one input. Keys are stable and sorted on dump."""
    st = inp["story"]
    lead = inp["lead"]
    latest = inp["latest"]
    kind = st["kind"]
    red = bool(inp["redacted"])
    b = inp["now_bar"] - lead["bar"]
    since_last = inp["now_bar"] - st["last_bar"]
    last = st["history"][-1]
    cond = (lead.get("meaning") or {}).get("condition")
    tc = time_class(inp)

    slots: dict = {
        "id": inp["id"],
        "instrument": st["instrument"],
        "kind": kind,
        "kind_term": {"tr": KIND[kind][0], "en": KIND[kind][1]},
        "direction": st["direction"],
        "time_kind": st["time"],
        "time_class": tc,
        "stage": st["stage"],
        "stage_term": {"tr": STAGE[st["stage"]][0], "en": STAGE[st["stage"]][1]},
        "last_change": {
            "reason": {"tr": REASON[last["reason"]][0], "en": REASON[last["reason"]][1]},
            "bars_ago": since_last,
        },
        "bars_ago": b,
        "when_phrases": time_phrases(b, red),
        "valid_now": validity(inp),
        "redacted": red,
    }
    if latest["id"] != lead["id"]:
        det, _, edge = latest["kind"].rpartition(".")
        slots["latest_fact"] = {"detector": det, "edge": edge}

    numbers: list[dict] = []

    def add(name: str, v: float, tr_label: str, en_label: str, k: int | None = None) -> None:
        numbers.append(
            {"slot": name, "label": {"tr": tr_label, "en": en_label},
             "tr": fmt(v, "tr", k), "en": fmt(v, "en", k), "value": v}
        )

    if cond:
        subj, key = cond["subject"], cond["key"]
        c = {
            "subject": {"tr": _label(kind, subj, 0), "en": _label(kind, subj, 1)},
            "relation": {"tr": OP[cond["op"]][0], "en": OP[cond["op"]][1]},
            "key": {"tr": _label(kind, key, 0), "en": _label(kind, key, 1)},
        }
        if cond.get("within_bars") is not None:
            c["within_bars"] = None if red else cond["within_bars"]
            if red:
                c["within_bars_phrase"] = {"tr": "birkaç bar içinde", "en": "within a few bars"}
        slots["condition"] = c
        if inp["current"] is not None:
            slots["current_holds"] = inp["still_holds"]

    if not red:
        for k_, v in sorted(lead["evidence"].items()):
            add(f"lead.{k_}", v, _label(kind, k_, 0), _label(kind, k_, 1))
        if latest["id"] != lead["id"]:
            lk = latest["kind"].split(".")[1] if latest["kind"].count(".") >= 2 else kind
            for k_, v in sorted(latest["evidence"].items()):
                if lead["evidence"].get(k_) == v:
                    continue
                add(f"latest.{k_}", v, _label(lk if lk in LABELS else kind, k_, 0),
                    _label(lk if lk in LABELS else kind, k_, 1))
        if inp["current"] is not None:
            subj = cond["subject"] if cond else "value"
            add("current", inp["current"], "şu anki " + _label(kind, subj, 0),
                "current " + _label(kind, subj, 1))
        led = st["ledger_at_birth"]
        if led["n"] > 0:
            add("ledger.n", led["n"], "karne: geçmiş örnek sayısı", "track record: past cases")
            add("ledger.realized", led["realized"], "karne: gerçekleşen", "track record: realized")
            add("ledger.pct", 100.0 * led["realized"] / led["n"], "karne: gerçekleşme yüzdesi",
                "track record: realized percent", k=0)
            slots["ledger_small_sample"] = led["n"] < 30
        if cond and cond.get("within_bars") is not None:
            add("within_bars", cond["within_bars"], "koşulun süresi (bar)", "condition window (bars)")
        add("bars_ago", b, "kaç bar önce", "bars ago")
    slots["allowed_numbers"] = numbers
    return slots


def allowed_values(slots: dict) -> list[tuple[str, float]]:
    """(slot name, value) list used by the verifier; includes bars since last change."""
    v = [(n["slot"], float(n["value"])) for n in slots["allowed_numbers"]]
    if not slots["redacted"]:
        v.append(("last_change.bars_ago", float(slots["last_change"]["bars_ago"])))
    return v


def for_prompt(slots: dict) -> dict:
    """Slots as shown to the model (no raw `value` floats, only pre-formatted strings)."""
    out = {k: v for k, v in slots.items() if k not in ("allowed_numbers", "id")}
    out["allowed_numbers"] = [
        {"slot": n["slot"], "label": n["label"], "tr": n["tr"], "en": n["en"]}
        for n in slots["allowed_numbers"]
    ]
    return out
