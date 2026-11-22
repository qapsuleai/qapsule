"""Renders docs/DEBT.md from scripts/arch-baseline.json and the named exceptions in
scripts/arch-config.json (one source; the Markdown is generated, not hand-written)."""

RULES = {
    "file.soft": ("Dosya 600 satırı aşıyor (kesin sınır 800 değil)", "konuya göre alt modül; ≤ 600"),
    "fn.lines": ("Fonksiyon 80 satırı aşıyor (RG1'de fonksiyon bölünmedi)", "adlandırılmış adımlara böl; ≤ 80"),
    "dir.files": ("Klasörde 15'ten çok kaynak dosya", "konu klasörleri; ≤ 15"),
    "crate.lines": ("Crate 12 bin test dışı satırı aşıyor", "alan sınırından böl; ≤ 12 bin"),
    "shell.lines": ("Kabuk crate'i 4 bin satırı aşıyor", "mantığı aşağı katmana taşı; ≤ 4 bin"),
    "test.file": ("Test dosyası 800 satırı aşıyor (M5 kesildi)", "konuya göre böl; ≤ 800"),
    "readme.module": ("Modül klasöründe README yok", "kısa README"),
    "doc.size": ("Belge 1.500 satır ya da 60 KB'ı aşıyor", "konuya göre böl, dizin bırak"),
    "lint.allow": ("Mevcut lint ihlali öğe başına `#[allow]` ile susturuldu (RG1/L)", "ihlali düzelt, izni kaldır"),
}


def render(base, cfg):
    out = ["# Borç taban çizgisi", "",
           "üretilmiştir: `scripts/arch-check --debt-md` (kaynak `scripts/arch-baseline.json` ve",
           "`scripts/arch-config.json` istisnaları). Elle düzenlemeyin.", "",
           "Kural `docs/ENGINEERING.md` §1.4: borç görünürdür ve büyümez. `scripts/arch-check` taban",
           "çizgisindeki değerden ya da sayıdan **fazlasına** hata verir (mandal); yeni kalem ancak",
           "`--write-baseline --allow-new` ile ve gerekçesiyle girer.", ""]
    exc = cfg.get("exceptions", [])
    ov = cfg.get("layer_overrides", {})
    if exc or ov:
        out += ["## İstisna istekleri (Arkin'in kararını bekliyor)", "",
                "| No | Kural | Kalem | Gerekçe | Durum |", "|---|---|---|---|---|"]
        for c, o in sorted(ov.items()):
            out.append(f"| {o.get('id', '')} | katman | `{c}` → `{o['layer']}` | {o.get('reason', '')} | {o.get('status', '')} |")
        for e in exc:
            cap = f"; tavan {e['max']}" if "max" in e else ""
            out.append(f"| {e.get('id', '')} | `{e['rule']}` | `{e['item']}` | {e.get('reason', '')} | {e.get('status', '')}{cap} |")
        out.append("")
    rules = base.get("rules", {})
    out += ["## Özet", "", "| Kural | Ne | Hedef | Kalem |", "|---|---|---|---|"]
    for r, e in sorted(rules.items()):
        what, target = RULES.get(r, (r, ""))
        out.append(f"| `{r}` | {what} | {target} | {e['count']} |")
    out.append("")
    for r, e in sorted(rules.items()):
        what, target = RULES.get(r, (r, ""))
        out += [f"## `{r}` — {what}", "", f"Hedef: {target}.", "", "| Kalem | Değer |", "|---|---|"]
        for item, v in sorted(e["items"].items(), key=lambda kv: (-(kv[1]["value"] if isinstance(kv[1], dict) else kv[1]), kv[0])):
            val = v["value"] if isinstance(v, dict) else v
            note = f" — {v['note']}" if isinstance(v, dict) and v.get("note") else ""
            out.append(f"| `{item}` | {val}{note} |")
        out.append("")
    return "\n".join(out)
