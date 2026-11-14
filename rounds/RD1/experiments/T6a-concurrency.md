# T6a — Eşzamanlı yazma: tek otorite + sıralı günlük mü, CRDT mi?

Round: RD1. Sınanan çalışma kararı: **K21-2** — "belge başına tek otorite ve sıralı işlem
günlüğü; her yazı kökeniyle kaydedilir". Bu deneyin amacı kararı savunmak değil; her
alternatifi en iyi savunucusunun kuracağı biçimde kurup dürüstçe ölçmek.

## 1. Hipotez (ölçümden önce yazıldı)

Bu bölüm kod yazılmadan ve hiçbir ölçüm alınmadan önce yazılıp ayrı bir commit olarak
kaydedildi. Sonuçlar bölümünde her madde tek tek doğrulanır ya da çürütülür.

Karşılaştırılacaklar:

- **A — Otorite + günlük:** `docs/PROTOCOL.md` §8'deki model; öğe düzeyinde iyimser
  eşzamanlılık (`base`), girdi başına `inverse`, `undo_conflict`.
- **B — CRDT (Automerge 0.12):** aynı doküman Automerge haritaları/listeleriyle;
  aktör kimliği = kişi; kendi senkron protokolü.
- **C — CRDT (Loro 1.17):** aynı doküman Loro konteynerleriyle; peer kimliği = kişi;
  yerleşik `UndoManager`.

Beklentiler:

1. **Yakınsama.** Üç model de bütün replikalarda aynı son duruma ulaşır (A bunu sırayla,
   B/C matematiksel olarak garanti eder). Fark yakınsamada değil **anlamsal geçerlilikte**
   çıkacak: CRDT'ler yapısal olarak birleşir ama alan değişmezlerini (tek kimlik, katmanın
   silinmiş seriye işaret etmemesi, kapsülün hem `rejected` hem canlı/onaylı olmaması,
   karnede aynı bulgu için çelişen iki karar) bilmez. Beklenti: A'da anlamsal anomali **0**;
   B ve C'de çakışmalı iş yükünde **sıfırdan büyük ve iş yüküyle artan** sayıda anomali
   (öngörü: 10 000 işlemde onlarca–yüzlerce).
2. **Aynı kimlikle eşzamanlı `add_study`.** A'da ikinci yazan `duplicate` alır. CRDT'de
   harita anahtarı çakışır: biri sessizce kazanır (Automerge: aktör sırasına göre; diğeri
   "conflict" olarak saklanır ama uygulama okumazsa kaybolur) ya da iki yazının alanları
   karışır (alan alan birleşme → hiç kimsenin yazmadığı bir parametre kümesi).
3. **Katman indeksi.** "scout 2. indekse ekler, arkin 1. katmanı siler": A'da `base` yoksa
   sunucu sırasıyla uygulanır ve scout'un katmanı niyet edilenden bir kayık yere düşebilir;
   `base` varsa `conflict` döner. CRDT'de liste öğe kimlikleriyle niyeti korur (CRDT bu
   konuda **daha iyi**).
4. **Mesaj boyutu.** İşlem başına tel boyutu: A'nın JSON komutu ~150–400 bayt, değişim
   bildirimi (tam öğe taşıdığı için) daha büyük, ~300–1500 bayt. Automerge/Loro'nun ikili
   değişim kodlaması işlem başına daha küçük (~50–200 bayt). Çevrimdışı dönüşte toplu
   senkron boyutu CRDT'de daha küçük olur (sıkıştırılmış sütunsal kodlama). Tam geçmişli
   doküman boyutu: A'nın JSON günlüğü (inverse dahil) en büyük olur; CRDT'ler 3–10× küçük.
5. **Süre.** A'nın tek işlem uygulaması mikrosaniye mertebesinde (klonlama ve JSON'a göre
   ~1–20 µs); CRDT'lerin işlem başına uygulaması ve özellikle birleştirme (merge) daha
   yavaş; 1 000 işlemlik çevrimdışı pencerenin birleşmesi CRDT'de milisaniyeler. Hiçbiri
   etkileşimli kullanım için engel değil (10 000 işlem < 1 s).
6. **Kod karmaşıklığı.** A'nın modeli (doğrulama + inverse + çakışma kontrolü) satır
   sayısında CRDT modellerinden **daha uzun** olur, çünkü kütüphane yerine her şeyi
   kendimiz yazarız. Ama istemcinin anlaması gereken kavram sayısı A'da daha az
   (komut, ack/error, seq, base) — CRDT'de aktör, heads/version vector, conflict değerleri,
   senkron durumu, container tipleri.
7. **Köken ve geri alma.** A'da her son değer günlükteki bir girdiye (yazar + onaylayan)
   izlenir; "arkin yalnızca kendi son değişikliğini geri alır" ve başkasının sonraki işi
   ezilirse `undo_conflict` döner. Automerge'de yazan aktöre op kimliği üzerinden ulaşılır
   ama "onaylayan" kavramı yoktur (uygulama alanı olarak yazılmalı ve doğrulanamaz);
   yerleşik undo yoktur. Loro'nun `UndoManager`'ı yalnızca yerel peer'in işini geri alır ve
   uzak değişiklikleri koruyacak biçimde dönüştürür — bu konuda Loro A kadar iyi ya da daha
   iyi olabilir; ama aynı alana sonradan yazılmışsa sessizce üzerine yazar (çakışma bildirmez).
8. **Determinizm.** A: aynı girdi → aynı bayt (BTreeMap, sabit sıra). Automerge/Loro: aynı
   aktör/peer kimlikleri ve aynı işlem sırasıyla aynı bayt; zaman damgası verilmezse
   deterministik. (Doğrulanacak; Automerge değişimlerine varsayılan zaman damgası
   koymayabilir, Loro koyabilir — kapatılması gerekebilir.)
9. **Çevrimdışı ajanın deneyimi.** A: dönüşte çakışan yazıları `conflict` hatası olarak
   **açıkça** görür, okuyup yeniden gönderir (ek gidiş-dönüş). CRDT: hiçbir hata görmez,
   her şey sessizce birleşir; yazısının ezilip ezilmediğini anlamak için geçmişi ayrıca
   incelemesi gerekir.

Ön tahmin (karar için): K21-2 korunur, çünkü bu ürünün değeri anlamsal geçerlilik, onay
ve denetim izinde; CRDT'nin üstün olduğu yerler (liste niyeti, çevrimdışı birleşme, tel
boyutu) ya küçük ya da otorite modeline eklenebilir (ör. katman kimliği). Kanıt aksini
gösterirse bu tahmin raporda açıkça çürütülür.
