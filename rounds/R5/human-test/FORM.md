# Gözlem formu ve CSV biçimi

Katılımcı başına bir form. Formda **tasarım adı yoktur**: her görevde "1. set / 2. set" yazar;
hangi setin hangi aday olduğu katılımcının sıra satırından çıkar (`TASKS.md` "Karşı dengeleme").
Yürütücü formu doldururken `key.md`'ye bakmaz; doğru/yanlış sütunu puanlamada (CSV'den) doldurulur.

## Başlık

| Alan | Değer |
|---|---|
| Kod | G01 … (genç) / A01 … (alışkın) |
| Sıra satırı | 01–10 (persona içinde geliş sırası) |
| Dil | tr / en |
| Gösterim | ekran (telefon) / masaüstü (telefon görünümü) |
| Renk görme (gönüllü beyan) | var / yok / söylemek istemiyor |
| Ad, yaş, tarih, iletişim | **yazılmaz** |

## Her madde için sütunlar

| Sütun | Ne | Nasıl |
|---|---|---|
| cevap | kart seçeneğinin numarası; açık uçluda katılımcının sözü aynen | yürütücü |
| doğru | 1 / 0 / ½ (yalnız `e_rests`, `c_evid` kısmi) | **puanlamada**, `key.md` ile; oturumda boş |
| süre | saniye; TASKS.md'nin yazdığı başlangıç–bitiş | kronometre |
| dokunma | ekrana ayrı dokunuş sayısı (bir kaydırma = 1, bir dokunuş = 1; iki parmak = not) | çetele |
| kararsız | ✓: cevabı değiştirdi, > 5 sn duraksadı ya da "emin değilim" dedi | yürütücü |
| ipucu | ✓: "ekranda ne görüyorsanız o" dışında bir şey söylendi (o cevap yanlış) | yürütücü |
| sesli düşünme | katılımcının kendiliğinden söylediği, aynen ("çarpı var, tutmamış") | yürütücü |
| not | yürütücü gözlemi (büyüttü, örtüyü kaldırdı, geri kaydırdı, set geçersiz …) | yürütücü |

## Form sayfaları (basılacak düzen)

Her görev bir blok; blokta 1. set ve 2. set yan yana, satırlar soru kodları:

```
G1 Kutular (5 sn)              1. set: 1a | 1b          2. set: 2a | 2b
  ilk ne çarptı (yalnız a)     ____________             ____________
  dir kind valid when trust advice   [ ][ ][ ][ ][ ][ ] ...
  süre / kararsız / ipucu / sesli düşünme / not
G2 Metin                       1. set                   2. set
  okuma (sn)
  l_what l_level(söz) l_when l_valid l_ledger(söz) l_advice l_same
  sözlük: g_found g_taps süre g_right(söz)
G3 İşaretler (örtülü)          1. set: 1 2 3            2. set: 1 2 3
  dir kind valid × 3 ; set süresi
G4 Açılan ekran                1. set                   2. set
  e_rests (süre, dokunma) e_who (süre, dokunma) e_valid
  yetkisiz: v_trust v_why v_who
G5 Grafik                      1. set                   2. set
  c_recent c_held c_live(göster) c_evid(dokunma + seçenek) — her biri süre, dokunma
G6 Akış (son bakış)            1. set                   2. set
  f_new(söz) f_neg(hisse + seçenek) — her biri süre, kaydırma
G7 Akış                        1. set                   2. set           boş akış
  r_trust r_best(göster) r_few                                       x_empty
G8 Hareket                     1. set     2. set     3. set (hareketi azalt)
  m_seen m_what(göster)        ...        ...        ...
  m_same m_lost                ...        ...        —
T Tercih (1 ilk / 2 ikinci / 0 fark yok) + neden
  G1 G2 G3 G4 G5 G6 G7 G8 ; k53_want ; serbest
```

## CSV (`answers.csv`)

UTF-8, bir satır = bir cevap (ya da bir süre / dokunma kaydı):

```
participant,persona,row,lang,mode,colour,item,q,answer,time_s,taps,hesitant,hint,thinkaloud,note
G01,genç,01,tr,ekran,yok,G1-1a,first,işaret,,,,,,
G01,genç,01,tr,ekran,yok,G1-1a,valid,4,,,,,,
G01,genç,01,tr,ekran,yok,G1-1a,_all,,11,0,evet,,"ok aşağı, kapandı galiba",
G01,genç,01,tr,ekran,yok,G2-1,l_level,son 20 günün en düşüğü 148,9,,,,,,
G01,genç,01,tr,ekran,yok,G5-2,c_live,göster:doğru,6,2,,,,
G01,genç,01,tr,ekran,yok,T,G5,2,,,,,"çizgiler daha az kalabalık",
```

- `item`: `G1-1a` … `G1-2b`, `G2-1`, `G2-2`, `G3-1.1` … `G3-2.3`, `G4-1`, `G4-2` (yetkisiz
  sorular aynı maddede `v_…`), `G5-1`, `G5-2`, `G6-1`, `G6-2`, `G7-1`, `G7-2`, `G7-0` (boş akış),
  `G8-1` … `G8-3`, `T`. Set numarası formdaki settir, aday değil; puanlama sıra satırından adaya çevirir.
- `q`: `TASKS.md` soru kodu; madde bütününün süresi için `_all`, G2 okuma süresi için `okuma`,
  G3 set süresi için `G3-1` maddesinde `_all`.
- `answer`: numara ya da söz; gösterme sorularında yürütücünün işareti `göster:doğru` /
  `göster:yanlış` / `göster:yok` (gösterme oturumda yürütücüce işaretlenir, çünkü sonradan
  denetlenemez; yürütücü bunu `key.md`'deki konuma göre yapar — bu yüzden `key.md` yalnız gösterme
  sorularının konumlarını içeren ayrı bir sayfa olarak oturumda elde durur).
- Boş alan boş bırakılır (cevapsız = yanlış; "atlandı" `note`'ta = eksik). İki seçenek işaretli ya
  da okunaksızsa `?` (yanlış, uyarıda listelenir).
- `thinkaloud`, `note` ve T nedenleri kişiyi tanıtabilecek her şey çıkarılarak yazılır (GUIDE
  "Kayıt").
