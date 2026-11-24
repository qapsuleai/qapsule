# Mimari

Durum: taslak v0. R0'daki denemelerin sonucuna göre güncellenir; RD1'de protokol katmanı eklendi.

## Sınır

Qapsule bir **kütüphane ve sözleşme** projesidir. Ajanların çalıştığı ortam, abonelik,
persona ve dağıtım ayrı sistemlerdir ve Qapsule'ı tüketir.

```
            Matriks yayınları / NATS / mevcut indicator-engine, anomaly-engine
                                   │  (seri ve olgu akışları)
                                   ▼
┌──────────────────────────── QAPSULE ─────────────────────────────┐
│  Seri kaynağı arayüzü   ── kaynağı soyutlar                      │
│  Hesap çekirdeği        ── deterministik, akışkan, istek anında  │
│  Olgu dedektörleri      ── seriden tipli olgu                    │
│  Doküman + komut yolu   ── tek şema, tek yazma yolu              │
│  Görsel gramer          ── görünüm tarifi                        │
│  Çizim çekirdeği        ── headless + etkileşimli                │
│  Ajan yüzeyi            ── CLI, MCP, anlamsal okuma              │
└───────────────────────────────────────────────────────────────────┘
        ▲                           ▲                        ▲
   Yorumlayan ajanlar        Kabuklar (web, iOS,       Gömme (asistanlar,
   (ayrı sistem)             Android, sunucu)          WhatsApp, widget)
```

## Kalıcı çekirdek

| Parça | Sorumluluk |
|-------|------------|
| Şema | Seri, Olgu, Bulgu, Kapsül, Görünüm, Doküman, Komut tipleri. Rust tipi tek kaynaktır; JSON Schema, TS tipleri ve ajan araç tanımları ondan üretilir. |
| Hesap çekirdeği | `f64`, akışkan (her yeni bar O(1)), çok girdili/çok çıktılı, parametre taraması için toplu çalıştırma. LLM yok. |
| Görsel gramer | İşaret (mark), ölçek, katman, kodlama. Mum grafiği ve anlam işaretleri bu gramerle ifade edilir. |
| Komut yolu | Dokümanı değiştiren tek yol. Geri alma, yeniden oynatma ve köken kaydı buradan gelir. |

## Değiştirilebilir kenarlar

| Kenar | Bugün | Yarın olabilir |
|-------|-------|----------------|
| Veri | Dosya, Matriks REST/MQTT | NATS, başka sağlayıcılar |
| Yorumlayan model | Bulut LLM | Cihaz üstü küçük model |
| Ekran | Web, sunucu PNG | Mobil, saat, gözlük, ses |
| Gömme | Kendi görüntüleyicimiz | MCP Apps, A2UI, widget SDK |
| Hesap donanımı | CPU (SIMD) | GPU hesap |
| Uzantı | Derlenmiş hesaplar | Sandbox'ta ajan yazımı hesap/görünüm |
| Tahmin | Yok | Takılabilir tahmin sağlayıcı |

## Alan protokolü (RD1)

Motor, arayüzler ve ajanlar aynı protokolle konuşur: `qp/0` (`docs/PROTOCOL.md`). Arayüz
yalnızca bir istemcidir (AI-native testi 5).

```
  insan arayüzü (web/telefon/saat/ses)      ajan (MCP, satır akışı, süreç içi)
              │  hello · command · query · subscribe · input · attend
              ▼
  ┌──── otorite (qp-authority; bileşim: qp-session) ─────┐
  │ yetki ve öneri ─► işlem açılımı ─► değişim ─► günlük │  ← saat bir girdidir (clock)
  │ sorgular (qp-compute ile)   sunucu (presenter)       │
  │ abonelik ve olaylar         varlık durumu (presence) │
  │ alt sistemler kayıtla (qp-subsystem; ör. qp-watch)   │
  └──────────────────────────────────────────────────────┘
              │  welcome · ack · result · notify · error
              ▼
     yansılar (cihazdaki kabuk, başka ajanlar)
```

- **Tek otorite, sıralı günlük.** Her komut bir günlük girdisi; girdide istek (anlamsal
  `Op`), değişim (ilkel `ChangeOp`, tam öğeli), ters değişim ve köken. Başlangıç dokümanı +
  değişimler = doküman, bayt düzeyinde (yeniden oynatma hesap kodu istemez).
- **Varlık durumu** (bakılan aralık, odak, dikkat yönergeleri) günlüğe girmez.
- **Taşımalar:** süreç içi (`Session::handle_raw`), satır akışı (`qp serve`), MCP
  (`qp mcp serve` ajanlar için; araçlar şemadan üretilir). Uygunluk dökümleri üçünde de
  aynı (MCP'de süreç içi `qp_mcp::conform`). Eski tek kimlikli `qp mcp --init --principal`
  bağdaştırıcısı RD2'de kaldırıldı (A62, K86).

## Kabuk sınırı (R4)

Kabuk (`shells/web/`, ileride iOS/Android) bir protokol istemcisidir. Aynı oturum kabuksuz,
yalnız protokol mesajlarıyla sürüldüğünde aynı doküman ve aynı sahne özetleri çıkar.

```
  tarayıcı                                        yerel barındırıcı (qp demo)
  ┌───────────── kabuk (JS, ince) ─────────────┐     ┌──────────────────────────────┐
  │ düzen öğeleri: liste, sekme, ayar, düğme   │     │ yeniden oynatma (qp-source)  │
  │ girdi → `input` / `command` / `replay`     │◄───►│   veri olayları + saat       │
  │ protokolden gelen metni ve sırayı gösterir │ qp/0│ otorite (qp-session bileşimi)│
  │ WASM (Rust çekirdeği):                     │     │ statik dosyalar (aynı köken) │
  │   yansı, gramer derleme, sahne → piksel    │     └──────────────────────────────┘
  └────────────────────────────────────────────┘
```

**Kabuk yapar:** statik dosyaları ve protokol bağlantısını **aynı kökenden** açar; sahne
tarifini (`SceneDesc`) ya da WASM'ın ürettiği pikselleri boyar; ham girdiyi (dokunma,
kaydırma, yakınlaştırma, düğme) protokol mesajına çevirir; protokolden gelen metni, listeyi
ve sırayı **olduğu gibi** gösterir; düzen öğelerini (liste kaydırma, sekme, ayar sayfası)
çizer; kendi arayüz etiketlerini (düğme adları) iki dilde tutar; ölçüm ekranı için süre
ölçer.

**Kabuk yapmaz:** piyasa verisi ya da hesap değeri üzerinde aritmetik; olgu tespiti;
kapsül/öğe sıralama, süzme, gruplama; cümle kurma ya da alan verisinden metin birleştirme
(sayı ve tarih biçimleme dahil); alan öğelerinin yerleşimi (konumlar sahne tarifinden
gelir); dış ağa istek. Bu kurallar `shells/web/` için mimari testiyle denetlenir
(`scripts/verify.sh` `shell` adımı).

Bir tasarım yalnız kabukta özel kodla mümkünse ya gramer genişletilir (gerekçesiyle,
`docs/GRAMMAR.md`) ya da tasarım elenir. Böylece aynı yüz yerel kabukta ve ekransız çıktıda
da aynıdır.

## Hesap nerede yapılır

- **Ortak ve yaygın olan** sunucuda bir kez hesaplanır ve akış olarak gelir.
- **Geri kalan her şey** çekirdekte, istek anında hesaplanır (cihazda ya da sunucuda;
  aynı kod).
- Bir değer hem akıştan hem yerel hesaptan gelebiliyorsa, tanım tektir ve sonuçlar
  test ile karşılaştırılır.

## Yeni alt sistem nasıl eklenir (RF1)

Otorite (`qp-authority`) hiçbir alt sistemi tanımaz; tek arayüz `qp-subsystem` sözleşmesidir.
Örnek: `qp-watch` (izleme). Sınama: `crates/qp-session/tests/extension/` (`Tally`).

1. **Crate.** Oturum katmanında yeni crate; `scripts/arch-config.json` sırasında
   `qp-subsystem`'den sonra, `qp-authority`'den önce. Bağımlılıkları `qp-subsystem`, `qp-ops`,
   `qp-schema` ve gerekiyorsa sırada önceki alt sistemler; otoriteye bağımlı olmaz.
2. **Durum.** Davranış durumsuz bir `static` nesnedir (`&'static dyn Subsystem`). Durum
   otoritenin yuvalarında: `init`'te `ctx.slots.put(T)`; iki dünyada ayrı tutulacaksa
   `put_world(PerWorld<T>)` (otorite dünya değiştirirken yuvayı da değiştirir); bağlantı başına
   `put_conn`.
3. **`impl Subsystem`.** Her şeyin varsayılanı vardır; alt sistem yalnız yaptığını söyler:
   - `ops()` → `OpHandler`: işlemin açılımı (`expand` → `ChangeOp`'lar) ve politikası
     (`allowed`, `proposable`, `agent`, `to_public`, `reader_private`). Kayıtlı işlem
     yerleşik açılımdan önce sorulur.
   - `phases()` + `collect` (ve `check`). Fazlar ve çağrıldıkları otorite dosyası
     (`crates/qp-authority/src/`): `Check`, `Expand`, `Connect`, `Subscribe`, `Unsubscribe`,
     `Adjust` → `authority/session/phases.rs`; `AfterCommit` → `authority/session/accept.rs`;
     `AfterData` → `authority/session/settle.rs` (`Data`) ve `capsule/run.rs` (`Look`,
     `Written`); `Proposals` → `authority/session/clock.rs`; `Settled` → `capsule/dispute.rs`;
     `Close` çağrılmaz (protokolde kapanış iletisi yok).
   - `queries()`, `inputs()`, `topics()`: `kind` etiketiyle kayıt; kayıtlı olan yerleşikten
     önce sorulur (konu: yerleşik abonelikten sonra kabul/ret).
   - `conditions()` (`ConditionKind`, canlı yol), `value_sources()` (`ValueSource`, veri
     düzlemi), `claims()` (`ClaimKind`, itiraz hükmü).
   - Aynı `op`/`kind`'i iki alt sistem kaydedemez (`Registry::add` hata verir).
4. **Okuma ve yazma.** Alt sistem `ctx.view` (`SessionView`) üstünden okur. Günlüğe yalnız
   iki yoldan yazar: işlemin açılımı ya da `ctx.out.entry(..)` (motor girdisi, yazar `qp`;
   fazın emit adımında, istek sırasıyla, en çok `MAX_EMIT_DEPTH` derinlik). Bildirim
   `ctx.out.notify`; o anın girdisine ek `amend` (`Moment::Look`), o satırdaki bildirim `say`
   (`Moment::Written`).
5. **Yeniden oynatma.** Değişimlerden çalışır: `qp_ops::apply::replay(initial, log)` alt sistem
   kodu istemez. Alt sistemin durumu yeniden oynatmada kurulmaz; dokümanda yaşaması gereken
   her şey değişimle yazılır.
6. **Yansı.** Yansıya giden protokol mesajıdır. Bileşimin kurduğu her otorite (oturum,
   yeniden oynatma barındırıcısı, yansı, görüntüleyicinin açık tarafı) aynı listeyle kurulur;
   otoritenin doğurduğu otorite listeyi ondan devralır.
7. **Kayıt.** `crates/qp-session/src/compose.rs` `LIST`'ine **bir satır**, ve
   `crates/qp-session/Cargo.toml`'a bir bağımlılık satırı. Otorite crate'i değişmez.
8. **Barındırıcı API'si.** Alt sistemin durumu `session.sub::<T>()` / `sub_mut::<T>()`
   (yuva tipiyle); `Deref` üstünden değil. Standart dışı bir liste (sınama):
   `qp_session::Authority::new(init, root, Subsystems::new(list))` ve
   `qp_authority::Replica::with_memory(.., list)`; durum `slots().get::<T>()`.

**Bilinen sınırlar.** (B-e) `amend`/`say` yanlış anda sessizce düşer; `Look`'tan önceki bir
`amend` o bakışın girdisine karışır. (B-f) `qp_session` `Authority` ve `Subsystems`'i (ve
`Subsystems::NONE`) dışa açar; boş listeyle kurulumu engelleyen bir kural yok. Ayrıca
`Query`, `Topic`, `Gesture`, `Event` kapalı `qp-schema` enum'larıdır: yeni bir tür şema
değişikliği ister (alt sistem var olan bir türü üstlenebilir). Kayıtsız alt sistem işlemi
yazma yetkili kişiye `unsupported`, izleyiciye `forbidden` döner (B-a).

## Yeni doküman nesnesi nasıl eklenir (RF1 D2)

`ChangeOp` için kayıt yok (D2): yeni bir nesne türü (dokümanda yeni koleksiyon) şemaya ve
kapalı eşlemelere girer. Ölçüm (RF1 E, `grep`; `match` bloklarında `_ =>` var mı): bugün
dokunulacak dosyalar şunlardır.

| Crate | Dosya | Neden |
|---|---|---|
| `qp-schema` | `types/document.rs` | `Document` alanı |
| `qp-schema` | `protocol/change.rs` | `ChangeOp::Put…` (yoksa genel `Delete` yeter) |
| `qp-schema` | `protocol/query.rs` | `Collection` türü |
| `qp-schema` | `protocol/ops.rs` | yeni `Op` (gerekiyorsa) |
| `qp-ops` | `apply.rs` | `find_collection`, `all_ids` (zincir); `apply_change` ve `touched` (iki kapalı `match`, 24/23 kol) |
| `qp-ops` | `expand.rs` | `coll_name` (`Collection` üstünde kapalı) |
| `qp-ops` | `expand/undo.rs` | `expand_remove` (`Collection` üstünde kapalı) |
| `qp-ops` | `expand/op.rs`, `expand/describe.rs` | yeni `Op` yerleşikse (kapalı, 38 kol); alt sistemin `OpHandler`'ı açarsa dokunulmaz |
| `qp-authority` | `read/query.rs` | `collection_items` (`Collection` üstünde kapalı, 9 kol) |
| `qp-authority` | `replica/viewer.rs` | açık taraf için `Document { … }` (alan alan; yeni koleksiyonun açık tarafa gidip gitmeyeceği bir gizlilik kararı) |
| `qp-authority` | `capsule/queries.rs` | `ledger_now`'ın `Document { … }` değişmezi (S3.2'de `qp-read`'e taşınır) |
| `qp-authority` | `capsule/public.rs` | kimlikle öğe bulma zinciri (derleyici zorlamaz) |

**Otorite: hedef 0 dosya, bugün 3 zorunlu + 1 yumuşak.** `read/query.rs` kapalı eşlemedir;
`replica/viewer.rs` ve `capsule/queries.rs` `Document`'ın tam yapı değişmezleridir (`Document`'ta
`Default` yok); üçünü de derleyici ister. S3.2 `capsule/queries.rs`'i `qp-read`'e taşıyınca
otoritede 2 zorunlu kalır. `capsule/public.rs`'teki zincir derleyicinin zorlamadığı bir listedir;
eklenmezse yeni tür o yolda bulunmaz. Joker kollu eşlemeler (`capsule/project.rs`,
`capsule/public.rs` `to_public`, `capsule/settled.rs`, `authority/session/accept.rs`,
`data/live.rs`) yeni türü değiştirmeden geçirir; gizlilik ya da kamu dünyası farkı
gerekiyorsa onlara da dokunulur. İşlemin politikası (`authority/agent.rs`,
`authority/session.rs` tabloları) alt sistemin `OpHandler`'ıyla otoriteye dokunmadan verilir.

## Kod düzeni (hedef)

```
qapsule/
  crates/                                                                  [katman]
    qp-schema/     tipler + JSON Schema üretimi; protokol mesajları (`protocol`)  [şema]
    qp-compute/    hesap çekirdeği (Study, tarama)                         [çekirdek]
    qp-source/     seri kaynakları (SeriesSource: dosya, sentetik; R6'da canlı)
                   ve veri düzlemi                [uyarlayıcılar; alana alma isteği İ1]
    qp-calendar/   borsa takvimleri, saat dilimi tabloları, bar hizası (K14, K16)  [çekirdek]
    qp-facts/      olgu dedektörleri                                       [alan]
    qp-capsule/    kapsül motoru: olgu → hikâye → karne, cümle ve sesler   [alan]
    qp-grammar/    görsel gramer                                           [alan]
    qp-look/, qp-glyph/, qp-designs/  gramerin görünüş, simge ve tasarım parçaları (RF1)  [alan]
    qp-voice/      kapsül sesleri ve cümleleri (RF1'de qp-capsule'den)     [alan]
    qp-render/     çizim (headless + GPU)                                  [çekirdek]
    qp-plane/      veri düzlemi, değer önbelleği, hata kurucusu (RF1)      [oturum]
    qp-ops/        işlem açılımı, değişim uygulama, geri alma, yeniden oynatma (RF1)  [oturum]
    qp-subsystem/  alt sistem sözleşmesi: Subsystem, fazlar, yuvalar, giden kutusu (RF1)  [oturum]
    qp-watch/      izleme alt sistemi (RF1 S3.1)                           [oturum]
    qp-authority/  otorite çekirdeği: komut yolu, günlük, sorgu, abonelik, öneri,
                   iki dünya, kapsül yaşamı, canlı yol; alt sistem tanımaz (RF1)  [oturum]
    qp-session/    bileşim: otorite + standart alt sistemler (`STANDARD`); dış yüz  [oturum]
    qp-mcp/        MCP uyarlayıcısı; araçlar şemadan üretilir (RD1)        [uyarlayıcılar]
    qp-live/       canlı Matriks bağlantısı (belirteç, MQTT/WebSocket; R6) [uyarlayıcılar]
    qp-matriks/    Matriks ileti katmanı: proto, fiyat kartı, veri olayları (R6)  [uyarlayıcılar]
    qp-shellcore/  kabuk çekirdeği (`ShellCore`; RG1'de qp-wasm'dan)       [uyarlayıcılar]
    qp-host/       barındırma: canlı barındırıcı, benzetim, demo ve MCP sunucusu
                   (RG1'de qp-cli'den; planlı)                             [uyarlayıcılar]
    qp-cli/        ajanların ve testlerin kullandığı komut satırı (`serve`, `conform`, `mcp`)  [kabuklar]
    qp-wasm/       web bağlaması                                           [kabuklar]
  shells/          web, ios, android (ince kabuklar)
  fixtures/        örnek veri, örnek doküman, altın görseller
  docs/  rounds/
```

Katmanlar ve bağımlılık yönü `docs/ENGINEERING.md` §2'de, makine biçimi
`scripts/arch-config.json`'dadır; her crate'in kökünde kısa bir `README.md` vardır.

## Doğrulama omurgası

Ajanların yaptığı işin doğruluğu üç mekanizmayla ölçülür:

1. **Altın görsel testi:** sabit girdi → sabit görsel. Referans çizici deterministiktir.
2. **Özellik testleri:** hesap ve şema için rastgele girdiyle değişmezler.
3. **Uçtan uca CLI senaryoları:** her round'un çıktısı `qp` komutuyla ekransız sürülebilir.

## Açık teknik kararlar

`DECISIONS.md` içindeki "Açık" bölümüne bakın. R0 denemesi çizim yığınını belirler.
