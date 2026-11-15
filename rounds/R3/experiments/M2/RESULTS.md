# M2 — Cümle üretimi: sonuçlar ve seçim

Düzenek: `rounds/R3/experiments/harness/` (M2 bölümü; commit `71fd241`, alternatiflerden önce).
Alternatifler ayrı dallarda, ayrı alt ajanlarca, birbirini görmeden: **M2a `template`**
(`r3/m2a-template` `85b5b5d`), **M2b `llmslot`** (`r3/m2b-llmslot` `4b9a81e`; dil modeli
yalnız karşılaştırma için, `claude-sonnet-5-5`). Değerlendirme dalı `r3/eval-m2` (ikisi
birlikte kayıtlı). Okuma testleri orkestratörce, alternatifler bittikten sonra koşuldu
(aynı tohumlar 1–2, aynı 24 durum; okuyucu `claude -p --model sonnet`, taze süreç, araçsız).

## Ham sonuçlar

| Ölçü | template TR | template EN | llmslot TR | llmslot EN |
|---|---:|---:|---:|---:|
| Zaman doğruluğu (73 durum, otomatik) | 1,00 | 1,00 | 1,00 | 1,00 |
| K26 (otomatik) | doğru | doğru | doğru | doğru |
| Kesin zaman başvurusu | 0,83 | 0,83 | 0,83 | 0,83 |
| Belirleyicilik (iki üretim bayt eşit) | **1,00** | **1,00** | 0,08 | 0,14 |
| Sızıntı (lisanslı, 18 durum) | 0/18 | 0/18 | 0/18 | 0/18 |
| Uydurma sayı (55 durum) | 0 | 0 | 0 | 0 |
| Anlama — tam doğru (2 okuyucu × 24) | **0,98** | **0,94** | 0,88 | 0,71 |
| Anlama — "hâlâ geçerli mi" | 1,00 | 1,00 | 0,92 | 0,79 |
| Anlama — K26 tam doğru | 2/2 | 2/2 | 2/2 | 2/2 |
| Maliyet / cümle | ~1–2 µs, 0 $ | ~1 µs, 0 $ | ~8 s, ~0,006 $ | (aynı çağrı) |

Kesin zaman başvurusundaki 6 ıska ikisinde de aynı: rakamsız (lisanslı) durumlarda "5/30 bar
önce" sözle ("birkaç gün önce") söyleniyor; tasarım gereği.

Ham kayıtlar: `M2/raw/{template,llmslot,llmslot_run2}/` (`auto.json`, `readers/…`), okuma
koşusunun günlüğü `M2/raw/readers-run.log`.

## Seçim: **M2a — şablon dilbilgisi**

- Anlama testinde belirgin üstün (TR 0,98 / EN 0,94 karşı 0,88 / 0,71); fark "hâlâ geçerli
  mi" sorusunda: dil modelinin cümleleri uzun ve geçerliliği daha az açık söylüyor.
- Belirleyici (değişmez: aynı veri → aynı cümle); dil modeli iki koşuda cümlelerin %86–92'sini
  farklı yazdı.
- Maliyetsiz, ağsız; cihazda ve WASM yansısında çalışır (K21, `docs/MODELS.md`).
- Sızıntı ve uydurma sayıda ikisi eşit (0); dil modelinin 0'ı doğrulayıcının reddi ve yeniden
  istemiyle elde edildi (146 metinde 1 + 3 ret).

## Sınırlar

- Otomatik zaman sınıflayıcısı yüzey işaretlerine bakar; M2b'nin doğrulayıcısı aynı kuralları
  uyguladığı için onun 1,00'ı kısmen kendini doğrulayan bir ölçü. Bağımsız ölçü okuma testi.
- Okuyucu bir dil modelidir, insan değil; okuyucu sayısı küçük (alternatif × dil başına 2).
- Şablon cümleleri 3–5 kısa cümle ve mekanik; her olay cümlesi "hâlâ geçerli olup olmadığı
  bilinmiyor" diyor (düzeneğin doğrusu bunu istiyor). İnsan testinde sınanmalı.
- Şablonda iki eşik ölçülmeden seçildi: karne cümlesi için n ≥ 20, nitel karne için %60/%40.
- M2b'nin bulduğu ortak bir motor sorunu: aşağı yönlü akış yuvası "net ortalamanın altında"
  okunuyor; "net satış ortalamanın altına düştü" yanlış anlaşılır. Yuva anlamı S'de düzeltilir.
