# Grafik yüzeyleri — maketler (RD2 M2/M3)

Şartname: [`../../../surfaces/chart.md`](../../../surfaces/chart.md) ve Y27 için
[`../../../surfaces/phone.md`](../../../surfaces/phone.md). Yön `tavek`
([`../../tavek/DIRECTION.md`](../../tavek/DIRECTION.md)).

| Yüzey | Kademe | Gösterilen | Profil |
|---|---|---|---|
| [Y01](Y01.html) Grafik hücresi | 1 | 11 / 11 durum + ana durum | masaüstü + telefon |
| [Y09](Y09.html) Grafik üstünde izleme işareti | 1 | 12 / 12 + ana durum | masaüstü + telefon |
| [Y11](Y11.html) Ajan katmanı | 1 | 8 / 8 + ana durum | masaüstü + telefon |
| [Y18](Y18.html) Lejant ve panel başlığı | 1 | 10 / 10 + ana durum (masaüstü) | masaüstü + telefon |
| [Y27](Y27.html) Telefonda grafik | 1 | 7 / 7 + ana durum (Ç4) | yalnız telefon |
| [Y02](Y02.html) Çizim araç çubuğu | 2 | ana durum + 8 satırlık liste | masaüstü |
| [Y03](Y03.html) Seçili çizim denetimleri | 2 | ana durum + 8 | masaüstü |
| [Y10](Y10.html) Köken ve nesne listesi | 2 | ana durum + 8 | masaüstü |
| [Y16](Y16.html) Kanıta açılma | 2 | ana durum + 7 | masaüstü |
| [Y17](Y17.html) Gösterge seçici | 2 | ana durum + 9 | masaüstü |
| [Y23](Y23.html) Periyot, karşılaştırma, ölçek | 2 | ana durum + 8 | masaüstü |

Kademe 1 sayfalarında her durum şartname tablosunun sırasıyla etiketli bir karedir
(numara, ad, kalıp adı; altında gösterilen, rol, metin kalıbı). Kademe 2 sayfaları ana
durumu tam hücre olarak, öbür durumları örnekli bir tabloda verir. Her ana kare yanında
akış (numaralı adımlar, bu karenin adımı işaretli), köken anahtarı ve telefon farkı durur.

## Açma

1. Değişken dosyalarını ve çekirdek grafiklerini üret (çıktı git-ignored klasöre gider,
   bu klasöre hiçbir şey yazılmaz):

   ```sh
   docs/design/mockups/surfaces/chart/render.sh
   # → rounds/RD2/experiments/gallery/out/surfaces/chart/{vars,charts}
   ```

   `render.sh` `qp look css tavek|tavek-tel --theme dark|light`, yazı tipleri ve
   `qp render --look … --scale 2|3 --transcript fixtures/design/desk.json --view desk`
   çalıştırır (`qp`: `$CARGO_TARGET_DIR` ya da ana ağacın `target/debug/qp`).

2. Sayfayı parametrelerle aç (`O` = yukarıdaki çıktı klasörünün `file://` adresi):

   ```text
   Y01.html?look=tavek&theme=dark&vars=O/vars/&charts=O/charts/tavek-dark/&dpr=2
   Y01.html?look=tavek-tel&theme=light&vars=O/vars/&charts=O/charts/tavek-tel-light/&dpr=3
   ```

   `look=tavek` masaüstü bölümlerini, `look=tavek-tel` telefon bölümlerini gösterir (aynı
   sayfa). `index.html` aynı parametrelerle açılırsa bağlantılar parametreleri taşır.
   Ekran görüntüsü: masaüstü DPR 2, telefon DPR 3.

## Kurallar ve ölçüler

- Değerler yalnız `var(--…)`; `chart.css` içinde renk kodu yok. Px yalnız çerçeve
  geometrisi: grafik yuvalarının boyutu (`render.sh` ile aynı: hücre 900×540 ve 600×380,
  telefon 390×300 / 390×340 / yatay 760×300, telefon görünümü 390×844, güvenli alanlar),
  Y18 kırpması (150) ve yatay kip çerçevesi.
- Bindirme konumları (`--x`, `--y`) o boydaki çekirdek sahnesinden (`qp scene`) hesaplanan
  kesirlerdir: seviye fiyatları, bar indisleri ve lejant satırının sonu veriye oturur.
  Hikâye fikstürün kendi barlarında: 120,50 direnci (Ağustos tepeleri), 11 Eyl kapanış
  120,79 ile kırılım, 15 Eyl üç bar tutunma; 117,50’de 2 Eyl kırılıp 3 Eyl geri dönüş;
  ajan önerisi 115,20 (28 Ağu ve 5 Eyl dipleri); motor önceki tepe 130,68.
- Sayfalar statik HTML’dir; metin sentetik, gerçek piyasa verisi yok. Görseller commit
  edilmez.

## Açıklar (çekirdek / gramer işi; maket bunları elle gösteriyor)

- Seviye eksen etiketi, sol uç çipi, kanıt bandı ve parantezi, niyet hapı bugün
  çekirdekte yok (DIRECTION "Bilinen zayıflıklar"); maket bindirme olarak çiziyor.
- Çakışma: dar hücrede (600×380, telefon) 120,50 etiketi son değer etiketine (124,69)
  değiyor; maket etiketi birkaç piksel kaydırarak çekirdeğin P6 davranışını taklit ediyor.
  Parantez altındaki tarih etiketini ve izleme etiketi altındaki fiyat etiketini çekirdek
  düşürmeli.
- Y18: şartname gösterge satırlarını OHLC satırının altına diziyor; çekirdek bugün
  SMA/EMA değerlerini ilk satırın sağına yazıyor. Maket çekirdeğin lejant satırını örtüp
  dizili lejantı gösteriyor. Dar hücrede `D-*` sözcüğü için yer kalmayınca sağdaki gösterge
  değerleri örtülüyor (çekirdekte kısaltma kuralı gerekir).
- Durum görselleri: gizli seri, hata bölmesi, bayat değer çizgisi ve oynatmada gelecek
  bölmelerin hacim/RSI etiketleri çekirdekte değişmeli; maket yalnız örtüyor.
- Y23: düzeltme seçimi ("Düzeltilmiş") sağ alt köşeye sığmadığı için üst çubuğun sonunda.
