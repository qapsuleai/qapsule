"""Deterministic slot verifier for the `llmslot` M2 candidate.

`check(slots, lang, sentence) -> list[str]` returns the rejection reasons (empty = accepted).
Rules (all surface checks, no parsing; written for this candidate, independently of the
harness code, though they necessarily encode the same K26/K27 rules):

 1. form: non-empty, one line, at most 2 sentences, <= 45 words, no markdown/quotes/brackets
    of the slot names, no advice words.
 2. numbers: every digit token (TR `1.345.000` / `196,96`; EN `1,345,000` / `196.96`) must
    equal round(v, k) for a slot value v (k = decimals written). Number words are not
    allowed at all (numbers are written with digits, copied from the slots).
 3. redacted: no digit, no number word, no `%`, and none of the licensed values.
 4. time expressions: "N bar(s) ago / N bar önce" must have N = bars_ago; any "N bar(s)"
    must have N in {bars_ago, bars since last change, within_bars}; "today/bugün" only if
    bars_ago or bars-since-last-change is 0, "yesterday/dün" only if one of them is 1.
    event and ended state must contain one of the engine's `when_phrases`.
 5. tense per time class: event -> past marker; holding state -> present marker and no
    "no longer"; ended state -> past marker + "no longer" (TR `artık … değil`);
    expectation -> conditional marker (TR `-sa/-se`, `eğer`; EN `if`, `unless`).
 6. kind term present (stem list per kind) so the reader can tell what happened.
"""

from __future__ import annotations

import re

TR_NUM_WORDS = {
    "sıfır", "iki", "üç", "dört", "beş", "altı", "yedi", "sekiz", "dokuz", "on", "yirmi",
    "otuz", "kırk", "elli", "altmış", "yetmiş", "seksen", "doksan", "yüz", "bin", "milyon",
    "milyar", "yarım", "çeyrek", "yüzde", "buçuk", "birinci", "ikinci", "üçüncü",
}
TR_NUM_STEMS = ("yetmiş", "altmış", "seksen", "doksan", "yirmi", "otuz", "elli", "sekiz",
                "dokuz", "milyon", "milyar", "yüzde", "dört", "sıfır")
EN_NUM_WORDS = {
    "zero", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen",
    "nineteen", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    "hundred", "thousand", "million", "billion", "half", "quarter", "dozen", "twice",
    "percent", "double", "triple", "first", "second", "third",
}

ADVICE = {
    "tr": ("tavsiye", "öneri", "almalı", "satmalı", "almanız", "satmanız", "fırsat", "kesin"),
    "en": ("should", "recommend", "advise", "opportunity", "must buy", "must sell", "guaranteed"),
}

KIND_STEMS = {
    "tr": {
        "continuation": ("trend", "devam"),
        "reversal": ("dönüş", "uyumsuzluk"),
        "breakout": ("kırılım", "kırdı", "kırıldı", "kırarak"),
        "squeeze": ("sıkışma", "sıkıştı", "daral"),
        "extreme": ("aşırı",),
        "flow": ("akış", "kurumsal"),
    },
    "en": {
        "continuation": ("trend", "continuation"),
        "reversal": ("reversal", "divergence"),
        "breakout": ("breakout", "broke"),
        "squeeze": ("squeeze", "narrow"),
        "extreme": ("overbought", "oversold", "extreme"),
        "flow": ("flow", "institutional"),
    },
}

TR_NOT_PAST = {"şimdi", "kendi", "gerçi", "hedefi", "kedi", "adı", "ardı", "idi", "di",
               "yedi", "seviyedi"}
TR_COND_EXCEPT = {"hisse", "borsa", "kasa", "masa", "kısa", "yasa", "bursa", "ise", "bile",
                  "kese", "pasa", "tasa"}
EN_NOT_PAST = {"need", "speed", "feed", "seed", "bed", "red", "shed", "indeed", "embed",
               "exceed", "proceed", "succeed", "breed", "hundred", "based"}
EN_IRREG_PAST = {"was", "were", "had", "did", "broke", "rose", "fell", "began", "became",
                 "went", "came", "held", "ran", "got", "made", "took", "left", "saw", "grew",
                 "sank", "swung", "struck", "met"}


def lower(s: str, lang: str) -> str:
    if lang == "tr":
        s = s.replace("I", "ı").replace("İ", "i")
    return s.lower()


def words(s: str, lang: str) -> list[str]:
    return [w for w in re.split(r"[^\w]+|[\d_]+", lower(s, lang)) if w and not w.isdigit()]


NUM_RE = re.compile(r"\d+(?:[.,]\d+)*")


def readings(tok: str) -> list[tuple[float, int]]:
    """(value, decimals) readings of a digit token, both TR and EN conventions."""
    seps = [(i, c) for i, c in enumerate(tok) if c in ".,"]
    digits = re.sub(r"\D", "", tok)
    out: list[tuple[float, int]] = []
    if not seps:
        out.append((float(tok), 0))
    elif len(seps) == 1:
        i, _ = seps[0]
        frac = tok[i + 1:]
        out.append((float(tok[:i] + "." + frac), len(frac)))
        if len(frac) == 3:
            out.append((float(digits), 0))
    else:
        last_i, last_c = seps[-1]
        if all(c == last_c for _, c in seps):
            out.append((float(digits), 0))
        else:
            intpart = re.sub(r"\D", "", tok[:last_i])
            frac = tok[last_i + 1:]
            out.append((float(intpart + "." + frac), len(frac)))
    return out


def _round(x: float, k: int) -> float:
    # same rule as harness `io::round` (half away from zero on the scaled value)
    m = 10.0 ** k
    y = abs(x) * m
    r = float(int(y + 0.5)) / m
    return r if x >= 0 else -r


def match(tok: str, values: list[tuple[str, float]]) -> str | None:
    for v, k in readings(tok):
        for name, x in values:
            for c in (x, abs(x)):
                if abs(_round(c, k) - v) <= 1e-9 * max(abs(v), 1.0):
                    return name
    return None


def _num_words(s: str, lang: str) -> list[str]:
    ws = words(s, lang)
    if lang == "tr":
        return [w for w in ws if w in TR_NUM_WORDS or any(w.startswith(p) for p in TR_NUM_STEMS)]
    return [w for w in ws if w in EN_NUM_WORDS]


def _past(ws: list[str], lang: str) -> bool:
    if lang == "en":
        return any((len(w) >= 4 and w.endswith("ed") and w not in EN_NOT_PAST) or w in EN_IRREG_PAST
                   for w in ws)
    for w in ws:
        if len(w) < 3 or w in TR_NOT_PAST:
            continue
        if re.search(r"m[ıiuü]ş", w) or re.search(r"[dt][ıiuü][ğ]", w):
            return True
        if re.search(r"[dt][ıiuü](lar|ler|k|m|n|nız|niz|ğı|ği)?$", w) and len(w) >= 4:
            return True
    return False


def _present(ws: list[str], text: str, lang: str) -> bool:
    if lang == "en":
        return any(w in {"is", "are", "remains", "remain", "stays", "holds", "still", "now",
                         "currently", "sits", "trades"} for w in ws)
    if any(w in {"şimdi", "hâlâ", "hala", "halen"} for w in ws) or re.search(r"\bşu an", text):
        return True
    for w in ws:
        if "yor" in w and not re.search(r"yor(du|muş)", w):
            return True
        if "makta" in w or "mekte" in w:
            return True
        if len(w) > 4 and re.search(r"[dt][ıiuü]r(lar|ler)?$", w):
            return True
    return False


def _no_longer(ws: list[str], text: str, lang: str) -> bool:
    if lang == "en":
        return "no longer" in text or ("anymore" in ws and "not" in ws)
    neg = any(w.startswith("değil") or re.search(r"m[ıiuü]yor|ma[zd]|me[zd]", w) for w in ws)
    return "artık" in ws and neg


def _conditional(ws: list[str], lang: str) -> bool:
    if lang == "en":
        return any(w in {"if", "unless", "provided"} for w in ws)
    if any(w in {"eğer", "takdirde", "halinde", "koşuluyla"} for w in ws):
        return True
    return any(len(w) >= 4 and w not in TR_COND_EXCEPT and (w.endswith("sa") or w.endswith("se"))
               for w in ws)


def check(slots: dict, lang: str, sentence: str, values: list[tuple[str, float]]) -> list[str]:
    """Rejection reasons for one sentence (empty list = accepted)."""
    errs: list[str] = []
    s = sentence.strip()
    if not s:
        return ["empty"]
    text = lower(s, lang)
    ws = words(s, lang)
    # 1. form
    if "\n" in s:
        errs.append("form: multiple lines")
    if len(re.findall(r"[.!?](\s|$)", s)) > 2:
        errs.append("form: more than 2 sentences")
    if len(s.split()) > 45:
        errs.append(f"form: {len(s.split())} words > 45")
    if any(c in s for c in "*_`#[]{}"):
        errs.append("form: markup characters")
    if re.search(r"\b(lead|latest|ledger|bars_ago|within_bars|valid_now|slot)\b", text):
        errs.append("form: slot name leaked into text")
    for a in ADVICE[lang]:
        if re.search(r"\b" + re.escape(a), text):
            errs.append(f"advice word: {a}")
    # 2./3. numbers
    toks = NUM_RE.findall(s)
    nw = _num_words(s, lang)
    if slots["redacted"]:
        if toks:
            errs.append(f"redacted: digits {toks}")
        if nw:
            errs.append(f"redacted: number words {nw}")
        if "%" in s:
            errs.append("redacted: percent sign")
    else:
        if nw:
            errs.append(f"number words not allowed (write digits from the slots): {nw}")
        for t in toks:
            if match(t, values) is None:
                errs.append(f"invented number: {t}")
    # 4. time expressions
    b = slots["bars_ago"]
    since = slots["last_change"]["bars_ago"]
    within = (slots.get("condition") or {}).get("within_bars")
    unit = r"(bar\w*|gün\w*|seans\w*|days?|sessions?)"
    if lang == "tr":
        ago = re.findall(r"(\d+)\s*" + unit + r"\s+(önce|evvel)", text)
    else:
        ago = re.findall(r"(\d+)\s*" + unit + r"\s+ago", text)
    for n, *_ in ago:
        if int(n) != b and int(n) != since:
            errs.append(f"time: '{n} … ago' but the fact is {b} bars ago")
    for n, _u in re.findall(r"(\d+)\s*" + unit, text):
        if int(n) not in {b, since} and (within is None or int(n) != within):
            errs.append(f"time: '{n} bar' matches no time slot")
    today = ("bugün", "bu bar") if lang == "tr" else ("today", "this bar")
    yday = ("dün", "önceki bar") if lang == "tr" else ("yesterday", "previous bar")
    if any(re.search(r"\b" + p + r"\b", text) for p in today) and 0 not in (b, since):
        errs.append("time: 'today' but nothing happened this bar")
    if any(re.search(r"\b" + p + r"\b", text) for p in yday) and 1 not in (b, since):
        errs.append("time: 'yesterday' but nothing happened one bar ago")
    tc = slots["time_class"]
    if tc in ("event", "state_ended"):
        phrases = slots["when_phrases"][lang]
        if not any(p.lower() in text for p in phrases):
            errs.append(f"time: must say when the fact happened, one of {phrases}")
    # 5. tense
    if tc == "event" and not _past(ws, lang):
        errs.append("tense: event needs past tense")
    if tc == "state_holding":
        if not _present(ws, text, lang):
            errs.append("tense: holding state needs present tense"
                        + (" (-yor, 'hâlâ' or 'şu an')" if lang == "tr" else ""))
        if _no_longer(ws, text, lang) or "artık" in ws or "no longer" in text:
            errs.append("tense: holding state must not say 'no longer'")
    if tc == "state_ended":
        if not _past(ws, lang):
            errs.append("tense: ended state needs past tense")
        if not _no_longer(ws, text, lang):
            errs.append("tense: ended state needs 'no longer'" + (" ('artık … değil')" if lang == "tr" else ""))
    if tc == "expectation" and not _conditional(ws, lang):
        errs.append("tense: expectation needs a conditional" + (" (-sa/-se, eğer)" if lang == "tr" else " (if)"))
    # 6. kind term
    if not any(st in text for st in KIND_STEMS[lang][slots["kind"]]):
        errs.append(f"kind: must name the kind ({slots['kind_term'][lang]})")
    return errs
