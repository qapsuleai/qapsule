# Cevap formu ve cevapların kayda geçirilmesi

Basılacak form: `form.html` (3 A4 sayfa). Kitapçıktan bağımsızdır: alfabeler "Bölüm 1" ve
"Bölüm 2" diye geçer; hangi bölümün hangi alfabe olduğunu kitapçık (A ya da B) belirler.
Katılımcı ayrıca her bölümün başında kitapçıkta gördüğü alfabe harfini (M/N) daire içine
alır; bu yalnız denetim içindir (kitapçıkla çelişirse görevli not eder).

| Kitapçık | Bölüm 1 | Bölüm 2 | Bölüm 3 | Bölüm 4 |
|---|---|---|---|---|
| A | Alfabe M (siluet) | Alfabe N (rozet) | 8 cümle | tercih |
| B | Alfabe N (rozet) | Alfabe M (siluet) | 8 cümle | tercih |

## Başlık

Katılımcı no (P01, P02, …; ad yok), kitapçık (A/B), gösterim (kâğıt/ekran), grup:
**alışkın kullanıcı** (grafiğe düzenli bakar ya da işlem yapar) ya da **genç, işlem
yapmayan** (NORTH_STAR'daki iki uç). Başka kişisel bilgi sorulmaz ve yazılmaz.

## Bölüm 1 ve 2 — işaretler (her bölümde 20 satır)

| Adım | Satırlar | Koşul |
|---|---|---|
| 1. adım | A1–B4 (8) | açıklamasız: yalnız sözlük (anlamlar, çizim kuralı yok) |
| 2. adım | C1–D4 (8) | alfabenin kendi ≤ 120 kelimelik açıklaması okunduktan sonra |
| 3. adım | L1–L4 (4) | 5 cevaplı örnek (E1–E5) incelendikten sonra yeni işaretler |

Her satırda beş soru, her biri bir seçeneği daire içine alarak:

| Sütun | Seçenekler (formdaki kod) | CSV sütunu | CSV değeri |
|---|---|---|---|
| Tür | DV devam · DN dönüş · KR kırılım · SK sıkışma · AŞ aşırılık · AK akış | `kind` | kod olduğu gibi (`DV`, `AŞ`…) ya da `continuation`… |
| Yön | ↑ yukarı · ↓ aşağı · — yönsüz | `direction` | `↑`/`↓`/`—` ya da `up`/`down`/`neutral` |
| Güç | Z zayıf · G güçlü | `strength` | `Z`/`G` ya da `low`/`high` |
| Aşama | AD aday · KE kesin · GE gerçekleşti · GM gerçekleşmedi · GÇ geri çekildi | `phase` | kod ya da `candidate`/`final`/`realized`/`failed`/`retracted` |
| Ne kadar eminim | 1 (tamamen tahmin) … 5 (çok eminim) | `confidence` | `1`…`5` |

## Bölüm 3 — kapsül cümleleri (S1–S8)

| Soru | Seçenekler | CSV sütunu | CSV değeri |
|---|---|---|---|
| Ne oldu: tür | 6 tür (yukarıdaki kodlar) | `kind` | kod |
| Ne oldu: yön | ↑ ↓ — | `direction` | `↑`/`↓`/`—` |
| Ne zaman | geçmişte (___ bar önce) · şu an sürüyor · bir koşula bağlı | `when`, `bars_ago` | `geçmişte`/`şu an`/`bir koşula bağlı` (ya da `past`/`now`/`conditional`); bar sayısı yazıldıysa `bars_ago` |
| Hâlâ geçerli mi? | evet · hayır · bilinmiyor | `still_valid` | `evet`/`hayır`/`bilinmiyor` (ya da `yes`/`no`/`unknown`) |

## Bölüm 4 — tercih

| Soru | CSV |
|---|---|
| T1. Hangi alfabe daha kolay okundu? M / N / fark yok | madde `T1`, `text` = `M`, `N` ya da `fark yok` |
| T2. Hangi işaretler karıştı, neyle karıştı? (serbest; "M-B3" gibi) | madde `T2`, `text` = yazılanın aynısı |
| T3. Eklemek istediğiniz başka bir şey (serbest) | madde `T3`, `text` = yazılanın aynısı |

## CSV (`answers.csv`)

UTF-8, başlık satırı şu (örnek: `answers.example.csv`):

```
participant,group,booklet,mode,item,kind,direction,strength,phase,confidence,when,bars_ago,still_valid,text
```

- Bir satır = bir madde. `item`: `1-A1` … `1-D4`, `1-L1` … `1-L4` (Bölüm 1), `2-…` (Bölüm 2),
  `S1` … `S8`, `T1` … `T3`. Bölüm numarası formdaki bölümdür, alfabe değil.
- `participant`, `group` (`trader` = alışkın, `young` = genç; Türkçe karşılıkları da
  tanınır), `booklet` (`A`/`B`), `mode` (`kâğıt`/`ekran`) her satırda tekrar edilir.
- Boş bırakılan alan boş yazılır (puanlamada yanlış sayılır). Bir sütunda iki seçenek
  işaretlenmişse ya da okunamıyorsa `?` yazılır (yanlış sayılır, uyarı listesine düşer).
  Aynı adım içinde karalanıp değiştirilmiş cevapta son cevap yazılır. Görevli bir 1. adım
  satırının açıklama okunduktan sonra değiştirildiğini gördüyse (yönerge bunu izlemesini
  ister) ilk cevap okunabiliyorsa o, okunamıyorsa `?` yazılır ve oturum günlüğüne not edilir.
- Kayda geçiren kişi doğru cevaplara (`key.json`) bakmadan yazar.
