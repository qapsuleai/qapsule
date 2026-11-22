# Kurumsal işlemler ve izlemeler (K88; RS1 girdisi)

## Yön ve biçim

Arkin'in yönü: bölünme akşam yapılır; yeni barlar ve fiyatlar Matriks'ten alınabilir; o
sırada var olan izleme kendini yenileyip yeni fiyatlara göre izlemeye dönmeli ya da
kullanıcıya mesaj atmalı.

Kabul edilen biçim (K88):

- Kurumsal işlem bir olaydır: sembol, tür, geçerlilik tarihi, düzeltme katsayısı.
- Geçerlilik anından yenileme bitene kadar o sembolün izlemeleri "beklemede" durur; fiyat boşluğu stop ya da kırılım diye okunmaz.
- Yenileme: düzeltilmiş barlar alınır; çizim, düzey ve izleme koşulları aynı katsayıyla taşınır; hesaplar yeniden kurulur. Taşıma deterministiktir.
- Kural: çizim, grafiğin gösterdiği seriyle aynı düzeltmeyi alır.
- Sonra kullanıcıya tek mesaj: ne oldu, kaç izleme taşındı, eski ve yeni düzeyler. Taşıma otomatik yapılamıyorsa (veri gelmedi, katsayı belirsiz) izleme beklemede kalır ve kullanıcıya sorulur.
- Sicilde açık iddiaların düzeyleri de taşınır; kayda katsayı yazılır. Taban oranlar düzeltilmiş seriden hesaplanır.
- Küresel: aynı olay modeli ABD bölünmeleri için de geçerlidir.
