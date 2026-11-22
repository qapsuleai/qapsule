# RG1 başlangıç tutanağı

Alındığı commit: ana akış kaydından hemen sonra, ilk kod taşımasından önce (`27656b1` kodu).

| Dosya | İçerik |
|---|---|
| `manifest.sha256` | `scripts/rg1-snapshot.sh` çıktısının her kaleminin SHA-256'sı (137 kalem): kayıtlı fikstürler, şema dışa aktarımı, 12 görünüm × (PNG, `explain`, `scene`), `compute` çıktıları, dört hesap özeti (aarch64, r3, x86_64 Rosetta, wasm32), uygunluk dökümleri (`inproc`, `serve`, `mcp`, görünümler), dört yansı/görüntüleyici bandı, yeniden oynatma akışı, MCP `initialize`+`tools/list` (4 yüzey + RD1 uyarlayıcısı), kabuksuz sürüş özetleri, görünüş kayıtları, komut satırı `--help` ağacı, iki puanlayıcı testi, test sayısı |
| `text/` | metin kalemlerinin kendisi (fark çıkarsa okumak için) |
| `verify.log` | `scripts/verify.sh` tam koşu (18/18 PASS, 631 s) |
| `perf.txt` | `scripts/rg1-perf.sh`: K17 `sweep_bench`, WASM boyutu, K52 `metrics.mjs` (CPU ×4) |
| `lines.txt` | crate satır tablosu (test dışı / test), crate başına `.rs` sayısı, en büyük 60 kaynak dosya |

Karşılaştırma: `scripts/rg1-compare.sh` (tutanağı yeniden üretir, `manifest.sha256` ile bayt bayt).
Tarayıcı altınları (`fixtures/golden/demo/**`) `verify.sh` adım 18'de kayıtlı dosyalarla
karşılaştırılır; kayıtlı dosyaların kendisi `manifest` içindeki `fixtures/committed.sha256` ile.
