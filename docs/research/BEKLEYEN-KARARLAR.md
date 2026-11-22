# Bekleyen kararlar (ana akış notu)

Arkin'in 2026-11-22'de verdiği, henüz karar kaydına işlenmemiş kararlar. RG1 karar
kaydını böldüğü için işleme RG1 bittikten sonra yapılır; o zaman bu dosya silinir.

## Toplu kabul

- RP4 önerileri Ö77–Ö93: kabul.
- R5B önerileri Ö94–Ö102: kabul.

## Sekiz karar (hepsi önerildiği gibi kabul)

1. Gün içi gösterimde yeni kural seti (`rp4`) varsayılan; protokol varsayılanı eski halinde kalır (Ö79).
2. Gün içi çıkış kuralı (A53): önceden yazılı kuralın seçtiği kalır; öbür aday sicil round'unda taze veriyle yeniden bakılır.
3. Yazı-tura eşiği (A54): sabit 0,5'e göre değil, türün kendi taban oranına göre; sicil round'unda uygulanır.
4. Dedektörlere en kısa periyot sınırı (A55): kırılım, ortalama dizilimi, RSI aşırılığı en kısa günlük; sıkışma en kısa 15 dakika. Sınırlar parametredir.
5. Elindeki hissede yazı-tura düzeyindeki tespit (A57): olumsuz olan "düşük güven" etiketiyle bildirilir; olumlu olan bildirilmez.
6. Araç yüzeyi istemciye göre seçilir (A60): tembel yükleyenlerde bugünkü, öbürlerinde küçük yüzey.
7. Doğrulayıcı katmanlıdır (A61): kişiye özel yorumda bugünkü doğrulayıcı + metin normalleştirme; ortak analizde sayılar yalnız motorun kurduğu cümlelerden gelir.
8. Eski `qp mcp --init --principal` bağdaştırıcısı kaldırılır (A62); RG1'den sonraki round'un ilk işi.

Karar gerektirmeyen notlar: A59 oyun alanı round'una (R7), A58 atlanır (K82).

## Kurumsal işlemler ve izlemeler (A56'nın genişlemesi; sicil round'u RS1 kapsamı)

Arkin'in yönü: bölünme akşam yapılır; yeni barlar ve fiyatlar Matriks'ten alınabilir; o
sırada var olan izleme kendini yenileyip yeni fiyatlara göre izlemeye dönmeli ya da
kullanıcıya mesaj atmalı.

Ana akışın önerdiği biçim (Arkin'in onayı bekleniyor):

- Kurumsal işlem bir olaydır: sembol, tür, geçerlilik tarihi, düzeltme katsayısı.
- Geçerlilik anından yenileme bitene kadar o sembolün izlemeleri "beklemede" durur; fiyat boşluğu stop ya da kırılım diye okunmaz.
- Yenileme: düzeltilmiş barlar alınır; çizim, düzey ve izleme koşulları aynı katsayıyla taşınır; hesaplar yeniden kurulur. Taşıma deterministiktir.
- Kural: çizim, grafiğin gösterdiği seriyle aynı düzeltmeyi alır.
- Sonra kullanıcıya tek mesaj: ne oldu, kaç izleme taşındı, eski ve yeni düzeyler. Taşıma otomatik yapılamıyorsa (veri gelmedi, katsayı belirsiz) izleme beklemede kalır ve kullanıcıya sorulur.
- Sicilde açık iddiaların düzeyleri de taşınır; kayda katsayı yazılır. Taban oranlar düzeltilmiş seriden hesaplanır.
- Küresel: aynı olay modeli ABD bölünmeleri için de geçerlidir.
