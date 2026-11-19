# R6 — Alt Görevler

| # | Görev | Yazdığı yer | Canlı bağlantı | Bağımlılık |
|---|-------|-------------|----------------|------------|
| T0 | Hazırlık: ana akış değişiklikleri; gösterim varsayılanları (`korda`, `live_new`); sözlük; ortam değişkenlerinin **varlığını** denetleyen betik (değerleri yazdırmaz; eksik olanın adını söyler) | `scripts/`, `shells/web/`, `docs/CONCEPTS.md` | yok | — |
| T1 | Proto ve çözücü: örnek bağlayıcının proto klasöründeki dosyaların **tamamını değiştirmeden kopyala** (K62) ve köken kaydını yaz (kaynak yol, `quantex` commit'i, tarih, dosya özetleri); Rust tiplerini derleme sırasında bu dosyalardan üret; alan varlığını koruyan çözme; elde kurulmuş test iletileri | `crates/qp-source/` (ya da yeni `crates/qp-matriks/`), proto klasörü onun altında | yok | — |
| T2 | Bağlantı çekirdeği: belirteç alma ve yenileme, MQTT (WebSocket üstünden), tek bağlantı kilidi, "çoklu bağlantı" bildirimi, geri çekilme, günlük maskeleme; sahte sunucuyla testler | aynı crate | yok (sahte sunucu) | — |
| L1 | **İlk canlı deneme (orkestratör):** gecikmeli kip, 3–5 sembol, kısa süre; ham kayıt alınır; çözücü kayıt üzerinde doğrulanır | `fixtures/local/` | **var, tek** | T1, T2 |
| T3 | Borsa saati, açıklık, seans evresi → oturum saat girdisi ve dil | `crates/qp-session/`, `crates/qp-calendar/`, `crates/qp-capsule/` | yok (kayıt) | L1 |
| M22 | Canlı bar üretici, alternatifleriyle; REST karşılaştırması | `crates/qp-source/` | yok (kayıt) | L1 |
| T4 | Ek yeri: geçmiş + canlı; hacim birimi; düzeltme seçimi | `crates/qp-source/`, `crates/qp-session/` | yok (kayıt) | M22 |
| M23 | Kopma, bayat, onarım | `crates/qp-source/`, `crates/qp-session/` | L2'de | T2, T4 |
| T5 | Kayıttan oynatma (`qp replay` ham Matriks kaydıyla) | `crates/qp-cli/`, `crates/qp-source/` | yok | T1 |
| T6 | İç içe gösterge adresi (K63) | `crates/qp-compute/` ya da `crates/qp-schema/` | yok | — |
| T7 | Canlı gösterim: `scripts/demo.sh --live`; otorite evreni dinler, cihaz görüntüleyici | `crates/qp-cli/`, `shells/web/` | L2'de | T3, T4 |
| L2 | **Canlı denemeler (orkestratör, sırayla):** tam seans karşılaştırması (M22), bilerek kesme (M23), evren büyütme (M24), yük ölçümü, canlı kapsül gözlemi; sonra anlık kip (Arkin kipi değiştirince) | `rounds/R6/experiments/` (yalnız toplu sayı) | **var, tek** | T7 |
| T8 | RI1 sorularının cevapları belgelere | `docs/matriks/` | yok | L1, L2 |
| A | Denetim: bağımsız alt ajan; koruma kurallarını kodda ve testte doğrular; gizli bilgi ve veri taraması; kayıt üzerinden ölçümleri yeniden üretir. **Canlı bağlantı açmaz** | `rounds/R6/AUDIT.md` | yok | hepsi |

## Notlar

- **Canlı bağlantı disiplini:** L1 ve L2 dışında hiçbir görev ağa çıkmaz. Orkestratör bir
  canlı deneme sürerken başka canlı deneme başlatmaz; kilit dosyası bunu zaten reddeder.
  Her canlı denemenin başı, sonu, kipi, sembol sayısı ve sonucu `experiments/LIVE-LOG.md`'ye
  yazılır (veri değil, olay kaydı).
- **Borsa saati:** canlı denemeler seans saatlerinde anlamlıdır. Seans dışında bağlantı ve
  ilk durum denenebilir; bar ve kapsül ölçümleri seansı bekler. Round seansa denk
  gelmiyorsa canlı ölçütler (4, 8, 9) "seans bekleniyor" diye açık bırakılır ve Arkin'e
  hangi komutu ne zaman çalıştıracağı yazılır; uydurulmaz.
- **Anlık kip:** gecikmeli kipte L1 ve L2'nin tamamı geçtikten sonra Arkin'den
  `MATRIKS_MQTT_MODE=realtime` yapması istenir; ajan `.env.local`'ı değiştirmez.
- **Örnek bağlayıcıyı okuma:** `quantex-stream/matriks-adapter` ve `docs/matriks/*`. Orada
  `.env*` ve gizli dosyalar açılmaz. **Proto dosyaları aynen kopyalanır** (K62); başka kod
  kopyalanmaz, mantık Rust'ta yeniden yazılır. Proto'da alan varlığı bilgisi yoksa
  (proto3 varsayılanları) çözücü bunu tel biçiminden okur; proto dosyası bunun için değiştirilmez.
- **Proto derleme:** tip üretimi için kullanılan araç CI'da ve temiz klonda ek kurulum
  istemeden çalışmalı (gerekirse saf Rust bir derleyici ya da depoya alınmış üretilmiş kod +
  "üretilmiş kod güncel" testi). Seçimden önce belgelerine bak ve küçük denemeyle doğrula.
  RI1'in işaretlediği kusurlar devralınmaz: eksik alanı 0 saymak, kopyalar arası belirteç
  paylaşımı, yenilenen belirtecin açık bağlantıya geçmemesi.
- **Bağımlılıklar:** MQTT, WebSocket, TLS ve protobuf için güncel, bakımlı Rust
  kitaplıkları; seçimden önce belgelerine bak ve küçük denemeyle doğrula; lisansları uygun
  olmalı. Bu bağımlılıklar yalnız sunucu tarafındadır: **WASM paketine girmez** (paket
  boyutu ölçülür, K52).
- **Hesap çekirdeği değişmez:** `qp-compute` ağ ve saat bilmez (K3). Bağlayıcı ayrı crate ya
  da `qp-source` içinde özellik bayrağı arkasındadır; hesap özeti aynı kalır.
- **Hacim ve düzeltme:** REST barlarında `divide=false` düzeltilmemiş veriyi verir
  (`docs/DATA_ACCESS.md`); canlı fiyat düzeltilmemiştir. Varsayma; ölç ve belgeye yaz.
- **Lisans:** bu round'da yalnız fiyat kartı. Lisanslı ailelerin (akış) dedektörü canlıda
  "veri yok" der; sentetik akış canlı gösterime karıştırılmaz.
- **Disk (K56):** `target/wt/<görev>`; elenen dalın worktree'si hemen kaldırılır; ham
  kayıtlar büyüyebilir: `fixtures/local/` boyutu raporda, eski kayıtlar için saklama kuralı.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli.
