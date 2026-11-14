# T6a — Eşzamanlı yazma: tek otorite + sıralı günlük mü, CRDT mi?

> **Yeniden üretim (denetim O1):** bu ölçüm `7776abb` (dal `rd1/t6a-exp`; kod `58de8e3`) üzerinde yapıldı. Deney crate'i, ölçümden sonra değişen protokol tiplerine (`now`, `ack.undoes`, boş yapı varyantları, `put_*.index`) taşınmadığı için güncel `main`'de derlenmez; yeniden üretmek için o commit kullanılır: `git checkout 7776abb && cd experiments/t6a-concurrency && cargo run --release -- all`.

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

## 2. Yöntem

Kod: `experiments/t6a-concurrency/` (ana çalışma alanının üyesi değil; kendi `[workspace]`
tablosu var, `qp-schema`'ya yol bağımlılığı). Çalıştırma:
`cd experiments/t6a-concurrency && cargo build --release && REPS=5 REPS_BIG=3 ./target/release/t6a-concurrency all`.
Ham çıktılar: `rounds/RD1/experiments/raw/t6a/` (`runs.json`, `cases.json`, `stdout-run1.txt`,
`machine.txt`).

Makine: Apple M4 Max (`sysctl -n machdep.cpu.brand_string`), 128 GB, rustc 1.92.0, release
(`lto = "thin"`, `codegen-units = 1`), tek iş parçacığı. Kütüphaneler: `automerge = "=0.12.0"`,
`loro = "=1.17.1"` (crates.io'daki güncel sürümler, 2026-11-14).

### 2.1 Modeller (her biri kendi en iyi savunucusunun kuracağı gibi)

- **A — otorite + günlük** (`src/log_model.rs`). PROTOCOL.md §2/§8/§9'un sadık küçük modeli:
  belge `qp_schema::Document`; telde gerçek `ClientMsg::Command`, `ServerMsg::{Ack, Error,
  Notify}`, `LogEntry`, `ChangeOp` tipleri JSON olarak kodlanıp **çözülür** (her abone
  bildirimi ayrıştırıp uygular). İşlemler: `add_study`, `set_params`, `add_layer`,
  `remove_layer`, `remove` (seri/bulgu), `add_finding`, `update_finding`,
  `approve/reject_finding`, `add_intent`, `undo`. Her girdi `inverse` taşır, dokunduğu
  kimlikleri kaydeder; `base` öğe düzeyinde denetlenir (görünüm katmanları için kimlik
  `view:main`, karne için `ledger`). Kararı verilmiş bulgu dondurulur (`update_finding` →
  `invalid`). Yeniden oynatma değişmezi (§2) her koşuda bayt düzeyinde doğrulanır
  (`assert_eq!`), ayrıca üç istemci yansısının otoriteyle bayt bayt aynı olduğu da.
  Üç değişke:
  - `A-log(base,strict)`: §8.2 yazıldığı gibi.
  - `A-log(base,own-excl)`: çakışma denetimi gönderenin **kendi** girdilerini yok sayar
    (önerilen iyileştirme; bkz. §6).
  - `A-log(no-base)`: istemciler `base` göndermez (sunucu sırasıyla son yazan kazanır).
  İstemci politikası (en iyi savunucu): hata gelirse yeniden okur, niyeti **kimlikle**
  yeniden çözer (katman indeksi → çapa serisi), yerelde doğrular, en çok iki kez yeniden
  gönderir; doğrulama geçmezse bırakır (açık ret). Çevrimdışı istemci yerel iyimser bir
  kopya tutar, işlemleri kuyruğa alır; dönüşte kuyruğu **eski `base`** ile gönderir, sonra
  `log` sorgusuyla yetişir.
- **B — Automerge 0.12** (`src/am_model.rs`). Replika başına bir `AutoCommit`, aktör kimliği
  = kişi (`[1]`, `[2]`, `[3]`). Öğeler kimlikle anahtarlanmış haritalar, `params` iç içe
  harita, katmanlar ve karne harita listeleri; barlar tek opak `Bytes` skaler (işbirlikli
  düzenlenmez). Çevrimiçi değişimler yerel commit başına yerel değişim baytı olarak
  yayılır (`load_incremental`), dönen replika Automerge **sync protokolünü** çalıştırır.
  Automerge'de undo yoktur; uygulama düzeyinde ve **dürüst** bir undo yazıldı: geri
  alınacak değerin şu anki kazanan op'u hâlâ geri alanın op'u değilse reddeder.
- **C — Loro 1.17** (`src/loro_model.rs`). Replika başına `LoroDoc`, peer kimliği = kişi.
  Öğeler Loro belgelerinin önerdiği **mergeable** haritalar (`ensure_mergeable_map`:
  "aynı anahtar = aynı varlık"), katmanlar/karne `LoroList` içinde haritalar. Commit başına
  `export(updates)`; dönen replika sürüm vektörü değiş tokuşu + updates. Undo: Loro'nun
  yerleşik `UndoManager`'ı (yalnızca yerel peer, uzak değişikliklere göre dönüştürülmüş).

CRDT istemcileri de yazmadan önce yerel görünüm üzerinde aynı doğrulamayı yapar
(`common::validate`); yani CRDT'ye "körü körüne yazma" haksızlığı yapılmadı. Fark yalnızca
eşzamanlılıkta ortaya çıkar.

Bilinçli sadeleştirmeler: öneri kipi (K21-5) modellenmedi — üç yazar da doğrudan yazar
(scout ve editor `editor` rolünde, karar ve undo yalnızca `arkin`'de); hibrit (otorite +
serbest metin için CRDT) ayrıca kodlanmadı, §6'da tartışıldı; `y` ölçek dizisi panel
eklenince güncellenmez (çizimle ilgili, eşzamanlılıkla değil).

### 2.2 İş yükü

`common::workload(seed, ops, window)`: sabit tohum (`0x5EED0001`), splitmix64, duvar saati
yok. Turlar halinde: her turda üç yazar birer niyet üretir (`ops/3` tur). Her niyet,
**yazarın kendi o anki görünümüne** göre somut işleme çevrilir (aynı rastgele çekilişler,
modelin kendi replikası üzerinde çözülür — her model "doğal" davranır). Yazara özgü
ağırlıklar (arkin: parametre, katman silme/ekleme, karar, undo; scout: çalışma, bulgu,
bulgu güncelleme; editor: parametre, katman, çalışma). Çakışma üreten sıcak noktalar:
`set_params` çağrılarının %30'u MACD'ye; karar/güncellemelerin %40'ı `fnd-overbought`'a;
çalışma kimliklerinin yarısı 8'lik bir havuzdan (`sma20`, `ema9`…), bulgu kimliklerinin
üçte biri 6'lık havuzdan — aynı kimlikle eşzamanlı oluşturma bu yüzden doğal olarak oluşur.

Eşzamanlılık: bir tur içindeki yazılar birbirini görmez (tur sonu = gecikme sınırı); editor
koşunun üçte birinden itibaren `window` tur çevrimdışıdır, sonra döner. Boyutlar:
100/1 000/10 000 işlem; pencere 10/100/1 000 (10 000 işlemde üçü de).

### 2.3 Ölçümler

- **Yakınsama:** tüm replikaların anlamsal anlık görüntüsü (`Snap`, BTreeMap'li JSON) bayt
  bayt eşit mi; A'da ayrıca `Document` JSON'ları bayt bayt eşit.
- **Anlamsal anomaliler** (`common::anomalies`, son durumda): katman yok seriye işaret
  ediyor; hesap girdisi/olgu/geçersizlik koşulu/niyet tetikleyicisi yok seriye işaret
  ediyor; kapsül ya da karne kaydı yok bulguya işaret ediyor; kapsül durumu son kararla
  çelişiyor ya da bir bulguya birden çok karar; kararın verildiği iddia metni şimdikiyle
  farklı (karar bayatlamış); çalışmaya ait olmayan parametre.
- **Eşzamanlılık sayaçları** (modelden bağımsız, vektör saatleriyle): yerelde kabul
  edilmiş yazılardan birbirini görmeden aynı alana yazan çiftler; aynı kimliği oluşturan
  çiftler; aynı bulguya eşzamanlı karar ve güncelleme. A'da bunun yerine otoritenin
  döndürdüğü `conflict` hataları ve `base` olmadan oluşan "kör üzerine yazma" sayılır.
- **Boyut:** yazı başına yukarı bayt (A: komut JSON'u; B/C: yerel değişim/update baytı),
  abone başına aşağı bayt (A: `notify` JSON'u; B/C: aynı değişim), dönüşte çevrimdışı
  ajanın gönderdiği/aldığı bayt ve tur sayısı, tam geçmişli doküman (A: başlangıç belgesi
  JSON + tüm `LogEntry` JSON'u, inverse dahil; B: `save()`; C: `export(Snapshot)`), gzip'li
  hali.
- **Süre:** yalnızca çoğaltma işi (yazma + kodlama + yayma + çözme + uygulama + dönüş
  senkronu); görünüm oluşturma ve iş yükü üretimi hariç. 100/1 000 işlemde 5, 10 000'de 3
  tekrarın medyanı. Tekrarlar arasında geçmiş baytlarının ve son durumun SHA-256'sı aynı
  olmak zorunda (`assert`), yani **determinizm her koşuda denetlendi**.
- **Kod karmaşıklığı:** boş ve yorum satırı hariç satır sayısı (rustfmt sonrası).


## 3. Sonuçlar — iş yükü

Tüm sayılar `raw/t6a/runs.json`'dan üretildi (`stdout-run1.txt` aynı koşunun özeti). A satırlarında "uygulandı" ilk denemede kabul edilen yazılar, "hata→yeniden" otoriteden açık hata alıp yeniden okuduktan sonra kabul edilenler, "açık ret" sonunda bırakılanlardır (yazar nedenini bilir). B/C'de "açık ret" yalnızca uygulama düzeyinde undo'nun yerelde reddettikleridir. "Eşzamanlı aynı alan çifti" vektör saatiyle sayılır ve çiftleri sayar (uzun pencerede karesel büyür); bunların her birinde CRDT sessizce bir kazanan seçmiştir.

### 3.1 Yakınsama, anomali, yazar deneyimi

| işlem | pencere | model | yakınsadı | anomali (tür) | uygulandı | hata→yeniden | açık ret | eşzamanlı aynı alan çifti | eşzamanlı aynı kimlik | karar∥güncelleme |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 100 | 10 | A-log(base,own-excl) | evet | 0  | 82 | 7 | 6 | — | — | — |
| 100 | 10 | A-log(base,strict) | evet | 0  | 82 | 7 | 6 | — | — | — |
| 100 | 10 | A-log(no-base) | evet | 0  | 89 | 0 | 6 | — | — | — |
| 100 | 10 | B-automerge | evet | 1 (stale_decision=1) | 92 | 0 | 2 | 1 | 0 | 2 |
| 100 | 10 | C-loro | evet | 1 (stale_decision=1) | 96 | 0 | 0 | 1 | 0 | 2 |
| 1000 | 10 | A-log(base,own-excl) | evet | 0  | 788 | 102 | 20 | — | — | — |
| 1000 | 10 | A-log(base,strict) | evet | 0  | 788 | 102 | 20 | — | — | — |
| 1000 | 10 | A-log(no-base) | evet | 0  | 891 | 0 | 19 | — | — | — |
| 1000 | 10 | B-automerge | evet | 3 (dangling_layer=2,stale_decision=1) | 905 | 0 | 6 | 17 | 0 | 2 |
| 1000 | 10 | C-loro | evet | 1 (stale_decision=1) | 913 | 0 | 0 | 17 | 0 | 2 |
| 1000 | 100 | A-log(base,own-excl) | evet | 0  | 812 | 86 | 18 | — | — | — |
| 1000 | 100 | A-log(base,strict) | evet | 0  | 812 | 86 | 18 | — | — | — |
| 1000 | 100 | A-log(no-base) | evet | 0  | 898 | 0 | 18 | — | — | — |
| 1000 | 100 | B-automerge | evet | 8 (dangling_layer=2,dangling_ref=1,stale_decision=5) | 910 | 0 | 6 | 121 | 1 | 13 |
| 1000 | 100 | C-loro | evet | 6 (dangling_ref=1,stale_decision=5) | 918 | 0 | 0 | 121 | 1 | 13 |
| 10000 | 10 | A-log(base,own-excl) | evet | 0  | 7885 | 997 | 106 | — | — | — |
| 10000 | 10 | A-log(base,strict) | evet | 0  | 7885 | 997 | 106 | — | — | — |
| 10000 | 10 | A-log(no-base) | evet | 0  | 8883 | 0 | 105 | — | — | — |
| 10000 | 10 | B-automerge | evet | 3 (dangling_layer=1,stale_decision=2) | 8944 | 0 | 28 | 57 | 0 | 4 |
| 10000 | 10 | C-loro | evet | 5 (dangling_ref=2,stale_decision=3) | 8984 | 0 | 0 | 61 | 0 | 4 |
| 10000 | 100 | A-log(base,own-excl) | evet | 0  | 7907 | 973 | 104 | — | — | — |
| 10000 | 100 | A-log(base,strict) | evet | 0  | 7907 | 973 | 104 | — | — | — |
| 10000 | 100 | A-log(no-base) | evet | 0  | 8880 | 0 | 104 | — | — | — |
| 10000 | 100 | B-automerge | evet | 12 (dangling_ref=3,stale_decision=9) | 8944 | 0 | 28 | 169 | 0 | 13 |
| 10000 | 100 | C-loro | evet | 13 (dangling_ref=4,stale_decision=9) | 8986 | 0 | 0 | 180 | 0 | 13 |
| 10000 | 1000 | A-log(base,own-excl) | evet | 0  | 8081 | 816 | 92 | — | — | — |
| 10000 | 1000 | A-log(base,strict) | evet | 0  | 8081 | 816 | 92 | — | — | — |
| 10000 | 1000 | A-log(no-base) | evet | 0  | 8898 | 0 | 91 | — | — | — |
| 10000 | 1000 | B-automerge | evet | 50 (dangling_layer=4,dangling_ref=19,stale_decision=27) | 8950 | 0 | 25 | 10780 | 0 | 94 |
| 10000 | 1000 | C-loro | evet | 33 (dangling_layer=1,dangling_ref=5,stale_decision=27) | 8988 | 0 | 0 | 11272 | 0 | 94 |

### 3.2 Boyut ve süre

| işlem | pencere | model | yukarı/yazı medyan B | aşağı/abone medyan B | geçmişli belge B | gzip B | dönüş: ajan→ B | dönüş: ajana B | dönüş tur | çoğaltma süresi ms (medyan) | µs/işlem |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 100 | 10 | A-log(base,own-excl) | 134 | 361 | 75480 | 10041 | 2324 | 19924 | 14 | 1.1 | 11.4 |
| 100 | 10 | A-log(base,strict) | 134 | 361 | 75480 | 10041 | 2324 | 19933 | 14 | 1.0 | 9.9 |
| 100 | 10 | A-log(no-base) | 120 | 361 | 75472 | 10037 | 1815 | 19330 | 11 | 0.9 | 8.9 |
| 100 | 10 | B-automerge | 186 | 199 | 8906 | 8528 | 2407 | 7767 | 2 | 11.0 | 110.3 |
| 100 | 10 | C-loro | 509 | 549 | 28778 | 16067 | 1704 | 2269 | 2 | 5.1 | 51.1 |
| 1000 | 10 | A-log(base,own-excl) | 133 | 349 | 570196 | 42431 | 1616 | 15780 | 13 | 10.2 | 10.2 |
| 1000 | 10 | A-log(base,strict) | 133 | 349 | 570196 | 42431 | 1616 | 15784 | 13 | 9.8 | 9.8 |
| 1000 | 10 | A-log(no-base) | 116 | 349 | 570908 | 42454 | 1228 | 15355 | 11 | 7.6 | 7.6 |
| 1000 | 10 | B-automerge | 193 | 195 | 22965 | 22993 | 2019 | 3816 | 2 | 127.8 | 127.8 |
| 1000 | 10 | C-loro | 538 | 544 | 89159 | 60961 | 986 | 1975 | 2 | 113.6 | 113.6 |
| 1000 | 100 | A-log(base,own-excl) | 142 | 349 | 561293 | 41761 | 20569 | 183388 | 186 | 12.3 | 12.3 |
| 1000 | 100 | A-log(base,strict) | 143 | 349 | 561301 | 41774 | 21440 | 187156 | 194 | 12.2 | 12.2 |
| 1000 | 100 | A-log(no-base) | 115 | 349 | 561821 | 41792 | 12730 | 165162 | 132 | 8.2 | 8.2 |
| 1000 | 100 | B-automerge | 181 | 189 | 23041 | 23069 | 15046 | 16588 | 2 | 124.0 | 124.0 |
| 1000 | 100 | C-loro | 456 | 526 | 87711 | 59899 | 6470 | 15131 | 2 | 88.1 | 88.1 |
| 10000 | 10 | A-log(base,own-excl) | 132 | 354 | 5585299 | 391282 | 1997 | 18837 | 14 | 253.8 | 25.4 |
| 10000 | 10 | A-log(base,strict) | 132 | 354 | 5585299 | 391282 | 1997 | 18852 | 14 | 249.9 | 25.0 |
| 10000 | 10 | A-log(no-base) | 117 | 354 | 5586639 | 391324 | 1402 | 17972 | 10 | 100.9 | 10.1 |
| 10000 | 10 | B-automerge | 189 | 189 | 157649 | 157717 | 5603 | 7851 | 2 | 2658.9 | 265.9 |
| 10000 | 10 | C-loro | 541 | 541 | 698719 | 517533 | 1245 | 2032 | 2 | 10224.8 | 1022.5 |
| 10000 | 100 | A-log(base,own-excl) | 133 | 354 | 5572609 | 389122 | 20095 | 189833 | 166 | 255.3 | 25.5 |
| 10000 | 100 | A-log(base,strict) | 133 | 354 | 5572611 | 389111 | 20330 | 193252 | 168 | 253.1 | 25.3 |
| 10000 | 100 | A-log(no-base) | 117 | 354 | 5573350 | 389222 | 12418 | 167835 | 117 | 102.1 | 10.2 |
| 10000 | 100 | B-automerge | 189 | 189 | 157731 | 157799 | 18219 | 41039 | 2 | 2359.1 | 235.9 |
| 10000 | 100 | C-loro | 535 | 539 | 697696 | 516382 | 7340 | 16044 | 2 | 9590.3 | 959.0 |
| 10000 | 1000 | A-log(base,own-excl) | 142 | 354 | 5452651 | 376599 | 197096 | 2748387 | 1740 | 400.5 | 40.1 |
| 10000 | 1000 | A-log(base,strict) | 144 | 354 | 5452706 | 376626 | 204526 | 3050097 | 1806 | 425.4 | 42.5 |
| 10000 | 1000 | A-log(no-base) | 117 | 354 | 5454437 | 376898 | 119910 | 1605990 | 1223 | 121.8 | 12.2 |
| 10000 | 1000 | B-automerge | 181 | 187 | 156792 | 156860 | 137173 | 94163 | 2 | 2119.8 | 212.0 |
| 10000 | 1000 | C-loro | 450 | 519 | 676222 | 499660 | 62725 | 156428 | 2 | 7300.6 | 730.1 |

### 3.3 Geri alma ve modele özgü sayaçlar

| işlem | pencere | model | undo denendi | uygulandı | reddedildi (çakışma) | yok/desteklenmiyor | başkasının işini ezdi | ek |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 100 | 10 | A-log(base,own-excl) | 5 | 3 | 2 | 0 | 0 | blind_overwrites=0, conflicts=10, log_entries=89, reconnect_final_catchup_changes_only_bytes=954, reconnect_final_catchup_log_query_bytes=1704, reconnect_final_catchup_snapshot_bytes=23857, validation_errors=6 |
| 100 | 10 | A-log(base,strict) | 5 | 3 | 2 | 0 | 0 | blind_overwrites=0, conflicts=10, log_entries=89, reconnect_final_catchup_changes_only_bytes=954, reconnect_final_catchup_log_query_bytes=1704, reconnect_final_catchup_snapshot_bytes=23857, validation_errors=6 |
| 100 | 10 | A-log(no-base) | 5 | 3 | 2 | 0 | 0 | blind_overwrites=1, log_entries=89, reconnect_final_catchup_changes_only_bytes=11665, reconnect_final_catchup_log_query_bytes=18800, reconnect_final_catchup_snapshot_bytes=23857, validation_errors=6 |
| 100 | 10 | B-automerge | 5 | 3 | 0 | 2 | 0 | relay_sync_bytes=2867, visible_multi_value_conflicts_in_series=0 |
| 100 | 10 | C-loro | 5 | 4 | 0 | 1 | 0 | relay_sync_bytes=1694 |
| 1000 | 10 | A-log(base,own-excl) | 32 | 20 | 12 | 0 | 0 | blind_overwrites=0, conflicts=104, log_entries=890, reconnect_final_catchup_changes_only_bytes=474, reconnect_final_catchup_log_query_bytes=767, reconnect_final_catchup_snapshot_bytes=44237, validation_errors=20 |
| 1000 | 10 | A-log(base,strict) | 32 | 20 | 12 | 0 | 0 | blind_overwrites=0, conflicts=104, log_entries=890, reconnect_final_catchup_changes_only_bytes=474, reconnect_final_catchup_log_query_bytes=767, reconnect_final_catchup_snapshot_bytes=44237, validation_errors=20 |
| 1000 | 10 | A-log(no-base) | 32 | 20 | 12 | 0 | 0 | blind_overwrites=18, log_entries=891, reconnect_final_catchup_changes_only_bytes=9053, reconnect_final_catchup_log_query_bytes=14805, reconnect_final_catchup_snapshot_bytes=44270, validation_errors=19 |
| 1000 | 10 | B-automerge | 32 | 26 | 2 | 4 | 0 | relay_sync_bytes=2314, visible_multi_value_conflicts_in_series=0 |
| 1000 | 10 | C-loro | 32 | 31 | 0 | 1 | 4 | relay_sync_bytes=974 |
| 1000 | 100 | A-log(base,own-excl) | 32 | 21 | 11 | 0 | 0 | blind_overwrites=0, conflicts=140, log_entries=881, reconnect_final_catchup_changes_only_bytes=262, reconnect_final_catchup_log_query_bytes=839, reconnect_final_catchup_snapshot_bytes=59315, validation_errors=35 |
| 1000 | 100 | A-log(base,strict) | 32 | 21 | 11 | 0 | 0 | blind_overwrites=0, conflicts=148, conflicts_with_own_entries=8, log_entries=881, reconnect_final_catchup_changes_only_bytes=262, reconnect_final_catchup_log_query_bytes=839, reconnect_final_catchup_snapshot_bytes=59315, validation_errors=35 |
| 1000 | 100 | A-log(no-base) | 32 | 21 | 11 | 0 | 0 | blind_overwrites=20, log_entries=881, reconnect_final_catchup_changes_only_bytes=1387, reconnect_final_catchup_log_query_bytes=2475, reconnect_final_catchup_snapshot_bytes=59326, validation_errors=35 |
| 1000 | 100 | B-automerge | 32 | 26 | 2 | 4 | 0 | relay_sync_bytes=15706, visible_multi_value_conflicts_in_series=3 |
| 1000 | 100 | C-loro | 32 | 31 | 0 | 1 | 4 | relay_sync_bytes=6458 |
| 10000 | 10 | A-log(base,own-excl) | 327 | 244 | 83 | 0 | 0 | blind_overwrites=0, conflicts=1001, log_entries=8882, reconnect_final_catchup_changes_only_bytes=2066, reconnect_final_catchup_log_query_bytes=3125, reconnect_final_catchup_snapshot_bytes=214898, validation_errors=106 |
| 10000 | 10 | A-log(base,strict) | 327 | 244 | 83 | 0 | 0 | blind_overwrites=0, conflicts=1001, log_entries=8882, reconnect_final_catchup_changes_only_bytes=2066, reconnect_final_catchup_log_query_bytes=3125, reconnect_final_catchup_snapshot_bytes=214898, validation_errors=106 |
| 10000 | 10 | A-log(no-base) | 327 | 244 | 83 | 0 | 0 | blind_overwrites=54, log_entries=8883, reconnect_final_catchup_changes_only_bytes=10282, reconnect_final_catchup_log_query_bytes=17459, reconnect_final_catchup_snapshot_bytes=214880, validation_errors=105 |
| 10000 | 10 | B-automerge | 327 | 299 | 7 | 21 | 0 | relay_sync_bytes=6063, visible_multi_value_conflicts_in_series=2 |
| 10000 | 10 | C-loro | 327 | 326 | 0 | 1 | 8 | relay_sync_bytes=1233 |
| 10000 | 100 | A-log(base,own-excl) | 327 | 248 | 79 | 0 | 0 | blind_overwrites=0, conflicts=1022, log_entries=8868, reconnect_final_catchup_changes_only_bytes=819, reconnect_final_catchup_log_query_bytes=1599, reconnect_final_catchup_snapshot_bytes=230771, validation_errors=116 |
| 10000 | 100 | A-log(base,strict) | 327 | 248 | 79 | 0 | 0 | blind_overwrites=0, conflicts=1024, conflicts_with_own_entries=2, log_entries=8868, reconnect_final_catchup_changes_only_bytes=819, reconnect_final_catchup_log_query_bytes=1599, reconnect_final_catchup_snapshot_bytes=230771, validation_errors=116 |
| 10000 | 100 | A-log(no-base) | 327 | 247 | 80 | 0 | 0 | blind_overwrites=63, log_entries=8868, reconnect_final_catchup_changes_only_bytes=2255, reconnect_final_catchup_log_query_bytes=3979, reconnect_final_catchup_snapshot_bytes=230757, validation_errors=116 |
| 10000 | 100 | B-automerge | 327 | 299 | 7 | 21 | 0 | relay_sync_bytes=18888, visible_multi_value_conflicts_in_series=2 |
| 10000 | 100 | C-loro | 327 | 326 | 0 | 1 | 8 | relay_sync_bytes=7328 |
| 10000 | 1000 | A-log(base,own-excl) | 327 | 256 | 71 | 0 | 0 | blind_overwrites=0, conflicts=1330, log_entries=8729, reconnect_final_catchup_changes_only_bytes=473, reconnect_final_catchup_log_query_bytes=764, reconnect_final_catchup_snapshot_bytes=379906, validation_errors=257 |
| 10000 | 1000 | A-log(base,strict) | 327 | 256 | 71 | 0 | 0 | blind_overwrites=0, conflicts=1396, conflicts_with_own_entries=66, log_entries=8729, reconnect_final_catchup_changes_only_bytes=473, reconnect_final_catchup_log_query_bytes=764, reconnect_final_catchup_snapshot_bytes=379906, validation_errors=257 |
| 10000 | 1000 | A-log(no-base) | 327 | 256 | 71 | 0 | 0 | blind_overwrites=113, log_entries=8733, reconnect_final_catchup_changes_only_bytes=1022, reconnect_final_catchup_log_query_bytes=1772, reconnect_final_catchup_snapshot_bytes=379441, validation_errors=256 |
| 10000 | 1000 | B-automerge | 327 | 302 | 6 | 19 | 0 | relay_sync_bytes=139670, visible_multi_value_conflicts_in_series=3 |
| 10000 | 1000 | C-loro | 327 | 326 | 0 | 1 | 7 | relay_sync_bytes=62713 |

### 3.4 Dönüşte çevrimdışı ajanın gördüğü (A modelleri)

- 100/10 A-log(base,own-excl): kuyruk=10, uygulandı=7, hata sonrası uygulandı=3, reddedildi=0, kodlar={'Conflict': 3}
- 100/10 A-log(base,strict): kuyruk=10, uygulandı=7, hata sonrası uygulandı=3, reddedildi=0, kodlar={'Conflict': 3}
- 100/10 A-log(no-base): kuyruk=10, uygulandı=10, hata sonrası uygulandı=0, reddedildi=0, kodlar={}
- 1000/10 A-log(base,own-excl): kuyruk=10, uygulandı=8, hata sonrası uygulandı=2, reddedildi=0, kodlar={'Conflict': 2}
- 1000/10 A-log(base,strict): kuyruk=10, uygulandı=8, hata sonrası uygulandı=2, reddedildi=0, kodlar={'Conflict': 2}
- 1000/10 A-log(no-base): kuyruk=10, uygulandı=10, hata sonrası uygulandı=0, reddedildi=0, kodlar={}
- 1000/100 A-log(base,own-excl): kuyruk=97, uygulandı=26, hata sonrası uygulandı=54, reddedildi=17, kodlar={'Conflict': 54, 'Duplicate': 2, 'InUse': 2, 'Invalid': 22, 'NotFound': 8}
- 1000/100 A-log(base,strict): kuyruk=97, uygulandı=18, hata sonrası uygulandı=62, reddedildi=17, kodlar={'Conflict': 62, 'Duplicate': 2, 'InUse': 2, 'Invalid': 22, 'NotFound': 8}
- 1000/100 A-log(no-base): kuyruk=97, uygulandı=80, hata sonrası uygulandı=0, reddedildi=17, kodlar={'Duplicate': 2, 'InUse': 2, 'Invalid': 22, 'NotFound': 8}
- 10000/10 A-log(base,own-excl): kuyruk=9, uygulandı=5, hata sonrası uygulandı=4, reddedildi=0, kodlar={'Conflict': 4}
- 10000/10 A-log(base,strict): kuyruk=9, uygulandı=5, hata sonrası uygulandı=4, reddedildi=0, kodlar={'Conflict': 4}
- 10000/10 A-log(no-base): kuyruk=9, uygulandı=9, hata sonrası uygulandı=0, reddedildi=0, kodlar={}
- 10000/100 A-log(base,own-excl): kuyruk=92, uygulandı=31, hata sonrası uygulandı=49, reddedildi=12, kodlar={'Conflict': 49, 'InUse': 2, 'Invalid': 8, 'NotFound': 14}
- 10000/100 A-log(base,strict): kuyruk=92, uygulandı=29, hata sonrası uygulandı=51, reddedildi=12, kodlar={'Conflict': 51, 'InUse': 2, 'Invalid': 8, 'NotFound': 14}
- 10000/100 A-log(no-base): kuyruk=92, uygulandı=80, hata sonrası uygulandı=0, reddedildi=12, kodlar={'InUse': 2, 'Invalid': 8, 'NotFound': 14}
- 10000/1000 A-log(base,own-excl): kuyruk=892, uygulandı=213, hata sonrası uygulandı=511, reddedildi=168, kodlar={'Conflict': 514, 'InUse': 6, 'Invalid': 206, 'NotFound': 121}
- 10000/1000 A-log(base,strict): kuyruk=892, uygulandı=147, hata sonrası uygulandı=577, reddedildi=168, kodlar={'Conflict': 580, 'InUse': 6, 'Invalid': 206, 'NotFound': 121}
- 10000/1000 A-log(no-base): kuyruk=892, uygulandı=727, hata sonrası uygulandı=0, reddedildi=165, kodlar={'InUse': 6, 'Invalid': 206, 'NotFound': 118}

### 3.5 Çakışma dökümü (A-log(base,own-excl), 10 000/100)

"gönderilen işlem vs çakıştığı girdi(ler)", en sık 10 (toplam 1 022 çakışma; tam liste `runs.json` → `extra`):

- `add_layer vs add_layer`: 144
- `add_layer vs remove_layer`: 95
- `add_study vs add_layer`: 81
- `remove_layer vs add_layer`: 74
- `add_layer vs add_study`: 54
- `add_study vs remove_layer`: 53
- `add_layer vs remove`: 51
- `remove vs add_layer`: 48
- `set_params vs set_params`: 48
- `remove_layer vs remove_layer`: 36

Sıralamaya göre 1 022 çakışmanın 964'ü (%94) katman listesine dokunan işlemler arasında (`add_layer`, `remove_layer`, `add_study`+`show`, katmanlarıyla birlikte silinen seri): §8.2 katman listesinin tamamını tek kimlik (`view:main`) saydığı için **farklı katmanlara** dokunan iki yazı da çakışır. Gerçek alan çakışması (`set_params vs set_params`, `update_finding vs update_finding`) yalnızca 54 (%5). Bütün boyutlarda bu oran %80–96.

## 4. Sonuçlar — adlandırılmış çakışma durumları

`src/cases.rs`, ham çıktı `raw/t6a/cases.json`. Her durum dört modelde aynı adımlarla
koşuldu. "Yazar" sütunu modelin **kendi** köken mekanizmasından okunur (A: günlük; B: kazanan
op'un aktörü, `|` eşzamanlı saklanan değerleri gösterir; C: `get_last_editor`).

| Durum | A-log(base) | A-log(no-base) | B Automerge | C Loro |
|---|---|---|---|---|
| **C1** editor çevrimdışı MACD `fast=8`; arkin `fast=10, slow=30` | editor dönüşte `conflict` alır, okur, bilerek yeniden gönderir → `fast=8, slow=30`; yazar: fast←editor@2, slow←arkin@1 | sessizce `fast=8` (günlükte kör üzerine yazma olarak görülür) | sessizce `fast=8, slow=30`; kimseye bir şey söylenmez; fast'ta `arkin\|editor` iki değer saklı | sessizce `fast=8, slow=30`; fast←editor |
| **C2** aynı turda arkin 1. katmanı (bb20) siler, scout 2. indekse (rsi14'ün önüne) macd ekler | `conflict` → çapa ile yeniden çözüm → doğru yer (rsi14'ün önü) | **kayar**: katman rsi14'ün **arkasına** düşer | doğru yer (RGA liste niyeti korur) | doğru yer |
| **C3** aynı turda arkin `fnd-overbought`'u reddeder, scout iddiasını günceller | scout `invalid` alır (karar verilmiş bulgu donmuş); karne kaydı ile iddia tutarlı; anomali 0 | aynı | ikisi de uygulanır: kapsül `rejected`, ama iddia **reddedildikten sonra** değişmiş; karne eski iddiayı gösteriyor → anomali 1 (bayat karar) | aynı, anomali 1 |
| **C3b** ters sıra (önce güncelleme, sonra ret) | arkin `conflict` alır, yeni iddiayı okur, onu reddeder; karne = yeni iddia | arkin yeni iddiayı görmeden reddeder ama otorite sırası tutarlı; karne = yeni iddia | arkin eski iddiayı reddetti, karne eski iddia, bulgu yeni → anomali 1 | anomali 1 |
| **C4** editor çevrimdışı `sma20` (period 50, gösterilen); arkin `sma20` (period 20, gösterilen) | editor dönüşte `duplicate` alır; tek seri, tek katman, arkin'in period 20'si | aynı | sessizce **editor'ün** seri nesnesi kazanır (period 50), **iki katman** aynı `sma20`'yi çizer; arkin'in "20" yazısı görünmez olur | mergeable harita alanları birleştirir: period 50 (LWW), iki katman |
| **C5** rsi7 var; editor çevrimdışı rsi7'ye katman + niyet ekler; arkin rsi7'yi siler | iki yazı da dönüşte `not_found` ile açıkça reddedilir; anomali 0 | aynı | ikisi de sessizce girer: **yok seriyi çizen katman** + **yok seriye bakan niyet** → anomali 2 | anomali 2 |
| **C6** arkin `fast=10`; scout (görerek) `fast=14`; arkin undo | `undo_conflict` (scout'un sonraki işi korunur), 14 | aynı | uygulama-undo'su reddeder (kazanan op scout'un), 14 | **UndoManager scout'un 14'ünü ezer → 12**; kimseye söylenmez |
| **C6b** arkin `fast=10`; scout `slow=30` (aynı öğe, başka alan); arkin undo | `undo_conflict` — **yanlış pozitif**: alanlar ayrı ama öğe düzeyi denetim engeller | aynı | doğru: fast=12, slow=30 | doğru: fast=12, slow=30 |
| **C6c** arkin `fast=10`; scout `rsi14.period=7`; arkin undo | doğru | doğru | doğru | doğru |
| **C6d** arkin `fast=10`; editor **çevrimdışı** `fast=14`; arkin undo; editor döner | arkin'in undo'su uygulanır (12); editor dönüşte `conflict` alır, okur, 14'ü bilerek yazar | editor'ün 14'ü sessizce yazılır | sessizce **12** (arkin'in undo'su LWW'de kazanır), editor'ün 14'ü kaybolur; `editor\|arkin` iki değer saklı | sessizce 12 |

## 5. Kod karmaşıklığı ve kavram sayısı

Boş ve yorum satırı hariç, rustfmt sonrası (`src/`):

| Parça | A otorite+günlük | B Automerge | C Loro |
|---|---:|---:|---:|
| İşlem açılımı / yazma (doğrulama dahil) | 556 (motor: doğrulama, açılım, inverse, `ChangeOp` uygulayıcı) | 167 | 141 |
| Otorite (sıra, `base`, undo, günlük) | 194 | — | — |
| İstemci: Op çevirisi + kimlikle yeniden çözme | 135 | — | — |
| Undo | (otoritede, yukarıda) | 68 (+ yazma içinde kayıt) | 0 (yerleşik) + 28 ezme muhasebesi |
| Belgeye çevirme / ilk belge | 82 | 84 + 72 | 96 + 81 |
| Model yapıştırıcısı (tel, yayma, dönüş, ölçüm) | 431 | 278 | 214 |
| **Toplam dosya** | **1 398** | **669** | **555** |
| Ortak yerel doğrulama (CRDT istemcilerinin de ihtiyacı) | (motorda) | +150 | +150 |

A'nın kodu ~2–2,5 kat uzun; çünkü kütüphanenin yaptığını (değişim uygulama, geri alma,
çoğaltma) elle yazıyor ve doğrulamayı otoritede **yetkili** olarak yapıyor. Öte yandan bu
kod tek yerde (Rust çekirdeğinde) yaşar; K21-4 ve "kabuklar incedir" gereği istemciler
onu WASM/FFI üzerinden kullanır, dil başına yeniden yazılmaz.

İstemcinin anlaması gereken kavramlar:

- **A (7):** kimlik/`hello`; `command` (ops, `mode`, `base`); `ack`/`error` ve kodlar
  (özellikle `conflict`, `undo_conflict`); `seq`; `change` bildirimi (`ChangeOp` uygulama);
  `log`/`document` ile yetişme; `undo`.
- **B (≥10):** aktör kimliği; nesne ağacı ve `ObjId`; skaler tipler; işlem/commit;
  değişim, hash, `heads`; sync durumu ve mesajları; çok değerli çakışmalar (`get_all`);
  uygulama düzeyinde değişmez denetimi; uygulama düzeyinde undo; uygulama düzeyinde köken
  alanları (onay).
- **C (≥10):** peer kimliği; konteyner tipleri; mergeable ve normal çocuk farkı (silip
  yeniden yaratınca eski durumun geri gelmesi dahil); commit; sürüm vektörü/frontiers;
  export kipleri (updates, snapshot, state-only); import; `UndoManager` (yalnızca yerel,
  ezebilir); `get_last_editor`; uygulama düzeyinde değişmez denetimi ve onay alanları.

## 6. Köken, geri alma, determinizm, çevrimdışı deneyim

**Köken.** Üç model de "bu değeri kim yazdı?" sorusunu cevaplayabildi (§4 tablosu).
Farklar:
- A'da yazar **ve onaylayan** otorite tarafından damgalanır; istemci köken yazamaz
  (`Op`'larda köken alanı yok, `deny_unknown_fields`). Günlük isteği, değişimi ve tersini
  taşır; "neden bu değer?" sorusunun cevabı (`cause`, `reason`, sıra) oradadır. Bedeli:
  geçmişli belge en büyük (10 000 işlemde 5,6 MB JSON, gzip 391 KB).
- B'de yazan aktör op kimliğinden kesin okunur; eşzamanlı değerler bile saklıdır
  (`arkin|editor`). Ama "onaylayan" kavramı yoktur: onay ancak uygulamanın yazdığı bir
  alandır ve **herhangi bir replika** o alana `approved_by: arkin` yazabilir; kimse
  reddedemez. K21-5'teki öneri/onay akışında kabul edilen yazının aktörü onaylayan olur,
  asıl öneren yalnızca uygulama alanında kalır.
- C'de `get_last_editor` yalnızca son yazanı verir; eşzamanlı kaybedeni göstermez.
  Onay için B ile aynı sınır.

**Geri alma.** "arkin yalnızca kendi son değişikliğini, başkalarının sonraki işini
ezmeden geri alabilir mi?"
- A: evet; başkasının sonraki işi aynı öğeye dokunduysa `undo_conflict` (C6). Ezme 0.
  Ama öğe düzeyi denetim yanlış pozitif verir (C6b): iş yükünde 327 undo'nun 71–83'ü
  reddedildi; bunların bir kısmı aynı öğenin başka alanı ya da (katmanlarda) aynı
  görünümün başka katmanı yüzünden.
- B (uygulama undo'su): ezme 0, yanlış pozitif daha az (alan düzeyi; 327'de 6–7 ret), ama
  19–21 undo "desteklenmiyor" (seri silmenin geri alınması yazılmadı — yazmak, silinen
  nesneyi alt nesneleriyle yeniden kurmayı gerektirir) ve bu kodu biz yazdık.
- C (yerleşik `UndoManager`): neredeyse her undo uygulanır (326/327), ama başkasının daha
  sonra yazdığı değeri **ezdiği** durumlar var: 1 000 işlemde 4, 10 000 işlemde 7–8 (C6'da
  somut). Loro'nun undo'su uzak değişikliklere göre dönüştürülür ama aynı yazmaca sonradan
  yazılmış değeri korumaz; bir çakışma da bildirmez.

**Determinizm.** Her yapılandırmada tekrarlar arasında geçmiş baytlarının ve son durumun
SHA-256'sı aynıydı (koşu içi `assert`). Ayrıca ayrı bir süreçte 100/1 000 işlemlik
yapılandırmalar yeniden koşuldu: 15/15 aynı bayt (`raw/t6a/runs-repeat.json`), durumlar
dosyası (`cases.json`) bayt bayt aynı. Koşul: CRDT'lerde aktör/peer kimlikleri sabit,
Automerge commit zamanı verilmez (varsayılan 0), Loro zaman damgası kaydetmez (varsayılan
kapalı). Yani üçü de bu kullanımda deterministik; A'nınki tasarımdan (BTreeMap, sıra),
CRDT'lerinki ayar disiplininden gelir (gerçek kullanımda rastgele aktör kimliği ve zaman
damgası varsayılanları bunu bozar).

**Çevrimdışı ajanın dönüşü.**
- A(base): açık. 1 000 işlemlik pencerede kuyruktaki 892 yazının 213'ü doğrudan, 511'i
  `conflict` sonrası yeniden okuyup göndererek uygulandı, 168'i nedeni bildirilerek
  reddedildi (`invalid` 206 kez, `not_found` 121, `in_use` 6). Bedeli tur sayısı (1 740
  istek/cevap) ve bayt (aşağı 2,7 MB, çünkü her çakışmada `log` sorgusuyla yetişiyor).
  `strict` değişkede bunlardan 66'sı ajanın **kendi** önceki kuyruk girdisiyle çakışması
  (§8.2'nin yazıldığı haliyle gereksiz tur).
- A(no-base): 727 doğrudan, 165 ret; ama 113 yazı başkasının görmediği işini sessizce
  ezdi (günlükte izlenebilir, yazara söylenmez).
- B/C: hiçbir hata yok; 2 mesaj turunda her şey birleşir (B: 137 KB yukarı / 94 KB aşağı;
  C: 63 KB / 156 KB). Sonuçta 33–50 anlamsal anomali ve 94 "karar ∥ güncelleme" çakışması
  sessizce belgeye girer; ajan yazısının ezilip ezilmediğini ancak geçmişi ayrıca
  inceleyerek öğrenebilir.

**Süre.** A işlem başına 8–42 µs (JSON kodlama/çözme ve üç abone dahil); `base`li
değişkede yeniden denemeler ve yetişme maliyetiyle 10 000/1 000'de 40 µs. B 110–270 µs,
C 50 µs'den başlayıp 10 000 işlemde ~1 ms'ye çıkıyor (`UndoManager` kapalı yan ölçümde
10 000/10: 6,4 s yerine 10,2 s → payı ~%35; kalan büyümenin nedeni incelenmedi, ham:
`raw/t6a/stdout-loro-no-undo.txt`, `runs-loro-no-undo.json`; o koşuda arkin'in undo'ları
boşa düştüğü için iş yükü ve anomali sayıları ana koşudan farklıdır, yalnızca süre için kullanıldı). Hepsi etkileşim için yeterli; süre bir karar ölçütü
değil.

**Boyut.** Tel: A'nın komutu yazı başına 115–145 B (B: ~185 B, C: ~450–540 B); ama A'nın
bildirimi tam öğe taşıdığı için abone başına ~350 B (B ~190 B). Tam geçmiş: A en büyük
(10 000 işlemde 5,6 MB; gzip 391 KB), B en küçük (158 KB, kendisi sıkıştırılmış), C 700 KB
(gzip 517 KB). Dönüşte A'nın yetişmesi `log` sorgusuyla ağır (girdi = istek + değişim +
ters); yalnızca `change` bildirimleri seçilirse yarıya iner (`extra.reconnect_final_catchup_*`).

## 7. Hipotezin denetimi

| # | Hipotez | Sonuç |
|---|---|---|
| 1 | Hepsi yakınsar; fark anlamsal geçerlilikte; A'da 0, CRDT'de onlarca–yüzlerce anomali | **Kısmen doğru.** Hepsi yakınsadı (30/30), A'da anomali her koşuda 0. CRDT'de anomali var ama sayı tahminden **küçük**: 10 000 işlemde 3–13 (pencere 10–100), 33–50 (pencere 1 000). Sayı toplam işlemle değil **çevrimdışı pencereyle** büyüyor; yerel doğrulama çevrimiçi durumdaki anomalilerin çoğunu önlüyor. Anomali sınıfları: bayat karar (en sık), yok seriye işaret eden koşul/niyet, yok seriyi çizen katman. |
| 2 | Aynı kimlikle `add_study`: A `duplicate`; CRDT'de biri sessizce kazanır ya da alanlar karışır | **Doğru** (C4). Automerge: bir nesne kazanır, öteki görünmez. Loro (mergeable): alanlar birleşir. Her ikisinde de iki katman aynı seriyi çizer ve bir yazarın parametresi sessizce kaybolur. Rastgele iş yükünde yerel doğrulama yüzünden nadir (0–1). |
| 3 | Katman indeksinde CRDT daha iyi | **Doğru** (C2). `base`siz A kayar; `base`li A ancak bir `conflict` turu ve istemci tarafı çapa çözümüyle doğru yere koyar. Ek bulgu: §8.2'nin görünüm düzeyindeki dokunma kimliği çakışmaların **%80–96**'sını üretiyor (çoğu yanlış pozitif). |
| 4 | Tel boyutu ve geçmiş: CRDT daha küçük, A JSON'u 3–10× büyük | **Kısmen.** Geçmişte doğru ve tahminden de büyük fark: A/B ≈ 35× ham, 2,5× gzip (10 000 işlem). Loro'nun snapshot'ı Automerge'ün 4,4 katı. Tel: A'nın **komutu** CRDT değişiminden küçük (115–145 B vs 185/450+ B); A'nın **bildirimi** daha büyük (~350 B). Dönüş senkronu CRDT'de küçük ve 2 turda biter; A'da çakışma başına tur ve `log` yetişmesi pahalı. |
| 5 | A µs mertebesinde, CRDT daha yavaş, hepsi yeterli | **Doğru.** A 8–43 µs/işlem, B 110–270 µs, C 50 µs–1 ms. |
| 6 | A daha çok satır, ama daha az kavram | **Doğru.** 1 398 vs 669/555 satır (CRDT'lere 150 satırlık ortak yerel doğrulama eklenince bile ~1,7–1,9×); kavram 7 vs ≥10. |
| 7 | A'da köken+onay tam; Automerge'de onay yok, undo yok; Loro undo iyi ama sessizce ezebilir | **Doğru.** Loro undo'su iş yükünde 7–8 kez başkasının sonraki değerini ezdi; A hiç ezmedi ama öğe düzeyi denetim yüzünden yanlış pozitif ret veriyor (C6b). Automerge'ün alan düzeyi uygulama undo'su bu konuda en iyisiydi — ama yazması bize düştü ve kapsamı eksik. |
| 8 | Hepsi deterministik (CRDT'de zaman damgası kapatılırsa) | **Doğru.** Koşu içi ve süreçler arası bayt eşitliği. Automerge ve Loro'da varsayılan zaman damgası zaten kapalı/0; aktör/peer kimliği sabitlenmeli. |
| 9 | A'da çevrimdışı ajan açık hata görür, CRDT'de sessiz birleşme | **Doğru.** Ayrıca `strict` §8.2 ajanın kendi kuyruk girdileriyle gereksiz çakışma üretiyor (10 000/1 000'de 66). |

## 8. Dürüst sonuç

**CRDT'nin açıkça daha iyi olduğu yerler:**
1. **Liste niyeti** (katman ekle/sil eşzamanlı): RGA tabanlı listeler hiçbir tur harcamadan
   doğru yeri buluyor; A ya kayıyor (`base`siz) ya da bir hata turu ödüyor.
2. **İnce taneli birleşme:** aynı öğenin farklı alanları (MACD `fast` ve `slow`) çakışmaz;
   A'nın öğe/görünüm düzeyi dokunma kimlikleri çakışmaların büyük kısmını yanlış pozitif
   yapıyor (iş yükünde yazıların %9–11'i en az bir gereksiz tur ödüyor).
3. **Çevrimdışı dönüş maliyeti:** 2 tur, onlarca–yüz KB; A'da 1 000 işlemlik pencere 1 740
   istek/cevap ve megabaytlarca yetişme demek.
4. **Geçmiş boyutu:** Automerge tam geçmişi A'nın gzip'li günlüğünden bile 2,5 kat küçük
   tutuyor.
5. **Kod miktarı:** çoğaltma, değişim uygulama ve (Loro'da) undo hazır geliyor.

**Otorite + günlüğün açıkça daha iyi olduğu yerler (ve bu ürün için belirleyici olanlar):**
1. **Anlamsal geçerlilik:** 30 koşunun hepsinde anomali 0. CRDT'de, istemciler yerelde
   aynı doğrulamayı yapsa bile, kararlar bayatlıyor (insan bir iddiayı reddediyor, iddia
   arkasından değişiyor), koşullar ve katmanlar silinmiş serilere işaret ediyor, aynı
   kimlikli iki varlık sessizce tek nesneye iniyor. NORTH_STAR'ın kapsül/karne/denetim izi
   vaadi tam olarak bu değişmezlere dayanıyor; bunlar CRDT'de **okuma anında düzeltme**
   ile kısmen gizlenebilir (yok seriyi çizen katmanı çizmemek), ama "reddedilen iddia
   değişti" ya da "insan 20 dedi, ajanın 50'si kazandı" gibi durumlar yapısal olarak
   çözülemez — birinin **söylenmesi** gerekir.
2. **Açık geri bildirim:** ajan ve insan, yazısının kabul edilmediğini ve nedenini
   (`conflict`, `invalid`, `not_found`, `duplicate`) o anda öğreniyor. CRDT'de hiçbir yazar
   hiçbir şey öğrenmedi; 10 000/1 000'de ~10 800–11 300 eşzamanlı aynı-alan çiftinin
   kaybedeni sessizce kayboldu.
3. **Onay ve köken:** onaylayan otoritece damgalanıyor ve sahtelenemiyor; CRDT'de onay
   herkesin yazabildiği bir uygulama alanı.
4. **Undo güvenliği:** başkasının sonraki işini hiç ezmedi; Loro'nun yerleşik undo'su ezdi.
5. **Kavram sayısı ve protokolün sadeliği:** istemci (insan arayüzü ya da dil modeli ajanı)
   yalnızca komut/ack/hata/değişim bilir; bu, K21-4'teki "arayüz ve ajan aynı protokolü
   kullanır" ilkesi için önemli — bir dil modeli ajanına CRDT çakışma semantiği
   öğretmektense `conflict` hatasını okuyup yeniden denemesini söylemek çok daha kolay.

## 9. Öneri (K21-2 için)

**K21-2 korunsun:** belge başına tek otorite ve sıralı işlem günlüğü; her yazı kökeniyle.
Kanıt: anlamsal anomali 0'a karşı CRDT'de pencereyle büyüyen sessiz anomali; açık hata
geri bildirimi; sahtelenemez onay; ezmeyen undo. CRDT'nin kazandığı yerler kararı
değiştirecek türden değil, **otorite modeline taşınabilecek** türden. Bu yüzden karar
şu iyileştirmelerle birlikte kapatılmalı (hepsi bu deneyden doğrudan çıkıyor):

1. **Katmanlara kimlik ver, dokunma kimliğini katman düzeyine indir.** `view:main` yerine
   `layer:<id>`; `add_layer`/`remove_layer` indeks yerine (ya da indeksle birlikte) çapa
   kimliği (`before`/`after`) alsın. Ölçülen çakışmaların %80–96'sı ve C2'deki kayma bundan.
   (Kapsam dışı; PROTOCOL.md ve `Layer` şeması değişir → öneri.)
2. **`base` denetimini alan düzeyine indir** (en azından `set_params` parametreleri ve
   bulgu alanları için): C6b'deki yanlış pozitif undo reddi ve aynı öğenin farklı
   alanlarına yazan ajanların gereksiz turu ortadan kalkar. Girdi zaten isteği taşıdığı
   için ek veri gerekmez.
3. **§8.2'de gönderenin kendi girdileri çakışma sayılmasın** (ya da istemci her kuyruk
   girdisinde `base`'i kendi ack'leriyle ilerletebilsin). Ölçüm: `strict` 10 000/1 000'de
   66 gereksiz çakışma ve tur; `own-excl` ile 0.
4. **Yetişme için `log` yerine değişim akışı ya da anlık görüntü:** dönüşte istemciye
   `inverse`/`request` taşıyan tam girdiler yerine yalnızca `change` bildirimleri
   (yaklaşık yarı boyut) ya da eşiğin üstünde `document` gönderilsin; §4.3 zaten ikisini
   anıyor, otoritenin seçmesi tanımlansın.
5. **Çakışmada ajan politikası:** ajanların insanın yazdığı bir öğede `conflict` aldıklarında
   körü körüne yeniden göndermek yerine öneriye dönmeleri (K21-5) protokolde tavsiye
   edilsin; deneydeki "oku-yeniden gönder" politikası C1'de insanın `fast=10`'unu bilerek
   de olsa yine ezdi.
6. **Hibrit, yalnızca serbest metin için ve gerekirse:** bulgu iddiası, not, anlatım
   metni gibi alanlarda eşzamanlı metin düzenlemesi gerçek bir ihtiyaç olursa, o alanın
   değeri bir CRDT metni (ör. Loro/Automerge text) olarak otorite girdisinin içinde
   taşınabilir; yapı, kimlik, karar ve karne otoritede kalır. Bu deneyde ölçülmedi; şu an
   ihtiyaç görülmüyor.

## 10. Sınırlar

- Öneri kipi (K21-5) modellenmedi; ajanlar doğrudan yazdı. Öneri kipiyle A'nın çakışma
  sayısı ve CRDT'nin anomali türleri değişir (kabul anında açılım yeni hatalar doğurabilir).
- Tur modeli kaba bir gecikme modeli; gerçek ağ sırası ve gecikme dağılımı yok.
- CRDT'ler için "okuma anında düzeltme" (yok seriyi çizen katmanı çizmemek vb.) kodlanmadı;
  bu, anomalilerin bir kısmını (dangling_layer/dangling_ref) kullanıcıdan gizleyebilirdi
  ama bayat karar ve sessiz kayıp sınıflarını gideremez.
- Automerge'ün uygulama düzeyi undo'su seri silmeyi geri almıyor (19–21 "desteklenmiyor").
- Loro'nun 10 000 işlemde işlem başı maliyetinin büyümesinin nedeni araştırılmadı; commit
  başına `export(updates)` kullanım biçiminden kaynaklanıyor olabilir.
- Süre ölçümleri yalnızca çoğaltma işini kapsar; her yazıdan önce görünümün yeniden
  kurulması (iş yükü sürücüsü) ölçüme dahil değildir.

## 11. Öneriler (kapsam dışı, yapılmadı)

- PROTOCOL.md §8.2: dokunma kimliği katman ve alan düzeyine; gönderenin kendi girdilerini
  hariç tutma; `Layer`'a kimlik (yukarıdaki 1–3).
- PROTOCOL.md §4.3/§8.2: dönüşte yetişme biçiminin (değişim akışı / anlık görüntü / `log`)
  otorite tarafından seçilmesi ve eşiklerin tanımı.
- `qp-session` (paralel yapılıyor): bu deneydeki iki değişmez denetimi — yeniden oynatma =
  otorite belgesi (bayt), yansılar = otorite (bayt) — uygunluk testlerine alınabilir.
