# R2 — Görsel Gramer (protokolün kelime dağarcığı)

RD1 konuşmayı tasarladı; bu round konuşulan dili tasarlar. Görünüm artık opak bir yük
değil: her parçası kimlikle ve anlamla adreslenebilen, protokol mesajlarıyla kurulan,
derinliğe göre indirgenen ve ekransız okunabilen bir yapıdır.

Girdi: `docs/PROTOCOL.md` (`qp/0`), `rounds/RD1/R2_INPUT.md` (G1–G10), `DECISIONS.md`
K21–K23.

## Ölçü

`NORTH_STAR.md` ilke 6, 7, 8 ve 9: her görsel öğe bir olguya bağlı; anlam zoom'u;
sabit grafik tipleri değil gramer; ekransız çalışır.

## Kapsam

1. Gramer v0 tipleri ve `docs/GRAMMAR.md`.
2. Protokol eki: gramer işlemleri kimlikle; anlamla başvuru; `welcome`'da doküman özeti.
3. Sahne tarifi (çizimden bağımsız ara biçim) ve gramer derleyicisi.
4. Referans çizicinin sahne tarifinden çizmesi; altın görseller.
5. Sunucunun (presenter) gramer kurallarına taşınması; derinliğe göre indirgeme.
6. Görüş alanı, ayrıntı düzeyi (LOD).
7. Anlamsal okuma: `screen` ve nokta sorgusu sahne tarifinden.
8. Uygunluk dökümleri ve ajan yazabilirlik ölçümü (gramer görevleri).
9. R1 denetiminden kalan küçük düzeltmeler.

## Kapsam dışı

- Etkileşimin uygulanması ve GPU yolu (R4). Anlam işaretlerinin ve kapsülün tasarımı (R3).
- Oturumun artımlı canlı yolu ve veri bağı (RP2). Canlı veri bağlayıcıları (R6).
- Çizim araçları, yeni indikatörler.

## Gramer v0'ın asgari içeriği

Adlar ve yapı T1'de tasarlanır, `docs/GRAMMAR.md`'de gerekçelendirilir.

- **Panel:** dikey yığılmış, ortak zaman ekseni; her panelin kendi değer ölçeği.
- **Katman:** bir işaret + veri başvurusu + kodlamalar. Her katmanın kalıcı kimliği var (G1).
- **İşaretler:** mum, dikdörtgen çubuk, çizgi, alan, bant (iki seri arası), yatay/dikey
  kural, nokta, metin. Belirsizlik için yelpaze/koni bunlarla kurulabilmeli.
- **Ölçekler:** bar sırasına dayalı zaman (seans boşlukları kapalı), doğrusal, logaritmik.
- **Kodlama kanalları:** kapalı ve tipli bir küme (RD1/T6e: serbest kanal adları modelin
  tanımsız kanallar yazmasına yol açtı). x, y, y2, renk (sabit, koşullu, değere bağlı),
  opaklık, kalınlık, boyut.
- **Veri başvurusu:** seri alanı, hesap çıktısı, olgu ya da bulgu. Sabit değerli işaretler
  (ör. 30/70 kuralı) de bir olguya ya da adlandırılmış bir eşiğe bağlanır; kökeni olmayan
  işaret otoritede reddedilir (G3).
- **Kalıcı aralık ile geçici bakış** ayrı kavramlardır (G6).
- **Hedef:** dikkat yönergesi ve anlatım adımı bir gramer öğesini işaret edebilir; gramer
  vurgu, odak ve karartmayı çizebilir (G7).
- **Derinlik:** aynı bulgunun `sentence`, `capsule`, `chart` derinliklerinde ne
  gösterdiği gramer kuralıdır; yüzey profili indirgemeyi belirler (G4).
- **Tema** görünümden ayrıdır.

## Anlamla başvuru

RD1'in en önemli bulgusu: ajanlar kimlikleri bilmediğinde ilk denemede başarı 20'de 6'ya
düştü. Bu round'da ajan bir öğeye kimlik yerine anlamla başvurabilmeli: "RSI(14)'ün
paneli", "kapanış fiyatı serisi", "en son kırılım bulgusu", "fiyat panelindeki üst bant".

Tasarım serbesttir (K18); alt sınır:
- Başvuru deterministik çözülür. Belirsizse otorite seçenekleri sıralayan bir hata döner;
  rastgele birini seçmez.
- Çözülen kimlik cevapta geri verilir; günlükte kimlik saklanır (yeniden oynatma değişmez).
- `welcome` ve `screen`, ajanın ilk mesajında neye başvurabileceğini görmesine yetecek
  bir doküman özeti verir.
- En az iki farklı tasarım kurulup ajan yazabilirlik düzeneğiyle ölçülür.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Gramer tipleri `qp-schema`'da; JSON Schema ve MCP araçları ondan üretiliyor; `docs/GRAMMAR.md` her kavramı örnekle anlatıyor, elenen tasarım seçenekleri sonunda; yeni kavramlar `docs/CONCEPTS.md`'de (koddan önce). |
| 2 | Katman, panel, ölçek kimlikle adresleniyor; gramer işlemleri (`insert_layer`, `delete_layer`, taşıma, kodlama değiştirme) kimlikle; indeksli alanlar `qp/0`'da duruyor ama kullanımdan kalkmış olarak işaretli. Eşzamanlı yazma senaryosunda (T6c'deki 18 durum) yanlış sonuç 0. |
| 3 | R0'daki mum+SMA sahnesi yalnızca protokol mesajlarıyla kurulmuş bir görünümden çizildiğinde var olan altın görselle bayt düzeyinde aynı. Olmuyorsa fark ölçülmüş, nedeni açıklanmış, yeni görsel onay için önerilmiş; altın görsel güncellenmemiş. |
| 4 | Beş yeni görünüm yalnızca protokol mesajlarıyla kurulmuş (motor koduna dokunmadan), her birinin altın görseli var: (a) üç panel: fiyat, RSI, MACD histogramı; (b) Bollinger bandı alan olarak; (c) belirsizlik yelpazesi (sentetik üç olasılık bandı); (d) logaritmik ölçek; (e) koşullu renkli çizgi/alan. |
| 5 | Derinlik: aynı bulgu için `sentence`, `capsule` ve `chart` çıktıları gramer kurallarından üretiliyor; sunucu (presenter) bu kurallara taşınmış; `capsule` ve `chart` derinliklerinin altın görselleri var. Üç yüzey profili (saat, telefon, masaüstü) indirgemesi dökümde. Sunucu çıktısı deterministik (RD1/T6e ölçümü tekrarlandı). |
| 6 | Kökeni olmayan işaret reddediliyor; hata iletisi katmanı, alanı ve nedeni söylüyor, yakın doğru seçeneği öneriyor. Geçersiz kodlama kanalı reddediliyor. Testler var. |
| 7 | Anlamsal okuma: `screen` her görünümü panel/katman/işaret/veri başvurusu olarak özetliyor; nokta sorgusu bir pikseldeki öğeyi panel, katman kimliği, bar sırası, zaman, değerler ve veri başvurusuyla veriyor. Veri → piksel → veri gidiş-dönüş özellik testi geçiyor. İkisi de sahne tarifinden çalışıyor; çiziciye ihtiyaç duymuyor. |
| 8 | Anlamla başvuru: en az iki tasarım kurulmuş ve ölçülmüş; seçilen tasarım şartnamede. Belirsiz başvuru seçenekli hata veriyor. |
| 9 | Ajan yazabilirlik (gramer): en az yirmi yeni görev (ör. "RSI'ı fiyatın altına ayrı panel olarak ekle", "üst bandı kırmızı yap", "son kırılımı vurgula", "bunu saat için göster"). Düzenek her zaman kimlikleri kopyalanamaz dokümanla koşuyor; alt ajanlar yalnızca şartname ve şemayı görüyor ve bu kısıt yönergeyle değil dosya erişimiyle sağlanıyor. Ölçüler: ilk denemede başarı, sonunda başarı, mesaj ve belirteç. RD1 v2r tabanıyla (ilk 6/20, son 19/20) karşılaştırma; en az bir tasarım düzeltmesi ve yeniden ölçüm. |
| 10 | Görüş alanı ve LOD: 100.000 bar görünürken kararlı kare süresi referans çizicide ölçülmüş; hedef dizüstü sınıfında 8 ms'nin altı. LOD'lu ve tam çizim farkı raporlanmış. Eski viewport/LOD matematiğinden alınan ve alınmayan yazılmış. |
| 11 | En az sekiz yeni uygunluk dökümü; üç taşımada geçiyor. RD1'in 17 dökümü değişmeden geçiyor. |
| 12 | R1 denetim bulguları kapatılmış: periyot üst sınırı, `Chain` düzeltme hatası ve özellik testi, göreli tolerans; `period: 1e18` iletisi (RD1 D3). |
| 13 | WASM boyutu ölçülmüş; RD1 ile karşılaştırılmış. |
| 14 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md` güncel; önerilen karar satırları `DECISIONS.md`'de ayrı bölümde. |
