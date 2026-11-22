# Form ve özet

Katılımcı başına 12 satır (8 aday + 4 başvuru; açık tema yapıldıysa +8). Formda görünüş adı
yoktur, yalnız kod; eşleme `GUIDE.md` "Ekranlar"da. Satırlar `form.csv`'ye aynen geçirilir.

## Sütunlar (`form.csv`)

| Sütun | Değer |
|---|---|
| `id` | P0 (Arkin), P1, P2 … |
| `order` | 1 / 2 / 3 (`GUIDE.md` "Sıralar") |
| `pos` | ekranın o kişideki sırası, 1–12 |
| `screen` | K1–K8, B1–B4 (açık tema: `K3a` …) |
| `profesyonel` | E / H |
| `guven` | 1–5 |
| `seconds` | ekran kapandıktan iki cevap bitene kadar, saniye |
| `notes` | kendiliğinden söylenenler **aynen**, tırnak içinde; yürütücü gözlemi `[köşeli]` |

Açık sorular kişi başına bir kez: `screen` = `Q1`, `Q2`, `Q3`; `profesyonel`, `guven`,
`seconds` boş; cevap `notes`'ta aynen (işaret ettiği kod `[K3]` gibi eklenir).

## Basılacak düzen

```
Kod: P__   Sıra: _   Blok: masaüstü / telefon       (ad, yaş, iletişim yazılmaz)
pos | ekran | profesyonel E/H | güven 1-5 | sn | aynen söyledikleri
 1  |       |                 |           |    |
 …  (12 satır)
Q1 Profesyonel durmayan ne vardı?      ______________________
Q2 Neyi görseydiniz daha çok güvenirdiniz? ___________________
Q3 Yanındaki ekrana göre eksik olan ne?  ____________________
```

## Özet (çözülmüş: kod → görünüş; P0 ayrı sütun)

| Çapa | Görünüş | Kod | P0 E/H | P0 güven | Diğerleri E sayısı / n | Güven ort. | Güven ortanca |
|---|---|---|---|---|---|---|---|
| Ç1 | `oltan` (önce) | K1 | | | / | | |
| Ç1 | `tavek` (sonra) | K3 | | | / | | |
| Ç1 | başvuru | B1 | | | / | | |
| Ç2 | `oltan` | K4 | | | / | | |
| Ç2 | `tavek` | K2 | | | / | | |
| Ç2 | başvuru | B2 | | | / | | |
| Ç3 | `oltan` | K7 | | | / | | |
| Ç3 | `tavek-tel` | K6 | | | / | | |
| Ç3 | başvuru | B3 | | | / | | |
| Ç4 | `oltan` | K5 | | | / | | |
| Ç4 | `tavek-tel` | K8 | | | / | | |
| Ç4 | başvuru | B4 | | | / | | |

**Önce / sonra** (diğerleri; dört çapa toplamı): `oltan` E __ / __, güven ortanca __ →
`tavek` E __ / __, güven ortanca __; başvuru E __ / __, güven ortanca __.
Kişi başına satır özete girmez; n küçük olduğu için yüzde değil sayı yazılır.
