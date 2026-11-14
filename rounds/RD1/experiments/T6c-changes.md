# T6c — Değişimlerin telde ve günlükte ifadesi

> **Yeniden üretim (denetim O1):** bu ölçüm `ade29fd` (dal `rd1/t6c-exp`) üzerinde yapıldı. Deney crate'i, ölçümden sonra değişen protokol tiplerine (`now`, `ack.undoes`, boş yapı varyantları, `put_*.index`) taşınmadığı için güncel `main`'de derlenmez; yeniden üretmek için o commit kullanılır: `git checkout ade29fd` ve rapordaki yöntem.

Soru: dokümandaki bir değişim telde (`notify` / `event.kind = "change"`) ve günlükte
(`LogEntry`) nasıl ifade edilmeli?

- **A. Genel doküman farkı** — iki doküman durumu arasındaki RFC 6902 JSON Patch
  (gerçek kütüphane: `json-patch` 5.0.0), ve RFC 7396 Merge Patch.
- **B. Anlamsal işlemler** — protokolün `ChangeOp`'u (ilkel ama alan tipli) ve/veya
  üst düzey `Op`'un kendisi günlük birimi olarak.
- **C. Karma** — anlamsal işlem adı + JSON Patch yükü.

## 1. Hipotez (ölçümden önce yazıldı)

H1 — **Boyut.** Tek öğelik değişimlerde (parametre, durum, aralık) A'nın elle üretilmiş
(emit) yamaları `ChangeOp`'tan küçük ya da ona yakın olacak; çünkü `ChangeOp` `put_*`
ile tam öğe taşır, JSON Patch ise yalnızca değişen alanı. Ama dizi ortasından silme
(katman, seri) gibi işlemlerde `json-patch`'in `diff` çıktısı konumsal olduğu için
kayan her öğe için `replace` üretecek ve `ChangeOp`'tan belirgin şekilde büyük olacak.
Merge Patch, dizileri bütün olarak değiştirdiği için bar eklemede ve katman işlemlerinde
en büyük olacak (10 000 barda bar eklemede yüz KB mertebesi).

H2 — **Hesap maliyeti.** Fark almak (`to_value` + `diff`) belge boyutuyla doğrusal; 10 000
barda tek canlı bar güncellemesi için milisaniye mertebesinde olacak, `ChangeOp` üretmek
mikrosaniyenin altında. 60 Hz sıcak yolda fark alma bütçenin (16,7 ms) belirgin bir
kısmını yiyecek ama tek başına imkânsız olmayacak.

H3 — **Uygulama maliyeti.** İstemci yansısında `json_patch::patch` ile `ChangeOp`
uygulaması küçük değişimlerde aynı mertebede olacak; fark, istemcinin `Value` mi yoksa
tipli `Document` mi tuttuğundan gelecek.

H4 — **Eşzamanlılık.** Dizi indeksli JSON Patch, iki eşzamanlı katman silmede ve dizi
kaymasında sessizce yanlış öğeyi değiştirecek. `test` işlemiyle korunmuş JSON Patch
yanlışı reddetmeye çevirecek. Ama protokolün kendi `remove_layer {view, index}` işlemi
de indeksli olduğu için `base` verilmezse **o da** yanlış sonuç verecek; anlamsal
yaklaşımın üstünlüğü indeks yerine kimlik kullanıldığı yerde (seri silme, `reject_finding`,
`set_params`) ortaya çıkacak.

H5 — **Geri alma.** RFC 6902 işlemleri önceki değeri taşımadığı için tersleri yalnızca
önceki durum elde varken türetilebilir; eski bir girdinin ters yaması, araya giren dizi
kaymalarından sonra yanlış yere uygulanacak. `ChangeOp` tersleri kimlikle çalıştığı için
seri/bulgu düzeyinde doğru kalacak; katman düzeyinde (`delete_layer {index}`) aynı kırılganlığı
taşıyacak ve ancak `undo_conflict` ile güvenli olacak.

H6 — **Niyet ve denetim.** "reject finding" gibi bir işlem JSON Patch'te iki-üç anonim
`replace`/`add` olarak görünecek; okuyan insan ya da ajan ne olduğunu ancak önceki
durumu açarak anlayabilecek. `Op` en okunur, `ChangeOp` ikinci, A en az okunur olacak.

H7 — **Doğrulama.** A'da otorite alan kurallarını (katman var olan seriye mi bakıyor,
seri kullanımda mı, barlar artan mı, kapsül durum geçişi geçerli mi) ancak yamayı
uygulayıp sonuç belgeyi baştan doğrulayarak kontrol edebilecek: maliyet belge boyutuyla
doğrusal, hata mesajı isteğe değil sonuca işaret eder ve geçiş kuralları (önce/sonra)
zahmetli olur. B'de her işlem kendi kuralını O(1)–O(öğe) maliyetle kontrol eder.

H8 — **Yeniden oynatma.** İki form da deterministik olacak; ancak `Op` günlüğü yeniden
oynatma için açılım kodunun **aynı sürümünü** gerektirecek (varsayılan parametre ya da
panel kuralı değişirse sonuç kayar), `ChangeOp` ve JSON Patch gerektirmeyecek.
`serde_json::Value` anahtarları alfabetik sıraladığı için JSON Patch yansısının baytları,
tipli dokümana geri çevrilmeden otoriteyle aynı olmayacak.

H9 — **Ajan yazabilirliği.** Ajan `Op`'u en kolay doğru üretir: kısa, kimlikle, durum
bilgisi istemez. JSON Patch yazmak indeks, güncel uzunluk, zaman damgası ve köken
bilgisini bilmeyi gerektirir.

Beklenen öneri: **telde ve günlükte `ChangeOp` (B), istekte `Op`; JSON Patch yalnızca
içe/dışa aktarma ve hata ayıklama aracı.** Katman işlemlerinde indeks yerine kararlı
kimlik eksikliği B'nin zayıf noktası olarak raporlanacak.

## 2. Yöntem

Deneme kodu: `experiments/t6c-changes/` (çalışma alanı üyesi değil; `qp-schema`'ya yol
bağımlılığı, `json-patch` 5.0.0, `flate2` 1.1.11, `jsonschema` 0.61.2). Ham çıktılar:
`rounds/RD1/experiments/raw/t6c/`. Makine: Apple M4 Max, macOS 27.0.1, rustc 1.92.0,
`--release`.

```
cargo build --release --manifest-path experiments/t6c-changes/Cargo.toml
target/release/t6c-changes fixtures/protocol/docs/thyao.json rounds/RD1/experiments/raw/t6c
```

**Otorite.** Gerçek otorite paralel yazıldığı için deneyde küçük bir otorite yazıldı
(`src/model.rs`): §5.1'deki işlemlerin çoğunu (`add_study`+`show`, `set_params`,
`add_layer`, `remove_layer`, `set_view_range`, `add_fact`, `add_finding`(+kapsül),
`update_finding`, `approve/reject_finding`, `add_intent`, `remove` (seri silinince
katmanları da), `append_bars`, `update_last_bar`, `undo`) alan kurallarıyla `ChangeOp`'a
açar; `ChangeOp`'u uygular, tersini türetir, dokunulan kimlikleri tutar ve `undo`'yu
`undo_conflict` ile uygular (§8.3). Ayrıca `feed` rolü yalnızca bar yazabilir.

**İş yükü.** Başlangıç dokümanı `thyao.json` (200 bar, 17 803 bayt) ve aynı dokümanın
sabit tohumlu (LCG) rastgele yürüyüşle 10 000 bara uzatılmışı (767 680 bayt; takvim
yok sayıldı, fiyat yürüyüşle 21'e kadar kayıyor, ölçüme etkisi yok). 29 adımlık
deterministik dizi: üç hesap ekle+göster, iki `set_params`, `set_view_range`, ortaya
katman ekle (index 4), katman sil (index 1 → kayma), olgu, bulgu+kapsül, `update_finding`,
`reject_finding`, bar ekle, iki canlı son bar güncellemesi, niyet, `remove macd`
(katmanıyla), beş geri alma (#4, #7, #12, #6, #24), 3 bar ekle, `remove rsi7` (katmanıyla),
`remove bb20`, `set_params bb10`, `approve_finding`, `add_study sma200`. 27 adım uygulandı,
2 geri alma otorite tarafından `undo_conflict` ile reddedildi (aşağıda).

**Alternatifler** (her adımda aynı otorite geçişi, farklı ifade):

| Kod | Ne | Nasıl üretildi |
|-----|----|----------------|
| A1 | RFC 6902, `json_patch::diff` (crate) | önceki/sonraki `serde_json::Value` farkı |
| A2 | RFC 6902, en iyi genel fark | kendi yazdığım LCS + `id` ile eşleyen dizi farkı (jsondiffpatch'in `objectHash` yaklaşımı), ön/son ek kırpma |
| A3 | RFC 6902, otoritenin **ürettiği** yama | fark almadan, uygulanan her `ChangeOp`'tan o anki indeksle; `put_*` için alan düzeyinde fark |
| M | RFC 7396 Merge Patch | önceki/sonraki farkı; `json_patch::merge` ile uygulandı |
| B | `ChangeOp` listesi (§5.3) | otoritenin açılımı |
| Bop | istenen `Op` listesi (günlük birimi olarak) | istek |
| C | karma: `{"request": Op[], "patch": A3}` | |

Her adımda tüm yansılar (A1/A2/A3/M için `Value`, B için tipli `Document`) otoriteyle
karşılaştırıldı (`assert_eq`); hepsi her adımda eşit kaldı. Süreler: N=200'de 21, N=10 000'de
7 tam koşunun adım başına medyanı. Boyut: sıkıştırılmamış JSON baytı; gzip hem mesaj başına
(her mesaj ayrı) hem de bütün günlük tek akış olarak.

## 3. Sonuçlar

### 3.1 Tel boyutu (`steps_N200.csv`, `steps_N10000.csv`)

29 adımın toplamı (bayt):

| | A1 | A2 | A3 | M | **B** | Bop | C |
|---|---:|---:|---:|---:|---:|---:|---:|
| ham, N=200 | 9 808 | 5 528 | 5 250 | 290 267 | **5 171** | 2 757 | 8 574 |
| gzip mesaj başına, N=200 | 4 649 | 3 945 | 3 964 | 88 331 | **4 073** | 2 798 | 4 943 |
| gzip akış, N=200 | 1 823 | 1 429 | 1 388 | 11 385 | **1 326** | 986 | 1 762 |
| ham, N=10 000 | 9 814 | 5 534 | 5 234 | **13 038 079** | **5 151** | 2 740 | 8 541 |

Seçilmiş adımlar (N=200, ham bayt):

| Adım | A1 | A2 | A3 | M | B | Bop |
|------|---:|---:|---:|---:|---:|---:|
| 04 `set_params ema50` | 71 | 71 | 71 | 16 374 | 126 | 63 |
| 05 `set_params rsi14` (seri `instrument` taşıyor) | 71 | 71 | 71 | 16 374 | 210 | 63 |
| 07 `add_layer` index 4 | 736 (11 işlem) | 115 | 115 | 770 | 121 | 118 |
| 08 `remove_layer` index 1 | **1 258 (20 işlem)** | 44 | 44 | 697 | 47 | 47 |
| 12 `reject_finding` | 260 | 260 | 260 | 823 | 251 | 104 |
| 14 `update_last_bar` | 272 (4 alan) | 272 | 137 | 16 452 | 121 | 124 |
| 17 `remove macd` (katmanıyla) | **1 445 (25 işlem)** | 79 | 79 | 16 888 | 75 | 29 |
| 22 `append_bars` (3 bar) | 395 | 395 | 389 | 16 589 | 277 | 277 |

- Toplamda A3 ≈ A2 ≈ B (±%7); fark yok denecek kadar küçük. A, alan düzeyinde değişimde
  kazanır (`set_params`: 71 B'ye karşı 126–210 B, çünkü `put_series` tüm seriyi yeniden yazar);
  B, dizi işlemlerinde ve toplu bar eklemede kazanır.
- `json-patch` crate'inin `diff`'i dizileri **konumsal** karşılaştırır: ortadan bir öğe
  silmek, kayan her öğe için alan alan `replace` üretir (adım 08: 47 B yerine 1 258 B).
- Merge Patch dizileri bütün olarak değiştirir: bar dokunan her değişimde tüm bar dizisi
  gider (N=10 000'de adım başına ~766 KB, toplam 13 MB).
- Bop en küçük (istek sunucu açılımını taşımaz); ama tek başına yeniden oynatılamaz (§3.8).

### 3.2 Değişimi hesaplama maliyeti (otoritede; adım medyanı, µs)

| | N=200 | N=10 000 |
|---|---:|---:|
| B: açılım + doğrulama (`expand`) | 0,21 | 1,00 |
| B: ters türetme | 0,08 | 0,29 |
| B: serileştirme | 0,25 | 0,50 |
| A3: üretme (emit) | 0,58 | 2,37 |
| A1/A2/M ön koşulu: `to_value(doc)` | 30,7 | 1 253,7 |
| A1: `json_patch::diff` | 75,6 | 3 139,9 |
| A2: LCS/id farkı | 12,1 | 388,0 |
| M: merge farkı | 15,2 | 623,2 |

Fark almak belge boyutuyla doğrusaldır: N=10 000'de A1 değişim başına **~4,4 ms**
(`to_value` + `diff`), B ~1,8 µs (≈2 400×). A3 (üretilen yama) fark almadığı için B ile
aynı mertebededir.

### 3.3 İstemci yansısında uygulama (çözümleme + uygulama; adım medyanı, µs)

| | N=200 | N=10 000 |
|---|---:|---:|
| B: `from_str::<Vec<ChangeOp>>` + uygula (tipli `Document`) | 0,79 + 0,17 | 3,33 + 0,46 |
| A1: `from_str::<Patch>` + `json_patch::patch` (`Value`) | 1,08 + 0,75 | 2,04 + 1,92 |
| A3: aynısı | 0,58 + 0,62 | 0,75 + 1,21 |
| M: `from_str::<Value>` + `merge` | 46,9 + 27,7 | 2 122 + 1 313 |

Küçük değişimlerde B ve A aynı mertebede (µs); Merge Patch dışında uygulama maliyeti
ayırt edici değil.

### 3.4 Sıcak yol: 10 000 barda saniyede 60 canlı bar güncellemesi (`hotpath.csv`)

600 ardışık `update_last_bar` (10 001 bar). Süre p50 / p99 (µs) ve 60 Hz'de tek çekirdek payı:

| | p50 | p99 | 60 Hz'de CPU |
|---|---:|---:|---:|
| B açılım + doğrulama + ters | 0,83 | 3,29 | %0,005 |
| B serileştirme | 0,83 | 1,88 | %0,005 |
| B istemci çözümleme + uygulama | 3,62 | 7,38 | %0,02 |
| A3 üretme + serileştirme | 9,75 | 16,46 | %0,06 |
| A3 istemci çözümleme + uygulama | 4,29 | 7,58 | %0,03 |
| **A1 `to_value` + `diff`** | **4 583** | **5 062** | **%27,5** |
| A1 `clone(prev)` + `diff` (otorite durumu `Value` ise) | 4 598 | 5 023 | %27,6 |
| A2 `to_value` + LCS farkı | 3 054 | 3 356 | %18,3 |
| M `to_value` + merge farkı | 3 045 | 3 353 | %18,3 |

Tel (ortalama bayt / gzip / 60 Hz'de bayt/s):

| | bayt | gzip | B/s @60 Hz |
|---|---:|---:|---:|
| B `set_last_bar` | 117 | 111 | 7 020 |
| A3 `replace /series/0/data/bars/10000` | 135 | 127 | 8 100 |
| A1/A2 alan farkı (`.../10000/c`, `.../10000/v`) | 127 | **101** | 7 620 |
| M merge patch | **766 039** | 212 128 | **45 962 340** |

Sıcak yolda telde fark yok (A1'in alan farkı gzip'te B'den bile küçük). Fark otoritede:
fark alma bir çekirdeğin ~%28'ini yer, aynı yüke ikinci bir belge eklemek iki katı demektir;
B'de maliyet belgeden bağımsızdır. Merge Patch bu yolda kullanılamaz (46 MB/s).

### 3.5 Eşzamanlılık ve sıra değişimi (`concurrency.csv`)

Taban: başlangıç dokümanı + ilk üç adım (7 seri, 8 katman). 9 senaryo × 2 sıra = 18 koşu.
İki istemci aynı tabanı görür, birbirinden habersiz yazar; otorite sırayla uygular.
Kâhin: işlemlerin kimlikle (indeks yerine öğenin kendisiyle) yorumlanmış sonucu.
Sınıflar: **doğru** (sonuç = kâhin), **ret** (ikinci yazı reddedildi; sonuç = yalnızca ilki,
güvenli ama yeniden denemek gerekir), **YANLIŞ** (sessizce farklı sonuç ya da bozuk belge).

| Alternatif | doğru | ret | YANLIŞ |
|---|---:|---:|---:|
| A-blind (istemci yaması, olduğu gibi uygulanır) | 8 | 0 | **10** |
| A-test (her remove/replace'ten önce o yoldaki değerin `test`'i) | 12 | 3 | 3 |
| A-test-id (yolun geçtiği her dizi öğesinin `id`'si de test edilir) | 12 | 4 | 2 |
| A-blind + sonradan alan doğrulaması | 10 | 0 | 8 |
| A-test + doğrulama | 14 | 3 | 1 |
| A-test-id + doğrulama | 14 | 4 | **0** |
| B-op (otorite `Op`'u doğrular, `base` yok) | 14 | 0 | **4** |
| B-op + `base` (§8.2) | 14 | 4 | **0** |
| B-id (kimlikle katman silme; protokolde **yok**, deney uzantısı) | **18** | 0 | 0 |

Öne çıkanlar:

- **S4** (`remove macd` ∥ `set_params bb20 period=30`): Y'nin yaması
  `replace /series/3/source/params/period`. X'ten sonra `/series/3` artık `sma20` ve onun
  da `period: 20` değeri var — değer `test`'i **geçer**, `sma20`'nin periyodu sessizce 30
  olur. Yalnızca kimlik testi (A-test-id) yakalar. B'de `set_params` kimlikle çalıştığı için
  doğru.
- **S1/S2/S3** (indeksli katman silme): A-blind ve **B-op** aynı yanlışı yapar; çünkü
  protokolün `remove_layer {view, index}` işlemi de indekslidir. B-op'un 4 yanlışının
  hepsi buradan. `base` verilirse çakışma olarak döner; kimlikle silme (B-id) her iki sırada
  da doğru sonuç verir ve ret bile gerekmez.
- **S6** (iki besleme aynı barı ekler): `add .../bars/-` için `test` anlamsızdır; yalnızca
  sonradan doğrulama (artan zaman) ya da B'nin açılım kuralı yakalar.
- **S8** (`reject` ∥ `approve`): A-blind karnede iki çelişkili karar bırakır; `test`
  (`state == "live"`) ya da B'nin geçiş kuralı ikinciyi reddeder.

### 3.6 Geri alma (`undo.csv`)

Ölçüt: "hedef girdi hiç olmasaydı" dokümanı (geri kalan adımlar yeniden oynatılarak
hesaplandı; kimlikle sıralı koleksiyonlarda sıra yok sayıldı). A için ters yama hedef
girdinin anında hesaplanmış `diff(sonra, önce)`'dir ve **bugünkü** duruma uygulanır.

| Geri alma | B (`inverse` + `undo_conflict`) | A1 ters | A2 ters | A2 + `test` | A2 + id `test` |
|---|---|---|---|---|---|
| #4 `set_params ema50` (arada `remove macd` seri indekslerini kaydırdı) | doğru | YANLIŞ | **YANLIŞ** (`rsi7`'nin periyodu 50 oldu) | ret | ret |
| #7 `add_layer` (arada katman silindi/eklendi) | ret (`undo_conflict`) | ret | YANLIŞ (başka katmanı sildi) | ret | ret |
| #12 `reject_finding` | doğru | doğru | doğru | doğru | doğru |
| #6 `set_view_range` (görünüme sonra başka işler dokundu) | **ret** (gereksiz) | doğru | doğru | doğru | doğru |
| #24 `remove bb20` | doğru, ama **bayt-eşit değil** | YANLIŞ | doğru | doğru | doğru |

- Ters türetilebilir mi? B: evet, her `ChangeOp` için önceki durumdan (otorite zaten tutar);
  ters de tam öğe taşıdığı için kendi başına okunur. RFC 6902: `replace`/`remove` eski değeri
  taşımaz; ters ancak önceki durum elde varken (fark ya da kayıt) türetilebilir.
- Boyut (27 adım toplamı): B tersleri 3 280 B, A2 3 557 B, A1 7 799 B. Korumalı (`test`li)
  ters 2–4× büyür (örn. `reject_finding` tersi 90 B → 360 B).
- B'nin iki zayıflığı ölçüldü: (1) dokunulan kimlik görünüm düzeyinde olduğu için #6 gereksiz
  yere reddedildi (A doğru yaptı); (2) `delete`'in tersi `put_*` öğeyi koleksiyonun **sonuna**
  ekler, ilk konumuna değil; anlamca doğru, bayt olarak değil (A2 tersi konumu korudu).

### 3.7 Niyet ve denetim (elle puanlandı)

Ölçüt (her biri 0–2): **N** ne oldu (alan fiili 2, ilkel alan fiili 1, `add/remove/replace` 0);
**H** hangi öğe (kararlı kimlik 2, kimlik+konum 1, yalnızca dizi yolu 0); **G** neden
(not/gerekçe 2, yalnızca özet/cause 1, hiç 0); **K** kendine yeterlik (önceki durum ve kod
olmadan anlaşılır 2, birini ister 1, ikisini ister ya da yanıltır 0). Puanlar benim;
örnekler `raw/t6c/examples.json`'da.

| İşlem | Bop | B (yalnız) | LogEntry (Op + ChangeOp + summary) | C | A3 | A1 |
|---|---:|---:|---:|---:|---:|---:|
| add_study + show | 6 | 4 | 7 | 7 | 2 | 2 |
| set_params | 6 | 4 | 7 | 7 | 1 | 1 |
| remove_layer (indeks) | 5 | 3 | 6 | 5 | 0 | 0* |
| remove series (katmanıyla) | 6 | 4 | 7 | 7 | 0 | 0* |
| reject_finding | 7 | 7 | 8 | 8 | 5 | 5 |
| update_last_bar | 6 | 6 | 7 | 7 | 1 | 1 |
| undo | 3 | 5 | 7 | 5 | 0 | 0 |
| **toplam / 56** | 39 | 33 | **49** | 46 | 9 | 9 |

\* A1 burada **yanıltıcı**: `remove macd` fark olarak "`/series/2/id` → `bb20`,
`/series/2/source/study` → `bollinger`, …" diye görünür; okuyan bir ajan bunu "macd'yi
bb20 olarak yeniden adlandır" diye anlar. Örnekler:

```
reject_finding (Bop) {"op":"reject_finding","finding":"fnd-overbought","note":"RSI(21) ile aşırı alım teyit edilmedi."}
reject_finding (B)   [{"op":"set_capsule_state","capsule":"cap-overbought","state":"rejected"},
                      {"op":"push_ledger","entry":{"finding":"fnd-overbought","state":"rejected","outcome":"pending","recorded_at":1760000720000,"note":"…"}}]
reject_finding (A3)  [{"op":"replace","path":"/capsules/0/state","value":"rejected"},
                      {"op":"add","path":"/ledger/entries/-","value":{…aynı kayıt…}}]
remove macd (Bop)    {"op":"remove","id":"macd"}
remove macd (B)      [{"op":"delete_layer","view":"main","index":4},{"op":"delete","id":"macd"}]
remove macd (A3)     [{"op":"remove","path":"/views/0/layers/4"},{"op":"remove","path":"/series/2"}]
remove macd (A1)     [{"op":"replace","path":"/series/2/id","value":"bb20"}, … 25 işlem]
```

`reject_finding`'de A bile okunur kalır, çünkü karne kaydı notu ve bulgu kimliğini taşır;
kimlik taşımayan işlemlerde (katman, parametre, bar) JSON Patch denetim izi olarak işe
yaramaz. Protokolün `LogEntry`'si (istek + değişimler + ters + özet) iki dünyanın en iyisini
zaten birleştiriyor; C'nin (Op + JSON Patch) ona üstünlüğü yok.

### 3.8 Doğrulama (`validation.csv`)

13 geçersiz istek, her biri hem `Op` hem elle yazılmış JSON Patch olarak. A tarafında
otoritenin elindeki her denetim denendi: yama uygulama hatası, tipli `Document`'e
çözümleme (`deny_unknown_fields`), üretilmiş JSON Schema (`qp_schema::validate`), sonuç
belgenin alan doğrulaması (`validate_doc`: referanslar, artan barlar, parametre adları ve
işaretleri, güven aralığı, görünüm aralığı) ve `feed` için yol tabanlı ACL.

| Durum | B | A: hangi denetim yakaladı |
|---|:-:|---|
| V1 katman olmayan seriye bakıyor | ✓ | alan doğrulaması |
| V2 kullanımdaki seriyi sil | ✓ | alan doğrulaması (sarkan referans) |
| V3 bar serisine `set_params` | ✓ | yalnızca JSON Schema (tipli çözümleme **kabul etti**) |
| V4 bulgu olmayan olguya dayanıyor | ✓ | alan doğrulaması |
| V5 güven 1,5 | ✓ | JSON Schema + alan |
| V6 eklenen bar son bardan önce | ✓ | alan doğrulaması |
| V7 `update_last_bar` zamanı değiştiriyor | ✓ | **yakalanamadı** (sonuç geçerli bir belge) |
| V8 reddedilmiş bulguyu onayla | ✓ | **yakalanamadı** (geçiş kuralı) |
| V9 bilinmeyen parametre adı | ✓ | alan doğrulaması |
| V10 `feed` bulgu yazıyor | ✓ | yol ACL'si |
| V11 katman indeksi 99 | ✓ | yama uygulama hatası |
| V12 ters görünüm aralığı | ✓ | alan doğrulaması |
| V13 negatif periyot | ✓ | alan doğrulaması |
| **toplam** | **13/13** | **11/13** |

A'nın kaçırdıkları **geçiş kurallarıdır** (önceki ve sonraki durum birlikte gerekir:
"son bar değiştirilirken zaman aynı kalır", "kapsül yalnızca `live`'dan karara geçer").
Sonuç belge geçerli olduğu için durum doğrulaması bunları göremez. Maliyet: A'da her
değişimden sonra belge baştan doğrulanır — N=10 000'de tipli çözümleme 2 377 µs + JSON Schema
(önceden derlenmiş) 457 µs + alan kuralları 13 µs; B'de açılım+doğrulama ~1 µs. Ayrıca A'nın
hata mesajı isteğe değil sonuca işaret eder ("layer 7: series nope missing"), B'nin
mesajı isteğe (`not_found: layer series 'nope'`, `path: /ops/0`). ACL de indeksle
çalışır: "feed yalnızca `px`'e yazar" kuralı yol düzeyinde ifade edilemez, yalnızca "herhangi
bir serinin barlarına yazar".

### 3.9 Yeniden oynatma

Başlangıç dokümanı + günlük → otoritenin baytları:

| Günlük | Sonuç |
|---|---|
| B (`ChangeOp`) | bayt-eşit (N=200 ve N=10 000) |
| A1/A2/A3/M (`Value` üstünde) | ham `Value` baytları **eşit değil**; tipli `Document`'e çevrilip yeniden yazılınca eşit |
| Bop, aynı açılım kodu (v1) | bayt-eşit |
| Bop, değiştirilmiş açılım (v2: `bollinger` varsayılan `k`=2,5, yeni katman kural katmanlarının önüne) | **eşit değil**; 27 girdiden 6'sının değişimleri farklı |

Hepsi deterministik; ama JSON Patch yansısı `serde_json::Value` anahtarlarını alfabetik
tuttuğu için bayt değişmezi (§2) ancak tipli dokümana geri dönüşle sağlanır. `Op` günlüğü
yeniden oynatmayı açılım kodunun sürümüne bağlar; bu yüzden `Op` yalnız başına günlük
birimi olamaz (protokol zaten `changes`'i saklıyor — doğrulandı).

### 3.10 Ajan yazabilirliği (yapıdan çıkarım; LLM denemesi yapılmadı)

27 uygulanan adımın toplamı (`writability.csv`):

| İstemcinin yazacağı biçim | bayt | dizi indeksi | köken/zaman alanı |
|---|---:|---:|---:|
| `Op` | 2 757 | 2 (`add_layer`/`remove_layer` index) | 0 |
| JSON Patch (A3, en iyi hâli) | 5 250 | 45 | 6 |
| (`ChangeOp` — otorite üretir, istemci yazmaz) | 5 171 | 9 | 7 |

JSON Patch yazmak için ajanın şunları bilmesi gerekir: her öğenin güncel dizi indeksi
(`/series/5`, `/capsules/0`), seri silinince hangi katmanların düşeceği (unutursa sarkan
referans), karne kaydının tam şekli (`outcome`, `recorded_at` — otoritenin saati), bulgu
eklerken `provenance`. `Op`'ta hiçbiri yoktur: kimlik ve fiil yeter. Boyut olarak da `Op`
JSON Patch'in yarısı. Ajanın sık yaptığı hatalar (yanlış indeks, eksik alan) A'da
§3.5'teki sessiz yanlışlara, B'de anlaşılır hatalara dönüşür.

## 4. Hipotezlerin durumu

| | Sonuç |
|---|---|
| H1 boyut | **Kısmen.** Üretilen yama (A3) toplamda B'ye eşit (5 250 / 5 171 B); alan düzeyinde (`set_params`) A kazanıyor. Crate `diff`'i konumsal: toplam 1,9×, tek adımda 27×. Merge Patch 10 000 barda 13 MB. |
| H2 hesap | **Doğrulandı.** 10 000 barda fark alma değişim başına ~4,4 ms; 60 Hz'de çekirdeğin %27,5'i. B ~1–2 µs. |
| H3 uygulama | **Doğrulandı.** İkisi de µs; A1 sıcak yolda B'den biraz hızlı (1,7 / 3,6 µs). |
| H4 eşzamanlılık | **Doğrulandı, B dahil.** A-blind 10/18 yanlış; `remove_layer` indeksli olduğu için B-op da 4/18 yanlış; `base` ya da kimlikle silme gerekli. |
| H5 geri alma | **Doğrulandı + iki sürpriz.** Korumasız A tersleri 5 durumda 2 yanlış; B 0 yanlış ama 1 gereksiz ret ve silmenin geri alınmasında konum kaybı. |
| H6 niyet | **Doğrulandı.** 49 (LogEntry) / 39 (Op) / 33 (ChangeOp) / 9 (JSON Patch) / 56. |
| H7 doğrulama | **Doğrulandı.** 13/13'e karşı 11/13; kaçanlar geçiş kuralları; A'da maliyet belge boyutuyla doğrusal. |
| H8 yeniden oynatma | **Doğrulandı.** v2 açılımla Op günlüğünde 6 girdi farklı; `Value` baytları anahtar sırası yüzünden farklı. |
| H9 ajan | **Yapıdan doğrulandı** (LLM denemesi yok): 45 indeks + 6 köken alanına karşı 2 indeks + 0. |

## 5. Dürüst sonuç: genel farkın kazandığı yerler

- **İşlem başına sıfır kod.** A1/A2 her yeni öğe türünü ya da alanı kendiliğinden kapsar.
  B'de her yeni tür için `ChangeOp` varyantı + uygulama + ters + dokunulan kimlik kodu
  gerekir (deneyde `apply`/`inverse_one`/`touched` ~230 satır ve açılım ayrıca; crate'in `diff`'i
  0 satır, en iyi genel fark (LCS+id) ~185 satır ve her türü kapsıyor).
- **İki keyfi belge arasındaki fark.** İçe/dışa aktarma, çevrimdışı düzenlenmiş bir dosyayı
  otoriteyle karşılaştırma, iki anlık görüntüyü hata ayıklamak, test başarısızlığında "ne
  farklı?" sorusu: `ChangeOp` iki durumdan türetilemez; genel fark türetir. RFC 6902/7396 her
  dilde hazır kütüphaneyle gelir.
- **Alan düzeyinde ayrıntı.** `set_params` 71 B'ye karşı 126–210 B; canlı barda gzip 101 B'ye
  karşı 111 B. Tam öğe taşımanın (değişmez §2 için seçilmiş) küçük bir bedeli var.
- **Konumu koruyan ters.** Silinen bir öğenin geri alınması A2 ile ilk konumuna döner; B'de
  sona eklenir (anlamca aynı, bayt olarak değil).
- **İnce taneli çakışma.** A'nın yolu (`/views/0/x/domain`) B'nin görünüm düzeyindeki dokunulan
  kimliğinden daha incedir; #6'daki gereksiz `undo_conflict` A'da yok.

Bunların hiçbiri otoritenin değişim formatı için belirleyici değil: genel farkın kazandıkları
araç ve kenar durumlarıdır; kaybettikleri (otorite maliyeti, sessiz yanlışlar, geçiş
kuralları, denetim izi, ajan yazabilirliği) protokolün çekirdeğidir.

## 6. Öneri

1. **Telde ve günlükte `ChangeOp` (B) kalsın; istekte `Op`; `LogEntry` ikisini birlikte
   tutsun** (protokolün mevcut tasarımı). `Op` tek başına günlük birimi olmasın (açılım
   sürümüne bağımlı; §3.9).
2. **İstemciden JSON Patch kabul edilmesin.** Kör uygulamada 18'de 10 yanlış; en iyi
   korumayla (id `test` + sonradan doğrulama) ancak `base`'li B'nin güvenliğine ulaşıyor,
   ama geçiş kurallarını yine göremiyor ve 10 000 barda değişim başına ~3 ms doğrulama istiyor.
3. **Merge Patch hiçbir yerde kullanılmasın** (dizileri bütün olarak taşır).
4. **JSON Patch türetilmiş, yetkisiz bir görünüm olarak kalsın:** `qp diff a.json b.json`
   gibi bir hata ayıklama/dışa aktarma aracı (LCS + `id` eşlemeli fark; crate'in konumsal
   `diff`'i değil). Gerekirse genel JSON tüketicileri için A3 biçimi otoritede ~2 µs'de
   üretilebilir; günlüğe yazılmaz.
5. **B'nin ölçülen zayıflıkları** kapatılmalı (aşağıdaki Öneriler); en önemlisi katman
   kimliği: indeksli `remove_layer`, B-op'un tüm eşzamanlılık yanlışlarının kaynağı.

## 7. Açık sorular ve öneriler (kapsam dışı; yapılmadı)

- **Katman kimliği.** `Layer`'a kararlı `id` (ya da `remove_layer`'ın katmanı seri/işaretle
  seçmesi). Kimlikle silme 18/18 doğru, indeksli 14/18. `delete_layer`/`insert_layer`
  değişimleri de bundan yararlanır (geri alma #7 bugün yalnızca `undo_conflict` ile güvende).
  Yeni bir alan/kavram olduğu için önce `docs/CONCEPTS.md`.
- **Daha ince dokunulan anahtarlar.** `main` yerine `main.x`, `main.layers` gibi; #6'daki
  gereksiz `undo_conflict` ve `base` çakışmaları azalır.
- **Silmenin geri alınmasında konum.** `put_*`'a isteğe bağlı konum ya da kimlikli
  koleksiyonlarda kanonik sıra (örn. kimliğe göre); aksi hâlde "geri al = önceki baytlar"
  sağlanmaz.
- **`LogEntry`'de `reason` yok.** PROTOCOL.md §4.2 "neden; günlüğe ve öneriye yazılır" diyor,
  ama `LogEntry` tipinde `reason` alanı yok (yalnızca `Proposal.reason`). Denetim izi için
  eklenmeli ya da belge düzeltilmeli.
- **`deny_unknown_fields` ve iç etiketli birim varyantı.** V3'te
  `{"kind":"inline","params":{…}}` tipli `Document`'e hatasız çözümlendi (fazla alan sessizce
  düşer); yalnızca JSON Schema yakaladı. `qp-schema`'da bir test ile doğrulanmalı.
- **`set_params` için daha küçük bir değişim** (`set_series_params`) isteğe bağlı; kazanç
  değişim başına ~100 B, şimdilik gerekmiyor.
- Ajan yazabilirliği yalnızca yapıdan çıkarıldı; gerçek bir LLM'e aynı 20 görevi `Op` ve
  JSON Patch olarak yazdıran bir deneme yapılmadı.
