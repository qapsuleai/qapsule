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
  ┌──────────────── otorite (qp-session) ────────────────┐
  │ yetki ve öneri ─► işlem açılımı ─► değişim ─► günlük │  ← saat bir girdidir (clock)
  │ sorgular (qp-compute ile)   sunucu (presenter)       │
  │ abonelik ve olaylar         varlık durumu (presence) │
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
  │ protokolden gelen metni ve sırayı gösterir │ qp/0│ otorite (qp-session)         │
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
    qp-render/     çizim (headless + GPU)                                  [çekirdek]
    qp-session/    otorite: komut yolu, günlük, sorgu, abonelik, öneri, sunucu (RD1)  [oturum]
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
