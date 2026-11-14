> **BEKLEMEDE.** Bu round RD1 (protokol tasarımı) bittikten sonra yeniden yazılacak. Bu haliyle koşulmaz.

# R2 — Görsel Gramer ve Çizim

## Hedef

"Mum grafiği çizen motor"dan "ajanın görünüm kurabildiği gramer"e geçmek. Bir görünüm
tamamen veridir: ajan onu yazar, motor çizer, her piksel kaynağına geri izlenebilir.
Aynı görünüm hem deterministik referans çizicide hem GPU'da çalışır.

## Kapsam

1. Görsel gramer v0: şema tipleri ve `docs/GRAMMAR.md`.
2. Gramer derleyicisi: Görünüm + veri → çizimden bağımsız sahne tarifi.
3. Referans çizicinin sahne tarifinden çizmesi.
4. Görüş alanı ve ayrıntı düzeyi (LOD).
5. İsabet testi ve açıklama: pikselden veriye geri dönüş.
6. GPU çizim yolu ve web gösterimi.
7. R1 denetiminden kalan küçük düzeltmeler.

## Kapsam dışı

- Etkileşim (fare, dokunma, hareketler) ve durum yönetimi (R4, R7). Görüş alanı burada
  yalnızca görünümün bir alanıdır.
- Anlam işaretlerinin tasarımı ve kapsül (R3). Gramer onları ifade edebilecek
  genişlikte olmalı, ama tasarlanmayacak.
- Çizim araçları, yeni indikatörler, canlı veri.
- Hesap çekirdeği performansı (RP1).

## Gramer v0'ın asgari içeriği

Aşağısı alt sınırdır; adlar ve yapı T1'de tasarlanır ve `docs/GRAMMAR.md`'de gerekçelendirilir.

- **Panel:** dikey yığılmış, ortak zaman ekseni; her panelin kendi değer ölçeği.
- **Katman:** bir işaret + veri bağı + kodlamalar.
- **İşaretler:** mum, dikdörtgen çubuk, çizgi, alan, bant (iki seri arası), yatay/dikey
  kural, nokta, metin. Belirsizlik için yelpaze/koni bu işaretlerle kurulabilmeli.
- **Ölçekler:** bar sırasına dayalı zaman (seans boşlukları kapalı), doğrusal, logaritmik.
- **Kodlamalar:** x, y, y2, renk (sabit, koşullu, değere bağlı), opaklık, kalınlık, boyut.
- **Veri bağı:** seri alanı ya da adlandırılmış hesap çıktısı (`StudyOutput`). Her katman
  neyi çizdiğini bildirir; kökeni olmayan katman olmaz.
- **Görüş alanı:** görünen bar aralığı, değer ekseni otomatik ya da sabit.
- **Tema:** renkler ve yazı boyutları görünümden ayrı; aynı görünüm farklı temayla çizilebilir.

Tasarım testi: bu gramerle, motor koduna dokunmadan, R0'daki mum+SMA sahnesi ve aşağıdaki
beş görünüm yalnızca JSON yazarak kurulabilmeli.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Gramer tipleri `qp-schema`'da; JSON Schema üretiliyor; `docs/GRAMMAR.md` her kavramı örnekle anlatıyor; yeni kavramlar `docs/CONCEPTS.md`'de. |
| 2 | R0'daki mum+SMA sahnesi bir Görünüm dokümanı olarak yazılmış ve gramer yolundan çizildiğinde var olan altın görselle bayt düzeyinde aynı. Aynı olmuyorsa fark ölçülmüş, nedeni açıklanmış, yeni görsel onay için raporda önerilmiş; altın görsel güncellenmemiş. |
| 3 | Beş yeni görünüm yalnızca JSON olarak eklenmiş, her birinin altın görseli var: (a) üç panel: fiyat, RSI, MACD histogramı; (b) Bollinger bandı alan olarak; (c) belirsizlik yelpazesi (sentetik veriyle üç olasılık bandı); (d) logaritmik ölçek; (e) koşullu renkli çizgi/alan. Denetçi her görseli açıp gözle inceliyor ve en az iki sayısal noktayı veriyle karşılaştırıyor. |
| 4 | Geçersiz görünüm anlaşılır hata veriyor (hangi katman, hangi alan, neden); ajan hatadan düzeltme yapabilecek kadar bilgi alıyor. Test var. |
| 5 | Görüş alanı ve LOD: 100.000 bar görünürken kararlı kare süresi referans çizicide ölçülmüş; hedef dizüstü sınıfında 8 ms'nin altı. LOD açıkken görsel, tam çizimle karşılaştırılmış ve fark raporlanmış. Eski kodun viewport/LOD matematiği değerlendirilmiş; alınan ve alınmayan kısımlar yazılmış. |
| 6 | İsabet testi: `qp explain <görünüm> --x --y` o pikseldeki öğeyi veriyor: panel, katman, bar sırası, zaman, değerler, veri bağı. Veri → piksel → veri gidiş-dönüş özellik testi geçiyor. |
| 7 | GPU yolu aynı sahne tarifini çiziyor. Referansla karşılaştırma ölçütü tanımlanmış ve gerekçelendirilmiş (bayt eşitliği değil, tolerans); altı görünümün hepsi bu ölçütü geçiyor. Kare süreleri ölçülmüş. GPU bulunmayan ortamda test açıkça SKIP olarak raporlanıyor, sessizce geçmiyor. |
| 8 | Web gösterimi: önce CPU karesi, GPU hazır olunca ona geçiş; WebGPU yoksa CPU'da kalıyor. Başsız tarayıcı testi var. |
| 9 | R1 denetim bulguları kapatılmış: periyot üst sınırı, `Chain` düzeltme hatası ve özellik testi, göreli tolerans. |
| 10 | WASM boyutu ölçülmüş ve R1 ile karşılaştırılmış; büyüme nedenleriyle raporlanmış. |
| 11 | `REPORT.md`, `AUDIT.md` yazılmış; `STATUS.md`, `ROADMAP.md` güncellenmiş. |
