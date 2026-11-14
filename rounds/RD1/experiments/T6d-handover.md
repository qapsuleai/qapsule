# T6d — Doküman devri (hand-over) mi, sunucu otoritesi mi? (K21-3)

Sınanan çalışma kararı (K21-3): "motor hem sunucuda hem cihazda çalışır, doküman
devredilebilir". Deneme kodu: `experiments/t6d-handover/` (çalışma alanı üyesi değil).
Ham çıktılar: `rounds/RD1/experiments/raw/t6d/`.

## 1. Hipotez (ölçümden önce yazıldı)

Senaryo: 10 000 barlık bir kapsül dokümanı (px + RSI(14) + MACD + Bollinger + bir olgu,
bir bulgu, canlı bir kapsül) sunucu otoritesinden cihaza devredilir; cihaz onu canlı tutar
(saniyede 1 yeni bar, saniyede 60 son-bar güncellemesi).

Alternatifler: **A** otorite devri (anlık görüntü + seq, cihazda `epoch+1` ile yeni
`Session`, hesaplar cihazda `Study::extend` ile geçmişe bağlanır); **B** sunucu otorite
kalır, cihaz ince ayna (anlık görüntü bir kez, sonra `change` olayları; hesap değerleri
sunucudan); **C** varyantlar (anlık görüntü + günlük; hesap çıktılarını anlık görüntüde
taşımak; aynada yerel hesap).

Beklentiler:

- **H1 (boyut):** 10 000 bar satır biçiminde JSON ≈ 0,7–0,8 MB ham, gzip ≈ 0,25–0,3 MB.
  Sütunlu biçim gzip'te belirgin (%20–40) küçülür. Hesap çıktılarını taşımak (7 sütun ×
  10 000) boyutu kabaca iki katına çıkarır; hesaplamaktan pahalıdır, taşınmamalı.
  Anlık görüntü `max_message_bytes` (1 MiB) sınırına yakın; 10 000+ bar sınırı aşabilir.
- **H2 (hazır olma):** cihazda ayrıştırma + `Session` kurma + üç hesabı `extend` ile
  bağlama yerelde < 30 ms, WASM'da (Node) < 100 ms. En büyük kalem JSON ayrıştırma.
- **H3 (canlı, A):** artımlı yol (akışlı `push`/`update_last` + koşul değerlendirme)
  tik başına < 10 µs. Referans `Session` ise her komutta dokümanı kopyalar ve hesap
  önbelleğini temizleyip toplu yeniden hesaplar: tik başına O(n), ~0,5–2 ms. 60/s'te
  dizüstünde sığar ama telefon sınıfında (K17, R4) israftır; cihaz otoritesi için
  artımlı bir yol gerekir.
- **H4 (canlı, B):** v0 protokolünde değer itme konusu yok; ayna her tikte `study`
  sorgusu yapmak zorunda. Tik başına ~0,3 KB `change` + ~0,5 KB sorgu/sonuç →
  ~40–60 KB/s ve sunucuda cihaz başına O(n) iş. "Değer itme" ya da aynada yerel hesap
  (belirleyicilik sayesinde) bunu ~15 KB/s'ye ve O(1)'e indirir.
- **H5 (çevrimdışı/geri dönüş):** A çevrimdışı çalışmaya devam eder ama canlı veri de
  ağdan geldiği için "canlı" kısmı zaten durur; 60 s canlı tik günlüğü (3 600 girdi)
  ≈ 1–2 MB, anlık görüntüden büyük. Sunucuda yeniden oynatma < 50 ms. §8.4'teki epoch
  kuralı soy karışıklığını *tespit eder* ama sunucu tarafında bu arada gelen veriyi
  *birleştirmez*; ayrıca devir sırasında doküman diğer istemciler (ajanlar) için
  erişilemez olur.
- **H6 (belirleyicilik):** cihaz ve sunucu aynı girdilerle bayt düzeyinde aynı dokümana
  ve bit düzeyinde aynı hesap değerlerine ulaşır; yerel ve WASM özetleri eşittir.

Ön tahmin: K21-3 "inceltilmeli": varsayılan sunucu otoritesi + cihazda yerel hesap
(ayna), tam devir yalnızca tek kullanıcılı/çevrimdışı oturumlar için.
