# M29 — Kapsülün ajana veriliş biçimi (sonuç)

Düzenek: harness SPEC §5.5 (kural ölçümden önce yazılı). Seçilen yüzey `thin` üstünde üç
biçim (`json` = protokol cevabı olduğu gibi, `card` = sıkıştırılmış kapsül kartı metni,
`layered` = özet + ayrıntı için ayrı araç), okuma görevleri (r1 + r2 = F, E, V, C, G; 31 görev),
3 model (haiku, sonnet, opus) = 18 koşu, 558 görev cevabı. Sentetik oturum; dökümler depoya girmedi.

## Önce: geçersiz ilk koşu (A/A)

İlk 18 koşu geçersizdi: `thin` araçlarında `format` alanı yoktu, `--capsule-format` hiçbir
şeyi değiştirmedi (sunucu günlüğünde `query_capsule` cevap boyları üç biçimde aynı). Düzeltme
(commit "thin yüzeyi sunucunun kapsül biçimini uygular"): sunucu biçimi `card`/`layered` ise
`query_capsule` ve `query_capsules` cevabı diğer yüzeylerle aynı çiziciyle yazılır; `json`
değişmez. Geçersiz koşu, **aynı koşulun üç kez ölçümü** olarak gürültüyü gösterir: ilk deneme
oranları 0,871 / 0,882 / 0,914 (aynı biçim!) — yani ~4 puanlık fark tek başına gürültüdür.
Toplam alt ajan belirteci (geçersiz koşu): 1 643 260.

## Seçim: `json` (değişiklik yok)

| biçim | birincil (model ort.) | haiku | sonnet | opus | havuz | Wilson %95 | sayı aktarım hatası | belirteç/görev | çağrı |
|---|---:|---:|---:|---:|---:|---|---:|---:|---:|
| card | 0,935 | 0,871 | 0,968 | 0,968 | 87/93 | [0,866, 0,970] | 0/21 | 6174 | 2,3 |
| layered | 0,925 | 0,871 | 0,968 | 0,935 | 86/93 | [0,853, 0,963] | 0/21 | 6141 | 1,9 |
| json | 0,903 | 0,806 | 0,935 | 0,968 | 84/93 | [0,826, 0,948] | 0/21 | 5952 | 1,7 |

Aralıklar örtüşüyor → "ayırt edilemedi" → (a) sayı aktarım hatası hepsinde 0 → (b) görev
başına belirteç en düşük **json** (5952). Kural `json`'u seçer; sunucunun varsayılanı zaten
budur (`--capsule-format` verilmezse protokol cevabı).

**Not (kurala girmez):** küçük modelde kart ve katmanlı 27/31, json 25/31; fark A/A gürültüsü
boyunda. `card` ürün için ileride yeniden ölçülmeye değer (daha büyük n); `--capsule-format card`
seçeneği kalır (bağlantı kılavuzunda anlatıldı).

Sızıntı 0, ajan karar denemesi 0 (bütün koşularda). Tablo: `summary.json`; seçim girdileri `selection.json`.
