# İnsan testi — puan raporu

Girdi: `answers.example.csv` · anahtar: `key.json` (tohum 1) · M = siluet, N = rozet.

## Katılımcılar

| grup | kitapçık A (siluet önce) | kitapçık B (rozet önce) | toplam |
|---|---:|---:|---:|
| alışkın | 2 | 1 | 3 |
| genç | 1 | 2 | 3 |
| **toplam** | 3 | 3 | 6 |

## İşaretler: alan başına doğruluk

Şans düzeyi: tür 0,17 · yön 0,33 · güç 0,50 · aşama 0,20 · tam 0,01. Çoğunluk tabanı (anahtardaki en sık doğru değeri hep söylemek) aşağıda ayrıca.

### rozet (Alfabe N)

| | tür | yön | güç | aşama | tam doğru |
|---|---:|---:|---:|---:|---:|
| açıklamasız (A1–B4) | 0,54 (26/48) | 0,81 (39/48) | 0,83 (40/48) | 0,60 (29/48) | 0,17 (8/48) |
| açıklamalı (C1–D4) | 0,88 (42/48) | 1,00 (48/48) | 0,94 (45/48) | 0,90 (43/48) | 0,75 (36/48) |
| öğrenme (L1–L4) | 0,83 (20/24) | 0,88 (21/24) | 0,92 (22/24) | 0,75 (18/24) | 0,50 (12/24) |
| hepsi (20 madde) | 0,73 (88/120) | 0,90 (108/120) | 0,89 (107/120) | 0,75 (90/120) | 0,47 (56/120) |

Çoğunluk tabanı (20 madde): kind 0,20 (`continuation`) · direction 0,45 (`up`) · strength 0,50 (`low`) · phase 0,25 (`final`)

### siluet (Alfabe M)

| | tür | yön | güç | aşama | tam doğru |
|---|---:|---:|---:|---:|---:|
| açıklamasız (A1–B4) | 0,75 (36/48) | 0,77 (37/48) | 0,83 (40/48) | 0,71 (34/48) | 0,38 (18/48) |
| açıklamalı (C1–D4) | 0,92 (44/48) | 0,90 (43/48) | 0,94 (45/48) | 0,94 (45/48) | 0,69 (33/48) |
| öğrenme (L1–L4) | 0,79 (19/24) | 1,00 (24/24) | 0,88 (21/24) | 0,96 (23/24) | 0,71 (17/24) |
| hepsi (20 madde) | 0,82 (99/120) | 0,87 (104/120) | 0,88 (106/120) | 0,85 (102/120) | 0,57 (68/120) |

Çoğunluk tabanı (20 madde): kind 0,20 (`flow`) · direction 0,45 (`up`) · strength 0,50 (`low`) · phase 0,25 (`final`)

## Gruba göre (bütün 20 madde)

| alfabe · grup | tür | yön | güç | aşama | tam doğru |
|---|---:|---:|---:|---:|---:|
| rozet · alışkın | 0,78 (47/60) | 0,95 (57/60) | 0,92 (55/60) | 0,77 (46/60) | 0,52 (31/60) |
| rozet · genç | 0,68 (41/60) | 0,85 (51/60) | 0,87 (52/60) | 0,73 (44/60) | 0,42 (25/60) |
| siluet · alışkın | 0,85 (51/60) | 0,90 (54/60) | 0,93 (56/60) | 0,90 (54/60) | 0,65 (39/60) |
| siluet · genç | 0,80 (48/60) | 0,83 (50/60) | 0,83 (50/60) | 0,80 (48/60) | 0,48 (29/60) |

## Sıra etkisi (bölüm 1 mi bölüm 2 mi)

| alfabe · sıra | tür | yön | güç | aşama | tam doğru |
|---|---:|---:|---:|---:|---:|
| rozet · 1. bölüm | 0,73 (44/60) | 0,88 (53/60) | 0,87 (52/60) | 0,78 (47/60) | 0,48 (29/60) |
| rozet · 2. bölüm | 0,73 (44/60) | 0,92 (55/60) | 0,92 (55/60) | 0,72 (43/60) | 0,45 (27/60) |
| siluet · 1. bölüm | 0,87 (52/60) | 0,87 (52/60) | 0,90 (54/60) | 0,87 (52/60) | 0,62 (37/60) |
| siluet · 2. bölüm | 0,78 (47/60) | 0,87 (52/60) | 0,87 (52/60) | 0,83 (50/60) | 0,52 (31/60) |

## Güven ve doğruluk (tür alanı)

| alfabe | tür doğruyken ort. güven | tür yanlışken ort. güven | güveni boş |
|---|---:|---:|---:|
| rozet | 3,19 (n=88) | 2,81 (n=32) | 0 |
| siluet | 3,34 (n=99) | 3,14 (n=21) | 0 |

## Tür karışıklık matrisi (satır = doğru tür, sütun = verilen cevap)

### rozet

| doğru \ cevap | devam | dönüş | kırılım | sıkışma | aşırılık | akış | boş | tanınmadı |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| devam | 18 | 1 | 1 | 1 | 1 | 2 | 0 | 0 |
| dönüş | 1 | 13 | 1 | 0 | 0 | 3 | 0 | 0 |
| kırılım | 1 | 1 | 19 | 0 | 2 | 1 | 0 | 0 |
| sıkışma | 0 | 1 | 0 | 11 | 0 | 0 | 0 | 0 |
| aşırılık | 0 | 3 | 3 | 1 | 11 | 0 | 0 | 0 |
| akış | 4 | 0 | 0 | 2 | 2 | 16 | 0 | 0 |

### siluet

| doğru \ cevap | devam | dönüş | kırılım | sıkışma | aşırılık | akış | boş | tanınmadı |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| devam | 22 | 0 | 0 | 1 | 0 | 1 | 0 | 0 |
| dönüş | 1 | 12 | 1 | 1 | 0 | 3 | 0 | 0 |
| kırılım | 1 | 1 | 17 | 0 | 0 | 5 | 0 | 0 |
| sıkışma | 1 | 0 | 0 | 11 | 0 | 0 | 0 | 0 |
| aşırılık | 0 | 0 | 0 | 0 | 17 | 1 | 0 | 0 |
| akış | 0 | 0 | 0 | 2 | 2 | 20 | 0 | 0 |

## Madde başına tür ve tam doğru (zor işaretleri bulmak için)

| alfabe | etiket | koşul | doğru (tür.yön, güç, aşama) | tür | tam |
|---|---|---|---|---:|---:|
| rozet | A1 | nolegend | reversal.up, low, retracted | 0,83 (5/6) | 0,33 (2/6) |
| rozet | A2 | nolegend | reversal.down, low, candidate | 0,50 (3/6) | 0,00 (0/6) |
| rozet | A3 | nolegend | continuation.up, high, realized | 0,67 (4/6) | 0,17 (1/6) |
| rozet | A4 | nolegend | flow.neutral, high, realized | 0,67 (4/6) | 0,00 (0/6) |
| rozet | B1 | nolegend | breakout.up, high, final | 0,33 (2/6) | 0,17 (1/6) |
| rozet | B2 | nolegend | squeeze.neutral, low, candidate | 0,83 (5/6) | 0,33 (2/6) |
| rozet | B3 | nolegend | extreme.up, high, final | 0,17 (1/6) | 0,17 (1/6) |
| rozet | B4 | nolegend | flow.down, low, failed | 0,33 (2/6) | 0,17 (1/6) |
| rozet | C1 | legend | flow.up, high, failed | 0,83 (5/6) | 0,83 (5/6) |
| rozet | C2 | legend | breakout.up, high, retracted | 0,83 (5/6) | 0,67 (4/6) |
| rozet | C3 | legend | extreme.down, low, failed | 1,00 (6/6) | 0,83 (5/6) |
| rozet | C4 | legend | continuation.up, low, realized | 0,67 (4/6) | 0,67 (4/6) |
| rozet | D1 | legend | continuation.down, high, candidate | 0,83 (5/6) | 0,83 (5/6) |
| rozet | D2 | legend | reversal.down, high, final | 0,83 (5/6) | 0,50 (3/6) |
| rozet | D3 | legend | breakout.down, low, retracted | 1,00 (6/6) | 0,83 (5/6) |
| rozet | D4 | legend | squeeze.neutral, low, final | 1,00 (6/6) | 0,83 (5/6) |
| rozet | L1 | learn | breakout.down, high, candidate | 1,00 (6/6) | 0,67 (4/6) |
| rozet | L2 | learn | flow.neutral, high, failed | 0,83 (5/6) | 0,67 (4/6) |
| rozet | L3 | learn | continuation.up, low, realized | 0,83 (5/6) | 0,50 (3/6) |
| rozet | L4 | learn | extreme.up, low, final | 0,67 (4/6) | 0,17 (1/6) |
| siluet | A1 | nolegend | reversal.down, low, candidate | 0,50 (3/6) | 0,50 (3/6) |
| siluet | A2 | nolegend | reversal.up, low, retracted | 0,83 (5/6) | 0,50 (3/6) |
| siluet | A3 | nolegend | flow.down, low, failed | 1,00 (6/6) | 0,00 (0/6) |
| siluet | A4 | nolegend | breakout.up, high, final | 0,33 (2/6) | 0,00 (0/6) |
| siluet | B1 | nolegend | flow.neutral, high, realized | 0,67 (4/6) | 0,50 (3/6) |
| siluet | B2 | nolegend | extreme.up, high, final | 0,83 (5/6) | 0,33 (2/6) |
| siluet | B3 | nolegend | squeeze.neutral, low, candidate | 0,83 (5/6) | 0,50 (3/6) |
| siluet | B4 | nolegend | continuation.up, high, realized | 1,00 (6/6) | 0,67 (4/6) |
| siluet | C1 | legend | reversal.down, high, final | 0,67 (4/6) | 0,33 (2/6) |
| siluet | C2 | legend | continuation.down, high, candidate | 1,00 (6/6) | 1,00 (6/6) |
| siluet | C3 | legend | breakout.down, low, retracted | 0,83 (5/6) | 0,50 (3/6) |
| siluet | C4 | legend | continuation.up, low, realized | 0,83 (5/6) | 0,50 (3/6) |
| siluet | D1 | legend | breakout.up, high, retracted | 1,00 (6/6) | 0,50 (3/6) |
| siluet | D2 | legend | squeeze.neutral, low, final | 1,00 (6/6) | 0,83 (5/6) |
| siluet | D3 | legend | extreme.down, low, failed | 1,00 (6/6) | 1,00 (6/6) |
| siluet | D4 | legend | flow.up, high, failed | 1,00 (6/6) | 0,83 (5/6) |
| siluet | L1 | learn | continuation.up, low, realized | 0,83 (5/6) | 0,83 (5/6) |
| siluet | L2 | learn | extreme.up, low, final | 1,00 (6/6) | 0,83 (5/6) |
| siluet | L3 | learn | flow.neutral, high, failed | 0,67 (4/6) | 0,50 (3/6) |
| siluet | L4 | learn | breakout.down, high, candidate | 0,67 (4/6) | 0,67 (4/6) |

## Eşli karşılaştırma: siluet ve rozet

Katılımcı başına iki ölçü: **alan doğruluğu** (20 maddenin 4 alanının ortalaması) ve **tam doğru oranı**. Fark = siluet − rozet. İşaret testi (eşitlikler atılır, iki yönlü, kesin binom).

| katılımcı | grup | kitapçık | alan siluet | alan rozet | fark | tam siluet | tam rozet | fark |
|---|---|---|---:|---:|---:|---:|---:|---:|
| P01 | alışkın | A | 0,93 | 0,82 | 0,10 | 0,75 | 0,45 | 0,30 |
| P02 | genç | B | 0,90 | 0,82 | 0,08 | 0,65 | 0,55 | 0,10 |
| P03 | alışkın | B | 0,89 | 0,85 | 0,04 | 0,60 | 0,55 | 0,05 |
| P04 | genç | A | 0,82 | 0,75 | 0,07 | 0,50 | 0,35 | 0,15 |
| P05 | alışkın | A | 0,88 | 0,89 | -0,01 | 0,60 | 0,55 | 0,05 |
| P06 | genç | B | 0,72 | 0,78 | -0,05 | 0,30 | 0,35 | -0,05 |

- **alan doğruluğu:** siluet daha iyi 4, rozet daha iyi 2, eşit 0; ortalama fark 0,04; işaret testi p = 0,69
- **tam doğru oranı:** siluet daha iyi 5, rozet daha iyi 1, eşit 0; ortalama fark 0,10; işaret testi p = 0,22

> **Güç uyarısı.** 5–10 kişiyle işaret testi ancak çok tek yönlü sonuçları yakalar: n = 6'da bile 6/6 aynı yönde olmadan p < 0,05 olmaz (6/6 → p = 0,031; 5/6 → p = 0,22). Anlamlı çıkmayan fark "fark yok" demek değildir. Karar, sayılarla birlikte karışıklık matrisi, madde tablosu ve nitel cevaplarla verilir (K39).

## Kapsül cümleleri

Şans düzeyi: tür 0,17 · yön 0,33 · ne zaman 0,33 · hâlâ geçerli 0,33. "Ne zaman": gerçek "0 bar önce" (bugün) olan geçmiş olaya `now` da doğru sayılır; "kesin" = geçmişte ise yazılan bar sayısı da doğru (rakamsız cümlelerde bar sayısı yazılamaz; bilgi amaçlı).

| | tür | yön | ne (ikisi) | ne zaman | ne zaman (kesin) | hâlâ geçerli | tam |
|---|---:|---:|---:|---:|---:|---:|---:|
| hepsi | 0,94 (45/48) | 0,90 (43/48) | 0,83 (40/48) | 0,94 (45/48) | 0,58 (28/48) | 0,92 (44/48) | 0,73 (35/48) |
| alışkın | 0,92 (22/24) | 1,00 (24/24) | 0,92 (22/24) | 0,96 (23/24) | 0,58 (14/24) | 0,96 (23/24) | 0,88 (21/24) |
| genç | 0,96 (23/24) | 0,79 (19/24) | 0,75 (18/24) | 0,92 (22/24) | 0,58 (14/24) | 0,88 (21/24) | 0,58 (14/24) |

| cümle | zaman türü | doğru (tür.yön, ne zaman, geçerli) | ne | ne zaman | hâlâ geçerli | tam |
|---|---|---|---:|---:|---:|---:|
| S1 | event | breakout.down, past 30, unknown | 0,83 (5/6) | 1,00 (6/6) | 0,83 (5/6) | 0,67 (4/6) |
| S2 **(K26)** | state | extreme.up, past 3, no | 1,00 (6/6) | 0,83 (5/6) | 1,00 (6/6) | 0,83 (5/6) |
| S3 | expectation | continuation.up, conditional, yes | 0,83 (5/6) | 1,00 (6/6) | 1,00 (6/6) | 0,83 (5/6) |
| S4 | event | extreme.down, past 0, unknown | 0,83 (5/6) | 1,00 (6/6) | 0,83 (5/6) | 0,67 (4/6) |
| S5 | expectation | flow.down, conditional, no | 0,67 (4/6) | 1,00 (6/6) | 0,67 (4/6) | 0,67 (4/6) |
| S6 | state | squeeze.neutral, now, yes | 1,00 (6/6) | 0,83 (5/6) | 1,00 (6/6) | 0,83 (5/6) |
| S7 | event, rakamsız | reversal.down, past 5, no | 0,67 (4/6) | 1,00 (6/6) | 1,00 (6/6) | 0,67 (4/6) |
| S8 | state, rakamsız | flow.down, past 5, no | 0,83 (5/6) | 0,83 (5/6) | 1,00 (6/6) | 0,67 (4/6) |

## Tercih ve nitel cevaplar

T1 (hangi alfabe daha kolay okundu): siluet (M) 3 · rozet (N) 2 · fark yok 1

**T2 — karışan işaretler:**

- P01 (A, alışkın): N-A1 ile N-A2 dönüş oklarını karıştırdım
- P02 (B, genç): M-B1 akış mı sıkışma mı?
- P04 (A, genç): gri olanların aşamasını ayıramadım

**T3 — serbest:**

- P06 (B, genç): Renkler yardımcı oldu; küçük boyutta çizgiler kayboluyor.
