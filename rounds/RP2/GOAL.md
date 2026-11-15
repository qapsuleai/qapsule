# RP2 — Canlı Yol ve Veri Bağı

Bu round üç şeyi bir arada çözer, çünkü birbirine bağlıdırlar:

1. **Artımlı canlı yol.** Referans oturum her fiyat değişiminde dokümanı kopyalayıp
   hesapları baştan yapıyor (RD1/T6d). Canlı veride bu sürdürülemez.
2. **Veri bağı (K23).** Doküman bar verisi taşımaz; enstrüman, periyot ve aralığı
   söyleyen bir bağ taşır. Veri, kaynağından kendi biçiminde gelir.
3. **İki düzlemin tutarlılığı.** Veri protokolün dışından akınca yeni bir soru doğar:
   otorite "RSI 70'i kesti" dediğinde, cihazdaki yansı aynı veriye baktığını nasıl bilir?
   Bu round'un araştırma kısmı budur (K18).

## Değişmezler

- Artımlı yolun sonucu, baştan hesaplamayla **bayt düzeyinde aynıdır**.
- Hesap özeti (`fixtures/compute/digest.txt`) değişmez.
- Günlüğü baştan oynatmak aynı dokümanı üretir. Oturumda saat ve rastgelelik yoktur.
- Var olan altın görseller güncellenmez.
- Otorite ve yansı aynı motor kodunu çalıştırır.

## Kapsam

1. Veri bağı: şema, çözümleme, yetki hatası, dondurulmuş kanıt başvurusu.
2. Veri kaynağı olayları: yeni bar, son barın düzeltilmesi, **yalnızca değişen alan**.
3. Oturumun artımlı canlı yolu: kopyasız, yalnızca etkilenen hesap ve görünüm güncellenir.
4. İki düzlemin tutarlılığı: araştırma, alternatifler, ölçüm, seçilen tasarımın uygulanması.
5. K25 kararlarının uygulanması: belirsiz sunum, tek katman adı kuralı, geleceğe uzanan bakış.
6. R2'den kalan küçük düzeltmeler.

## Kapsam dışı

- Gerçek Matriks/NATS bağlayıcısı ve protobuf çözümü (R6). Burada aynı olay biçimini
  üreten sentetik bir kaynak kullanılır.
- Kapsül, anlam işaretleri, olgu dedektörleri (R3). Etkileşim ve GPU (R4).
- Ağ sunucusu, kimlik doğrulama, kalıcı depolama.

## Tasarım yönleri

**Veri bağı.** Bağ en az şunları söyler: enstrüman, periyot, aralık ya da pencere
("son N bar", "şu tarihten itibaren"). Oturum bağı, barındırıcının verdiği bir veri
sağlayıcı üzerinden çözer. Sağlayıcı "yetkin yok" diyebilir; bu, protokolde tanımlı bir
hatadır ve belgenin geri kalanı çalışmaya devam eder. Test verisi ve küçük örnekler için
belge içi veri bir kaynak türü olarak kalır.

**Dondurulmuş kanıt.** Bir bulgu, dayandığı veriyi başvuruyla ve içerik özetiyle
gösterir: hangi bağ, hangi bar aralığı, o aralığın özeti. Böylece "bu iddia o gün şu
veriye dayanıyordu" sonradan sınanabilir; veri değişmişse (düzeltme, bölünme) fark edilir.

**Tutarlılık.** En az üç yaklaşım kurulup ölçülür; liste sınır değildir:
- Otoritenin mesajlarına veri konumu iliştirmesi (ör. son bar zamanı ve o ana kadarki
  verinin özeti); yansı yetişmeden uygulamaz.
- Otoritenin anlam taşıyan değerleri (olgunun kanıt değerleri) mesajla göndermesi;
  yansının kendi hesabıyla karşılaştırması.
- Yansının yalnızca kendi verisiyle çalışması ve sapmayı sonradan bildirmesi.

Ölçüler: yanlış ya da erken gösterilen bulgu sayısı; gecikme; mesaj boyutu; veri
düzeltildiğinde davranış; ağ gecikmesi ve sıra bozulması altında dayanıklılık.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Veri bağı şemada ve `docs/PROTOCOL.md`'de; yeni kavramlar önce `docs/CONCEPTS.md`'de. Oturum bağı veri sağlayıcıyla çözüyor; "yetki yok" ve "veri yok" tanımlı hatalar, döküm var. |
| 2 | 10.000 barlı bağlı dokümanın anlık görüntüsü bar içermiyor; boyutu ölçülmüş ve RD1/T6d ile (750 KB / gzip 211 KB) karşılaştırılmış. Cihazda hazır olma süresi yeniden ölçülmüş. |
| 3 | Veri kaynağı olayları alan düzeyinde güncelleme taşıyor; sentetik kaynak bu biçimde üretiyor. Biçim, `docs/MATRIKS_PROTOCOL.md`'deki akışla eşlenebilirliği açısından değerlendirilmiş (uygulanmadan). |
| 4 | Artımlı canlı yol: tikte doküman kopyalanmıyor. Özellik testi: rastgele tik, yeni bar ve düzeltme dizileri için artımlı sonuç = baştan hesap, bayt düzeyinde. Ölçüm: 10.000 barlı belge, 5 / 50 / 500 hesap; tik başına oturum maliyeti, yerel ve WASM. Hedef: oturumun eklediği yük, hesap çekirdeğinin aynı işi tek başına yapma süresinin üç katını aşmıyor. Önce/sonra tablosu (RD1/T6d: 84 µs). |
| 5 | Tik yalnızca etkilenen görünümleri ve abonelikleri uyandırıyor; etkilenmeyenlere mesaj gitmiyor. Test var. |
| 6 | Dondurulmuş kanıt: bulgu veri başvurusu ve içerik özeti taşıyor; kanıt doğrulama sorgusu var; veri sonradan düzeltilirse sorgu farkı bildiriyor. Döküm var. |
| 7 | Tutarlılık: en az üç yaklaşım kurulmuş, ayrı dallarda, ayrı alt ajanlarla; ölçüm senaryoları ağ gecikmesi, sıra bozulması, veri düzeltmesi ve yansının geride kalmasını içeriyor. Seçilen yaklaşım uygulanmış ve şartnamede; öneri olarak `DECISIONS.md`'de. |
| 8 | Otorite ve yansı aynı tik akışıyla çalıştırıldığında doküman ve görünüm özetleri aynı (yerel ↔ WASM). Yansı geride kaldığında yanlış bulgu göstermiyor. |
| 9 | K25: (a) bulgu belirtilmeden sunum ve birden çok canlı bulgu → `ambiguous`, seçenekli; (b) katman adlarında tek kural; (c) bakış geleceğe uzanıyor, yelpaze ekranlı bağlantıda `screen` ve nokta sorgusunda görünüyor. Değişen dökümler tek tek listelenmiş ve gerekçelendirilmiş. |
| 10 | R2 kalanları: `welcome.summary` alan sırası; seçicili komutta doküman kopyası; sunucuda gereksiz kopyalar; döküm koşucusunun ham iletmesi; MACD paneli açıklaması ve RSI eşik etiketleri (yeni altın görsel gerektiriyorsa öneri olarak, güncellenmeden). |
| 11 | Uygunluk dökümleri üç taşımada geçiyor; yeni dökümler eklenmiş (veri bağı, yetki hatası, kanıt doğrulama, tutarlılık, belirsiz sunum, geleceğe bakış). |
| 12 | `REPORT.md`, `RESEARCH.md` (tutarlılık), `AUDIT.md`; `STATUS.md`, `ROADMAP.md`; önerilen kararlar `DECISIONS.md`'de ayrı bölümde. |
