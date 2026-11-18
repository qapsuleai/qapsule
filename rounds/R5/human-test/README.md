# R5 — İnsan testi v3 paketi (GOAL kabul ölçütü 12)

R5'in tasarım seçimleri (M16–M21) ajan okuyucu ve eleştirmenlerle yapılmış **ön elemedir**;
**ajan insan değildir**. Son söz bu testtedir (ve Arkin'in telefondaki galeri karşılaştırmasında).
v3, koşulmamış v2'nin (`rounds/R4/human-test/`) yerine geçer; yapısı v2'den (sıra dengesi, önceden
yazılı karar kuralı, kodla anonim kayıt) ve R3 paketinden alınmıştır.

Bir oturum: bir katılımcı, yaklaşık 20 dakika, gösterim (`scripts/demo.sh`) telefonda. 5–10 kişi,
iki persona: **genç** (teknik analiz bilmeyen, yeni yatırımcı) ve **alışkın** (40+, grafiğe ve
terminale alışkın).

## İçindekiler

| Dosya | Ne |
|---|---|
| `GUIDE.md` | Yürütücü yönergesi: hazırlık (gösterim, telefon, galeri), prova ve cevap anahtarı, katılımcı profili, onam, ~20 dakikalık akış, söylenmeyecekler, kayıt ve verinin depoya girişi |
| `TASKS.md` | Görev listesi (G0–G9), aynen söylenecek cümleler, yanıt kartı seçenekleri, karşı dengeleme tablosu, bağlantı kodları; İngilizce eki |
| `FORM.md` | Gözlem formu (her görevde: doğru/yanlış, süre, dokunma, kararsızlık, sesli düşünme notu, tercih + neden) ve CSV biçimi |
| `SCORING.md` | Puanlama kuralları (doğruluk, süre ortancası, tercih oranı), küçük örnekte belirsizlik ve "ayırt edilemedi" kuralı; ajan sonuçlarıyla karşılaştırma tablosu şablonu |
| `DECISION.md` | Hangi açık karar nasıl kapanır: A21, A32, A33, K53 (ve K42 muafiyeti), akış sırası, M16, M17, M18, M20, M21 |

## Karşılaştırılan adaylar (ajan ön elemesinin ilk ikisi)

| Konu | Ayar (URL) | Aday 1 (ajan seçimi) | Aday 2 | Kaynak |
|---|---|---|---|---|
| Dil (M17) | `voice` | `marla` | `selvi` | `rounds/R5/experiments/M17/RESULTS.md` |
| Görünüş (M16) | `look` | `oltan` | `zemre` | M16 ölçümü (sürüyor) |
| Hikâye çizelgesi (M18) | `story` | `pavka` | `korda` | `rounds/R5/experiments/M18/RESULTS.md` (son düzeltme: `pavka` varsayılan) |
| Akış sırası (M19) | `feed_order` | `grouped` | `live_new` | `rounds/R5/experiments/M19/RESULTS.md` §4 |
| Yazı tura eşiği (M19, K53) | `feed_rule` | `rate55` | `none` | aynı |
| Açılmış kapsül (M20) | `open` | `tesir` | `nobra` | M20 ölçümü (sürüyor) |
| Hareket (M21) | `motion` | `surek` | `nabiz` | M21 ölçümü (sürüyor) |
| İşaret alfabesi (A21) | `alphabet` | `siluet` | `rozet` | `rounds/R3/human-test/`, Ö32 |

Katılımcıya hiçbir ad söylenmez; formda yalnız "1. set / 2. set" yazar.

## Doldurulacak yer tutucular (oturumlardan önce)

1. **Aday adları:** `oltan`, `zemre`, `tesir`, `nobra`, `surek`,
   `nabiz` — ölçümler bitince orkestratör dört dosyada (`README.md`, `TASKS.md`,
   `DECISION.md`, `SCORING.md`) bul-değiştir ile doldurur. Aday 1 = ajan ölçümünün seçtiği.
   Bir konu insan testine kadar hazır değilse o konunun görevi taban ayarla yapılır, karşılaştırması
   atlanır ve raporda "koşulmadı" yazılır (DECISION.md "Eksik konu").
2. **İçerik anları:** `<ADIM-…>`, `<SATIR-…>`, `<KİTLE-…>` — gösterimin hangi adımında hangi
   kapsülün kullanılacağı. Seçim ölçütleri `GUIDE.md` "Prova ve cevap anahtarı"nda önceden yazılı;
   yürütücü provada ölçütlere uyan anları bulur, `key.md`'ye (katılımcı görmez) yazar. Anahtar
   motorun durumundan çıkarılır, tasarımdan değil.
3. **URL adları:** `story`, `open`, `motion` ayarlarının URL'den seçilebildiği bütünleştirmeden
   (R5/S) sonra provada denetlenir (bugün `main`'de `voice`, `look`, `feed_order`, `feed_rule`,
   `alphabet`, `principal`, `anim`, `seen`, `watch`, `long` var; `story` M18 ölçüm derlemelerinde
   kullanıldı).

## Bilinen sınırlar

- Gösterim K42 muafiyetini açık çalıştırır (`exempt_negative: true`); muafiyetsiz biçim gösterimden
  seçilemez. Bu yüzden insan testi K42 muafiyetinin yalnız **muaf** biçimini gözler (DECISION.md K53).
- Sentetik pazar tekdüze olabilir (R4 v2 "Sınırlar"): yön ve tür soruları az ayırt edebilir; prova
  ölçütleri bunu azaltmak için karışık yön ve tür ister.
- 5–10 kişi istatistiksel anlamlılık vermez; kural bunu açıkça yazar (`SCORING.md` §5).
- Puanlama elle ya da R4'ün `score.py`'sinin bu pakete uyarlanmasıyla yapılır (bu pakette betik yok).

> **Adaylar dolduruldu (2026-11-18):** Görünüş `oltan`/`zemre`, Açılmış kapsül `tesir`/`nobra`, Hareket `surek`/`nabiz`
> (M21 ölçümü geçerli kazanan çıkarmadı; `surek` geçici varsayılandır — G8 bu karşılaştırmayı karara bağlar).
> Hareketi azalt: `?reduced_motion=1` (eski `?anim=0`). K42 muafiyetsiz biçim: `?feed_exempt=0`.
