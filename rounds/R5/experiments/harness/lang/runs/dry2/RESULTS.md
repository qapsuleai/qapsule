# Okuyucu ve eleştirmen sonuçları — `runs/dry2`

Ajan insan değildir; bu ölçüm ön elemedir (SPEC §0). Okuyucu koşusu 48, eleştirmen koşusu 6, çözümlenemeyen 0; maliyet 0.1579 $. Önyükleme tohumu 2.

## Anlama doğruluğu (birincil; F 2, H 1; Wilson %95)

| dil | kod | birincil | alt | üst | tavsiye okuması |
|---|---|---:|---:|---:|---:|
| `r4` | 42154f | 0.876 | 0.841 | 0.904 | 0.000 |
| `bozuk` | c74cef | 0.494 | 0.447 | 0.541 | 0.000 |

### Soru × persona × koşul

| dil | persona | koşul | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---:|---:|---:|---:|---:|---:|
| `r4` | F | acik | 1.00 (36/36) | 0.69 (25/36) | 0.94 (34/36) | 0.92 (33/36) | 0.91 (30/33) | 1.00 (36/36) |
| `r4` | F | yuz | 1.00 (36/36) | 0.08 (3/36) | 1.00 (36/36) | 1.00 (36/36) | 0.97 (32/33) | 1.00 (36/36) |
| `r4` | H | acik | 1.00 (36/36) | 0.72 (26/36) | 0.94 (34/36) | 0.94 (34/36) | 0.91 (30/33) | 1.00 (36/36) |
| `r4` | H | yuz | 1.00 (36/36) | 0.08 (3/36) | 0.94 (34/36) | 1.00 (36/36) | 0.97 (32/33) | 1.00 (36/36) |
| `bozuk` | F | acik | 0.92 (33/36) | 0.08 (3/36) | 0.75 (27/36) | 0.78 (28/36) | 0.00 (0/33) | 1.00 (36/36) |
| `bozuk` | F | yuz | 0.92 (33/36) | 0.08 (3/36) | 0.00 (0/36) | 0.33 (12/36) | 0.00 (0/33) | 1.00 (36/36) |
| `bozuk` | H | acik | 0.92 (33/36) | 0.08 (3/36) | 0.75 (27/36) | 0.72 (26/36) | 0.00 (0/33) | 1.00 (36/36) |
| `bozuk` | H | yuz | 0.97 (35/36) | 0.08 (3/36) | 0.00 (0/36) | 0.28 (10/36) | 0.00 (0/33) | 1.00 (36/36) |

Şans düzeyi: S1 0.09, S2 0.00, S3 0.20, S4 0.17, S5 0.00, S6 0.25.

## Eleştirmen ikili karşılaştırma (Bradley–Terry, önyükleme %95)

| dil | kod | log güç | alt | üst |
|---|---|---:|---:|---:|
| `r4` | 42154f | 2.145 | 2.145 | 2.145 |
| `bozuk` | c74cef | -2.145 | -2.145 | -2.145 |

Çelişki oranı (iki sırada farklı kazanan; > %30 ise çift "ayırt edilemedi", SPEC §4.2):

- `r4` – `bozuk`: 0/36 = 0.00

Uzunluk yanlılığı (U2): kararlı birimlerde uzun metin 0/36 kez kazandı; korelasyon 0.000.

## Seçim kuralı girdileri (SPEC §5)

- Birincil: `r4` 0.876 [0.841, 0.904] ↔ `bozuk` 0.494 [0.447, 0.541] → ayrıldı
- BT: `r4` ↔ `bozuk` → ayrıldı
- **Bozuk dil sınaması:** birincilde son: evet; aralığı ötekilerden ayrık: evet; BT'de son: evet → düzenek **geçerli**.
