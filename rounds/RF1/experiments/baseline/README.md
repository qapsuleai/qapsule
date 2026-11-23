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

## v2 (ön iş Ö'den sonra, kod değişmeden)

Ö (RD2 denetim koşulu K1: P8 çizilen ayrı fikstürde, P9 `design-check`'te; `793a0975`,
denetim `experiments/audit-o.md`) yalnız iki kalemi bilerek değiştirdi; tutanak bunun üstüne
yeniden alındı (manifest v1 ↔ v2 farkı tam bu iki satır):

- `design/check.txt`: `PASS desk-marks` satırı, uygunluk özeti 1 → 2 döküm, her görünüşte P8
  satırı (`record / drawn, desk + desk-marks`) ve P9 satırı; eski görünüşlerin karşılaştırma
  sayacı `oltan` 19 → 20, `r4` 22 → 23 FAIL (çıkış kodunu etkilemez).
- `fixtures/committed.sha256`: iki yeni fikstür (`fixtures/design/desk-marks.json`,
  `fixtures/design/docs/syn03-breakout.json`).

Ö'nün eklediği test `numeric_tokens_match_the_gallery` `new-tests.txt`'te. Öbür 268 kalem v1 ile
aynı.

## Araç düzeltmesi: galeri font yolları (kod değişmeden)

`gallery/vars/fonts-*.css` fontları mutlak dosya adresiyle anıyordu (`file:///…/qapsule/…`).
Bu yüzden worktree'de alınan tutanak hep farklı çıkıyordu; G1 ajanı fark etti. Depo yolu
davranış değildir. `snapshot.sh` artık onu `<root>` diye yazıyor ve tutanağın bu dört satırı aynı
normalleştirmeyle yeniden hesaplandı (dosyaların geri kalanı bayt eşit). Öbür 266 kalem
değişmedi.
