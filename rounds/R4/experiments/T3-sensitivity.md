# R4/T3 — Hassasiyet benzetimi, K42 sonrası (VERIFY 18)

Araç: R3/S-A'nın aracı `crates/qp-capsule/examples/sensitivity.rs` (R3 klasörüne dokunmadan;
çıktı yolu `rounds/R4/experiments/T3/` yapıldı, başlık değişti, hesap aynı).
`qp_capsule::admits` her olayı kendi geçişinin aşamasıyla değerlendirir (olayın hikâye
anlık görüntüsü), bu yüzden K42/A22 kuralı araçta kendiliğinden uygulanır.

```text
cargo run --release -p qp-capsule --example sensitivity -- synthetic
QAPSULE_LOCAL_BARS=…/fixtures/local/bars cargo run --release -p qp-capsule --example sensitivity -- real
```

- **Belirleyici:** her iki koşu iki kez çalıştırıldı, JSON çıktıları bayt eşit (`cmp`).
- **Sistem seviyeyi değiştirmedi** (araçtaki önce/sonra assert): evet.
- Ham çıktılar: `T3/sensitivity.{md,json}` (sentetik 30 × 500 × 3 tohum),
  `T3/sensitivity-real.{md,json}` (yerel gerçek veri, yalnız toplu), K42 öncesi taban
  `T3/sensitivity-real-preK42.{md,json}` (aynı güncel yerel veride `4ee7fc6` kodu; geçici
  bir worktree'de koşuldu, sonra silindi). Sentetiğin K42 öncesi tabanı R3'ün kendi çıktısıdır
  (`rounds/R3/experiments/S/sensitivity.md`; aynı veri, aynı tohumlar).

## Ana sonuç: `important` seviyesindeki doğumların pozisyon/niyet payı

| Veri | Kod | `important`'ta geçen doğum | Yalnız pozisyon/niyetle geçen doğum | Pay |
|---|---|---:|---:|---:|
| Sentetik 30×500×3 | R3 (K42 öncesi) | 2143 | 1229 | %57,3 |
| Sentetik 30×500×3 | R4/T3 (K42) | 906 | 0 | **%0** |
| Gerçek, R3'ün yerel verisi (6 enstrüman) | R3 | 5006 | 4220 | %84,3 (K42'nin dayandığı sayı) |
| Gerçek, güncel yerel veri (31 enstrüman, 1day + 1hour) | K42 öncesi (`4ee7fc6`) | 5984 | 4248 | %71,0 |
| Gerçek, güncel yerel veri | R4/T3 (K42) | 1934 | 0 | **%0** |

Doğum artık pozisyonla seviyeyi aşmıyor; pozisyon/niyet yolundan yalnız olumsuz geçişler
geliyor.

## Seviyeyi pozisyon/niyetle aşan olaylar (havuz, aşamalarıyla)

| Veri | Seviye | K42 öncesi | K42 sonrası |
|---|---|---|---|
| Sentetik | `important` | 3177: born 1229, confirmed 19, expired 97, invalidated 617, realized 515, strengthened 319, weakened 381 | 996: invalidated 617, weakened 379 |
| Sentetik | `balanced` | 2267: born 849, confirmed 12, expired 74, invalidated 441, realized 364, strengthened 203, weakened 324 | 772: invalidated 442, weakened 330 |
| Gerçek (güncel) | `important` | 11113: born 4248, confirmed 17, expired 323, invalidated 1895, realized 2134, strengthened 1179, weakened 1317 | 3206: invalidated 1890, weakened 1316 |
| Gerçek (güncel) | `balanced` | 8864: born 3267, … | 2623: invalidated 1507, weakened 1116 |

`retracted` benzetimde yok (veri düzeltmesi yok). Olumsuz geçiş sayılarındaki küçük farklar
(381 → 379, 1895 → 1890) karne yönünün (A23) kademeyi değiştirmesinden: bazı hikâyeler
kademe süzgecinden zaten geçer hale geldi ya da tersi.

## Günlük kapsül sayısı (kitle; ortalama / medyan / p90 / en çok / sıfır gün)

| Veri | Seviye | K42 öncesi | K42 sonrası |
|---|---|---|---|
| Sentetik | `important` doğum | 1,59 / 1 / 3 / 7 / 0,20 | 0,67 / 1 / 2 / 4 / 0,50 |
| Sentetik | `balanced` doğum | 2,06 / 2 / 4 / 9 / 0,13 | 1,42 / 1 / 3 / 7 / 0,23 |
| Sentetik | `all` doğum | 3,23 / 3 / 6 / 11 / 0,04 | 3,23 / 3 / 6 / 11 / 0,04 |
| Sentetik | `important` tüm olaylar | 4,94 / 5 / 9 / 18 / 0,03 | 3,30 / 3 / 7 / 13 / 0,11 |
| Gerçek (güncel) | `important` doğum | 2,24 / 1 / 7 / 22 / 0,34 | 0,73 / 0 / 2 / 11 / 0,63 |
| Gerçek (güncel) | `balanced` doğum | 2,86 / 1 / 9 / 29 / 0,29 | 1,63 / 1 / 5 / 18 / 0,45 |
| Gerçek (güncel) | `all` doğum | 4,90 / 2 / 16 / 44 / 0,15 | 4,90 / 2 / 16 / 44 / 0,15 |
| Gerçek (güncel) | `important` tüm olaylar | 6,00 / 3 / 19 / 58 / 0,12 | 3,16 / 1 / 10 / 41 / 0,30 |

`all` değişmedi (her şey geçer). `future` grubunun varsayılanı (`important`) artık gerçekten
"az ama önemli": sentetikte günde ortalama 0,67 doğum, günlerin yarısı boş.

## Piyasa geneli (kitle yok), karne yönünün etkisi

| Veri | `important` doğum, K42 öncesi | sonrası |
|---|---|---|
| Sentetik | 1,79 / 2 / 4 / 8 / 0,15 | 1,79 / 2 / 4 / 8 / 0,16 |
| Gerçek (güncel) | 1,78 / 1 / 5 / 38 / 0,47 | 1,97 / 1 / 6 / 38 / 0,41 |

Kitlesiz sayılar yalnız karne yönü (A23) yüzünden oynar (karne → önem → kademe). Aracın
markdown'ındaki "M3 ile aynı sayılar" cümlesi aracın sabit metnidir; sentetikte sıfır gün
payı 0,150 → 0,16 oldu (M3 R3 karnesiyle ölçülmüştü).

## Okuma

- K42/A22 ölçülen etkiyi verdi: `important` seviyesine pozisyon/niyet yoluyla doğum hiç
  girmiyor (%57–84 → %0). Pozisyonu olan kişi kendi hissesindeki olumsuz gelişmeyi
  (zayıfladı, geçersiz) seviyesi ne olursa olsun alıyor; günlük yük `important`'ta yaklaşık
  üçte birine indi.
- Açık soru (öneri): `important`'ta sıfır gün payı 0,50–0,63; "geleceğin yatırımcısı" için çok
  sessiz mi? İnsan testi ve M9 (akış düzeni) bunu söylemeli; kademe eşiği karnesi yönlü
  olunca yeniden ayarlanabilir (M3 tekrarı).
