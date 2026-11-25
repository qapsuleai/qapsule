"""RX4 uygunluk düzeneğinin ortak yardımcıları (yalnız standart kütüphane).

Yollar: depo kökü bu dosyadan dört üst dizin; RX4 düzeneği varsayılan olarak
`<depo>/../refs/experiments/rx4/harness` (salt okunur; yalnız veri ve beklenen çıktı okunur,
RX4 kodu içe aktarılmaz ya da çağrılmaz, K91).
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
DEFAULT_RX4 = REPO.parent / "refs" / "experiments" / "rx4" / "harness"
PROGRAMS_DIR = HERE / "programs"
PROBES_DIR = HERE / "probes"
DATA_DIR = HERE / "data"
SETS = ("seen", "heldout")


def load_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def dump_line(obj):
    """Tek satır JSON (alan sırası korunur; ondalıklar Python `repr` = en kısa gidiş-dönüş)."""
    return json.dumps(obj, ensure_ascii=False, separators=(",", ":"))


def load_tasks(rx4):
    return [load_json(p) for p in sorted((Path(rx4) / "tasks").glob("*.json"))]


def is_reject_task(task):
    return task["accept"] != "program"
