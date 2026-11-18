"""R5/D density test: draw the six R2 views (fixtures/views/v0..v5) with the deterministic
`ref` renderer (`qp render`) and dump their scene descriptions (`qp scene --roles`) for every
look x theme x size. The theme is set on the view of a *copy* of the transcript (the put_view
op gets `"theme"`), because `qp render` has no theme flag; the copy lives in the work dir.

    python3 rounds/R5/experiments/D/render.py --qp /path/to/qp --work DIR   # from the repo root

Output: DIR/<view>-<look>-<theme>-<size>.png and .json (outside the repo).
"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

VIEWS = {"v0": ("v0-r0", "r0"), "v1": ("v1-three-panels", "three"), "v2": ("v2-bollinger-band", "boll"),
         "v3": ("v3-fan", "fan"), "v4": ("v4-log", "logv"), "v5": ("v5-conditional", "cond"),
         "s8": (None, "dense")}
# s8 is not an R2 view: a stress view built here from the v1 transcript (D only, never a
# fixture): v1's three panels plus eight SMA lines on the price panel, so the series palette
# s1..s8 and eight legends share the title row.
SMA_PERIODS = (5, 10, 20, 30, 50, 80, 100, 150)
LOOKS = ("oltan", "r4")
THEMES = ("light", "dark")
# phone and desktop: the sizes a reader sees (R7's targets); narrow and quad are stress sizes
# (the smallest common phone width; one quarter of a 1280 x 720 desktop split 2 x 2).
SIZES = {"phone": (402, 600), "desk": (1280, 720), "narrow": (320, 480), "quad": (640, 360)}


def stress() -> dict:
    d = json.loads(Path("fixtures/views/v1-three-panels.json").read_text())
    ops = d["steps"][1]["send"]["msg"]["ops"]
    view = next(op for op in ops if op.get("op") == "put_view")["view"]
    view["id"] = "dense"
    for n in SMA_PERIODS:
        ops.insert(0, {"id": f"sma{n}", "inputs": ["px"], "op": "add_study", "params": {"period": n},
                       "study": "sma"})
        view["layers"].append({"id": f"sma{n}", "mark": "line", "panel": 0, "series": f"sma{n}"})
    for op in d["steps"][2]["send"]["msg"]["ops"]:
        op["view"] = "dense"
    for st in d["steps"]:
        st["expect"] = []
    return d


def themed(src, theme: str, dst: Path) -> None:
    d = stress() if src is None else json.loads(src.read_text())
    n = 0
    for s in d["steps"]:
        for op in s["send"]["msg"].get("ops") or []:
            if op.get("op") == "put_view":
                op["view"]["theme"] = theme
                n += 1
    assert n == 1, src
    dst.write_text(json.dumps(d))


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--qp", required=True)
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    w = Path(a.work)
    (w / "tr").mkdir(parents=True, exist_ok=True)
    for v, (fname, vid) in VIEWS.items():
        for th in THEMES:
            t = w / "tr" / f"{v}-{th}.json"
            themed(None if fname is None else Path("fixtures/views") / f"{fname}.json", th, t)
            for look in LOOKS:
                for size, (wd, ht) in SIZES.items():
                    stem = w / f"{v}-{look}-{th}-{size}"
                    common = ["--transcript", str(t), "--view", vid, "--look", look,
                              "--width", str(wd), "--height", str(ht)]
                    subprocess.run([a.qp, "render", *common, "-o", f"{stem}.png"], check=True)
                    out = subprocess.run([a.qp, "scene", *common, "--roles"], check=True,
                                         capture_output=True, text=True).stdout
                    Path(f"{stem}.json").write_text(out)
    print("ok", len(VIEWS) * len(THEMES) * len(LOOKS) * len(SIZES))


if __name__ == "__main__":
    main()
