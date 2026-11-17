# RP3/T1 denetim notu (kalibrasyon)

Denetçi: bağımsız alt ajan (işi yapan ajan değil). Tarih 2026-11-17.
Dal `rp3/t1-calib` (2f5856e, temel 6877bda), worktree `.claude/worktrees/agent-a6d0c8cec64bef193`.
Dal değiştirilmedi. Port: 8505.

## Özet

| İddia | Sonuç |
|---|---|
| A — iOS 27, güvenli bağlam olmayan sayfada JIT'siz; WebKit 324968 | **PASS (koşullu)**: kaynak doğru aktarılmış; Arkin'in cihazında HTTPS ile doğrulanmadı |
| B — `__XPC_JSC_useJIT=false` JIT'i kapatıyor; WASM 23×, JS 4× | **PASS**: yeniden üretildi |
| C — PHONE: yüz 31 / sahne 38 / tik 12 ms | **PASS** (yeniden üretim); VERIFY 3 ölçütü (üçü de ±%30) **karşılanmıyor**: yalnız yüz tutuyor. Rapor bunu açıkça söylüyor |
| D — üretim kodu değişmedi; DEVICE.md yalnız ekleme | **PASS** |

## A. WebKit hata kaydı 324968

https://bugs.webkit.org/show_bug.cgi?id=324968 (WebFetch ile okundu):

- Başlık: "iOS 27: JS and Wasm run without JIT on non-secure-context (http://) pages".
  WebKit / JavaScriptCore, Safari 27, iOS 27. Bildiren alexey.pelykh, 2026-09-23 05:24 PDT.
  Durum **RESOLVED INVALID** (son değişiklik 2026-11-09). "INVALID" hata olmadığı, davranışın
  kasıtlı olduğu anlamına geliyor; kapatma gerekçesi yorumlarda ayrıca yazılmamış.
- Bildiren: "The **same page** served over an `https://` origin, on the **same device**,
  minutes later, runs at full JIT speed." / "The behaviour is identical in Safari and in
  Chrome for iOS (WKWebView)" / iOS 26.6'da aynı kod tam hızdaymış; macOS 26.6.2 Safari'de
  düz HTTP ile tam hız.
- Yorum 1, Alexey Proskuryakov (WebKit), 2026-09-23 23:27 PDT: "As far as I know, this is
  intentional, and details are subject to change, so documenting those may not be realistic."
  Özel ağ kökenlerine JIT güveni verilirse "an active network attacker on one's corporate
  network could once again inject malicious content" diyor.
- Yorum 3, ronan_turner, 2026-09-30 08:31 PDT: "I can confirm this is intended behaviour -
  this is part of a feature named Enhanced Security"; "Requests relating to plaintext HTTP are
  one such scenario"; test için "Enable EnhancedSecurity Heuristics" bayrağını kapatmak bunu
  önlemeli.
- Bildirenin telefon ölçümleri (http / https): JS döngüsü ≈ 409–550 / 98 ms (4–5,6×), WASM
  döngüsü ≈ 517–568 / 66 ms (≈ 8×), h3-js latLngToCell ≈ 5.236–5.948 / 204 ms (≈ 26–29×).
  CALIBRATION.md'deki "JS 4–5,6×, h3-js ≈ 26×" doğru aktarılmış.
- Bağlantılı: bug 307047 "Enable Enhanced Security Heuristics flag by default" (FIXED,
  2026-02-05; içinde JIT/HTTP sözü yok), rdar 188805621. **Apple / WebKit resmi
  dokümantasyonu veya blog bağlantısı yok**; bildiren de belgelenmiş bulamadığını yazıyor.

Değerlendirme: kaynak, iddiayı (iOS 27, düz HTTP sayfalarında JS ve WASM JIT'siz; kasıtlı,
"Enhanced Security"; HTTPS tam hız; Chrome-on-iOS da etkilenir) destekliyor. Dikkat edilecekler:
1. "Kasıtlı" ifadesi iki yorumcudan geliyor (Proskuryakov "as far as I know"); resmi belge yok,
   "details are subject to change" ve "heuristics" deniyor — hangi koşulların tetiklediği
   kesin tanımlı değil.
2. HTTPS'in JIT'i geri getirdiği bildirenin kendi cihazında ölçülmüş; **Arkin'in iPhone'unda
   (iOS 27.0.1, CriOS 157) HTTPS karşılaştırması yapılmadı.** Arkin'in koşusunun JIT'siz
   olduğu bir çıkarım: güçlü (PHONE yüz süresini %5 içinde veriyor, mekanizma uyumlu), ama
   doğrudan kanıt değil. Round için belirleyici olduğundan, aynı telefonda HTTPS ölçümü
   CALIBRATION.md'nin de dediği gibi önce yapılmalı.

## B. JIT kapatma (Playwright WebKit 26.5)

`node rounds/RP3/experiments/T1/jit.mjs` iki kez (ilk denemede son ortam,
`__XPC_JSC_useWasmIPInt=false…`, WebContent'i çökertti; tekrarlarda betik bu satıra
gelmeden çıktıyı verdi — CALIBRATION'ın kullanmadığı bir satır). Yük ≈ 17–29.

| ortam | JS ms (1. / 2.) | WASM ms (1. / 2.) |
|---|---:|---:|
| (yok) | 148 / 149 | 27 / 28 |
| `JSC_useJIT=false` | 147 / 148 | 28 / 28 |
| `JSC_useWasmJIT`, `JSC_useBBQJIT+OMG`, `JSC_useOMGJIT` | 147–152 | 27–28 |
| **`__XPC_JSC_useJIT=false`** | **617 / 616** | **645 / 656** |
| `__XPC_JSC_useWasmJIT=false` | 149 / 149 | 29 / 28 |
| `__XPC_JSC_useBBQJIT+OMGJIT=false` | 147 / 148 | 658 / 644 |
| `__XPC_JSC_useOMGJIT=false` | 148 / 148 | 64 / 64 |

Düz `JSC_*` etkisiz, `__XPC_JSC_useJIT=false` etkili: WASM ≈ 23,5×, JS ≈ 4,2×. İddia
aynen doğrulandı. (`useOMGJIT=false` → 2,3×, o da doğru.)

## C. PHONE yapılandırması (6877bda kodu)

- `target/release/qp`: worktree'de `cargo build --release` "Finished … 0.20s" (güncel);
  `git diff 6877bda -- crates shells/web/main.js shells/web/index.html` boş.
- Demo WASM'ını kendim yeniden derledim (`wasm-pack build crates/qp-wasm --target web
  --profile wasm --no-opt -- --features replica`, strip), scratchpad'e; ajanın
  `web-base/pkg` ve `shells/web/pkg` dosyasıyla **bayt bayt aynı**. Koşu benim derlememle.
- Komut: `node tests/calibrate.mjs ../../target/release/qp --config phone --web <benim> --port 8505 --reps 2`
- `uptime` önce 21,69 24,38 21,85 → sonra 13,38 20,87 20,72 (12 P + 4 E çekirdek).

| p50 (ms) | rapor | denetim (2 koşu medyanı) | fark | iPhone | oran |
|---|---:|---:|---:|---:|---:|
| tik | 12 | 11,5 | −%4 | 74 | 0,16 |
| yüz | 31 | 30,5 | −%2 | 32 | 0,95 |
| sahne | 38 | 37,5 | −%1 | 71 | 0,53 |
| p95 tik / yüz / sahne | 47 / 36 / 41 | 47 / 36 / 41 | 0 | | |
| sahne/tik | 0,25 | 0,252 | | | |

Açılan kapsül her iki koşuda `cap.SYN16.breakout.up.200`, ölçek 3, sahne 1170×900, WASM 64 MB.
Raporlanan değerler %5 içinde yeniden üretildi.

Ayrıştırma aritmetiği kontrol edildi (§5.3): yüz ln çarpanları 3,43 / 0,83 / 15,5 / 1,03 →
%32 / −%5 / %72 / %1 (toplam ×46); sahne 4,86 / 0,88 / 12,7 / 1,87 → %34 / −%3 / %55 / %14
(×101). Tutarlı. Kopya + put JIT'siz sahnede 0,5 / 43,5 ms ≈ %1 (< %3 iddiası tutuyor;
yalnız ajanın probe tablosundan, ben probe'u koşmadım).

**Ölçüt durumu:** VERIFY satır 3 "tik, yüz, sahne ±%30" ister. PHONE ile yalnız yüz
(×0,95) içeride; sahne ×0,53, tik ×0,16 dışarıda. CDP16 yüz ve sahneyi sokuyor ama tik yine
dışarıda. Yani **satır 3 bu haliyle karşılanmadı**; rapor bunu gizlemiyor ve nedenini (tik
karışımı, telefonda sahne yolunun orantısız pahalılığı) açıklıyor. Kabul Arkin'in kararı.

## D. Üretim kodu ve DEVICE.md

- `git diff 6877bda..rp3/t1-calib --name-only`: `rounds/` dışında yalnız
  `shells/web/tests/calibrate.mjs` (test düzeneği, yeni dosya). `crates/`, `scripts/`,
  shell kaynakları değişmedi.
- `rounds/R4/DEVICE.md`: tek hunk, dosya sonuna 28 satır ekleme ("## RP3 düzeltmesi"),
  silinen satır 0.
- Üretilmiş `web-*/` dizinleri `.gitignore`'da, dalda izlenmiyor. Sonuç dosyaları yalnız süre
  içeriyor (sentetik veri).

## Notlar

- `jit.mjs`'in son ortam satırı (`__XPC_JSC_useWasmIPInt=false` + BBQ/OMG kapalı) bir
  koşuda tarayıcıyı çökertti ve betik istisnayla çıktı; ölçümü etkilemiyor, betik sağlamlığı
  için not.
- Makine yükü yüksekti (≈ 13–29); buna rağmen sayılar rapordakiyle örtüştü (JIT'siz yol
  tek çekirdek, 1 ms adımlı zamanlayıcı).
