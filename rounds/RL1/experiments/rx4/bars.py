#!/usr/bin/env python3
"""RX4 bar verisi → Qapsule bar serisi dosyası (yalnız standart kütüphane).

    python3 -I bars.py [--rx4 DIR] [--set seen|heldout] [--out PATH]

Biçim: `fixtures/bars` biçimi, yani gömülü barlı bir `qp_schema::Series`
(`qp_source::FileSource::open` bunu okur; `qp compute INPUT` aynı dosyayı alır):

    {"id":"rx4-seen","instrument":{"symbol":"RX4"},"interval":"1h",
     "source":{"kind":"inline"},"data":{"kind":"bars","bars":[{"t","o","h","l","c","v"}...]}}

`instrument.calendar` yazılmaz: takvimsiz seri → UTC hizası (PROGRAMS §3.2, §15.1). Sayılar
RX4 dosyasındaki yazımla (Python float okuması/yazımı en kısa gidiş-dönüş, bayt değeri aynı).
Veri sentetiktir (RX4 `gen_data.py`, tohumlu); `seen` commit edilir (≈ 200 KB). `heldout`
RX4'te yalıtılmış kümedir: commit edilmez, `conform.py` onu koşu anında geçici dizine üretir.
"""
import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from rx4common import DATA_DIR, DEFAULT_RX4, load_json  # noqa: E402

INTERVAL_MS = 3_600_000
FIELDS = ("t", "o", "h", "l", "c", "v")


def series_from_rx4(rx4, which):
    src = load_json(Path(rx4) / "data" / f"{which}.json")
    if src.get("tf") != "1h":
        raise SystemExit(f"{which}: tf {src.get('tf')!r}, 1h bekleniyordu")
    bars = src["bars"]
    for i, b in enumerate(bars):
        if tuple(b) != FIELDS:
            raise SystemExit(f"{which}: bar {i} alanları {tuple(b)}")
        if i and b["t"] - bars[i - 1]["t"] != INTERVAL_MS:
            raise SystemExit(f"{which}: bar {i} boşluk ya da sırasız")
    return {"id": f"rx4-{which}", "instrument": {"symbol": "RX4"}, "interval": "1h",
            "source": {"kind": "inline"}, "data": {"kind": "bars", "bars": bars}}


def write_series(series, out):
    import json
    head = {k: v for k, v in series.items() if k != "data"}
    lines = [json.dumps(b, separators=(",", ":")) for b in series["data"]["bars"]]
    text = (json.dumps(head, separators=(",", ":"))[:-1]
            + ',"data":{"kind":"bars","bars":[\n' + ",\n".join(lines) + "\n]}}\n")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8")
    return out


def bar_times(series):
    return [b["t"] for b in series["data"]["bars"]]


def main():
    ap = argparse.ArgumentParser(description="RX4 verisi → Qapsule bar serisi")
    ap.add_argument("--rx4", type=Path, default=DEFAULT_RX4)
    ap.add_argument("--set", default="seen", choices=["seen", "heldout"])
    ap.add_argument("--out", type=Path)
    a = ap.parse_args()
    out = a.out or DATA_DIR / f"{a.set}.bars.json"
    if a.set == "heldout" and a.out is None:
        raise SystemExit("heldout commit edilmez: --out ile depo dışı bir yol verin")
    s = series_from_rx4(a.rx4, a.set)
    write_series(s, out)
    print(f"{out}: {len(s['data']['bars'])} bar, {out.stat().st_size} bayt")


if __name__ == "__main__":
    main()
