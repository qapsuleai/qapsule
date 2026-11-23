# RF1 başlangıç tutanağı

Alındığı kod: `05430e06` (ana akış kaydı; RD2 kapanışının kodu), ilk kod değişikliğinden önce.
Araç: `scripts/refactor/snapshot.sh` (RG1'in `rg1-snapshot.sh`'ı, `scripts/refactor/`'a taşındı ve
RD2 kalemleriyle genişletildi). Karşılaştırma: `scripts/refactor/compare.sh`.

| Dosya | İçerik |
|---|---|
| `manifest.sha256` | 270 kalemin SHA-256'sı: RG1'in bütün kalemleri (kayıtlı fikstürler — `rd2-*` tarayıcı altınları ve `fixtures/design` dahil —, şema dışa aktarımı, 12 görünüm × PNG/`explain`/`scene`, hesap çıktıları ve dört hesap özeti, uygunluk dökümleri ve her dökümün `qp serve` üstünden gerçek cevap akışı, yansı/görüntüleyici bantları, yeniden oynatma akışı, dört MCP yüzeyi + sabit `tools/call` istekleri, kabuksuz sürüş, görünüş kayıtları, `--help` ağacı, puanlayıcı testleri, test adları ve sayısı) + RD2: `design/check.txt` (`scripts/design-check`: P1–P13, H1–H3 tabloları), `design/contract.txt` (`design-contract --check`), `gallery/` (`design-gallery`'nin çekirdek kısmı: `tavek`, `oltan`, `r4` için her maket yuvasına çizilen grafik PNG'leri, görünüş değişkenleri, P9 düzen dökümleri, koşu manifestosu) |
| `text/` | 100 KB'tan küçük metin kalemlerinin kendisi (fark çıkarsa okumak için; büyükler `05430e06`'dan yeniden üretilir) |
| `lines.txt` | `arch-check --table`, crate başına `.rs` sayısı, en büyük 60 kaynak dosya |
| `verify.log`, `perf.txt` | `scripts/verify.sh` tam koşu; `scripts/refactor/perf.sh` (K17, WASM, K52) |

Belirlenimcilik: tutanak iki kez üretildi (`a`, `b`), `manifest.sha256` bayt eşit.
Test adları `experiments/new-tests.txt`'teki adlar dışarıda tutularak alınır (round'un bilerek
eklediği karakterizasyon ve ek noktası sınamaları).
