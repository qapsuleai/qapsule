# R3 — Kapsül

Projenin adını veren şey bu round'da doğar. Girdi: `docs/CAPSULE.md` (çerçeve),
`DECISIONS.md` K26–K39, `docs/PROTOCOL.md`, `docs/GRAMMAR.md`.

## Ölçü

`NORTH_STAR.md`: "İnsan grafiğe bakmaz; grafik insana haber verir." İlke 5 (kapsül en
küçük birim), 6 (her öğe bir olguya bağlı), 7 (anlam zoom'u), 9 (ekransız çalışır) ve
üç vazgeçilmez: kanıta açılma, görünür karne, belirleyici hesap.

## Kapsam

1. **Olgu dedektörleri:** altı anlam türünün her biri için en az bir dedektör.
2. **Bulgu:** olguların birleşmesi, kanıt ailelerine göre teyit, önem sıralaması.
3. **Kapsülün yaşam döngüsü:** hikâye olarak; aday, kesinleşme, eskime, sonuç, geri çekilme.
4. **Karne:** sonuçların kaydı, ileriye bakma hatası olmadan.
5. **Dil:** üç zaman türü; Türkçe ve İngilizce; lisanslı veride rakamsız yorum.
6. **Anlam işaretleri:** alfabe; gramere işaret olarak eklenmesi; kapsül görünümü.
7. **Sunucu olgusu ve güven yolu;** otoritenin yanlış iddiasının onarımı (itiraz, geri çekme).
8. **Hassasiyet ve dikkat bütçesi.**
9. **Protokol eki, dökümler, ajan yazabilirlik.**

## Kapsam dışı

- Gerçek Matriks/NATS bağlayıcısı (R6). Etkileşim, GPU, mobil kabuk (R4, R8).
- Yorumlayan dil modeli ajanlarının çalışma ortamı. Bu round'un kapsülleri tamamen
  deterministik motordan çıkar; dil modeli yalnızca ölçümde okuyucu olarak yer alır.
- Tam formasyon kütüphanesi. Amaç altı türün her birinin uçtan uca çalışması.
- Kişiselleştirme için portföy ve hesap bağlantısı; pozisyon ve izleme listesi dokümandaki
  basit alanlardan okunur.

## Değişmezler

- Kapsülde yer alan her sayı motordan gelir; hiçbir cümle ve işaret olguya dayanmayan
  bilgi taşımaz (K32).
- Cümle, olgunun zamanını ve bugünkü geçerliliğini yansıtır (K26).
- Karne ileriye bakmaz: bir kapsülün doğduğu andaki karnesi yalnızca o andan önce
  sonuçlanmış kapsüllere dayanır.
- Yetkisiz kullanıcıya sayısal kanıt değeri ve veri özeti gitmez; yorum ve karne gider (K33, K36).
- Aynı veri ve aynı günlük her zaman aynı kapsülleri, aynı cümleleri ve aynı görselleri üretir.
- Var olan altın görseller ve hesap özeti değişmez.

## Ölçümle karar verilecek konular (K39)

Aşağıdakilerin her biri için en az iki, tercihen üç alternatif, birbirini görmeyen alt
ajanlarca en iyi halleriyle kurulur; ortak ve önceden yazılmış bir düzenekle ölçülür.

| # | Konu | Ölçüm |
|---|------|-------|
| M1 | **İşaret alfabesi** | (a) Küçük boyutta (16, 24, 32 px) ikili ayırt edilebilirlik, hesapla; renk körlüğü benzetimi; açık ve koyu tema. (b) Okuma testi: taze, görsel okuyabilen alt ajanlara yalnızca işaret gösterilir; tür, yön, güç, aşama sorulur; bir kez açıklamasız, bir kez kısa açıklamayla. (c) Öğrenme: beş örnekten sonra yeni işaretlerde doğruluk. Bu bir ön elemedir; insan testi paketi ayrıca hazırlanır. |
| M2 | **Cümle üretimi** | Şablon dilbilgisi ile motorun doğruladığı yuvalara dil modelinin yazdığı cümle. Ölçüler: zaman doğruluğu (K26), belirleyicilik, lisanslı olguda ham değer sızıntısı, taze alt ajanların cümleyi okuyup "ne oldu, ne zaman, hâlâ geçerli mi" sorularına doğru cevabı, maliyet. İki dilde. |
| M3 | **Önem sıralaması** | En az üç kural (ör. nadirlik; karne; bağımsız teyit sayısı; birleşimleri). Ölçüler: seviyeye göre günlük kapsül sayısının dağılımı ve kararlılığı; aynı enstrümanda art arda gelen kapsül sayısı; sonradan gerçekleşenlerin payı. |
| M4 | **Tema kimliği** (hangi olgular aynı hikâyeye aittir) | En az iki kural. Ölçüler: kapsül sayısındaki azalma, yanlış birleştirilen ayrı olaylar, kaybolan bilgi. |
| M5 | **Sonuç tanımı** (tür başına "gerçekleşti" ne demek, ufuk kaç bar) | En az iki tanım. Ölçüler: karnenin örneklem büyüdükçe kararlılığı; tanımın küçük değişikliğine duyarlılık; ileriye bakma sızıntısı testi (gelecekteki veriyi karıştırınca geçmiş karne değişmemeli). |
| M6 | **Sunucu olgusu güven yolu** | En az iki tasarım. Ölçüler: yetkisiz kullanıcıya giden mesajlardan ham değerin geri çıkarılması denemesi (art arda gelen nitel yorumlardan değer aralığını daraltan saldırgan); gösterim gecikmesi; kullanıcıya dürüstlük (doğrulanmamış olduğunun belli olması). |
| M7 | **İtiraz ve geri çekme** (A20) | RP2 düzeneği ve denetçinin AUD1–AUD3 senaryoları; hedef: yanlış gösterim 0 ve kaçırılan kesin iddia RP2'ye göre belirgin azalmış. |

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Altı anlam türünün her biri için en az bir dedektör; her biri tipli olgu üretiyor, kanıt değerleri ve dondurulmuş kanıt başvurusuyla. Dedektör çıktısı toplu ve akışkan çalıştırmada bit düzeyinde aynı. Yeni kavramlar koddan önce sözlükte. |
| 2 | Aday: bar kapanmadan görülen oluşum yalnızca yansıda, yerel veriden hesaplanıyor; otorite günlüğüne ve karneye girmiyor; bar kapanınca kesinleşiyor ya da kayboluyor. Test var. |
| 3 | Bulgu: kanıt aileleri (fiyat, hacim, akış) tanımlı; aynı aileden birden çok olgu tek teyit sayılıyor; teyit sayısı bulguda ve işarette görünüyor. |
| 4 | Yaşam döngüsü: aynı enstrüman ve tema için tek kapsül evriliyor; durum geçişleri (doğdu, güçlendi, zayıfladı, teyit edildi, gerçekleşti, geçersiz, süresi doldu, geri çekildi) dökümlerde. Eskime bar sayısıyla. Günlükten yeniden oynatma aynı kapsül geçmişini üretiyor. |
| 5 | Karne: her kapsül bir sonuçla kapanıyor; karne tür ve dedektör başına tutuluyor ve kapsülde görünüyor. İleriye bakma testi geçiyor. "Geri çekildi" karneye yazılmıyor. |
| 6 | Dil: olay, durum ve beklenti türleri ayrı; cümleler Türkçe ve İngilizce; K26 testi: R2'deki "RSI 70'in üstünde" durumu (güncel değer 55) artık doğru zamanla sunuluyor. Lisanslı olgunun cümlesi rakam içermiyor. |
| 7 | İşaret alfabesi M1 ölçümüyle seçilmiş; gramere ek olarak (var olan görünümleri kırmadan) yeni bir işaret türü; altı tür × yön × aşama için altın görseller; kapsül görünümü işaret, cümle, küçük grafik ve karneyi birlikte gösteriyor. |
| 8 | Anlam zoom'u: aynı kapsül `sentence`, `capsule` ve `chart` derinliklerinde; `chart` derinliğinde formasyonun geometrisi (ör. kırılan seviye, bant, tepe noktaları) olgunun kanıtından çiziliyor. Her derinliğin altın görseli var. |
| 9 | Kanıta açılma: yetkili kullanıcı kanıt değerlerini görüyor; yetkisiz kullanıcı yorumu, karneyi ve "bu kanıt şu veri yetkisini gerektirir" bilgisini alıyor. Yetkisize sayısal değer ve veri özeti gitmediği testle kanıtlı. |
| 10 | Sunucu olgusu: sentetik bir lisanslı akış beslemesiyle uçtan uca; M6 ile seçilen güven yolu şartnamede; geri çıkarma saldırısının sonucu raporda. |
| 11 | İtiraz ve geri çekme: M7 ölçümü; seçilen tasarım uygulanmış; "geri çekildi" durumu dökümde. |
| 12 | Hassasiyet: üç seviye; iki kullanıcı grubu için varsayılanlar; pozisyon ve niyetle ilgili kapsüller seviyeden bağımsız geliyor; sistem seviyeyi kendiliğinden değiştirmiyor. Benzetim: en az 250 işlem günü, en az 30 enstrüman; seviye başına günlük kapsül dağılımı raporda. |
| 13 | M1–M7'nin her biri için: alternatifler, düzenek (alternatiflerden önce yazılmış), ham sonuçlar, seçim ve gerekçe `rounds/R3/experiments/` altında. Seçimler `DECISIONS.md`'de öneri olarak. |
| 14 | Veri: sentetik üreteç farklı piyasa rejimleri üretiyor (yükselen, düşen, yatay, oynak). `fixtures/local/` altında gerçek geçmiş veri varsa ölçümler onunla da koşulmuş ve ayrı raporlanmış; yoksa bu açıkça yazılmış ve karnenin sentetik veride anlamlı bir başarı oranı göstermediği belirtilmiş. Gerçek veri depoya girmiyor. |
| 15 | İnsan testi paketi: `rounds/R3/human-test/` altında yazdırılabilir ya da ekranda gösterilebilir işaret sayfaları, soru formu, uygulama yönergesi (yaklaşık on beş dakika, beş ila on kişi) ve sonuçların nasıl puanlanacağı. |
| 16 | Protokol eki şartnamede; yeni dökümler üç taşımada; var olan 36 döküm değişmeden geçiyor (değişen varsa gerekçeli). Ajan yazabilirlik: kapsülle ilgili en az yirmi görev ("bugün bu hissede ne var", "bu kapsülün kanıtını aç", "neden geçersiz oldu", "bunu reddet"); R2 düzeneğiyle. |
| 17 | Performans: 30 enstrüman × altı dedektör canlı tikte; tik başına maliyet RP2 tabanıyla karşılaştırılmış. WASM boyutu ölçülmüş. |
| 18 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md`; `docs/CAPSULE.md` uygulamayla tutarlı hale getirilmiş (çerçeve değişmeden, ayrıntı eklenerek). |
