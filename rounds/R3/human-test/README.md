# R3 — İnsan testi paketi (GOAL ölçütü 15, K39)

İşaret alfabesinin (M1) ve şablon cümlelerinin (M2) gerçek kullanıcıyla sınanması.
Yapay zekâ ön elemesi `siluet`'i önerdi, `rozet`'i yakın ikinci buldu
(`../experiments/M1/RESULTS.md`); K39 gereği son söz bu testtedir. Paket ikisini eşli
karşılaştırır; `iz` (özgün dil, uzun öğrenme) öneri olarak dışarıda kalır.

Katılımcıya dönük adlar tarafsızdır: **Alfabe M = `siluet`**, **Alfabe N = `rozet`**.

## İçindekiler

| Dosya | Ne |
|---|---|
| `GUIDE.md` | Görevli yönergesi: hazırlık, onam, aynen söylenecekler, süreler, söylenmeyecekler, kayıt |
| `kitapcik-A.html`, `kitapcik-B.html` | Uyaran kitapçıkları (9 sayfa; A: M önce, B: N önce). Sözlük, her alfabe için okuma / açıklama / öğrenme sayfası, 8 cümle, tercih |
| `form.html` | Basılacak cevap formu (3 A4 sayfa; kitapçıktan bağımsız) |
| `FORM.md` | Formun alanları, kodlar ve `answers.csv` biçimi |
| `pages/alfabe-{M,N}-okuma.png` | 16 işaret A1–D4 (A–B açıklamasız, C–D açıklamalı); her biri gerçek boyutta (24 px) ve 2× (48 px) |
| `pages/alfabe-{M,N}-ogrenme.png` | 5 cevaplı örnek E1–E5 ve 4 yeni işaret L1–L4 |
| `key.json` | Cevap anahtarı (katılımcı görmez) |
| `score.py`, `SCORING.md` | Puanlayıcı (`python3 -I score.py answers.csv`) ve kuralları |
| `answers.example.csv`, `answers.example.scored.md`, `make_example.py` | Uydurma örnek cevaplar ve puanlanmış hâli |
| `generate.sh`, `gen/` | Üretici (Rust; düzenek ve ürün kodunu kullanır) |

## Nasıl uygulanır

1. `GUIDE.md`'yi baştan sona okuyun; bir kez kendiniz geçin (yaklaşık 16 dakika).
2. Kişi başına bir `form.html` çıktısı; kâğıt gösterimde renkli kitapçık çıktısı, ekran
   gösteriminde telefonda kitapçık (`pages/` klasörüyle birlikte).
3. 5–10 kişi, iki grup (alışkın kullanıcı / genç, işlem yapmayan), her grupta A ve B
   kitapçıkları sırayla.
4. Formları `answers.csv`'ye geçirin (`FORM.md`), `python3 -I score.py answers.csv > results.md`.
5. Sonuç ve kararı `SCORING.md`'deki önceden yazılmış kurala göre yazın.

## Yeniden üretme

```sh
./generate.sh          # gen/ derlenir, pages/*.png, kitapcik-*.html, form.html, key.json yazılır; sha1 basılır
rm -rf gen/target      # disk az ise
```

Belirleyici: sabit tohum (`SEED = 1`, `gen/src/main.rs`), saat yok, ağ yok; iki koşu bayt
düzeyinde aynı çıktı verir (denendi). İşaretler M1 düzeneğinin çizim yolundan
(`Alphabet::draw` → `qp_render`, altın görsellerle aynı CPU yolu) çizilir; cümleler ürünün
`qp_capsule::realize::Template` cümlecisiyle, M2 düzeneğinin durumlarından üretilir. PNG'ler
2 aygıt pikseli / CSS pikseli yoğunluğundadır: "24 px" işaret 24 pikselde taranıp piksel
piksel ikiye katlanır (gerçek 24 px görünümü korunur), "2×" işaret 48 pikselde taranır;
HTML her PNG'yi yarı genişlikte gösterir, böylece ekranda ve kâğıtta (96 CSS px / inç)
gerçek boyutlu işaret tam 24 CSS pikseldir. Sayfa genişliği 360 px: telefonda küçültülmeden sığar.

Nasıl seçildi (ayrıntı `gen/src/main.rs` başındaki sabitlerde):

- **Okuma sayfası:** 16 anlam; her yarıda (A–B, C–D) altı türün hepsi ve beş aşamanın
  hepsi, güç 4 zayıf (0,25) / 4 güçlü (0,9); iki yarı birlikte `K`'nin 12 tür × yön
  birleşiminin hepsini içerir. Teyit (1–3) ve güven (0,3 / 0,8) tohumlu; tazelik 1, sunucu
  işareti yok (sorulmayan kanallar sabit). AI ön elemesinde aşama hep `final` kalmıştı;
  burada aşama dolaşır.
- **İki alfabe aynı anlam kümesini** görür (eşli karşılaştırma), alfabe adıyla tohumlu farklı sırada.
- **Öğrenme:** 5 örnek (5 tür, 5 aşama) ve 4 test; hiçbir test tür × yön × aşaması örneklerde yok.
- **Cümleler:** M2 durumlarından tohumlu karıştırmayla, sırayla şu ölçütlere uyan ilk
  (mümkünse türü kullanılmamış) durum: olay (rakamlı, geçmiş), olay (rakamsız), olay
  (bugün), süren durum, bitmiş durum (rakamsız), açık beklenti, kapanmış beklenti; artı K26.
  Sonuç: 3 olay, 3 durum (K26 dahil), 2 beklenti; 2'si lisanssız okuyucu için rakamsız.

## Sınırlar

- **Süre sıkı.** Kişi başına 40 işaret satırı (5 alan) + 8 cümle ≈ 16 dakika. "Yaklaşık
  on beş dakika" hedefi için her koşulda madde sayısı küçük tutuldu (açıklamasız 8,
  açıklamalı 8, öğrenme 4); koşul başına doğruluk bu yüzden gürültülüdür.
- **Açıklamasız ve açıklamalı maddeler farklıdır** (aynı sayfanın üst ve alt yarısı);
  aynı maddeyi iki kez sormak süreyi iki katına çıkarırdı. Yarılar tür ve aşamada
  dengelendi ama birebir eş değil; "açıklamanın katkısı" bir alfabe içinde değil, iki
  alfabe arasında aynı yarılar üzerinden karşılaştırılmalıdır.
- **Gerçek boyut ile 2× yan yana.** Katılımcı büyük olana bakabilir; okunan şey 24 px'in
  tek başına okunurluğu değil, ürünün "küçük işaret + açılınca büyük" kullanımına yakın
  bir durumdur. Saf 24 px okunurluğu M1 (a) hesap ölçüsünde.
- **Kâğıtta görsel açı biraz büyük:** 24 CSS px basılınca 6,4 mm; telefonda 24 CSS px
  yaklaşık 4 mm. Ekran (telefon) gösterimi birincildir; gösterim türü formda kaydedilir.
- **Önceki adıma dönme** kâğıtta tam önlenemez; görevli izler ve not eder.
- **Güç:** 5–10 kişiyle eşli işaret testi yalnız çok tek yönlü farkları yakalar (`SCORING.md`).
- İlk üretimde S7'de "Sinyalın ömrü" ünlü uyumu hatası vardı; ürün tarafında düzeltildi
  (S-C, `37b78b7`) ve paket `./generate.sh` ile yeniden üretildi.
- Gerçek piyasa verisi yok: bütün değerler M2 düzeneğinin uydurma durumlarıdır.

## Denetimden sonra (D3)

- Rakamsız ("sunucu doğruladı") cümleler artık yalnız akış türünden seçiliyor; önceki S7 bir
  fiyat dönüşüne bu etiketi veriyordu (üründe çıkmaz). Paket yeniden üretildi.
- Beklenti cümlelerinde "koşul hâlâ açık mı" sorusuna "bilinmiyor" cevabı savunulabilir;
  puanlamada bu soruyu ayrı raporlayın (`score.py` alan başına veriyor).
- Ekran gösteriminde 1. adımda alfabenin açıklaması sonraki ekranda duruyor: görevli kaydırmayı
  denetlemeli; daha sağlamı 1. adımı ayrı dosyada göstermek (öneri).
- Rozet'te çerçeve kalınlığı ve gri dolgu 24 px'te zor okunuyor (denetçinin gözlemi).
