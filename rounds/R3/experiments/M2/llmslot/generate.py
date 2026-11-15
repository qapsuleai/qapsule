"""Generator for the `llmslot` M2 candidate (comparison only; the product has no LLM, K3/K21).

    python3 -I generate.py --run 1            # -> sentences.json, calls.json, log.run1.json
    python3 -I generate.py --run 2            # -> sentences.run2.json, calls.run2.json, log.run2.json

One `claude -p --model sonnet` call per situation writes both languages (JSON {"tr","en"}).
Each sentence goes through `verify.check`; a rejected language is re-requested with the
verifier's reasons (at most MAX_ATTEMPTS calls per situation). A language still rejected
after that gets a minimal "slot dump" (counted in the log; no template prose).

Each call runs in a fresh empty temp directory with `--tools ""`, `--strict-mcp-config`,
`--setting-sources ""`, `--no-session-persistence` and the system prompt from
`prompts/system.txt`; the user message is `prompts/user.txt` with the slots JSON.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import slots as S  # noqa: E402
import verify as V  # noqa: E402

MODEL = "sonnet"
MAX_ATTEMPTS = 3
LANGS = ("tr", "en")

SYSTEM = (HERE / "prompts" / "system.txt").read_text(encoding="utf-8")
USER = (HERE / "prompts" / "user.txt").read_text(encoding="utf-8")
RETRY = (HERE / "prompts" / "retry.txt").read_text(encoding="utf-8")


def call_claude(user: str) -> dict:
    with tempfile.TemporaryDirectory(prefix="llmslot-") as d:
        t0 = time.monotonic()
        p = subprocess.run(
            ["claude", "-p", "--model", MODEL, "--tools", "", "--strict-mcp-config",
             "--setting-sources", "", "--no-session-persistence", "--output-format", "json",
             "--system-prompt", SYSTEM, user],
            cwd=d, capture_output=True, text=True, timeout=300,
            env={k: v for k, v in os.environ.items() if k != "CLAUDECODE"},
        )
        secs = time.monotonic() - t0
    try:
        j = json.loads(p.stdout)
    except json.JSONDecodeError:
        return {"ok": False, "seconds": secs, "raw": p.stdout[-2000:], "stderr": p.stderr[-2000:],
                "cost": 0.0, "in": 0, "out": 0, "model": None}
    u = j.get("usage", {})
    tin = u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0) + u.get(
        "cache_read_input_tokens", 0)
    models = list((j.get("modelUsage") or {}).keys())
    return {"ok": not j.get("is_error", False), "seconds": secs, "raw": j.get("result", ""),
            "cost": j.get("total_cost_usd", 0.0), "in": tin, "out": u.get("output_tokens", 0),
            "cache_read": u.get("cache_read_input_tokens", 0), "model": models[0] if models else None}


def parse(raw: str) -> dict:
    a, b = raw.find("{"), raw.rfind("}")
    if a < 0 or b <= a:
        return {}
    try:
        o = json.loads(raw[a:b + 1])
    except json.JSONDecodeError:
        return {}
    return {k: v.strip() for k, v in o.items() if k in LANGS and isinstance(v, str)}


def slot_dump(sl: dict, lang: str) -> str:
    """Fallback: minimal slot dump (not prose, not a template)."""
    parts = [sl["instrument"], sl["kind"], sl["direction"], sl["time_kind"], sl["stage"],
             sl["when_phrases"][lang][0] if sl["time_class"] in ("event", "state_ended") else "",
             f"valid_now={sl['valid_now']}"]
    return " · ".join(p for p in parts if p)


def run_one(inp: dict) -> dict:
    sl = S.build(inp)
    vals = S.allowed_values(sl)
    slots_json = json.dumps(S.for_prompt(sl), ensure_ascii=False, sort_keys=True)
    accepted: dict[str, str] = {}
    attempts: list[dict] = []
    need = list(LANGS)
    feedback: dict[str, list[str]] = {}
    last: dict[str, str] = {}
    for k in range(MAX_ATTEMPTS):
        if k == 0:
            user = USER.replace("{SLOTS}", slots_json)
        else:
            fb = "\n".join(f'- {l}: previous text "{last.get(l, "")}" was rejected: '
                           + "; ".join(feedback[l]) for l in need)
            user = (RETRY.replace("{SLOTS}", slots_json).replace("{FEEDBACK}", fb)
                    .replace("{KEYS}", ", ".join(f'"{l}"' for l in need)))
        r = call_claude(user)
        out = parse(r["raw"]) if r["ok"] else {}
        rec = {"attempt": k + 1, "langs": list(need), "seconds": round(r["seconds"], 3),
               "cost_usd": r["cost"], "input_tokens": r["in"], "output_tokens": r["out"],
               "model": r["model"], "raw": r["raw"], "verdict": {}}
        for l in list(need):
            txt = out.get(l, "")
            errs = V.check(sl, l, txt, vals) if txt else ["no text for this language"]
            rec["verdict"][l] = errs
            if errs:
                feedback[l] = errs
                last[l] = txt
            else:
                accepted[l] = txt
        need = [l for l in need if l not in accepted]
        attempts.append(rec)
        if not need:
            break
    fallback = {}
    for l in need:
        accepted[l] = slot_dump(sl, l)
        fallback[l] = True
    return {"id": inp["id"], "sentences": {l: accepted[l] for l in LANGS}, "attempts": attempts,
            "fallback": fallback}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", type=int, required=True)
    ap.add_argument("--inputs", default=str(HERE / "inputs.json"))
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--only", default="", help="comma-separated ids (trial runs)")
    ap.add_argument("--tag", default="", help="suffix for trial outputs")
    a = ap.parse_args()
    inputs = json.loads(Path(a.inputs).read_text(encoding="utf-8"))
    if a.only:
        keep = set(a.only.split(","))
        inputs = [x for x in inputs if x["id"] in keep]
    # first situation alone so the system prompt is cached before the parallel calls
    res = [run_one(inputs[0])]
    with ThreadPoolExecutor(max_workers=a.workers) as ex:
        res += list(ex.map(run_one, inputs[1:]))
    res.sort(key=lambda r: r["id"])
    suffix = "" if a.run == 1 else f".run{a.run}"
    if a.tag:
        suffix = f".{a.tag}"
    sentences = {"sentences": {r["id"]: r["sentences"] for r in res}}
    calls = []
    for r in res:
        for at in r["attempts"]:
            n = len(at["langs"])
            for l in at["langs"]:
                calls.append({"id": r["id"], "lang": l, "attempt": at["attempt"],
                              "seconds": at["seconds"],
                              "input_tokens": at["input_tokens"] // n,
                              "output_tokens": at["output_tokens"] // n,
                              "cost_usd": round(at["cost_usd"] / n, 6),
                              "shared_call_langs": n, "model": at["model"]})
    log = {"run": a.run, "model_alias": MODEL, "max_attempts": MAX_ATTEMPTS, "results": res}
    (HERE / f"sentences{suffix}.json").write_text(
        json.dumps(sentences, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    (HERE / f"calls{suffix}.json").write_text(
        json.dumps(calls, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    (HERE / f"log{suffix or '.run1'}.json").write_text(
        json.dumps(log, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    tot = sum(at["cost_usd"] for r in res for at in r["attempts"])
    ncalls = sum(len(r["attempts"]) for r in res)
    fb = sum(len(r["fallback"]) for r in res)
    rej = sum(1 for r in res for at in r["attempts"] for e in at["verdict"].values() if e)
    print(f"situations={len(res)} calls={ncalls} rejections={rej} fallbacks={fb} cost=${tot:.4f}")


if __name__ == "__main__":
    main()
