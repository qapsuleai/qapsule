# qp-render

**Başvuru çizicisi.** Bir sahne betimini (`SceneDesc`, `qp-grammar`'ın ürettiği) belirlenimci
olarak piksele çeviren başsız çizici (tiny-skia + fontdue, yalnız skaler yollar). Aynı kod
`qp-wasm` ile wasm32'ye derlenir. Çıktı: düz (önçarpımsız) RGBA8 ya da PNG. Çizim yığını
denemesi ve belirlenimcilik kuralları `crates/qp-render/SPIKE.md`'de.

**Katman:** çekirdek (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`, `tiny-skia`, `fontdue`, `png`; isteğe bağlı (yalnız deneme
özellikleri `spike-vello-cpu` / `spike-vello-gpu`) `vello_cpu`, `vello`, `pollster`.
Geliştirme (örnekler ve altın testleri): `qp-compute`, `qp-grammar`, `qp-source`,
`qp-facts`, `qp-capsule`, `qp-session`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):**

- `render_scene_rgba`, `render_scene_rgba_scaled`, `render_scene_png`, `encode_png`,
  `render_stress_rgba`, `DEFAULT_WIDTH`, `DEFAULT_HEIGHT`.
- `parse_bars`, `RenderError`; yeniden dışa aktarılan `SceneDesc`, `Bar`, `Series`.
- Modüller: `cpu` (başvuru arka ucu), `scene` (çizici tarafı sahne yardımcıları),
  `retained` (son kareyi tutan yazılım boyayıcı), `fonts` (gömülü yazı tipleri);
  `spikes` yalnız deneme özellikleriyle.

**Düzen (`src/`):** `spikes/` — karşılaştırma amaçlı vello arka uçları (özellik kapılı);
geri kalanı kökte. `assets/fonts/` gömülü yazı tipleri; `examples/` örnek, ölçüm ve deneme
sürücüleri.

**Ne değildir:** yerleşim, veri erişimi ya da hesap yapmaz; sahneyi kurmaz (o `qp-grammar`);
gösterim kabuğu değildir. RG1'de `synthetic_bars` sarmalayıcısı kaldırıldı; sentetik seri
`qp_source::synthetic_series`'tendir (`rounds/RG1/PLAN.md` §13.3).
