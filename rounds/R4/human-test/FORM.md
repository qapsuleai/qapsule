# Gözlem formu ve cevapların kayda geçirilmesi

Basılacak: `form.html` (A4; 1. sayfa **yanıt kartı** — masada katılımcının önünde durur, bir kez
basılır; 2.–4. sayfalar katılımcı başına **gözlem formu**). Formda tasarım adı yoktur: her görevde
"1. set / 2. set" yazar; hangi setin hangi tasarım olduğunu katılımcı kodu belirler
(`GUIDE.md` "Sıra", `score.py` `first_side`).

## Başlık

Katılımcı kodu (A01 … alışkın, G01 … genç), gösterim (`ekran` = telefon, `masaüstü` = masaüstü
tarayıcıda telefon görünümü), tarih yok, ad yok.

## Her görevde yazılanlar

| Sütun | Ne |
|---|---|
| cevap | yanıt kartındaki seçeneğin **numarası** (G2a/G2b/G2c'de kısa metin) |
| süre | saniye (kronometre; GUIDE'da her görevde ne zaman başlayıp bittiği yazar) |
| ipucu | yürütücü "ekranda ne görüyorsanız o" dışında bir şey söylemek zorunda kaldıysa ✓ (o cevap yanlış sayılır, ayrıca sayılır) |
| not | yürütücü gözlemi (büyüttü, örtüyü kaldırdı, geri kaydırdı, kaydırma sayısı …) |
| söz | katılımcının kendi sözü, aynen ("çarpı var, tutmamış") |

## Sorular ve kodlar

| Görev · madde | `q` | Seçenekler (kartta numaralı) |
|---|---|---|
| G1 her kapsül (1a–1c, 2a–2c) | `s1` | 1 yukarı · 2 aşağı · 3 belirsiz · 4 anlamadım |
| | `s1b` | 1 fiyat önemli bir seviyeyi geçti · 2 yön değişiyor · 3 hareket sürüyor · 4 fiyat sıkıştı, hareketsiz · 5 aşırı gitti · 6 büyük alım ya da satım var · 7 anlamadım |
| | `s2` | 1 bugün · 2 dün · 3 birkaç gün önce · 4 bir haftadan önce · 5 birkaç saat önce · 6 bilmiyorum |
| | `s3` | 1 evet, sürüyor · 2 hayır, artık geçerli değil · 3 kapandı: gerçekleşti · 4 kapandı: tutmadı · 5 kapandı: süresi doldu ya da geri çekildi · 6 bilmiyorum |
| | `s4` | 1 geçmişte çoğunlukla tuttu · 2 yarı yarıya · 3 çoğunlukla tutmadı · 4 az örnek var, bilinmiyor · 5 hiçbir bilgi göremedim |
| | `s5` | 1 al · 2 sat · 3 bekle · 4 hiçbir şey, yalnız bilgi veriyor · 5 bilmiyorum |
| G5 her işaret (1.1–1.3, 2.1–2.3) | `yon`, `ne`, `durum` | `s1`, `s1b`, `s3` ile aynı |
| G4 her işaret (1.1–1.3, 2.1–2.3) | `sonuc` | 1 gerçekleşti · 2 tutmadı · 3 hâlâ açık · 4 bilmiyorum |
| G3 her kapsül (1a, 1b, 2a, 2b) | `yer` | 1 doğru yer · 2 yanlış yer · 3 gösteremedi (yürütücü işaretler: doğru = vurgulu sütun ya da bir yanındaki bar, seviye varsa ona yakın) |
| | `ayni` | 1 evet · 2 hayır · 3 emin değilim |
| | `s3` | `s3` ile aynı |
| | `kayip` | 1 (hiç) … 5 (tamamen kayboldum) |
| G2 her pencere (G2-1, G2-2) | `g2a` | değişenler, "SYN09 yukarı; SYN03 aşağı" ya da "yok" |
| | `g2b` | olumsuz haber olan hisse ("SYN09") ya da "yok" |
| | `g2b_ne` | 1 tutmadı, geçersiz oldu · 2 zayıfladı · 3 geri çekildi · 4 başka · 5 bilmiyorum |
| | `g2c` | gösterilen kutunun hissesi ve karnesi ("SYN22 20/33") |
| T (tercih) | `G5`, `G1`, `G3`, `G4`, `G2` | 1 ilk gösterilen · 2 ikinci gösterilen · fark yok |
| | `serbest` | eklemek istediği (aynen) |

Süre satırları: G1 ve G3 her kapsül, G5 ve G4 her set (madde `G5-1`, `G4-2` …), G2 her soru
(madde `G2-1.g2a`, `G2-1.g2b`, `G2-1.g2c`); `q` = `sure`, cevap = saniye.

## CSV (`answers.csv`)

UTF-8, bir satır = bir cevap (örnek: `answers.example.csv`, uydurma):

```
participant,group,mode,item,q,answer,hint,note,quote
A01,alışkın,ekran,G1-1a,s3,4,,,çarpı var
A01,alışkın,ekran,G1-1a,sure,34,,,
A01,alışkın,ekran,G5-1.2,yon,1,,,
A01,alışkın,ekran,G2-1,g2a,SYN09 yukarı,,,
A01,alışkın,ekran,T,G1,2,,,
```

- `item`: `G1-1a` … `G1-2c`, `G3-1a` … `G3-2b`, `G5-1.1` … `G5-2.3`, `G4-1.1` … `G4-2.3`,
  `G2-1`, `G2-2`; süre için `G5-1`, `G4-2`, `G2-1.g2a` gibi; tercih `T`. Set numarası formdaki
  settir, tasarım değil.
- `hint`: ipucu verildiyse `evet`, değilse boş.
- Boş alan boş yazılır (yanlış sayılır). İki seçenek işaretli ya da okunaksızsa `?` (yanlış,
  uyarıda listelenir). Kayda geçiren `key.md`'ye bakmaz.
