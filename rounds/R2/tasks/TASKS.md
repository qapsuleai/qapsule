# R2/T1 — Yirmi gramer görevi (taslak, tiplerden önce)

Ajan yazabilirlik ölçümünün (T9) görevleri. Grameri ve anlamla başvuruyu sınar. Tiplerden
**önce** yazıldı; puanlama ölçütleri tasarımdan bağımsızdır (sonuç dokümana ve cevaba
bakar, yazılan mesajın biçimine değil).

**Doküman:** RD1'in `thyao` dokümanı, **her koşuda kimlikleri yeniden adlandırılmış**
(rastgele, koşu başına farklı; şartname örneklerindeki kimlikler kopyalanamaz). Gizli
kurulum ayrıca şunları ekler: SMA(50) serisi (görünümde değil); `kind: breakout` iki olgu
(bar 120 ve bar 185, fiyat serisinde, `evidence.level`) ve onlara dayanan iki bulgu (eski ve
yeni kırılım); kullanıcı `arkin` masaüstünde bağlı. Görünüm: fiyat paneli (mum + Bollinger
çizgisi), RSI paneli (çizgi + 70 kuralı), MACD paneli.

| # | Görev (kullanıcının sözü) | Başarı ölçütü (son doküman / cevap) |
|---|---------------------------|-------------------------------------|
| R01 | "RSI'ı fiyatın hemen altına ayrı panel olarak ekle." (RSI(7), yeni) | Yeni RSI(7) serisi; onu çizen çizgi katmanı **yeni** bir panelde; o panel fiyat panelinin hemen altında (panel sırasında 2.). |
| R02 | "Üst bandı kırmızı yap." | Bollinger `upper`'ı çizen bir katmanın rengi kırmızı (tema `down` ya da kırmızı tonlu onaltılık); `middle` ve `lower` hâlâ çiziliyor ve kırmızı değil. |
| R03 | "Son kırılımı vurgula." | Görünümde yeni kırılım bulgusunu (bar 185) hedefleyen bir vurgu; eski kırılım vurgulanmamış. |
| R04 | "Bunu saat için göster: kullanıcıya ne söylenir?" (bulgu = yeni kırılım) | `present` saat yüzeyi (ya da `sentence` derinliği) sorgulandı; cevap metni bulgunun kapsül başlığı/iddiasıyla aynı. |
| R05 | "Fiyat panelini logaritmik ölçeğe çevir." | Fiyat panelinin y ölçeği `log`; diğer paneller değişmemiş. |
| R06 | "RSI paneline 30 çizgisini de ekle." | RSI panelinde y = 30 kural katmanı, kökeni aşırı satım eşiği; 70 kuralı duruyor. |
| R07 | "MACD panelini kaldır." | MACD'yi çizen katman yok ve MACD paneli yok; MACD serisi dokümanda duruyor (yalnız görünümden kalktı) ya da silindi — ikisi de kabul; diğer paneller duruyor. |
| R08 | "Hacmi fiyatın altına çubuk olarak ekle." | Fiyat serisinin `v` alanını çizen `bar` işaretli katman; fiyat panelinden farklı bir panelde; o panel fiyat panelinin hemen altında. |
| R09 | "Bollinger bandını dolgulu alan olarak göster." | `upper` ile `lower` arasını dolduran bir katman (`band` ya da `y2`'li `area`), opaklık < 1. |
| R10 | "MACD histogramını çubuk yap; pozitifse yeşil, negatifse kırmızı." | MACD `histogram` çıktısını çizen `bar` katmanı; rengi değerin işaretine göre koşullu (≥ 0 yeşil/`up`, < 0 kırmızı/`down`). |
| R11 | "Kapanışı SMA50'nin üstündeyken yeşil, altındayken kırmızı çiz." | Fiyat panelinde kapanışı çizen katman; rengi kapanış ≥ SMA50 koşuluyla (SMA50 var olan seri). |
| R12 | "Grafiği son 60 bara ayarla, böyle kalsın." | Görünümün kalıcı aralığı son 60 bar (`x.domain` bar 140–199). |
| R13 | "Ekranda şu an ne var? Kaç panel, hangi göstergeler?" | Cevap panel sayısını ve her paneldeki göstergeleri doğru söylüyor (`screen` ile). |
| R14 | "Fiyat panelinde, 1200×600'lük ekranda (900, 200) pikselinde ne var?" | Nokta sorgusu yapıldı; cevaptaki bar ve değer otoritenin cevabıyla aynı. |
| R15 | "RSI çizgisini kalınlaştır (2 px)." | RSI çizgi katmanının kalınlığı 2. |
| R16 | "Eski kırılımın olgusunu fiyat panelinde nokta olarak işaretle." | Fiyat panelinde eski kırılım olgusuna (bar 120) bağlı `point` katmanı. |
| R17 | "Fiyat paneline 123,45'te yatay çizgi çiz." | Kökensiz sabit reddedilir; ajan ya bir olgu ekleyip ona bağlar (`add_fact` + kural, köken olgu, değer 123,45) ya da kullanıcıya neden yapılamadığını ve seçeneği söyler. İkisi de başarı; kökensiz çizgi dokümana girerse başarısız. |
| R18 | "MACD panelini RSI'ın üstüne taşı." | Panel sırası: fiyat, MACD, RSI. |
| R19 | "Bu bulguyu telefonda nasıl görünür? Kaç bar gösterilir?" (yeni kırılım) | `present` telefon; cevap derinliği (`capsule`) ve gösterilen bar sayısını (profilin `max_points`'i, 60) doğru söylüyor. |
| R20 | "Bant genişliğini ayrı panelde çizgi olarak göster." | Bollinger `bandwidth` çıktısını çizen çizgi katmanı, fiyat panelinden ayrı (yeni) bir panelde. |

## Notlar

- Bazı görevler bir **çıktıya** (`upper`, `histogram`, `bandwidth`) başvurur; bazıları bir
  **bulguya** anlamıyla ("son kırılım"); bazıları bir **panele** ("RSI paneli"); biri
  **kökensiz sabit** reddini (R17); ikisi **ekransız okuma** (R13, R14); ikisi **derinlik**
  (R04, R19).
- R02 bilerek zor: Bollinger tek katmanla çiziliyorsa, yalnız üst bandı boyamak katmanı
  bölmeyi gerektirir. Ölçüm bunun ne kadar zor olduğunu gösterecek.
