# T6e — Deterministik sunucu (presenter) mi, dil modeli grameri mi yazsın?

Deney: RD1 / T6e. Sınanan çalışma kararı: K21-1 (`DECISIONS.md`) — "dil modeli çizim
döngüsünde yer almaz; ajan anlam düzeyinde konuşur, motordaki deterministik sunucu gramere
çevirir. Ajan grameri doğrudan da yazabilir."

Alternatif: her sunumda dil modeli tam `View` JSON'unu (grameri) kendisi yazar.

## 1. Hipotez (ölçümden önce yazıldı)

H1 — Gecikme. `show_finding` uçtan uca (oturum: ayrıştırma, yetki, açılım, uygulama,
günlük) release derlemede medyanı < 100 µs, p99 < 1 ms olur; sunucu fonksiyonunun kendisi
< 10 µs. Dil modeli ile grameri yazmak ise çağrı başına yüzlerce ms – birkaç saniye sürer;
yani telefonda "bir kare / 100 ms" bütçesini model yolu karşılayamaz, sunucu yolu 3–4 kertelik
payla karşılar.

H2 — Belirlenimcilik. Aynı doküman 1 000 çalıştırmada bayt bayt aynı görünümü verir
(tek farklı çıktı). Model yolunda N örnek içinde birden fazla farklı çıktı olur (sıcaklık
> 0 ise neredeyse kesin; yapı bile değişebilir: panel sayısı, renk, alan sırası, `title`).

H3 — Doğruluk. Sunucu her zaman şemaya uyar ve anlamca doğrudur (var olan seriler,
hesap kendi panelinde, olgu seviyesi kural olarak işaretli). Model çıktısı çoğunlukla
şemaya uyar ama anlam hataları (yanlış/uydurma seri kimliği, `domain` için bar indeksi
yerine zaman damgası karışıklığı, `encoding.y`'nin sayı olarak yazılması, olgu barının
işaretlenmemesi) örneklerin anlamlı bir kısmında görülür; küçük model büyükten daha sık
hata yapar.

H4 — Maliyet ve ölçek. 10k kullanıcı × 50 kapsül görüntüleme/gün × birkaç yüzey
≈ 10⁶ sunum/gün. Sunucu yolunda maliyet bir CPU çekirdeğinin saniyelerine denk gelir
(pratikte sıfır). Model yolunda sunum başına ~3–5k girdi + ~200–400 çıktı token'ı ile
günlük maliyet küçük modelde yüzlerce, büyük modelde binlerce dolar mertebesinde olur.

H5 — Denetim. Sunucu yolunda bir düzenleyici, günlükteki `show_finding` girdisini ve
dokümanı yeniden oynatarak görünümü bayt bayt yeniden üretebilir. Model yolunda
yeniden üretim yalnızca modelin çıktısı saklanırsa mümkündür; "neden böyle göründü"
sorusunun cevabı ise modelin içindedir ve tekrar sorulduğunda farklı olabilir.

Beklenen sonuç: K21-1 doğrulanır. Grameri ajanın doğrudan yazması, sunucunun bilmediği
yeni görünümler (ilke 8) için değerlidir; ama bu yazım bir kez yapılır, komut yolundan
geçer, dokümana kaydedilir ve sonraki her sunumda deterministik olarak yeniden kullanılır —
yani model yine çizim döngüsünde değildir.

Çürütme ölçütü: model yolu medyan gecikmede < 100 ms'ye inerse, N örneğin tamamında tek
bir yapısal çıktıya yakınsarsa ve anlam hatası yapmazsa, K21-1'in "model çizim döngüsünde
değil" kısmı gerekçesini kaybeder (yalnızca maliyet gerekçesi kalır).
