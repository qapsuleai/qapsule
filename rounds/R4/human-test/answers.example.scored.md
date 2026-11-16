# İnsan testi v2 — puan raporu

Girdi: `answers.example.csv` · anahtar: `key.json` · plan: `plan.json`. Ağırlık: genç 2, alışkın 1 (persona F/H gibi). Persona ajanı insan değildir; bu rapor insan sonucudur.

> **Uyarı:** `plan.json` yüz adları doldurulmamış; yüz satırlarında yer tutucu görünür.

## Katılımcılar

Toplam 6: alışkın 3, genç 3. Gösterim: ekran 5, masaüstü 1.

| katılımcı | G5 önce | G1 önce | G3 önce | G4 önce | G2 önce |
|---|---|---|---|---|---|
| A01 | `siluet` | `<<YUZ1>>` | `zumlak` | `tg5` | `vakiro` |
| A02 | `rozet` | `<<YUZ2>>` | `tutamir` | `r3` | `elikov` |
| A03 | `siluet` | `<<YUZ2>>` | `zumlak` | `r3` | `elikov` |
| G01 | `siluet` | `<<YUZ1>>` | `zumlak` | `tg5` | `vakiro` |
| G02 | `rozet` | `<<YUZ2>>` | `tutamir` | `r3` | `elikov` |
| G03 | `siluet` | `<<YUZ2>>` | `zumlak` | `r3` | `elikov` |

## G5 — alfabe (işaret): `siluet` (A) ile `rozet` (B)

Birincil ölçü: işaret başına tam doğru (yön + ne + durum).

| tasarım | grup | n | birincil | yon | ne | durum | süre medyan (sn) | ipucu |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| `siluet` | alışkın | 3 | 0,33 | 0,78 (7/9) | 0,78 (7/9) | 0,67 (6/9) | 20,00 | 0 |
| `siluet` | genç | 3 | 0,11 | 0,44 (4/9) | 0,78 (7/9) | 0,44 (4/9) | 31,00 | 0 |
| `siluet` | hepsi | 6 | 0,22 | 0,61 (11/18) | 0,78 (14/18) | 0,56 (10/18) | 27,00 | 0 |
| `rozet` | alışkın | 3 | 0,89 | 1,00 (9/9) | 0,89 (8/9) | 1,00 (9/9) | 25,00 | 0 |
| `rozet` | genç | 3 | 0,56 | 0,89 (8/9) | 0,89 (8/9) | 0,78 (7/9) | 31,00 | 0 |
| `rozet` | hepsi | 6 | 0,72 | 0,94 (17/18) | 0,89 (16/18) | 0,89 (16/18) | 28,50 | 0 |

Sıra etkisi (birincil, ilk / ikinci gösterilen): `siluet` 0,17 / 0,33 · `rozet` 0,67 / 0,75

| katılımcı | grup | `siluet` | `rozet` | fark (A − B) | süre A | süre B | tercih |
|---|---|---:|---:|---:|---:|---:|---|
| A01 | alışkın | 0,67 | 1,00 | -0,33 | 18,00 | 34,00 | rozet |
| A02 | alışkın | 0,33 | 1,00 | -0,67 | 20,00 | 17,00 | rozet |
| A03 | alışkın | 0,00 | 0,67 | -0,67 | 37,00 | 25,00 | fark yok |
| G01 | genç | 0,00 | 0,33 | -0,33 | 23,00 | 31,00 | rozet |
| G02 | genç | 0,33 | 0,33 | 0,00 | 31,00 | 26,00 | siluet |
| G03 | genç | 0,00 | 1,00 | -1,00 | 34,00 | 34,00 | rozet |

- Eşli: 6 katılımcı; A daha iyi 0, B daha iyi 5, eşit 1; ortalama fark -0,50, ağırlıklı fark -0,48; işaret testi p = 0,06 (bilgi için; güç düşük).
- Gruplar: alışkın ort. fark -0,56 (n=3), genç ort. fark -0,44 (n=3)
- Süre farkı (A − B, katılımcı medyanlarının medyanı): 1,50 sn (eksi: A hızlı).
- Tercih: `siluet` 1, `rozet` 4, fark yok 1.

## G1 — kapsülün yüzü (ilk bakış): `<<YUZ1>>` (A) ile `<<YUZ2>>` (B)

Birincil ölçü: yüz başına tam doğru (S1–S5).

| tasarım | grup | n | birincil | s1 | s1b | s2 | s3 | s4 | s5 | s2+s3 | süre medyan (sn) | ipucu |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `<<YUZ1>>` | alışkın | 3 | 0,78 | 1,00 (9/9) | 1,00 (9/9) | 0,89 (8/9) | 0,89 (8/9) | 1,00 (9/9) | 1,00 (9/9) | 0,78 (7/9) | 43,00 | 0 |
| `<<YUZ1>>` | genç | 3 | 0,11 | 0,89 (8/9) | 0,56 (5/9) | 0,89 (8/9) | 0,89 (8/9) | 0,78 (7/9) | 0,89 (8/9) | 0,78 (7/9) | 32,00 | 0 |
| `<<YUZ1>>` | hepsi | 6 | 0,44 | 0,94 (17/18) | 0,78 (14/18) | 0,89 (16/18) | 0,89 (16/18) | 0,89 (16/18) | 0,94 (17/18) | 0,78 (14/18) | 39,00 | 0 |
| `<<YUZ2>>` | alışkın | 3 | 0,22 | 0,56 (5/9) | 1,00 (9/9) | 0,89 (8/9) | 0,89 (8/9) | 0,67 (6/9) | 0,67 (6/9) | 0,78 (7/9) | 37,00 | 0 |
| `<<YUZ2>>` | genç | 3 | 0,22 | 0,56 (5/9) | 0,89 (8/9) | 0,78 (7/9) | 0,33 (3/9) | 0,89 (8/9) | 0,78 (7/9) | 0,33 (3/9) | 36,00 | 0 |
| `<<YUZ2>>` | hepsi | 6 | 0,22 | 0,56 (10/18) | 0,94 (17/18) | 0,83 (15/18) | 0,61 (11/18) | 0,78 (14/18) | 0,72 (13/18) | 0,56 (10/18) | 37,00 | 0 |

Sıra etkisi (birincil, ilk / ikinci gösterilen): `<<YUZ1>>` 0,33 / 0,50 · `<<YUZ2>>` 0,08 / 0,50

| katılımcı | grup | `<<YUZ1>>` | `<<YUZ2>>` | fark (A − B) | süre A | süre B | tercih |
|---|---|---:|---:|---:|---:|---:|---|
| A01 | alışkın | 0,67 | 0,33 | 0,33 | 43,00 | 37,00 | <<YUZ1>> |
| A02 | alışkın | 0,67 | 0,33 | 0,33 | 29,00 | 33,00 | <<YUZ1>> |
| A03 | alışkın | 1,00 | 0,00 | 1,00 | 50,00 | 40,00 | <<YUZ1>> |
| G01 | genç | 0,00 | 0,67 | -0,67 | 30,00 | 33,00 | <<YUZ1>> |
| G02 | genç | 0,33 | 0,00 | 0,33 | 32,00 | 35,00 | <<YUZ1>> |
| G03 | genç | 0,00 | 0,00 | 0,00 | 34,00 | 52,00 | <<YUZ1>> |

- Eşli: 6 katılımcı; A daha iyi 4, B daha iyi 1, eşit 1; ortalama fark 0,22, ağırlıklı fark 0,11; işaret testi p = 0,38 (bilgi için; güç düşük).
- Gruplar: alışkın ort. fark 0,56 (n=3), genç ort. fark -0,11 (n=3)
- Süre farkı (A − B, katılımcı medyanlarının medyanı): -3,00 sn (eksi: A hızlı).
- Tercih: `<<YUZ1>>` 6, `<<YUZ2>>` 0, fark yok 0.

## G3 — açılma geçişi: `zumlak` (A) ile `tutamir` (B)

Birincil ölçü: kapsül başına ortalama (yer, aynı şey, hâlâ geçerli).

| tasarım | grup | n | birincil | yer | ayni | s3 | süre medyan (sn) | ipucu |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| `zumlak` | alışkın | 3 | 0,89 | 0,83 (5/6) | 0,83 (5/6) | 1,00 (6/6) | 7,00 | 0 |
| `zumlak` | genç | 3 | 0,67 | 0,83 (5/6) | 0,33 (2/6) | 0,83 (5/6) | 6,50 | 0 |
| `zumlak` | hepsi | 6 | 0,78 | 0,83 (10/12) | 0,58 (7/12) | 0,92 (11/12) | 6,50 | 0 |
| `tutamir` | alışkın | 3 | 0,83 | 0,83 (5/6) | 0,83 (5/6) | 0,83 (5/6) | 7,00 | 0 |
| `tutamir` | genç | 3 | 0,78 | 0,83 (5/6) | 0,50 (3/6) | 1,00 (6/6) | 8,50 | 0 |
| `tutamir` | hepsi | 6 | 0,81 | 0,83 (10/12) | 0,67 (8/12) | 0,92 (11/12) | 7,00 | 0 |

`zumlak` — "nerede olduğumu kaybettim" (1 hiç … 5 tamamen) ortalaması: 2,25 (n=12)

`tutamir` — "nerede olduğumu kaybettim" (1 hiç … 5 tamamen) ortalaması: 1,83 (n=12)


Sıra etkisi (birincil, ilk / ikinci gösterilen): `zumlak` 0,75 / 0,83 · `tutamir` 0,92 / 0,75

| katılımcı | grup | `zumlak` | `tutamir` | fark (A − B) | süre A | süre B | tercih |
|---|---|---:|---:|---:|---:|---:|---|
| A01 | alışkın | 1,00 | 0,83 | 0,17 | 5,00 | 5,50 | tutamir |
| A02 | alışkın | 0,83 | 1,00 | -0,17 | 5,50 | 7,50 | tutamir |
| A03 | alışkın | 0,83 | 0,67 | 0,17 | 8,00 | 9,50 | tutamir |
| G01 | genç | 0,50 | 0,67 | -0,17 | 4,50 | 6,00 | tutamir |
| G02 | genç | 0,83 | 0,83 | 0,00 | 6,50 | 12,00 | tutamir |
| G03 | genç | 0,67 | 0,83 | -0,17 | 8,00 | 7,50 | tutamir |

- Eşli: 6 katılımcı; A daha iyi 2, B daha iyi 3, eşit 1; ortalama fark -0,03, ağırlıklı fark -0,06; işaret testi p = 1,00 (bilgi için; güç düşük).
- Gruplar: alışkın ort. fark 0,06 (n=3), genç ort. fark -0,11 (n=3)
- Süre farkı (A − B, katılımcı medyanlarının medyanı): -1,50 sn (eksi: A hızlı).
- Tercih: `zumlak` 0, `tutamir` 6, fark yok 0.

## G4 — sonuç işareti: `tg5` (A) ile `r3` (B)

Birincil ölçü: işaret başına sonuç doğru.

| tasarım | grup | n | birincil | sonuc | süre medyan (sn) | ipucu |
|---|---|---:|---:|---:|---:|---:|
| `tg5` | alışkın | 3 | 0,89 | 0,89 (8/9) | 29,00 | 0 |
| `tg5` | genç | 3 | 0,89 | 0,89 (8/9) | 32,00 | 0 |
| `tg5` | hepsi | 6 | 0,89 | 0,89 (16/18) | 30,50 | 0 |
| `r3` | alışkın | 3 | 0,78 | 0,78 (7/9) | 17,00 | 0 |
| `r3` | genç | 3 | 0,44 | 0,44 (4/9) | 24,00 | 0 |
| `r3` | hepsi | 6 | 0,61 | 0,61 (11/18) | 20,50 | 0 |

Sıra etkisi (birincil, ilk / ikinci gösterilen): `tg5` 0,83 / 0,92 · `r3` 0,67 / 0,50

| katılımcı | grup | `tg5` | `r3` | fark (A − B) | süre A | süre B | tercih |
|---|---|---:|---:|---:|---:|---:|---|
| A01 | alışkın | 0,67 | 0,33 | 0,33 | 29,00 | 17,00 | tg5 |
| A02 | alışkın | 1,00 | 1,00 | 0,00 | 34,00 | 15,00 | tg5 |
| A03 | alışkın | 1,00 | 1,00 | 0,00 | 23,00 | 17,00 | tg5 |
| G01 | genç | 1,00 | 0,67 | 0,33 | 36,00 | 31,00 | tg5 |
| G02 | genç | 1,00 | 0,67 | 0,33 | 32,00 | 24,00 | fark yok |
| G03 | genç | 0,67 | 0,00 | 0,67 | 26,00 | 24,00 | tg5 |

- Eşli: 6 katılımcı; A daha iyi 4, B daha iyi 0, eşit 2; ortalama fark 0,28, ağırlıklı fark 0,33; işaret testi p = 0,12 (bilgi için; güç düşük).
- Gruplar: alışkın ort. fark 0,11 (n=3), genç ort. fark 0,44 (n=3)
- Süre farkı (A − B, katılımcı medyanlarının medyanı): 7,00 sn (eksi: A hızlı).
- Tercih: `tg5` 5, `r3` 0, fark yok 1.

## G2 — akış düzeni: `elikov` (A) ile `vakiro` (B)

Birincil ölçü: pencere başına ortalama (G2a F1, G2b, G2c).

| tasarım | grup | n | birincil | g2a | g2a_enstrüman | g2b | g2c | süre medyan (sn) | ipucu |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `elikov` | alışkın | 3 | 0,44 | 0,67 | 0,67 | 0,33 (1/3) | 0,33 (1/3) | 15,00 | 0 |
| `elikov` | genç | 3 | 0,22 | 0,33 | 0,33 | 0,33 (1/3) | 0,00 (0/3) | 11,00 | 1 |
| `elikov` | hepsi | 6 | 0,33 | 0,50 | 0,50 | 0,33 (2/6) | 0,17 (1/6) | 12,00 | 1 |
| `vakiro` | alışkın | 3 | 0,89 | 1,00 | 1,00 | 1,00 (3/3) | 0,67 (2/3) | 19,00 | 0 |
| `vakiro` | genç | 3 | 0,67 | 0,67 | 0,67 | 0,67 (2/3) | 0,67 (2/3) | 17,00 | 0 |
| `vakiro` | hepsi | 6 | 0,78 | 0,83 | 0,83 | 0,83 (5/6) | 0,67 (4/6) | 18,00 | 0 |

Sıra etkisi (birincil, ilk / ikinci gösterilen): `elikov` 0,33 / 0,33 · `vakiro` 0,67 / 0,83

| katılımcı | grup | `elikov` | `vakiro` | fark (A − B) | süre A | süre B | tercih |
|---|---|---:|---:|---:|---:|---:|---|
| A01 | alışkın | 0,33 | 1,00 | -0,67 | 10,00 | 19,00 | vakiro |
| A02 | alışkın | 0,67 | 1,00 | -0,33 | 21,00 | 15,00 | vakiro |
| A03 | alışkın | 0,33 | 0,67 | -0,33 | 15,00 | 26,00 | vakiro |
| G01 | genç | 0,33 | 0,33 | 0,00 | 10,00 | 8,00 | vakiro |
| G02 | genç | 0,33 | 1,00 | -0,67 | 10,00 | 17,00 | vakiro |
| G03 | genç | 0,00 | 0,67 | -0,67 | 19,00 | 21,00 | vakiro |

- Eşli: 6 katılımcı; A daha iyi 0, B daha iyi 5, eşit 1; ortalama fark -0,44, ağırlıklı fark -0,44; işaret testi p = 0,06 (bilgi için; güç düşük).
- Gruplar: alışkın ort. fark -0,44 (n=3), genç ort. fark -0,44 (n=3)
- Süre farkı (A − B, katılımcı medyanlarının medyanı): -4,50 sn (eksi: A hızlı).
- Tercih: `elikov` 0, `vakiro` 6, fark yok 0.

## Karar (önceden yazılmış kural, `DECISION.md`)

Persona ajanı insan değildir: aşağıdaki insan sonucu persona ölçümünden (M8–M11) farklıysa insan sonucu geçerlidir.

| karşılaştırma | ön seçim (A) | sonuç | gerekçe |
|---|---|---|---|
| A21 alfabe | `siluet` | **`rozet` (değişir)** | birincil ölçüde ağırlıklı fark -0,48 (eşik 0,10), işaret 0–5 |
| yüz (M8 ilk iki) | `<<YUZ1>>` | **`<<YUZ1>>` (onay)** | gruplar ayrışıyor (alışkın 0,56, genç -0,11), kural 1 uygulanmadı; tercih 6–0 (en az ⅔ ve çoğunluk) |
| M10 geçiş | `zumlak` | **`tutamir` (değişir)** | doğruluk farkı belirleyici değil (-0,06); tercih 0–6 (en az ⅔ ve çoğunluk) |
| M11 sonuç işareti | `tg5` | **`tg5` (onay)** | birincil ölçüde ağırlıklı fark 0,33 (eşik 0,10), işaret 4–0 |
| M9 akış düzeni | `elikov` | **`vakiro` (değişir)** | birincil ölçüde ağırlıklı fark -0,44 (eşik 0,10), işaret 0–5 |

## Katılımcının kendi sözleri ve yürütücü notları

- A02 G1-1A s3: "çarpı var, demek ki tutmamış"
- G02 G2-1 g2c: (not: kaydırmadan önce ipucu istedi)
- G03 T serbest: Kapanmış olanlar açık olanlardan daha çok göze batıyor.
