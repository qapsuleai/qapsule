# R4/M11 — "Gerçekleşti" ile "tutmadı"nın işarette ayrımı: sonuç

> Persona ajanı insan değildir; model küçük pikseli ve kısa bakışı insan gibi deneyimlemez.
> Bu ölçüm **ön elemedir**; son söz insan testi v2'dedir (SPEC §0.5).

## Düzen ve sıra (VERIFY 14)

| Adım | Commit | Dal |
|---|---|---|
| Şartname (SPEC §5, seçim kuralı) | `bad3d67` | `r4/int` |
| Düzenek + `OutcomeStyle` arayüzü + başvuru `r3` ölçüsü | `8e3b33e`, `2db6d25` | `r4/h-m11` → `r4/int` `4151112` |
| Alternatif X: `muhrak` ("mühür / kırık mühür") | `187edc2` (+ `clamp` düzeltmesi) | `r4/m11-x` |
| Alternatif Y: `tg5` ("negatif karo / sönük iz") | `10fabcd` | `r4/m11-y` |
| Okuyucu koşuları ve puanlama | bu commit | `r4/eval-m11` |

İki alternatif ayrı alt ajanlarca, ayrı worktree'lerde, birbirinin dosyasına bakmamaları
söylenerek yazıldı; ikisine de yalnız birer başlangıç yönü verildi (X: glifin bütününün
dönüşümü; Y: glifin yanında büyük, şekilce zıt bir hüküm işareti). Sonuçta ikisi de
"gerçekleşti" için koyu dolu levha fikrine vardı (bağımsız yakınsama); ayrıştıkları yer
"tutmadı" (X: boş çerçeve + çatlak; Y: soluk glif + köşede kalın ✗) ve "gerçekleşti"de köşe
onayı (Y'de var, X'te yok).

## Körleme

Okuyucu sayfaları tohumlu karma kodla adlandırıldı (`raw/screens/<kod>.png`); stil eşlemesi
`raw/key.json` ve `raw/keys/`'te, okuyucu dizinine kopyalanmadı. Her istem gönderilmeden önce
`m11 check-prompt` ile yasaklı sözcük (stil adları, dal adları, `alternatif`, `variant`…)
denetimi yapıldı. Okuyucu: taze `claude -p --model sonnet --restricted --strict-mcp-config
--tools Read --permission-mode dontAsk --no-session-persistence`, çalışma dizininde yalnız
tek PNG. Eleştirmen ajanı bu deneyde yok (SPEC §5).

## Ölçüler

**(a) Hesap — en kötü koşulda en zayıf `realized ↔ failed` çifti (ΔE76)**, 16/24/32 px ×
açık/koyu × normal + üç renk körlüğü + gri:

| stil | siluet | rozet | eleme (alfabe başına başvurunun altı) |
|---|---:|---:|---|
| `r3` (başvuru, R3 onay/çarpı) | 2,98 | 11,33 | — |
| `muhrak` | 40,49 | 22,93 | geçti |
| `tg5` | 42,11 | 16,44 | geçti |

**(b) Persona** (siluet, 4×4 ızgara, 32 px × 4 en yakın komşu; persona × koşul × 3 okuyucu;
16 işaretin hepsi doğruysa 1):

| stil | F açıklamasız | H açıklamasız | ağırlıklı (F2, H1) | F açıklamalı | H açıklamalı |
|---|---:|---:|---:|---:|---:|
| `r3` | 1,00 | 1,00 | 1,00 | 1,00 | 1,00 |
| `muhrak` | 0,06 | 0,17 | 0,10 | 1,00 | 1,00 |
| `tg5` | 1,00 | 1,00 | 1,00 | 1,00 | 1,00 |

Şans düzeyi işaret başına 0,33, çoğunluk tabanı 0,38. Ham kayıtlar `raw/readers/`, puanlar
`raw/scores.{json,md}` (`m11 score`, ham cevaptan belirleyici). Okuyucu maliyeti toplam
≈ 0,43 $ (36 koşu).

## Seçim kuralının uygulanışı (SPEC §5, önceden yazılı)

1. ΔE eşiği: iki alternatif de iki alfabede başvurunun üstünde → elenen yok.
2. Birincil (açıklamasız ağırlıklı doğruluk): `tg5` 1,00 − `muhrak` 0,10 = 0,90 ≥ 0,05 →
   **`tg5`**.

**Seçim (öneri): `tg5`.** Varsayılan değişikliği bütünleştirmede (S) yeni altınla yapılır;
`r3` adla seçilebilir kalır, var olan `g1` altınları değişmez.

## Gözlemler ve sınır

- **Tavan etkisi:** başvuru `r3` okumada 1,00 aldı; ΔE'si ise siluette 2,98 (renk körlüğü +
  16 px koşulunda neredeyse görünmez). Model 4× büyütülmüş 32 px sayfada birkaç piksellik
  onay/çarpıyı bir insanın telefonda göremeyeceği kadar iyi okuyor. Bu yüzden okuma testi
  `r3` ile `tg5`'i ayıramaz; ayrımı hesap ölçüsü (×14 / ×1,45) yapıyor. İnsan testi v2'de
  `r3` ↔ `tg5` karşılaştırması gerçek boyutta (büyütmesiz) yapılmalı.
- **`muhrak` açıklamasız sistematik yanlış:** okuyucular koyu dolu levhayı "gerçekleşti"
  dışında okudu (tasarımcının DESIGN.md'de öngördüğü zayıflık: levhada onay yok, "güçlü /
  seçili" gibi okunuyor); açıklamayla 1,00'e çıkıyor. Yani öğrenilebilir ama kendiliğinden
  okunmuyor. `tg5`'in köşe ✓/✗'si bu belirsizliği kaldırıyor.
- İki alternatifin ortak zayıflığı: gerçekleşen kapsül akışta en koyu öğe olur; kapanmış bir
  kapsül açık olandan çok göze batabilir. M9 (akış) ve insan testinde izlenmeli.
- T3'ün açık sorusu ("artık geçerli değil" rozeti 32 px'te ✓'ye benziyor): `tg5`'in ✓'si
  dolu açık disk içinde koyu, geçerlilik rozeti halka + çizgi; insan testinde ayrıca bakılmalı.
