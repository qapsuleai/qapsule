"""Blind reader and critic runner (SPEC §G4, §G5, §G9).

One call = one fresh `claude -p` process with only the `Read` tool (no Bash: it cannot
crop, zoom or run code), restricted mode (no user/project settings, no MCP), no session
persistence, a few turns, in a new temporary directory that holds exactly one image under
a random name. The prompt never names a design, a branch, "old/new" or a round
(`check_prompt`). Each answer is cut to one sentence before scoring; the cut is recorded.
Every call's prompt, image id, raw stream, answer, cost and model go to a log record.
"""

from __future__ import annotations

import json
import os
import random
import re
import shutil
import string
import subprocess
import tempfile
import time
from pathlib import Path

from .keys import QUESTIONS, SETS, UNKNOWN

MAX_SENTENCE = 200  # characters kept of one answer (SPEC §G4.4)
LETTERS = "ABCDEFGH"

# SPEC §G4.5: the personas, verbatim.
PERSONAS = {
    "F": {
        "tr": "Yirmi iki yaşındasın, birkaç aydır telefondan hisse alıp satıyorsun. Teknik analiz "
              "bilmiyorsun: grafik, indikatör, mum, direnç gibi sözcükler sana bir şey ifade etmiyor. "
              "Telefonuna bakıyorsun ve ekrana yalnızca beş saniye bakacaksın; uzun uzun okumazsın, "
              "ilk bakışta gözüne çarpanla cevap verirsin.",
        "en": "You are twenty-two and have been trading stocks on your phone for a few months. You do "
              "not know technical analysis: chart, indicator, candle, resistance mean nothing to you. "
              "You glance at your phone for only five seconds; you do not read at length, you answer "
              "with what catches your eye at first sight.",
    },
    "H": {
        "tr": "Kırk beş yaşındasın, yirmi yıldır borsada işlem yapıyorsun; terminal ekranlarına, "
              "mum grafiklerine ve indikatörlere alışkınsın, ayrıntıya güvenirsin. Telefonunda bir "
              "piyasa uygulamasına kısa bir bakış atıyorsun.",
        "en": "You are forty-five and have traded for twenty years; you are used to terminal screens, "
              "candlestick charts and indicators, and you trust details. You take a short look at a "
              "market app on your phone.",
    },
}

INTRO = {
    "tr": "Bu klasörde tek bir görüntü var: {file}. Read aracıyla onu bir kez aç (başka araç yok, "
          "büyütme ya da kırpma yok) ve gördüğün hâliyle cevap ver.",
    "en": "This folder holds one image: {file}. Open it once with the Read tool (no other tool, no "
          "zoom or crop) and answer from what you see.",
}
RULES = {
    "tr": "Her soruya EN ÇOK BİR CÜMLE yaz; cümle seçtiğin seçeneğin harfiyle başlasın (ör. \"B, "
          "çünkü …\"). Emin değilsen bilmiyorum seçeneğini seç. Cevabını yalnız şu biçimde bir JSON "
          "nesnesi olarak ver, başka bir şey yazma: {keys}",
    "en": "Write AT MOST ONE SENTENCE per question, starting with the letter of the option you "
          "choose (e.g. \"B, because …\"). If unsure, choose the don't-know option. Reply only with "
          "a JSON object of this form, nothing else: {keys}",
}
UNK_TEXT = {"tr": "bilmiyorum / göremiyorum", "en": "I don't know / can't see it"}

THROTTLE_NOTE = {
    "tr": "Görüntü bilerek bulanık ve küçük: uzaktan, bir anlık bakış gibi.",
    "en": "The image is blurred and small on purpose: like a glance from a distance.",
}
HALF_NOTE = {
    "tr": "Görüntü küçük: telefonu kol mesafesinde tutup bir an bakıyor gibisin.",
    "en": "The image is small: as if you glance at the phone held at arm's length.",
}
GRAY_NOTE = {"tr": "Görüntü gri tonlamalı.", "en": "The image is in greyscale."}
MOTION_NOTE = {
    "tr": "Görüntüde iki kare yan yana: solda bir ekran, sağda o ekrandan sonraki bir geçişin "
          "ortasından donmuş bir ara kare (harfli bölgeler düzeneğin çizgileridir).",
    "en": "The image shows two frames side by side: a screen on the left and, on the right, a frozen "
          "in-between frame from the transition that follows it (the lettered regions are guides).",
}

# (M21, Ek G6) the numbered bands over a feed frame are the harness's.
NOTICE_NOTE = {
    "tr": "Ekranın solundaki 1, 2, 3, 4 numaraları ve ince gri yatay çizgiler uygulamanın değil, bu sorunun "
          "yardımcı çizgileridir: ekranı yukarıdan aşağıya dört eşit şeride böler.",
    "en": "The numbers 1, 2, 3, 4 on the left and the thin grey horizontal lines are not part of the app; they "
          "are guides for this question: they split the screen into four equal bands, top to bottom.",
}
NOTICE_PAIR_NOTE = {
    "tr": "Görüntüde iki kare yan yana: solda bir ekran, sağda aynı ekranın kısa bir süre sonraki hâli. Sağdaki "
          "karenin solundaki 1, 2, 3, 4 numaraları ve ince gri çizgiler düzeneğin yardımcı çizgileridir (dört eşit yatay şerit).",
    "en": "The image shows two frames side by side: a screen on the left and the same screen a moment later on "
          "the right. The numbers 1, 2, 3, 4 and thin grey lines on the right frame are guides (four equal horizontal bands).",
}

# (M18) the lettered columns over the chart are the harness's, not the app's.
CHART_NOTE = {
    "tr": "Grafiğin üstündeki ince gri dikey çizgiler ve altındaki 1, 2, 3, 4 numaraları uygulamanın değil, "
          "bu sorunun yardımcı çizgileridir: grafiği soldan sağa dört eşit zaman dilimine böler.",
    "en": "The thin grey vertical lines over the chart and the numbers 1, 2, 3, 4 below it are not part of "
          "the app; they are guides for this question: they split the chart into four equal time slices, left to right.",
}

# Words that must never reach a reader or critic (design and branch names are added at
# run time from the key file; SPEC §G9).
FORBIDDEN = ["r4", "r5", "eski", "yeni tasarım", "old design", "new design", "variant", "alternatif",
             "alternative", "baseline", "başvuru", "broken", "bozuk", "look=", "face=", "vadrel", "fotolay",
             "tavrik", "elikov", "zumlak"]


def check_prompt(text: str, extra: list[str] = ()) -> list[str]:
    """Forbidden words found in a prompt (case-insensitive, word-ish match)."""
    low = text.lower()
    bad = []
    for w in list(FORBIDDEN) + list(extra):
        if re.search(r"(?<![a-z0-9])" + re.escape(w.lower()) + r"(?![a-z0-9])", low):
            bad.append(w)
    return bad


def option_order(qid: str, rng: random.Random) -> list[str]:
    codes = [c for c, _ in QUESTIONS[qid]["options"]]
    rng.shuffle(codes)
    return codes + [UNKNOWN]


def build_prompt(qset: str, lang: str, persona: str, file: str, seed: int, note: str = ""):
    """The reader prompt and its letter -> code map per question (options shuffled with
    `seed`, the don't-know option always last)."""
    rng = random.Random(seed)
    lines = [PERSONAS[persona][lang], INTRO[lang].format(file=file)]
    if note:
        lines.append(note)
    letters = {}
    for n, q in enumerate(SETS[qset], 1):
        opts = dict(QUESTIONS[q]["options"])
        order = option_order(q, rng)
        letters[f"q{n}"] = {"q": q, "map": {LETTERS[i]: c for i, c in enumerate(order)}}
        opt_txt = "; ".join(f"{LETTERS[i]}) {opts[c][lang] if c != UNKNOWN else UNK_TEXT[lang]}" for i, c in enumerate(order))
        lines.append(f"q{n}. {QUESTIONS[q][lang]} {opt_txt}")
    keys = "{" + ", ".join(f'"q{n}": "..."' for n in range(1, len(SETS[qset]) + 1)) + "}"
    lines.append(RULES[lang].format(keys=keys))
    return "\n".join(lines), letters


def one_sentence(text: str) -> tuple[str, bool]:
    """First sentence, at most MAX_SENTENCE characters; (kept, was_cut)."""
    t = " ".join(str(text).split())
    m = re.search(r"[.!?](\s|$)", t)
    kept = t[: m.start() + 1] if m else t
    kept = kept[:MAX_SENTENCE]
    return kept, kept != t


def code_of(sentence: str, letter_map: dict) -> str | None:
    """The option letter the sentence starts with, mapped to its code (None: no letter)."""
    m = re.match(r"\s*[\(\[\"']?([A-H])(?![a-zçğıöşü])", sentence)
    if not m:
        return None
    return letter_map.get(m.group(1))


def parse_answer(result: str, letters: dict) -> dict:
    """Parses the reader's JSON (the first {...} block), cuts each answer to one sentence and
    maps letters to codes."""
    m = re.search(r"\{.*\}", result or "", re.S)
    raw = {}
    if m:
        try:
            raw = json.loads(m.group(0))
        except json.JSONDecodeError:
            raw = {}
    out = {}
    for k, spec in letters.items():
        kept, cut = one_sentence(raw.get(k, "")) if k in raw else ("", False)
        out[spec["q"]] = {"sentence": kept, "cut": cut, "code": code_of(kept, spec["map"]) if kept else None,
                          "missing": k not in raw}
    return out


def rand_name(rng: random.Random, n: int = 8) -> str:
    return "".join(rng.choice(string.ascii_lowercase + string.digits) for _ in range(n)) + ".png"


def call_claude(image: Path, prompt_fn, model: str, seed: int, workroot: Path, max_turns: int = 3,
                timeout: int = 240) -> dict:
    """Runs one blind call. `prompt_fn(file_name) -> prompt`. Returns the log record."""
    rng = random.Random(seed)
    workroot.mkdir(parents=True, exist_ok=True)
    d = Path(tempfile.mkdtemp(prefix="r", dir=workroot))
    name = rand_name(rng)
    shutil.copyfile(image, d / name)
    prompt = prompt_fn(name)
    cmd = ["claude", "-p", "--model", model, "--tools", "Read", "--allowedTools", "Read", "--restricted",
           "--strict-mcp-config", "--permission-mode", "dontAsk", "--no-session-persistence",
           "--max-turns", str(max_turns), "--output-format", "stream-json", "--verbose", prompt]
    t0 = time.time()
    env = {k: v for k, v in os.environ.items() if k not in ("CLAUDECODE",)}
    try:
        p = subprocess.run(cmd, cwd=d, capture_output=True, text=True, timeout=timeout, env=env)
        stdout, stderr, code = p.stdout, p.stderr, p.returncode
    except subprocess.TimeoutExpired as e:
        stdout, stderr, code = e.stdout or "", "timeout", -1
    rec = {"file": name, "prompt": prompt, "model_requested": model, "seconds": round(time.time() - t0, 1),
           "exit": code, "stderr": (stderr or "")[-2000:], "tools": [], "result": None, "num_turns": None,
           "cost_usd": None, "models": [], "init_tools": None}
    for line in (stdout or "").splitlines():
        try:
            o = json.loads(line)
        except json.JSONDecodeError:
            continue
        if o.get("type") == "system" and o.get("subtype") == "init":
            rec["init_tools"] = o.get("tools")
        if o.get("type") == "assistant":
            for c in o["message"].get("content", []):
                if c.get("type") == "tool_use":
                    rec["tools"].append({"name": c["name"], "input": c.get("input")})
        if o.get("type") == "result":
            rec["result"] = o.get("result")
            rec["num_turns"] = o.get("num_turns")
            rec["cost_usd"] = o.get("total_cost_usd")
            rec["models"] = sorted((o.get("modelUsage") or {}).keys())
    shutil.rmtree(d, ignore_errors=True)
    # One image, one look: exactly one Read of the one file (SPEC §G4.3).
    reads = [t for t in rec["tools"] if t["name"] == "Read"]
    rec["one_look"] = len(reads) == 1 and str(reads[0]["input"].get("file_path", "")).endswith(name) \
        and not {"offset", "limit", "pages"} & set(reads[0]["input"])
    return rec


# --- critic (SPEC §G5) ------------------------------------------------------------------

CRITIC = {
    "tr": "Bir telefon uygulaması için iki ekran tasarımını karşılaştıran bir tasarım eleştirmenisin. "
          "Bu klasörde tek bir görüntü var: {file}. Read aracıyla bir kez aç. Görüntüde aynı içerik iki "
          "tasarımda yan yana: solda A, sağda B. Uygulama piyasada olan biteni gündelik dille anlatır, "
          "tavsiye vermez.\n"
          "Önce, ölçütlere bakmadan, ekranlarda ilk ne gördüğünü tek cümleyle yaz (\"gozlem\"). Sonra şu "
          "ölçütlerle hangisinin daha iyi olduğuna karar ver: (1) tek bakışta ne olduğu anlaşılıyor mu; "
          "(2) önce neyin okunacağı belli mi, en hızlı okunan öğe anlam işareti mi; (3) hâlâ geçerli / "
          "artık geçerli değil / kapandı ayrımı ilk bakışta görülüyor mu; (4) güvenilirlik (geçmiş, az "
          "örnek) gizlenmeden görünür mü; (5) renk tek başına anlam taşımıyor mu (yön ve sonuç biçimle de "
          "okunuyor mu); (6) okunaklı mı (karşıtlık, yazı boyu); (7) ekran bir eylem (al/sat) önermiyor "
          "mu. Daha çok yazı daha iyi demek değildir.\n"
          "Yalnız şu JSON'u yaz: {{\"gozlem\": \"<bir cümle>\", \"karar\": \"A\" | \"B\" | \"ayırt edemiyorum\", "
          "\"neden\": \"<bir cümle>\"}}",
    "en": "You are a design critic comparing two screen designs for a phone app. This folder holds one "
          "image: {file}. Open it once with the Read tool. It shows the same content in two designs side by "
          "side: A on the left, B on the right. The app tells what happens in the market in everyday words "
          "and gives no advice.\n"
          "First, before the criteria, write in one sentence what you see first (\"observation\"). Then decide "
          "which is better by these criteria: (1) is it clear at a glance what happened; (2) is it clear what "
          "to read first, is the meaning mark the fastest-read element; (3) are still valid / no longer valid / "
          "closed told apart at first sight; (4) is reliability (track record, few cases) visible, not hidden; "
          "(5) colour does not carry meaning alone (direction and outcome also read by shape); (6) legibility "
          "(contrast, text size); (7) the screen does not suggest an action (buy/sell). More text is not "
          "better.\n"
          "Reply only with this JSON: {{\"observation\": \"<one sentence>\", \"verdict\": \"A\" | \"B\" | "
          "\"cannot tell\", \"why\": \"<one sentence>\"}}",
}


# (M21, Ek G6) the motion critic: three frames (start | A | B), A and B frozen at the same
# moment after the same start. Same JSON keys as CRITIC (parse_critic).
CRITIC_M21 = {
    "tr": "Bir telefon uygulamasının iki hareket tasarımını karşılaştıran bir tasarım eleştirmenisin. "
          "Bu klasörde tek bir görüntü var: {file}. Read aracıyla bir kez aç. Görüntüde üç kare yan yana: "
          "solda \"0\" başlangıç ekranı, ortada A, sağda B. A ve B aynı başlangıçtan sonra aynı anda (bir "
          "geçişin ya da ekrandaki bir değişimin ortasında) dondurulmuş karelerdir ve iki farklı hareket "
          "tasarımına aittir. Uygulama piyasada olan biteni gündelik dille anlatır, tavsiye vermez.\n"
          "Önce, ölçütlere bakmadan, karelerde ilk ne gördüğünü tek cümleyle yaz (\"gozlem\"). Sonra şu "
          "ölçütlerle hangisinin daha iyi olduğuna karar ver: (1) başlangıç ekranındaki nesnenin nereye "
          "gittiği, aynı nesne olduğu izlenebiliyor mu; (2) bir kapsülde bir değişiklik olduysa hangisinde "
          "olduğu ilk bakışta görülüyor mu; (3) hareket dikkat dağıtmıyor, ekranı karmaşıklaştırmıyor mu; "
          "(4) ara karede bile içerik okunaklı mı; (5) hareket bir eylem (al/sat) önermiyor mu. Daha çok "
          "hareket daha iyi demek değildir.\n"
          "Yalnız şu JSON'u yaz: {{\"gozlem\": \"<bir cümle>\", \"karar\": \"A\" | \"B\" | \"ayırt edemiyorum\", "
          "\"neden\": \"<bir cümle>\"}}",
    "en": "You are a design critic comparing two motion designs for a phone app. This folder holds one "
          "image: {file}. Open it once with the Read tool. It shows three frames side by side: \"0\" the start "
          "screen on the left, A in the middle, B on the right. A and B are frozen at the same moment after the "
          "same start (in the middle of a transition or of a change on the screen) and belong to two different "
          "motion designs. The app tells what happens in the market in everyday words and gives no advice.\n"
          "First, before the criteria, write in one sentence what you see first (\"observation\"). Then decide "
          "which is better by these criteria: (1) can you follow where the object of the start screen went, that "
          "it is the same object; (2) if a capsule changed, is it clear at first sight which one; (3) the motion "
          "does not distract or clutter the screen; (4) the content stays legible even in the in-between frame; "
          "(5) the motion does not suggest an action (buy/sell). More motion is not better.\n"
          "Reply only with this JSON: {{\"observation\": \"<one sentence>\", \"verdict\": \"A\" | \"B\" | "
          "\"cannot tell\", \"why\": \"<one sentence>\"}}",
}


def parse_critic(result: str) -> dict:
    m = re.search(r"\{.*\}", result or "", re.S)
    raw = {}
    if m:
        try:
            raw = json.loads(m.group(0))
        except json.JSONDecodeError:
            raw = {}
    v = str(raw.get("karar", raw.get("verdict", ""))).strip().upper()
    verdict = "A" if v.startswith("A") and not v.startswith("AY") else "B" if v.startswith("B") else None
    obs, cut1 = one_sentence(raw.get("gozlem", raw.get("observation", "")))
    why, cut2 = one_sentence(raw.get("neden", raw.get("why", "")))
    return {"verdict": verdict, "observation": obs, "why": why, "cut": cut1 or cut2, "parsed": bool(raw)}
